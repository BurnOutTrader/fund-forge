use rust_decimal::Decimal;
use serde::{Serialize, Deserialize};
use crate::oanda_api::models::primitives::{InstrumentName};
use crate::oanda_api::models::trade::TradeID;
use crate::oanda_api::models::transaction_related::TransactionID;

/// A filter that can be used when fetching Transactions.
#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionFilter {
    /// Type of Transactions to filter.
    #[serde(rename = "type")]
    pub type_of: String,

    /// The ID of the most recent Transaction created for the Account.
    #[serde(rename = "lastTransactionID")]
    pub last_transaction_id: TransactionID,

    /// The date/time when the TransactionHeartbeat was created.
    pub time: String,
}

/// A TransactionHeartbeat object is injected into the Transaction stream to ensure that the HTTP connection remains active.
#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionHeartbeat {
    /// The string “HEARTBEAT”.
    #[serde(rename = "type")]
    pub type_of: String,

    /// The ID of the most recent Transaction created for the Account.
    #[serde(rename = "lastTransactionID")]
    pub last_transaction_id: TransactionID,

    /// The date/time when the TransactionHeartbeat was created.
    pub time: String,
}

/// The specification of a Position within an Account.
#[derive(Serialize, Deserialize, Debug)]
pub struct OandaPosition {
    /// The Position’s Instrument.
    pub instrument: InstrumentName,

    /// Profit/loss realized by the Position over the lifetime of the Account.
    pub pl: Decimal,

    /// The unrealized profit/loss of all open Trades that contribute to this Position.
    #[serde(rename = "unrealizedPL")]
    pub unrealized_pl: Decimal,

    /// Margin currently used by the Position.
    #[serde(rename = "marginUsed")]
    pub margin_used: Decimal,

    /// Profit/loss realized by the Position since the Account’s resettablePL was last reset by the client.
    #[serde(rename = "resettablePL")]
    pub resettable_pl: Decimal,

    /// The total amount of financing paid/collected for this instrument over the lifetime of the Account.
    pub financing: Decimal,

    /// The total amount of commission paid for this instrument over the lifetime of the Account.
    pub commission: Decimal,

    /// The total amount of dividend adjustment paid for this instrument over the lifetime of the Account.
    #[serde(rename = "dividendAdjustment")]
    pub dividend_adjustment: Decimal,

    /// The total amount of fees charged over the lifetime of the Account for the execution of guaranteed Stop Loss Orders for this instrument.
    #[serde(rename = "guaranteedExecutionFees")]
    pub guaranteed_execution_fees: Decimal,

    /// The details of the long side of the Position.
    pub long: PositionSide,

    /// The details of the short side of the Position.
    pub short: PositionSide,
}

/// The representation of a Position for a single direction (long or short).
#[derive(Serialize, Deserialize, Debug)]
pub struct PositionSide {
    /// Number of units in the position (negative value indicates a short position, positive indicates a long position).
    pub units: Decimal,

    /// Volume-weighted average of the underlying Trade open prices for the PositionSide.
    #[serde(rename = "averagePrice")]
    pub average_price: Decimal,

    /// List of the open Trade IDs which contribute to the open PositionSide.
    #[serde(rename = "tradeIDs")]
    pub trade_ids: Vec<TradeID>,

    /// Profit/loss realized by the PositionSide over the lifetime of the Account.
    pub pl: Decimal,

    /// The unrealized profit/loss of all open Trades that contribute to this PositionSide.
    #[serde(rename = "unrealizedPL")]
    pub unrealized_pl: Decimal,

    /// Profit/loss realized by the PositionSide since the Account’s resettablePL was last reset by the client.
    #[serde(rename = "resettablePL")]
    pub resettable_pl: Decimal,

    /// The total amount of financing paid/collected for this PositionSide over the lifetime of the Account.
    pub financing: Decimal,

    /// The total amount of dividend adjustment paid for the PositionSide over the lifetime of the Account.
    #[serde(rename = "dividendAdjustment")]
    pub dividend_adjustment: Decimal,

    /// The total amount of fees charged over the lifetime of the Account for the execution of guaranteed Stop Loss Orders attached to Trades for this PositionSide.
    #[serde(rename = "guaranteedExecutionFees")]
    pub guaranteed_execution_fees: Decimal,
}

/// The dynamic (calculated) state of a Position.
#[derive(Serialize, Deserialize, Debug)]
pub struct CalculatedPositionState {
    /// The Position’s Instrument.
    pub instrument: InstrumentName,

    /// The Position’s net unrealized profit/loss.
    #[serde(rename = "netUnrealizedPL")]
    pub net_unrealized_pl: Decimal,

    /// The unrealized profit/loss of the Position’s long open Trades.
    #[serde(rename = "longUnrealizedPL")]
    pub long_unrealized_pl: Decimal,

    /// The unrealized profit/loss of the Position’s short open Trades.
    #[serde(rename = "shortUnrealizedPL")]
    pub short_unrealized_pl: Decimal,

    /// Margin currently used by the Position.
    #[serde(rename = "marginUsed")]
    pub margin_used: Decimal,
}
