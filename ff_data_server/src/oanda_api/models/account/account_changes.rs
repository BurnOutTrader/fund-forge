use serde::Deserialize;
use crate::oanda_api::models::order::placement::OandaOrder;
use crate::oanda_api::models::position::OandaPosition;
use crate::oanda_api::models::trade::TradeSummary;
use crate::oanda_api::models::transaction::transaction::Transaction;

/// An AccountChanges Object is used to represent the changes to an Account's Orders, Trades,
/// and Positions since a specified Account TransactionID in the past.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AccountChanges {
    /// The Orders created. These Orders may have been filled, cancelled, or triggered in the same period.
    #[serde(rename = "ordersCreated")]
    orders_created: Vec<OandaOrder>,

    /// The Orders cancelled.
    #[serde(rename = "ordersCancelled")]
    orders_cancelled: Vec<OandaOrder>,

    /// The Orders filled.
    #[serde(rename = "ordersFilled")]
    orders_filled: Vec<OandaOrder>,

    /// The Orders triggered.
    #[serde(rename = "ordersTriggered")]
    orders_triggered: Vec<OandaOrder>,

    /// The Trades opened.
    #[serde(rename = "tradesOpened")]
    trades_opened: Vec<TradeSummary>,

    /// The Trades reduced.
    #[serde(rename = "tradesReduced")]
    trades_reduced: Vec<TradeSummary>,

    /// The Trades closed.
    #[serde(rename = "tradesClosed")]
    trades_closed: Vec<TradeSummary>,

    /// The Positions changed.
    #[serde(rename = "positions")]
    positions: Vec<OandaPosition>,

    /// The Transactions that have been generated.
    #[serde(rename = "transactions")]
    transactions: Vec<Transaction>,
}
