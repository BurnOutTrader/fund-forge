use async_trait::async_trait;
use crate::messages::data_server_messaging::DataServerResponse;
use crate::standardized_types::enums::{MarketType, StrategyMode};
use crate::standardized_types::subscriptions::{DataSubscription, SymbolName};
use crate::StreamName;

/// The trait allows the server to implement the vendor specific methods for the DataVendor enum without the client needing to implement them.
#[async_trait]
pub trait VendorApiResponse: Sync + Send {
    /// return `DataServerResponse::Symbols` or `DataServerResponse::Error(FundForgeError)`
    /// server or client error depending on who caused this problem
    async fn symbols_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName, // The stream name is just the u16 port number the strategy is connecting to
        market_type: MarketType,
        callback_id: u64
    ) -> DataServerResponse;

    /// return `DataServerResponse::Resolutions` or `DataServerResponse::Error(FundForgeError)`
    /// server or client error depending on who caused this problem
    /// Note that we are not just returning resolutions here,
    /// we return `Vec<SubscriptionResolutionType>`
    /// `SubscriptionResolutionType` is a struct which pairs a `Resolution` and a `BaseDataType`
    /// This is used to match data types to resolutions for consolidating data, and choosing correct consolidators automatically.
    async fn resolutions_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        market_type: MarketType,
        callback_id: u64
    ) -> DataServerResponse;

    /// return `DataServerResponse::Markets` or `DataServerResponse::Error(FundForgeError)`
    /// server or client error depending on who caused this problem
    async fn markets_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        callback_id: u64
    ) -> DataServerResponse;

    /// return `DataServerResponse::DecimalAccuracy` or `DataServerResponse::Error(FundForgeError)`
    /// server or client error depending on who caused this problem.
    /// decimal accuracy is an integer, for AUD-USD it is accurate to 5 decimal places
    async fn decimal_accuracy_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse;

    /// return `DataServerResponse::TickSize` or `DataServerResponse::Error(FundForgeError)`
    /// server or client error depending on who caused this problem
    async fn tick_size_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse;

    /// return `DataServerResponse::SubscribeResponse` or `DataServerResponse::Error(FundForgeError)`
    /// server or client error depending on who caused this problem
    /// The caller does not await this method, but it lets the strategy know if the subscription was successful.
    async fn data_feed_subscribe(
        &self,
        stream_name: StreamName,
        subscription: DataSubscription
    ) -> DataServerResponse;

    /// return `DataServerResponse::UnSubscribeResponse` or `DataServerResponse::Error(FundForgeError)`
    /// server or client error depending on who caused this problem
    /// The caller does not await this method, but it lets the strategy know if the subscription was successful.
    async fn data_feed_unsubscribe(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        subscription: DataSubscription,
    ) -> DataServerResponse;

    /// The server handle history responses, but you will need an update data function and latest bars function
    async fn base_data_types_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        callback_id: u64
    ) -> DataServerResponse;

    /// This command doesn't require a response,
    /// it is sent when a connection is dropped so that we can remove any items associated with the stream
    /// (strategy that is connected to this port)
    async fn logout_command_vendors(
        &self,
        stream_name: StreamName,
    );
}
