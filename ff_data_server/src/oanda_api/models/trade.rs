use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use ff_standard_lib::standardized_types::orders::OrderId;
use crate::oanda_api::models::order::stop_loss_order::StopLossOrder;
use crate::oanda_api::models::order::take_profit_order::TakeProfitOrder;
use crate::oanda_api::models::order::trailing_stop_loss_order::TrailingStopLossOrder;
use crate::oanda_api::models::primitives::{DateTime, InstrumentName};
use crate::oanda_api::models::transaction_related::{ClientExtensions, TransactionID};

pub type TradeID = String;
pub type TradeSpecifier = String;

/// The current state of the Trade.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TradeState {
    /// The Trade is currently open
    Open,

    /// The Trade has been fully closed
    Closed,

    /// The Trade will be closed as soon as the trade’s instrument becomes tradeable
    CloseWhenTradeable,
}

/// The state to filter the Trades by.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TradeStateFilter {
    /// The Trades that are currently open
    Open,

    /// The Trades that have been fully closed
    Closed,

    /// The Trades that will be closed as soon as the trades’ instrument becomes tradeable
    CloseWhenTradeable,

    /// The Trades that are in any of the possible states listed above.
    All,
}

/// The specification of a Trade within an Account. This includes the full representation
/// of the Trade’s dependent Orders in addition to the IDs of those Orders.
#[derive(Serialize, Deserialize, Debug)]
pub struct Trade {
    /// The Trade’s identifier, unique within the Trade’s Account.
    #[serde(rename = "id")]
    pub trade_id: TradeID,

    /// The Trade’s Instrument.
    pub instrument: InstrumentName,

    /// The execution prices of the Trade.
    pub price: Decimal,

    /// The date/time when the Trade was opened.
    #[serde(rename = "openTime")]
    pub open_time: DateTime,

    /// The current state of the Trade.
    pub state: TradeState,

    /// The initial size of the Trade. Negative values indicate a short Trade,
    /// and positive values indicate a long Trade.
    #[serde(rename = "initialUnits")]
    pub initial_units: Decimal,

    /// The margin required at the time the Trade was created. Note, this is the
    /// ‘pure’ margin required, it is not the ‘effective’ margin used that
    /// factors in the trade risk if a GSLO is attached to the trade.
    #[serde(rename = "initialMarginRequired")]
    pub initial_margin_required: Decimal,

    /// The number of units currently open for the Trade. This value is reduced
    /// to 0.0 as the Trade is closed.
    #[serde(rename = "currentUnits")]
    pub current_units: Decimal,

    /// The total profit/loss realized on the closed portion of the Trade.
    #[serde(rename = "realizedPL")]
    pub realized_pl: Decimal,

    /// The unrealized profit/loss on the open portion of the Trade.
    #[serde(rename = "unrealizedPL")]
    pub unrealized_pl: Decimal,

    /// Margin currently used by the Trade.
    #[serde(rename = "marginUsed")]
    pub margin_used: Decimal,

    /// The average closing prices of the Trade. Only present if the Trade has
    /// been closed or reduced at least once.
    #[serde(rename = "averageClosePrice")]
    pub average_close_price: Decimal,

    /// The IDs of the Transactions that have closed portions of this Trade.
    #[serde(rename = "closingTransactionIDs")]
    pub closing_transaction_ids: Vec<TransactionID>,

    /// The financing paid/collected for this Trade.
    pub financing: Decimal,

    /// The dividend adjustment paid for this Trade.
    #[serde(rename = "dividendAdjustment")]
    pub dividend_adjustment: Decimal,

    /// The date/time when the Trade was fully closed. Only provided for Trades
    /// whose state is CLOSED.
    #[serde(rename = "closeTime")]
    pub close_time: DateTime,

    /// The client extensions of the Trade.
    #[serde(rename = "clientExtensions")]
    pub client_extensions: ClientExtensions,

    /// Full representation of the Trade’s Take Profit Order, only provided if
    /// such an Order exists.
    #[serde(rename = "takeProfitOrder")]
    pub take_profit_order: Option<TakeProfitOrder>,

    /// Full representation of the Trade’s Stop Loss Order, only provided if such
    /// an Order exists.
    #[serde(rename = "stopLossOrder")]
    pub stop_loss_order: Option<StopLossOrder>,

    /// Full representation of the Trade’s Trailing Stop Loss Order, only
    /// provided if such an Order exists.
    #[serde(rename = "trailingStopLossOrder")]
    pub trailing_stop_loss_order: Option<TrailingStopLossOrder>,
}

/// The summary of a Trade within an Account. This representation does not
/// provide the full details of the Trade’s dependent Orders.
#[derive(Debug, Serialize, Deserialize)]
pub struct TradeSummary {
    /// The Trade's identifier, unique within the Trade's Account.
    #[serde(rename = "id")]
    pub id: TradeID,

