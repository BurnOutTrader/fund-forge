use std::collections::{HashMap};
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
use tokio::io;
use once_cell::sync::OnceCell;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::sync::mpsc::{Sender};
use tokio_rustls::TlsStream;
use crate::strategies::client_features::connection_types::ConnectionType;
use crate::strategies::handlers::drawing_object_handler::DrawingObjectHandler;
use crate::strategies::handlers::indicator_handler::IndicatorHandler;
use crate::strategies::handlers::market_handlers::{live_order_update, MarketMessageEnum};
use crate::standardized_types::enums::StrategyMode;
use crate::strategies::strategy_events::{StrategyEvent, StrategyEventBuffer};
use crate::strategies::handlers::subscription_handler::SubscriptionHandler;
use crate::standardized_types::subscriptions::{DataSubscription, DataSubscriptionEvent};
use crate::standardized_types::time_slices::TimeSlice;
use crate::strategies::handlers::timed_events_handler::TimedEventHandler;
use crate::standardized_types::bytes_trait::Bytes;
use crate::strategies::historical_time::update_backtest_time;

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


lazy_static! {
    pub static ref EVENT_BUFFER: Arc<Mutex<StrategyEventBuffer>> = Arc::new(Mutex::new(StrategyEventBuffer::new()));
}

#[inline(always)]
pub(crate) async fn add_buffer(time: DateTime<Utc>, event: StrategyEvent) {
    let mut buffer = EVENT_BUFFER.lock().await;
    buffer.add_event(time, event);
}

#[allow(dead_code)]
#[inline(always)]
pub(crate) async fn add_buffers(time: DateTime<Utc>, events: Vec<StrategyEvent>) {
    let mut buffer =  EVENT_BUFFER.lock().await;
    for event in events {
        buffer.add_event(time, event);
    }
}

#[allow(dead_code)]
#[inline(always)]
pub(crate) async fn extend_buffer(time: DateTime<Utc>, events: Vec<StrategyEvent>) {
    let mut buffer = EVENT_BUFFER.lock().await;
    for event in events {
        buffer.add_event(time, event);
    }
}

#[inline(always)]
pub(crate) async fn forward_buffer(time: DateTime<Utc>) {
    update_backtest_time(time);
    let mut buffer = EVENT_BUFFER.lock().await;
    if !buffer.is_empty() {
        let last_buffer = buffer.clone();
        buffer.clear();
        send_strategy_event_slice(last_buffer).await;
    }
}


pub(crate) static SUBSCRIPTION_HANDLER: OnceCell<Arc<SubscriptionHandler>> = OnceCell::new();
pub(crate) async fn subscribe_primary_subscription_updates(name: String, sender: Sender<Vec<DataSubscription>>) {
    SUBSCRIPTION_HANDLER.get().unwrap().subscribe_primary_subscription_updates(name, sender).await // Return a clone of the Arc to avoid moving the value out of the OnceCell
}
pub(crate) async fn unsubscribe_primary_subscription_updates(stream_name: &str) {
    SUBSCRIPTION_HANDLER.get().unwrap().unsubscribe_primary_subscription_updates(stream_name).await // Return a clone of the Arc to avoid moving the value out of the OnceCell
}

pub(crate) static INDICATOR_HANDLER: OnceCell<Arc<IndicatorHandler>> = OnceCell::new();
static TIMED_EVENT_HANDLER: OnceCell<Arc<TimedEventHandler>> = OnceCell::new();
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

static STRATEGY_SENDER: OnceCell<Sender<StrategyEventBuffer>> = OnceCell::new();

#[inline(always)]
pub(crate) async fn send_strategy_event_slice(slice: StrategyEventBuffer) {
    STRATEGY_SENDER.get().unwrap().send(slice).await.unwrap();
}

