use std::collections::{HashMap};
use std::str::FromStr;
use crate::communicators::communications_async::ExternalSender;
use crate::strategies::client_features::init_clients::create_async_api_client;
use crate::strategies::client_features::connection_settings::client_settings::{initialise_settings, ConnectionSettings};
use crate::messages::data_server_messaging::{DataServerRequest, DataServerResponse, StreamRequest};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use ahash::AHashMap;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use lazy_static::lazy_static;
use tokio::{io};
use once_cell::sync::OnceCell;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};
use tokio::sync::mpsc::Sender;
use tokio_rustls::TlsStream;
use crate::strategies::client_features::connection_types::ConnectionType;
use crate::strategies::handlers::drawing_object_handler::DrawingObjectHandler;
use crate::strategies::handlers::indicator_handler::IndicatorHandler;
use crate::standardized_types::enums::StrategyMode;
use crate::strategies::strategy_events::{StrategyEvent};
use crate::strategies::handlers::subscription_handler::SubscriptionHandler;
use crate::standardized_types::subscriptions::{DataSubscription, DataSubscriptionEvent};
use crate::standardized_types::time_slices::TimeSlice;
use crate::strategies::handlers::timed_events_handler::TimedEventHandler;
use crate::standardized_types::bytes_trait::Bytes;
use crate::standardized_types::orders::OrderUpdateEvent;
use crate::strategies::handlers::market_handler::price_service::{get_price_service_sender, PriceServiceMessage};
use crate::strategies::ledgers::{LEDGER_SERVICE};

lazy_static! {
    static ref WARM_UP_COMPLETE: AtomicBool = AtomicBool::new(false);
}
#[inline(always)]
pub fn set_warmup_complete() {
    WARM_UP_COMPLETE.store(true, Ordering::SeqCst);
}
#[inline(always)]
pub fn is_warmup_complete() -> bool {
    WARM_UP_COMPLETE.load(Ordering::SeqCst)
}



pub(crate) static SUBSCRIPTION_HANDLER: OnceCell<Arc<SubscriptionHandler>> = OnceCell::new();
pub(crate) fn subscribe_primary_subscription_updates() -> broadcast::Receiver<Vec<DataSubscription>> {
    SUBSCRIPTION_HANDLER.get().unwrap().subscribe_primary_subscription_updates() // Return a clone of the Arc to avoid moving the value out of the OnceCell
}



pub(crate) static INDICATOR_HANDLER: OnceCell<Arc<IndicatorHandler>> = OnceCell::new();
pub(crate) static TIMED_EVENT_HANDLER: OnceCell<Arc<TimedEventHandler>> = OnceCell::new();
static DRAWING_OBJECTS_HANDLER: OnceCell<Arc<DrawingObjectHandler>> = OnceCell::new();


pub(crate) enum StrategyRequest {
    CallBack(ConnectionType, DataServerRequest, oneshot::Sender<DataServerResponse>),
    OneWay(ConnectionType, DataServerRequest),
}
static DATA_SERVER_SENDER: OnceCell<Arc<Mutex<Sender<StrategyRequest>>>> = OnceCell::new();

#[inline(always)]
pub(crate) async fn send_request(req: StrategyRequest) {
    let sender = DATA_SERVER_SENDER.get().unwrap().lock().await;
    sender.send(req).await.unwrap(); // Return a clone of the Arc to avoid moving the value out of the OnceCell
}

