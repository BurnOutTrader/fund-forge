use std::collections::BTreeMap;
use std::fs;
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
use ff_rithmic_api::rithmic_proto_objects::rti::{RequestMarketDataUpdate};
use ff_rithmic_api::systems::RithmicSystem;
use structopt::lazy_static::lazy_static;
use tokio::sync::{broadcast, oneshot};
use ff_standard_lib::helpers::get_data_folder;
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::enums::{FuturesExchange, MarketType, StrategyMode, SubscriptionResolutionType};
use ff_standard_lib::standardized_types::orders::{Order, OrderId};
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol, SymbolName};
use ff_standard_lib::standardized_types::symbol_info::SymbolInfo;
use ff_standard_lib::strategies::handlers::market_handlers::BookLevel;
use ff_standard_lib::strategies::ledgers::{AccountId, AccountInfo, Currency};
use ff_standard_lib::StreamName;
use crate::rithmic_api::handle_history_plant::handle_responses_from_history_plant;
use crate::rithmic_api::handle_order_plant::handle_responses_from_order_plant;
use crate::rithmic_api::handle_pnl_plant::handle_responses_from_pnl_plant;
use crate::rithmic_api::handle_tick_plant::handle_responses_from_ticker_plant;
use prost::Message as ProstMessage;
use rust_decimal_macros::dec;
use ff_standard_lib::helpers::converters::fund_forge_formatted_symbol_name;
use ff_standard_lib::server_features::server_side_brokerage::BrokerApiResponse;
use ff_standard_lib::server_features::server_side_datavendor::VendorApiResponse;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::new_types::Volume;
use ff_standard_lib::standardized_types::resolution::Resolution;
use crate::stream_tasks::{subscribe_stream, unsubscribe_stream};

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
        system: RithmicSystem
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

    //todo run start up... we might need to connect to all plants for this..
    pub async fn run_start_up(client: Arc<Self>, connect_accounts: bool, connect_data: bool) -> Result<(), FundForgeError> {
        if connect_data {
            match client.client.connect_and_login(SysInfraType::TickerPlant).await {
                Ok(r) => {
                   tokio::spawn(handle_responses_from_ticker_plant(client.clone(), r));
                },
                Err(e) => {
                    return Err(FundForgeError::ServerErrorDebug(e.to_string()))
                }
            }

            match client.client.connect_and_login(SysInfraType::HistoryPlant).await {
                Ok(r) => {
                    tokio::spawn(handle_responses_from_history_plant(client.clone(), r));
                },
                Err(e) => {
                    return Err(FundForgeError::ServerErrorDebug(e.to_string()))
                }
            }
        }

        if connect_accounts {
            match client.client.connect_and_login(SysInfraType::OrderPlant).await {
                Ok(r) => {
                    tokio::spawn(handle_responses_from_order_plant(client.clone(), r));
                },
                Err(e) => {
                    return Err(FundForgeError::ServerErrorDebug(e.to_string()))
                }
            }

            match client.client.connect_and_login(SysInfraType::PnlPlant).await {
                Ok(r) => {
                    tokio::spawn(handle_responses_from_pnl_plant(client.clone(), r));
                },
                Err(e) => {
                    return Err(FundForgeError::ServerErrorDebug(e.to_string()))
                }
            }
        }

        Ok(())
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
        plant: SysInfraType,
        message: T
    ) -> Result<(), RithmicApiError> {
        self.client.send_message(plant, message).await
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
            symbol_names: vec!["MNQ".to_string()],
        }
    }

    async fn account_info_response(&self, mode: StrategyMode, _stream_name: StreamName, account_id: AccountId, callback_id: u64) -> DataServerResponse {
        //todo use match mode to create sim account
        match mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                todo!("Not implemented for backtest")
            }
            StrategyMode::Live => {
                match self.accounts.get(&account_id) {
                    None => DataServerResponse::Error {callback_id, error:FundForgeError::ClientSideErrorDebug(format!("{} Has No Account for {}",self.brokerage, account_id))},
                    Some(account_info) => DataServerResponse::AccountInfo {
                        callback_id,
                        account_info: account_info.value().clone(),
                    }
                }
            }
        }
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
        let (pnl_currency, value_per_tick, tick_size, decimal_accuracy) = match symbol_name.as_str() {
            "MNQ" => (Currency::USD, dec!(2.0), dec!(0.25), 2), // MNQ in USD with $2 per tick at 0.25 tick size and decimal accuracy of 2
            _ => todo!()       // Default values
        };

        let symbol_info = SymbolInfo {
            symbol_name,
            pnl_currency,
            value_per_tick,
            tick_size,
            decimal_accuracy,
        };

        DataServerResponse::SymbolInfo {
            callback_id,
            symbol_info,
        }
    }

    async fn margin_required_response(&self,  _mode: StrategyMode, _stream_name: StreamName, symbol_name: SymbolName, quantity: Volume, callback_id: u64) -> DataServerResponse {
        //todo get dynamically from server using stream name to fwd callback
        DataServerResponse::MarginRequired {
            callback_id,
            symbol_name,
            price: quantity * dec!(150.0),
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
                vec![
                    SubscriptionResolutionType::new(Resolution::Instant, BaseDataType::Quotes),
                    SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks),
                ]
            }
            StrategyMode::LivePaperTrading | StrategyMode::Live => {
                vec![
                    SubscriptionResolutionType::new(Resolution::Instant, BaseDataType::Quotes),
                    SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks),
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
            "MNQ" => 2,
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
            "MNQ" => dec!(0.25),
            _ => todo!(),
        };
        DataServerResponse::TickSize {
            callback_id,
            tick_size,
        }
    }

    async fn data_feed_subscribe(&self, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
        let exchange = match subscription.market_type {
            MarketType::Futures(exchange) => {
                exchange.to_string()
            }
            _ => todo!()
        };


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
        let available_subscriptions = vec![
            DataSubscription::new(SymbolName::from("MNQ"), self.data_vendor.clone(), Resolution::Ticks(1), BaseDataType::Ticks, MarketType::Futures(FuturesExchange::CME)),
            DataSubscription::new(SymbolName::from("MNQ"), self.data_vendor.clone(), Resolution::Instant, BaseDataType::Quotes, MarketType::Futures(FuturesExchange::CME))
        ];
        if !available_subscriptions.contains(&subscription) {
            eprintln!("Rithmic Subscription Not Available: {:?}", subscription);
            return DataServerResponse::SubscribeResponse{ success: false, subscription: subscription.clone(), reason: Some(format!("This subscription is not available with DataVendor::Test: {}", subscription))}
        }

        let mut is_subscribed = true;
        //todo have a unique function per base data type.
        match subscription.base_data_type {
            BaseDataType::Ticks => {
                if let Some(broadcaster) = self.tick_feed_broadcasters.get(&subscription.symbol.name) {
                    let receiver = broadcaster.value().subscribe();
                    subscribe_stream(&stream_name, subscription.clone(), receiver).await;
                    is_subscribed = false;
                } else {
                    let (sender, receiver) = broadcast::channel(500);
                    self.tick_feed_broadcasters.insert(subscription.symbol.name.clone(), sender);
                    subscribe_stream(&stream_name, subscription.clone(), receiver).await;
                }
            }
            BaseDataType::Quotes => {
                if let Some(broadcaster) = self.quote_feed_broadcasters.get(&subscription.symbol.name) {
                    let receiver = broadcaster.value().subscribe();
                    subscribe_stream(&stream_name, subscription.clone(), receiver).await;
                    is_subscribed = false;
                } else {
                    let (sender, receiver) = broadcast::channel(500);
                    self.quote_feed_broadcasters.insert(subscription.symbol.name.clone(), sender);
                    subscribe_stream(&stream_name, subscription.clone(), receiver).await;
                }
            }
            BaseDataType::Candles => {
                if let Some(broadcaster) = self.candle_feed_broadcasters.get(&subscription.symbol.name) {
                    let receiver = broadcaster.value().subscribe();
                    subscribe_stream(&stream_name, subscription.clone(), receiver).await;
                    is_subscribed = false;
                } else {
                    let (sender, receiver) = broadcast::channel(500);
                    self.candle_feed_broadcasters.insert(subscription.symbol.name.clone(), sender);
                    subscribe_stream(&stream_name, subscription.clone(), receiver).await;
                }
            }
            _ => todo!("Handle gracefully by returning err")
        }

        if !is_subscribed {
            let bits = match subscription.base_data_type {
                BaseDataType::Ticks => 1,
                BaseDataType::Quotes => 2,
                _ => return DataServerResponse::SubscribeResponse{ success: false, subscription: subscription.clone(), reason: Some(format!("This subscription is not available with DataVendor::Test: {}", subscription))}
                //BaseDataType::Candles => {}
            };
            let req = RequestMarketDataUpdate {
                template_id: 100,
                user_msg: vec![],
                symbol: Some(subscription.symbol.name.to_string()),
                exchange: Some(exchange),
                request: Some(1), //1 subscribe 2 unsubscribe
                update_bits: Some(bits), //1 for ticks 2 for quotes
            };
            //todo Fix ff_rithmic_api switch heartbeat fn. this causes a lock.
       /*     match self.client.switch_heartbeat_required(SysInfraType::TickerPlant, false).await {
                Ok(_) => {}
                Err(_) => {}
            }*/
            match self.send_message(SysInfraType::TickerPlant, req).await {
                Ok(_) => {
                    println!("Subscribed to new ticker subscription");
                }
                Err(e) => {
                    eprintln!("Error sending subscribe request to ticker plant: {}", e);
                }
            }
        }
        println!("{} Subscribed: {}", stream_name, subscription);
        DataServerResponse::SubscribeResponse{ success: true, subscription: subscription.clone(), reason: None}
    }

    async fn data_feed_unsubscribe(&self, _mode: StrategyMode, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
        let exchange = match subscription.market_type {
            MarketType::Futures(exchange) => exchange.to_string(),
            _ => return DataServerResponse::SubscribeResponse {
                success: false,
                subscription: subscription.clone(),
                reason: Some(format!("Unsupported market type: {:?}", subscription.market_type)),
            },
        };

        unsubscribe_stream(&stream_name, &subscription).await;

        let (bits, broadcaster_map) = match subscription.base_data_type {
            BaseDataType::Ticks => (1, &self.tick_feed_broadcasters),
            BaseDataType::Quotes => (2, &self.quote_feed_broadcasters),
            BaseDataType::Candles => (3, &self.candle_feed_broadcasters),
            _ => return DataServerResponse::SubscribeResponse {
                success: false,
                subscription: subscription.clone(),
                reason: Some(format!("Unsupported data type: {:?}", subscription.base_data_type)),
            },
        };

        let symbol = subscription.symbol.name.clone();
        let mut should_disconnect = false;

        if let Some(broadcaster) = broadcaster_map.get_mut(&symbol) {
            should_disconnect = broadcaster.receiver_count() == 0;
        }

        if should_disconnect {
            broadcaster_map.remove(&symbol);

            let req = RequestMarketDataUpdate {
                template_id: 100,
                user_msg: vec![],
                symbol: Some(symbol.clone()),
                exchange: Some(exchange),
                request: Some(2), // 2 for unsubscribe
                update_bits: Some(bits),
            };

            if let Err(e) = self.send_message(SysInfraType::TickerPlant, req).await {
                return DataServerResponse::UnSubscribeResponse {
                    success: false,
                    subscription,
                    reason: Some(format!("Failed to send unsubscribe message: {}", e)),
                };
            }

            // Additional cleanup for quotes
            if subscription.base_data_type == BaseDataType::Quotes {
                self.ask_book.remove(&symbol);
                self.bid_book.remove(&symbol);
            }
        }

        // Check if we need to switch heartbeat
        if self.tick_feed_broadcasters.is_empty() &&
            self.quote_feed_broadcasters.is_empty() &&
            self.candle_feed_broadcasters.is_empty()
        {
            //todo fix in ff_rithmic api this causes a lock
         /*   if let Err(e) = self.client.switch_heartbeat_required(SysInfraType::TickerPlant, true).await {
                eprintln!("Failed to switch heartbeat: {}", e);
            }*/
        }

        DataServerResponse::UnSubscribeResponse {
            success: true,
            subscription,
            reason: None,
        }
    }

    async fn base_data_types_response(&self, _mode: StrategyMode, _stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        //todo get dynamically from server using stream name to fwd callback
        DataServerResponse::BaseDataTypes {
            callback_id,
            base_data_types: vec![BaseDataType::Ticks, BaseDataType::Quotes],
        }
    }

    async fn logout_command_vendors(&self, stream_name: StreamName) {
        self.callbacks.remove(&stream_name);
    }
}
