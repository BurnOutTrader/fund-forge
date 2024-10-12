use crate::strategies::ledgers::AccountId;
use crate::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use crate::standardized_types::subscriptions::SymbolName;
use crate::standardized_types::enums::{StrategyMode};
use async_trait::async_trait;
use crate::server_features::bitget_api::api_client::{BITGET_CLIENT};
use crate::standardized_types::broker_enum::Brokerage;
use crate::server_features::rithmic_api::api_client::{get_rithmic_client, RITHMIC_CLIENTS};
use crate::server_features::StreamName;
use crate::server_features::test_api::api_client::TEST_CLIENT;
use crate::standardized_types::new_types::Volume;

/// The trait allows the server to implement the vendor specific methods for the DataVendor enum without the client needing to implement them.
#[async_trait]
pub trait BrokerApiResponse: Sync + Send {
    /// return `DataServerResponse::Symbols` or `DataServerResponse::Error(FundForgeError)`.
    /// server or client error depending on who caused this problem
    async fn symbol_names_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName, // The stream name is just the u16 port number the strategy is connecting to
        callback_id: u64
    ) -> DataServerResponse;

    /// return `DataServerResponse::AccountInfo` or `DataServerResponse::Error(FundForgeError)`
    /// server or client error depending on who caused this problem
    async fn account_info_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        account_id: AccountId,
        callback_id: u64
    ) -> DataServerResponse;

    /// return` DataServerResponse::SymbolInfo` or `DataServerResponse::Error(FundForgeError)`
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
    /// return `DataServerResponse::MarginRequired` or `DataServerResponse::Error(FundForgeError)`
    /// server or client error depending on who caused this problem
    async fn margin_required_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        symbol_name: SymbolName,
        quantity: Volume,
        callback_id: u64
    ) -> DataServerResponse;

    /// return `DataServerResponse::Accounts or DataServerResponse::Error(FundForgeError)`
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
}

/// Responses
#[async_trait]
impl BrokerApiResponse for Brokerage {
    /// return `DataServerResponse::Symbols` or `DataServerResponse::Error(FundForgeError)`.
    /// server or client error depending on who caused this problem
    async fn symbol_names_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        callback_id: u64
    ) -> DataServerResponse {
        match self {
            Brokerage::Rithmic(system) => {
                if let Some(client) = RITHMIC_CLIENTS.get(system) {
                    return client.symbol_names_response(mode, stream_name, callback_id).await
                }
            },
            Brokerage::Test => return TEST_CLIENT.symbol_names_response(mode, stream_name, callback_id).await,
            Brokerage::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.symbol_names_response(mode, stream_name, callback_id).await
                }
            }
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    /// return `DataServerResponse::AccountInfo` or `DataServerResponse::Error(FundForgeError)`
    /// server or client error depending on who caused this problem
    async fn account_info_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        account_id: AccountId,
        callback_id: u64
    ) -> DataServerResponse {
        match self {
            Brokerage::Rithmic(system) => {
                if let Some(client) = RITHMIC_CLIENTS.get(system) {
                    return client.account_info_response(mode, stream_name, account_id, callback_id).await
                }
            },
            Brokerage::Test => return TEST_CLIENT.account_info_response(mode, stream_name, account_id, callback_id).await,
            Brokerage::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.account_info_response(mode, stream_name, account_id, callback_id).await
                }
            }
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    async fn symbol_info_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        match self {
            Brokerage::Rithmic(system) => {
                if let Some(client) = RITHMIC_CLIENTS.get(system) {
                    return client.symbol_info_response(mode, stream_name, symbol_name, callback_id).await
                }
            },
            Brokerage::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.symbol_info_response(mode, stream_name, symbol_name, callback_id).await
                }
            }
            Brokerage::Test => return TEST_CLIENT.symbol_info_response(mode, stream_name, symbol_name, callback_id).await
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    /// Margin required for x units of the symbol, the mode is passed in
    /// We can return hard coded values for backtesting and live values for live or live paper
    /// return `DataServerResponse::MarginRequired` or `DataServerResponse::Error(FundForgeError)`
    /// server or client error depending on who caused this problem
    async fn margin_required_response(&self,  mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, quantity: Volume, callback_id: u64) -> DataServerResponse {
        match self {
            Brokerage::Rithmic(system) => {
                if let Some(client) = RITHMIC_CLIENTS.get(system) {
                    return client.margin_required_response(mode, stream_name, symbol_name, quantity, callback_id).await
                }
            },
            Brokerage::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.margin_required_response(mode, stream_name, symbol_name, quantity, callback_id).await
                }
            }
            Brokerage::Test => return TEST_CLIENT.margin_required_response(mode, stream_name, symbol_name, quantity, callback_id).await
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    /// return `DataServerResponse::Accounts or DataServerResponse::Error(FundForgeError)`
    /// server or client error depending on who caused this problem
    async fn accounts_response(&self, mode: StrategyMode, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        match self {
            Brokerage::Rithmic(system) => {
                if let Some(client) = RITHMIC_CLIENTS.get(system) {
                    return client.accounts_response(mode, stream_name, callback_id).await
                }
            },
            Brokerage::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.accounts_response(mode, stream_name, callback_id).await
                }
            }
            Brokerage::Test => return TEST_CLIENT.accounts_response(mode, stream_name, callback_id).await
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    /// This command doesn't require a response,
    /// it is sent when a connection is dropped so that we can remove any items associated with the stream
    /// (strategy that is connected to this port)
    async fn logout_command(&self, stream_name: StreamName) {
        match self {
            Brokerage::Rithmic(system) => {
                if let Some(client) = get_rithmic_client(system) {
                    client.logout_command(stream_name).await;
                }
            },
            Brokerage::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    client.logout_command(stream_name).await
                }
            }
            Brokerage::Test => TEST_CLIENT.logout_command(stream_name).await
        }
    }
}