use std::str::FromStr;
use chrono::{DateTime, Utc};
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use crate::server_features::server_side_brokerage::BrokerApiResponse;
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::enums::{OrderSide, StrategyMode};
use ff_standard_lib::standardized_types::new_types::{TimeString};
use ff_standard_lib::standardized_types::orders::{Order, OrderId, OrderUpdateEvent, OrderUpdateType};
use ff_standard_lib::standardized_types::subscriptions::SymbolName;
use ff_standard_lib::standardized_types::accounts::{Account, AccountId, Currency};
use ff_standard_lib::StreamName;
use crate::bitget_api::api_client::BITGET_CLIENT;
use crate::rithmic_api::api_client::{get_rithmic_client, RITHMIC_CLIENTS};
use crate::test_api::api_client::TEST_CLIENT;
use tokio::time::{timeout, Duration};
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::orders::OrderUpdateEvent::OrderUpdateRejected;
use crate::oanda_api::api_client::{get_oanda_client, OANDA_CLIENT};
use crate::server_features::database::DATA_STORAGE;

pub const TIMEOUT_DURATION: Duration = Duration::from_secs(10);

#[allow(unused)]
/// This request is only sent to Oanda server
pub async fn exchange_rate_response(mode: StrategyMode, from_currency: Currency, to_currency: Currency, time_string: TimeString, data_vendor: DataVendor, side: OrderSide, callback_id: u64) -> DataServerResponse {
    //todo, if live mode we request the api directly for the latest rate.
    //todo get exchange rate from hybrid storage,
    let time = match DateTime::<Utc>::from_str(&time_string) {
        Ok(time) => time,
        Err(_) => return DataServerResponse::Error {
            callback_id,
            error: FundForgeError::ServerErrorDebug("Invalid time string".to_string())
        }
    };
    match DATA_STORAGE.get().unwrap().get_exchange_rate(from_currency, to_currency, time, data_vendor, side).await {
        Ok(rate) => DataServerResponse::ExchangeRate {
            callback_id,
            rate,
        },
        Err(e) => DataServerResponse::Error {
            callback_id,
            error: FundForgeError::ServerErrorDebug(format!("Exchange rate not found: {}", e))
        }
    }
}

/// return `DataServerResponse::CommissionInfo` or `DataServerResponse::Error(FundForgeError)`.
pub async fn commission_info_response(mode: StrategyMode, brokerage: Brokerage, symbol_name: SymbolName, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
    let operation = async {
        match brokerage {
            Brokerage::Rithmic(system) => {
                if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                    return client.value().commission_info_response(mode, stream_name, symbol_name, callback_id).await
                }
            },
            Brokerage::Test => return TEST_CLIENT.commission_info_response(mode, stream_name, symbol_name, callback_id).await,
            Brokerage::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.commission_info_response(mode, stream_name, symbol_name, callback_id).await
                }
            }
            Brokerage::Oanda => if let Some(client) = OANDA_CLIENT.get() {
                return client.commission_info_response(mode, stream_name, symbol_name, callback_id).await
            }
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", brokerage))}
    };

    timeout(TIMEOUT_DURATION, operation).await.unwrap_or_else(|_| DataServerResponse::Error { callback_id, error: FundForgeError::ServerErrorDebug("Operation timed out".to_string()) })
}

/// return `DataServerResponse::Symbols` or `DataServerResponse::Error(FundForgeError)`.
/// server or client error depending on who caused this problem
pub async fn symbol_names_response(
    brokerage: Brokerage,
    mode: StrategyMode,
    stream_name: StreamName,
    time: Option<DateTime<Utc>>,
    callback_id: u64
) -> DataServerResponse {
    let operation = async {
        match brokerage {
            Brokerage::Rithmic(system) => {
                if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                    return client.value().symbol_names_response(mode, time, stream_name, callback_id).await
                }
            },
            Brokerage::Test => return TEST_CLIENT.symbol_names_response(mode, time, stream_name, callback_id).await,
            Brokerage::Bitget => {
                if let Some(client) = BITGET_CLIENT.get() {
                    return client.symbol_names_response(mode, time, stream_name, callback_id).await
                }
            }
            Brokerage::Oanda => if let Some(client) = OANDA_CLIENT.get() {
                return client.symbol_names_response(mode, time, stream_name, callback_id).await
            }
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", brokerage))}
    };

    timeout(TIMEOUT_DURATION, operation).await.unwrap_or_else(|_| DataServerResponse::Error { callback_id, error: FundForgeError::ServerErrorDebug("Operation timed out".to_string()) })
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
    let operation = async {
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
            Brokerage::Oanda => if let Some(client) = OANDA_CLIENT.get() {
                return client.account_info_response(mode, stream_name, account_id, callback_id).await
            }
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", brokerage))}
    };

    timeout(TIMEOUT_DURATION, operation).await.unwrap_or_else(|_| DataServerResponse::Error { callback_id, error: FundForgeError::ServerErrorDebug("Operation timed out".to_string()) })
}

