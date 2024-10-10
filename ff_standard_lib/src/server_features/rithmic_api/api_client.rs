use std::path::PathBuf;
use std::sync::Arc;
use ahash::AHashMap;
use async_trait::async_trait;
use dashmap::DashMap;
use ff_rithmic_api::api_client::RithmicApiClient;
use ff_rithmic_api::credentials::RithmicCredentials;
use ff_rithmic_api::errors::RithmicApiError;
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
#[allow(unused_imports)]
use ff_rithmic_api::rithmic_proto_objects::rti::{AccountPnLPositionUpdate, RequestAccountList, RequestAccountRmsInfo, RequestLoginInfo, RequestPnLPositionSnapshot, RequestProductCodes, ResponseAccountRmsInfo};
use ff_rithmic_api::rithmic_proto_objects::rti::RequestMarketDataUpdate;
use ff_rithmic_api::systems::RithmicSystem;
use lazy_static::lazy_static;
use prost::Message as ProstMessage;
use rust_decimal_macros::dec;
use tokio::sync::mpsc::Sender;
use crate::standardized_types::broker_enum::Brokerage;
use crate::server_features::server_side_brokerage::BrokerApiResponse;
use crate::standardized_types::datavendor_enum::DataVendor;
use crate::server_features::server_side_datavendor::VendorApiResponse;
use crate::server_features::StreamName;
use crate::helpers::get_data_folder;
use crate::communicators::internal_broadcaster::StaticInternalBroadcaster;
use crate::strategies::ledgers::{AccountId, AccountInfo, Currency};
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use crate::standardized_types::enums::{FuturesExchange, MarketType, StrategyMode, SubscriptionResolutionType};
use crate::standardized_types::subscriptions::{DataSubscription, Symbol, SymbolName};
use crate::standardized_types::symbol_info::SymbolInfo;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use crate::helpers::converters::fund_forge_formatted_symbol_name;
use crate::server_features::rithmic_api::handle_history_plant::handle_responses_from_history_plant;
use crate::server_features::rithmic_api::handle_order_plant::handle_responses_from_order_plant;
use crate::server_features::rithmic_api::handle_pnl_plant::handle_responses_from_pnl_plant;
use crate::server_features::rithmic_api::handle_tick_plant::handle_responses_from_ticker_plant;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::new_types::Volume;
use crate::standardized_types::orders::{Order, OrderId};
use crate::standardized_types::resolution::Resolution;

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

    pub callbacks: DashMap<StreamName, AHashMap<u64, oneshot::Sender<DataServerResponse>>>,

    /// Rithmic clients
    pub client: Arc<RithmicApiClient>,
    pub symbol_info: DashMap<SymbolName, SymbolInfo>,
    pub handlers: DashMap<SysInfraType, JoinHandle<()>>,

    // accounts
    pub accounts: DashMap<AccountId, AccountInfo>,
    pub orders_open: DashMap<OrderId, Order>,

    //products
    pub products: DashMap<MarketType, Vec<Symbol>>,

    //subscribers
    pub tick_feed_broadcasters: Arc<DashMap<SymbolName, Arc<StaticInternalBroadcaster<BaseDataEnum>>>>,
    pub quote_feed_broadcasters: Arc<DashMap<SymbolName, Arc<StaticInternalBroadcaster<BaseDataEnum>>>>,
    pub candle_feed_broadcasters: Arc<DashMap<SymbolName, Arc<StaticInternalBroadcaster<BaseDataEnum>>>>,
}

impl RithmicClient {
    pub async fn new(
        system: RithmicSystem,
        aggregated_quotes: bool,
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

        let client = RithmicApiClient::new(credentials.clone(), aggregated_quotes, server_domains_toml).unwrap();
        let client = Self {
            brokerage,
            data_vendor,
            system,
            fcm_id: credentials.fcm_id.clone(),
            ib_id: credentials.ib_id.clone(),
            user_type: credentials.user_type.clone(),
            callbacks: Default::default(),
            client: Arc::new(client),
            symbol_info: Default::default(),
            tick_feed_broadcasters: Default::default(),
            quote_feed_broadcasters: Default::default(),
            accounts: Default::default(),
            orders_open: Default::default(),
            products: Default::default(),
            candle_feed_broadcasters: Arc::new(Default::default()),
            handlers: DashMap::with_capacity(5),
        };
        Ok(client)
    }

