use crate::standardized_types::accounts::ledgers::AccountId;
use crate::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use crate::standardized_types::subscriptions::SymbolName;
use crate::standardized_types::enums::{MarketType, StrategyMode};
use async_trait::async_trait;
use crate::standardized_types::broker_enum::Brokerage;
use crate::server_features::rithmic_api::api_client::RITHMIC_CLIENTS;
use crate::server_features::StreamName;
use crate::server_features::test_api::api_client::TEST_CLIENT;
use crate::standardized_types::new_types::Volume;

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

/// Responses
#[async_trait]
impl BrokerApiResponse for Brokerage {
    async fn symbols_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        market_type: MarketType,
        callback_id: u64
    ) -> DataServerResponse {
        match self {
            Brokerage::Rithmic(system) => {
                if let Some(client) = RITHMIC_CLIENTS.get(system) {
                    return client.symbols_response(mode, stream_name, market_type, callback_id).await
                }
            },
            Brokerage::Test => return TEST_CLIENT.symbols_response(mode, stream_name, market_type, callback_id).await
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

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
            Brokerage::Test => return TEST_CLIENT.account_info_response(mode, stream_name, account_id, callback_id).await
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    async fn symbol_info_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse {
        match self {
            Brokerage::Rithmic(system) => {
                if let Some(client) = RITHMIC_CLIENTS.get(system) {
                    return client.symbol_info_response(mode, stream_name, symbol_name, callback_id).await
                }
            },
            Brokerage::Test => return TEST_CLIENT.symbol_info_response(mode, stream_name, symbol_name, callback_id).await
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    async fn margin_required_response(&self,  mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, quantity: Volume, callback_id: u64) -> DataServerResponse {
        match self {
            Brokerage::Rithmic(system) => {
                if let Some(client) = RITHMIC_CLIENTS.get(system) {
                    return client.margin_required_response(mode, stream_name, symbol_name, quantity, callback_id).await
                }
            },
            Brokerage::Test => return TEST_CLIENT.margin_required_response(mode, stream_name, symbol_name, quantity, callback_id).await
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    async fn accounts_response(&self, mode: StrategyMode, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        match self {
            Brokerage::Rithmic(system) => {
                if let Some(client) = RITHMIC_CLIENTS.get(system) {
                    return client.accounts_response(mode, stream_name, callback_id).await
                }
            },
            Brokerage::Test => return TEST_CLIENT.accounts_response(mode, stream_name, callback_id).await
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }
}