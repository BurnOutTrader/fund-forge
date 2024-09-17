use std::collections::HashMap;
use crate::servers::communications_async::{ExternalReceiver, ExternalSender};
use crate::servers::init_clients::{create_async_api_client};
use crate::servers::settings::client_settings::{initialise_settings, ConnectionSettings};
use crate::standardized_types::data_server_messaging::{DataServerRequest, DataServerResponse, FundForgeError, StreamRequest, StreamResponse};
use heck::ToPascalCase;
use lazy_static::lazy_static;
use serde_derive::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use ahash::AHashMap;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use futures::SinkExt;
use strum_macros::Display;
use tokio::io;
use once_cell::sync::OnceCell;
use tokio::io::{AsyncReadExt, ReadHalf};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
use tokio::sync::mpsc::Sender;
use tokio_rustls::TlsStream;
use crate::apis::brokerage::broker_enum::Brokerage;
use crate::apis::data_vendor::datavendor_enum::DataVendor;
use crate::drawing_objects::drawing_object_handler::DrawingObjectHandler;
use crate::indicators::indicator_handler::{IndicatorEvents, IndicatorHandler};
use crate::indicators::values::IndicatorValues;
use crate::interaction_handler::InteractionHandler;
use crate::market_handler::market_handlers::MarketHandler;
use crate::servers::internal_broadcaster::StaticInternalBroadcaster;
use crate::standardized_types::accounts::ledgers::{AccountId};
use crate::standardized_types::Price;
use crate::standardized_types::accounts::position::{Position, PositionId};
use crate::standardized_types::enums::StrategyMode;
use crate::standardized_types::orders::orders::{OrderRequest};
use crate::standardized_types::strategy_events::{EventTimeSlice};
use crate::standardized_types::subscription_handler::SubscriptionHandler;
use crate::standardized_types::subscriptions::{DataSubscription};
use crate::standardized_types::time_slices::TimeSlice;
use crate::timed_events_handler::TimedEventHandler;

pub const GUI_ENABLED: bool = true;
pub const GUI_DISABLED: bool = false;
const DEFAULT: ConnectionType = ConnectionType::Default;

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

/*
    1. primary data comes from either backtest engine or server stream
    2. primary data is broadcast to PRIMARY_DATA_BROADCASTER subscribers
        a. subscription handler
        b. indicator handler
    3. consolidated data is broadcast to subscribers
        a indicator handler

    4. each buffer iteration before sending the buffer to the engine or strategy, we update consolidator time.
        consolidator and indicator handler can return empty vec so we only add to slice if !is_empty(),
        The reason for returning empty buffer is so that we can block until the handlers have cycled the last data input,
        since we always expect Some to return. this allows async timings with concurrent data updates.

*/

// Connections
lazy_static! {
    static ref MAX_SERVERS: usize = initialise_settings().unwrap().len();
    static ref ASYNC_OUTGOING:  DashMap<ConnectionType, Arc<ExternalSender>> = DashMap::with_capacity(*MAX_SERVERS);
    static ref ASYNC_INCOMING: DashMap<ConnectionType, Arc<Mutex<ReadHalf<TlsStream<TcpStream>>>>> = DashMap::with_capacity(*MAX_SERVERS);
    static ref PRIMARY_DATA_BROADCASTER: Arc<StaticInternalBroadcaster<TimeSlice>> = Arc::new(StaticInternalBroadcaster::new());
    static ref TIME_BROADCASTER_BROADCASTER: Arc<StaticInternalBroadcaster<DateTime<Utc>>> = Arc::new(StaticInternalBroadcaster::new());

    // Account Objects
    static ref CASH_BALANCE: DashMap<Brokerage, DashMap<AccountId, Price>> = DashMap::new();
    static ref MARGIN_USED: DashMap<Brokerage, DashMap<AccountId, Price>> = DashMap::new();
    static ref CASH_AVAILABLE: DashMap<Brokerage, DashMap<AccountId, Price>> = DashMap::new();
    static ref ORDERS_FILLED: DashMap<Brokerage, DashMap<AccountId, Price>> = DashMap::new();
    static ref ORDERS_OPEN: DashMap<Brokerage, DashMap<AccountId, Price>> = DashMap::new();
    static ref POSITIONS: DashMap<Brokerage, DashMap<AccountId, DashMap<PositionId, Position>>> = DashMap::new();
    static ref POSITIONS_CLOSED: DashMap<Brokerage, DashMap<AccountId, DashMap<PositionId, Position>>> = DashMap::new();
    static ref PRIMARY_SUBSCRIPTIONS: Arc<Mutex<Vec<DataSubscription>>> = Arc::new(Mutex::new(Vec::new()));
}
static DATA_SERVER_SENDER: OnceCell<Arc<Mutex<mpsc::Sender<StrategyRequest>>>> = OnceCell::new();
static STRATEGY_SENDER: OnceCell<Arc<Mutex<mpsc::Sender<EventTimeSlice>>>> = OnceCell::new();
static SUBSCRIPTION_HANDLER: OnceCell<Arc<SubscriptionHandler>> = OnceCell::new();
static INDICATOR_HANDLER: OnceCell<Arc<IndicatorHandler>> = OnceCell::new();
static MARKET_HANDLER: OnceCell<Arc<MarketHandler>> = OnceCell::new();
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
}