    //todo run start up... we might need to connect to all plants for this..
    pub async fn run_start_up(client: Arc<Self>, connect_accounts: bool, connect_data: bool) -> Result<(), FundForgeError> {
        if connect_data {
            match client.client.connect_and_login(SysInfraType::TickerPlant).await {
                Ok(r) => {
                    client.handlers.insert(SysInfraType::TickerPlant, handle_responses_from_ticker_plant(client.clone(), r).await);
                },
                Err(e) => {
                    return Err(FundForgeError::ServerErrorDebug(e.to_string()))
                }
            }

            match client.client.connect_and_login(SysInfraType::HistoryPlant).await {
                Ok(r) => {
                    client.handlers.insert(SysInfraType::HistoryPlant, handle_responses_from_history_plant(client.clone(), r).await);
                },
                Err(e) => {
                    return Err(FundForgeError::ServerErrorDebug(e.to_string()))
                }
            }
        }

        if connect_accounts {
            match client.client.connect_and_login(SysInfraType::OrderPlant).await {
                Ok(r) => {
                    client.handlers.insert(SysInfraType::OrderPlant, handle_responses_from_order_plant(client.clone(), r).await);
                },
                Err(e) => {
                    return Err(FundForgeError::ServerErrorDebug(e.to_string()))
                }
            }

            match client.client.connect_and_login(SysInfraType::PnlPlant).await {
                Ok(r) => {
                    client.handlers.insert(SysInfraType::PnlPlant, handle_responses_from_pnl_plant(client.clone(), r).await);
                },
                Err(e) => {
                    return Err(FundForgeError::ServerErrorDebug(e.to_string()))
                }
            }
        }

        let rms_req = RequestAccountRmsInfo {
            template_id: 304,
            user_msg: vec![],
            fcm_id: client.fcm_id.clone(),
            ib_id: client.ib_id.clone(),
            user_type: client.user_type.clone(),
        };
        client.client.send_message(SysInfraType::OrderPlant, rms_req).await.unwrap();

        Ok(())
    }

