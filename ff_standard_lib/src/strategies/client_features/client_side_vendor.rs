use chrono::{DateTime, Utc};
use tokio::sync::oneshot;
use tokio::time::timeout;
use crate::messages::data_server_messaging::{DataServerRequest, DataServerResponse, FundForgeError};
use crate::product_maps::oanda::maps::OANDA_SYMBOL_INFO;
use crate::product_maps::rithmic::maps::get_futures_symbol_info;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::datavendor_enum::DataVendor;
use crate::standardized_types::enums::{MarketType, PrimarySubscription};
use crate::standardized_types::new_types::Price;
use crate::standardized_types::subscriptions::{Symbol, SymbolName};
use crate::strategies::client_features::client_side_brokerage::TIME_OUT;
use crate::strategies::client_features::connection_types::ConnectionType;
use crate::strategies::client_features::request_handler::{send_request, StrategyRequest};
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
        match timeout(TIME_OUT, receiver).await {
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
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Operation timed out after {} seconds", e)))
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
        match timeout(TIME_OUT, receiver).await {
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
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Operation timed out after {} seconds", e)))
        }
    }

    pub async fn resolutions(&self, market_type: MarketType) -> Result<Vec<PrimarySubscription>, FundForgeError> {
        let request = DataServerRequest::Resolutions {
            callback_id: 0,
            data_vendor: self.clone(),
            market_type,
        };
        let (sender, receiver) = oneshot::channel();
        let msg = StrategyRequest::CallBack(ConnectionType::Vendor(self.clone()), request,sender);
        send_request(msg).await;
        match timeout(TIME_OUT, receiver).await {
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
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Operation timed out after {} seconds", e)))
        }
    }

    pub async fn warm_up_resolutions(&self, market_type: MarketType) -> Result<Vec<PrimarySubscription>, FundForgeError> {
        let request = DataServerRequest::WarmUpResolutions {
            callback_id: 0,
            data_vendor: self.clone(),
            market_type,
        };
        let (sender, receiver) = oneshot::channel();
        let msg = StrategyRequest::CallBack(ConnectionType::Vendor(self.clone()), request,sender);
        send_request(msg).await;
        match timeout(TIME_OUT, receiver).await {
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
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Operation timed out after {} seconds", e)))
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
        match timeout(TIME_OUT, receiver).await {
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
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Operation timed out after {} seconds", e)))
        }
    }

    pub async fn decimal_accuracy(&self, symbol_name: SymbolName) -> Result<u32, FundForgeError> {
        match self {
            DataVendor::Rithmic => {
                return match get_futures_symbol_info(&symbol_name) {
                    Ok(info) => Ok(info.decimal_accuracy),
                    Err(e) => return Err(FundForgeError::ClientSideErrorDebug(format!("Error getting decimal accuracy Symbol not found in rithmic_symbol_info: {}", e)))
                };
            }
            DataVendor::Oanda => {
                return match OANDA_SYMBOL_INFO.get(&symbol_name) {
                    Some(info) => Ok(info.decimal_accuracy),
                    None => Err(FundForgeError::ClientSideErrorDebug("Error getting decimal accuracy Symbol not found in OANDA_SYMBOL_INFO".to_string()))
                };
            }
            _ => {}
        }

        let request = DataServerRequest::DecimalAccuracy {
            callback_id: 0,
            data_vendor: self.clone(),
            symbol_name,
        };
        let (sender, receiver) = oneshot::channel();
        let msg = StrategyRequest::CallBack(ConnectionType::Vendor(self.clone()), request,sender);
        send_request(msg).await;
        match timeout(TIME_OUT, receiver).await {
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
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Operation timed out after {} seconds", e)))
        }
    }

    pub async fn tick_size(&self, symbol_name: SymbolName) -> Result<Price, FundForgeError> {
        match self {
            DataVendor::DataBento => {}
            DataVendor::Rithmic => {
                return match get_futures_symbol_info(&symbol_name) {
                    Ok(info) => Ok(info.tick_size),
                    Err(e) => return Err(FundForgeError::ClientSideErrorDebug(format!("Error getting tick size: {}", e)))
                };
            }
            DataVendor::Bitget => {}
            DataVendor::Oanda => {
                return match OANDA_SYMBOL_INFO.get(&symbol_name) {
                    Some(info) => Ok(info.tick_size),
                    None => Err(FundForgeError::ClientSideErrorDebug("Symbol not found in OANDA_SYMBOL_INFO".to_string()))
                };
            }
        }

        //if we don't have local map check with server
        let request = DataServerRequest::TickSize {
            callback_id: 0,
            data_vendor: self.clone(),
            symbol_name,
        };
        let (sender, receiver) = oneshot::channel();
        let msg = StrategyRequest::CallBack(ConnectionType::Vendor(self.clone()), request,sender);
        send_request(msg).await;
        match timeout(TIME_OUT, receiver).await {
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
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Operation timed out after {} seconds", e)))
        }
    }
}