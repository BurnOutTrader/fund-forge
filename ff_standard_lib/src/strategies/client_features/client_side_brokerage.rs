use std::time::Duration;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tokio::sync::oneshot;
use tokio::time::timeout;
use crate::messages::data_server_messaging::{DataServerRequest, DataServerResponse, FundForgeError};
use crate::standardized_types::broker_enum::Brokerage;
use crate::standardized_types::enums::StrategyMode;
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::subscriptions::SymbolName;
use crate::standardized_types::symbol_info::{CommissionInfo, SymbolInfo};
use crate::strategies::client_features::connection_types::ConnectionType;
use crate::strategies::client_features::server_connections::{send_request, StrategyRequest};
use crate::strategies::ledgers::{AccountId, AccountInfo, Currency, Ledger};

impl Brokerage {
    pub async fn paper_account_init(&self, mode: StrategyMode, starting_balance: Decimal, currency: Currency, account_id: AccountId) -> Result<Ledger, FundForgeError> {
        let request = DataServerRequest::PaperAccountInit {
            account_id,
            callback_id: 0,
            brokerage: self.clone(),
        };
        let (sender, receiver) = oneshot::channel();
        let msg = StrategyRequest::CallBack(ConnectionType::Broker(self.clone()), request, sender);
        send_request(msg).await;
        //todo, we need reconnect and resend callback logic for time outs and disconnects
        match timeout(Duration::from_secs(10), receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(response) => match response {
                    DataServerResponse::PaperAccountInit { account_info, .. } => {
                        Ok(Ledger {
                            account_id: account_info.account_id,
                            brokerage: account_info.brokerage,
                            cash_value: starting_balance,
                            cash_available: starting_balance,
                            currency,
                            cash_used: dec!(0.0),
                            positions: DashMap::new(),
                            margin_used: DashMap::new(),
                            positions_closed: DashMap::new(),
                            positions_counter: DashMap::new(),
                            symbol_info: DashMap::new(),
                            open_pnl: DashMap::new(),
                            booked_pnl: dec!(0.0),
                            mode,
                            leverage: account_info.leverage
                        })
                    },
                    DataServerResponse::Error { error, .. } => Err(error),
                    _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            },
            Err(_) => Err(FundForgeError::ClientSideErrorDebug("Operation timed out after 10 seconds".to_string()))
        }
    }

    pub async fn intraday_margin_required(&self, symbol_name: SymbolName, quantity: Volume) -> Result<Option<Price>, FundForgeError> {
        let request = DataServerRequest::IntradayMarginRequired {
            callback_id: 0,
            brokerage: self.clone(),
            symbol_name,
            quantity
        };
        let (sender, receiver) = oneshot::channel();
        let msg = StrategyRequest::CallBack(ConnectionType::Broker(self.clone()), request, sender);
        send_request(msg).await;
        match timeout(Duration::from_secs(10), receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(response) => match response {
                    DataServerResponse::IntradayMarginRequired { price, .. } => Ok(price),
                    DataServerResponse::Error { error, .. } => Err(error),
                    _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            },
            Err(_) => Err(FundForgeError::ClientSideErrorDebug("Operation timed out after 10 seconds".to_string()))
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
        match timeout(Duration::from_secs(10), receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(response) => match response {
                    DataServerResponse::SymbolInfo { symbol_info, .. } => Ok(symbol_info),
                    DataServerResponse::Error { error, .. } => Err(error),
                    _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            },
            Err(_) => Err(FundForgeError::ClientSideErrorDebug("Operation timed out after 10 seconds".to_string()))
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
        match timeout(Duration::from_secs(10), receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(response) => match response {
                    DataServerResponse::Accounts { accounts, .. } => Ok(accounts),
                    DataServerResponse::Error { error, .. } => Err(error),
                    _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            },
            Err(_) => Err(FundForgeError::ClientSideErrorDebug("Operation timed out after 10 seconds".to_string()))
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
        match timeout(Duration::from_secs(10), receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(response) => match response {
                    DataServerResponse::SymbolNames { symbol_names, .. } => Ok(symbol_names),
                    DataServerResponse::Error { error, .. } => Err(error),
                    _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            },
            Err(_) => Err(FundForgeError::ClientSideErrorDebug("Operation timed out after 10 seconds".to_string()))
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
        match timeout(Duration::from_secs(10), receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(response) => match response {
                    DataServerResponse::CommissionInfo { commission_info, .. } => Ok(commission_info),
                    DataServerResponse::Error { error, .. } => Err(error),
                    _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            },
            Err(_) => Err(FundForgeError::ClientSideErrorDebug("Operation timed out after 10 seconds".to_string()))
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
        match timeout(Duration::from_secs(10), receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(response) => match response {
                    DataServerResponse::AccountInfo { account_info, .. } => Ok(account_info),
                    DataServerResponse::Error { error, .. } => Err(error),
                    _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect response received at callback".to_string()))
                },
                Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Receiver error at callback recv: {}", e)))
            },
            Err(_) => Err(FundForgeError::ClientSideErrorDebug("Operation timed out after 10 seconds".to_string()))
        }
    }
}