use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use ahash::AHashMap;
use chrono::{DateTime, Datelike, Local, TimeZone, Timelike, Utc, Weekday};
use chrono_tz::America::Chicago;
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use futures::future::join_all;
use futures::stream::{SplitSink, SplitStream};
#[allow(unused_imports)]
use structopt::lazy_static::lazy_static;
use tokio::sync::{broadcast, oneshot, Mutex};
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::enums::{FuturesExchange, MarketType, OrderSide, PositionSide, StrategyMode};
use ff_standard_lib::standardized_types::orders::{Order, OrderId, OrderType, OrderUpdateEvent, OrderUpdateType, TimeInForce};
use ff_standard_lib::standardized_types::subscriptions::{Symbol, SymbolName};
use ff_standard_lib::standardized_types::symbol_info::{FrontMonthInfo};
use ff_standard_lib::standardized_types::books::BookLevel;
use ff_standard_lib::standardized_types::accounts::AccountId;
use ff_standard_lib::StreamName;
use prost::Message as ProstMessage;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;
use tokio::net::TcpStream;
use tokio::{select, task};
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep_until, timeout, Instant};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::{Message};
use ff_standard_lib::apis::rithmic::rithmic_systems::RithmicSystem;
use crate::server_features::server_side_datavendor::VendorApiResponse;
use ff_standard_lib::standardized_types::accounts::AccountInfo;
use ff_standard_lib::standardized_types::new_types::Volume;
use ff_standard_lib::standardized_types::position::PositionId;
use uuid::Uuid;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use crate::{get_data_folder, rithmic_api, subscribe_server_shutdown};
use crate::rithmic_api::client_base::api_base::{RithmicApiClient, TEMPLATE_VERSION};
use crate::rithmic_api::client_base::credentials::RithmicCredentials;
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_login::SysInfraType;
#[allow(unused_imports)]
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::{request_tick_bar_replay, RequestAccountList, RequestAccountRmsInfo, RequestFrontMonthContract, RequestHeartbeat, RequestNewOrder, RequestPnLPositionUpdates, RequestReferenceData, RequestShowOrders, RequestSubscribeForOrderUpdates, RequestTickBarReplay, RequestTimeBarReplay, RequestTradeRoutes};
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_new_order::{OrderPlacement, PriceType, TransactionType};
use crate::rithmic_api::plant_handlers::handler_loop::handle_rithmic_responses;
use ff_standard_lib::product_maps::rithmic::maps::{get_exchange_by_symbol_name};
use once_cell::sync::OnceCell;
use ff_standard_lib::server_launch_options::ServerLaunchOptions;
use ff_standard_lib::standardized_types::resolution::Resolution;
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_time_bar_replay::{Direction, TimeOrder};

lazy_static! {
    pub static ref RITHMIC_CLIENTS: DashMap<RithmicSystem , Arc<RithmicBrokerageClient>> = DashMap::with_capacity(16);
    pub static ref RITHMIC_DATA_IS_CONNECTED: AtomicBool = AtomicBool::new(false);
}

static MARKET_DATA_SYSTEM: OnceCell<RithmicSystem> = OnceCell::new();

pub fn get_rithmic_market_data_system() -> Option<RithmicSystem> {
    match MARKET_DATA_SYSTEM.get() {
        Some(system) => Some(system.clone()),
        None => None
    }
}

// We do not want to initialize here, that should be done at server launch, else a strategy could sign out the client of the correct server.
pub fn get_rithmic_client(rithmic_system: &RithmicSystem) -> Option<Arc<RithmicBrokerageClient>> {
    if let Some(client) = RITHMIC_CLIENTS.get(&rithmic_system) {
        return Some(client.value().clone())
    }
    None
}

//todo make a seperate client for data, so we arent initializing pointless maps, this will also make it much more maintainable
pub struct RithmicBrokerageClient {
    pub brokerage: Brokerage,
    pub data_vendor: DataVendor,
    pub system: RithmicSystem,
    pub fcm_id: Option<String>,
    pub ib_id: Option<String>,
    pub user_type: Option<i32>,
    pub credentials: RithmicCredentials,

    pub callbacks: DashMap<StreamName, AHashMap<u64, oneshot::Sender<DataServerResponse>>>,
    pub callback_id: Arc<Mutex<u64>>,
    pub writers: DashMap<SysInfraType, Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>>,
    pub heartbeat_times: Arc<DashMap<SysInfraType, DateTime<Utc>>>,
    pub heartbeat_latency: Arc<DashMap<SysInfraType, i64>>,
    pub heartbeat_tasks: Arc<DashMap<SysInfraType, JoinHandle<()>>>,
    /// Rithmic clients
    pub client: Arc<RithmicApiClient>,
    pub front_month_info: Arc<DashMap<SymbolName, FrontMonthInfo>>,

    // accounts
    pub account_info: DashMap<AccountId, AccountInfo>,
    pub account_balance: DashMap<AccountId, Decimal>,
    pub account_cash_available: DashMap<AccountId, Decimal>,
    pub account_cash_used: DashMap<AccountId, Decimal>,
    pub margin_used: DashMap<AccountId, Decimal>,
    pub margin_available: DashMap<AccountId, Decimal>,
    pub open_pnl: DashMap<AccountId, Decimal>,
    pub closed_pnl: DashMap<AccountId, Decimal>,
    pub max_size: DashMap<AccountId, Volume>,
    pub long_quantity: DashMap<AccountId, DashMap<SymbolName, Volume>>,
    pub short_quantity: DashMap<AccountId, DashMap<SymbolName, Volume>>,
    pub last_tag: DashMap<AccountId, DashMap<SymbolName, String>>,

