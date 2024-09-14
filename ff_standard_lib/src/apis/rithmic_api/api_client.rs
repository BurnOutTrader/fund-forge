use std::sync::Arc;
use ahash::AHashMap;
use dashmap::DashMap;
use ff_rithmic_api::api_client::RithmicApiClient;
use ff_rithmic_api::credentials::RithmicCredentials;
use ff_rithmic_api::errors::RithmicApiError;
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
use ff_rithmic_api::rithmic_proto_objects::rti::RequestGiveTickSizeTypeTable;
use futures::channel::oneshot;
use futures::stream::SplitStream;
use prost::Message as ProstMessage;
use tokio::net::TcpStream;
use crate::apis::brokerage::client_requests::ClientSideBrokerage;
use crate::apis::brokerage::server_responses::BrokerApiResponse;
use crate::apis::brokerage::SymbolInfo;
use crate::apis::vendor::client_requests::ClientSideDataVendor;
use crate::apis::vendor::server_responses::VendorApiResponse;
use crate::standardized_types::accounts::ledgers::{AccountId, Currency, Ledger};
use crate::standardized_types::data_server_messaging::{FundForgeError, SynchronousResponseType};
use crate::standardized_types::enums::{MarketType, SubscriptionResolutionType};
use crate::standardized_types::Price;
use crate::standardized_types::subscriptions::{Symbol, SymbolName};

type ResponseRequestId = u64;
pub struct RithmicClient {
    pub client: Arc<RithmicApiClient>,
    pub tick_size: AHashMap<SymbolName, f64>,

    /// ResponseRequestId will be attached to the request and so should return with the rithmic response
    synchronous_request_map: Arc<DashMap<ResponseRequestId, oneshot::Sender<SynchronousResponseType>>>,
    synchronous_request_counter: Arc<std::sync::Mutex<ResponseRequestId>>,
}

impl RithmicClient {
    pub async fn new() -> Self {
        let credentials: RithmicCredentials = RithmicCredentials::load_credentials_from_file(&"rithmic_credentials.toml".to_string()).unwrap();
        let client = Self {
            client: Arc::new(RithmicApiClient::new(credentials)),
            tick_size: Default::default(),
            synchronous_request_map: Arc::new(DashMap::new()),
            synchronous_request_counter: Arc::new(std::sync::Mutex::new(0)),
        };
        const MAIN_RITHMIC_PLANTS: Vec<SysInfraType> = vec![SysInfraType::HistoryPlant, SysInfraType::OrderPlant, SysInfraType::PnlPlant, SysInfraType::TickerPlant];
        for plant in MAIN_RITHMIC_PLANTS {
            match client.client.connect_and_login(plant).await {
                Ok(_) => {}
                Err(e) => eprintln!("{}", e)
            }
        }
        client
    }

    // Generate a unique request ID
    fn register_synchronous_request(&self) -> ResponseRequestId {
        let mut counter = self.synchronous_request_counter.lock().unwrap();
        *counter += 1;
        *counter
    }

    // Send a request and wait for a response
    pub async fn send_synchronous_request<T: ProstMessage + Default>(&self, plant: &SysInfraType, request: T) -> Result<SynchronousResponseType, String> {
        let request_id = self.get_next_request_id();

        // Create a oneshot channel to receive the response
        let (tx, rx) = oneshot::channel();

        // Store the sender in the map so the stream can send the response back
        self.synchronous_request_map.insert(request_id, tx);

        //send request to api client
        match self.client.send_message(plant, request).await {
            Ok(_) => {}
            Err(_) => {}
        }

        // Wait for the response
        match rx.await {
            Ok(response) => Ok(response),
            Err(_) => Err("Failed to receive response".to_string()),
        }
    }
}

impl BrokerApiResponse for RithmicClient {
    async fn symbols_response(&self, market_type: MarketType) -> Result<SynchronousResponseType, FundForgeError> {
        todo!()
    }

    async fn account_currency_response(&self, account_id: AccountId) -> Result<SynchronousResponseType, FundForgeError> {
        todo!()
    }

    async fn account_info_response(&self, account_id: AccountId) -> Result<SynchronousResponseType, FundForgeError> {
        todo!()
    }

    async fn symbol_info_response(&self, symbol_name: SymbolName) -> Result<SynchronousResponseType, FundForgeError> {
        todo!()
    }
}

impl VendorApiResponse for RithmicClient {
    async fn basedata_symbols_response(&self, market_type: MarketType) -> Result<SynchronousResponseType, FundForgeError> {
        todo!()
    }

    async fn resolutions_response(&self, market_type: MarketType) -> Result<SynchronousResponseType, FundForgeError> {
        todo!()
    }

    async fn markets_response(&self) -> Result<SynchronousResponseType, FundForgeError> {
        todo!()
    }

    async fn decimal_accuracy_response(&self, symbol_name: SymbolName) -> Result<SynchronousResponseType, FundForgeError> {
        todo!()
    }

    async fn tick_size_response(&self, symbol_name: SymbolName) -> Result<SynchronousResponseType, FundForgeError> {
        let request = RequestGiveTickSizeTypeTable {
            template_id: 107,
            user_msg: vec![],
            tick_size_type: None,
        }
    }
}
