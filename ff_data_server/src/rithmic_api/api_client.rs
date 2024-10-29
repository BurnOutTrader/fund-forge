use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use ahash::AHashMap;
use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::{Tz};
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
use tokio::time::{interval, timeout};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::Message;
use ff_standard_lib::apis::rithmic::rithmic_systems::RithmicSystem;
use ff_standard_lib::server_features::server_side_datavendor::VendorApiResponse;
use ff_standard_lib::standardized_types::accounts::AccountInfo;
use ff_standard_lib::standardized_types::new_types::Volume;
use ff_standard_lib::standardized_types::position::PositionId;
use uuid::Uuid;
use crate::{get_data_folder, subscribe_server_shutdown, ServerLaunchOptions};
use crate::rithmic_api::client_base::api_base::RithmicApiClient;
use crate::rithmic_api::client_base::credentials::RithmicCredentials;
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_login::SysInfraType;
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::{RequestAccountRmsInfo, RequestFrontMonthContract, RequestHeartbeat, RequestNewOrder, RequestPnLPositionUpdates, RequestShowOrders, RequestSubscribeForOrderUpdates, RequestTradeRoutes, ResponseHeartbeat};
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_new_order::{OrderPlacement, PriceType, TransactionType};
use crate::rithmic_api::plant_handlers::handler_loop::handle_rithmic_responses;
use crate::rithmic_api::products::get_exchange_by_symbol_name;

lazy_static! {
    pub static ref RITHMIC_CLIENTS: DashMap<RithmicSystem , Arc<RithmicClient>> = DashMap::with_capacity(16);
}

// We do not want to initialize here, that should be done at server launch, else a strategy could sign out the client of the correct server.
pub fn get_rithmic_client(rithmic_system: &RithmicSystem) -> Option<Arc<RithmicClient>> {
    if let Some(client) = RITHMIC_CLIENTS.get(&rithmic_system) {
        return Some(client.value().clone())
    }
    None
}

pub struct RithmicClient {
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
    pub heartbeat_tasks: Arc<DashMap<SysInfraType, JoinHandle<()>>>,
    pub latency: Arc<DashMap<SysInfraType, i64>>,
    /// Rithmic clients
    pub client: Arc<RithmicApiClient>,
    pub front_month_info: DashMap<SymbolName, FrontMonthInfo>,

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

    //subscribers
    pub tick_feed_broadcasters: Arc<DashMap<SymbolName, broadcast::Sender<BaseDataEnum>>>,
    pub quote_feed_broadcasters: Arc<DashMap<SymbolName, broadcast::Sender<BaseDataEnum>>>,
    pub candle_feed_broadcasters: Arc<DashMap<SymbolName, broadcast::Sender<BaseDataEnum>>>,

    // first string is fcm id second is trade route
    pub default_trade_route: DashMap<RithmicSystem, AHashMap<(String, FuturesExchange), String>>,

    pub bid_book: DashMap<SymbolName, BTreeMap<u16, BookLevel>>,
    pub ask_book: DashMap<SymbolName, BTreeMap<u16, BookLevel>>,

    pub order_broadcaster: broadcast::Sender<DataServerResponse>
}