    pub open_orders: DashMap<AccountId, DashMap<OrderId, Order>>,
    pub id_to_basket_id_map: DashMap<AccountId, DashMap<OrderId, String>>,
    pub pending_order_updates: DashMap<Brokerage, DashMap<OrderId , OrderUpdateType>>,

    pub orders_open: DashMap<OrderId, Order>,

    //products
    pub products: DashMap<MarketType, Vec<Symbol>>,
    pub historical_callbacks: DashMap<u64, oneshot::Sender<BTreeMap<DateTime<Utc>,BaseDataEnum>>>,

    //todo, since only 1 connection is used for data this could all be moved, we could have a rithmic data client and a rithmic broker client
    //subscribers
    pub tick_feed_broadcasters: Arc<DashMap<SymbolName, broadcast::Sender<BaseDataEnum>>>,
    pub quote_feed_broadcasters: Arc<DashMap<SymbolName, broadcast::Sender<BaseDataEnum>>>,
    pub candle_feed_broadcasters: Arc<DashMap<SymbolName, broadcast::Sender<BaseDataEnum>>>,

    // first string is fcm id second is trade route
    pub default_trade_route: DashMap<RithmicSystem, AHashMap<(String, FuturesExchange), String>>,

    pub bid_book: DashMap<SymbolName, BTreeMap<u16, BookLevel>>,
    pub ask_book: DashMap<SymbolName, BTreeMap<u16, BookLevel>>,

    pub order_broadcaster: broadcast::Sender<DataServerResponse>,
}

impl RithmicBrokerageClient {
    pub async fn new(
        system: RithmicSystem,
    ) -> Result<Self, FundForgeError> {
        let brokerage = Brokerage::Rithmic(system.clone());
        let data_vendor = DataVendor::Rithmic;
        let credentials = RithmicBrokerageClient::rithmic_credentials(&brokerage)?;
        println!("Activating {} {} on Rithmic Server: {}, Template Version: {}", credentials.user, credentials.system_name, credentials.server_name, TEMPLATE_VERSION);
        let data_folder = get_data_folder();
        let server_domains_toml = PathBuf::from(data_folder)
            .join("credentials")
            .join("rithmic_credentials")
            .join("server_domains")
            .join("servers.toml")
            .to_string_lossy()
            .into_owned();

        let client = RithmicApiClient::new(credentials.clone(), server_domains_toml).unwrap();
        let (sender, _) = broadcast::channel(500);
        let client = Self {
            brokerage,
            data_vendor,
            system,
            fcm_id: credentials.fcm_id.clone(),
            ib_id: credentials.ib_id.clone(),
            user_type: credentials.user_type.clone(),
            credentials,
            callbacks: Default::default(),
            callback_id: Arc::new(Mutex::new(0)),
            writers: DashMap::with_capacity(5),
            heartbeat_times: Arc::new(DashMap::with_capacity(5)),
            heartbeat_latency: Arc::new(DashMap::with_capacity(5)),
            heartbeat_tasks: Arc::new(DashMap::with_capacity(5)),
            client: Arc::new(client),
            front_month_info: Default::default(),
            account_info: Default::default(),
            account_balance: Default::default(),
            account_cash_available: Default::default(),
            account_cash_used: Default::default(),
            margin_used: Default::default(),
            margin_available: Default::default(),
            open_pnl: Default::default(),
            closed_pnl: Default::default(),
            max_size: Default::default(),
            tick_feed_broadcasters: Arc::new(Default::default()),
            quote_feed_broadcasters: Arc::new(Default::default()),
            bid_book: Default::default(),

            orders_open: Default::default(),
            products: Default::default(),
            candle_feed_broadcasters: Arc::new(Default::default()),
            ask_book: Default::default(),
            order_broadcaster: sender,
            long_quantity: Default::default(),
            short_quantity: Default::default(),
            default_trade_route: DashMap::new(),
            last_tag: Default::default(),
            open_orders: Default::default(),
            id_to_basket_id_map: Default::default(),
            pending_order_updates: Default::default(),
            historical_callbacks: Default::default(),
        };

        Ok(client)
    }

    pub fn generate_id(
        &self,
        side: PositionSide,
    ) -> PositionId {
        // Generate a UUID v4 (random)
        let guid = Uuid::new_v4();

        // Return the generated position ID with both readable prefix and GUID
        format!(
            "{}-{}",
            side,
            guid.to_string()
        )
    }


