use async_trait::async_trait;
use chrono::{DateTime, Utc};
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

    /// return `DataServerResponse::BaseDataTypes` or `DataServerResponse::Error(FundForgeError)`
    /// This is to help the strategy engine determine which data types are supplied by the `DataVendor`
    /// We only need to supply the types that we want to use for subscriptions.
    /// There is no point using candles if we have tick history and live data.
    /// There is no point using QuoteBars if we have quote history and live data.
    /// We may need to match based on `StrategyMode` maybe we only have backtest data as candles but in live we can use ticks.
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

    /// return `DataServerResponse::SessionMarketHours` or `DataServerResponse::Error(FundForgeError)` //todo build historical closing hours function and add it to the function BaseDataEnum::serialize_and_save()
    /// we might be able to get this from the vendor when trading live, or we could use serialized lists.
    /// In historical we use the datetime to determine the date, so we can determine the response.
    /// The level of detail we go into here can change over time, but we could create a hardcoded historical list of session times easily by using a function to generate a historical map<Date, (open_time, close_time)> of open and close times
    /// based on historical data then we could serialize that map to disk and use it for backtesting look-ups of closing hours.
    ///
    /// `has_close: bool` would be `false` for crypto, true for `forex` regardless of day, since forex has a close.
    ///
    /// `is_24_hour: bool` would be `true` for crypto and forex.
    ///
    /// `pub open_utc: Option<String>` would always be `None` for crypto and `Some(time.to_utc().to_string())` for forex only on market open (New Zealand monday morning).
    ///
    /// `pub close_utc: Option<String>` would always be None for crypto and `None` for forex midweek and `Some(time.to_utc().to_string())` on close of US markets Friday or Saturday night, depending on original time zone `(Tz)`.
    /// ```rust
    /// use chrono::{DateTime, Utc};
    ///
    /// pub struct SessionMarketHours {
    ///     pub has_close: bool,
    ///     pub is_24_hour: bool,
    ///     pub open_time_utc_string: Option<String>,
    ///     pub close_time_utc_string: Option<String>,
    /// }
    /// ```
    async fn session_market_hours_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        symbol_name: SymbolName,
        date_time: DateTime<Utc>,
        callback_id: u64
    ) -> DataServerResponse;
}
