use std::collections::{HashMap};
use crate::servers::communications_async::{ExternalSender};
use crate::servers::init_clients::{create_async_api_client};
use crate::servers::settings::client_settings::{initialise_settings, ConnectionSettings};
use crate::standardized_types::data_server_messaging::{DataServerRequest, DataServerResponse, FundForgeError, StreamRequest};
use heck::ToPascalCase;
use serde_derive::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use ahash::AHashMap;
use chrono::{DateTime, Utc,  Duration as ChronoDuration};
use dashmap::DashMap;
use futures::SinkExt;
use strum_macros::Display;
use tokio::io;
use once_cell::sync::OnceCell;
use tokio::io::{AsyncReadExt, ReadHalf};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, Mutex, Notify, RwLock};
use tokio::sync::mpsc::Sender;
use tokio::time::{sleep_until, Instant};
use tokio_rustls::TlsStream;
use crate::apis::brokerage::broker_enum::Brokerage;
use crate::apis::data_vendor::datavendor_enum::DataVendor;
use crate::drawing_objects::drawing_object_handler::DrawingObjectHandler;
use crate::indicators::indicator_handler::{IndicatorHandler};
use crate::interaction_handler::InteractionHandler;
use crate::market_handler::market_handlers::MarketHandler;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::enums::StrategyMode;
use crate::standardized_types::orders::orders::{OrderRequest};
use crate::standardized_types::strategy_events::{EventTimeSlice, StrategyEvent};
use crate::standardized_types::subscription_handler::SubscriptionHandler;
use crate::standardized_types::subscriptions::{DataSubscription, DataSubscriptionEvent};
use crate::standardized_types::time_slices::TimeSlice;
use crate::timed_events_handler::TimedEventHandler;
use crate::traits::bytes::Bytes;

pub const GUI_ENABLED: bool = true;
pub const GUI_DISABLED: bool = false;

/// A wrapper to allow us to pass in either a `Brokerage` or a `DataVendor`
/// # Variants
/// * `Broker(Brokerage)` - Containing a `Brokerage` object
/// * `Vendor(DataVendor)` - Containing a `DataVendor` object
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize, Debug, Display)]
pub enum ConnectionType {
    Vendor(DataVendor),
    Broker(Brokerage),
    Default,
    StrategyRegistry,
}

impl FromStr for ConnectionType {
    type Err = FundForgeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let string = s.to_pascal_case();
        match string.as_str() {
            "Default" => Ok(ConnectionType::Default),
            "StrategyRegistry" => Ok(ConnectionType::StrategyRegistry),
            _ if s.starts_with("Broker:") => {
                let data = s.trim_start_matches("Broker:").trim();
                Ok(ConnectionType::Broker(Brokerage::from_str(data)?))
            }
            _ if s.starts_with("Vendor:") => {
                let data = s.trim_start_matches("Vendor:").trim();
                Ok(ConnectionType::Vendor(DataVendor::from_str(data)?))
            }
            _ => Err(FundForgeError::ClientSideErrorDebug(format!(
                "Connection Type {} is not recognized",
                s
            ))),
        }
    }
}


pub static SUBSCRIPTION_HANDLER: OnceCell<Arc<SubscriptionHandler>> = OnceCell::new();
pub async fn subscribe_primary_subscription_updates(name: String, sender: Sender<Vec<DataSubscription>>) {
    SUBSCRIPTION_HANDLER.get().unwrap().subscribe_primary_subscription_updates(name, sender).await // Return a clone of the Arc to avoid moving the value out of the OnceCell
}
pub async fn unsubscribe_primary_subscription_updates(name: String) {
    SUBSCRIPTION_HANDLER.get().unwrap().unsubscribe_primary_subscription_updates(name).await // Return a clone of the Arc to avoid moving the value out of the OnceCell
}

pub static INDICATOR_HANDLER: OnceCell<Arc<IndicatorHandler>> = OnceCell::new();
pub static MARKET_HANDLER: OnceCell<Arc<MarketHandler>> = OnceCell::new();
static INTERACTION_HANDLER: OnceCell<Arc<InteractionHandler>> = OnceCell::new();
static TIMED_EVENT_HANDLER: OnceCell<Arc<TimedEventHandler>> = OnceCell::new();
static DRAWING_OBJECTS_HANDLER: OnceCell<Arc<DrawingObjectHandler>> = OnceCell::new();