pub(crate) fn get_sender() -> Arc<Mutex<mpsc::Sender<StrategyRequest>>> {
    DATA_SERVER_SENDER.get().unwrap().clone() // Return a clone of the Arc to avoid moving the value out of the OnceCell
}

pub fn get_strategy_sender() -> Arc<Mutex<mpsc::Sender<EventTimeSlice>>> {
    STRATEGY_SENDER.get().unwrap().clone()
}

pub async fn subscribe_consolidated_time_slice(sender: mpsc::Sender<TimeSlice>) {
    SUBSCRIPTION_HANDLER.get().unwrap().clone().subscribe_consolidated_timeslices(sender).await // Return a clone of the Arc to avoid moving the value out of the OnceCell
}

pub async fn subscribe_primary_subscription_updates(sender: mpsc::Sender<Vec<DataSubscription>>) {
    SUBSCRIPTION_HANDLER.get().unwrap().clone().subscribe_primary_subscription_updates(sender).await // Return a clone of the Arc to avoid moving the value out of the OnceCell
}

pub async fn subscribe_all_subscription_updates(sender: mpsc::Sender<Vec<DataSubscription>>) {
    SUBSCRIPTION_HANDLER.get().unwrap().clone().subscribe_secondary_subscription_updates(sender).await // Return a clone of the Arc to avoid moving the value out of the OnceCell
}

pub async fn subscribe_indicator_values(sender: mpsc::Sender<Vec<IndicatorValues>>) {
    INDICATOR_HANDLER.get().unwrap().clone().subscribe_values(sender).await
}

pub async fn subscribe_indicator_events(sender: mpsc::Sender<Vec<IndicatorEvents>>) {
    INDICATOR_HANDLER.get().unwrap().clone().subscribe_events(sender).await
}

pub async fn subscribe_markets(sender: Sender<EventTimeSlice>) {
    MARKET_HANDLER.get().unwrap().clone().subscribe_events(sender).await
}

pub(crate) async fn subscribe_primary_feed(sender: mpsc::Sender<TimeSlice>) {
    PRIMARY_DATA_BROADCASTER.subscribe(sender).await;
}
pub(crate) async fn subscribe_strategy_time(sender: mpsc::Sender<DateTime<Utc>>) {
    TIME_BROADCASTER_BROADCASTER.subscribe(sender).await;
}

pub async fn broadcast_primary_data(data: TimeSlice) {
    PRIMARY_DATA_BROADCASTER.broadcast(data).await;
}

pub async fn broadcast_time(time: DateTime<Utc>) {
    TIME_BROADCASTER_BROADCASTER.broadcast(time).await;
}

pub(crate) enum StrategyRequest {
    CallBack(ConnectionType, DataServerRequest, oneshot::Sender<DataServerResponse>),
    OneWay(ConnectionType, DataServerRequest),
    Orders(ConnectionType, OrderRequest)
}

