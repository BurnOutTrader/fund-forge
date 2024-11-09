use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ff_standard_lib::messages::data_server_messaging::DataServerResponse;
use crate::server_features::server_side_brokerage::BrokerApiResponse;
use ff_standard_lib::standardized_types::accounts::{Account, AccountId};
use ff_standard_lib::standardized_types::enums::StrategyMode;
use ff_standard_lib::standardized_types::orders::{Order, OrderId, OrderUpdateEvent, OrderUpdateType};
use ff_standard_lib::standardized_types::subscriptions::SymbolName;
use ff_standard_lib::StreamName;
use crate::bitget_api::api_client::BitgetClient;

#[async_trait]
impl BrokerApiResponse for BitgetClient {
    #[allow(unused)]
    async fn symbol_names_response(&self, mode: StrategyMode, time: Option<DateTime<Utc>>, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    #[allow(unused)]
    async fn account_info_response(&self, mode: StrategyMode, stream_name: StreamName, account_id: AccountId, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    #[allow(unused)]
    async fn symbol_info_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    #[allow(unused)]
    async fn accounts_response(&self, mode: StrategyMode, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        todo!()
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