    /// The Trade's Instrument.
    pub instrument: InstrumentName,

    /// The execution price of the Trade.
    pub price: Decimal,

    /// The date/time when the Trade was opened.
    #[serde(rename = "openTime")]
    pub open_time: DateTime,

    /// The current state of the Trade.
    pub state: TradeState,

    /// The initial size of the Trade. Negative values indicate a short Trade,
    /// and positive values indicate a long Trade.
    #[serde(rename = "initialUnits")]
    pub initial_units: Decimal,

    /// The margin required at the time the Trade was created. Note, this is the
    /// 'pure' margin required, it is not the 'effective' margin used that
    /// factors in the trade risk if a GSLO is attached to the trade.
    #[serde(rename = "initialMarginRequired")]
    pub initial_margin_required: Decimal,

    /// The number of units currently open for the Trade. This value is reduced
    /// to 0.0 as the Trade is closed.
    #[serde(rename = "currentUnits")]
    pub current_units: Decimal,

    /// The total profit/loss realized on the closed portion of the Trade.
    #[serde(rename = "realizedPL")]
    pub realized_pl: Decimal,

    /// The unrealized profit/loss on the open portion of the Trade.
    #[serde(rename = "unrealizedPL")]
    pub unrealized_pl: Decimal,

    /// Margin currently used by the Trade.
    #[serde(rename = "marginUsed")]
    pub margin_used: Decimal,

    /// The average closing price of the Trade. Only present if the Trade has
    /// been closed or reduced at least once.
    #[serde(rename = "averageClosePrice", skip_serializing_if = "Option::is_none")]
    pub average_close_price: Option<Decimal>,

    /// The IDs of the Transactions that have closed portions of this Trade.
    #[serde(rename = "closingTransactionIDs", default)]
    pub closing_transaction_ids: Vec<TransactionID>,

    /// The financing paid/collected for this Trade.
    pub financing: Decimal,

    /// The dividend adjustment paid for this Trade.
    #[serde(rename = "dividendAdjustment")]
    pub dividend_adjustment: Decimal,

    /// The date/time when the Trade was fully closed. Only provided for Trades
    /// whose state is CLOSED.
    #[serde(rename = "closeTime", skip_serializing_if = "Option::is_none")]
    pub close_time: Option<DateTime>,

    /// The client extensions of the Trade.
    #[serde(rename = "clientExtensions", skip_serializing_if = "Option::is_none")]
    pub client_extensions: Option<ClientExtensions>,

    /// ID of the Trade's Take Profit Order, only provided if such an Order exists.
    #[serde(rename = "takeProfitOrderID", skip_serializing_if = "Option::is_none")]
    pub take_profit_order_id: Option<OrderId>,

    /// ID of the Trade's Stop Loss Order, only provided if such an Order exists.
    #[serde(rename = "stopLossOrderID", skip_serializing_if = "Option::is_none")]
    pub stop_loss_order_id: Option<OrderId>,

    /// ID of the Trade's Guaranteed Stop Loss Order, only provided if such an Order exists.
    #[serde(rename = "guaranteedStopLossOrderID", skip_serializing_if = "Option::is_none")]
    pub guaranteed_stop_loss_order_id: Option<OrderId>,

    /// ID of the Trade's Trailing Stop Loss Order, only provided if such an Order exists.
    #[serde(rename = "trailingStopLossOrderID", skip_serializing_if = "Option::is_none")]
    pub trailing_stop_loss_order_id: Option<OrderId>,
}

/// The dynamic (calculated) state of an open Trade.
#[derive(Serialize, Deserialize, Debug)]
pub struct CalculatedTradeState {
    /// The Trade’s ID.
    #[serde(rename = "id")]
    pub trade_id: TradeID,

    /// The Trade’s unrealized profit/loss.
    #[serde(rename = "unrealizedPL")]
    pub unrealized_pl: Decimal,

    /// Margin currently used by the Trade.
    #[serde(rename = "marginUsed")]
    pub margin_used: Decimal,
}

/// The classification of TradePLs.
#[derive(Serialize, Deserialize, Debug)]
pub enum TradePL {
    /// An open Trade currently has a positive (profitable) unrealized P/L, or a closed Trade realized a positive amount of P/L.
    #[serde(rename = "POSITIVE")]
    Positive,

    /// An open Trade currently has a negative (losing) unrealized P/L, or a closed Trade realized a negative amount of P/L.
    #[serde(rename = "NEGATIVE")]
    Negative,

    /// An open Trade currently has unrealized P/L of zero (neither profitable nor losing), or a closed Trade realized a P/L amount of zero.
    #[serde(rename = "ZERO")]
    Zero,
}