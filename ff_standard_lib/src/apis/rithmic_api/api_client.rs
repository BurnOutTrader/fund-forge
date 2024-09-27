use std::sync::{Arc};
use ahash::AHashMap;
use async_trait::async_trait;
use dashmap::DashMap;
use ff_rithmic_api::api_client::RithmicApiClient;
use ff_rithmic_api::credentials::RithmicCredentials;
use ff_rithmic_api::errors::RithmicApiError;
use ff_rithmic_api::rithmic_proto_objects::rti::request_account_list::UserType;
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
use ff_rithmic_api::rithmic_proto_objects::rti::{AccountPnLPositionUpdate, RequestAccountList, RequestAccountRmsInfo, RequestLoginInfo, RequestPnLPositionSnapshot, RequestProductCodes, ResponseAccountRmsInfo};
use ff_rithmic_api::systems::RithmicSystem;
use futures::stream::SplitStream;
use lazy_static::lazy_static;
use prost::Message as ProstMessage;
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tokio::sync::{Mutex};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use crate::apis::brokerage::broker_enum::Brokerage;
use crate::apis::brokerage::server_side_brokerage::BrokerApiResponse;
use crate::apis::data_vendor::datavendor_enum::DataVendor;
use crate::apis::data_vendor::server_side_datavendor::VendorApiResponse;
use crate::apis::StreamName;
use crate::helpers::get_data_folder;
use crate::servers::internal_broadcaster::StaticInternalBroadcaster;
use crate::standardized_types::accounts::ledgers::{AccountId, AccountInfo};
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::data_server_messaging::{FundForgeError, DataServerResponse};
use crate::standardized_types::enums::{Exchange, MarketType, Resolution, StrategyMode, SubscriptionResolutionType};
use crate::standardized_types::subscriptions::{DataSubscription, Symbol, SymbolName};
use crate::standardized_types::symbol_info::SymbolInfo;
use crate::standardized_types::{Volume};
use tokio::sync::{oneshot};

lazy_static! {
    pub static ref RITHMIC_CLIENTS: DashMap<RithmicSystem , Arc<RithmicClient>> = DashMap::with_capacity(16);
}

pub fn get_rithmic_client(data_vendor: &DataVendor) -> Option<Arc<RithmicClient>> {
    match data_vendor {
        DataVendor::Rithmic(system) => {
            if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                return Some(client.value().clone())
            }
            None
        }
        DataVendor::Test => panic!("Incorrect vendor for this fn")
    }
}

pub fn get_rithmic_client_brokers(brokerage: &Brokerage) -> Option<Arc<RithmicClient>> {
    match brokerage {
        Brokerage::Rithmic(system) => {
            if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                return Some(client.value().clone())
            }
            None
        }
        Brokerage::Test => panic!("Incorrect vendor for this fn")
    }
}

pub struct RithmicClient {
    pub brokerage: Brokerage,
    pub data_vendor: DataVendor,
    pub system: RithmicSystem,
    pub fcm_id: Option<String>,
    pub ib_id: Option<String>,
    pub user_type: Option<i32>,

    pub callbacks: DashMap<StreamName, AHashMap<u64, oneshot::Sender<DataServerResponse>>>,

    /// Rithmic clients
    pub client: Arc<RithmicApiClient>,
    pub symbol_info: DashMap<SymbolName, SymbolInfo>,
    pub readers: DashMap<SysInfraType, Arc<Mutex<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>>>,

    // accounts
    pub accounts: DashMap<AccountId, AccountInfo>,
    pub account_rms_info: DashMap<AccountId, ResponseAccountRmsInfo>,

    //products
    pub products: DashMap<MarketType, Vec<Symbol>>,

    //subscribers
    data_feed_broadcasters: Arc<DashMap<DataSubscription, Arc<StaticInternalBroadcaster<DataServerResponse>>>>,
}

