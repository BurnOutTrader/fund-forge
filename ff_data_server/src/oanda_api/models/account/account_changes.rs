use serde::Deserialize;
use serde::Serialize;
use crate::oanda_api::models::order::orders::Order;
use crate::oanda_api::models::position::OandaPosition;
use crate::oanda_api::models::trade::TradeSummary;
use crate::oanda_api::models::transaction::transaction::Transaction;

/// An AccountChanges Object is used to represent the changes to an Account's Orders, Trades,
/// and Positions since a specified Account TransactionID in the past.
#[derive(Debug, Serialize, Deserialize)]
struct AccountChanges {
    /// The Orders created. These Orders may have been filled, cancelled, or triggered in the same period.
    #[serde(rename = "ordersCreated")]
    orders_created: Vec<Order>,

    /// The Orders cancelled.
    #[serde(rename = "ordersCancelled")]
    orders_cancelled: Vec<Order>,

    /// The Orders filled.
    #[serde(rename = "ordersFilled")]
    orders_filled: Vec<Order>,

    /// The Orders triggered.
    #[serde(rename = "ordersTriggered")]
    orders_triggered: Vec<Order>,

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