pub(crate) async fn live_subscription_handler(
    mode: StrategyMode,
) {
    if mode == StrategyMode::Backtest {
        return;
    }

    let settings_map = Arc::new(initialise_settings().unwrap());
    let mut subscription_update_channel = subscribe_primary_subscription_updates();

    let settings_map_ref = settings_map.clone();
    println!("Handler: Start Live handler");
    tokio::task::spawn(async move {
        let mut current_subscriptions = SUBSCRIPTION_HANDLER.get().unwrap().primary_subscriptions().await.clone();
        {
            let mut subscribed = vec![];
            println!("Handler: {:?}", current_subscriptions);
            for subscription in &*current_subscriptions {
                let request = DataServerRequest::StreamRequest {
                    request: StreamRequest::Subscribe(subscription.clone())
                };
                let connection = ConnectionType::Vendor(subscription.symbol.data_vendor.clone());
                let connection_type = match settings_map_ref.contains_key(&connection) {
                    true => connection,
                    false => ConnectionType::Default
                };
                if !subscribed.contains(&connection_type) {
                    let register = StrategyRequest::OneWay(connection_type.clone(), DataServerRequest::Register(mode.clone()));
                    send_request(register).await;
                    subscribed.push(connection_type.clone());
                }
                let request = StrategyRequest::OneWay(connection_type, request);
                send_request(request).await;
            }
        }
        while let Ok(updated_subscriptions) = subscription_update_channel.recv().await {
            let mut requests_map = AHashMap::new();
            if current_subscriptions != updated_subscriptions {
                for subscription in &updated_subscriptions {
                    if !current_subscriptions.contains(&subscription) {
                        let connection = ConnectionType::Vendor(subscription.symbol.data_vendor.clone());
                        let connection_type = match settings_map_ref.contains_key(&connection) {
                            true => connection,
                            false => ConnectionType::Default
                        };
                        let request = DataServerRequest::StreamRequest { request: StreamRequest::Subscribe(subscription.clone())};
                        if !requests_map.contains_key(&connection_type) {
                            requests_map.insert(connection_type, vec![request]);
                        } else {
                            requests_map.get_mut(&connection_type).unwrap().push(request);
                        }
                    }
                }
                for subscription in &*current_subscriptions {
                    if !updated_subscriptions.contains(&subscription) {
                        let connection = ConnectionType::Vendor(subscription.symbol.data_vendor.clone());
                        let connection_type = match settings_map_ref.contains_key(&connection) {
                            true => connection,
                            false => ConnectionType::Default
                        };
                        let request = DataServerRequest::StreamRequest { request: StreamRequest::Unsubscribe(subscription.clone())};

                        if !requests_map.contains_key(&connection_type) {
                            requests_map.insert(connection_type, vec![request]);
                        } else {
                            requests_map.get_mut(&connection_type).unwrap().push(request);
                        }
                    }
                }
                for (connection, requests) in requests_map {
                    for request in requests {
                        let request = StrategyRequest::OneWay(connection.clone(), request);
                        send_request(request).await;
                    }
                }
                current_subscriptions = updated_subscriptions.clone();
            }
        }
    });
}

/// This response handler is also acting as a live engine.
async fn request_handler(
    receiver: mpsc::Receiver<StrategyRequest>,
    settings_map: HashMap<ConnectionType, ConnectionSettings>,
    server_senders: DashMap<ConnectionType, ExternalSender>,
    callbacks: Arc<DashMap<u64, oneshot::Sender<DataServerResponse>>>,
) {
    let mut receiver = receiver;
    let connection_map = settings_map.clone();
    let callbacks_ref = callbacks.clone();
    let server_senders = server_senders;
    tokio::task::spawn(async move {
        let mut callback_id_counter: u64 = 0;
        let callbacks = callbacks_ref.clone();
        while let Some(outgoing_message) = receiver.recv().await {
            match outgoing_message {
                StrategyRequest::CallBack(connection_type, mut request, oneshot) => {
                    callback_id_counter = callback_id_counter.wrapping_add(1);
                    let callbacks = callbacks.clone();
                    let id = callback_id_counter.clone();
                    callbacks.insert(id, oneshot);
                    request.set_callback_id(id.clone());
                    let connection_type = match connection_map.contains_key(&connection_type) {
                        true => connection_type,
                        false => ConnectionType::Default
                    };
                    let sender = server_senders.get(&connection_type).unwrap();
                    sender.send(&request.to_bytes()).await;
                }
                StrategyRequest::OneWay(connection_type, request) => {
                    let connection_type = match connection_map.contains_key(&connection_type) {
                        true => connection_type,
                        false => ConnectionType::Default
                    };
                    let sender = server_senders.get(&connection_type).unwrap();
                    sender.send(&request.to_bytes()).await;
                }
            }
        }
    });
}

