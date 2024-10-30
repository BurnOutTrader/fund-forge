use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// The dynamically calculated state of a client’s Account.
/// Implemented by: AccountSummary, AccountChangesState
#[derive(Debug, Serialize, Deserialize)]
struct CalculatedAccountState {
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

    /// The Account's margin closeout percentage. When this value is 1.0 or above, the
    /// Account is in a margin closeout situation.
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

    /// The Account's margin call percentage. When this value is 1.0 or above, the Account
    /// is in a margin call situation.
    #[serde(rename = "marginCallPercent")]
    margin_call_percent: Decimal,
}
