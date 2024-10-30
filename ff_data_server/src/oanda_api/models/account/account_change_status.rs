use rust_decimal::Decimal;
use serde::Deserialize;
use serde::Serialize;
use crate::oanda_api::models::order::order_related::DynamicOrderState;
use crate::oanda_api::models::position::CalculatedPositionState;
use crate::oanda_api::models::primitives::DateTime;
use crate::oanda_api::models::trade::CalculatedTradeState;

/// Represents the prices-dependent state changes of an Account.
#[derive(Debug, Serialize, Deserialize)]
struct AccountChangesState {
    /// The total unrealized profit/loss for all Trades currently open in the Account.
    #[serde(rename = "unrealizedPL")]
    unrealized_pl: Decimal,

    /// The net asset value of the Account. Equal to Account balance + unrealizedPL.
    #[serde(rename = "NAV")]
    nav: Decimal,

    /// Margin currently used for the Account.
    #[serde(rename = "marginUsed")]
    margin_used: Decimal,

    /// Margin available for Account currency.
    #[serde(rename = "marginAvailable")]
    margin_available: Decimal,

    /// The value of the Account's open positions represented in the Account's home currency.
    #[serde(rename = "positionValue")]
    position_value: Decimal,

    /// The Account's margin closeout unrealized PL.
    #[serde(rename = "marginCloseoutUnrealizedPL")]
    margin_closeout_unrealized_pl: Decimal,

    /// The Account's margin closeout NAV.
    #[serde(rename = "marginCloseoutNAV")]
    margin_closeout_nav: Decimal,

    /// The Account's margin closeout margin used.
    #[serde(rename = "marginCloseoutMarginUsed")]
    margin_closeout_margin_used: Decimal,

    /// The Account's margin closeout percentage.
    /// When this value is 1.0 or above the Account is in a margin closeout situation.
    #[serde(rename = "marginCloseoutPercent")]
    margin_closeout_percent: Decimal,

    /// The value of the Account's open positions as used for margin closeout calculations
    /// represented in the Account's home currency.
    #[serde(rename = "marginCloseoutPositionValue")]
    margin_closeout_position_value: Decimal,

    /// The current WithdrawalLimit for the account which will be zero or a positive value
    /// indicating how much can be withdrawn from the account.
    #[serde(rename = "withdrawalLimit")]
    withdrawal_limit: Decimal,

    /// The Account's margin call margin used.
    #[serde(rename = "marginCallMarginUsed")]
    margin_call_margin_used: Decimal,

    /// The Account's margin call percentage.
    /// When this value is 1.0 or above the Account is in a margin call situation.
    #[serde(rename = "marginCallPercent")]
    margin_call_percent: Decimal,

    /// The current balance of the account.
    #[serde(rename = "balance")]
    balance: Decimal,

    /// The total profit/loss realized over the lifetime of the Account.
    #[serde(rename = "pl")]
    pl: Decimal,

    /// The total realized profit/loss for the account since it was last reset by the client.
    #[serde(rename = "resettablePL")]
    resettable_pl: Decimal,

    /// The total amount of financing paid/collected over the lifetime of the account.
    #[serde(rename = "financing")]
    financing: Decimal,

    /// The total amount of commission paid over the lifetime of the Account.
    #[serde(rename = "commission")]
    commission: Decimal,

    /// The total amount of dividend adjustment paid over the lifetime of the Account in
    /// the Account's home currency.
    #[serde(rename = "dividendAdjustment")]
    dividend_adjustment: Decimal,

    /// The total amount of fees charged over the lifetime of the Account for the execution
    /// of guaranteed Stop Loss Orders.
    #[serde(rename = "guaranteedExecutionFees")]
    guaranteed_execution_fees: Decimal,

    /// The date/time when the Account entered a margin call state.
    /// Only provided if the Account is in a margin call.
    #[serde(rename = "marginCallEnterTime")]
    margin_call_enter_time: Option<DateTime>,

    /// The number of times that the Account's current margin call was extended.
    #[serde(rename = "marginCallExtensionCount")]
    margin_call_extension_count: i32,

    /// The date/time of the Account's last margin call extension.
    #[serde(rename = "lastMarginCallExtensionTime")]
    last_margin_call_extension_time: Option<DateTime>,

    /// The prices-dependent state of each pending Order in the Account.
    #[serde(rename = "orders")]
    orders: Vec<DynamicOrderState>,

    /// The prices-dependent state for each open Trade in the Account.
    #[serde(rename = "trades")]
    trades: Vec<CalculatedTradeState>,

    /// The prices-dependent state for each open Position in the Account.
    #[serde(rename = "positions")]
    positions: Vec<CalculatedPositionState>,
}
