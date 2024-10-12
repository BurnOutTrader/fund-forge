use async_trait::async_trait;
use ff_standard_lib::messages::data_server_messaging::DataServerResponse;
use ff_standard_lib::server_features::server_side_datavendor::VendorApiResponse;
use ff_standard_lib::standardized_types::enums::{MarketType, StrategyMode};
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, SymbolName};
use ff_standard_lib::StreamName;
use crate::bitget_api::api_client::BitgetClient;

#[async_trait]
impl VendorApiResponse for BitgetClient {
    #[allow(unused)]
    async fn symbols_response(&self, mode: StrategyMode, stream_name: StreamName, market_type: MarketType, callback_id: u64) -> DataServerResponse {
        todo!()
    }
    #[allow(unused)]
    async fn resolutions_response(&self, mode: StrategyMode, stream_name: StreamName, market_type: MarketType, callback_id: u64) -> DataServerResponse {
        todo!()
    }
    #[allow(unused)]
    async fn markets_response(&self, mode: StrategyMode, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        todo!()
    }
    #[allow(unused)]
    async fn decimal_accuracy_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        todo!()
    }
    #[allow(unused)]
    async fn tick_size_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        todo!()
    }
    #[allow(unused)]
    async fn data_feed_subscribe(&self, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
        todo!()
    }
    #[allow(unused)]
    async fn data_feed_unsubscribe(&self, mode: StrategyMode, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
        todo!()
    }
    #[allow(unused)]
    async fn base_data_types_response(&self, mode: StrategyMode, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        todo!()
    }
    #[allow(unused)]
    async fn logout_command_vendors(&self, stream_name: StreamName) {
        todo!()
    }
}