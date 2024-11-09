use std::time::Duration;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tokio::sync::oneshot;
use tokio::time::timeout;
use crate::messages::data_server_messaging::{DataServerRequest, DataServerResponse, FundForgeError};
use crate::product_maps::oanda::maps::calculate_oanda_margin;
use crate::product_maps::rithmic::maps::get_rithmic_intraday_margin;
use crate::standardized_types::accounts::{AccountId, AccountInfo};
use crate::standardized_types::broker_enum::Brokerage;
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::subscriptions::SymbolName;
use crate::standardized_types::symbol_info::{CommissionInfo, SymbolInfo};
use crate::strategies::client_features::connection_types::ConnectionType;
use crate::strategies::client_features::request_handler::{send_request, StrategyRequest};
pub(crate) const TIME_OUT: Duration = Duration::from_secs(15);
impl Brokerage {
    pub async fn intraday_margin_required(&self, symbol_name: &SymbolName, quantity: Volume, price: Price) -> Result<Option<Decimal>, FundForgeError> {
        match self {
            Brokerage::Test => Ok(Some((quantity * price) / dec!(30))),
            Brokerage::Rithmic(_) => {
                match get_rithmic_intraday_margin(symbol_name) {
                    Some(margin) => Ok(Some(margin * quantity)),
                    None => Ok(None)
                }
            },
            Brokerage::Bitget => {
                todo!()
            }
            Brokerage::Oanda => {
                match calculate_oanda_margin(symbol_name, quantity, price) {
                    Some(margin) => Ok(Some(margin)),
                    None => Ok(None)
                }
            }
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
        match timeout(TIME_OUT, receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(response) => match response {
                    DataServerResponse::SymbolInfo { symbol_info, .. } => Ok(symbol_info),
                    DataServerResponse::Error { error, .. } => Err(error),
                    _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            },
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Operation timed out after {} seconds", e)))
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
        match timeout(TIME_OUT, receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(response) => match response {
                    DataServerResponse::Accounts { accounts, .. } => Ok(accounts),
                    DataServerResponse::Error { error, .. } => Err(error),
                    _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            },
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Operation timed out after {} seconds", e)))
        }
    }

    pub async fn symbol_names(&self, callback_id: u64, time: Option<DateTime<Utc>>) -> Result<Vec<SymbolName>, FundForgeError> {
        let time = match time {
            None => None,
            Some(t) => Some(t.to_string())
        };
        let request = DataServerRequest::SymbolNames {
            callback_id,
            brokerage: self.clone(),
            time
        };
        let (sender, receiver) = oneshot::channel();
        let msg = StrategyRequest::CallBack(ConnectionType::Broker(self.clone()), request, sender);
        send_request(msg).await;
        match timeout(TIME_OUT, receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(response) => match response {
                    DataServerResponse::SymbolNames { symbol_names, .. } => Ok(symbol_names),
                    DataServerResponse::Error { error, .. } => Err(error),
                    _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            },
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Operation timed out after {} seconds", e)))
        }
    }

    pub async fn commission_info(&self, symbol_name: SymbolName) -> Result<CommissionInfo, FundForgeError> {
        let request = DataServerRequest::CommissionInfo {
            callback_id: 0,
            brokerage: self.clone(),
            symbol_name,
        };
        let (sender, receiver) = oneshot::channel();
        let msg = StrategyRequest::CallBack(ConnectionType::Broker(self.clone()), request, sender);
        send_request(msg).await;
        match timeout(TIME_OUT, receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(response) => match response {
                    DataServerResponse::CommissionInfo { commission_info, .. } => Ok(commission_info),
                    DataServerResponse::Error { error, .. } => Err(error),
                    _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            },
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Operation timed out after {} seconds", e)))
        }
    }

    pub async fn account_info(&self, account_id: AccountId) -> Result<AccountInfo, FundForgeError> {
        let request = DataServerRequest::AccountInfo {
            callback_id: 0,
            brokerage: self.clone(),
            account_id
        };
        let (sender, receiver) = oneshot::channel();
        let msg = StrategyRequest::CallBack(ConnectionType::Broker(self.clone()), request, sender);
        send_request(msg).await;
        match timeout(TIME_OUT, receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(response) => match response {
                    DataServerResponse::AccountInfo { account_info, .. } => Ok(account_info),
                    DataServerResponse::Error { error, .. } => Err(error),
                    _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            },
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Operation timed out after {} seconds", e)))
        }
    }
}