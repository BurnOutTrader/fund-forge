use std::collections::BTreeMap;
use std::time::Duration;
use tokio::sync::mpsc::{Sender};
use std::sync::Arc;
use chrono::{Utc};
use tokio::runtime::Runtime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsStream;
use crate::messages::data_server_messaging::DataServerRequest;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::bytes_trait::Bytes;
use crate::standardized_types::time_slices::TimeSlice;
use crate::strategies::client_features::connection_settings::client_settings::ConnectionSettings;
use crate::strategies::client_features::init_clients::create_async_api_client;
use crate::strategies::client_features::server_connections::set_warmup_complete;
use crate::strategies::handlers::indicator_handler::IndicatorHandler;
use crate::strategies::handlers::live_warmup::WARMUP_COMPLETE_BROADCASTER;
use crate::strategies::handlers::market_handler::price_service::{get_price_service_sender, PriceServiceMessage};
use crate::strategies::handlers::subscription_handler::SubscriptionHandler;
use crate::strategies::ledgers::ledger_service::LedgerService;
use crate::strategies::strategy_events::StrategyEvent;

pub async fn handle_live_data(
    connection_settings: ConnectionSettings,
    stream_name: u16,
    buffer_duration: Duration,
    strategy_event_sender: Sender<StrategyEvent>,
    ledger_service: Arc<LedgerService>,
    indicator_handler: Arc<IndicatorHandler>,
    subscription_handler: Arc<SubscriptionHandler>,
) {

    let mut stream_client = match create_async_api_client(&connection_settings, true).await {
        Ok(client) => client,
        Err(e) => {
            panic!("Unable to establish connection to server @ address: {:?}: {}", connection_settings, e);
        }
    };

    // Register with server
    let stream_registration = DataServerRequest::RegisterStreamer {
        port: stream_name,
        secs: buffer_duration.as_secs(),
        subsec: buffer_duration.subsec_nanos()
    };
    let data = stream_registration.to_bytes();
    let length: [u8; 4] = (data.len() as u32).to_be_bytes();
    let mut prefixed_msg = Vec::new();
    prefixed_msg.extend_from_slice(&length);
    prefixed_msg.extend_from_slice(&data);

    if let Err(e) = stream_client.write_all(&prefixed_msg).await {
        eprintln!("Failed to register stream: {}", e);
        return;
    }

    let _ = tokio::task::spawn_blocking(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let mut buffered_data: BTreeMap<i64, TimeSlice> = BTreeMap::new();
            let price_service_sender = get_price_service_sender();
            receive_and_process(
                stream_client,
                &mut buffered_data,
                strategy_event_sender,
                ledger_service,
                indicator_handler,
                subscription_handler,
                price_service_sender,
            ).await;
        });
    });
}