impl RithmicClient {
    async fn run_start_up(&self) {
     /*   let accounts = RequestAccountList {
            template_id: 302,
            user_msg: vec![],
            fcm_id: self.fcm_id.clone(),
            ib_id: self.ib_id.clone(),
            user_type: self.user_type.clone()
        };
        self.client.send_message(SysInfraType::OrderPlant, accounts).await.unwrap();*/
        let rms_req = RequestAccountRmsInfo {
            template_id: 304,
            user_msg: vec![],
            fcm_id: self.fcm_id.clone(),
            ib_id: self.ib_id.clone(),
            user_type: self.user_type.clone(),
        };
        self.client.send_message(SysInfraType::OrderPlant, rms_req).await.unwrap();
    }

    pub async fn new(
        system: RithmicSystem,
        app_name: String,
        app_version: String,
        aggregated_quotes: bool,
        server_domains_toml: String,
    ) -> Self {
        let brokerage = Brokerage::Rithmic(system.clone());
        let data_vendor = DataVendor::Rithmic(system.clone());
        let credentials = RithmicClient::rithmic_credentials(&brokerage).unwrap();
        let client = RithmicApiClient::new(credentials.clone(), app_name, app_version, aggregated_quotes, server_domains_toml).unwrap();
        let mut client = Self {
            brokerage,
            data_vendor,
            system,
            fcm_id: credentials.fcm_id.clone(),
            ib_id: credentials.ib_id.clone(),
            user_type: credentials.user_type.clone(),
            callbacks: Default::default(),
            client: Arc::new(client),
            symbol_info: Default::default(),
            readers: DashMap::with_capacity(5),
            data_feed_broadcasters: Default::default(),
            accounts: Default::default(),
            account_rms_info: Default::default(),
            products: Default::default(),
        };
        let _ticker_receiver = match client.client.connect_and_login(SysInfraType::TickerPlant).await {
            Ok(r) => client.readers.insert(SysInfraType::TickerPlant, Arc::new(Mutex::new(r))),
            Err(_e) => {
                let _ = client.client.shutdown_all().await;
                return client
            }
        };
        let _history_receiver = match client.client.connect_and_login(SysInfraType::HistoryPlant).await {
            Ok(r) =>  client.readers.insert(SysInfraType::HistoryPlant, Arc::new(Mutex::new(r))),
            Err(_e) => {
                let _ = client.client.shutdown_all().await;
                return client
            }
        };
        let _order_receiver = match client.client.connect_and_login(SysInfraType::OrderPlant).await {
            Ok(r) =>  client.readers.insert(SysInfraType::OrderPlant, Arc::new(Mutex::new(r))),
            Err(_e) => {
                let _ = client.client.shutdown_all().await;
                return client
            }
        };
        let _pnl_receiver = match client.client.connect_and_login(SysInfraType::PnlPlant).await {
            Ok(r) =>  client.readers.insert(SysInfraType::PnlPlant, Arc::new(Mutex::new(r))),
            Err(_e) => {
                let _ = client.client.shutdown_all().await;
                return client
            }
        };
        client.run_start_up().await;
        client
    }

    pub fn is_long(&self, account_id: AccountId, symbol_name: SymbolName) -> bool {
        todo!()
    }

    pub fn is_short(&self, account_id: AccountId, symbol_name: SymbolName) -> bool {
        todo!()
    }

    pub fn is_flat(&self, account_id: AccountId, symbol_name: SymbolName) -> bool {
        todo!()
    }

    pub fn available_subscriptions(&self) -> Vec<DataSubscription> {
        vec![DataSubscription::new(SymbolName::from("NQ"), self.data_vendor.clone(), Resolution::Instant, BaseDataType::Ticks, MarketType::Futures(Exchange::CME))]
    }

