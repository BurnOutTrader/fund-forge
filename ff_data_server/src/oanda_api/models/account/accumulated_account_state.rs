use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use crate::oanda_api::models::primitives::DateTime;

/// The mutable state of a client's Account.
/// Implemented by: AccountSummary, AccountChangesState
#[derive(Debug, Serialize, Deserialize)]
struct AccumulatedAccountState {
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

    /// The total amount of dividend adjustment paid over the lifetime of the Account
    /// in the Account's home currency.
    #[serde(rename = "dividendAdjustment")]
    dividend_adjustment: Decimal,

    /// The total amount of fees charged over the lifetime of the Account for the execution
    /// of guaranteed Stop Loss Orders.
    #[serde(rename = "guaranteedExecutionFees")]
    guaranteed_execution_fees: Decimal,

    /// The date/time when the Account entered a margin call state. Only provided if the
    /// Account is in a margin call.
    #[serde(rename = "marginCallEnterTime")]
    margin_call_enter_time: Option<DateTime>,

    /// The number of times that the Account's current margin call was extended.
    #[serde(rename = "marginCallExtensionCount")]
    margin_call_extension_count: i32,

    /// The date/time of the Account's last margin call extension.
    #[serde(rename = "lastMarginCallExtensionTime")]
    last_margin_call_extension_time: Option<DateTime>,
}