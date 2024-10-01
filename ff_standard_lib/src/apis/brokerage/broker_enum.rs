use async_trait::async_trait;
use serde_derive::{Deserialize, Serialize};
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use strum_macros::Display;
use std::str::FromStr;
use ff_rithmic_api::systems::RithmicSystem;
use crate::apis::brokerage::server_side_brokerage::BrokerApiResponse;
use crate::apis::rithmic_api::api_client::RITHMIC_CLIENTS;
use crate::apis::StreamName;
use crate::apis::test_api::api_client::TEST_CLIENT;
use crate::standardized_types::accounts::ledgers::AccountId;
use crate::standardized_types::data_server_messaging::{DataServerResponse, FundForgeError};
use crate::standardized_types::enums::{MarketType, StrategyMode};
use crate::standardized_types::subscriptions::SymbolName;
use crate::standardized_types::new_types::Volume;

#[derive(Serialize, Deserialize, Clone, Eq, Serialize_rkyv, Deserialize_rkyv,
    Archive, PartialEq, Debug, Hash, PartialOrd, Ord, Display, Copy)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum Brokerage {
    Test, //DO NOT CHANGE ORDER
    Rithmic(RithmicSystem),
}
impl FromStr for Brokerage {
    type Err = FundForgeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "Test" {
            Ok(Brokerage::Test)
        } else if s.starts_with("Rithmic") {
            let system_name = s.trim_start_matches("Rithmic ");
            if let Some(system) = RithmicSystem::from_string(system_name) {
                Ok(Brokerage::Rithmic(system))
            } else {
                Err(FundForgeError::ClientSideErrorDebug(format!(
                    "Unknown RithmicSystem string: {}",
                    system_name
                )))
            }
        } else {
            Err(FundForgeError::ClientSideErrorDebug(format!(
                "Invalid brokerage string: {}",
                s
            )))
        }
    }
}

pub mod client_side_brokerage {
    use crate::apis::brokerage::broker_enum::Brokerage;
    use tokio::sync::oneshot;
    use crate::server_connections::{send_request, ConnectionType, StrategyRequest};
    use crate::standardized_types::data_server_messaging::{DataServerRequest, DataServerResponse, FundForgeError};
    use crate::standardized_types::subscriptions::SymbolName;
    use crate::standardized_types::accounts::ledgers::AccountId;
    use crate::standardized_types::new_types::{Price, Volume};
    use crate::standardized_types::symbol_info::SymbolInfo;
    impl Brokerage {
        pub async fn margin_required(&self, symbol_name: SymbolName, quantity: Volume) -> Result<Price, FundForgeError> {
            let request = DataServerRequest::MarginRequired {
                callback_id: 0,
                brokerage: self.clone(),
                symbol_name,
                quantity
            };
            let (sender, receiver) = oneshot::channel();
            let msg = StrategyRequest::CallBack(ConnectionType::Broker(self.clone()), request, sender);
            send_request(msg).await;
            match receiver.await {
                Ok(response) => {
                    match response {
                        DataServerResponse::MarginRequired { price, .. } => Ok(price),
                        DataServerResponse::Error { error, .. } => Err(error),
                        _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                    }
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            }
        }

        pub async fn symbol_info(&self, symbol_name: SymbolName) -> Result<SymbolInfo, FundForgeError> {
            let request = DataServerRequest::SymbolInfo {
                callback_id: 0,
                brokerage: self.clone(),
                symbol_name,
            };
            let (sender, receiver) = oneshot::channel();
            let msg = StrategyRequest::CallBack(ConnectionType::Broker(self.clone()), request, sender);
            send_request(msg).await;
            match receiver.await {
                Ok(response) => {
                    match response {
                        DataServerResponse::SymbolInfo { symbol_info, .. } => Ok(symbol_info),
                        DataServerResponse::Error { error, .. } => Err(error),
                        _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                    }
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            }
        }

        pub async fn accounts(&self) -> Result<Vec<AccountId>, FundForgeError> {
            let request = DataServerRequest::Accounts {
                callback_id: 0,
                brokerage: self.clone(),
            };
            let (sender, receiver) = oneshot::channel();
            let msg = StrategyRequest::CallBack(ConnectionType::Broker(self.clone()), request, sender);
            send_request(msg).await;
            match receiver.await {
                Ok(response) => {
                    match response {
                        DataServerResponse::Accounts { accounts, .. } => Ok(accounts),
                        DataServerResponse::Error { error, .. } => Err(error),
                        _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                    }
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            }
        }
    }
}

/// Responses
#[async_trait]
impl BrokerApiResponse for Brokerage {
    async fn symbols_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        market_type: MarketType,
        callback_id: u64
    ) -> DataServerResponse {
        match self {
            Brokerage::Rithmic(system) => {
                if let Some(client) = RITHMIC_CLIENTS.get(system) {
                    return client.symbols_response(mode, stream_name, market_type, callback_id).await
                }
            },
            Brokerage::Test => return TEST_CLIENT.symbols_response(mode, stream_name, market_type, callback_id).await
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    async fn account_info_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        account_id: AccountId,
        callback_id: u64
    ) -> DataServerResponse {
        match self {
            Brokerage::Rithmic(system) => {
                if let Some(client) = RITHMIC_CLIENTS.get(system) {
                    return client.account_info_response(mode, stream_name, account_id, callback_id).await
                }
            },
            Brokerage::Test => return TEST_CLIENT.account_info_response(mode, stream_name, account_id, callback_id).await
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    async fn symbol_info_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse {
        match self {
            Brokerage::Rithmic(system) => {
                if let Some(client) = RITHMIC_CLIENTS.get(system) {
                    return client.symbol_info_response(mode, stream_name, symbol_name, callback_id).await
                }
            },
            Brokerage::Test => return TEST_CLIENT.symbol_info_response(mode, stream_name, symbol_name, callback_id).await
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    async fn margin_required_response(&self,  mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, quantity: Volume, callback_id: u64) -> DataServerResponse {
        match self {
            Brokerage::Rithmic(system) => {
                if let Some(client) = RITHMIC_CLIENTS.get(system) {
                    return client.margin_required_response(mode, stream_name, symbol_name, quantity, callback_id).await
                }
            },
            Brokerage::Test => return TEST_CLIENT.margin_required_response(mode, stream_name, symbol_name, quantity, callback_id).await
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    async fn accounts_response(&self, mode: StrategyMode, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        match self {
            Brokerage::Rithmic(system) => {
                if let Some(client) = RITHMIC_CLIENTS.get(system) {
                    return client.accounts_response(mode, stream_name, callback_id).await
                }
            },
            Brokerage::Test => return TEST_CLIENT.accounts_response(mode, stream_name, callback_id).await
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }
}