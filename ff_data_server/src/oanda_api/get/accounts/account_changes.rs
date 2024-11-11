use std::sync::Arc;
use ff_standard_lib::messages::data_server_messaging::FundForgeError;
use serde_derive::Deserialize;
use serde_json::Value;
use rust_decimal::Decimal;
use crate::oanda_api::api_client::OandaClient;
use crate::oanda_api::get::positions::OandaPosition;

pub async fn get_account_changes(
    client: &Arc<OandaClient>,
    account_id: &str,
    last_transaction_id: &str,
) -> Result<AccountChangesResponse, FundForgeError> {
    let request_uri = format!(
        "/accounts/{}/changes?sinceTransactionID={}",
        account_id,
        last_transaction_id
    );

    let response = match client.send_rest_request(&request_uri).await {
        Ok(r) => r,
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(
                format!("Failed to poll account changes: {:?}", e)
            ));
        }
    };

    if !response.status().is_success() {
        return Err(FundForgeError::ServerErrorDebug(
            format!("Server returned error status: {}", response.status())
        ));
    }

    let content = response.text().await.map_err(|e| {
        FundForgeError::ServerErrorDebug(format!("Failed to read response content: {:?}", e))
    })?;

    let changes: AccountChangesResponse = serde_json::from_str(&content).map_err(|e| {
        FundForgeError::ServerErrorDebug(format!("Failed to parse JSON response: {:?}", e))
    })?;

    Ok(changes)
}

#[derive(Debug, Deserialize)]
pub struct AccountChangesResponse {
    pub(crate) changes: AccountChanges,
    #[serde(rename = "lastTransactionID")]
    pub(crate) last_transaction_id: String,
    pub(crate) state: AccountState,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AccountChanges {
    #[serde(rename = "ordersCancelled")]
    orders_cancelled: Vec<Value>,
    #[serde(rename = "ordersCreated")]
    orders_created: Vec<Value>,
    #[serde(rename = "ordersFilled")]
    orders_filled: Vec<Value>,
    #[serde(rename = "ordersTriggered")]
    orders_triggered: Vec<Value>,
    pub(crate) positions: Vec<OandaPosition>,
    #[serde(rename = "tradesClosed")]
    trades_closed: Vec<Value>,
    #[serde(rename = "tradesOpened")]
    trades_opened: Vec<Value>,
    #[serde(rename = "tradesReduced")]
    trades_reduced: Vec<Value>,
    transactions: Vec<Value>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AccountState {
    #[serde(rename = "NAV")]
    nav: Decimal,
    #[serde(rename = "marginAvailable")]
    pub(crate) margin_available: Decimal,
    #[serde(rename = "marginUsed")]
    pub(crate) margin_used: Decimal,
    #[serde(rename = "positionValue")]
    position_value: Decimal,
    #[serde(rename = "marginCloseoutPercent")]
    margin_closeout_percent: Decimal,
    #[serde(rename = "marginCloseoutUnrealizedPL")]
    margin_closeout_unrealized_pl: Decimal,
    #[serde(rename = "unrealizedPL")]
    pub(crate) unrealized_pl: Decimal,
    #[serde(rename = "withdrawalLimit")]
    withdrawal_limit: Decimal,
}