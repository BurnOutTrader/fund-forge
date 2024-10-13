use async_trait::async_trait;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::server_features::server_side_brokerage::BrokerApiResponse;
use ff_standard_lib::standardized_types::enums::StrategyMode;
use ff_standard_lib::standardized_types::new_types::Volume;
use ff_standard_lib::standardized_types::subscriptions::SymbolName;
use ff_standard_lib::strategies::ledgers::AccountId;
use ff_standard_lib::StreamName;
use crate::rithmic_api::api_client::RithmicClient;
use crate::rithmic_api::products::{get_available_symbol_names, get_futures_commissions_info, get_intraday_margin, get_overnight_margin, get_symbol_info};

#[async_trait]
impl BrokerApiResponse for RithmicClient {
    async fn symbol_names_response(&self, _mode: StrategyMode, _stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        let symbol_names = get_available_symbol_names();

        if symbol_names.is_empty() {
            DataServerResponse::Error {
                callback_id,
                error: FundForgeError::ClientSideErrorDebug("No symbols available".to_string()),
            }
        } else {
            DataServerResponse::SymbolNames {
                callback_id,
                symbol_names: symbol_names.clone(),
            }
        }
    }

    async fn account_info_response(&self, mode: StrategyMode, _stream_name: StreamName, account_id: AccountId, callback_id: u64) -> DataServerResponse {
        //todo use match mode to create sim account
        match mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                todo!("Not implemented for backtest")
            }
            StrategyMode::Live => {
                match self.accounts.get(&account_id) {
                    None => DataServerResponse::Error {callback_id, error:FundForgeError::ClientSideErrorDebug(format!("{} Has No Account for {}",self.brokerage, account_id))},
                    Some(account_info) => DataServerResponse::AccountInfo {
                        callback_id,
                        account_info: account_info.value().clone(),
                    }
                }
            }
        }
    }

    async fn symbol_info_response(
        &self,
        _mode: StrategyMode,
        _stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse {
        match get_symbol_info(&symbol_name) {
            Ok(symbol_info) => DataServerResponse::SymbolInfo {callback_id, symbol_info},
            Err(e) => DataServerResponse::Error {callback_id, error: FundForgeError::ClientSideErrorDebug(format!("{}", e))}
        }
    }

    async fn intraday_margin_required_response(
        &self,
        _mode: StrategyMode, //todo we should check with broker when live
        _stream_name: StreamName,
        symbol_name: SymbolName,
        quantity: Volume,
        callback_id: u64
    ) -> DataServerResponse {
        let margin = get_intraday_margin(&symbol_name).unwrap_or_else(|| dec!(0.0));
        let required_margin = margin * Decimal::from(quantity.abs());

        DataServerResponse::MarginRequired {
            callback_id,
            symbol_name,
            price: required_margin,
        }
    }

    async fn overnight_margin_required_response(
        &self,
        _mode: StrategyMode, //todo we should check with broker when live
        _stream_name: StreamName,
        symbol_name: SymbolName,
        quantity: Volume,
        callback_id: u64
    ) -> DataServerResponse {
        let margin = get_overnight_margin(&symbol_name).unwrap_or_else(|| dec!(0.0));
        let required_margin = margin * Decimal::from(quantity.abs());

        DataServerResponse::MarginRequired {
            callback_id,
            symbol_name,
            price: required_margin,
        }
    }

    async fn accounts_response(&self, _mode: StrategyMode, _stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        // The accounts are collected on initializing the client
        let accounts = self.accounts.iter().map(|entry| entry.key().clone()).collect();
        DataServerResponse::Accounts {
            callback_id,
            accounts,
        }
    }

    async fn logout_command(&self, stream_name: StreamName) {
        //todo handle dynamically from server using stream name to remove subscriptions and callbacks
        self.callbacks.remove(&stream_name);
    }

    async fn commission_info_response(&self, _mode: StrategyMode, _stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        //todo add a mode to get live commsions from specific brokerage.
        match get_futures_commissions_info(&symbol_name) {
            Ok(commission_info) => DataServerResponse::CommissionInfo {
                callback_id,
                commission_info,
            },
            Err(e) => DataServerResponse::Error {
                callback_id,
                error: FundForgeError::ClientSideErrorDebug(e)
            }
        }
    }
}