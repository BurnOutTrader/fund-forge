use std::str::FromStr;
use chrono::{DateTime, Utc};
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::enums::{MarketType, StrategyMode};
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, SymbolName};
use ff_standard_lib::StreamName;
use crate::bitget_api::api_client::BITGET_CLIENT;
use crate::rithmic_api::api_client::{get_rithmic_market_data_system, RITHMIC_CLIENTS};
use tokio::time::{timeout, Duration};
use crate::data_bento_api::api_client::get_data_bento_client;
use crate::oanda_api::api_client::OANDA_CLIENT;
use crate::server_features::server_side_datavendor::VendorApiResponse;

const TIMEOUT_DURATION: Duration = Duration::from_secs(10);

// Responses
/// return `DataServerResponse::SessionMarketHours` or `DataServerResponse::Error(FundForgeError)`.
pub async fn session_market_hours_response(mode: StrategyMode, data_vendor: DataVendor, symbol_name: SymbolName, date: String, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
    let operation = async {
        let time = match DateTime::<Utc>::from_str(&date) {
            Ok(time) => time,
            Err(e) => return DataServerResponse::Error {error: FundForgeError::ClientSideErrorDebug(format!("{}", e)), callback_id}
        };
        match data_vendor {
            DataVendor::Rithmic => {
                let system = match get_rithmic_market_data_system() {
                    Some(system) => system,
                    None => return DataServerResponse::Error {error: FundForgeError::ServerErrorDebug("Rithmic market data system not found".to_string()), callback_id}
                };
                if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                    return client.value().session_market_hours_response(mode, stream_name, symbol_name, time, callback_id).await
                }
            },
            DataVendor::DataBento => {
                return match get_data_bento_client() {
                    Ok(client) => client.session_market_hours_response(mode, stream_name, symbol_name, time, callback_id).await,
                    Err(e) => DataServerResponse::Error { error: e, callback_id }
                }
            }
            DataVendor::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.session_market_hours_response(mode, stream_name, symbol_name, time, callback_id).await
                }
            }
            DataVendor::Oanda => {
                if let Some(client) = OANDA_CLIENT.get() {
                    return client.session_market_hours_response(mode, stream_name, symbol_name, time, callback_id).await
                }
            }
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", data_vendor))}
    };

    timeout(TIMEOUT_DURATION, operation).await.unwrap_or_else(|_| DataServerResponse::Error { callback_id, error: FundForgeError::ServerErrorDebug("Operation timed out".to_string()) })
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
    let operation = async {
        match data_vendor {
            DataVendor::Rithmic=> {
                let system = match get_rithmic_market_data_system() {
                    Some(system) => system,
                    None => return DataServerResponse::Error {error: FundForgeError::ServerErrorDebug("Rithmic market data system not found".to_string()), callback_id}
                };
                if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                    return client.symbols_response(mode, stream_name, market_type, time, callback_id).await
                }
            },
            DataVendor::DataBento => {
                return match get_data_bento_client() {
                    Ok(client) => client.symbols_response(mode, stream_name, market_type, time, callback_id).await,
                    Err(e) => DataServerResponse::Error { error: e, callback_id }
                }
            }
            DataVendor::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.symbols_response(mode, stream_name, market_type, time, callback_id).await;
                }
            }
            DataVendor::Oanda => {
                if let Some(client) = OANDA_CLIENT.get() {
                    return client.symbols_response(mode, stream_name, market_type, time, callback_id).await;
                }
            }
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", data_vendor))}
    };

    timeout(TIMEOUT_DURATION, operation).await.unwrap_or_else(|_| DataServerResponse::Error { callback_id, error: FundForgeError::ServerErrorDebug("Operation timed out".to_string()) })
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
    let operation = async {
        match data_vendor {
            DataVendor::Rithmic => {
                let system = match get_rithmic_market_data_system() {
                    Some(system) => system,
                    None => return DataServerResponse::Error {error: FundForgeError::ServerErrorDebug("Rithmic market data system not found".to_string()), callback_id}
                };
                if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                    return client.resolutions_response(mode, stream_name, market_type, callback_id).await
                }
            },
            DataVendor::DataBento => {
                return match get_data_bento_client() {
                    Ok(client) => client.resolutions_response(mode, stream_name, market_type, callback_id).await,
                    Err(e) => DataServerResponse::Error { error: e, callback_id }
                }
            }
            DataVendor::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.resolutions_response(mode, stream_name, market_type, callback_id).await;
                }
            }
            DataVendor::Oanda => {
                if let Some(client) = OANDA_CLIENT.get() {
                    return client.resolutions_response(mode, stream_name, market_type, callback_id).await;
                }
            }
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", data_vendor))}
    };

    timeout(TIMEOUT_DURATION, operation).await.unwrap_or_else(|_| DataServerResponse::Error { callback_id, error: FundForgeError::ServerErrorDebug("Operation timed out".to_string()) })
}

