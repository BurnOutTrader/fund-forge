use rust_decimal::Decimal;
use serde::Deserialize;
use serde::Serialize;
use ff_standard_lib::standardized_types::accounts::{AccountId, Currency};
use crate::oanda_api::models::account::enums::{GuaranteedStopLossOrderMode, GuaranteedStopLossOrderMutability};
use crate::oanda_api::models::account::guaranteed_stop_loss_parameters::GuaranteedStopLossOrderParameters;
use crate::oanda_api::models::primitives::DateTime;
use crate::oanda_api::models::transaction_related::TransactionID;

/// A summary representation of a client's Account.
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountSummary {
    /// The Account's identifier.
    #[serde(rename = "id")]
    id: AccountId,

    /// Client-assigned alias for the Account. Only provided if the Account has an alias set.
    #[serde(rename = "alias")]
    alias: String,

    /// The home currency of the Account.
    #[serde(rename = "currency")]
    currency: Currency,

    /// ID of the user that created the Account.
    #[serde(rename = "createdByUserID")]
    created_by_user_id: i64,

    /// The date/time when the Account was created.
    #[serde(rename = "createdTime")]
    created_time: DateTime,

    /// The current guaranteed Stop Loss Order settings of the Account.
    /// This field will only be present if the guaranteedStopLossOrderMode is not 'DISABLED'.
    #[serde(rename = "guaranteedStopLossOrderParameters")]
    guaranteed_stop_loss_order_parameters: GuaranteedStopLossOrderParameters,

    /// The current guaranteed Stop Loss Order mode of the Account.
    #[serde(rename = "guaranteedStopLossOrderMode")]
    guaranteed_stop_loss_order_mode: GuaranteedStopLossOrderMode,

    /// The current guaranteed Stop Loss Order mutability setting of the Account.
    /// This field will only be present if the guaranteedStopLossOrderMode is not 'DISABLED'.
    #[serde(rename = "guaranteedStopLossOrderMutability")]
    guaranteed_stop_loss_order_mutability: Option<GuaranteedStopLossOrderMutability>,

    /// The date/time that the Account's resettablePL was last reset.
    #[serde(rename = "resettablePLTime")]
    resettable_pl_time: DateTime,

    /// Client-provided margin rate override for the Account.
    /// The effective margin rate of the Account is the lesser of this value and the OANDA
    /// margin rate for the Account's division. This value is only provided if a margin rate
    /// override exists for the Account.
    #[serde(rename = "marginRate")]
    margin_rate: Option<Decimal>,

    /// The number of Trades currently open in the Account.
    #[serde(rename = "openTradeCount")]
    open_trade_count: i32,

    /// The number of Positions currently open in the Account.
    #[serde(rename = "openPositionCount")]
    open_position_count: i32,

    /// The number of Orders currently pending in the Account.
    #[serde(rename = "pendingOrderCount")]
    pending_order_count: i32,

    /// Flag indicating that the Account has hedging enabled.
    #[serde(rename = "hedgingEnabled")]
    hedging_enabled: bool,

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
    /// When this value is 1.0 or above, the Account is in a margin closeout situation.
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
    /// When this value is 1.0 or above, the Account is in a margin call situation.
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

    /// The ID of the last Transaction created for the Account.
    #[serde(rename = "lastTransactionID")]
    last_transaction_id: TransactionID,
}