//Notes
//todo, make all handlers event driven.. we will need to use senders and receivers.
// 1. Subscriptions will be handled by the subscription handler, it will only send subscription request if it needs a new primary, it will alo cancel existing if need be.

async fn live_subscription_handler(mode: StrategyMode, subscription_update_channel: mpsc::Receiver<Vec<DataSubscription>>) {
    if mode == StrategyMode::Live || mode == StrategyMode::LivePaperTrading {
        let mut subscription_update_channel = subscription_update_channel;
        let current_subscriptions = PRIMARY_SUBSCRIPTIONS.clone();
        tokio::task::spawn(async move {
            while let Some(updated_subscriptions) = subscription_update_channel.recv().await {
                let mut current_subscriptions = current_subscriptions.lock().await;
                let mut requests_map = AHashMap::new();
                if *current_subscriptions != updated_subscriptions {
                    for subscription in &updated_subscriptions {
                        if !current_subscriptions.contains(&subscription) {
                            let connection = ConnectionType::Vendor(subscription.symbol.data_vendor.clone());
                            let request = DataServerRequest::StreamRequest { request: StreamRequest::Subscribe(subscription.clone())};
                            if !requests_map.contains_key(&connection) {
                                requests_map.insert(connection, vec![request]);
                            } else {
                                requests_map.get_mut(&connection).unwrap().push(request);
                            }
                        }
                    }
                    for subscription in &*current_subscriptions {
                        if !updated_subscriptions.contains(&subscription) {
                            let connection = ConnectionType::Vendor(subscription.symbol.data_vendor.clone());
                            let request = DataServerRequest::StreamRequest { request: StreamRequest::Unsubscribe(subscription.clone())};
                            if !requests_map.contains_key(&connection) {
                                requests_map.insert(connection, vec![request]);
                            } else {
                                requests_map.get_mut(&connection).unwrap().push(request);
                            }
                        }
                    }
                    for (connection, requests) in requests_map {
                        for request in requests {
                            send(connection.clone(), request.to_bytes()).await;
                        }
                    }
                    *current_subscriptions = updated_subscriptions.clone();
                }
            }
        });
    }
}
/// This response handler is also acting as a live engine.
async fn request_handler(receiver: mpsc::Receiver<StrategyRequest>, buffer_duration: Option<Duration>, settings_map: HashMap<ConnectionType, ConnectionSettings>)  {
    let mut receiver = receiver;
    let callbacks: Arc<RwLock<AHashMap<u64, oneshot::Sender<DataServerResponse>>>> = Default::default();
    let connection_map = Arc::new(settings_map);
    let callbacks_ref = callbacks.clone();
    tokio::task::spawn(async move {
        let mut callback_id_counter: u64 = 0;
        let mut callbacks = callbacks_ref.clone();
        //println!("Request handler start");
        while let Some(outgoing_message) = receiver.recv().await {
            let connection_map = connection_map.clone();
            match outgoing_message {
                StrategyRequest::CallBack(connection_type, mut request, oneshot) => {
                    callback_id_counter += 1;
                    let id = callback_id_counter.clone();
                    let callbacks = callbacks.clone();
                    request.set_callback_id(id.clone());
                    callbacks.write().await.insert(id, oneshot);
                    let connection_type = match connection_map.contains_key(&connection_type) {
                        true => connection_type,
                        false => ConnectionType::Default
                    };
                    //println!("{}: request_received: {:?}", connection_type, request);
                    send(connection_type, request.to_bytes()).await;
                }
                StrategyRequest::OneWay(connection_type, mut request) => {
                    tokio::task::spawn(async move {
                        let connection_type = match connection_map.contains_key(&connection_type) {
                            true => connection_type,
                            false => ConnectionType::Default
                        };
                        //println!("{}: request_received: {:?}", connection_type, request);
                        send(connection_type, request.to_bytes()).await;
                    });
                }
                StrategyRequest::Orders(connection_type, request) => {
                    tokio::task::spawn(async move {
                        let brokerage = request.brokerage();
                        let request = DataServerRequest::OrderRequest {
                            request
                        };
                        let connection_type= ConnectionType::Broker(brokerage);
                        let connection_type = match connection_map.contains_key(&connection_type) {
                            true => connection_type,
                            false => ConnectionType::Default
                        };
                        //println!("{}: request_received: {:?}", connection_type, request);
                        send(connection_type, request.to_bytes()).await;
                    });
                }
            }
        }
        //println!("request handler end");
    });

    let callbacks = callbacks.clone();
    for incoming in ASYNC_INCOMING.iter() {
        let receiver = incoming.clone();
        let callbacks = callbacks.clone();
        tokio::task::spawn(async move {
            let mut receiver = receiver.lock().await;
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
                let callbacks = callbacks.clone();
                let response = DataServerResponse::from_bytes(&message_body).unwrap();
                //println!("{:?}", response);
                match response.get_callback_id() {
                    None => {
                        let stream_response = response.stream_response().unwrap();
                        match stream_response {
                            StreamResponse::BaseData(base_data) => {},
                            StreamResponse::AccountState(_, _, _) => {},
                            StreamResponse::OrderUpdates(update) => {},
                            StreamResponse::PositionUpdates(_) => {},
                        }
                    }
                    Some(id) => {
                        if let Some(callback) = callbacks.write().await.remove(&id) {
                            let _ = callback.send(response);
                        }
                    }
                }
            }
            //println!("response handler end");
        });
    }
}