pub async fn set_warmup_complete() {
    SUBSCRIPTION_HANDLER.get_or_init(|| {
        panic!("SUBSCRIPTION_HANDLER Not found")
    }).set_warmup_complete().await;
    INDICATOR_HANDLER.get_or_init(|| {
        panic!("INDICATOR_HANDLER Not found")
    }).set_warmup_complete().await;
    INTERACTION_HANDLER.get_or_init(|| {
        panic!("INTERACTION_HANDLER Not found")
    }).set_warmup_complete().await;
    TIMED_EVENT_HANDLER.get_or_init(|| {
        panic!("TIMED_EVENT_HANDLER Not found")
    }).set_warmup_complete().await;
    MARKET_HANDLER.get_or_init(|| {
        panic!("MARKET_HANDLER Not found")
    }).set_warmup_complete().await;
}


pub(crate) enum StrategyRequest {
    CallBack(ConnectionType, DataServerRequest, oneshot::Sender<DataServerResponse>),
    OneWay(ConnectionType, DataServerRequest),
}
static DATA_SERVER_SENDER: OnceCell<Arc<Mutex<Sender<StrategyRequest>>>> = OnceCell::new();
pub(crate) fn get_sender() -> Arc<Mutex<Sender<StrategyRequest>>> {
    DATA_SERVER_SENDER.get().unwrap().clone() // Return a clone of the Arc to avoid moving the value out of the OnceCell
}
pub(crate) async fn send_request(req: StrategyRequest) {
    get_sender().lock().await.send(req).await.unwrap(); // Return a clone of the Arc to avoid moving the value out of the OnceCell
}

static STRATEGY_SENDER: OnceCell<Sender<EventTimeSlice>> = OnceCell::new();
pub async fn send_strategy_event_slice(slice: EventTimeSlice) {
    STRATEGY_SENDER.get().unwrap().send(slice).await.unwrap();
}

pub async fn live_subscription_handler(
    mode: StrategyMode,
    start_time: DateTime<Utc>,
    end_date: DateTime<Utc>,
    warmup_duration: ChronoDuration,
    buffer_resolution: Duration,
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
                let register = StrategyRequest::OneWay(connection_type.clone(), DataServerRequest::Register(mode.clone()));
                send_request(register).await;
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

pub async fn live_order_handler(
    mode: StrategyMode,
    order_receiver: mpsc::Receiver<OrderRequest>
) {
    let settings_map = Arc::new(initialise_settings().unwrap());
    if mode == StrategyMode::Live {
        let connection_map = settings_map;
        tokio::task::spawn(async move {
            let mut order_receiver = order_receiver;
            while let Some(order_request) = order_receiver.recv().await {
                let connection_type = ConnectionType::Broker(order_request.brokerage());
                let connection_type = match connection_map.contains_key(&connection_type) {
                    true => connection_type,
                    false => ConnectionType::Default
                };
                let request = DataServerRequest::OrderRequest {
                    request: order_request
                };
                send_request(StrategyRequest::OneWay(connection_type, request)).await;
            }
        });
    }
}

/// This response handler is also acting as a live engine.
pub async fn request_handler(
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
                    callback_id_counter += 1;
                    let callbacks = callbacks.clone();
                    let id = callback_id_counter.clone();
                    callbacks.insert(id, oneshot);
                    request.set_callback_id(id.clone());
                    let connection_type = match connection_map.contains_key(&connection_type) {
                        true => connection_type,
                        false => ConnectionType::Default
                    };
                    let sender = server_senders.get(&connection_type).unwrap();
                    match sender.send(&request.to_bytes()).await {
                        Ok(_) => {}
                        Err(e) => eprintln!("{}", e)
                    }
                }
                StrategyRequest::OneWay(connection_type, request) => {
                    let connection_type = match connection_map.contains_key(&connection_type) {
                        true => connection_type,
                        false => ConnectionType::Default
                    };
                    let sender = server_senders.get(&connection_type).unwrap();
                    match sender.send(&request.to_bytes()).await {
                        Ok(_) => {}
                        Err(e) => eprintln!("{}", e)
                    }
                }
            }
        }
    });
}