pub async fn symbol_info_response(brokerage: Brokerage, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
    let operation = async {
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
            Brokerage::Test => return TEST_CLIENT.symbol_info_response(mode, stream_name, symbol_name, callback_id).await,
            Brokerage::Oanda => if let Some(client) = get_oanda_client() {
                return client.symbol_info_response(mode, stream_name, symbol_name, callback_id).await
            }
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", brokerage))}
    };

    timeout(TIMEOUT_DURATION, operation).await.unwrap_or_else(|_| DataServerResponse::Error { callback_id, error: FundForgeError::ServerErrorDebug("Operation timed out".to_string()) })
}

/// return `DataServerResponse::Accounts or DataServerResponse::Error(FundForgeError)`
/// server or client error depending on who caused this problem
pub async fn accounts_response(brokerage: Brokerage, mode: StrategyMode, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
    let operation = async {
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
            Brokerage::Test => return TEST_CLIENT.accounts_response(mode, stream_name, callback_id).await,
            Brokerage::Oanda => if let Some(client) = get_oanda_client() {
                return client.accounts_response(mode, stream_name, callback_id).await
            },
        }
        DataServerResponse::Error{ callback_id, error: FundForgeError::ServerErrorDebug(format!("Unable to find api client instance for: {}", brokerage))}
    };

    timeout(TIMEOUT_DURATION, operation).await.unwrap_or_else(|_| DataServerResponse::Error { callback_id, error: FundForgeError::ServerErrorDebug("Operation timed out".to_string()) })
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
        Brokerage::Test => TEST_CLIENT.logout_command(stream_name).await,
        Brokerage::Oanda => if let Some(client) = OANDA_CLIENT.get() {
            client.logout_command(stream_name).await
        },
    }
}

fn create_order_rejected(order: &Order, reason: String) -> OrderUpdateEvent {
    OrderUpdateEvent::OrderRejected {
        account: order.account.clone(),
        order_id: order.id.clone(),
        symbol_name: order.symbol_name.clone(),
        symbol_code: "".to_string(),
        reason,
        tag: order.tag.clone(),
        time: Utc::now().to_string(),
    }
}