impl RithmicClient {
    pub async fn new(
        system: RithmicSystem,
    ) -> Result<Self, FundForgeError> {
        let brokerage = Brokerage::Rithmic(system.clone());
        let data_vendor = DataVendor::Rithmic(system.clone());
        let credentials = RithmicClient::rithmic_credentials(&brokerage)?;
        println!("Activating {} {} on Rithmic Server: {}", credentials.user, credentials.system_name, credentials.server_name);
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
            heartbeat_tasks: Arc::new(DashMap::with_capacity(5)),
            latency: Arc::new(DashMap::with_capacity(5)),
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
                if self.credentials.ib_id != *self.client.ib_id.read().await {
                 /*   let mut credentials = self.credentials.clone();
                    credentials.ib_id = self.client.ib_id.read().await.clone();
                    credentials.fcm_id = self.client.fcm_id.read().await.clone();
                    let file_name = PathBuf::from(get_data_folder())
                        .join("credentials")
                        .join("rithmic_credentials")
                        .join("active")
                        .join(credentials.file_name())
                        .to_string_lossy()
                        .into_owned();
                    credentials.save_credentials_to_file(&file_name).unwrap();*/
                }
                Ok(receiver)
            },
            Err(e) => {
                Err(FundForgeError::ServerErrorDebug(e.to_string()))
            }
        }
    }

    pub async fn send_callback(&self, stream_name: StreamName, callback_id: u64, response: DataServerResponse) {
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
            self.send_message(plant, request).await;
        }
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
                //println!("{}", file_path);
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
                Err(e) => eprintln!("Failed to send message to {:?}: {}. Retrying...", plant, e),
            }
        } else {
            eprintln!("No write stream available for {:?}. Retrying...", plant);
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
            Some(interval) => Duration::from_secs(interval.value().clone()),
            None => Duration::from_secs(59)
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

    pub fn handle_response_heartbeat(&self, plant: SysInfraType, response: ResponseHeartbeat) {
        if let (Some(ssboe), Some(usecs)) = (response.ssboe, response.usecs) {
            // Get current time components
            let now = Utc::now();
            let now_secs = now.timestamp() as u64;
            let now_nanos = now.timestamp_subsec_nanos() as u64;

            // Convert server time to the same components
            let server_secs = ssboe as u64;
            let server_nanos = (usecs as u64) * 1000;  // Convert microseconds to nanoseconds

            // Calculate latency components
            let secs_diff = if now_secs >= server_secs {
                now_secs - server_secs
            } else {
                0
            };

            // Calculate total nanoseconds difference
            let latency = if secs_diff == 0 {
                // If within the same second, just compare nanoseconds
                if now_nanos >= server_nanos {
                    now_nanos - server_nanos
                } else {
                    server_nanos - now_nanos
                }
            } else {
                // If seconds differ, calculate total nanosecond difference
                (secs_diff * 1_000_000_000) + now_nanos - server_nanos
            };

            self.latency.insert(plant.clone(), latency as i64);

            #[allow(unused_variables)]
            let formatted_latency = if latency < 1_000 {
                format!("{} ns", latency)
            } else if latency < 1_000_000 {
                format!("{:.2} µs", latency as f64 / 1_000.0)
            } else if latency < 1_000_000_000 {
                format!("{:.2} ms", latency as f64 / 1_000_000.0)
            } else {
                format!("{:.3} s", latency as f64 / 1_000_000_000.0)
            };

            //println!("Round Trip Latency for Rithmic {:?}: {}", plant, formatted_latency);
        }
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
            self.send_message(&SysInfraType::OrderPlant, req).await;
            let req = RequestSubscribeForOrderUpdates {
                template_id: 308 ,
                user_msg: vec![],
                fcm_id: self.fcm_id.clone(),
                ib_id: self.ib_id.clone(),
                account_id: Some(id_account_info_kvp.key().clone()),
            };
            self.send_message(&SysInfraType::OrderPlant, req).await;
            let req = RequestPnLPositionUpdates {
                template_id: 400,
                user_msg: vec![],
                request: Some(1),
                fcm_id: self.fcm_id.clone(),
                ib_id: self.ib_id.clone(),
                account_id: Some(id_account_info_kvp.key().clone()),
            };
            self.send_message(&SysInfraType::PnlPlant, req).await;
            
            let req = RequestTradeRoutes {
                template_id: 310,
                user_msg: vec![],
                subscribe_for_updates: Some(true), //todo not sure if we want updates, they never seem to stop.
            };
            self.send_message(&SysInfraType::OrderPlant, req).await;
            /*   let symbol = Symbol {
                name: "MNQ".to_string(),
                market_type: MarketType::Futures(FuturesExchange::CME),
                data_vendor: DataVendor::Rithmic(RithmicSystem::Rithmic01),
            };
            match self.update_historical_data_for(1, symbol, BaseDataType::Candles, Resolution::Seconds(1)).await {
                Ok(_) => {}
                Err(_) => {}
            }*/
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

    pub async fn front_month(&self, stream_name: StreamName, symbol_name: SymbolName, exchange: FuturesExchange) -> Result<FrontMonthInfo, String> {
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

            match timeout(Duration::from_secs(10), receiver).await {
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
            Err(e) => return Err(RithmicClient::reject_order(&order, format!("{}",e))),
            Ok(_) => {}
        }

        let quantity = match order.quantity_open.to_i32() {
            None => {
                return Err(RithmicClient::reject_order(&order, "Invalid quantity".to_string()))
            }
            Some(q) => q
        };

        let (symbol_code, exchange): (SymbolName, FuturesExchange) = {
            match get_exchange_by_symbol_name(&order.symbol_name) {
                None => return Err(RithmicClient::reject_order(&order, format!("Exchange Not found with {} for {}",order.account.brokerage, order.symbol_name))),
                Some(mut exchange) => {
                    exchange = match &order.exchange {
                        None => exchange,
                        Some(preset) => {
                            match FuturesExchange::from_string(preset) {
                                Err(e) => return Err(RithmicClient::reject_order(&order, format!("Error parsing exchange string {} {}",preset, e))),
                                Ok(ex) => ex.clone()
                            }
                        }
                    };
                    match &order.symbol_code {
                        None => {
                            let front_month = match self.front_month(stream_name, order.symbol_name.clone(), exchange.clone()).await {
                                Ok(info) => info,
                                Err(e) => return Err(RithmicClient::reject_order(&order, format!("{}",e)))
                            };
                            (front_month.symbol_code, exchange)
                        }
                        Some(code) => {
                            (code.clone(), exchange)
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
                    _ => return Err(RithmicClient::reject_order(&order, format!("Order Route Not found with {} for {}",order.account.brokerage, order.symbol_name)))
                }
            }
            Some(route_map) => {
                let fcm_id = match &self.credentials.fcm_id {
                    None => return Err(RithmicClient::reject_order(&order, format!("Order Route Not found with {}, fcm_id not found",order.account.brokerage))),
                    Some(id) => id.clone()
                };
                if let Some(exchange_route) = route_map.get(&(fcm_id.clone(), exchange.clone())) {
                    exchange_route.clone()
                } else {
                    match self.system {
                        RithmicSystem::RithmicPaperTrading | RithmicSystem::TopstepTrader | RithmicSystem::SpeedUp | RithmicSystem::TradeFundrr | RithmicSystem::UProfitTrader | RithmicSystem::Apex | RithmicSystem::MESCapital |
                        RithmicSystem::TheTradingPit | RithmicSystem::FundedFuturesNetwork | RithmicSystem::Bulenox | RithmicSystem::PropShopTrader | RithmicSystem::FourPropTrader | RithmicSystem::FastTrackTrading
                        => "simulator".to_string(),
                        _ => return Err(RithmicClient::reject_order(&order, format!("Order Route Not found with {} for {}",order.account.brokerage, order.symbol_name)))
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
            TimeInForce::Day(ref tz_string) => {
                let time_zone = match Tz::from_str(tz_string) {
                    Ok(time_zone) => time_zone,
                    Err(e) => return Err(Self::reject_order(&order, format!("Failed to parse Tz: {}", e)))
                };

                let now = Utc::now();
                let end_of_day = match time_zone.from_utc_datetime(&now.naive_utc())
                    .date_naive()
                    .and_hms_opt(23, 59, 59)
                    .and_then(|naive_dt| naive_dt.and_local_timezone(time_zone).single())
                    .map(|tz_dt| tz_dt.with_timezone(&Utc))
                {
                    Some(dt) => dt,
                    None => return Err(Self::reject_order(&order, format!("Failed to calculate end of day for timezone: {}", tz_string)))
                };

                (crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_bracket_order::Duration::Gtc.into(),
                 Some(end_of_day.timestamp() as i32),
                 Some(end_of_day.timestamp_subsec_micros() as i32))
            }
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

        if let Some(account_map) = self.open_orders.get(&order.account.account_id) {
            account_map.insert(order.account.account_id.clone(), order.clone());
        }

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

        if let Some(account_map) = self.open_orders.get(&order.account.account_id) {
            account_map.insert(order.account.account_id.clone(), order.clone());
        }
        //this is used to update positions when synchronise positions is used
        if let Some(account_map) = self.last_tag.get(&order.account.account_id) {
            account_map.insert(details.symbol_code, order.tag.clone());
        }
        self.send_message(&SysInfraType::OrderPlant, req).await;
    }

    #[allow(dead_code)]
    pub(crate) async fn init_rithmic_apis(options: ServerLaunchOptions) {
        let options = options;
        if options.disable_rithmic_server == 1 {
            return
        }
        let toml_files = RithmicClient::get_rithmic_tomls();
        if toml_files.is_empty() {
            return;
        }
        let init_tasks = toml_files.into_iter().filter_map(|file| {
            RithmicSystem::from_file_string(file.as_str()).map(|system| {
                task::spawn(async move {
                    let running = Arc::new(AtomicBool::new(true));
                    let running_clone = running.clone();

                    // Task 1: Handle shutdown signal
                    tokio::spawn(async move {
                        let mut shutdown_receiver = subscribe_server_shutdown();
                        let _ = shutdown_receiver.recv().await;
                        running_clone.store(false, Ordering::SeqCst);
                    });

                    match RithmicClient::new(system).await {
                        Ok(client) => {
                            let client = Arc::new(client);
                            match client.connect_plant(SysInfraType::TickerPlant).await {
                                Ok(receiver) => {
                                    RITHMIC_CLIENTS.insert(system, client.clone());
                                    handle_rithmic_responses(client.clone(), receiver, SysInfraType::TickerPlant, running.clone());
                                }
                                Err(e) => {
                                    eprintln!("Failed to run rithmic client for: {}, reason: {}", system, e);
                                }
                            }
                            match client.connect_plant(SysInfraType::HistoryPlant).await {
                                Ok(receiver) => {
                                    RITHMIC_CLIENTS.insert(system, client.clone());
                                    handle_rithmic_responses(client.clone(), receiver, SysInfraType::HistoryPlant, running.clone());
                                }
                                Err(e) => {
                                    eprintln!("Failed to run rithmic client for: {}, reason: {}", system, e);
                                }
                            }
                            match client.connect_plant(SysInfraType::OrderPlant).await {
                                Ok(receiver) => {
                                    RITHMIC_CLIENTS.insert(system, client.clone());
                                    handle_rithmic_responses(client.clone(), receiver, SysInfraType::OrderPlant, running.clone());
                                }
                                Err(e) => {
                                    eprintln!("Failed to run rithmic client for: {}, reason: {}", system, e);
                                }
                            }
                            match client.connect_plant(SysInfraType::PnlPlant).await {
                                Ok(receiver) => {
                                    RITHMIC_CLIENTS.insert(system, client.clone());
                                    handle_rithmic_responses(client.clone(), receiver, SysInfraType::PnlPlant, running.clone());
                                }
                                Err(e) => {
                                    eprintln!("Failed to run rithmic client for: {}, reason: {}", system, e);
                                }
                            }
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
}

pub struct CommonRithmicOrderDetails {
    pub symbol_code: String,
    pub exchange: FuturesExchange,
    pub transaction_type: TransactionType,
    pub route: String,
    pub quantity: i32
}
