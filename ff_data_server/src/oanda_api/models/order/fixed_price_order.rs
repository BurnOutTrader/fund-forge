use rust_decimal::Decimal;
use serde::{Serialize, Deserialize};
use ff_standard_lib::standardized_types::orders::OrderId;
use crate::oanda_api::models::order::order_related::{OrderPositionFill, OrderState, OrderType};
use crate::oanda_api::models::primitives::{DateTime, InstrumentName};
use crate::oanda_api::models::trade::TradeID;
use crate::oanda_api::models::transaction_related::{ClientExtensions, GuaranteedStopLossDetails, StopLossDetails, TakeProfitDetails, TrailingStopLossDetails, TransactionID};

/// A FixedPriceOrder is an order that is filled immediately upon creation using a fixed prices.
#[derive(Serialize, Deserialize, Debug)]
pub struct FixedPriceOrder {
    /// The Order’s identifier, unique within the Order’s Account.
    #[serde(rename = "id")]
    pub id: OrderId,

    /// The time when the Order was created.
    #[serde(rename = "createTime")]
    pub create_time: DateTime,

    /// The current state of the Order.
    #[serde(rename = "state")]
    pub state: OrderState,

    /// The client extensions of the Order. Do not set, modify, or delete clientExtensions if your account is associated with MT4.
    #[serde(rename = "clientExtensions")]
    pub client_extensions: ClientExtensions,

    /// The type of the Order. Always set to “FIXED_PRICE” for Fixed Price Orders.
    #[serde(rename = "type", default = "default_fixed_price_order_type")]
    pub order_type: OrderType,

    /// The Fixed Price Order’s Instrument.
    #[serde(rename = "instrument")]
    pub instrument: InstrumentName,

    /// The quantity requested to be filled by the Fixed Price Order. A positive number of units results in a long Order, and a negative number of units results in a short Order.
    #[serde(rename = "units")]
    pub units: Decimal,

    /// The prices specified for the Fixed Price Order. This prices is the exact prices that the Fixed Price Order will be filled at.
    #[serde(rename = "prices")]
    pub price: Decimal,

    /// Specification of how Positions in the Account are modified when the Order is filled.
    #[serde(rename = "positionFill")]
    pub position_fill: OrderPositionFill,

    /// The state that the trade resulting from the Fixed Price Order should be set to.
    #[serde(rename = "tradeState")]
    pub trade_state: String,

    /// TakeProfitDetails specifies the details of a Take Profit Order to be created on behalf of a client. This may happen when an Order is filled that opens a Trade requiring a Take Profit, or when a Trade’s dependent Take Profit Order is modified directly through the Trade.
    #[serde(rename = "takeProfitOnFill")]
    pub take_profit_on_fill: Option<TakeProfitDetails>,

    /// StopLossDetails specifies the details of a Stop Loss Order to be created on behalf of a client. This may happen when an Order is filled that opens a Trade requiring a Stop Loss, or when a Trade’s dependent Stop Loss Order is modified directly through the Trade.
    #[serde(rename = "stopLossOnFill")]
    pub stop_loss_on_fill: Option<StopLossDetails>,

    /// GuaranteedStopLossDetails specifies the details of a Guaranteed Stop Loss Order to be created on behalf of a client. This may happen when an Order is filled that opens a Trade requiring a Guaranteed Stop Loss, or when a Trade’s dependent Guaranteed Stop Loss Order is modified directly through the Trade.
    #[serde(rename = "guaranteedStopLossOnFill")]
    pub guaranteed_stop_loss_on_fill: Option<GuaranteedStopLossDetails>,

    /// TrailingStopLossDetails specifies the details of a Trailing Stop Loss Order to be created on behalf of a client. This may happen when an Order is filled that opens a Trade requiring a Trailing Stop Loss, or when a Trade’s dependent Trailing Stop Loss Order is modified directly through the Trade.
    #[serde(rename = "trailingStopLossOnFill")]
    pub trailing_stop_loss_on_fill: Option<TrailingStopLossDetails>,

    /// Client Extensions to add to the Trade created when the Order is filled (if such a Trade is created). Do not set, modify, or delete tradeClientExtensions if your account is associated with MT4.
    #[serde(rename = "tradeClientExtensions")]
    pub trade_client_extensions: ClientExtensions,

    /// ID of the Transaction that filled this Order (only provided when the Order’s state is FILLED)
    #[serde(rename = "fillingTransactionID")]
    pub filling_transaction_id: TransactionID,

    /// Date/time when the Order was filled (only provided when the Order’s state is FILLED)
    #[serde(rename = "filledTime")]
    pub filled_time: DateTime,

    /// Trade ID of Trade opened when the Order was filled (only provided when the Order’s state is FILLED and a Trade was opened as a result of the fill)
    #[serde(rename = "tradeOpenedID")]
    pub trade_opened_id: TradeID,

    /// Trade ID of Trade reduced when the Order was filled (only provided when the Order’s state is FILLED and a Trade was reduced as a result of the fill)
    #[serde(rename = "tradeReducedID")]
    pub trade_reduced_id: TradeID,

    /// Trade IDs of Trades closed when the Order was filled (only provided when the Order’s state is FILLED and one or more Trades were closed as a result of the fill)
    #[serde(rename = "tradeClosedIDs")]
    pub trade_closed_ids: Vec<TradeID>,

    /// ID of the Transaction that cancelled the Order (only provided when the Order’s state is CANCELLED)
    #[serde(rename = "cancellingTransactionID")]
    pub cancelling_transaction_id: TransactionID,

    /// Date/time when the Order was cancelled (only provided when the state of the Order is CANCELLED)
    #[serde(rename = "cancelledTime")]
    pub cancelled_time: DateTime,
}

fn default_fixed_price_order_type() -> OrderType {
    OrderType::FixedPrice
}