pub async fn live_market_order(stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {

    let operation = async {
        match order.account.brokerage {
            Brokerage::Test => {
                Err(create_order_rejected(&order, "Test Brokerage Can Not Place Live Orders".to_string()))
            }
            Brokerage::Rithmic(system) => {
                RITHMIC_CLIENTS.get(&system)
                    .ok_or_else(|| create_order_rejected(&order, format!("Client Not found for Rithmic system: {}", system)))?
                    .live_market_order(stream_name, mode, order.clone())
                    .await
            }
            Brokerage::Bitget => {
                BITGET_CLIENT.get()
                    .ok_or_else(|| create_order_rejected(&order, "Bitget client not found".to_string()))?
                    .live_market_order(stream_name, mode, order.clone())
                    .await
            }
            Brokerage::Oanda => OANDA_CLIENT.get()
                .ok_or_else(|| create_order_rejected(&order, "Oanda client not found".to_string()))?
                .live_market_order(stream_name, mode, order.clone())
                .await
        }
    };

    match timeout(TIMEOUT_DURATION, operation).await {
        Ok(result) => result,
        Err(_) => Err(create_order_rejected(&order, "Operation timed out".to_string()))
    }
}

pub async fn live_enter_long(stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
    let operation = async {
        match order.account.brokerage {
            Brokerage::Test => {
                Err(create_order_rejected(&order, "Test Brokerage Can Not Place Live Orders".to_string()))
            }
            Brokerage::Rithmic(system) => {
                RITHMIC_CLIENTS.get(&system)
                    .ok_or_else(|| create_order_rejected(&order, format!("Client Not found for Rithmic system: {}", system)))?
                    .live_enter_long(stream_name, mode, order.clone())
                    .await
            }
            Brokerage::Bitget => {
                BITGET_CLIENT.get()
                    .ok_or_else(|| create_order_rejected(&order, "Bitget client not found".to_string()))?
                    .live_enter_long(stream_name, mode, order.clone())
                    .await
            }
            Brokerage::Oanda => OANDA_CLIENT.get()
                .ok_or_else(|| create_order_rejected(&order, "Oanda client not found".to_string()))?
                .live_enter_long(stream_name, mode, order.clone())
                .await
        }
    };

    match timeout(TIMEOUT_DURATION, operation).await {
        Ok(result) => result,
        Err(_) => Err(create_order_rejected(&order, "Operation timed out".to_string()))
    }
}

pub async fn live_enter_short(stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
    let operation = async {
        match order.account.brokerage {
            Brokerage::Test => {
                Err(create_order_rejected(&order, "Test Brokerage Can Not Place Live Orders".to_string()))
            }
            Brokerage::Rithmic(system) => {
                RITHMIC_CLIENTS.get(&system)
                    .ok_or_else(|| create_order_rejected(&order, format!("Client Not found for Rithmic system: {}", system)))?
                    .live_enter_short(stream_name, mode, order.clone())
                    .await
            }
            Brokerage::Bitget => {
                BITGET_CLIENT.get()
                    .ok_or_else(|| create_order_rejected(&order, "Bitget client not found".to_string()))?
                    .live_enter_short(stream_name, mode, order.clone())
                    .await
            }
            Brokerage::Oanda => OANDA_CLIENT.get()
                .ok_or_else(|| create_order_rejected(&order, "Oanda client not found".to_string()))?
                .live_enter_short(stream_name, mode, order.clone())
                .await

        }
    };

    match timeout(TIMEOUT_DURATION, operation).await {
        Ok(result) => result,
        Err(_) => Err(create_order_rejected(&order, "Operation timed out".to_string()))
    }
}

pub async fn live_exit_short(stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
    let operation = async {
        match order.account.brokerage {
            Brokerage::Test => {
                Err(create_order_rejected(&order, "Test Brokerage Can Not Place Live Orders".to_string()))
            }
            Brokerage::Rithmic(system) => {
                RITHMIC_CLIENTS.get(&system)
                    .ok_or_else(|| create_order_rejected(&order, format!("Client Not found for Rithmic system: {}", system)))?
                    .live_exit_short(stream_name, mode, order.clone())
                    .await
            }
            Brokerage::Bitget => {
                BITGET_CLIENT.get()
                    .ok_or_else(|| create_order_rejected(&order, "Bitget client not found".to_string()))?
                    .live_exit_short(stream_name, mode, order.clone())
                    .await
            }
            Brokerage::Oanda => OANDA_CLIENT.get()
                .ok_or_else(|| create_order_rejected(&order, "Bitget client not found".to_string()))?
                .live_exit_short(stream_name, mode, order.clone())
                .await
        }
    };

    match timeout(TIMEOUT_DURATION, operation).await {
        Ok(result) => result,
        Err(_) => Err(create_order_rejected(&order, "Operation timed out".to_string()))
    }
}


pub async fn live_exit_long(stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
    let operation = async {
        match order.account.brokerage {
            Brokerage::Test => {
                Err(create_order_rejected(&order, "Test Brokerage Can Not Place Live Orders".to_string()))
            }
            Brokerage::Rithmic(system) => {
                RITHMIC_CLIENTS.get(&system)
                    .ok_or_else(|| create_order_rejected(&order, format!("Client Not found for Rithmic system: {}", system)))?
                    .live_exit_long(stream_name, mode, order.clone())
                    .await
            }
            Brokerage::Bitget => {
                BITGET_CLIENT.get()
                    .ok_or_else(|| create_order_rejected(&order, "Bitget client not found".to_string()))?
                    .live_exit_long(stream_name, mode, order.clone())
                    .await
            }
            Brokerage::Oanda => OANDA_CLIENT.get()
                .ok_or_else(|| create_order_rejected(&order, "Bitget client not found".to_string()))?
                .live_exit_long(stream_name, mode, order.clone())
                .await
        }
    };

    match timeout(TIMEOUT_DURATION, operation).await {
        Ok(result) => result,
        Err(_) => Err(create_order_rejected(&order, "Operation timed out".to_string()))
    }
}

pub async fn other_orders(stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
    let operation = async {
        match order.account.brokerage {
            Brokerage::Test => {
                Err(create_order_rejected(&order, "Test Brokerage Can Not Place Live Orders".to_string()))
            }
            Brokerage::Rithmic(system) => {
                RITHMIC_CLIENTS.get(&system)
                    .ok_or_else(|| create_order_rejected(&order, format!("Client Not found for Rithmic system: {}", system)))?
                    .other_orders(stream_name, mode, order.clone())
                    .await
            }
            Brokerage::Bitget => {
                BITGET_CLIENT.get()
                    .ok_or_else(|| create_order_rejected(&order, "Bitget client not found".to_string()))?
                    .other_orders(stream_name, mode, order.clone())
                    .await
            }
            Brokerage::Oanda => OANDA_CLIENT.get()
                .ok_or_else(|| create_order_rejected(&order, "Bitget client not found".to_string()))?
                .other_orders(stream_name, mode, order.clone())
                .await
        }
    };

    match timeout(TIMEOUT_DURATION, operation).await {
        Ok(result) => result,
        Err(_) => Err(create_order_rejected(&order, "Operation timed out".to_string()))
    }
}

pub async fn cancel_order(account: Account, order_id: OrderId) {
    match account.brokerage {
        Brokerage::Test => {}
        Brokerage::Rithmic(system) => {
           if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                client.cancel_order(account, order_id).await;
           }
        }
        Brokerage::Bitget => {
            if let Some(client) = BITGET_CLIENT.get() {
                client.cancel_order(account, order_id).await;
            }
        }
        Brokerage::Oanda => {
            if let Some(client) = OANDA_CLIENT.get() {
                client.cancel_order(account, order_id).await;
            }
        }
    }
}