    pub async fn send_callback(&self, stream_name: &StreamName, callback_id: u64, response: DataServerResponse) {
        if let Some(mut stream_map) = self.callbacks.get_mut(stream_name) {
            if let Some(sender) = stream_map.value_mut().remove(&callback_id) {
                match sender.send(response) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
        }
    }

    pub async fn register_callback(&self, stream_name: &StreamName, callback_id: u64, sender: oneshot::Sender<DataServerResponse>) {
        if let Some(mut stream_map) = self.callbacks.get_mut(stream_name) {
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
                match RithmicCredentials::load_credentials_from_file(&file_path) {
                    Ok(file) => Ok(file),
                    Err(e) => Err(FundForgeError::ServerErrorDebug(format!("Failed to load credentials for: {}", broker)))
                }
            },
            _ => Err(FundForgeError::ServerErrorDebug(format!("{} Incorrect brokerage to load rithmic credentials", broker)))
        }
    }

    pub async fn send_message<T: ProstMessage>(
        &self,
        plant: SysInfraType,
        message: T
    ) -> Result<(), RithmicApiError> {
        self.client.send_message(plant, message).await
    }

    async fn intermittent(&self) {
        //spawan a task, sleepuntil x minutes then runstartup, data upaters etc
    }

    pub async fn shutdown(&self) {
        match self.client.shutdown_all().await {
            Ok(_) => {}
            Err(e) => eprintln!("Rithmic Client shutdown error: {}", e)
        }
        RITHMIC_CLIENTS.remove(&self.system);
    }
}

