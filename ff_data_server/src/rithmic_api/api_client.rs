use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use ahash::AHashMap;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use ff_rithmic_api::api_client::RithmicApiClient;
use ff_rithmic_api::credentials::RithmicCredentials;
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
use ff_rithmic_api::rithmic_proto_objects::rti::RequestHeartbeat;
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
use ff_standard_lib::standardized_types::enums::MarketType;
use ff_standard_lib::standardized_types::orders::{Order, OrderId};
use ff_standard_lib::standardized_types::subscriptions::{Symbol, SymbolName};
use ff_standard_lib::standardized_types::symbol_info::SymbolInfo;
use ff_standard_lib::strategies::handlers::market_handlers::BookLevel;
use ff_standard_lib::strategies::ledgers::{AccountId, AccountInfo};
use ff_standard_lib::StreamName;
use prost::Message as ProstMessage;
use tokio::net::TcpStream;
use tokio::{select, task};
use tokio::time::interval;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::Message;
use ff_standard_lib::server_features::server_side_datavendor::VendorApiResponse;
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
    pub writers: DashMap<SysInfraType, Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>>,
    pub heartbeat_times: Arc<DashMap<SysInfraType, DateTime<Utc>>>,
    pub latency: Arc<DashMap<SysInfraType, i64>>,
    /// Rithmic clients
    pub client: Arc<RithmicApiClient>,
    pub symbol_info: DashMap<SymbolName, SymbolInfo>,

    // accounts
    pub accounts: DashMap<AccountId, AccountInfo>,
    pub orders_open: DashMap<OrderId, Order>,

    //products
    pub products: DashMap<MarketType, Vec<Symbol>>,

    //subscribers
    pub tick_feed_broadcasters: DashMap<SymbolName, broadcast::Sender<BaseDataEnum>>,
    pub quote_feed_broadcasters: DashMap<SymbolName, broadcast::Sender<BaseDataEnum>>,
    pub candle_feed_broadcasters: DashMap<SymbolName, broadcast::Sender<BaseDataEnum>>,

    pub bid_book: DashMap<SymbolName, BTreeMap<u16, BookLevel>>,
    pub ask_book: DashMap<SymbolName, BTreeMap<u16, BookLevel>>,
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
        let client = Self {
            brokerage,
            data_vendor,
            system,
            fcm_id: credentials.fcm_id.clone(),
            ib_id: credentials.ib_id.clone(),
            user_type: credentials.user_type.clone(),
            credentials,
            callbacks: Default::default(),
            writers: DashMap::with_capacity(5),
            heartbeat_times: Arc::new(DashMap::with_capacity(5)),
            latency: Arc::new(DashMap::with_capacity(5)),
            client: Arc::new(client),
            symbol_info: Default::default(),
            tick_feed_broadcasters: Default::default(),
            quote_feed_broadcasters: Default::default(),
            bid_book: Default::default(),
            accounts: Default::default(),
            orders_open: Default::default(),
            products: Default::default(),
            candle_feed_broadcasters: Default::default(),
            ask_book: Default::default(),
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

    pub async fn connect_plant(&self, system: SysInfraType) -> Result<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>, FundForgeError >{
        match self.client.connect_and_login(system.clone()).await {
            Ok(r) => {
                let (writer, receiver) = r.split();
                let writer = Arc::new(Mutex::new(writer));
                self.writers.insert(system.clone(), writer.clone());
                self.heartbeat_times.insert(system.clone(), Utc::now());
                let _ = self.start_heart_beat(system, writer, self.heartbeat_times.clone());
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

    pub async fn register_callback(&self, stream_name: StreamName, callback_id: u64, sender: oneshot::Sender<DataServerResponse>) {
        if let Some(mut stream_map) = self.callbacks.get_mut(&stream_name) {
            stream_map.value_mut().insert(callback_id, sender);
        } else {
            let mut map = AHashMap::new();
            map.insert(callback_id, sender);
            self.callbacks.insert(stream_name.clone(), map);
        }
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
            None => Duration::from_secs(60)
        };

        let mut interval = interval(heart_beat_interval);
        task::spawn(async move {
            let mut shutdown_receiver = subscribe_server_shutdown();
            'heartbeat_loop: loop {
                select! {
                    _ = interval.tick() => {
                        let now = Utc::now();
                        let should_send_heartbeat = if let Some(last_time) = heartbeat_times.get(&plant) {
                            now.signed_duration_since(*last_time) >= chrono::Duration::from_std(heart_beat_interval).unwrap()
                        } else {
                            true // If no last heartbeat time, we should send one
                        };

                        if should_send_heartbeat {
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
                                },
                                Err(e) => {
                                    eprintln!("Failed to send heartbeat to {:?}: {}", plant, e);
                                    break 'heartbeat_loop;
                                }
                            }
                        }
                    }
                    _ = shutdown_receiver.recv() => {
                        println!("Shutting down heartbeat task for system {:?}", plant);
                        break 'heartbeat_loop;
                    }
                }
            }
        });
    }
}


