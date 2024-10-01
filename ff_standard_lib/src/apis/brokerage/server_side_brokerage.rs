use crate::standardized_types::accounts::ledgers::AccountId;
use crate::standardized_types::data_server_messaging::{DataServerResponse};
use crate::standardized_types::subscriptions::SymbolName;
use crate::standardized_types::enums::{MarketType, StrategyMode};
use async_trait::async_trait;
use crate::apis::StreamName;
use crate::standardized_types::Volume;

/// The trait allows the server to implement the vendor specific methods for the DataVendor enum without the client needing to implement them.
#[async_trait]
pub trait BrokerApiResponse: Sync + Send {
    async fn symbols_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        market_type: MarketType,
        callback_id: u64
    ) -> DataServerResponse;
    async fn account_info_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        account_id: AccountId,
        callback_id: u64
    ) -> DataServerResponse;

    async fn symbol_info_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse;

    async fn margin_required_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        symbol_name: SymbolName,
        quantity: Volume,
        callback_id: u64
    ) -> DataServerResponse;

    async fn accounts_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        callback_id: u64
    ) -> DataServerResponse;
}


