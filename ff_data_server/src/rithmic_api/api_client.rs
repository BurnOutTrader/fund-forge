use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use ahash::AHashMap;
use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::{Tz};
use dashmap::DashMap;
use ff_rithmic_api::api_client::RithmicApiClient;
use ff_rithmic_api::credentials::RithmicCredentials;
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
use ff_rithmic_api::rithmic_proto_objects::rti::{RequestFrontMonthContract, RequestHeartbeat, RequestNewOrder, RequestPnLPositionUpdates, RequestShowOrders, RequestSubscribeForOrderUpdates, ResponseHeartbeat};
use ff_rithmic_api::rithmic_proto_objects::rti::request_bracket_order::OrderPlacement;
use ff_rithmic_api::rithmic_proto_objects::rti::request_new_order::PriceType;
use ff_rithmic_api::rithmic_proto_objects::rti::rithmic_order_notification::TransactionType;
use ff_rithmic_api::systems::RithmicSystem;
use futures::{SinkExt, StreamExt};
use futures::stream::{SplitSink, SplitStream};
#[allow(unused_imports)]
use structopt::lazy_static::lazy_static;
use tokio::sync::{broadcast, oneshot, Mutex};
use ff_standard_lib::helpers::get_data_folder;
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::enums::{FuturesExchange, MarketType, OrderSide, StrategyMode};
use ff_standard_lib::standardized_types::orders::{Order, OrderId, OrderType, OrderUpdateEvent, TimeInForce};
use ff_standard_lib::standardized_types::subscriptions::{Symbol, SymbolName};
use ff_standard_lib::standardized_types::symbol_info::{FrontMonthInfo, SymbolInfo};
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
use ff_standard_lib::server_features::server_side_datavendor::VendorApiResponse;
use ff_standard_lib::standardized_types::accounts::AccountInfo;
use ff_standard_lib::standardized_types::new_types::Volume;
use crate::{get_shutdown_sender, subscribe_server_shutdown};
use crate::rithmic_api::products::get_exchange_by_code;

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
    pub symbol_info: DashMap<SymbolName, SymbolInfo>,
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
    pub day_open_pnl: DashMap<AccountId, Decimal>,
    pub day_closed_pnl: DashMap<AccountId, Decimal>,
    pub max_size: DashMap<AccountId, Volume>,
    pub long_quantity: DashMap<AccountId, DashMap<SymbolName, Volume>>,
    pub short_quantity: DashMap<AccountId, DashMap<SymbolName, Volume>>,

    pub orders_open: DashMap<OrderId, Order>,

    //products
    pub products: DashMap<MarketType, Vec<Symbol>>,

    //subscribers
    pub tick_feed_broadcasters: Arc<DashMap<SymbolName, broadcast::Sender<BaseDataEnum>>>,
    pub quote_feed_broadcasters: Arc<DashMap<SymbolName, broadcast::Sender<BaseDataEnum>>>,
    pub candle_feed_broadcasters: Arc<DashMap<SymbolName, broadcast::Sender<BaseDataEnum>>>,

    pub default_trade_route: DashMap<FuturesExchange, String>,

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
        println!("{:?}", credentials);
        let data_folder = get_data_folder();
        let server_domains_toml = PathBuf::from(data_folder)
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
            symbol_info: Default::default(),
            front_month_info: Default::default(),
            account_info: Default::default(),
            account_balance: Default::default(),
            account_cash_available: Default::default(),
            account_cash_used: Default::default(),
            margin_used: Default::default(),
            margin_available: Default::default(),
            open_pnl: Default::default(),
            closed_pnl: Default::default(),
            day_open_pnl: Default::default(),
            day_closed_pnl: Default::default(),
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
        };
        Ok(client)
    }

    pub fn total_open_size(&self) -> Volume {
        let mut size = dec!(0);

        // Sum up long positions
        for account_map in self.long_quantity.iter() {
            for symbol_map in account_map.value().iter() {
                size += *symbol_map.value();
            }
        }

        // Sum up short positions
        for account_map in self.short_quantity.iter() {
            for symbol_map in account_map.value().iter() {
                size += *symbol_map.value();
            }
        }

        size
    }


    pub fn get_rithmic_tomls() -> Vec<String> {
        let mut toml_files = Vec::new();
        let dir = PathBuf::from(get_data_folder())
            .join("rithmic_credentials")
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
                let file_path = format!("{}/rithmic_credentials/{}", data_folder, file);
                println!("{}", file_path);
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
        let now = Utc::now();
        if let (Some(ssboe), Some(usecs)) = (response.ssboe, response.usecs) {
            let response_time = Utc.timestamp_opt(ssboe as i64, usecs as u32 * 1000).unwrap();
            let latency = (now - response_time).num_microseconds().unwrap_or(i64::MAX);

            self.latency.insert(plant.clone(), latency);

            let formatted_latency = if latency < 1_000 {
                format!("{} µs", latency)
            } else if latency < 1_000_000 {
                format!("{:.2} ms", latency as f64 / 1_000.0)
            } else {
                format!("{:.3} s", latency as f64 / 1_000_000.0)
            };

            println!("Round Trip Latency for Rithmic {:?}: {}", plant, formatted_latency);
        } else {
            println!("Unable to calculate latency: missing timestamp in ResponseHeartbeat");
        }
    }

    pub async fn request_updates(&self, account_id: AccountId) {
        if let Some(id_account_info_kvp) = self.account_info.get(&account_id) {
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


    pub async fn rithmic_order_details(&self, mode: StrategyMode, stream_name: StreamName, order: &Order) -> Result<CommonRithmicOrderDetails, OrderUpdateEvent> {
        if mode != StrategyMode::Live {
            get_shutdown_sender().send(()).unwrap();
            panic!("This should never happen, Live order sent by Backtest")
        }

        match self.is_valid_order(&order) {
            Err(e) =>
                return Err(OrderUpdateEvent::OrderRejected {
                    account: order.account.clone(),
                    symbol_name: order.symbol_name.clone(),
                    symbol_code: order.symbol_name.clone(),
                    order_id: order.id.clone(),
                    reason: e,
                    tag: order.tag.clone(),
                    time: Utc::now().to_string() }),

            Ok(_) => {}
        }

        let quantity = match order.quantity_open.to_i32() {
            None => {
                return Err(OrderUpdateEvent::OrderRejected {
                    account: order.account.clone(),
                    symbol_name: order.symbol_name.clone(),
                    symbol_code: order.symbol_name.clone(),
                    order_id: order.id.clone(),
                    reason: "Invalid Quantity".to_string(),
                    tag: order.tag.clone(),
                    time: Utc::now().to_string() })
            }
            Some(q) => q
        };

        let (symbol_code, exchange): (SymbolName, FuturesExchange) = {
            match get_exchange_by_code(&order.symbol_name) {
                None => {
                    return Err(OrderUpdateEvent::OrderRejected {
                        account: order.account.clone(),
                        symbol_name: order.symbol_name.clone(),
                        symbol_code: order.symbol_name.clone(),
                        order_id: order.id.clone(),
                        reason: format!("Exchange Not found with {} for {}",order.account.brokerage, order.symbol_name),
                        tag: order.tag.clone(),
                        time: Utc::now().to_string() })
                }
                Some(mut exchange) => {
                    exchange = match &order.exchange {
                        None => exchange,
                        Some(_preset) => {
                            exchange
                            //todo[Rithmic Api] Set this up to parse from string
                            /*match preset {

                            }*/
                        }
                    };
                    match &order.symbol_code {
                        None => {
                            let front_month = match self.front_month(stream_name, order.symbol_name.clone(), exchange.clone()).await {
                                Ok(info) => info,
                                Err(e) => {
                                    return Err(OrderUpdateEvent::OrderRejected {
                                        account: order.account.clone(),
                                        symbol_name: order.symbol_name.clone(),
                                        symbol_code: "No Front Month Found".to_string(),
                                        order_id: order.id.clone(),
                                        reason: e,
                                        tag: order.tag.clone(),
                                        time: Utc::now().to_string() })
                                }
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

        let route = match self.default_trade_route.get(&exchange) {
            None => {
                return Err(OrderUpdateEvent::OrderRejected {
                    account: order.account.clone(),
                    symbol_name: order.symbol_name.clone(),
                    symbol_code: order.symbol_name.clone(),
                    order_id: order.id.clone(),
                    reason: format!("Order Route Not found with {} for {}",order.account.brokerage, order.symbol_name),
                    tag: order.tag.clone(),
                    time: Utc::now().to_string() })
            }
            Some(route) => route.value().clone(),
        };

        let transaction_type = match order.side {
            OrderSide::Buy => TransactionType::Buy,
            OrderSide::Sell => TransactionType::Sell,
        };

        Ok(CommonRithmicOrderDetails {
            symbol_code,
            exchange,
            transaction_type,
            route,
            quantity,
        })
    }

    pub async fn submit_order(&self, stream_name: StreamName, order: Order, details: CommonRithmicOrderDetails) {
        let (duration, cancel_at_ssboe, cancel_at_usecs) = match order.time_in_force {
            TimeInForce::IOC => (ff_rithmic_api::rithmic_proto_objects::rti::request_bracket_order::Duration::Ioc.into(), None, None),
            TimeInForce::FOK => (ff_rithmic_api::rithmic_proto_objects::rti::request_bracket_order::Duration::Fok.into(), None, None),
            TimeInForce::GTC => (ff_rithmic_api::rithmic_proto_objects::rti::request_bracket_order::Duration::Gtc.into(), None, None),
            TimeInForce::Day(ref tz_string) => {
                let time_zone = match Tz::from_str(tz_string) {
                    Ok(time_zone) => time_zone,
                    Err(e) => {
                        eprintln!("Failed to parse TZ in rithmic submit_order(): {}", e);
                        return;
                    }
                };
                let now = Utc::now();
                let end_of_day = match time_zone.from_utc_datetime(&now.naive_utc())
                    .date_naive()
                    .and_hms_opt(23, 59, 59)
                    .and_then(|naive_dt| naive_dt.and_local_timezone(time_zone).single())
                    .map(|tz_dt| tz_dt.with_timezone(&Utc))
                {
                    Some(dt) => dt,
                    None => return eprintln!("Failed to calculate end of day for timezone: {}", tz_string),
                };

                (ff_rithmic_api::rithmic_proto_objects::rti::request_bracket_order::Duration::Gtc.into(),
                 Some(end_of_day.timestamp() as i32),
                 Some(end_of_day.timestamp_subsec_micros() as i32))
            }
            TimeInForce::Time(ref time_stamp, ref tz_string) => {
                let time_zone = match Tz::from_str(tz_string) {
                    Ok(tz) => tz,
                    Err(e) => {
                        eprintln!("Failed to parse time zone in rithmic submit_order(): {}", e);
                        return;
                    }
                };
                let cancel_time = time_zone.timestamp_opt(*time_stamp, 0).unwrap();

                (ff_rithmic_api::rithmic_proto_objects::rti::request_bracket_order::Duration::Gtc.into(),
                 Some(cancel_time.timestamp() as i32),
                 Some(cancel_time.timestamp_subsec_micros() as i32))
            }
        };

        match order.side {
            OrderSide::Buy => eprintln!("Buying {}" , order.quantity_open),
            OrderSide::Sell => eprintln!("Selling {}" , order.quantity_open),
        }

        let trigger_price = match order.trigger_price {
            None => None,
            Some(price) => {
                match price.to_f64() {
                    None => return,
                    Some(price) => Some(price)
                }
            }
        };

        let limit_price = match order.limit_price {
            None => None,
            Some(price) => {
                match price.to_f64() {
                    None => return,
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

        let req = RequestNewOrder {
            template_id: 312,
            user_msg: vec![stream_name.to_string(), order.account.account_id.clone(), order.tag.clone(), order.symbol_name, details.symbol_code.clone()],
            user_tag: Some(order.id.clone()),
            window_name: Some(stream_name.to_string()),
            fcm_id: self.fcm_id.clone(),
            ib_id: self.ib_id.clone(),
            account_id: Some(order.account.account_id.clone()),
            symbol: Some(details.symbol_code),
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

        self.send_message(&SysInfraType::OrderPlant, req).await;
    }

    pub async fn submit_market_order(&self, stream_name: StreamName, order: Order, details: CommonRithmicOrderDetails) {
        let duration = match order.time_in_force {
            TimeInForce::IOC => ff_rithmic_api::rithmic_proto_objects::rti::request_bracket_order::Duration::Ioc.into(),
            TimeInForce::FOK => ff_rithmic_api::rithmic_proto_objects::rti::request_bracket_order::Duration::Fok.into(),
            _ => ff_rithmic_api::rithmic_proto_objects::rti::request_bracket_order::Duration::Fok.into()
        };

        match order.side {
            OrderSide::Buy => eprintln!("Buying {}" , order.quantity_open),
            OrderSide::Sell => eprintln!("Selling {}" , order.quantity_open),
        }

        let req = RequestNewOrder {
            template_id: 312,
            user_msg: vec![stream_name.to_string(), order.account.account_id.clone(), order.tag.clone(), order.symbol_name, details.symbol_code.clone()],
            user_tag: Some(order.id.clone()),
            window_name: Some(stream_name.to_string()),
            fcm_id: self.fcm_id.clone(),
            ib_id: self.ib_id.clone(),
            account_id: Some(order.account.account_id.clone()),
            symbol: Some(details.symbol_code),
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

        self.send_message(&SysInfraType::OrderPlant, req).await;
    }
}

pub struct CommonRithmicOrderDetails {
    pub symbol_code: String,
    pub exchange: FuturesExchange,
    pub transaction_type: TransactionType,
    pub route: String,
    pub quantity: i32
}
