use std::str::FromStr;
use chrono::{DateTime, Utc};
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::server_features::server_side_datavendor::VendorApiResponse;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::enums::{MarketType, StrategyMode};
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, SymbolName};
use ff_standard_lib::StreamName;
use crate::bitget_api::api_client::BITGET_CLIENT;
use crate::rithmic_api::api_client::{get_rithmic_client, RITHMIC_CLIENTS};
use crate::test_api::api_client::TEST_CLIENT;

// Responses


/// return `DataServerResponse::SessionMarketHours` or `DataServerResponse::Error(FundForgeError)`.
pub async fn session_market_hours_response(mode: StrategyMode, data_vendor: DataVendor, symbol_name: SymbolName, date: String, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
    let time = match DateTime::<Utc>::from_str(&date) {
        Ok(time) => time,
        Err(e) => return DataServerResponse::Error {error: FundForgeError::ClientSideErrorDebug(format!("{}", e)), callback_id}
    };
    match data_vendor {
        DataVendor::Rithmic(system) => {
            if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                return client.value().session_market_hours_response(mode, stream_name, symbol_name, time, callback_id).await
            }
        },
        DataVendor::Test => return TEST_CLIENT.session_market_hours_response(mode, stream_name, symbol_name, time, callback_id).await,
        DataVendor::Bitget => {
            if let Some(client) = BITGET_CLIENT.get() {
                return client.session_market_hours_response(mode, stream_name, symbol_name, time, callback_id).await
            }
        }
    }
    DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", data_vendor))}
}

/// return `DataServerResponse::Symbols` or `DataServerResponse::Error(FundForgeError)`
/// server or client error depending on who caused this problem
/// `time` is optional, during backtesting conditions some markets such as equities might require us to filter the symbols that were available during the time period.
pub async fn symbols_response(
    data_vendor: DataVendor,
    mode: StrategyMode,
    stream_name: StreamName,
    market_type: MarketType,
    time: Option<DateTime<Utc>>,
    callback_id: u64
) -> DataServerResponse {
    match data_vendor {
        DataVendor::Rithmic(system) => {
            if let Some(client) = get_rithmic_client(&system) {
                return client.symbols_response(mode, stream_name, market_type, time, callback_id).await
            }
        },
        DataVendor::Test => return TEST_CLIENT.symbols_response(mode, stream_name, market_type, time, callback_id).await,
        DataVendor::Bitget => {
            if let Some(client) = BITGET_CLIENT.get() {
                return client.symbols_response(mode, stream_name, market_type, time, callback_id).await;
            }
        }
    }
    DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", data_vendor))}
}

/// return `DataServerResponse::Resolutions` or `DataServerResponse::Error(FundForgeError)`
/// server or client error depending on who caused this problem
/// Note that we are not just returning resolutions here,
/// we return `Vec<SubscriptionResolutionType>`
/// `SubscriptionResolutionType` is a struct which pairs a `Resolution` and a `BaseDataType`
/// This is used to match data types to resolutions for consolidating data, and choosing correct consolidators automatically.
pub async fn resolutions_response(
    data_vendor: DataVendor,
    mode: StrategyMode,
    stream_name: StreamName,
    market_type: MarketType,
    callback_id: u64
) -> DataServerResponse {
    match data_vendor {
        DataVendor::Rithmic(system) => {
            if let Some(client) = get_rithmic_client(&system) {
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
    DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", data_vendor))}
}

/// return `DataServerResponse::Markets` or `DataServerResponse::Error(FundForgeError)`
/// server or client error depending on who caused this problem
pub async fn markets_response(
    data_vendor: DataVendor,
    mode: StrategyMode,
    stream_name: StreamName,
    callback_id: u64
) -> DataServerResponse {
    match data_vendor {
        DataVendor::Rithmic(system) => {
            if let Some(client) = get_rithmic_client(&system) {
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
    DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", data_vendor))}
}

/// return `DataServerResponse::DecimalAccuracy` or `DataServerResponse::Error(FundForgeError)`
/// server or client error depending on who caused this problem.
/// decimal accuracy is an integer, for AUD-USD it is accurate to 5 decimal places
pub async fn decimal_accuracy_response(
    data_vendor: DataVendor,
    mode: StrategyMode,
    stream_name: StreamName,
    symbol_name: SymbolName,
    callback_id: u64
) -> DataServerResponse {
    match data_vendor {
        DataVendor::Rithmic(system) => {
            if let Some(client) = get_rithmic_client(&system) {
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
    DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", data_vendor))}
}

/// return `DataServerResponse::TickSize` or `DataServerResponse::Error(FundForgeError)`
/// server or client error depending on who caused this problem
pub async fn tick_size_response(
    data_vendor: DataVendor,
    mode: StrategyMode,
    stream_name: StreamName,
    symbol_name: SymbolName,
    callback_id: u64
) -> DataServerResponse {
    match data_vendor {
        DataVendor::Rithmic(system) => {
            if let Some(client) = get_rithmic_client(&system) {
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
    DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", data_vendor))}
}

/// return `DataServerResponse::SubscribeResponse` or `DataServerResponse::Error(FundForgeError)`
/// server or client error depending on who caused this problem
/// The caller does not await this method, but it lets the strategy know if the subscription was successful.
pub async fn data_feed_subscribe(
    stream_name: StreamName,
    subscription: DataSubscription
) -> DataServerResponse {
    match &subscription.symbol.data_vendor {
        DataVendor::Rithmic(system) => {
            if let Some(client) = get_rithmic_client(system) {
                return client.data_feed_subscribe(stream_name, subscription).await
            }
        },
        DataVendor::Test => return TEST_CLIENT.data_feed_subscribe(stream_name, subscription).await,
        DataVendor::Bitget => {
            if let Some(client) = BITGET_CLIENT.get() {
                return client.data_feed_subscribe(stream_name, subscription).await;
            }
        }
    }
    DataServerResponse::SubscribeResponse{ success: false, subscription: subscription.clone(), reason: Some(format!("Unable to find api client instance for: {}", subscription.symbol.data_vendor))}
}

/// return `DataServerResponse::UnSubscribeResponse` or `DataServerResponse::Error(FundForgeError)`
/// server or client error depending on who caused this problem
/// The caller does not await this method, but it lets the strategy know if the subscription was successful.
pub async fn data_feed_unsubscribe(
    data_vendor: DataVendor,
    mode: StrategyMode,
    stream_name: StreamName,
    subscription: DataSubscription
) -> DataServerResponse {
    match data_vendor {
        DataVendor::Rithmic(system) => {
            if let Some(client) = get_rithmic_client(&system) {
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
    DataServerResponse::UnSubscribeResponse{ success: false, subscription, reason: Some(format!("Unable to find api client instance for: {}", data_vendor))}
}

/// return `DataServerResponse::BaseData` or `DataServerResponse::Error(FundForgeError)`
/// server or client error depending on who caused this problem
pub async fn base_data_types_response(
    data_vendor: DataVendor,
    mode: StrategyMode,
    stream_name: StreamName,
    callback_id: u64
) -> DataServerResponse {
    match data_vendor {
        DataVendor::Rithmic(system) => {
            if let Some(client) = get_rithmic_client(&system) {
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
    DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", data_vendor))}
}

/// This command doesn't require a response,
/// it is sent when a connection is dropped so that we can remove any items associated with the stream
/// (strategy that is connected to this port)
pub async fn logout_command_vendors(data_vendor: DataVendor, stream_name: StreamName) {
    match data_vendor {
    DataVendor::Rithmic(system) => {
        if let Some(client) = get_rithmic_client(&system) {
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