pub async fn response_handler(
    mode: StrategyMode,
    buffer_duration: Duration,
    settings_map: HashMap<ConnectionType, ConnectionSettings>,
    server_receivers: DashMap<ConnectionType, ReadHalf<TlsStream<TcpStream>>>,
    callbacks: Arc<DashMap<u64, oneshot::Sender<DataServerResponse>>>,
    order_updates_sender: Sender<(OrderUpdateEvent, DateTime<Utc>)>,
    synchronise_accounts: bool,
    strategy_event_sender: Sender<StrategyEvent>,
) {
    for (connection, settings) in &settings_map {
        let order_updates_sender = order_updates_sender.clone();
        if let Some((connection, stream)) = server_receivers.remove(connection) {
            let register_message = StrategyRequest::OneWay(connection.clone(), DataServerRequest::Register(mode.clone()));
            send_request(register_message).await;

            let mut receiver = stream;
            let callbacks = callbacks.clone();
            let settings = settings.clone();
            let strategy_event_sender = strategy_event_sender.clone();
            tokio::task::spawn(async move {
                const LENGTH: usize = 8;
                let mut length_bytes = [0u8; LENGTH];
                while let Ok(_) = receiver.read_exact(&mut length_bytes).await {
                    let msg_length = u64::from_be_bytes(length_bytes) as usize;
                    let mut message_body = vec![0u8; msg_length];

                    match receiver.read_exact(&mut message_body).await {
                        Ok(_) => {},
                        Err(e) => {
                            eprintln!("Error reading message body: {}", e);
                            continue;
                        }
                    }

                    let response = DataServerResponse::from_bytes(&message_body).unwrap();
                    match response.get_callback_id() {
                        None => {
                            match response {
                                DataServerResponse::SubscribeResponse { success, subscription, reason } => {
                                    let event = if success {
                                        DataSubscriptionEvent::Subscribed(subscription.clone())
                                    } else {
                                        DataSubscriptionEvent::FailedToSubscribe(subscription.clone(), reason.unwrap())
                                    };
                                    let event = StrategyEvent::DataSubscriptionEvent(event);
                                    match strategy_event_sender.send(event).await {
                                        Ok(_) => {}
                                        Err(_) => {}
                                    }
                                }
                                DataServerResponse::UnSubscribeResponse { success, subscription, reason } => {
                                    let event = if success {
                                        DataSubscriptionEvent::Unsubscribed(subscription)
                                    } else {
                                        DataSubscriptionEvent::FailedUnSubscribed(subscription, reason.unwrap())
                                    };
                                    let event = StrategyEvent::DataSubscriptionEvent(event);
                                    match strategy_event_sender.send(event).await {
                                        Ok(_) => {}
                                        Err(_) => {}
                                    }
                                }
                                DataServerResponse::OrderUpdates{ event, time} => {
                                    //println!("Event received: {}", update_event);
                                    let time = DateTime::<Utc>::from_str(&time).unwrap();
                                    order_updates_sender.send((event, time)).await.unwrap()
                                }
                                DataServerResponse::LiveAccountUpdates { account, cash_value, cash_available, cash_used } => {
                                    tokio::task::spawn(async move {
                                        LEDGER_SERVICE.live_account_updates(&account, cash_value, cash_available, cash_used).await;
                                    });
                                }
                                DataServerResponse::LivePositionUpdates { account, position, time } => {
                                   if synchronise_accounts {
                                       //println!("Live Position: {:?}", position);
                                        //tokio::task::spawn(async move {
                                       //todo, we need a message que for ledger, where orders and positions are update the ledger 1 at a time per symbol_code, this should fix the possible race conditions of positions updates
                                       let time = DateTime::<Utc>::from_str(&time).unwrap();
                                       match LEDGER_SERVICE.synchronize_live_position(account, position, time) {
                                           None => {}
                                           Some(event) => {
                                               strategy_event_sender.send(StrategyEvent::PositionEvents(event)).await.unwrap();
                                           }
                                       }
                                        //});
                                    }
                                }
                                DataServerResponse::RegistrationResponse(port) => {
                                    if mode != StrategyMode::Backtest {
                                        handle_live_data(settings.clone(), port, buffer_duration, strategy_event_sender.clone()).await;
                                    }
                                }
                                _ => panic!("Incorrect response here: {:?}", response)
                            }
                        }
                        Some(id) => {
                            if let Some((_, callback)) = callbacks.remove(&id) {
                                let _ = callback.send(response);
                            }
                        }
                    }
                }
            });
        }
    }
}