async fn receive_and_process(
    mut stream_client: TlsStream<TcpStream>,
    buffered_data: &mut BTreeMap<i64, TimeSlice>,
    strategy_event_sender: Sender<StrategyEvent>,
    ledger_service: Arc<LedgerService>,
    indicator_handler: Arc<IndicatorHandler>,
    subscription_handler: Arc<SubscriptionHandler>,
    price_service_sender: Sender<PriceServiceMessage>,
) {
    const LENGTH: usize = 4;
    let mut length_bytes = [0u8; LENGTH];
    let mut warmup_completion_receiver = WARMUP_COMPLETE_BROADCASTER.subscribe();
    #[allow(unused_assignments)]
    let mut warm_up_end = Utc::now();
    // First phase: Buffer data during warmup
    loop {
        tokio::select! {
            result = stream_client.read_exact(&mut length_bytes) => {
                match result {
                    Ok(_) => {
                        let msg_length = u32::from_be_bytes(length_bytes) as usize;
                        let mut message_body = vec![0u8; msg_length];

                        if let Err(e) = stream_client.read_exact(&mut message_body).await {
                            eprintln!("Error reading message body: {}", e);
                            continue;
                        }

                        if let Ok(time_slice) = TimeSlice::from_bytes(&message_body) {
                            for data in time_slice.iter() {
                                let timestamp = data.time_closed_utc().timestamp_nanos_opt().unwrap();
                                buffered_data.entry(timestamp)
                                    .and_modify(|slice| slice.extend(time_slice.clone()))
                                    .or_insert_with(|| {
                                        let mut new_slice = TimeSlice::new();
                                        new_slice.extend(time_slice.clone());
                                        new_slice
                                    });
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading length bytes: {}", e);
                        return;
                    }
                }
            }
            Ok(time) = warmup_completion_receiver.recv() => {
                warm_up_end = time;
                break;
            }
        }
    }

    // Process buffered data
    let buffer_to_process = std::mem::take(buffered_data);
    for (_, slice) in buffer_to_process
        .range(warm_up_end.timestamp()..=Utc::now().timestamp())
        .filter(|(_, slice)| !slice.is_empty())
    {
        let mut strategy_time_slice = TimeSlice::new();
        let arc_slice = Arc::new(slice.clone());

        let _ = price_service_sender.send(PriceServiceMessage::TimeSliceUpdate(arc_slice.clone())).await;
        ledger_service.timeslice_updates(Utc::now(), arc_slice.clone()).await;

        if let Some(consolidated_data) = subscription_handler.update_time_slice(arc_slice).await {
            strategy_time_slice.extend(consolidated_data);
        }
        strategy_time_slice.extend(slice.clone());

        if let Some(events) = indicator_handler.update_time_slice(&strategy_time_slice).await {
            let _ = strategy_event_sender.send(StrategyEvent::IndicatorEvent(events)).await;
        }
        let _ = strategy_event_sender.send(StrategyEvent::TimeSlice(strategy_time_slice)).await;
    }
    drop(buffer_to_process);
    set_warmup_complete();
    drop(warmup_completion_receiver);

    let now = tokio::time::Instant::now();
    let nanos_into_second = now.elapsed().subsec_nanos();
    if nanos_into_second > 0 {
        let wait_nanos = 1_000_000_000 - nanos_into_second;
        tokio::time::sleep(Duration::from_nanos(wait_nanos as u64)).await;
    }

    // Switch to live processing
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let mut last_update_second = Utc::now().timestamp();
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let now = Utc::now();
                let current_second = now.timestamp();
                // Only update if we haven't processed data for this second
                if current_second > last_update_second {
                    if let Some(consolidated_data) = subscription_handler.update_consolidators_time(now).await {
                        if let Some(events) = indicator_handler.update_time_slice(&consolidated_data).await {
                            let _ = strategy_event_sender.send(StrategyEvent::IndicatorEvent(events)).await;
                        }
                        let _ = strategy_event_sender.send(StrategyEvent::TimeSlice(consolidated_data)).await;
                    }
                    last_update_second = current_second;
                }
            }
            result = stream_client.read_exact(&mut length_bytes) => {
                match result {
                    Ok(_) => {
                        let msg_length = u32::from_be_bytes(length_bytes) as usize;
                        let mut message_body = vec![0u8; msg_length];

                        if let Err(e) = stream_client.read_exact(&mut message_body).await {
                            eprintln!("Error reading message body: {}", e);
                            continue;
                        }

                        if let Ok(time_slice) = TimeSlice::from_bytes(&message_body) {
                             let mut strategy_time_slice = TimeSlice::new();
                            if !time_slice.is_empty() {
                                last_update_second =  Utc::now().timestamp();  // Mark this second as updated
                                let arc_slice = Arc::new(time_slice.clone());
                                let _ = price_service_sender.send(PriceServiceMessage::TimeSliceUpdate(arc_slice.clone())).await;
                                ledger_service.timeslice_updates(Utc::now(), arc_slice.clone()).await;

                                if let Some(consolidated_data) = subscription_handler.update_time_slice(arc_slice).await {
                                    strategy_time_slice.extend(consolidated_data);
                                }
                                strategy_time_slice.extend(time_slice);

                                if let Some(events) = indicator_handler.update_time_slice(&strategy_time_slice).await {
                                    let _ = strategy_event_sender.send(StrategyEvent::IndicatorEvent(events)).await;
                                }
                                let _ = strategy_event_sender.send(StrategyEvent::TimeSlice(strategy_time_slice)).await;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading length bytes: {}", e);
                        break;
                    }
                }
            }
        }
    }

    let _ = strategy_event_sender
        .send(StrategyEvent::ShutdownEvent(String::from("Live stream disconnected")))
        .await;
}




//old
/*
pub async fn handle_live_data(
    connection_settings: ConnectionSettings,
    stream_name: u16,
    buffer_duration: Duration,
    strategy_event_sender: Sender<StrategyEvent>,
    ledger_service: Arc<LedgerService>,
    indicator_handler: Arc<IndicatorHandler>,
    subscription_handler: Arc<SubscriptionHandler>,
) {
    // set up async client
    let mut stream_client = match create_async_api_client(&connection_settings, true).await {
        Ok(client) => client,
        Err(e) => {
            panic!("Unable to establish connection to server @ address: {:?}: {}", connection_settings, e);
        }
    };

    const LENGTH: usize = 4;
    // Register with the servers streaming handler
    let stream_registration = DataServerRequest::RegisterStreamer{port: stream_name.clone(), secs: buffer_duration.as_secs(), subsec: buffer_duration.subsec_nanos() };
    let data = stream_registration.to_bytes();
    let length: [u8; 4] = (data.len() as u32).to_be_bytes();
    let mut prefixed_msg = Vec::new();
    prefixed_msg.extend_from_slice(&length);
    prefixed_msg.extend_from_slice(&data);
    // Lock the mutex to get mutable access
    match stream_client.write_all(&prefixed_msg).await {
        Ok(_) => { }
        Err(e) => {
            eprintln!("{}", e);
        }
    }

    let _ = tokio::task::spawn_blocking(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {

            //here i create a task that handles buffering warm up data and then when warm up is complete it finish and we pass in the buffered data to the live handler
            // it might be best to send a message, since then we will set warm up complete from here, instead of the live_warm_up fn.

            let price_service_sender = get_price_service_sender();
            let mut length_bytes = [0u8; LENGTH];
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            let mut warm_up_slice = TimeSlice::new();
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let now = Utc::now();
                        let mut strategy_time_slice = TimeSlice::new();

                        if let Some(consolidated_data) = subscription_handler.update_consolidators_time(now).await {
                            strategy_time_slice.extend(consolidated_data);
                        }

                        if !strategy_time_slice.is_empty() {
                            match strategy_event_sender.send(StrategyEvent::TimeSlice(strategy_time_slice)).await {
                                Ok(_) => {}
                                Err(e) => eprintln!("Live Handler: {}", e)
                            }
                        }
                    }
                    result = stream_client.read_exact(&mut length_bytes) => {
                        match result {
                            Ok(_) => {
                                let msg_length = u32::from_be_bytes(length_bytes) as usize;
                                let mut message_body = vec![0u8; msg_length];

                                if let Err(e) = stream_client.read_exact(&mut message_body).await {
                                    eprintln!("Error reading message body: {}", e);
                                    continue;
                                }

                                let time_slice = match TimeSlice::from_bytes(&message_body) {
                                    Ok(ts) => ts,
                                    Err(e) => {
                                        eprintln!("Error parsing TimeSlice: {}", e);
                                        continue;
                                    }
                                };

                                // we could do this as separate fn, that then calls this fn
                                if !is_warmup_complete() {
                                    warm_up_slice.extend(time_slice);
                                    continue
                                }

                                let mut strategy_time_slice = TimeSlice::new();
                                if !time_slice.is_empty() {
                                    let arc_slice = Arc::new(time_slice.clone());
                                    match price_service_sender.send(PriceServiceMessage::TimeSliceUpdate(arc_slice.clone())).await {
                                        Ok(_) => {},
                                        Err(_) => {}
                                    }
                                    ledger_service.timeslice_updates(Utc::now(), arc_slice.clone()).await;
                                    if let Some(consolidated_data) = subscription_handler.update_time_slice(arc_slice.clone()).await {
                                        strategy_time_slice.extend(consolidated_data);
                                    }
                                    strategy_time_slice.extend(time_slice);
                                }

                                if !strategy_time_slice.is_empty() {
                                    indicator_handler.update_time_slice(&strategy_time_slice).await;
                                    match strategy_event_sender.send(StrategyEvent::TimeSlice(strategy_time_slice)).await {
                                        Ok(_) => {}
                                        Err(e) => eprintln!("Live Handler: {}", e)
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Error reading length bytes: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
            match strategy_event_sender.send(StrategyEvent::ShutdownEvent(String::from("Disconnected Live Stream"))).await {
                Ok(_) => {}
                Err(e) => eprintln!("Live Handler: {}", e)
            }
        });
    });
}*/