    pub fn get_rithmic_tomls() -> Vec<String> {
        let mut toml_files = Vec::new();
        let dir = PathBuf::from(get_data_folder())
            .join("credentials")
            .join("rithmic_credentials")
            .join("active")
            .to_string_lossy()
            .into_owned();

        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("toml") {
                if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                    toml_files.push(file_name.to_string());
                }
            }
        }

        toml_files
    }

    pub async fn connect_plant(&self, system: SysInfraType) -> Result<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>, FundForgeError> {
        match self.client.connect_and_login(system.clone()).await {
            Ok(r) => {
                let (writer, receiver) = r.split();
                let writer = Arc::new(Mutex::new(writer));
                self.writers.insert(system.clone(), writer.clone());
                self.heartbeat_times.insert(system.clone(), Utc::now());
                self.start_heart_beat(system, writer, self.heartbeat_times.clone()).await;
                Ok(receiver)
            },
            Err(e) => {
                Err(FundForgeError::ServerErrorDebug(e.to_string()))
            }
        }
    }

    pub async fn return_callback(&self, stream_name: StreamName, callback_id: u64, response: DataServerResponse) {
        let mut disconnected = false;
        if let Some(mut stream_map) = self.callbacks.get_mut(&stream_name) {
            if let Some(sender) = stream_map.value_mut().remove(&callback_id) {
                match sender.send(response) {
                    Ok(_) => {}
                    Err(e) => {
                        disconnected = true;
                        eprintln!("Callback error: {:?} Dumping subscriber: {}", e, stream_name);
                    }
                }
            }
        }
        if disconnected {
            self.logout_command_vendors(stream_name).await;
        }
    }

    pub async fn register_callback_and_send<T: ProstMessage>(&self, plant: &SysInfraType, stream_name: StreamName, callback_id: u64, sender: oneshot::Sender<DataServerResponse>, request: T) {
        if let Some(mut stream_map) = self.callbacks.get_mut(&stream_name) {
            stream_map.value_mut().insert(callback_id, sender);
        } else {
            let mut map = AHashMap::new();
            map.insert(callback_id, sender);
            self.callbacks.insert(stream_name.clone(), map);
        }
        self.send_message(plant, request).await;
    }

    pub async fn generate_callback_id(&self) -> u64 {
        let mut callback_id= self.callback_id.lock().await;
        *callback_id = callback_id.wrapping_add(1);
        callback_id.clone()
    }

    fn rithmic_credentials(broker: &Brokerage) -> Result<RithmicCredentials, FundForgeError> {
        match broker {
            Brokerage::Rithmic(system) => {
                let file = system.file_string();
                let data_folder = match get_data_folder().to_str() {
                    Some(s) => s.to_string(),
                    None => String::from("Invalid UTF-8 sequence"), // Handle the error case as needed
                };
                let file_path = format!("{}/credentials/rithmic_credentials/active/{}", data_folder, file);
                match RithmicCredentials::load_credentials_from_file(&file_path) {
                    Ok(file) => Ok(file),
                    Err(_e) => Err(FundForgeError::ServerErrorDebug(format!("Failed to load credentials for: {}", broker)))
                }
            },
            _ => Err(FundForgeError::ServerErrorDebug(format!("{} Incorrect brokerage to load rithmic credentials", broker)))
        }
    }

    pub async fn send_message<T: ProstMessage>(
        &self,
        plant: &SysInfraType,
        message: T
    ) {
        let mut buf = Vec::new();
        match message.encode(&mut buf).map_err(|e| format!("Failed to encode message: {}", e)) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Failed to encode rithmic proto message: {}", e);
                return;
            }
        }

        let length = buf.len() as u32;
        let mut prefixed_msg = length.to_be_bytes().to_vec();
        prefixed_msg.extend(buf);

        if let Some(write_stream) = self.writers.get(plant) {
            self.heartbeat_times.insert(plant.clone(), Utc::now());

            let mut write_stream = write_stream.lock().await;
            match write_stream.send(Message::Binary(prefixed_msg.clone())).await {
                Ok(_) => {},
                Err(e) => {
                    match plant {
                        SysInfraType::HistoryPlant | SysInfraType::TickerPlant => {
                            RITHMIC_DATA_IS_CONNECTED.store(false, Ordering::SeqCst);
                        },
                        _ => {}
                    }
                    eprintln!("Failed to send message to {:?}: {}. Retrying...", plant, e)
                },
            }
            match write_stream.flush().await {
                Ok(_) => {}
                Err(_) => {}
            }
        }
    }

    pub async fn shutdown(&self) {
        // shutdown using self.client.shutdown_plant() for each connection
        RITHMIC_CLIENTS.remove(&self.system);
    }

    async fn start_heart_beat(
        &self,
        plant: SysInfraType,
        write_stream: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
        heartbeat_times: Arc<DashMap<SysInfraType, DateTime<Utc>>>
    ) {
        let heart_beat_interval = match self.client.heartbeat_interval_seconds.get(&plant) {
            Some(interval) => Duration::from_secs(interval.value().clone() - 3),
            None => Duration::from_secs(57)
        };

        let mut interval = interval(heart_beat_interval);
        let quote_broadcasters = self.quote_feed_broadcasters.clone();
        let tick_feed_broadcasters = self.tick_feed_broadcasters.clone();
        let candle_broadcasters = self.candle_feed_broadcasters.clone();
        let task = task::spawn(async move {
            let mut shutdown_receiver = subscribe_server_shutdown();
            'heartbeat_loop: loop {
                select! {
                    _ = interval.tick() => {
                        let now = Utc::now();
                        let skip_heartbeat = match plant {
                            SysInfraType::TickerPlant => {
                                !quote_broadcasters.is_empty() || !tick_feed_broadcasters.is_empty()
                            }
                            SysInfraType::HistoryPlant => {
                                !candle_broadcasters.is_empty()
                            }
                            _ => false
                        };

                        if !skip_heartbeat {
                            let request = RequestHeartbeat {
                                template_id: 18,
                                user_msg: vec![],
                                ssboe: Some(now.timestamp() as i32),
                                usecs: Some(now.timestamp_subsec_micros() as i32),
                            };
                            let mut buf = Vec::new();
                            if let Err(e) = request.encode(&mut buf) {
                                eprintln!("Failed to encode message for {:?}: {}", plant, e);
                                continue 'heartbeat_loop;
                            }

                            let length = buf.len() as u32;
                            let mut prefixed_msg = length.to_be_bytes().to_vec();
                            prefixed_msg.extend(buf);

                            let mut write_stream = write_stream.lock().await;
                            match write_stream.send(Message::Binary(prefixed_msg)).await {
                                Ok(_) => {
                                    heartbeat_times.insert(plant.clone(), now);
                                    //eprintln!("Heartbeat sent for {:?} at {:?}", plant, now);
                                },
                                Err(e) => {
                                    eprintln!("Failed to send heartbeat to {:?}: {}", plant, e);
                                    break 'heartbeat_loop;
                                }
                            }
                            heartbeat_times.insert(plant.clone(), now);
                        } else {
                            //eprintln!("Skipping heartbeat for {:?} due to active broadcasters at {:?}", plant, now);
                        }
                    }
                    _ = shutdown_receiver.recv() => {
                        println!("Shutting down heartbeat task for system {:?}", plant);
                        break 'heartbeat_loop;
                    }
                }
            }
        });
        self.heartbeat_tasks.insert(plant, task);
    }

    pub async fn request_updates(&self, account_id: AccountId) {
        if let Some(id_account_info_kvp) = self.account_info.get(&account_id) {
            self.last_tag.insert(account_id.clone(), DashMap::new());
            let req = RequestShowOrders {
                template_id: 320,
                user_msg: vec![],
                fcm_id: self.fcm_id.clone(),
                ib_id: self.ib_id.clone(),
                account_id: Some(id_account_info_kvp.key().clone()),
            };
            //println!("Requesting account: {:?}", req);
            self.send_message(&SysInfraType::OrderPlant, req).await;
            let req = RequestSubscribeForOrderUpdates {
                template_id: 308 ,
                user_msg: vec![],
                fcm_id: self.fcm_id.clone(),
                ib_id: self.ib_id.clone(),
                account_id: Some(id_account_info_kvp.key().clone()),
            };
            //println!("Requesting account: {:?}", req);
            self.send_message(&SysInfraType::OrderPlant, req).await;

            /*let req = RequestAccountList {
                template_id: 302,
                user_msg: vec![],
                fcm_id: self.fcm_id.clone(),
                ib_id: self.ib_id.clone(),
                user_type: self.user_type.clone(),
            };
            self.send_message(&SysInfraType::OrderPlant, req).await;*/

            let req = RequestPnLPositionUpdates {
                template_id: 400,
                user_msg: vec![],
                request: Some(1),
                fcm_id: self.fcm_id.clone(),
                ib_id: self.ib_id.clone(),
                account_id: Some(id_account_info_kvp.key().clone()),
            };
            //println!("Requesting account: {:?}", req);
            self.send_message(&SysInfraType::PnlPlant, req).await;
            
            let req = RequestTradeRoutes {
                template_id: 310,
                user_msg: vec![],
                subscribe_for_updates: Some(true), //todo not sure if we want updates, they never seem to stop.
            };
            //println!("Requesting account: {:?}", req);
            self.send_message(&SysInfraType::OrderPlant, req).await;
        }
    }

    /// Checks that an order will not go over max size
    pub fn is_valid_order(&self, order: &Order) -> Result<(), String> {
        if let Some(max_size) = self.max_size.get(&order.account.account_id) {
            let max_size = *max_size.value();
            let current_total_long = self.long_quantity
                .get(&order.account.account_id)
                .map(|account_map| account_map.iter().fold(dec!(0), |acc, item| acc + *item.value()))
                .unwrap_or_else(|| dec!(0));
            let current_total_short = self.short_quantity
                .get(&order.account.account_id)
                .map(|account_map| account_map.iter().fold(dec!(0), |acc, item| acc + *item.value()))
                .unwrap_or_else(|| dec!(0));

            let new_total = match order.side {
                OrderSide::Buy => {
                    let account_short = self.short_quantity
                        .entry(order.account.account_id.clone())
                        .or_insert_with(DashMap::new);
                    let current_symbol_short = account_short
                        .entry(order.symbol_name.clone())
                        .or_insert(dec!(0));
                    let current_symbol_short_value = *current_symbol_short.value();

                    if order.quantity_open <= current_symbol_short_value {
                        // Closing short position
                        current_total_long + current_total_short - order.quantity_open
                    } else {
                        // Opening long position or flipping from short to long
                        current_total_long + current_total_short + (order.quantity_open - current_symbol_short_value)
                    }
                },
                OrderSide::Sell => {
                    let account_long = self.long_quantity
                        .entry(order.account.account_id.clone())
                        .or_insert_with(DashMap::new);
                    let current_symbol_long = account_long
                        .entry(order.symbol_name.clone())
                        .or_insert(dec!(0));
                    let current_symbol_long_value = *current_symbol_long.value();

                    if order.quantity_open <= current_symbol_long_value {
                        // Closing long position
                        current_total_long + current_total_short - order.quantity_open
                    } else {
                        // Opening short position or flipping from long to short
                        current_total_long + current_total_short + (order.quantity_open - current_symbol_long_value)
                    }
                }
            };

            if new_total > max_size {
                return Err(format!("Order would exceed max total position size of {}: {}", max_size, order.tag));
            }
        }
        Ok(())
    }

    pub async fn start_front_month_cleaner(front_month_info: Arc<DashMap<SymbolName, FrontMonthInfo>>) {
        let mut shutdown_signal = subscribe_server_shutdown();
        tokio::spawn(async move {
            loop {
                let chicago_now = Local::now().with_timezone(&Chicago);
                let today_weekday = chicago_now.weekday();

                let next_close = match today_weekday {
                    Weekday::Fri if chicago_now.hour() >= 16 => Chicago.with_ymd_and_hms(chicago_now.year(), chicago_now.month(), chicago_now.day() + 3, 16, 0, 0).unwrap(),
                    Weekday::Sat => Chicago.with_ymd_and_hms(chicago_now.year(), chicago_now.month(), chicago_now.day() + 2, 16, 0, 0).unwrap(),
                    Weekday::Sun => Chicago.with_ymd_and_hms(chicago_now.year(), chicago_now.month(), chicago_now.day() + 1, 16, 0, 0).unwrap(),
                    _ if chicago_now.hour() >= 16 => Chicago.with_ymd_and_hms(chicago_now.year(), chicago_now.month(), chicago_now.day() + 1, 16, 0, 0).unwrap(),
                    _ => Chicago.with_ymd_and_hms(chicago_now.year(), chicago_now.month(), chicago_now.day(), 16, 0, 0).unwrap(),
                };
                let target = Instant::now() + (next_close - chicago_now).to_std().unwrap();

                tokio::select! {
                    _ = sleep_until(target) => {
                        front_month_info.clear();
                    }
                    _ = shutdown_signal.recv() => {
                        log::info!("Front month cleaner received shutdown signal");
                        break;
                    }
                }
            }
        });
    }

    pub async fn get_front_month(&self, stream_name: StreamName, symbol_name: SymbolName, exchange: FuturesExchange) -> Result<FrontMonthInfo, String> {
        const PLANT: SysInfraType = SysInfraType::TickerPlant;
        if let Some(info) = self.front_month_info.get(&symbol_name) {
            Ok(info.clone())
        } else {
            let id = self.generate_callback_id().await;
            let request = RequestFrontMonthContract {
                template_id: 113,
                user_msg: vec![stream_name.to_string(), id.to_string()],
                symbol: Some(symbol_name),
                exchange: Some(exchange.to_string()),
                need_updates: Some(true),
            };
            let (sender, receiver) = oneshot::channel();
            self.register_callback_and_send(&PLANT, stream_name, id, sender, request).await;

            match timeout(Duration::from_secs(30), receiver).await {
                Ok(receiver_result) => match receiver_result {
                    Ok(response) => match response {
                        DataServerResponse::FrontMonthInfo { info, .. } => {
                            self.front_month_info.insert(info.symbol_name.clone(), info.clone());
                            Ok(info)
                        },
                        _ => Err("Incorrect response received at server callback".to_string())
                    },
                    Err(e) => Err(format!("Receiver error at api callback recv: {}", e))
                },
                Err(_) => Err("Operation timed out after 10 seconds".to_string())
            }
        }
    }


    pub async fn rithmic_order_details(&self, _mode: StrategyMode, stream_name: StreamName, order: &Order) -> Result<CommonRithmicOrderDetails, OrderUpdateEvent> {
        match self.is_valid_order(&order) {
            Err(e) => return Err(RithmicBrokerageClient::reject_order(&order, format!("{}", e))),
            Ok(_) => {}
        }

        let quantity = match order.quantity_open.to_i32() {
            None => {
                return Err(RithmicBrokerageClient::reject_order(&order, "Invalid quantity".to_string()))
            }
            Some(q) => q
        };

        let (symbol_code, exchange): (SymbolName, FuturesExchange) = {
            match get_exchange_by_symbol_name(&order.symbol_name) {
                None => return Err(RithmicBrokerageClient::reject_order(&order, format!("Exchange Not found with {} for {}", order.account.brokerage, order.symbol_name))),
                Some(mut exchange) => {
                    exchange = match &order.exchange {
                        None => exchange,
                        Some(preset) => {
                            match FuturesExchange::from_string(preset) {
                                Err(e) => return Err(RithmicBrokerageClient::reject_order(&order, format!("Error parsing exchange string {} {}", preset, e))),
                                Ok(ex) => ex.clone()
                            }
                        }
                    };
                    match order.symbol_code == order.symbol_name {
                        true => {
                            let front_month = match self.get_front_month(stream_name, order.symbol_name.clone(), exchange.clone()).await {
                                Ok(info) => info,
                                Err(e) => return Err(RithmicBrokerageClient::reject_order(&order, format!("{}", e)))
                            };
                            (front_month.symbol_code, exchange)
                        }
                        false => {
                            (order.symbol_code.clone(), exchange)
                        }
                    }
                }
            }
        };

        let route = match self.default_trade_route.get(&self.system) {
            None => {
                match self.system {
                    RithmicSystem::RithmicPaperTrading | RithmicSystem::TopstepTrader | RithmicSystem::SpeedUp | RithmicSystem::TradeFundrr | RithmicSystem::UProfitTrader | RithmicSystem::Apex | RithmicSystem::MESCapital |
                    RithmicSystem::TheTradingPit | RithmicSystem::FundedFuturesNetwork | RithmicSystem::Bulenox | RithmicSystem::PropShopTrader | RithmicSystem::FourPropTrader | RithmicSystem::FastTrackTrading
                    => "simulator".to_string(),
                    //RithmicSystem::Rithmic01 => "globex".to_string(),
                    _ => return Err(RithmicBrokerageClient::reject_order(&order, format!("Order Route Not found with {} for {}", order.account.brokerage, order.symbol_name)))
                }
            }
            Some(route_map) => {
                let fcm_id = match &self.credentials.fcm_id {
                    None => return Err(RithmicBrokerageClient::reject_order(&order, "Server error: fcm_id not found".to_string())),
                    Some(id) => id.clone()
                };
                if let Some(exchange_route) = route_map.get(&(fcm_id.clone(), exchange.clone())) {
                    exchange_route.clone()
                } else {
                    match self.system {
                        RithmicSystem::RithmicPaperTrading | RithmicSystem::TopstepTrader | RithmicSystem::SpeedUp | RithmicSystem::TradeFundrr | RithmicSystem::UProfitTrader | RithmicSystem::Apex | RithmicSystem::MESCapital |
                        RithmicSystem::TheTradingPit | RithmicSystem::FundedFuturesNetwork | RithmicSystem::Bulenox | RithmicSystem::PropShopTrader | RithmicSystem::FourPropTrader | RithmicSystem::FastTrackTrading
                        => "simulator".to_string(),
                        //RithmicSystem::Rithmic01 => "globex".to_string(),
                        _ => return Err(RithmicBrokerageClient::reject_order(&order, format!("Order Route Not found with {} for {}", order.account.brokerage, order.symbol_name)))
                    }
                }
            },
        };
        //println!("Route: {}", route);

        let transaction_type = match order.side {
            OrderSide::Buy => TransactionType::Buy,
            OrderSide::Sell => TransactionType::Sell,
        };

        self.open_orders.entry(order.account.account_id.clone()).or_insert(DashMap::new()).insert(order.id.clone(), order.clone());

        Ok(CommonRithmicOrderDetails {
            symbol_code,
            exchange,
            transaction_type,
            route,
            quantity,
        })
    }

    fn reject_order(order: &Order, reason: String) -> OrderUpdateEvent {
        OrderUpdateEvent::OrderRejected {
            account: order.account.clone(),
            symbol_name: order.symbol_name.clone(),
            symbol_code: order.symbol_name.clone(),
            order_id: order.id.clone(),
            reason,
            tag: order.tag.clone(),
            time: Utc::now().to_string()
        }
    }

    pub async fn submit_order(&self, stream_name: StreamName, mut order: Order, details: CommonRithmicOrderDetails) -> Result<(), OrderUpdateEvent> {
        let (duration, cancel_at_ssboe, cancel_at_usecs) = match order.time_in_force {
            TimeInForce::IOC => (crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_bracket_order::Duration::Ioc.into(), None, None),
            TimeInForce::FOK => (crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_bracket_order::Duration::Fok.into(), None, None),
            TimeInForce::GTC => (crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_bracket_order::Duration::Gtc.into(), None, None),
            TimeInForce::Day => (crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_bracket_order::Duration::Day.into(), None, None),
            TimeInForce::Time(ref time_stamp) => {
                let cancel_time = match DateTime::<Utc>::from_timestamp(*time_stamp, 0) {
                    Some(dt) => dt,
                    None => return Err(Self::reject_order(&order, format!("Failed to parse time stamp: {}", time_stamp)))
                };

                (crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_bracket_order::Duration::Gtc.into(),
                 Some(cancel_time.timestamp() as i32),
                 Some(cancel_time.timestamp_subsec_micros() as i32))
            }
        };

        // Rest of the function remains the same...
        match order.side {
            OrderSide::Buy => eprintln!("Buying {}" , order.quantity_open),
            OrderSide::Sell => eprintln!("Selling {}" , order.quantity_open),
        }

        let trigger_price = match order.trigger_price {
            None => None,
            Some(price) => {
                match price.to_f64() {
                    None => return Err(Self::reject_order(&order, format!("Failed to parse trigger price: {}", price))),
                    Some(price) => Some(price)
                }
            }
        };

        let limit_price = match order.limit_price {
            None => None,
            Some(price) => {
                match price.to_f64() {
                    None => return Err(Self::reject_order(&order, format!("Failed to parse limit price: {}", price))),
                    Some(price) => Some(price)
                }
            }
        };

        let order_type = match order.order_type {
            OrderType::Limit => 1,
            OrderType::Market => 2,
            OrderType::MarketIfTouched => 5,
            OrderType::StopMarket => 4,
            OrderType::StopLimit => 3,
            OrderType::EnterLong => 2,
            OrderType::EnterShort => 2,
            OrderType::ExitLong => 2,
            OrderType::ExitShort => 2,
        };

        if order.exchange.is_none() {
            order.exchange = Some(details.exchange.to_string());
        }

        let req = RequestNewOrder {
            template_id: 312,
            user_msg: vec![stream_name.to_string(), order.account.account_id.clone(), order.tag.clone(), order.symbol_name.clone(), details.symbol_code.clone()],
            user_tag: Some(order.id.clone()),
            window_name: Some(stream_name.to_string()),
            fcm_id: self.fcm_id.clone(),
            ib_id: self.ib_id.clone(),
            account_id: Some(order.account.account_id.clone()),
            symbol: Some(details.symbol_code.clone()),
            exchange: Some(details.exchange.to_string()),
            quantity: Some(details.quantity),
            price: limit_price,
            trigger_price,
            transaction_type: Some(details.transaction_type.into()),
            duration: Some(duration),
            price_type: Some(order_type),
            trade_route: Some(details.route),
            manual_or_auto: Some(OrderPlacement::Auto.into()),
            trailing_stop: None,
            trail_by_ticks: None,
            trail_by_price_id: None,
            release_at_ssboe: None,
            release_at_usecs: None,
            cancel_at_ssboe,
            cancel_at_usecs,
            cancel_after_secs: None,
            if_touched_symbol: None,
            if_touched_exchange: None,
            if_touched_condition: None,
            if_touched_price_field: None,
            if_touched_price: None,
        };

        if let Some(account_map) = self.last_tag.get(&order.account.account_id) {
            account_map.insert(details.symbol_code, order.tag.clone());
        }
        self.send_message(&SysInfraType::OrderPlant, req).await;
        Ok(())
    }

    pub async fn submit_market_order(&self, stream_name: StreamName, mut order: Order, details: CommonRithmicOrderDetails) {
        let duration = match order.time_in_force {
            TimeInForce::IOC => crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_bracket_order::Duration::Ioc.into(),
            TimeInForce::FOK => crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_bracket_order::Duration::Fok.into(),
            _ => crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_bracket_order::Duration::Fok.into()
        };

        if order.exchange.is_none() {
            order.exchange = Some(details.exchange.to_string());
        }

        let req = RequestNewOrder {
            template_id: 312,
            user_msg: vec![stream_name.to_string(), order.account.account_id.clone(), order.tag.clone(), order.symbol_name.clone(), details.symbol_code.clone()],
            user_tag: Some(order.id.clone()),
            window_name: Some(stream_name.to_string()),
            fcm_id: self.fcm_id.clone(),
            ib_id: self.ib_id.clone(),
            account_id: Some(order.account.account_id.clone()),
            symbol: Some(details.symbol_code.clone()),
            exchange: Some(details.exchange.to_string()),
            quantity: Some(details.quantity),
            price: None,
            trigger_price: None,
            transaction_type: Some(details.transaction_type.into()),
            duration: Some(duration),
            price_type: Some(PriceType::Market.into()),
            trade_route: Some(details.route),
            manual_or_auto: Some(OrderPlacement::Auto.into()),
            trailing_stop: None,
            trail_by_ticks: None,
            trail_by_price_id: None,
            release_at_ssboe: None,
            release_at_usecs: None,
            cancel_at_ssboe: None,
            cancel_at_usecs: None,
            cancel_after_secs: None,
            if_touched_symbol: None,
            if_touched_exchange: None,
            if_touched_condition: None,
            if_touched_price_field: None,
            if_touched_price: None,
        };

        //this is used to update positions when synchronise positions is used
        if let Some(account_map) = self.last_tag.get(&order.account.account_id) {
            account_map.insert(details.symbol_code, order.tag.clone());
        }
        self.send_message(&SysInfraType::OrderPlant, req).await;
    }

    pub(crate) async fn init_rithmic_apis(options: ServerLaunchOptions) {
        let options = options;
        if options.disable_rithmic_server != 0 {
            return;
        }

        let toml_files = RithmicBrokerageClient::get_rithmic_tomls();
        if toml_files.is_empty() {
            return;
        }

        // First, find the system to use for history and ticker plants
        let market_data_system = Arc::new(
            toml_files.iter()
                .find(|file| {
                    matches!(
                RithmicSystem::from_file_string(file),
                Some(RithmicSystem::Rithmic04Colo)
            )
                })
                .or_else(|| toml_files.iter().find(|file| {
                    matches!(
                RithmicSystem::from_file_string(file),
                Some(RithmicSystem::Rithmic01)
            )
                }))
                .or_else(|| toml_files.first())
                .expect("We checked for empty earlier")
                .clone()
        );

        MARKET_DATA_SYSTEM.get_or_init(||
            RithmicSystem::from_file_string(&market_data_system)
                .expect("Failed to get_requests RithmicSystem from file string")
        );

        let init_tasks = toml_files.into_iter().filter_map(|file| {
            let market_data_system = Arc::clone(&market_data_system);
            RithmicSystem::from_file_string(file.as_str()).map(|system| {
                task::spawn(async move {

                    match RithmicBrokerageClient::new(system).await {
                        Ok(client) => {
                            let client = Arc::new(client);

                            // Always connect OrderPlant and PnlPlant
                            for plant_type in [SysInfraType::OrderPlant, SysInfraType::PnlPlant] {
                                match client.connect_plant(plant_type).await {
                                    Ok(receiver) => {
                                        handle_rithmic_responses(client.clone(), receiver, plant_type);
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to connect {:?} for system {}, reason: {}",
                                                  plant_type, system, e);
                                    }
                                }
                            }

                            // Only connect TickerPlant and HistoryPlant for the chosen system
                            if file == *market_data_system {
                                for plant_type in [SysInfraType::TickerPlant, SysInfraType::HistoryPlant] {
                                    match client.connect_plant(plant_type).await {
                                        Ok(receiver) => {
                                            handle_rithmic_responses(client.clone(), receiver, plant_type);
                                            RITHMIC_DATA_IS_CONNECTED.store(true, Ordering::SeqCst);
                                        }
                                        Err(e) => {
                                            eprintln!("Failed to connect {:?} for system {}, reason: {}",
                                                      plant_type, system, e);
                                            RITHMIC_DATA_IS_CONNECTED.store(false, Ordering::SeqCst);
                                        }
                                    }
                                }
                            }
                            RithmicBrokerageClient::start_front_month_cleaner(client.front_month_info.clone()).await;
                            RITHMIC_CLIENTS.insert(system, client.clone());
                        }
                        Err(e) => {
                            eprintln!("Failed to create rithmic client for: {}, reason: {}", system, e);
                        }
                    }

                })
            })
        }).collect::<Vec<_>>();

        // Wait for all initialization tasks to complete
        join_all(init_tasks).await;

        // Send RMS info and trade routes requests for all clients
        for api in RITHMIC_CLIENTS.iter() {
            let api = api.value().clone();
            let rms_req = RequestAccountRmsInfo {
                template_id: 304,
                user_msg: vec![],
                fcm_id: api.credentials.fcm_id.clone(),
                ib_id: api.credentials.ib_id.clone(),
                user_type: api.credentials.user_type.clone(),
            };
            api.send_message(&SysInfraType::OrderPlant, rms_req).await;

            let routes = RequestTradeRoutes {
                template_id: 310,
                user_msg: vec![],
                subscribe_for_updates: Some(true),
            };
            api.send_message(&SysInfraType::OrderPlant, routes).await;
        }
    }


    pub(crate) async fn send_replay_request(&self, max_bars: i32, base_data_type: BaseDataType, resolution: Resolution, symbol_name: SymbolName, exchange: FuturesExchange, window_start: DateTime<Utc>, window_end: DateTime<Utc>, sender: oneshot::Sender<BTreeMap<DateTime<Utc>,BaseDataEnum>>) {
        const SYSTEM: SysInfraType = SysInfraType::HistoryPlant;
        // Send the request based on data type
        let callback_id = self.generate_callback_id().await;
        self.historical_callbacks.insert(callback_id.clone(), sender);
        match base_data_type {
            BaseDataType::Candles => {
                let (num, res_type) = match resolution {
                    Resolution::Seconds(num) => (num as i32, rithmic_api::client_base::rithmic_proto_objects::rti::request_time_bar_replay::BarType::SecondBar.into()),
                    Resolution::Minutes(num) =>
                        if num == 1 {
                            (60, rithmic_api::client_base::rithmic_proto_objects::rti::request_time_bar_replay::BarType::SecondBar.into())
                        }else {
                            (num as i32, rithmic_api::client_base::rithmic_proto_objects::rti::request_time_bar_replay::BarType::MinuteBar.into())
                        }
                    _ => return
                };
                //println!("Requesting candles for {} {} {} {} {} {}", symbol_name, exchange, window_start, window_end, num, res_type);
                let req = RequestTimeBarReplay {
                    template_id: 202,
                    user_msg: vec![callback_id.to_string()],
                    symbol: Some(symbol_name.clone()),
                    exchange: Some(exchange.to_string()),
                    bar_type: Some(res_type),
                    bar_type_period: Some(num),
                    start_index: Some(window_start.timestamp() as i32),
                    finish_index: Some(window_end.timestamp() as i32),
                    user_max_count: Some(max_bars),
                    direction: Some(Direction::First.into()),
                    time_order: Some(TimeOrder::Forwards.into()),
                    resume_bars: Some(false),
                };
                self.send_message(&SYSTEM, req).await;
            }
            BaseDataType::Ticks => {
                if resolution != Resolution::Ticks(1) {
                    return
                }
                let req = RequestTickBarReplay {
                    template_id: 206,
                    user_msg: vec![callback_id.to_string()],
                    symbol: Some(symbol_name.clone()),
                    exchange: Some(exchange.to_string()),
                    bar_type: Some(request_tick_bar_replay::BarType::TickBar.into()),
                    bar_sub_type: Some(1),
                    bar_type_specifier: Some("1".to_string()),
                    start_index: Some(window_start.timestamp() as i32),
                    finish_index: Some(window_end.timestamp() as i32),
                    user_max_count: Some(max_bars),
                    custom_session_open_ssm: None,
                    custom_session_close_ssm: None,
                    direction: Some(request_tick_bar_replay::Direction::First.into()),
                    time_order: Some(request_tick_bar_replay::TimeOrder::Forwards.into()),
                    resume_bars: Some(false),
                };
                self.send_message(&SYSTEM, req).await;
            }
            _ => return
        }
    }
}

pub struct CommonRithmicOrderDetails {
    pub symbol_code: String,
    pub exchange: FuturesExchange,
    pub transaction_type: TransactionType,
    pub route: String,
    pub quantity: i32
}
