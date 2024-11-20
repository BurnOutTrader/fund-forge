use std::sync::Arc;
use structopt::lazy_static::lazy_static;
use ff_standard_lib::messages::data_server_messaging::FundForgeError;
use crate::ServerLaunchOptions;
use std::fs;
use dashmap::DashMap;
use databento::{HistoricalClient, LiveClient};
use databento::dbn::{PitSymbolMap, SType, Schema};
use tokio::sync::OnceCell;
use toml::Value;
use ff_standard_lib::standardized_types::subscriptions::SymbolName;
use crate::oanda_api::api_client::OandaClient;


static DATA_BENTO_CLIENT: OnceCell<Arc<DataBentoClient>> = OnceCell::const_new();

pub async fn data_bento_init(options: ServerLaunchOptions) -> Result<(), FundForgeError> {
    let client = DataBentoClient::new(options)?;
    DATA_BENTO_CLIENT.set(Arc::new(client)).map_err(|_| {
        FundForgeError::ServerErrorDebug("Failed to set Data Bento client".to_string())
    })?;
    println!("Data Bento client initialized");
    Ok(())
}

pub fn get_data_bento_client() -> Result<Arc<DataBentoClient>, FundForgeError> {
    DATA_BENTO_CLIENT.get().cloned().ok_or_else(|| {
        FundForgeError::ServerErrorDebug("Data Bento client not initialized".to_string())
    })
}

pub struct DataBentoClient {
    historical_client: HistoricalClient,
    live_clients: DashMap<(String, Schema, SType), LiveClient>,
}

impl DataBentoClient {
    fn new(options: ServerLaunchOptions) -> Result<Self, FundForgeError> {
        let key = match DataBentoClient::get_api_key(&options) {
            Ok(key) => key,
            Err(e) => return Err(e)
        };
        let historical_client = match HistoricalClient::builder().key(key.clone()) {
            Ok(client) => client,
            Err(e) => return Err(FundForgeError::ServerErrorDebug(format!("Failed to create Data Bento client: {}", e)))
        };

        let historical_client = match historical_client.build() {
            Ok(client) => client,
            Err(e) => return Err(FundForgeError::ServerErrorDebug(format!("Failed to build Data Bento client: {}", e)))
        };

        Ok(Self {
            historical_client,
            live_clients: DashMap::new()
        })
    }

    pub fn shutdown(&self) {

    }

    pub fn get_api_key(options: &ServerLaunchOptions) -> Result<String, FundForgeError> {
        let path = options.data_folder.clone()
            .join("credentials")
            .join("databento_credentials")
            .join("active")
            .join("databento_credentials.toml");

        if !path.exists() {
            return Err(FundForgeError::ServerErrorDebug(
                "No Data Bento credentials toml".to_string(),
            ));
        }

        // Read the toml file
        let content = fs::read_to_string(&path).map_err(|e| {
            FundForgeError::ServerErrorDebug(format!("Failed to read file: {}", e))
        })?;

        // Parse the toml content
        let toml_value: Value = content.parse::<Value>().map_err(|e| {
            FundForgeError::ServerErrorDebug(format!("Failed to parse TOML: {}", e))
        })?;

        // Extract the `api_key`
        let api_key = toml_value
            .get("api_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                FundForgeError::ServerErrorDebug("Missing or invalid `api_key` in TOML".to_string())
            })?;

        Ok(api_key.to_string())
    }
}

