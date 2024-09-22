use std::sync::Arc;
use async_std::task::block_on;
use async_trait::async_trait;
use dashmap::DashMap;
use ff_rithmic_api::api_client::RithmicApiClient;
use ff_rithmic_api::credentials::RithmicCredentials;
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
use lazy_static::lazy_static;
use prost::Message as ProstMessage;
use tokio::io::WriteHalf;
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tokio_rustls::server::TlsStream;
use crate::apis::brokerage::broker_enum::Brokerage;
use crate::apis::brokerage::server_side_brokerage::BrokerApiResponse;
use crate::apis::data_vendor::datavendor_enum::DataVendor;
use crate::apis::data_vendor::server_side_datavendor::VendorApiResponse;
use crate::standardized_types::accounts::ledgers::{AccountId};
use crate::standardized_types::data_server_messaging::{FundForgeError, DataServerResponse};
use crate::standardized_types::enums::{MarketType};
use crate::standardized_types::subscriptions::{DataSubscription, SymbolName};
use crate::standardized_types::symbol_info::SymbolInfo;
use crate::standardized_types::Volume;

lazy_static! {
    pub static ref RITHMIC_CLIENTS: DashMap<Brokerage , Arc<RithmicClient>> = DashMap::with_capacity(3);
}

pub fn get_rithmic_client(data_vendor: &DataVendor) -> Option<Arc<RithmicClient>> {
    match data_vendor {
        DataVendor::RithmicTest => {
            if let Some(client) = RITHMIC_CLIENTS.get(&Brokerage::RithmicTest) {
                return Some(client.value().clone())
            }
            None
        }
        DataVendor::Test => panic!("Incorrect vendor for this fn")
    }
}

pub struct RithmicClient {
    /// The primary client is the only client used for data feeds, it will also have all brokerage features.
    pub client: Arc<RithmicApiClient>,
    pub symbol_info: DashMap<SymbolName, SymbolInfo>,
    pub callbacks: DashMap<String, DashMap<u64, Arc<Mutex<WriteHalf<TlsStream<TcpStream>>>>>>
}

impl RithmicClient {
    pub fn add_callback(&self, stream_name: String, id: u64, client_ref: Arc<Mutex<WriteHalf<TlsStream<TcpStream>>>>) {
        if let Some(map) = self.callbacks.get(&stream_name) {
            map.insert(id, client_ref);
        } else {
            let mut map = DashMap::new();
            map.insert(id, client_ref);
            self.callbacks.insert(stream_name.clone(), map);
        }
    }

    pub fn get_callback_client(&self, stream_name: &str, id: u64) -> Option<Arc<Mutex<WriteHalf<TlsStream<TcpStream>>>>> {
        if let Some(map) = self.callbacks.get(stream_name) {
            if let Some((_, client)) = map.value().remove(&id) {
                return Some(client)
            }
        }
        None
    }
    fn rithmic_credentials(broker: &Brokerage) -> RithmicCredentials {
        let file = format!("rithmic_credentials_{}.toml", broker.to_string().to_lowercase());
        let file_path = format!("rithmic_credentials/{}", file);
        RithmicCredentials::load_credentials_from_file(&file_path).unwrap()
    }
    pub fn new(broker: &Brokerage) -> Self {
        let credentials = RithmicClient::rithmic_credentials(&broker);
        let client = RithmicApiClient::new(credentials);
        let client = Self {
            client: Arc::new(client),
            symbol_info: Default::default(),
            callbacks: Default::default(),
        };
        let ticker_receiver = match block_on(client.client.connect_and_login(SysInfraType::TickerPlant)) {
            Ok(r) => r,
            Err(e) => {
                let _ = block_on(client.client.shutdown_all());
                return client
            }
        };
        let history_receiver = match block_on(client.client.connect_and_login(SysInfraType::HistoryPlant)) {
            Ok(r) => r,
            Err(e) => {
                let _ = block_on(client.client.shutdown_all());
                return client
            }
        };
        let order_receiver = match block_on(client.client.connect_and_login(SysInfraType::OrderPlant)) {
            Ok(r) => r,
            Err(e) => {
                let _ = block_on(client.client.shutdown_all());
                return client
            }
        };
        let pnl_receiver = match block_on(client.client.connect_and_login(SysInfraType::PnlPlant)) {
            Ok(r) => r,
            Err(e) => {
                let _ = block_on(client.client.shutdown_all());
                return client
            }
        };
        client
    }

    pub async fn shutdown(&self) {
        match self.client.shutdown_all().await {
            Ok(_) => {}
            Err(e) => eprintln!("Rithmic Client shutdown error: {}", e)
        }
    }

    // Send a request and wait for a response
    pub async fn send_synchronous_request<T: ProstMessage + Default>(&self, stream_name: String, plant: &SysInfraType, request: T, id: u64 ) -> Result<DataServerResponse, FundForgeError> {
       todo!()
    }
}

#[async_trait]
impl BrokerApiResponse for RithmicClient {
    async fn symbols_response(&self, stream_name: String, market_type: MarketType, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn account_info_response(&self, stream_name: String, account_id: AccountId, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn symbol_info_response(&self, stream_name: String, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn margin_required_historical_response(&self, stream_name: String, symbol_name: SymbolName, quantity: Volume, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn margin_required_live_response(&self, stream_name: String, symbol_name: SymbolName, quantity: Volume, callback_id: u64) -> DataServerResponse {
        todo!()
    }
}

#[async_trait]
impl VendorApiResponse for RithmicClient {
    async fn symbols_response(&self, stream_name: String, market_type: MarketType, callback_id: u64) -> DataServerResponse{
        //1. create a oneshot
        //2. create the rithmic message
        //3. send to rithmic
        //4. await on oneshot
        //5. process rithmic message here, don't parse it until here, so that each response type can be used for diff functions
        todo!()
    }

    async fn resolutions_response(&self, stream_name: String, market_type: MarketType, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn markets_response(&self, stream_name: String, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn decimal_accuracy_response(&self, stream_name: String, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn tick_size_response(&self, stream_name: String, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn data_feed_subscribe(&self,stream_name: String, subscription: DataSubscription, sender: Sender<DataServerResponse>) -> DataServerResponse {
        todo!()
    }

    async fn data_feed_unsubscribe(&self, stream_name: String, subscription: DataSubscription) -> DataServerResponse {
        todo!()
    }
}