#[async_trait]
impl BrokerApiResponse for RithmicClient {
    async fn symbols_response(&self, mode: StrategyMode, _stream_name: String, _market_type: MarketType, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn account_info_response(&self, mode: StrategyMode, _stream_name: String, _account_id: AccountId, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn symbol_info_response(&self, mode: StrategyMode, _stream_name: String, _symbol_name: SymbolName, _callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn margin_required_response(&self,  mode: StrategyMode, _stream_name: String, _symbol_name: SymbolName, _quantity: Volume, _callback_id: u64) -> DataServerResponse {
        todo!()
    }


    async fn accounts_response(&self, mode: StrategyMode, stream_name: String, callback_id: u64) -> DataServerResponse {
        todo!()
    }
}

#[async_trait]
impl VendorApiResponse for RithmicClient {
    async fn symbols_response(&self, mode: StrategyMode, stream_name: String, market_type: MarketType, callback_id: u64) -> DataServerResponse{
        const SYSTEM: SysInfraType = SysInfraType::TickerPlant;
        match mode {
            StrategyMode::Backtest => {

            }
            StrategyMode::LivePaperTrading | StrategyMode::Live => {
                match market_type {
                    MarketType::Futures(exchange) => {
                        let req = RequestProductCodes {
                            template_id: 111 ,
                            user_msg: vec![stream_name, callback_id.to_string()],
                            exchange: Some(exchange.to_string()),
                            give_toi_products_only: Some(true),
                        };
                    }
                    _ => return DataServerResponse::Error {callback_id, error: FundForgeError::ClientSideErrorDebug(format!("Incrorrect market type: {} for: {}", market_type, self.data_vendor))}
                }
            }
        }

        todo!()
    }

    async fn resolutions_response(&self, mode: StrategyMode, _stream_name: String, market_type: MarketType, callback_id: u64) -> DataServerResponse {
        let subs = match mode {
            StrategyMode::Backtest => {
                vec![]
            }
            StrategyMode::LivePaperTrading | StrategyMode::Live => {
                vec![
                    SubscriptionResolutionType::new(Resolution::Instant, BaseDataType::Quotes),
                    SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks),
                    SubscriptionResolutionType::new(Resolution::Seconds(1), BaseDataType::Candles),
                    SubscriptionResolutionType::new(Resolution::Minutes(1), BaseDataType::Candles),
                ]
            }
        };
        DataServerResponse::Resolutions {
            callback_id,
            subscription_resolutions_types: subs,
            market_type: MarketType::Forex,
        }
    }

    async fn markets_response(&self, mode: StrategyMode, _stream_name: String, _callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn decimal_accuracy_response(&self, mode: StrategyMode, _stream_name: String, _symbol_name: SymbolName, _callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn tick_size_response(&self, mode: StrategyMode, _stream_name: String, _symbol_name: SymbolName, _callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn data_feed_subscribe(&self, mode: StrategyMode,stream_name: String, subscription: DataSubscription, sender: Sender<DataServerResponse>) -> DataServerResponse {
       /* let req = RequestMarketDataUpdate {
            template_id: 100,
            user_msg: vec![],
            symbol: Some("NQ".to_string()),
            exchange: Some(Exchange::CME.to_string()),
            request: Some(Request::Subscribe.into()),
            update_bits: Some(2), 1 for ticks 2 for quotes
        };
        let req = RequestTimeBarUpdate {
        template_id: 200,
        user_msg: vec![],
        symbol: Some("NQ".to_string()),
        exchange: Some(Exchange::CME.to_string()),
        request: Some(Request::Subscribe.into()),
        bar_type: Some(1),
        bar_type_period: Some(5),
    };
    };


        */

        /*let available_subscriptions = self.available_subscriptions();
        if available_subscriptions.contains(&subscription) {
            return DataServerResponse::SubscribeResponse{ success: false, subscription: subscription.clone(), reason: Some(format!("This subscription is not available with DataVendor::Test: {}", subscription))}
        }
        if !self.data_feed_broadcasters.contains_key(&subscription) {
            self.data_feed_broadcasters.insert(subscription.clone(), Arc::new(StaticInternalBroadcaster::new()));
            self.data_feed_broadcasters.get(&subscription).unwrap().value().subscribe(stream_name, sender).await;
            println!("Subscribing: {}", subscription);
        } else {
            // If we already have a running task, we dont need a new one, we just subscribe to the broadcaster
            self.data_feed_broadcasters.get(&subscription).unwrap().value().subscribe(stream_name, sender).await;
            return DataServerResponse::SubscribeResponse{ success: true, subscription: subscription.clone(), reason: None}
        }
        println!("data_feed_subscribe Starting loop");
        let broadcasters = self.data_feed_broadcasters.clone();
        let broadcaster = self.data_feed_broadcasters.get(&subscription).unwrap().value().clone();
        tokio::task::spawn(async move {
            let mut last_time = utc_dt_1;
            'main_loop: while last_time < utc_dt_2 {
                let data_folder = PathBuf::from(get_data_folder());
                let file = BaseDataEnum::file_path(&data_folder, &subscription, &last_time).unwrap();
                let data = load_as_bytes(file.clone()).unwrap();
                let month_time_slices = BaseDataEnum::from_array_bytes(&data).unwrap();

                for mut base_data in month_time_slices {
                    last_time = base_data.time_created_utc();
                    match base_data {
                        BaseDataEnum::Quote(ref mut quote) => {
                            if broadcaster.has_subscribers() {
                                quote.time = Utc::now().to_string();
                                let response = DataServerResponse::DataUpdates(vec![base_data.clone()]);
                                broadcaster.broadcast(response).await;
                                sleep(Duration::from_millis(20)).await;
                            } else {
                                println!("No subscribers");
                                break 'main_loop;
                            }
                        }
                        _ => {}
                    }
                }
            }
            broadcasters.remove(&subscription_clone);
        });
        DataServerResponse::SubscribeResponse{ success: true, subscription: subscription_clone_2.clone(), reason: None}*/
        todo!()
    }

    async fn data_feed_unsubscribe(&self, mode: StrategyMode,_stream_name: String, _subscription: DataSubscription) -> DataServerResponse {
        todo!()
    }

    async fn base_data_types_response(&self, mode: StrategyMode, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        todo!()
    }
}
