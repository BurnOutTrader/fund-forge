use tokio::sync::oneshot;
use crate::messages::data_server_messaging::{DataServerRequest, DataServerResponse, FundForgeError};
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::datavendor_enum::DataVendor;
use crate::standardized_types::enums::{MarketType, SubscriptionResolutionType};
use crate::standardized_types::new_types::Price;
use crate::standardized_types::subscriptions::{Symbol, SymbolName};
use crate::strategies::client_features::connection_types::ConnectionType;
use crate::strategies::client_features::server_connections::{send_request, StrategyRequest};
impl Symbol {
    pub async fn tick_size(&self) -> Result<Price, FundForgeError> {
        let request = DataServerRequest::TickSize {
            callback_id: 0,
            data_vendor: self.data_vendor.clone(),
            symbol_name: self.name.clone(),
        };
        let (sender, receiver) = oneshot::channel();
        let msg = StrategyRequest::CallBack(ConnectionType::Vendor(self.data_vendor.clone()), request, sender);
        send_request(msg).await;
        match receiver.await {
            Ok(response) => {
                match response {
                    DataServerResponse::TickSize { tick_size, .. } => Ok(tick_size),
                    DataServerResponse::Error { error, .. } => Err(error),
                    _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                }
            },
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
        }
    }
}

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

    pub async fn decimal_accuracy(&self, symbol_name: SymbolName) -> Result<u32, FundForgeError> {
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

