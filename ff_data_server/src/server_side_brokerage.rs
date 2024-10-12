use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::server_features::server_side_brokerage::BrokerApiResponse;
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::enums::StrategyMode;
use ff_standard_lib::standardized_types::new_types::Volume;
use ff_standard_lib::standardized_types::subscriptions::SymbolName;
use ff_standard_lib::strategies::ledgers::AccountId;
use ff_standard_lib::StreamName;
use crate::bitget_api::api_client::BITGET_CLIENT;
use crate::rithmic_api::api_client::{get_rithmic_client, RITHMIC_CLIENTS};
use crate::test_api::api_client::TEST_CLIENT;

/// Responses
    /// return `DataServerResponse::Symbols` or `DataServerResponse::Error(FundForgeError)`.
    /// server or client error depending on who caused this problem
pub async fn symbol_names_response(
    brokerage: Brokerage,
    mode: StrategyMode,
    stream_name: StreamName,
    callback_id: u64
) -> DataServerResponse {
    match brokerage {
        Brokerage::Rithmic(system) => {
            if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                return client.value().symbol_names_response(mode, stream_name, callback_id).await
            }
        },
        Brokerage::Test => return TEST_CLIENT.symbol_names_response(mode, stream_name, callback_id).await,
        Brokerage::Bitget => {
            if let Some(client) = BITGET_CLIENT.get() {
                return client.symbol_names_response(mode, stream_name, callback_id).await
            }
        }
    }
    DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", brokerage))}
}

/// return `DataServerResponse::AccountInfo` or `DataServerResponse::Error(FundForgeError)`
/// server or client error depending on who caused this problem
pub async fn account_info_response(
    brokerage: Brokerage,
    mode: StrategyMode,
    stream_name: StreamName,
    account_id: AccountId,
    callback_id: u64
) -> DataServerResponse {
    match brokerage {
        Brokerage::Rithmic(system) => {
            if let Some(client) = RITHMIC_CLIENTS.get(&system) {
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
    DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", brokerage))}
}

pub async fn symbol_info_response(brokerage: Brokerage, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
    match brokerage {
        Brokerage::Rithmic(system) => {
            if let Some(client) = RITHMIC_CLIENTS.get(&system) {
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
    DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", brokerage))}
}

/// Margin required for x units of the symbol, the mode is passed in
/// We can return hard coded values for backtesting and live values for live or live paper
/// return `DataServerResponse::MarginRequired` or `DataServerResponse::Error(FundForgeError)`
/// server or client error depending on who caused this problem
pub async fn margin_required_response(brokerage: Brokerage,  mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, quantity: Volume, callback_id: u64) -> DataServerResponse {
    match brokerage {
        Brokerage::Rithmic(system) => {
            if let Some(client) = RITHMIC_CLIENTS.get(&system) {
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
    DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", brokerage))}
}

/// return `DataServerResponse::Accounts or DataServerResponse::Error(FundForgeError)`
/// server or client error depending on who caused this problem
pub async fn accounts_response(brokerage: Brokerage, mode: StrategyMode, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
    match brokerage {
        Brokerage::Rithmic(system) => {
            if let Some(client) = RITHMIC_CLIENTS.get(&system) {
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
    DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", brokerage))}
}

/// This command doesn't require a response,
/// it is sent when a connection is dropped so that we can remove any items associated with the stream
/// (strategy that is connected to this port)
pub async fn logout_command(brokerage: Brokerage, stream_name: StreamName) {
    match brokerage {
        Brokerage::Rithmic(system) => {
            if let Some(client) = get_rithmic_client(&system) {
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