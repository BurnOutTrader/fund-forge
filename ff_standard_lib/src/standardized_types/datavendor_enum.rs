use std::str::FromStr;
use ff_rithmic_api::systems::RithmicSystem;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use serde_derive::{Deserialize, Serialize};
use strum_macros::Display;
use crate::messages::data_server_messaging::FundForgeError;

#[derive(Serialize, Deserialize, Clone, Eq, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Hash, PartialOrd, Ord, Display)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
/// A `DataVendor` enum is a company that provides the data that is used to feed the algorithm.
/// The `DataVendor` is used to specify the data vendor that is being used to feed a `Subscription`.
/// Each `DataVendor` implements its own logic to fetch the data from the source, this logic can be modified in the `ff_data_server` crate.
pub enum DataVendor {
    Test, //DO NOT CHANGE ORDER
    Rithmic(RithmicSystem),
}

impl FromStr for DataVendor {
    type Err = FundForgeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "Test" {
            Ok(DataVendor::Test)
        } else if s.starts_with("Rithmic") {
            let system_name = s.trim_start_matches("Rithmic ");
            if let Some(system) = RithmicSystem::from_string(system_name) {
                Ok(DataVendor::Rithmic(system))
            } else {
                Err(FundForgeError::ClientSideErrorDebug(format!(
                    "Unknown RithmicSystem string: {}",
                    system_name
                )))
            }
        } else {
            Err(FundForgeError::ClientSideErrorDebug(format!(
                "Unknown DataVendor string: {}",
                s
            )))
        }
    }
}

pub mod client_side_data_vendors {
    use tokio::sync::oneshot;
    use crate::client_features::connections::ConnectionType;
    use crate::standardized_types::datavendor_enum::DataVendor;
    use crate::server_connections::{send_request, StrategyRequest};
    use crate::standardized_types::base_data::base_data_type::BaseDataType;
    use crate::messages::data_server_messaging::{DataServerRequest, DataServerResponse, FundForgeError};
    use crate::standardized_types::enums::{MarketType, SubscriptionResolutionType};
    use crate::standardized_types::new_types::Price;
    use crate::standardized_types::subscriptions::{Symbol, SymbolName};

    impl DataVendor {
        pub async fn symbols(&self, market_type: MarketType) -> Result<Vec<Symbol>, FundForgeError> {
            let request = DataServerRequest::SymbolsVendor {
                callback_id: 0,
                data_vendor: self.clone(),
                market_type,
            };
            let (sender, receiver) = oneshot::channel();
            let msg = StrategyRequest::CallBack(ConnectionType::Vendor(self.clone()), request,sender);
            send_request(msg).await;
            match receiver.await {
                Ok(response) => {
                    match response {
                        DataServerResponse::Symbols { symbols, .. } => Ok(symbols),
                        DataServerResponse::Error {error,..} => Err(error),
                        _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                    }
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            }
        }

        pub async fn base_data_types(&self) -> Result<Vec<BaseDataType>, FundForgeError> {
            let request = DataServerRequest::BaseDataTypes {
                callback_id: 0,
                data_vendor: self.clone(),
            };
            let (sender, receiver) = oneshot::channel();
            let msg = StrategyRequest::CallBack(ConnectionType::Vendor(self.clone()), request,sender);
            send_request(msg).await;
            match receiver.await {
                Ok(response) => {
                    match response {
                        DataServerResponse::BaseDataTypes { base_data_types, .. } => Ok(base_data_types),
                        DataServerResponse::Error {error,..} => Err(error),
                        _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                    }
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            }
        }

        pub async fn resolutions(&self, market_type: MarketType) -> Result<Vec<SubscriptionResolutionType>, FundForgeError> {
            let request = DataServerRequest::Resolutions {
                callback_id: 0,
                data_vendor: self.clone(),
                market_type,
            };
            let (sender, receiver) = oneshot::channel();
            let msg = StrategyRequest::CallBack(ConnectionType::Vendor(self.clone()), request,sender);
            send_request(msg).await;
            match receiver.await {
                Ok(response) => {
                    match response {
                        DataServerResponse::Resolutions { subscription_resolutions_types, .. } => Ok(subscription_resolutions_types),
                        DataServerResponse::Error {error,..} => Err(error),
                        _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                    }
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            }
        }

        pub async fn markets(&self) -> Result<Vec<MarketType>, FundForgeError> {
            let request = DataServerRequest::Markets {
                callback_id: 0,
                data_vendor: self.clone(),
            };
            let (sender, receiver) = oneshot::channel();
            let msg = StrategyRequest::CallBack(ConnectionType::Vendor(self.clone()), request,sender);
            send_request(msg).await;
            match receiver.await {
                Ok(response) => {
                    match response {
                        DataServerResponse::Markets { markets, .. } => Ok(markets),
                        DataServerResponse::Error {error,..} => Err(error),
                        _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                    }
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            }
        }

        pub async fn decimal_accuracy(&self, symbol_name: SymbolName) -> Result<u8, FundForgeError> {
            let request = DataServerRequest::DecimalAccuracy {
                callback_id: 0,
                data_vendor: self.clone(),
                symbol_name,
            };
            let (sender, receiver) = oneshot::channel();
            let msg = StrategyRequest::CallBack(ConnectionType::Vendor(self.clone()), request,sender);
            send_request(msg).await;
            match receiver.await {
                Ok(response) => {
                    match response {
                        DataServerResponse::DecimalAccuracy { accuracy, .. } => Ok(accuracy),
                        DataServerResponse::Error {error,..} => Err(error),
                        _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                    }
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            }
        }

        pub async fn tick_size(&self, symbol_name: SymbolName) -> Result<Price, FundForgeError> {
            let request = DataServerRequest::TickSize {
                callback_id: 0,
                data_vendor: self.clone(),
                symbol_name,
            };
            let (sender, receiver) = oneshot::channel();
            let msg = StrategyRequest::CallBack(ConnectionType::Vendor(self.clone()), request,sender);
            send_request(msg).await;
            match receiver.await {
                Ok(response) => {
                    match response {
                        DataServerResponse::TickSize { tick_size, .. } => Ok(tick_size),
                        DataServerResponse::Error {error,..} => Err(error),
                        _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                    }
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            }
        }
    }
}