/*
    1. primary data comes from server stream
    2. primary data is fed to
        a. subscription handler
        b. market handler
    3. consolidated data + primary data is fed to
        a. indicator handler

    4. data is added to the buffer

    5. each buffer iteration (duration) before sending the buffer to the engine or strategy, we update consolidator time again.
        to see if we have any closed bars, if we do we run the indicator update again.

    6. All base data is added to a TimeSlice and the timeslice is added to the event slice

    7. the buffer loop sleeps until the next scheduled instant while it is refilled by the streams and the process repeats.
*/
pub async fn response_handler(
    mode: StrategyMode,
    buffer_duration: Duration,
    settings_map: HashMap<ConnectionType, ConnectionSettings>,
    notify: Arc<Notify>,
    server_receivers: DashMap<ConnectionType, ReadHalf<TlsStream<TcpStream>>>,
    callbacks: Arc<DashMap<u64, oneshot::Sender<DataServerResponse>>>,
) {
    let callbacks = callbacks.clone();
    let event_buffer = Arc::new(RwLock::new(EventTimeSlice::new()));
    let open_bars: Arc<DashMap<DataSubscription, AHashMap<DateTime<Utc>, BaseDataEnum>>> = Arc::new(DashMap::new());
    let time_slice = Arc::new(RwLock::new(TimeSlice::new()));
    if mode == StrategyMode::Live || mode == StrategyMode::LivePaperTrading {
        let event_buffer_ref = event_buffer.clone();
        let time_slice_ref = time_slice.clone();
        let open_bars_ref = open_bars.clone();
        let subscription_handler = SUBSCRIPTION_HANDLER.get().unwrap().clone();
        let indicator_handler = INDICATOR_HANDLER.get().unwrap().clone();
        tokio::task::spawn(async move {
            subscription_handler.strategy_subscriptions().await;
            let mut instant = Instant::now() + buffer_duration;
            loop {
                sleep_until(instant.into()).await;
                { //we use a block here so if we await notified the buffer can keep filling up as we will drop locks
                    let mut buffer = event_buffer_ref.write().await;
                    let mut time_slice = time_slice_ref.write().await;
                    {
                        for mut map in open_bars_ref.iter_mut() {
                            for (_, data) in &*map {
                                time_slice.push(data.clone());
                            }
                            map.clear();
                        }
                    }
                    if let Some(remaining_time_slice) = subscription_handler.update_consolidators_time(Utc::now()).await {
                        if let Some(indicator_events) = indicator_handler.as_ref().update_time_slice(&remaining_time_slice).await {
                            buffer.extend(indicator_events);
                        }
                        time_slice.extend(remaining_time_slice);
                    }

                    let slice = StrategyEvent::TimeSlice(Utc::now().to_string(), time_slice.clone());
                    *time_slice = TimeSlice::new();

                    buffer.push(slice);
                    if !buffer.is_empty() {
                        send_strategy_event_slice(buffer.clone()).await;
                        *buffer = EventTimeSlice::new();
                    }
                    instant = Instant::now() + buffer_duration;
                }
                notify.notified().await;
            }
        });
    }


    for (connection, settings) in &settings_map {
        if let Some((connection, stream)) = server_receivers.remove(connection) {
            let register_message = StrategyRequest::OneWay(connection.clone(), DataServerRequest::Register(mode.clone()));
            send_request(register_message).await;
            let mut receiver = stream;
            let callbacks = callbacks.clone();
            let subscription_handler = SUBSCRIPTION_HANDLER.get().unwrap().clone();
            let strategy_subscriptions = subscription_handler.strategy_subscriptions().await;
            let event_buffer = event_buffer.clone();
            let time_slice = time_slice.clone();
            let open_bars = open_bars.clone();
            let indicator_handler = INDICATOR_HANDLER.get().unwrap().clone();
            tokio::task::spawn(async move {
                let subscription_handler = SUBSCRIPTION_HANDLER.get().unwrap().clone(); //todo this needs to exist before this fn is called, put response handler in own fn
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
                    let callbacks = callbacks.clone();
                    let strategy_subscriptions = strategy_subscriptions.clone();
                    let subscription_handler = subscription_handler.clone();
                    let event_buffer = event_buffer.clone();
                    let time_slice = time_slice.clone();
                    let open_bars = open_bars.clone();
                    let indicator_handler = indicator_handler.clone();
                    tokio::task::spawn(async move {
                        let response = DataServerResponse::from_bytes(&message_body).unwrap();
                        match response.get_callback_id() {
                            // if there is no callback id then we add it to the strategy event buffer
                            None => {
                                match response {
                                    DataServerResponse::SubscribeResponse { success, subscription, reason } => {
                                        //determine success or fail and add to the strategy event buffer
                                        match success {
                                            true => {
                                                let event = DataSubscriptionEvent::Subscribed(subscription.clone());
                                                let event_slice = StrategyEvent::DataSubscriptionEvents(vec![event], Utc::now().timestamp());
                                                send_strategy_event_slice(vec![event_slice]).await;
                                            }
                                            false => {
                                                let event = DataSubscriptionEvent::FailedSubscribed(subscription.clone(), reason.unwrap());
                                                let event_slice = StrategyEvent::DataSubscriptionEvents(vec![event], Utc::now().timestamp());
                                                event_buffer.write().await.push(event_slice);
                                            }
                                        }
                                    }
                                    DataServerResponse::UnSubscribeResponse { success, subscription, reason } => {
                                        match success {
                                            true => {
                                                let event = DataSubscriptionEvent::Unsubscribed(subscription);
                                                let event_slice = StrategyEvent::DataSubscriptionEvents(vec![event], Utc::now().timestamp());
                                                event_buffer.write().await.push(event_slice);
                                            }
                                            false => {
                                                let event = DataSubscriptionEvent::FailedUnSubscribed(subscription, reason.unwrap());
                                                let event_slice = StrategyEvent::DataSubscriptionEvents(vec![event], Utc::now().timestamp());
                                                event_buffer.write().await.push(event_slice);
                                            }
                                        }
                                    }
                                    DataServerResponse::DataUpdates(primary_data) => {
                                        MARKET_HANDLER.get().unwrap().update_time_slice(Utc::now(), &primary_data).await;
                                        let mut strategy_time_slice = TimeSlice::new();
                                        if let Some(consolidated) = subscription_handler.update_time_slice(primary_data.clone()).await {
                                            strategy_time_slice.extend(consolidated);
                                        }
                                        if let Some(consolidated) = subscription_handler.update_consolidators_time(Utc::now()).await {
                                            strategy_time_slice.extend(consolidated);
                                        }
                                        for base_data in primary_data {
                                            if strategy_subscriptions.contains(&base_data.subscription()) {
                                                strategy_time_slice.push(base_data);
                                            }
                                        }
                                        if strategy_time_slice.is_empty() {
                                            return;
                                        }

                                        if let Some(indicator_events) = indicator_handler.as_ref().update_time_slice(&strategy_time_slice).await {
                                            event_buffer.write().await.extend(indicator_events);
                                        }

                                        for data in strategy_time_slice {
                                            let subscription = data.subscription();
                                            if data.is_closed() {
                                                time_slice.write().await.push(data);
                                            } else {
                                                let mut bar_map = AHashMap::new();
                                                bar_map.insert(data.time_utc(), data);
                                                open_bars.insert(subscription.clone(), bar_map);
                                            }
                                        }
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
                    });
                }
            });
        }
    }
}

/*pub async fn response_handler(mode: StrategyMode, receiver: mpsc::Receiver<StrategyRequest>, buffer_duration: Duration, settings_map: HashMap<ConnectionType, ConnectionSettings>, notify: Arc<Notify>) {

}*/

pub async fn init_sub_handler(subscription_handler: Arc<SubscriptionHandler>,  event_sender: Sender<EventTimeSlice>, indicator_handler: Arc<IndicatorHandler>,) {
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
    market_handler: Arc<MarketHandler>,
    timed_event_handler: Arc<TimedEventHandler>,
    interaction_handler: Arc<InteractionHandler>,
    drawing_objects_handler: Arc<DrawingObjectHandler>,
) {
    let _ = TIMED_EVENT_HANDLER.get_or_init(|| {
        timed_event_handler
    }).clone();
    let _ = MARKET_HANDLER.get_or_init(|| {
        market_handler
    }).clone();
    let _ = INTERACTION_HANDLER.get_or_init(|| {
        interaction_handler
    }).clone();
    let _ = DRAWING_OBJECTS_HANDLER.get_or_init(|| {
        drawing_objects_handler
    }).clone();
}

pub async fn init_connections(gui_enabled: bool, buffer_duration: Duration, mode: StrategyMode, notify: Arc<Notify>) {
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
        let async_client = match create_async_api_client(&settings).await {
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
    response_handler(mode, buffer_duration, settings_map, notify, server_receivers, callbacks).await;
}