/// return `DataServerResponse::Markets` or `DataServerResponse::Error(FundForgeError)`
/// server or client error depending on who caused this problem
pub async fn markets_response(
    data_vendor: DataVendor,
    mode: StrategyMode,
    stream_name: StreamName,
    callback_id: u64
) -> DataServerResponse {
    let operation = async {
        match data_vendor {
            DataVendor::Rithmic => {
                let system = match get_rithmic_market_data_system() {
                    Some(system) => system,
                    None => return DataServerResponse::Error {error: FundForgeError::ServerErrorDebug("Rithmic market data system not found".to_string()), callback_id}
                };
                if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                    return client.markets_response(mode, stream_name, callback_id).await
                }
            },
            DataVendor::DataBento => {
                return match get_data_bento_client() {
                    Ok(client) => client.markets_response(mode, stream_name, callback_id).await,
                    Err(e) => DataServerResponse::Error { error: e, callback_id }
                }
            }
            DataVendor::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.markets_response(mode, stream_name, callback_id).await;
                }
            }
            DataVendor::Oanda => {
                if let Some(client) = OANDA_CLIENT.get() {
                    return client.markets_response(mode, stream_name, callback_id).await;
                }
            }
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", data_vendor))}
    };

    timeout(TIMEOUT_DURATION, operation).await.unwrap_or_else(|_| DataServerResponse::Error { callback_id, error: FundForgeError::ServerErrorDebug("Operation timed out".to_string()) })
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
    let operation = async {
        match data_vendor {
            DataVendor::Rithmic => {
                let system = match get_rithmic_market_data_system() {
                    Some(system) => system,
                    None => return DataServerResponse::Error {error: FundForgeError::ServerErrorDebug("Rithmic market data system not found".to_string()), callback_id}
                };
                if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                    return client.decimal_accuracy_response(mode, stream_name, symbol_name, callback_id).await
                }
            },
            DataVendor::DataBento => {
                return match get_data_bento_client() {
                    Ok(client) => client.decimal_accuracy_response(mode, stream_name, symbol_name, callback_id).await,
                    Err(e) => DataServerResponse::Error { error: e, callback_id }
                }
            }
            DataVendor::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.decimal_accuracy_response(mode, stream_name, symbol_name, callback_id).await;
                }
            }
            DataVendor::Oanda => {
                if let Some(client) = OANDA_CLIENT.get() {
                    return client.decimal_accuracy_response(mode, stream_name, symbol_name, callback_id).await;
                }
            }
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", data_vendor))}
    };

    timeout(TIMEOUT_DURATION, operation).await.unwrap_or_else(|_| DataServerResponse::Error { callback_id, error: FundForgeError::ServerErrorDebug("Operation timed out".to_string()) })
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
    let operation = async {
        match data_vendor {
            DataVendor::Rithmic => {
                let system = match get_rithmic_market_data_system() {
                    Some(system) => system,
                    None => return DataServerResponse::Error {error: FundForgeError::ServerErrorDebug("Rithmic market data system not found".to_string()), callback_id}
                };
                if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                    return client.tick_size_response(mode, stream_name, symbol_name, callback_id).await
                }
            },
            DataVendor::DataBento => {
                return match get_data_bento_client() {
                    Ok(client) => client.tick_size_response(mode, stream_name, symbol_name, callback_id).await,
                    Err(e) => DataServerResponse::Error { error: e, callback_id }
                }
            }
            DataVendor::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.tick_size_response(mode, stream_name, symbol_name, callback_id).await;
                }
            }
            DataVendor::Oanda => {
                if let Some(client) = OANDA_CLIENT.get() {
                    return client.tick_size_response(mode, stream_name, symbol_name, callback_id).await;
                }
            }
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", data_vendor))}
    };

    timeout(TIMEOUT_DURATION, operation).await.unwrap_or_else(|_| DataServerResponse::Error { callback_id, error: FundForgeError::ServerErrorDebug("Operation timed out".to_string()) })
}

/// return `DataServerResponse::SubscribeResponse` or `DataServerResponse::Error(FundForgeError)`
/// server or client error depending on who caused this problem
/// The caller does not await this method, but it lets the strategy know if the subscription was successful.
pub async fn data_feed_subscribe(
    stream_name: StreamName,
    subscription: DataSubscription,
) -> DataServerResponse {
    let operation = async {
        match &subscription.symbol.data_vendor {
            DataVendor::Rithmic=> {
                let system = match get_rithmic_market_data_system() {
                    Some(system) => system,
                    None => return DataServerResponse::SubscribeResponse{ success: false, subscription: subscription.clone(), reason: Some(format!("Unable to find api client instance for: {}", subscription.symbol.data_vendor))}
                };
                if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                    return client.data_feed_subscribe(stream_name, subscription.clone()).await
                }
            },
            DataVendor::DataBento => {
                return match get_data_bento_client() {
                    Ok(client) => client.data_feed_subscribe(stream_name, subscription.clone()).await,
                    Err(e) => DataServerResponse::SubscribeResponse { success: false, subscription: subscription.clone(), reason: Some(format!("{}", e))}
                }
            }
            DataVendor::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.data_feed_subscribe(stream_name, subscription.clone()).await;
                }
            }
            DataVendor::Oanda => {
                if let Some(client) = OANDA_CLIENT.get() {
                    return client.data_feed_subscribe(stream_name, subscription.clone()).await;
                }
            }
        }
        DataServerResponse::SubscribeResponse{ success: false, subscription: subscription.clone(), reason: Some(format!("Unable to find api client instance for: {}", subscription.symbol.data_vendor))}
    };

    timeout(TIMEOUT_DURATION, operation).await.unwrap_or_else(|_| DataServerResponse::SubscribeResponse {
        success: false,
        subscription: subscription.clone(),
        reason: Some("Operation timed out".to_string())
    })
}

