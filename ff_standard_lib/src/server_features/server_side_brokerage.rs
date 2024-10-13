use async_trait::async_trait;
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
    /// server or client error depending on who caused this problem
    async fn symbol_names_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName, // The stream name is just the u16 port number the strategy is connecting to
        callback_id: u64
    ) -> DataServerResponse;

    /// return `DataServerResponse::AccountInfo` or `DataServerResponse::Error{error: FundForgeError, callback_id: u64}`
    /// server or client error depending on who caused this problem
    async fn account_info_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        account_id: AccountId,
        callback_id: u64
    ) -> DataServerResponse;

    /// return` DataServerResponse::SymbolInfo` or `DataServerResponse::Error{error: FundForgeError, callback_id: u64}`
    /// server or client error depending on who caused this problem
    async fn symbol_info_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse;

    /// Margin required for x units of the symbol, the mode is passed in
    /// We can return hard coded values for backtesting and live values for live or live paper
    /// return `DataServerResponse::MarginRequired` or `DataServerResponse::Error{error: FundForgeError, callback_id: u64}`
    /// server or client error depending on who caused this problem
    async fn intraday_margin_required_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        symbol_name: SymbolName,
        quantity: Volume,
        callback_id: u64
    ) -> DataServerResponse;

    async fn overnight_margin_required_response(
        &self,
        _mode: StrategyMode,
        _stream_name: StreamName,
        symbol_name: SymbolName,
        quantity: Volume,
        callback_id: u64
    ) -> DataServerResponse;

    /// return `DataServerResponse::Accounts or DataServerResponse::Error{error: FundForgeError, callback_id: u64}`
    /// server or client error depending on who caused this problem
    async fn accounts_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        callback_id: u64
    ) -> DataServerResponse;

    /// This command doesn't require a response,
    /// it is sent when a connection is dropped so that we can remove any items associated with the stream
    /// (strategy that is connected to this port)
    async fn logout_command(
        &self,
        stream_name: StreamName,
    );

    /// Returns a `DataServerResponse::CommissionInfo` object if symbol is found, else returns a `DataServerResponse::Error{error: FundForgeError, callback_id: u64}`
    async fn commission_info_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse;
}