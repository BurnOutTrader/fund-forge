use async_trait::async_trait;
use tokio::sync::mpsc::{Sender};
use crate::apis::data_vendor::datavendor_enum::DataVendor;
use crate::apis::rithmic_api::api_client::{get_rithmic_client};
use crate::apis::StreamName;
use crate::apis::test_api::TEST_CLIENT;
use crate::standardized_types::data_server_messaging::{DataServerResponse, FundForgeError};
use crate::standardized_types::enums::{MarketType, StrategyMode};
use crate::standardized_types::subscriptions::{DataSubscription, SymbolName};

/// The trait allows the server to implement the vendor specific methods for the DataVendor enum without the client needing to implement them.
#[async_trait]
pub trait VendorApiResponse: Sync + Send {
    async fn symbols_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        market_type: MarketType,
        callback_id: u64
    ) -> DataServerResponse;
    async fn resolutions_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        market_type: MarketType,
        callback_id: u64
    ) -> DataServerResponse;
    async fn markets_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        callback_id: u64
    ) -> DataServerResponse;
    async fn decimal_accuracy_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse;
    async fn tick_size_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse;
    async fn data_feed_subscribe(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        subscription: DataSubscription,
        sender: Sender<DataServerResponse>
    ) -> DataServerResponse;
    async fn data_feed_unsubscribe(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        subscription: DataSubscription,
    ) -> DataServerResponse;
    async fn base_data_types_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        callback_id: u64
    ) -> DataServerResponse;
}

/// Responses
#[async_trait]
impl VendorApiResponse for DataVendor {
    async fn symbols_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        market_type: MarketType,
        callback_id: u64
    ) -> DataServerResponse {
        match self {
            DataVendor::Rithmic(_) => {
                if let Some(client) = get_rithmic_client(&self) {
                    return client.symbols_response(mode, stream_name, market_type, callback_id).await
                }
            },
            DataVendor::Test => return TEST_CLIENT.symbols_response(mode, stream_name, market_type, callback_id).await
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    async fn resolutions_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        market_type: MarketType,
        callback_id: u64
    ) -> DataServerResponse {
        match self {
            DataVendor::Rithmic(_) => {
                if let Some(client) = get_rithmic_client(self) {
                    return client.resolutions_response(mode, stream_name, market_type, callback_id).await
                }
            },
            DataVendor::Test => return TEST_CLIENT.resolutions_response(mode, stream_name, market_type, callback_id).await
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    async fn markets_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        callback_id: u64
    ) -> DataServerResponse {
        match self {
            DataVendor::Rithmic(_) => {
                if let Some(client) = get_rithmic_client(self) {
                    return client.markets_response(mode, stream_name, callback_id).await
                }
            },
            DataVendor::Test => return TEST_CLIENT.markets_response(mode, stream_name, callback_id).await
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    async fn decimal_accuracy_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse {
        match self {
            DataVendor::Rithmic(_) => {
                if let Some(client) = get_rithmic_client(self) {
                    return client.decimal_accuracy_response(mode, stream_name, symbol_name, callback_id).await
                }
            },
            DataVendor::Test => return TEST_CLIENT.decimal_accuracy_response(mode, stream_name, symbol_name, callback_id).await
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    async fn tick_size_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse {
        match self {
            DataVendor::Rithmic(_) => {
                if let Some(client) = get_rithmic_client(self) {
                    return client.tick_size_response(mode, stream_name, symbol_name, callback_id).await
                }
            },
            DataVendor::Test => return TEST_CLIENT.tick_size_response(mode, stream_name, symbol_name, callback_id).await
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    async fn data_feed_subscribe(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        subscription: DataSubscription,
        sender: Sender<DataServerResponse>
    ) -> DataServerResponse {
        match self {
            DataVendor::Rithmic(_) => {
                if let Some(client) = get_rithmic_client(self) {
                    return client.data_feed_subscribe(mode, stream_name, subscription, sender).await
                }
            },
            DataVendor::Test => return TEST_CLIENT.data_feed_subscribe(mode, stream_name, subscription, sender).await
        }
        DataServerResponse::SubscribeResponse{ success: false, subscription, reason: Some(format!("Unable to find api client instance for: {}", self))}
    }

    async fn data_feed_unsubscribe(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        subscription: DataSubscription
    ) -> DataServerResponse {
        match self {
            DataVendor::Rithmic(_) => {
                if let Some(client) = get_rithmic_client(self) {
                    return client.data_feed_unsubscribe(mode, stream_name, subscription).await
                }
            },
            DataVendor::Test => return TEST_CLIENT.data_feed_unsubscribe(mode, stream_name, subscription).await
        }
        DataServerResponse::UnSubscribeResponse{ success: false, subscription, reason: Some(format!("Unable to find api client instance for: {}", self))}
    }

    async fn base_data_types_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        callback_id: u64
    ) -> DataServerResponse {
        match self {
            DataVendor::Rithmic(_) => {
                if let Some(client) = get_rithmic_client(self) {
                    return client.base_data_types_response(mode, stream_name, callback_id).await
                }
            },
            DataVendor::Test => return TEST_CLIENT.base_data_types_response(mode, stream_name, callback_id).await
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }
}