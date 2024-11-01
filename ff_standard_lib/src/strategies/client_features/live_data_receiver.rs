use std::time::Duration;
use tokio::sync::mpsc::Sender;
use std::sync::Arc;
use chrono::Utc;
use tokio::runtime::Runtime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::messages::data_server_messaging::DataServerRequest;
use crate::standardized_types::bytes_trait::Bytes;
use crate::standardized_types::time_slices::TimeSlice;
use crate::strategies::client_features::connection_settings::client_settings::ConnectionSettings;
use crate::strategies::client_features::init_clients::create_async_api_client;
use crate::strategies::handlers::indicator_handler::IndicatorHandler;
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
    subscription_handler: Arc<SubscriptionHandler>
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
            let price_service_sender = get_price_service_sender();
            let mut length_bytes = [0u8; LENGTH];
            let mut interval = tokio::time::interval(Duration::from_secs(1));

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
}