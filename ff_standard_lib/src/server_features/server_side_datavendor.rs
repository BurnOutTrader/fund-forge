use async_trait::async_trait;
use tokio::sync::mpsc::{Sender};
use crate::standardized_types::datavendor_enum::DataVendor;
use crate::server_features::rithmic_api::api_client::{get_rithmic_client};
use crate::server_features::StreamName;
use crate::server_features::test_api::api_client::TEST_CLIENT;
use crate::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use crate::server_features::bitget_api::api_client::{BITGET_CLIENT};
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::enums::{MarketType, StrategyMode};
use crate::standardized_types::subscriptions::{DataSubscription, SymbolName};

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
        subscription: DataSubscription,
        sender: Sender<BaseDataEnum>
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

/// Responses
#[async_trait]
impl VendorApiResponse for DataVendor {
    /// return `DataServerResponse::Symbols` or `DataServerResponse::Error(FundForgeError)`
    /// server or client error depending on who caused this problem
    async fn symbols_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        market_type: MarketType,
        callback_id: u64
    ) -> DataServerResponse {
        match self {
            DataVendor::Rithmic(system) => {
                if let Some(client) = get_rithmic_client(system) {
                    return client.symbols_response(mode, stream_name, market_type, callback_id).await
                }
            },
            DataVendor::Test => return TEST_CLIENT.symbols_response(mode, stream_name, market_type, callback_id).await,
            DataVendor::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.symbols_response(mode, stream_name, market_type, callback_id).await;
                }
            }
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

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
    ) -> DataServerResponse {
        match self {
            DataVendor::Rithmic(system) => {
                if let Some(client) = get_rithmic_client(system) {
                    return client.resolutions_response(mode, stream_name, market_type, callback_id).await
                }
            },
            DataVendor::Test => return TEST_CLIENT.resolutions_response(mode, stream_name, market_type, callback_id).await,
            DataVendor::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.resolutions_response(mode, stream_name, market_type, callback_id).await;
                }
            }
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    /// return `DataServerResponse::Markets` or `DataServerResponse::Error(FundForgeError)`
    /// server or client error depending on who caused this problem
    async fn markets_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        callback_id: u64
    ) -> DataServerResponse {
        match self {
            DataVendor::Rithmic(system) => {
                if let Some(client) = get_rithmic_client(system) {
                    return client.markets_response(mode, stream_name, callback_id).await
                }
            },
            DataVendor::Test => return TEST_CLIENT.markets_response(mode, stream_name, callback_id).await,
            DataVendor::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.markets_response(mode, stream_name, callback_id).await;
                }
            }
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    /// return `DataServerResponse::DecimalAccuracy` or `DataServerResponse::Error(FundForgeError)`
    /// server or client error depending on who caused this problem.
    /// decimal accuracy is an integer, for AUD-USD it is accurate to 5 decimal places
    async fn decimal_accuracy_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse {
        match self {
            DataVendor::Rithmic(system) => {
                if let Some(client) = get_rithmic_client(system) {
                    return client.decimal_accuracy_response(mode, stream_name, symbol_name, callback_id).await
                }
            },
            DataVendor::Test => return TEST_CLIENT.decimal_accuracy_response(mode, stream_name, symbol_name, callback_id).await,
            DataVendor::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.decimal_accuracy_response(mode, stream_name, symbol_name, callback_id).await;
                }
            }
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    /// return `DataServerResponse::TickSize` or `DataServerResponse::Error(FundForgeError)`
    /// server or client error depending on who caused this problem
    async fn tick_size_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse {
        match self {
            DataVendor::Rithmic(system) => {
                if let Some(client) = get_rithmic_client(system) {
                    return client.tick_size_response(mode, stream_name, symbol_name, callback_id).await
                }
            },
            DataVendor::Test => return TEST_CLIENT.tick_size_response(mode, stream_name, symbol_name, callback_id).await,
            DataVendor::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.tick_size_response(mode, stream_name, symbol_name, callback_id).await;
                }
            }
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    /// return `DataServerResponse::SubscribeResponse` or `DataServerResponse::Error(FundForgeError)`
    /// server or client error depending on who caused this problem
    /// The caller does not await this method, but it lets the strategy know if the subscription was successful.
    async fn data_feed_subscribe(
        &self,
        stream_name: StreamName,
        subscription: DataSubscription,
        sender: Sender<BaseDataEnum>
    ) -> DataServerResponse {
        match self {
            DataVendor::Rithmic(system) => {
                if let Some(client) = get_rithmic_client(system) {
                    return client.data_feed_subscribe(stream_name, subscription, sender).await
                }
            },
            DataVendor::Test => return TEST_CLIENT.data_feed_subscribe(stream_name, subscription, sender).await,
            DataVendor::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.data_feed_subscribe(stream_name, subscription, sender).await;
                }
            }
        }
        DataServerResponse::SubscribeResponse{ success: false, subscription, reason: Some(format!("Unable to find api client instance for: {}", self))}
    }

    /// return `DataServerResponse::UnSubscribeResponse` or `DataServerResponse::Error(FundForgeError)`
    /// server or client error depending on who caused this problem
    /// The caller does not await this method, but it lets the strategy know if the subscription was successful.
    async fn data_feed_unsubscribe(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        subscription: DataSubscription
    ) -> DataServerResponse {
        match self {
            DataVendor::Rithmic(system) => {
                if let Some(client) = get_rithmic_client(system) {
                    return client.data_feed_unsubscribe(mode, stream_name, subscription).await
                }
            },
            DataVendor::Test => return TEST_CLIENT.data_feed_unsubscribe(mode, stream_name, subscription).await,
            DataVendor::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.data_feed_unsubscribe(mode, stream_name, subscription).await;
                }
            }
        }
        DataServerResponse::UnSubscribeResponse{ success: false, subscription, reason: Some(format!("Unable to find api client instance for: {}", self))}
    }

    /// return `DataServerResponse::BaseData` or `DataServerResponse::Error(FundForgeError)`
    /// server or client error depending on who caused this problem
    async fn base_data_types_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        callback_id: u64
    ) -> DataServerResponse {
        match self {
            DataVendor::Rithmic(system) => {
                if let Some(client) = get_rithmic_client(system) {
                    return client.base_data_types_response(mode, stream_name, callback_id).await
                }
            },
            DataVendor::Test => return TEST_CLIENT.base_data_types_response(mode, stream_name, callback_id).await,
            DataVendor::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.base_data_types_response(mode, stream_name, callback_id).await;
                }
            }
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", self))}
    }

    /// This command doesn't require a response,
    /// it is sent when a connection is dropped so that we can remove any items associated with the stream
    /// (strategy that is connected to this port)
    async fn logout_command_vendors(&self, stream_name: StreamName) {
        match self {
        DataVendor::Rithmic(system) => {
            if let Some(client) = get_rithmic_client(system) {
                client.logout_command_vendors(stream_name).await
            }
        },
            DataVendor::Test => TEST_CLIENT.logout_command_vendors(stream_name).await,
            DataVendor::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    client.logout_command_vendors(stream_name).await;
                }
            }
        }
    }
}