    pub async fn send_callback(&self, stream_name: StreamName, callback_id: u64, response: DataServerResponse) {
        if let Some(mut stream_map) = self.callbacks.get_mut(&stream_name) {
            if let Some(sender) = stream_map.value_mut().remove(&callback_id) {
                match sender.send(response) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Callback error: {:?} Dumping subscriber: {}", e, stream_name);
                        self.callbacks.remove(&stream_name);
                        for broadcaster in self.tick_feed_broadcasters.iter() {
                            broadcaster.unsubscribe(&stream_name.to_string());
                        }
                        for broadcaster in self.quote_feed_broadcasters.iter() {
                            broadcaster.unsubscribe(&stream_name.to_string());
                        }
                        for broadcaster in self.candle_feed_broadcasters.iter() {
                            broadcaster.unsubscribe(&stream_name.to_string());
                        }
                    }
                }
            }
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
        plant: SysInfraType,
        message: T
    ) -> Result<(), RithmicApiError> {
        self.client.send_message(plant, message).await
    }

    #[allow(dead_code)]
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
    async fn symbol_names_response(&self, _mode: StrategyMode, _stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        //todo get dynamically from server using stream name to fwd callback
        DataServerResponse::SymbolNames {
            callback_id,
            symbol_names: vec!["M6E".to_string()],
        }
    }

    async fn account_info_response(&self, _mode: StrategyMode, _stream_name: StreamName, _account_id: AccountId, _callback_id: u64) -> DataServerResponse {
        todo!()
    }


    async fn symbol_info_response(
        &self,
        _mode: StrategyMode,
        _stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse {
        //todo get dynamically from server using stream name to fwd callback
        let symbol_name = fund_forge_formatted_symbol_name(&symbol_name);
        let (pnl_currency, value_per_tick, tick_size) = match symbol_name.as_str() {
            "M6E" => (Currency::USD, dec!(1.0), dec!(0.0001)), // EUR/USD with $1 per tick
            _ => todo!()       // Default values
        };

        let symbol_info = SymbolInfo {
            symbol_name,
            pnl_currency,
            value_per_tick,
            tick_size,
            decimal_accuracy: 4,
        };

        DataServerResponse::SymbolInfo {
            callback_id,
            symbol_info,
        }
    }

    async fn margin_required_response(&self,  _mode: StrategyMode, _stream_name: StreamName, symbol_name: SymbolName, _quantity: Volume, callback_id: u64) -> DataServerResponse {
        //todo get dynamically from server using stream name to fwd callback
        DataServerResponse::MarginRequired {
            callback_id,
            symbol_name,
            price: dec!(0.0),
        }
    }

    async fn accounts_response(&self, _mode: StrategyMode, _stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        // The accounts are collected on initializing the client
        let accounts = self.accounts.iter().map(|entry| entry.key().clone()).collect();
        DataServerResponse::Accounts {
            callback_id,
            accounts,
        }
    }

    async fn logout_command(&self, stream_name: StreamName) {
        //todo handle dynamically from server using stream name to remove subscriptions and callbacks
        self.callbacks.remove(&stream_name);
        for broadcaster in self.tick_feed_broadcasters.iter() {
            broadcaster.unsubscribe(&stream_name.to_string());
        }
        for broadcaster in self.quote_feed_broadcasters.iter() {
            broadcaster.unsubscribe(&stream_name.to_string());
        }
        for broadcaster in self.candle_feed_broadcasters.iter() {
            broadcaster.unsubscribe(&stream_name.to_string());
        }
        for handle in &self.handlers {
            handle.abort();
        }
    }
}
#[allow(dead_code)]
#[async_trait]
impl VendorApiResponse for RithmicClient {
    async fn symbols_response(&self, mode: StrategyMode, stream_name: StreamName, market_type: MarketType, callback_id: u64) -> DataServerResponse{
        const SYSTEM: SysInfraType = SysInfraType::TickerPlant;
        match mode {
            StrategyMode::Backtest => {

            }
            StrategyMode::LivePaperTrading | StrategyMode::Live => {
                match market_type {
                    MarketType::Futures(exchange) => {
                        let _req = RequestProductCodes {
                            template_id: 111 ,
                            user_msg: vec![stream_name.to_string(), callback_id.to_string()],
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

    async fn resolutions_response(&self, mode: StrategyMode, _stream_name: StreamName, _market_type: MarketType, callback_id: u64) -> DataServerResponse {
        let subs = match mode {
            StrategyMode::Backtest => {
                vec![]
            }
            StrategyMode::LivePaperTrading | StrategyMode::Live => {
                vec![
                    SubscriptionResolutionType::new(Resolution::Instant, BaseDataType::Quotes),
                    SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks),
                    SubscriptionResolutionType::new(Resolution::Seconds(1), BaseDataType::Candles),
                ]
            }
        };
        DataServerResponse::Resolutions {
            callback_id,
            subscription_resolutions_types: subs,
            market_type: MarketType::Forex,
        }
    }

    async fn markets_response(&self, _mode: StrategyMode, _stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        DataServerResponse::Markets {
            callback_id,
            markets: vec![
                MarketType::Futures(FuturesExchange::CME),
                MarketType::Futures(FuturesExchange::CBOT),
                MarketType::Futures(FuturesExchange::COMEX),
                MarketType::Futures(FuturesExchange::NYBOT),
                MarketType::Futures(FuturesExchange::NYMEX),
                MarketType::Futures(FuturesExchange::MGEX)
            ],
        }
    }

    async fn decimal_accuracy_response(&self, _mode: StrategyMode, _stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        //todo get dynamically from server using stream name to fwd callback
        let accuracy = match symbol_name.as_str() {
            "M6E" => 4,
            _ => todo!(),
        };
        DataServerResponse::DecimalAccuracy {
            callback_id,
            accuracy,
        }
    }

    async fn tick_size_response(&self, _mode: StrategyMode, _stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        //todo get dynamically from server using stream name to fwd callback
        let tick_size = match symbol_name.as_str() {
            "M6E" => dec!(0.0001),
            _ => todo!(),
        };
        DataServerResponse::TickSize {
            callback_id,
            tick_size,
        }
    }

    async fn data_feed_subscribe(&self, stream_name: StreamName, subscription: DataSubscription, sender: Sender<BaseDataEnum>) -> DataServerResponse {
        let exchange = match subscription.market_type {
            MarketType::Futures(exchange) => {
                exchange.to_string()
            }
            _ => todo!()
        };
        let req = RequestMarketDataUpdate {
            template_id: 100,
            user_msg: vec![],
            symbol: Some(subscription.symbol.name.to_string()),
            exchange: Some(exchange),
            request: Some(1), //1 subscribe 2 unsubscribe
            update_bits: Some(1), //1 for ticks 2 for quotes
        };

        //todo dont send if already subscribed
        self.send_message(SysInfraType::TickerPlant, req).await.unwrap();

 /*       let req = RequestTimeBarUpdate {
        template_id: 200,
        user_msg: vec![],
        symbol: Some("NQ".to_string()),
        exchange: Some(Exchange::CME.to_string()),
        request: Some(Request::Subscribe.into()),
        bar_type: Some(1),
        bar_type_period: Some(5),
    };*/

        //todo if not working try resolution Instant
        let available_subscriptions = vec![DataSubscription::new(SymbolName::from("M6E"), self.data_vendor.clone(), Resolution::Ticks(1), BaseDataType::Ticks, MarketType::Futures(FuturesExchange::CME))];
        if available_subscriptions.contains(&subscription) {
            return DataServerResponse::SubscribeResponse{ success: false, subscription: subscription.clone(), reason: Some(format!("This subscription is not available with DataVendor::Test: {}", subscription))}
        }

        //todo have a unique function per base data type.
        match subscription.base_data_type {
            BaseDataType::Ticks => {
                let broadcaster = self.tick_feed_broadcasters
                    .entry(subscription.symbol.name.clone())
                    .or_insert_with(|| {
                        println!("Subscribing: {}", subscription);
                        Arc::new(StaticInternalBroadcaster::new())
                    });
                broadcaster.value().subscribe(stream_name.to_string(), sender);
            }
            BaseDataType::Quotes => {
                let broadcaster = self.quote_feed_broadcasters
                    .entry(subscription.symbol.name.clone())
                    .or_insert_with(|| {
                        println!("Subscribing: {}", subscription);
                        Arc::new(StaticInternalBroadcaster::new())
                    });
                broadcaster.value().subscribe(stream_name.to_string(), sender);
            }
            BaseDataType::Candles => {
                let broadcaster = self.candle_feed_broadcasters
                    .entry(subscription.symbol.name.clone())
                    .or_insert_with(|| {
                        println!("Subscribing: {}", subscription);
                        Arc::new(StaticInternalBroadcaster::new())
                    });
                broadcaster.value().subscribe(stream_name.to_string(), sender);
            }
            _ => todo!("Handle gracefully by returning err")
        }


        DataServerResponse::SubscribeResponse{ success: true, subscription: subscription.clone(), reason: None}
    }

    async fn data_feed_unsubscribe(&self, _mode: StrategyMode, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
        let exchange = match subscription.market_type {
            MarketType::Futures(exchange) => {
                exchange.to_string()
            }
            _ => todo!()
        };
        let req = RequestMarketDataUpdate {
            template_id: 100,
            user_msg: vec![],
            symbol: Some(subscription.symbol.name.to_string()),
            exchange: Some(exchange),
            request: Some(2), //1 subscribe 2 unsubscribe
            update_bits: Some(1), //1 for ticks 2 for quotes
        };
        match subscription.base_data_type {
            BaseDataType::Ticks => {
                if let Some(broadcaster) = self.tick_feed_broadcasters.get(&subscription.symbol.name) {
                    broadcaster.value().unsubscribe(&stream_name.to_string());
                    if !broadcaster.has_subscribers() {
                        self.quote_feed_broadcasters.remove(&subscription.symbol.name);
                        self.send_message(SysInfraType::TickerPlant, req).await.unwrap();
                    }
                    return DataServerResponse::UnSubscribeResponse {
                        success: true,
                        subscription,
                        reason: None,
                    }
                }
            }
            BaseDataType::Quotes => {
                if let Some(broadcaster) = self.quote_feed_broadcasters.get(&subscription.symbol.name) {
                    broadcaster.value().unsubscribe(&stream_name.to_string());
                    if !broadcaster.has_subscribers() {
                        self.quote_feed_broadcasters.remove(&subscription.symbol.name);
                        self.send_message(SysInfraType::TickerPlant, req).await.unwrap();
                    }
                    return DataServerResponse::UnSubscribeResponse {
                        success: true,
                        subscription,
                        reason: None,
                    }
                }
            }
            BaseDataType::Candles => {
                if let Some (broadcaster) = self.candle_feed_broadcasters.get(&subscription.symbol.name) {
                    broadcaster.value().unsubscribe(&stream_name.to_string());
                    if !broadcaster.has_subscribers() {
                        self.quote_feed_broadcasters.remove(&subscription.symbol.name);
                        self.send_message(SysInfraType::TickerPlant, req).await.unwrap();
                    }
                    return DataServerResponse::UnSubscribeResponse {
                        success: true,
                        subscription,
                        reason: None,
                    }
                }
            }
            _ => todo!("Handle gracefully by returning err")
        }
        DataServerResponse::UnSubscribeResponse {
            success: false,
            subscription,
            reason: Some(String::from("Subscription Not Found or Incorrect type for Rithmic")),
        }
    }

    async fn base_data_types_response(&self, _mode: StrategyMode, _stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        //todo get dynamically from server using stream name to fwd callback
        DataServerResponse::BaseDataTypes {
            callback_id,
            base_data_types: vec![BaseDataType::Candles, BaseDataType::Ticks, BaseDataType::Quotes],
        }
    }

    async fn logout_command(&self, stream_name: StreamName) {
        //todo handle dynamically from server using stream name to remove subscriptions and callbacks
        self.callbacks.remove(&stream_name);
        for broadcaster in self.tick_feed_broadcasters.iter() {
            broadcaster.unsubscribe(&stream_name.to_string());
        }
        for broadcaster in self.quote_feed_broadcasters.iter() {
            broadcaster.unsubscribe(&stream_name.to_string());
        }
        for broadcaster in self.candle_feed_broadcasters.iter() {
            broadcaster.unsubscribe(&stream_name.to_string());
        }

        for handle in &self.handlers {
            handle.abort();
        }
    }
}
