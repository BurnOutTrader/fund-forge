use serde::{Deserialize, Serialize};

/// The financing mode of an Account.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountFinancingMode {
    /// No financing is paid/charged for open Trades in the Account.
    NoFinancing,

    /// Second-by-second financing is paid/charged for open Trades in the Account,
    /// both daily and when the Trade is closed.
    SecondBySecond,

    /// Daily financing is paid/charged for open Trades in the Account.
    Daily,
}

/// The overall behavior of the Account regarding guaranteed Stop Loss Orders.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GuaranteedStopLossOrderMode {
    /// The Account is not permitted to create guaranteed Stop Loss Orders.
    Disabled,
    /// The Account is able, but not required to have guaranteed Stop Loss Orders for open Trades.
    Allowed,
    /// The Account is required to have guaranteed Stop Loss Orders for all open Trades.
    Required,
}

/// For Accounts that support guaranteed Stop Loss Orders, describes the actions that can be performed on guaranteed Stop Loss Orders.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum GuaranteedStopLossOrderMutability {
    /// Once a guaranteed Stop Loss Order has been created it cannot be replaced or cancelled.
    Fixed,
    /// An existing guaranteed Stop Loss Order can only be replaced, not cancelled.
    Replaceable,
    /// Once a guaranteed Stop Loss Order has been created it can be either replaced or cancelled.
    Cancelable,
    /// An existing guaranteed Stop Loss Order can only be replaced to widen the gap from the current prices, not cancelled.
    PriceWidenOnly,
}

/// The way that position values for an Account are calculated and aggregated.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum PositionAggregationMode {
    /// The Position value or margin for each side (long and short) of the Position are computed independently and added together.
    AbsoluteSum,

    /// The Position value or margin for each side (long and short) of the Position are computed independently. The Position value or margin chosen is the maximal absolute value of the two.
    MaximalSide,

    /// The Position value or margin is computed as the net sum of the long and short sides.
    NetSum,
}