pub(crate) async fn live_subscription_handler(
    mode: StrategyMode,
) {
    if mode == StrategyMode::Backtest {
        return;
    }

    let (tx, rx) = mpsc::channel(100);
    subscribe_primary_subscription_updates("Live Subscription Updates".to_string(), tx).await;

    let settings_map = Arc::new(initialise_settings().unwrap());
    let mut subscription_update_channel = rx;

    let settings_map_ref = settings_map.clone();
    println!("Handler: Start Live handler");
    tokio::task::spawn(async move {
        {
            //let mut engine = HistoricalEngine::new(strategy_mode.clone(), start_time.to_utc(),  end_time.to_utc(), warmup_duration.clone(), buffering_resolution.clone(), notify, gui_enabled.clone()).await;
            //engine.warmup().await;
        }
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
        while let Some(updated_subscriptions) = subscription_update_channel.recv().await {
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

async fn fwd_data(time_slice: TimeSlice) {
    let slice = StrategyEvent::TimeSlice(time_slice);
    add_buffer(Utc::now(), slice).await;
    forward_buffer(Utc::now()).await;
}

pub async fn response_handler(
    mode: StrategyMode,
    buffer_duration: Duration,
    settings_map: HashMap<ConnectionType, ConnectionSettings>,
    server_receivers: DashMap<ConnectionType, ReadHalf<TlsStream<TcpStream>>>,
    callbacks: Arc<DashMap<u64, oneshot::Sender<DataServerResponse>>>,
    market_update_sender: Sender<MarketMessageEnum>
) {
    for (connection, settings) in &settings_map {
        if let Some((connection, stream)) = server_receivers.remove(connection) {
            let register_message = StrategyRequest::OneWay(connection.clone(), DataServerRequest::Register(mode.clone()));
            send_request(register_message).await;

            let mut receiver = stream;
            let callbacks = callbacks.clone();
            let market_update_sender = market_update_sender.clone();
            let settings = settings.clone();
            tokio::task::spawn(async move {
                const LENGTH: usize = 8;
                //println!("{:?}: response handler start", incoming.key());
                let mut length_bytes = [0u8; LENGTH];
                while let Ok(_) = receiver.read_exact(&mut length_bytes).await {
                    // Parse the length from the header
                    let msg_length = u64::from_be_bytes(length_bytes) as usize;
                    let mut message_body = vec![0u8; msg_length];

                    // Read the message body based on the length
                    match receiver.read_exact(&mut message_body).await {
                        Ok(_) => {
                            //eprintln!("Ok reading message body");
                        },
                        Err(e) => {
                            eprintln!("Error reading message body: {}", e);
                            continue;
                        }
                    }
                    // these will be buffered eventually into an EventTimeSlice
                    let response = DataServerResponse::from_bytes(&message_body).unwrap();
                    //todo handle responses async on a per response type basis
                    match response.get_callback_id() {
                        // if there is no callback id then we add it to the strategy event buffer
                        None => {
                            match response {
                                DataServerResponse::SubscribeResponse { success, subscription, reason } => {
                                    //determine success or fail and add to the strategy event buffer
                                    match success {
                                        true => {
                                            let event = DataSubscriptionEvent::Subscribed(subscription.clone());
                                            let event_slice = StrategyEvent::DataSubscriptionEvent(event);
                                            add_buffer(Utc::now(), event_slice).await;
                                        }
                                        false => {
                                            eprintln!("Failed to subscribe to {}, {:?}", subscription, reason);
                                            let event = DataSubscriptionEvent::FailedToSubscribe(subscription.clone(), reason.unwrap());
                                            let event_slice = StrategyEvent::DataSubscriptionEvent(event);
                                            add_buffer(Utc::now(), event_slice).await;
                                        }
                                    }
                                }
                                DataServerResponse::UnSubscribeResponse { success, subscription, reason } => {
                                    match success {
                                        true => {
                                            let event = DataSubscriptionEvent::Unsubscribed(subscription);
                                            let event_slice = StrategyEvent::DataSubscriptionEvent(event);
                                            add_buffer(Utc::now(), event_slice).await;
                                        }
                                        false => {
                                            eprintln!("Failed to unsubscribe from {}, {:?}", subscription, reason);
                                            let event = DataSubscriptionEvent::FailedUnSubscribed(subscription, reason.unwrap());
                                            let event_slice = StrategyEvent::DataSubscriptionEvent(event);
                                            add_buffer(Utc::now(), event_slice).await;
                                        }
                                    }
                                }
                                DataServerResponse::OrderUpdates(event) => {
                                    live_order_update(event, true).await;
                                }
                                DataServerResponse::RegistrationResponse(port) => {
                                    handle_live_data(settings.clone(), port, buffer_duration, mode, market_update_sender.clone()).await;
                                }
                                _ => panic!("Incorrect response here: {:?}", response)
                            }
                        }
                        // if there is a callback id we just send it to the awaiting oneshot receiver
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

pub async fn handle_live_data(connection_settings: ConnectionSettings, stream_name: u16, buffer_duration: Duration , mode: StrategyMode, market_update_sender: Sender<MarketMessageEnum>) {
    if mode == StrategyMode::Backtest {
        panic!("Backtest should not start live handler")
    }
    // set up async client
    let mut stream_client = match create_async_api_client(&connection_settings, true).await {
        Ok(client) => client,
        Err(__e) => {
            eprintln!("{}", format!("Unable to establish connection to server @ address: {:?}", connection_settings));
            return;
        }
    };

    // Register with the servers streaming handler
    let stream_registration = DataServerRequest::RegisterStreamer{port: stream_name.clone(), secs: buffer_duration.as_secs(), subsec: buffer_duration.subsec_nanos() };
    let data = stream_registration.to_bytes();
    let length = (data.len() as u64).to_be_bytes();
    let mut prefixed_msg = Vec::new();
    prefixed_msg.extend_from_slice(&length);
    prefixed_msg.extend_from_slice(&data);
    // Lock the mutex to get mutable access
    match stream_client
        .write_all(&prefixed_msg)
        .await {
        Ok(_) => { }
        Err(_e) => {}
    }


    let market_update_sender = market_update_sender.clone();
    let indicator_handler = INDICATOR_HANDLER.get().unwrap();
    tokio::task::spawn(async move {
        let subscription_handler = SUBSCRIPTION_HANDLER.get().unwrap().clone(); //todo this needs to exist before this fn is called, put response handler in own fn
        const LENGTH: usize = 4;
        //println!("{:?}: response handler start", incoming.key());
        let mut length_bytes = [0u8; LENGTH];
        while let Ok(_) = stream_client.read_exact(&mut length_bytes).await {
            // Parse the length from the header
            let msg_length = u32::from_be_bytes(length_bytes) as usize;
            let mut message_body = vec![0u8; msg_length];

            // Read the message body based on the length
            match stream_client.read_exact(&mut message_body).await {
                Ok(_) => {
                    //eprintln!("Ok reading message body");
                },
                Err(e) => {
                    eprintln!("Error reading message body: {}", e);
                    continue;
                }
            }
            // these will be buffered eventually into an EventTimeSlice
            let subscription_handler = subscription_handler.clone();
            let market_update_sender = market_update_sender.clone();
            tokio::task::spawn(async move {
                let time_slice = TimeSlice::from_bytes(&message_body).unwrap();
                market_update_sender.send(MarketMessageEnum::TimeSliceUpdate(time_slice.clone())).await.unwrap();
                let mut strategy_time_slice = TimeSlice::new();
                if let Some(consolidated) = subscription_handler.update_time_slice(time_slice.clone()).await {
                    strategy_time_slice.extend(consolidated);
                }
                if let Some(consolidated) = subscription_handler.update_consolidators_time(Utc::now()).await {
                    strategy_time_slice.extend(consolidated);
                }

                strategy_time_slice.extend(time_slice);
                indicator_handler.update_time_slice(Utc::now(), &strategy_time_slice).await;

                fwd_data(strategy_time_slice).await;
            });
        }
    });
}

/*pub async fn response_handler(mode: StrategyMode, receiver: mpsc::Receiver<StrategyRequest>, buffer_duration: Duration, settings_map: HashMap<ConnectionType, ConnectionSettings>, notify: Arc<Notify>) {

}*/

pub async fn init_sub_handler(subscription_handler: Arc<SubscriptionHandler>, event_sender: Sender<StrategyEventBuffer>, indicator_handler: Arc<IndicatorHandler>) {
    let _ = STRATEGY_SENDER.get_or_init(|| {
        event_sender
    }).clone();
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

pub async fn init_connections(gui_enabled: bool, buffer_duration: Duration, mode: StrategyMode, market_update_sender: Sender<MarketMessageEnum>) {
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
    response_handler(mode, buffer_duration, settings_map, server_receivers, callbacks, market_update_sender).await;
}
