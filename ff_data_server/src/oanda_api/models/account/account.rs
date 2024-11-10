use serde::{Deserialize};
use serde_json::Value;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use crate::oanda_api::models::position::OandaPosition;
use crate::oanda_api::models::trade::TradeSummary;

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