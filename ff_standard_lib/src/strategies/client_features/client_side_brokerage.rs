use tokio::sync::oneshot;
use crate::messages::data_server_messaging::{DataServerRequest, DataServerResponse, FundForgeError};
use crate::standardized_types::broker_enum::Brokerage;
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::subscriptions::SymbolName;
use crate::standardized_types::symbol_info::{CommissionInfo, SymbolInfo};
use crate::strategies::client_features::connection_types::ConnectionType;
use crate::strategies::client_features::server_connections::{send_request, StrategyRequest};
use crate::strategies::ledgers::AccountId;

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

    pub async fn symbol_names(&self, callback_id: u64) -> Result<Vec<SymbolName>, FundForgeError> {
        let request = DataServerRequest::SymbolNames {
            callback_id,
            brokerage: self.clone()
        };
        let (sender, receiver) = oneshot::channel();
        let msg = StrategyRequest::CallBack(ConnectionType::Broker(self.clone()), request, sender);
        send_request(msg).await;
        match receiver.await {
            Ok(response) => {
                match response {
                    DataServerResponse::SymbolNames { symbol_names, .. } => Ok(symbol_names),
                    DataServerResponse::Error { error, .. } => Err(error),
                    _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                }
            },
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
        }
    }

    pub async fn commission_info(&self, callback_id: u64, symbol_name: SymbolName) -> Result<CommissionInfo, FundForgeError> {
        let request = DataServerRequest::CommissionInfo {
            callback_id,
            brokerage: self.clone(),
            symbol_name,
        };
        let (sender, receiver) = oneshot::channel();
        let msg = StrategyRequest::CallBack(ConnectionType::Broker(self.clone()), request, sender);
        send_request(msg).await;
        match receiver.await {
            Ok(response) => {
                match response {
                    DataServerResponse::CommissionInfo { commission_info, .. } => Ok(commission_info),
                    DataServerResponse::Error { error, .. } => Err(error),
                    _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                }
            },
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
        }
    }
}