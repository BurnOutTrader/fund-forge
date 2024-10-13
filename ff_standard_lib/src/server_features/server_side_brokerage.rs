use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crate::messages::data_server_messaging::DataServerResponse;
use crate::standardized_types::enums::StrategyMode;
use crate::standardized_types::new_types::Volume;
use crate::standardized_types::subscriptions::SymbolName;
use crate::strategies::ledgers::AccountId;
use crate::StreamName;

/// The trait allows the server to implement the vendor specific methods for the DataVendor enum without the client needing to implement them.
#[async_trait]
pub trait BrokerApiResponse: Sync + Send {
    /// return `DataServerResponse::Symbols` or `DataServerResponse::Error{error: FundForgeError, callback_id: u64}`
    /// `FundForgeError::ServerSideErrorDebug` or `FundForgeError::ClientSideErrorDebug` depending on who caused this problem.
    ///
    /// You can return a hard coded list here for most brokers, equities we might want to get dynamically from the brokerage.
    async fn symbol_names_response(
        &self,
        mode: StrategyMode,
        time: Option<DateTime<Utc>>,
        // The `stream_name` is just the u16 port number of the strategy which the server is connecting to,
        // it is used to link the streaming port to a async port, you just need to know it represents a single strategy instance.
        stream_name: StreamName,
        callback_id: u64
    ) -> DataServerResponse;

    /// return `DataServerResponse::AccountInfo` or `DataServerResponse::Error{error: FundForgeError, callback_id: u64}`
    /// `FundForgeError::ServerSideErrorDebug` or `FundForgeError::ClientSideErrorDebug` depending on who caused this problem.
    async fn account_info_response(
        &self,
        mode: StrategyMode,
        // The `stream_name` is just the u16 port number of the strategy which the server is connecting to,
        // it is used to link the streaming port to a async port, you just need to know it represents a single strategy instance.
        stream_name: StreamName,
        account_id: AccountId,
        callback_id: u64
    ) -> DataServerResponse;

    /// return` DataServerResponse::SymbolInfo` or `DataServerResponse::Error{error: FundForgeError, callback_id: u64}`
    /// `FundForgeError::ServerSideErrorDebug` or `FundForgeError::ClientSideErrorDebug` depending on who caused this problem.
    ///
    /// You can return a hard coded list here for most brokers, equities we might want to get dynamically from the brokerage.
    async fn symbol_info_response(
        &self,
        mode: StrategyMode,
        // The `stream_name` is just the u16 port number of the strategy which the server is connecting to,
        // it is used to link the streaming port to a async port, you just need to know it represents a single strategy instance.
        stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse;

    /// return `DataServerResponse::IntraDayMarginRequired` or `DataServerResponse::Error{error: FundForgeError, callback_id: u64}`
    /// `FundForgeError::ServerSideErrorDebug` or `FundForgeError::ClientSideErrorDebug` depending on who caused this problem.
    ///
    /// Margin required for x units of the symbol, the mode is passed in
    /// We can return hard coded values for backtesting and live values for live or live paper
    /// You can return a hard coded list here for most brokers, since margin requirements can change we might want to handle historical and live differently.
    /// equities we might want to get dynamically from the brokerage, or use a calculation based on current market price
    async fn intraday_margin_required_response(
        &self,
        mode: StrategyMode,
        // The `stream_name` is just the u16 port number of the strategy which the server is connecting to,
        // it is used to link the streaming port to a async port, you just need to know it represents a single strategy instance.
        stream_name: StreamName,
        symbol_name: SymbolName,
        quantity: Volume,
        callback_id: u64
    ) -> DataServerResponse;


    /// return `DataServerResponse::OvernightMarginRequired` or `DataServerResponse::Error{error: FundForgeError, callback_id: u64}`
    /// `FundForgeError::ServerSideErrorDebug` or `FundForgeError::ClientSideErrorDebug` depending on who caused this problem.
    ///
    /// Margin required for x units of the symbol, the mode is passed in
    /// We can return hard coded values for backtesting and live values for live or live paper
    /// You can return a hard coded list here for most brokers, since margin requirements can change we might want to handle historical and live differently.
    /// equities we might want to get dynamically from the brokerage, or use a calculation based on current market price
    async fn overnight_margin_required_response(
        &self,
        mode: StrategyMode,
        // The `stream_name` is just the u16 port number of the strategy which the server is connecting to,
        // it is used to link the streaming port to a async port, you just need to know it represents a single strategy instance.
        stream_name: StreamName,
        symbol_name: SymbolName,
        quantity: Volume,
        callback_id: u64
    ) -> DataServerResponse;

    /// return `DataServerResponse::Accounts or DataServerResponse::Error{error: FundForgeError, callback_id: u64}`
    /// `FundForgeError::ServerSideErrorDebug` or `FundForgeError::ClientSideErrorDebug` depending on who caused this problem.
    async fn accounts_response(
        &self,
        mode: StrategyMode,
        // The `stream_name` is just the u16 port number of the strategy which the server is connecting to,
        // it is used to link the streaming port to a async port, you just need to know it represents a single strategy instance.
        stream_name: StreamName,
        callback_id: u64
    ) -> DataServerResponse;

    /// This command doesn't require a response,
    /// it is sent when a connection is dropped so that we can remove any items associated with the stream
    /// (strategy that is connected to this port)
    async fn logout_command(
        &self,
        // The `stream_name` is just the u16 port number of the strategy which the server is connecting to,
        // it is used to link the streaming port to a async port, you just need to know it represents a single strategy instance.
        stream_name: StreamName,
    );

    /// Returns a `DataServerResponse::CommissionInfo` object if symbol is found, else returns a `DataServerResponse::Error{error: FundForgeError, callback_id: u64}`
    /// `FundForgeError::ServerSideErrorDebug` or `FundForgeError::ClientSideErrorDebug` depending on who caused this problem.
    ///
    /// You can return a hard coded list here for most brokers or we can ask the broker dynamically,
    /// we could also serialize a static csv and parse it to a static map on start up, this would allow us to manually edit changes to commissions.
    async fn commission_info_response(
        &self,
        mode: StrategyMode,
        // The `stream_name` is just the u16 port number of the strategy which the server is connecting to,
        // it is used to link the streaming port to a async port, you just need to know it represents a single strategy instance.
        stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse;
}