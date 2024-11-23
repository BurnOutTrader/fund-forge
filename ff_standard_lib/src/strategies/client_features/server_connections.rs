use std::collections::HashMap;
use crate::strategies::client_features::init_clients::create_async_api_client;
use crate::strategies::client_features::connection_settings::client_settings::{initialise_settings, ConnectionSettings};
use crate::messages::data_server_messaging::DataServerResponse;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use lazy_static::lazy_static;
use tokio::io;
use tokio::io::{ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio::sync::mpsc::Sender;
use tokio_rustls::TlsStream;
use crate::strategies::client_features::connection_types::ConnectionType;
use crate::strategies::handlers::indicator_handler::IndicatorHandler;
use crate::standardized_types::enums::StrategyMode;
use crate::strategies::strategy_events::StrategyEvent;
use crate::strategies::handlers::subscription_handler::SubscriptionHandler;
use crate::standardized_types::orders::OrderUpdateEvent;
use crate::strategies::client_features::{request_handler, response_handler};
use crate::strategies::client_features::request_handler::DATA_SERVER_SENDER;
use crate::strategies::ledgers::ledger_service::LedgerService;

lazy_static! {
    static ref WARM_UP_COMPLETE: AtomicBool = AtomicBool::new(false);
    pub(crate) static ref SETTINGS_MAP: Arc<HashMap<ConnectionType, ConnectionSettings>> = Arc::new(initialise_settings().unwrap());
}

#[inline(always)]
pub(crate) fn set_warmup_complete() {
    WARM_UP_COMPLETE.store(true, Ordering::SeqCst);
}
#[inline(always)]
pub(crate) fn is_warmup_complete() -> bool {
    WARM_UP_COMPLETE.load(Ordering::SeqCst)
}

pub(crate) async fn init_connections(
    gui_enabled: bool,
    buffer_duration: Duration,
    mode: StrategyMode,
    order_updates_sender: Sender<(OrderUpdateEvent, DateTime<Utc>)>,
    synchronise_accounts: bool,
    strategy_event_sender: Sender<StrategyEvent>,
    ledger_service: Arc<LedgerService>,
    indicator_handler: Arc<IndicatorHandler>,
    subscription_handler: Arc<SubscriptionHandler>
) {
    let server_receivers: DashMap<ConnectionType, ReadHalf<TlsStream<TcpStream>>> = DashMap::with_capacity(SETTINGS_MAP.len());
    let server_senders: DashMap<ConnectionType, WriteHalf<TlsStream<TcpStream>>> = DashMap::with_capacity(SETTINGS_MAP.len());

    // for each connection type specified in our server_settings.toml we will establish a connection
    for (connection_type, settings) in SETTINGS_MAP.iter() {
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
        server_senders.insert(connection_type.clone(), write_half);
        server_receivers.insert(connection_type.clone(), read_half);
    }
    let (tx, rx) = mpsc::channel(1000);
    let _ = DATA_SERVER_SENDER.get_or_init(|| {
        tx
    }).clone();

    let callbacks: Arc<DashMap<u64, oneshot::Sender<DataServerResponse>>> = Default::default();
    request_handler::request_handler(rx, server_senders, callbacks.clone()).await;
    response_handler::response_handler(mode, buffer_duration, server_receivers, callbacks, order_updates_sender, synchronise_accounts, strategy_event_sender, ledger_service, indicator_handler, subscription_handler).await;
}
