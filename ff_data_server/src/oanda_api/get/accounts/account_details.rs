use ff_standard_lib::messages::data_server_messaging::FundForgeError;
use serde_derive::Deserialize;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::Value;
use crate::oanda_api::api_client::OandaClient;
use crate::oanda_api::get::positions::OandaPosition;
use crate::oanda_api::models::trade::TradeSummary;

pub(crate) async fn get_oanda_account_details(oanda_client: &OandaClient, account_id: &str) -> Result<OandaAccount, FundForgeError> {
    let request_uri = format!("/accounts/{}", account_id);

    let response = match oanda_client.send_rest_request(&request_uri).await {
        Ok(response) => response,
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(
                format!("Failed to get_requests account summary from server: {:?}", e)
            ));
        }
    };

    if !response.status().is_success() {
        return Err(FundForgeError::ServerErrorDebug(
            format!("Server returned error status: {}", response.status())
        ));
    }

    let content = match response.text().await {
        Ok(content) => content,
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(
                format!("Failed to read response content: {:?}", e)
            ));
        }
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(json) => json,
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(
                format!("Failed to parse JSON response: {:?}", e)
            ));
        }
    };

    //eprintln!("Account JSON: {}", json["account"]);

    match serde_json::from_value(json["account"].clone()) {
        Ok(summary) => Ok(summary),
        Err(e) => Err(FundForgeError::ServerErrorDebug(
            format!("Failed to deserialize account summary: {:?}", e)
        ))
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct OandaAccount {
    pub id: String,
    pub alias: Option<String>,
    pub currency: String,
    #[serde(rename = "createdByUserID")]
    pub created_by_user_id: i64,
    #[serde(rename = "createdTime")]
    pub created_time: DateTime<Utc>,
    #[serde(rename = "marginRate")]
    pub margin_rate: Decimal,
    #[serde(rename = "openTradeCount")]
    pub open_trade_count: i32,
    #[serde(rename = "openPositionCount")]
    pub open_position_count: i32,
    #[serde(rename = "pendingOrderCount")]
    pub pending_order_count: i32,
    #[serde(rename = "hedgingEnabled")]
    pub hedging_enabled: bool,
    #[serde(rename = "unrealizedPL")]
    pub unrealized_pl: Decimal,
    #[serde(rename = "NAV")]  // Changed from nav to NAV
    pub nav: Decimal,
    #[serde(rename = "marginUsed")]
    pub margin_used: Decimal,
    #[serde(rename = "marginAvailable")]
    pub margin_available: Decimal,
    #[serde(rename = "positionValue")]
    pub position_value: Decimal,
    #[serde(rename = "marginCloseoutUnrealizedPL")]
    pub margin_closeout_unrealized_pl: Decimal,
    #[serde(rename = "marginCloseoutNAV")]
    pub margin_closeout_nav: Decimal,
    #[serde(rename = "marginCloseoutMarginUsed")]
    pub margin_closeout_margin_used: Decimal,
    #[serde(rename = "marginCloseoutPercent")]
    pub margin_closeout_percent: Decimal,
    #[serde(rename = "marginCloseoutPositionValue")]
    pub margin_closeout_position_value: Decimal,
    #[serde(rename = "withdrawalLimit")]
    pub withdrawal_limit: Decimal,
    #[serde(rename = "marginCallMarginUsed")]
    pub margin_call_margin_used: Decimal,
    #[serde(rename = "marginCallPercent")]
    pub margin_call_percent: Decimal,
    pub balance: Decimal,
    pub pl: Decimal,
    #[serde(rename = "resettablePL")]
    pub resettable_pl: Decimal,
    pub financing: Decimal,
    pub commission: Decimal,
    #[serde(rename = "dividendAdjustment")]
    pub dividend_adjustment: Decimal,
    #[serde(rename = "guaranteedExecutionFees")]
    pub guaranteed_execution_fees: Decimal,
    #[serde(rename = "lastTransactionID")]
    pub last_transaction_id: String,
    #[serde(default)]
    pub trades: Vec<TradeSummary>,
    #[serde(default)]
    pub positions: Vec<OandaPosition>,
    #[serde(default)]
    pub orders: Vec<Value>,
}