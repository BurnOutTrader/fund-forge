use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::product_maps::oanda::maps::{calculate_oanda_margin, OANDA_SYMBOL_INFO};
use crate::server_features::server_side_brokerage::BrokerApiResponse;
use ff_standard_lib::standardized_types::accounts::{Account, AccountId};
use ff_standard_lib::standardized_types::enums::StrategyMode;
use ff_standard_lib::standardized_types::new_types::Volume;
use ff_standard_lib::standardized_types::orders::{Order, OrderId, OrderUpdateEvent, OrderUpdateType};
use ff_standard_lib::standardized_types::subscriptions::{SymbolName};
use ff_standard_lib::StreamName;
use crate::oanda_api::api_client::OandaClient;

#[async_trait]
impl BrokerApiResponse for OandaClient {
    #[allow(unused)]
    async fn symbol_names_response(&self, mode: StrategyMode, time: Option<DateTime<Utc>>, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        let mut symbol_names: Vec<SymbolName> = Vec::new();
        for symbol in self.instruments_map.iter() {
            symbol_names.push(symbol.key().clone());
        }
        DataServerResponse::SymbolNames {
            callback_id,
            symbol_names,
        }
    }

    #[allow(unused)]
    async fn account_info_response(&self, mode: StrategyMode, stream_name: StreamName, account_id: AccountId, callback_id: u64) -> DataServerResponse {
        match self.account_info.get(&account_id) {
            None => {
                DataServerResponse::Error {callback_id, error: FundForgeError::ClientSideErrorDebug(
                    format!("No account found for id: {}", account_id)
                )}
            }
            Some(account_info) => {
                DataServerResponse::AccountInfo {callback_id, account_info: account_info.clone()}
            }
        }
    }

    #[allow(unused)]
    async fn symbol_info_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        if let Some(info) = OANDA_SYMBOL_INFO.get(&symbol_name) { //todo this will all be replaced, for live only, the backtesting info will come from the coded maps
            return DataServerResponse::SymbolInfo {
                callback_id,
                symbol_info: info.clone(),
            }
        }
        DataServerResponse::Error {
            callback_id,
            error: FundForgeError::ClientSideErrorDebug(format!("Symbol not found: {}", symbol_name)),
        }
    }

    #[allow(unused)]
    async fn intraday_margin_required_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, quantity: Volume, callback_id: u64) -> DataServerResponse {
        // Calculate the margin required based on symbol and position size
        if let Some(margin_used) = calculate_oanda_margin(&symbol_name, quantity, quantity) { //todo this will all be replaced, for live only, the backtesting info will come from the coded maps
            return DataServerResponse::IntradayMarginRequired {
                callback_id,
                symbol_name,
                price: Some(margin_used),
            }
        }

        DataServerResponse::Error {
            callback_id,
            error: FundForgeError::ClientSideErrorDebug(format!("Symbol not found in margin map: {}", symbol_name)),
        }
    }

    #[allow(unused)]
    async fn overnight_margin_required_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, quantity: Volume, callback_id: u64) -> DataServerResponse {
        self.intraday_margin_required_response(mode, stream_name, symbol_name, quantity, callback_id).await
    }

    #[allow(unused)]
    async fn accounts_response(&self, mode: StrategyMode, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        let accounts: Vec<AccountId> = self.accounts.iter().map(|a| a.account_id.clone()).collect();
        DataServerResponse::Accounts {
            callback_id,
            accounts,
        }
    }

    #[allow(unused)]
    async fn logout_command(&self, stream_name: StreamName) {
        todo!()
    }

    #[allow(unused)]
    async fn commission_info_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    #[allow(unused)]
    async fn live_market_order(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        todo!()
    }

    #[allow(unused)]
    async fn live_enter_long(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        todo!()
    }

    #[allow(unused)]
    async fn live_enter_short(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        todo!()
    }

    #[allow(unused)]
    async fn live_exit_short(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        todo!()
    }

    #[allow(unused)]
    async fn live_exit_long(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        todo!()
    }

    #[allow(unused)]
    async fn other_orders(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        todo!()
    }

    #[allow(unused)]
    async fn cancel_orders_on_account(&self, account: Account) {
        todo!()
    }

    #[allow(unused)]
    async fn cancel_order(&self, account: Account, order_id: OrderId) {
        todo!()
    }

    #[allow(unused)]
    async fn flatten_all_for(&self, account: Account) {
        todo!()
    }

    #[allow(unused)]
    async fn update_order(&self, account: Account, order_id: OrderId, update: OrderUpdateType) -> Result<(), OrderUpdateEvent> {
        todo!()
    }
}