/// return `DataServerResponse::UnSubscribeResponse` or `DataServerResponse::Error(FundForgeError)`
/// server or client error depending on who caused this problem
/// The caller does not await this method, but it lets the strategy know if the subscription was successful.
pub async fn data_feed_unsubscribe(
    data_vendor: DataVendor,
    stream_name: StreamName,
    subscription: DataSubscription
) -> DataServerResponse {
    let operation = async {
        match data_vendor {
            DataVendor::Rithmic => {
                let system = match get_rithmic_market_data_system() {
                    Some(system) => system,
                    None => return DataServerResponse::UnSubscribeResponse{ success: false, subscription: subscription.clone(), reason: Some(format!("Unable to find api client instance for: {}", data_vendor))}
                };
                if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                    return client.data_feed_unsubscribe(stream_name, subscription.clone()).await
                }
            },
            DataVendor::DataBento => {
                return match get_data_bento_client() {
                    Ok(client) => client.data_feed_unsubscribe(stream_name, subscription.clone()).await,
                    Err(e) => DataServerResponse::UnSubscribeResponse { success: false, subscription: subscription.clone(), reason: Some(format!("{}", e))}
                }
            }
            DataVendor::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.data_feed_unsubscribe(stream_name, subscription.clone()).await;
                }
            }
            DataVendor::Oanda => {
                if let Some(client) = OANDA_CLIENT.get() {
                    return client.data_feed_unsubscribe(stream_name, subscription.clone()).await;
                }
            }
        }
        DataServerResponse::UnSubscribeResponse{ success: false, subscription: subscription.clone(), reason: Some(format!("Unable to find api client instance for: {}", data_vendor))}
    };

    timeout(TIMEOUT_DURATION, operation).await.unwrap_or_else(|_| DataServerResponse::UnSubscribeResponse {
        success: false,
        subscription,
        reason: Some("Operation timed out".to_string())
    })
}

/// return `DataServerResponse::BaseData` or `DataServerResponse::Error(FundForgeError)`
/// server or client error depending on who caused this problem
pub async fn base_data_types_response(
    data_vendor: DataVendor,
    mode: StrategyMode,
    stream_name: StreamName,
    callback_id: u64
) -> DataServerResponse {
    let operation = async {
        match data_vendor {
            DataVendor::Rithmic => {
                let system = match get_rithmic_market_data_system() {
                    Some(system) => system,
                    None => return DataServerResponse::Error {error: FundForgeError::ServerErrorDebug("Rithmic market data system not found".to_string()), callback_id}
                };
                if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                    return client.base_data_types_response(mode, stream_name, callback_id).await
                }
            },
            DataVendor::DataBento => {
                return match get_data_bento_client() {
                    Ok(client) => client.base_data_types_response(mode, stream_name, callback_id).await,
                    Err(e) => DataServerResponse::Error { error: e, callback_id }
                }
            },
            DataVendor::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.base_data_types_response(mode, stream_name, callback_id).await;
                }
            }
            DataVendor::Oanda => {
                if let Some(client) = OANDA_CLIENT.get() {
                    return client.base_data_types_response(mode, stream_name, callback_id).await;
                }
            }
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", data_vendor))}
    };

    timeout(TIMEOUT_DURATION, operation).await.unwrap_or_else(|_| DataServerResponse::Error { callback_id, error: FundForgeError::ServerErrorDebug("Operation timed out".to_string()) })
}

/// This command doesn't require a response,
/// it is sent when a connection is dropped so that we can remove any items associated with the stream
/// (strategy that is connected to this port)
pub async fn logout_command_vendors(data_vendor: DataVendor, stream_name: StreamName) {
    let operation = async {
        match data_vendor {
            DataVendor::Rithmic => {
                let system = match get_rithmic_market_data_system() {
                    Some(system) => system,
                    None => return
                };
                if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                    client.logout_command_vendors(stream_name).await
                }
            },
            DataVendor::DataBento => {
                if let Ok(client) = get_data_bento_client() {
                    client.logout_command_vendors(stream_name).await;
                }
            },
            DataVendor::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    client.logout_command_vendors(stream_name).await;
                }
            }
            DataVendor::Oanda => {
                if let Some(client) = OANDA_CLIENT.get() {
                    client.logout_command_vendors(stream_name).await;
                }
            }
        }
    };

    match timeout(TIMEOUT_DURATION, operation).await {
        Ok(_) => {},
        Err(_) => eprintln!("Logout command for {} timed out after {} seconds", stream_name, TIMEOUT_DURATION.as_secs()),
    }
}
