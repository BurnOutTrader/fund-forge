use rust_decimal::Decimal;
use serde::{Serialize, Deserialize};
use ff_standard_lib::standardized_types::orders::OrderId;
use crate::oanda_api::models::order::order_related::{OrderPositionFill, OrderState, OrderTriggerCondition, OrderType, TimeInForce};
use crate::oanda_api::models::primitives::{DateTime, InstrumentName};
use crate::oanda_api::models::trade::TradeID;
use crate::oanda_api::models::transaction_related::{ClientExtensions, GuaranteedStopLossDetails, StopLossDetails, TakeProfitDetails, TrailingStopLossDetails, TransactionID};

/// A MarketIfTouchedOrder is an order that is created with a prices threshold and will only be filled by a market prices that touches or crosses the threshold.
#[derive(Serialize, Deserialize, Debug)]
pub struct OandaMarketIfTouchedOrder {
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

    /// The type of the Order. Always set to “MARKET_IF_TOUCHED” for Market If Touched Orders.
    #[serde(rename = "type", default = "default_market_if_touched_order_type")]
    pub order_type: OrderType,

    /// The MarketIfTouched Order’s Instrument.
    #[serde(rename = "instrument")]
    pub instrument: InstrumentName,

    /// The quantity requested to be filled by the MarketIfTouched Order. A positive number of units results in a long Order, and a negative number of units results in a short Order.
    #[serde(rename = "units")]
    pub units: Decimal,

    /// The prices threshold specified for the MarketIfTouched Order. The MarketIfTouched Order will only be filled by a market prices that crosses this prices from the direction of the market prices at the time when the initialMarketPrice was recorded. Depending on the value of the Order’s prices and initialMarketPrice, the MarketIfTouched Order will behave like a Limit or a Stop Order.
    #[serde(rename = "prices")]
    pub price: Decimal,

    /// The worst market prices that may be used to fill this MarketIfTouched Order.
    #[serde(rename = "priceBound")]
    pub price_bound: Option<Decimal>,

    /// The time-in-force requested for the MarketIfTouched Order. Restricted to “GTC”, “GFD” and “GTD” for MarketIfTouched Orders, default = "TimeInForce::GTC"
    #[serde(rename = "timeInForce")]
    pub time_in_force: TimeInForce,

    /// The date/time when the MarketIfTouched Order will be cancelled if its timeInForce is “GTD”.
    #[serde(rename = "gtdTime")]
    pub gtd_time: Option<DateTime>,

    /// Specification of how Positions in the Account are modified when the Order is filled, default = "OrderPositionFill::DEFAULT"
    #[serde(rename = "positionFill")]
    pub position_fill: OrderPositionFill,

    /// Specification of which prices component should be used when determining if an Order should be triggered and filled, default = "OrderTriggerCondition::DEFAULT"
    #[serde(rename = "triggerCondition")]
    pub trigger_condition: OrderTriggerCondition,

    /// The Market prices at the time when the MarketIfTouched Order was created.
    #[serde(rename = "initialMarketPrice")]
    pub initial_market_price: Decimal,

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

    /// The ID of the Order that was replaced by this Order (only provided if this Order was created as part of a cancel/replace).
    #[serde(rename = "replacesOrderID")]
    pub replaces_order_id: Option<OrderId>,

    /// The ID of the Order that replaced this Order (only provided if this Order was cancelled as part of a cancel/replace).
    #[serde(rename = "replacedByOrderID")]
    pub replaced_by_order_id: Option<OrderId>,
}

fn default_market_if_touched_order_type() -> OrderType {
    OrderType::MarketIfTouched
}

/// A MarketIfTouchedOrderRequest specifies the parameters that may be set when creating a Market-if-Touched Order.
#[derive(Serialize, Deserialize, Debug)]
pub struct MarketIfTouchedOrderRequest {
    /// The type of the Order to Create. Must be set to “MARKET_IF_TOUCHED” when creating a Market If Touched Order.
    #[serde(rename = "type", default = "default_market_if_touched_order_type")]
    pub order_type: OrderType,

    /// The MarketIfTouched Order’s Instrument.
    #[serde(rename = "instrument")]
    pub instrument: InstrumentName,

    /// The quantity requested to be filled by the MarketIfTouched Order. A positive number of units results in a long Order, and a negative number of units results in a short Order.
    #[serde(rename = "units")]
    pub units: Decimal,

    /// The prices threshold specified for the MarketIfTouched Order. The MarketIfTouched Order will only be filled by a market prices that crosses this prices from the direction of the market prices at the time when the Order was created (the initialMarketPrice). Depending on the value of the Order’s prices and initialMarketPrice, the MarketIfTouched Order will behave like a Limit or a Stop Order.
    #[serde(rename = "prices")]
    pub price: Decimal,

    /// The worst market prices that may be used to fill this MarketIfTouched Order.
    #[serde(rename = "priceBound")]
    pub price_bound: Option<Decimal>,

    /// The time-in-force requested for the MarketIfTouched Order. Restricted to “GTC”, “GFD” and “GTD” for MarketIfTouched Orders, default = "TimeInForce::GTC"
    #[serde(rename = "timeInForce")]
    pub time_in_force: TimeInForce,

    /// The date/time when the MarketIfTouched Order will be cancelled if its timeInForce is “GTD”.
    #[serde(rename = "gtdTime")]
    pub gtd_time: Option<DateTime>,

    /// Specification of how Positions in the Account are modified when the Order is filled, default = "OrderPositionFill::DEFAULT"
    #[serde(rename = "positionFill")]
    pub position_fill: OrderPositionFill,

    /// Specification of which prices component should be used when determining if an Order should be triggered and filled.
    /// This allows Orders to be triggered based on the bid, ask, mid, default (ask for buy, bid for sell) or inverse (ask for sell, bid for buy)
    /// prices depending on the desired behaviour. Orders are always filled using their default prices component. This feature is only provided through the REST API.
    /// Clients who choose to specify a non-default trigger condition will not see it reflected in any of OANDA’s proprietary or partner trading platforms,
    /// their transaction history or their account statements. OANDA platforms always assume that an Order’s trigger condition is set to the default value when
    /// indicating the distance from an Order’s trigger prices, and will always provide the default trigger condition when creating or modifying an Order.
    /// A special restriction applies when creating a Guaranteed Stop Loss Order. In this case the TriggerCondition value must either be “DEFAULT”, or the “natural” trigger side “DEFAULT” results in.
    /// So for a Guaranteed Stop Loss Order for a long trade valid values are “DEFAULT” and “BID”, and for short trades “DEFAULT” and “ASK” are valid.
    /// default = "OrderTriggerCondition::DEFAULT"
    #[serde(rename = "triggerCondition")]
    pub trigger_condition: OrderTriggerCondition,

    /// The client extensions to add to the Order. Do not set, modify, or delete clientExtensions if your account is associated with MT4.
    #[serde(rename = "clientExtensions")]
    pub client_extensions: Option<ClientExtensions>,

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
    pub trade_client_extensions: Option<ClientExtensions>,
}