async fn send(connection_type: ConnectionType, msg: Vec<u8>) {
    let sender = ASYNC_OUTGOING.get(&connection_type).unwrap_or_else(|| ASYNC_OUTGOING.get(&ConnectionType::Default).unwrap()).value().clone();
    match sender.send(&msg).await {
        Ok(_) => {}
        Err(e) => eprintln!("{}", e)
    }
}

pub async fn init_sub_handler(subscription_handler: Arc<SubscriptionHandler>) {
    SUBSCRIPTION_HANDLER.get_or_init(|| {
        subscription_handler
    }).clone();
}
pub async fn initialize_static(
    mode: StrategyMode,
    event_sender: mpsc::Sender<EventTimeSlice>,
    indicator_handler: Arc<IndicatorHandler>,
    market_handler: Arc<MarketHandler>,
    timed_event_handler: Arc<TimedEventHandler>,
    interaction_handler: Arc<InteractionHandler>,
    drawing_objects_handler: Arc<DrawingObjectHandler>,
) {

    STRATEGY_SENDER.get_or_init(|| {
        Arc::new(Mutex::new(event_sender))
    }).clone();
    INDICATOR_HANDLER.get_or_init(|| {
        indicator_handler
    }).clone();
    TIMED_EVENT_HANDLER.get_or_init(|| {
        timed_event_handler
    }).clone();
    MARKET_HANDLER.get_or_init(|| {
        market_handler
    }).clone();
    INTERACTION_HANDLER.get_or_init(|| {
        interaction_handler
    }).clone();
    DRAWING_OBJECTS_HANDLER.get_or_init(|| {
        drawing_objects_handler
    }).clone();

    match mode {
        StrategyMode::Backtest => {}
        StrategyMode::Live | StrategyMode::LivePaperTrading => {
            let (tx, rx) = mpsc::channel(100);
            subscribe_primary_subscription_updates(tx).await;
            live_subscription_handler(mode, rx).await
        },
    }
}

pub async fn init_connections(gui_enabled: bool, buffer_duration: Option<Duration>,) {
    //initialize_strategy_registry(platform_mode.clone()).await;
    let settings_map = initialise_settings().unwrap();
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
        ASYNC_OUTGOING.insert(connection_type.clone(), Arc::new(async_sender));
        ASYNC_INCOMING.insert(connection_type.clone(), Arc::new(Mutex::new(read_half)));
    }
    let (tx, rx) = mpsc::channel(1000);
    request_handler(rx, buffer_duration, settings_map).await;

    DATA_SERVER_SENDER.get_or_init(|| {
        Arc::new(Mutex::new(tx))
    }).clone();
}