pub async fn handle_live_data(connection_settings: ConnectionSettings, stream_name: u16, buffer_duration: Duration , strategy_event_sender: Sender<StrategyEvent>) {
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
            let subscription_handler = SUBSCRIPTION_HANDLER.get().unwrap().clone();
            let timed_event_handler = TIMED_EVENT_HANDLER.get().unwrap();
            let price_service_sender = get_price_service_sender();
            let indicator_handler = INDICATOR_HANDLER.get().unwrap().clone();
            let mut length_bytes = [0u8; LENGTH];
            let mut interval = tokio::time::interval(Duration::from_secs(1));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let now = Utc::now();
                        timed_event_handler.update_time(now).await;
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
                                    LEDGER_SERVICE.timeslice_updates(Utc::now(), arc_slice.clone()).await;
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

pub async fn init_sub_handler(subscription_handler: Arc<SubscriptionHandler>, indicator_handler: Arc<IndicatorHandler>) {
    let _ = SUBSCRIPTION_HANDLER.get_or_init(|| {
        subscription_handler
    }).clone();
    let _ = INDICATOR_HANDLER.get_or_init(|| {
        indicator_handler
    }).clone();
}

pub async fn initialize_static(
    timed_event_handler: Arc<TimedEventHandler>,
    drawing_objects_handler: Arc<DrawingObjectHandler>,
) {
    let _ = TIMED_EVENT_HANDLER.get_or_init(|| {
        timed_event_handler
    }).clone();
    let _ = DRAWING_OBJECTS_HANDLER.get_or_init(|| {
        drawing_objects_handler
    }).clone();
}

pub async fn init_connections(
    gui_enabled: bool,
    buffer_duration: Duration,
    mode: StrategyMode,
    order_updates_sender: Sender<(OrderUpdateEvent, DateTime<Utc>)>,
    synchronise_accounts: bool,
    strategy_event_sender: Sender<StrategyEvent>
) {
    let settings_map = initialise_settings().unwrap();
    let server_receivers: DashMap<ConnectionType, ReadHalf<TlsStream<TcpStream>>> = DashMap::with_capacity(settings_map.len());
    let server_senders: DashMap<ConnectionType, ExternalSender> = DashMap::with_capacity(settings_map.len());

    println!("Connections: {:?}", settings_map);
    // for each connection type specified in our server_settings.toml we will establish a connection
    for (connection_type, settings) in settings_map.iter() {
        if !gui_enabled && connection_type == &ConnectionType::StrategyRegistry {
            continue
        }
        // set up async client
        let async_client = match create_async_api_client(&settings, false).await {
            Ok(client) => client,
            Err(__e) => { eprintln!("{}", format!("Unable to establish connection to: {:?} server @ address: {:?}", connection_type, settings));
                continue;
            }
        };
        let (read_half, write_half) = io::split(async_client);
        let async_sender = ExternalSender::new(write_half);
        server_senders.insert(connection_type.clone(), async_sender);
        server_receivers.insert(connection_type.clone(), read_half);
    }
    let (tx, rx) = mpsc::channel(1000);
    let _ = DATA_SERVER_SENDER.get_or_init(|| {
        Arc::new(Mutex::new(tx))
    }).clone();

    let callbacks: Arc<DashMap<u64, oneshot::Sender<DataServerResponse>>> = Default::default();
    request_handler(rx, settings_map.clone(), server_senders, callbacks.clone()).await;
    response_handler(mode, buffer_duration, settings_map, server_receivers, callbacks, order_updates_sender, synchronise_accounts, strategy_event_sender).await;
}
