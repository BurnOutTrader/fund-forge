use serde_derive::{Deserialize, Serialize};
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use strum_macros::Display;
use std::str::FromStr;
use ff_rithmic_api::systems::RithmicSystem;
use crate::messages::data_server_messaging::FundForgeError;
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
    use crate::standardized_types::broker_enum::Brokerage;
    use tokio::sync::oneshot;
    use crate::client_features::connection_types::ConnectionType;
    use crate::client_features::server_connections::{send_request, StrategyRequest};
    use crate::messages::data_server_messaging::{DataServerRequest, DataServerResponse, FundForgeError};
    use crate::standardized_types::subscriptions::SymbolName;
    use crate::strategies::accounts::ledgers::AccountId;
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

