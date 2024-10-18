use std::time::Duration;
use chrono::{DateTime, Utc};
use tokio::sync::oneshot;
use tokio::time::timeout;
use crate::messages::data_server_messaging::{DataServerRequest, DataServerResponse, FundForgeError};
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::datavendor_enum::DataVendor;
use crate::standardized_types::enums::{MarketType, SubscriptionResolutionType};
use crate::standardized_types::new_types::Price;
use crate::standardized_types::subscriptions::{Symbol, SymbolName};
use crate::standardized_types::symbol_info::SessionMarketHours;
use crate::strategies::client_features::connection_types::ConnectionType;
use crate::strategies::client_features::server_connections::{send_request, StrategyRequest};
const TIMEOUT_DURATION: Duration = Duration::from_secs(30);
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
        match timeout(TIMEOUT_DURATION, receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(response) => {
                    match response {
                        DataServerResponse::TickSize { tick_size, .. } => Ok(tick_size),
                        DataServerResponse::Error { error, .. } => Err(error),
                        _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                    }
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            },
            Err(_) => Err(FundForgeError::ClientSideErrorDebug("Operation timed out".to_string()))
        }
    }
}

impl DataVendor {
    pub async fn symbols(&self, market_type: MarketType, time: Option<DateTime<Utc>>) -> Result<Vec<Symbol>, FundForgeError> {
        let time = match time {
            None => None,
            Some(t) => Some(t.to_string())
        };
        let request = DataServerRequest::SymbolsVendor {
            callback_id: 0,
            time,
            data_vendor: self.clone(),
            market_type,
        };
        let (sender, receiver) = oneshot::channel();
        let msg = StrategyRequest::CallBack(ConnectionType::Vendor(self.clone()), request,sender);
        send_request(msg).await;
        match timeout(TIMEOUT_DURATION, receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(response) => {
                    match response {
                        DataServerResponse::Symbols { symbols, .. } => Ok(symbols),
                        DataServerResponse::Error {error,..} => Err(error),
                        _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                    }
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            },
            Err(_) => Err(FundForgeError::ClientSideErrorDebug("Operation timed out".to_string()))
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
        match timeout(TIMEOUT_DURATION, receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(response) => {
                    match response {
                        DataServerResponse::BaseDataTypes { base_data_types, .. } => Ok(base_data_types),
                        DataServerResponse::Error {error,..} => Err(error),
                        _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                    }
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            },
            Err(_) => Err(FundForgeError::ClientSideErrorDebug("Operation timed out".to_string()))
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
        match timeout(TIMEOUT_DURATION, receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(response) => {
                    match response {
                        DataServerResponse::Resolutions { subscription_resolutions_types, .. } => Ok(subscription_resolutions_types),
                        DataServerResponse::Error {error,..} => Err(error),
                        _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                    }
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            },
            Err(_) => Err(FundForgeError::ClientSideErrorDebug("Operation timed out".to_string()))
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
        match timeout(TIMEOUT_DURATION, receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(response) => {
                    match response {
                        DataServerResponse::Markets { markets, .. } => Ok(markets),
                        DataServerResponse::Error {error,..} => Err(error),
                        _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                    }
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            },
            Err(_) => Err(FundForgeError::ClientSideErrorDebug("Operation timed out".to_string()))
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
        match timeout(TIMEOUT_DURATION, receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(response) => {
                    match response {
                        DataServerResponse::DecimalAccuracy { accuracy, .. } => Ok(accuracy),
                        DataServerResponse::Error {error,..} => Err(error),
                        _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                    }
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            },
            Err(_) => Err(FundForgeError::ClientSideErrorDebug("Operation timed out".to_string()))
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
        match timeout(TIMEOUT_DURATION, receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(response) => {
                    match response {
                        DataServerResponse::TickSize { tick_size, .. } => Ok(tick_size),
                        DataServerResponse::Error {error,..} => Err(error),
                        _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                    }
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            },
            Err(_) => Err(FundForgeError::ClientSideErrorDebug("Operation timed out".to_string()))
        }
    }

    pub async fn session_market_hours(&self, data_vendor: DataVendor, symbol_name: SymbolName, time: DateTime<Utc>) -> Result<SessionMarketHours, FundForgeError> {
        let request = DataServerRequest::SessionMarketHours {
            callback_id: 0,
            data_vendor,
            symbol_name,
            date: time.to_string(),
        };
        let (sender, receiver) = oneshot::channel();
        let msg = StrategyRequest::CallBack(ConnectionType::Vendor(self.clone()), request,sender);
        send_request(msg).await;
        match timeout(TIMEOUT_DURATION, receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(response) => {
                    match response {
                        DataServerResponse::SessionMarketHours { session_market_hours, .. } => Ok(session_market_hours),
                        DataServerResponse::Error {error,..} => Err(error),
                        _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                    }
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            },
            Err(_) => Err(FundForgeError::ClientSideErrorDebug("Operation timed out".to_string()))
        }
    }
}