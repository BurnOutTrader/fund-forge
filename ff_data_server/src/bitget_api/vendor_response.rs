use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use crate::server_features::server_side_datavendor::VendorApiResponse;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::enums::{MarketType, StrategyMode};
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol, SymbolName};
use ff_standard_lib::StreamName;
use crate::bitget_api::api_client::BitgetClient;

#[async_trait]
impl VendorApiResponse for BitgetClient {
    #[allow(unused)]
    async fn symbols_response(&self, mode: StrategyMode, stream_name: StreamName, market_type: MarketType, time: Option<DateTime<Utc>>, callback_id: u64) -> DataServerResponse {
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

    #[allow(unused)]
    async fn session_market_hours_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, date_time: DateTime<Utc>, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    #[allow(unused)]
    async fn update_historical_data_for(&self, symbol: Symbol, base_data_type: BaseDataType, resolution: Resolution) -> Result<(), FundForgeError>  {
        todo!()
    }
}