pub async fn cancel_orders_on_account(account: Account) {
    match account.brokerage {
        Brokerage::Test => {}
        Brokerage::Rithmic(system) => {
            if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                client.cancel_orders_on_account(account).await;
            }
        }
        Brokerage::Bitget => {
            if let Some(client) = BITGET_CLIENT.get() {
                client.cancel_orders_on_account(account).await;
            }
        }
        Brokerage::Oanda => {
            if let Some(client) = OANDA_CLIENT.get() {
                client.cancel_orders_on_account(account).await;
            }
        }
    }
}

pub async fn flatten_all_for(account: Account) {
    match account.brokerage {
        Brokerage::Test => {}
        Brokerage::Rithmic(system) => {
            if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                client.flatten_all_for(account).await;
            }
        }
        Brokerage::Bitget => {
            if let Some(client) = BITGET_CLIENT.get() {
                client.flatten_all_for(account).await;
            }
        }
        Brokerage::Oanda => {
            if let Some(client) = OANDA_CLIENT.get() {
                client.flatten_all_for(account).await;
            }
        }
    }
}

pub async fn update_order(account: Account, order_id: OrderId, update: OrderUpdateType) -> Result<(), OrderUpdateEvent> {
    match account.brokerage {
        Brokerage::Test => return Err(OrderUpdateRejected {
            account,
            order_id,
            reason: "Test Brokerage Can Not Modify Live Orders".to_string(),
            time: Utc::now().to_string(),
        }),
        Brokerage::Rithmic(system) => {
            if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                return client.update_order(account, order_id, update).await;
            }
        }
        Brokerage::Bitget => {
            if let Some(client) = BITGET_CLIENT.get() {
                return client.update_order(account, order_id, update).await;
            }
        }
        Brokerage::Oanda => {
            if let Some(client) = OANDA_CLIENT.get() {
                return client.update_order(account, order_id, update).await;
            }
        }
    }
    Err(OrderUpdateRejected {
        account: account.clone(),
        order_id,
        reason: format!("No Client Found For: {}", account),
        time: Utc::now().to_string(),
    })
}

