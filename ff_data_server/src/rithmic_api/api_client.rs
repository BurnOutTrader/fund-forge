use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use ahash::AHashMap;
use chrono::{DateTime, TimeZone, Utc};
use dashmap::DashMap;
use ff_rithmic_api::api_client::RithmicApiClient;
use ff_rithmic_api::credentials::RithmicCredentials;
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
use ff_rithmic_api::rithmic_proto_objects::rti::{RequestFrontMonthContract, RequestHeartbeat, RequestShowOrders, RequestSubscribeForOrderUpdates, ResponseHeartbeat};
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
use ff_standard_lib::standardized_types::enums::{FuturesExchange, MarketType, OrderSide};
use ff_standard_lib::standardized_types::orders::{Order, OrderId};
use ff_standard_lib::standardized_types::subscriptions::{Symbol, SymbolName};
use ff_standard_lib::standardized_types::symbol_info::{FrontMonthInfo, SymbolInfo};
use ff_standard_lib::strategies::handlers::market_handlers::BookLevel;
use ff_standard_lib::strategies::ledgers::{AccountId, AccountInfo};
use ff_standard_lib::StreamName;
use prost::Message as ProstMessage;
use rust_decimal::Decimal;
use tokio::net::TcpStream;
use tokio::{select, task};
use tokio::task::JoinHandle;
use tokio::time::{interval, timeout};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::Message;
use ff_standard_lib::server_features::server_side_datavendor::VendorApiResponse;
use ff_standard_lib::standardized_types::new_types::Volume;
use crate::subscribe_server_shutdown;

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
    pub total_open_size: DashMap<AccountId, Volume>,
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
            total_open_size: Default::default(),
            long_quantity: Default::default(),
            short_quantity: Default::default(),
            default_trade_route: DashMap::new(),
        };
        Ok(client)
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
        if let Some(write_stream) = self.writers.get(plant) {
            let mut buf = Vec::new();
            self.heartbeat_times.insert(plant.clone(), Utc::now());
            match message.encode(&mut buf) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Failed to send message to: {:?}: {}", plant, e);
                    return;
                }
            }

            let length = buf.len() as u32;
            let mut prefixed_msg = length.to_be_bytes().to_vec();
            prefixed_msg.extend(buf);

            let mut write_stream = write_stream.lock().await;
            match write_stream.send(Message::Binary(prefixed_msg)).await {
                Ok(_) => {},
                Err(e) => eprintln!("Failed to send message to: {:?}: {}", plant, e)
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

    pub async fn request_updates(&self) {
        for id_account_info_kvp in self.account_info.iter() {
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
        }
    }

    pub fn is_valid_order(&self, order: &Order) -> Result<(), String> {
        if let Some(max_size) = self.max_size.get(&order.account_id) {
            match order.side {
                OrderSide::Buy => {
                    if let Some(account_long_quantity) = self.long_quantity.get(&order.id) {
                        if let Some(symbol_long_quantity) = account_long_quantity.value().get(&order.symbol_name) {
                            if symbol_long_quantity.value() + order.quantity_open.clone() > *max_size.value() {
                                return Err(format!("Order Would Exceed Max Position Size: {}", order.tag))
                            }
                        }
                    }
                    else if let Some(account_short_quantity) = self.short_quantity.get(&order.id) {
                        if let Some(symbol_short_quantity) = account_short_quantity.get(&order.symbol_name) {
                            if (order.quantity_open - symbol_short_quantity.value()).abs() > *max_size.value() {
                                return Err(format!("Order Would Exceed Max Position Size: {}", order.tag))
                            }
                        }
                    }
                }
                OrderSide::Sell => {
                    if let Some(account_short_quantity) = self.short_quantity.get(&order.id) {
                        if let Some(symbol_short_quantity) = account_short_quantity.value().get(&order.symbol_name) {
                            if symbol_short_quantity.value() + order.quantity_open > *max_size.value() {
                                return Err(format!("Order Would Exceed Max Position Size: {}", order.tag))
                            }
                        }
                    } else if let Some(account_long_quantity) = self.long_quantity.get(&order.id) {
                        if let Some(symbol_long_quantity) = account_long_quantity.get(&order.symbol_name) {
                            if (order.quantity_open - symbol_long_quantity.value().clone()).abs() > *max_size.value() {
                                return Err(format!("Order Would Exceed Max Position Size: {}", order.tag))
                            }
                        }
                    }
                }
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
                            self.front_month_info.insert(info.symbol.clone(), info.clone());
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
}
