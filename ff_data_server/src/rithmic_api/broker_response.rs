use async_trait::async_trait;
use rust_decimal_macros::dec;
use ff_standard_lib::helpers::converters::fund_forge_formatted_symbol_name;
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::server_features::server_side_brokerage::BrokerApiResponse;
use ff_standard_lib::standardized_types::enums::StrategyMode;
use ff_standard_lib::standardized_types::new_types::Volume;
use ff_standard_lib::standardized_types::subscriptions::SymbolName;
use ff_standard_lib::standardized_types::symbol_info::SymbolInfo;
use ff_standard_lib::strategies::ledgers::{AccountId, Currency};
use ff_standard_lib::StreamName;
use crate::rithmic_api::api_client::RithmicClient;

#[async_trait]
impl BrokerApiResponse for RithmicClient {
    async fn symbol_names_response(&self, _mode: StrategyMode, _stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        //todo get dynamically from server using stream name to fwd callback
        DataServerResponse::SymbolNames {
            callback_id,
            symbol_names: vec!["MNQ".to_string()],
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
        //todo get dynamically from server using stream name to fwd callback
        let symbol_name = fund_forge_formatted_symbol_name(&symbol_name);
        let (pnl_currency, value_per_tick, tick_size, decimal_accuracy) = match symbol_name.as_str() {
            "MNQ" => (Currency::USD, dec!(2.0), dec!(0.25), 2), // MNQ in USD with $2 per tick at 0.25 tick size and decimal accuracy of 2
            _ => todo!()       // Default values
        };

        let symbol_info = SymbolInfo {
            symbol_name,
            pnl_currency,
            value_per_tick,
            tick_size,
            decimal_accuracy,
        };

        DataServerResponse::SymbolInfo {
            callback_id,
            symbol_info,
        }
    }

    async fn margin_required_response(&self,  _mode: StrategyMode, _stream_name: StreamName, symbol_name: SymbolName, quantity: Volume, callback_id: u64) -> DataServerResponse {
        //todo get dynamically from server using stream name to fwd callback
        DataServerResponse::MarginRequired {
            callback_id,
            symbol_name,
            price: quantity * dec!(150.0),
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
}