use rust_decimal::Decimal;
use serde::{Serialize, Deserialize};
use ff_standard_lib::standardized_types::orders::OrderId;
use crate::oanda_api::models::order::order_related::{OrderState, OrderTriggerCondition, OrderType, TimeInForce};
use crate::oanda_api::models::primitives::DateTime;
use crate::oanda_api::models::trade::TradeID;
use crate::oanda_api::models::transaction_related::{ClientExtensions, ClientID, TransactionID};

/// A TakeProfitOrder is an order that is linked to an open Trade and created with a prices threshold. The Order will be filled (closing the Trade) by the first prices that is equal to or better than the threshold. A TakeProfitOrder cannot be used to open a new Position.
#[derive(Serialize, Deserialize, Debug)]
pub struct TakeProfitOrder {
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

    /// The type of the Order. Always set to “TAKE_PROFIT” for Take Profit Orders.
    #[serde(rename = "type", default = "default_take_profit_order_type")]
    pub order_type: OrderType,

    /// The ID of the Trade to close when the prices threshold is breached.
    #[serde(rename = "tradeID")]
    pub trade_id: TradeID,

    /// The client ID of the Trade to be closed when the prices threshold is breached.
    #[serde(rename = "clientTradeID")]
    pub client_trade_id: Option<ClientID>,

    /// The prices threshold specified for the TakeProfit Order. The associated Trade will be closed by a market prices that is equal to or better than this threshold.
    #[serde(rename = "prices")]
    pub price: Decimal,

    /// The time-in-force requested for the TakeProfit Order. Restricted to “GTC”, “GFD” and “GTD” for TakeProfit Orders, default = "TimeInForce::GTC"
    #[serde(rename = "timeInForce")]
    pub time_in_force: TimeInForce,

    /// The date/time when the TakeProfit Order will be cancelled if its timeInForce is “GTD”.
    #[serde(rename = "gtdTime")]
    pub gtd_time: Option<DateTime>,

    /// Specification of which prices component should be used when determining if an Order should be triggered and filled, default = "OrderTriggerCondition::DEFAULT"
    #[serde(rename = "triggerCondition")]
    pub trigger_condition: OrderTriggerCondition,

    /// ID of the Transaction that filled this Order (only provided when the Order’s state is FILLED)
    #[serde(rename = "fillingTransactionID")]
    pub filling_transaction_id: Option<TransactionID>,

    /// Date/time when the Order was filled (only provided when the Order’s state is FILLED)
    #[serde(rename = "filledTime")]
    pub filled_time: Option<DateTime>,

    /// Trade ID of Trade opened when the Order was filled (only provided when the Order’s state is FILLED and a Trade was opened as a result of the fill)
    #[serde(rename = "tradeOpenedID")]
    pub trade_opened_id: Option<TradeID>,

    /// Trade ID of Trade reduced when the Order was filled (only provided when the Order’s state is FILLED and a Trade was reduced as a result of the fill)
    #[serde(rename = "tradeReducedID")]
    pub trade_reduced_id: Option<TradeID>,

    /// Trade IDs of Trades closed when the Order was filled (only provided when the Order’s state is FILLED and one or more Trades were closed as a result of the fill)
    #[serde(rename = "tradeClosedIDs")]
    pub trade_closed_ids: Option<Vec<TradeID>>,

    /// ID of the Transaction that cancelled the Order (only provided when the Order’s state is CANCELLED)
    #[serde(rename = "cancellingTransactionID")]
    pub cancelling_transaction_id: Option<TransactionID>,

    /// Date/time when the Order was cancelled (only provided when the state of the Order is CANCELLED)
    #[serde(rename = "cancelledTime")]
    pub cancelled_time: Option<DateTime>,

    /// The ID of the Order that was replaced by this Order (only provided if this Order was created as part of a cancel/replace).
    #[serde(rename = "replacesOrderID")]
    pub replaces_order_id: Option<OrderId>,

    /// The ID of the Order that replaced this Order (only provided if this Order was cancelled as part of a cancel/replace).
    #[serde(rename = "replacedByOrderID")]
    pub replaced_by_order_id: Option<OrderId>,
}

fn default_take_profit_order_type() -> OrderType {
    OrderType::TakeProfit
}


/// A TakeProfitOrderRequest specifies the parameters that may be set when creating a Take Profit Order.
#[derive(Serialize, Deserialize, Debug)]
pub struct TakeProfitOrderRequest {
    /// The type of the Order to Create. Must be set to “TAKE_PROFIT” when creating a Take Profit Order.
    #[serde(rename = "type", default = "default_take_profit_order_type")]
    pub order_type: OrderType,

    /// The ID of the Trade to close when the prices threshold is breached.
    #[serde(rename = "tradeID")]
    pub trade_id: TradeID,

    /// The client ID of the Trade to be closed when the prices threshold is breached.
    #[serde(rename = "clientTradeID")]
    pub client_trade_id: Option<ClientID>,

    /// The prices threshold specified for the TakeProfit Order. The associated Trade will be closed by a market prices that is equal to or better than this threshold.
    #[serde(rename = "prices")]
    pub price: Decimal,

    /// The time-in-force requested for the TakeProfit Order. Restricted to “GTC”, “GFD” and “GTD” for TakeProfit Orders, default = "TimeInForce::GTC"
    #[serde(rename = "timeInForce")]
    pub time_in_force: TimeInForce,

    /// The date/time when the TakeProfit Order will be cancelled if its timeInForce is “GTD”.
    #[serde(rename = "gtdTime")]
    pub gtd_time: Option<DateTime>,

    /// Specification of which prices component should be used when determining if an Order should be triggered and filled.
    /// This allows Orders to be triggered based on the bid, ask, mid, default (ask for buy, bid for sell) or inverse (ask for sell, bid for buy)
    /// prices depending on the desired behaviour. Orders are always filled using their default prices component. This feature is only provided through the REST API.
    /// Clients who choose to specify a non-default trigger condition will not see it reflected in any of OANDA’s proprietary or partner trading platforms,
    /// their transaction history or their account statements. OANDA platforms always assume that an Order’s trigger condition is set to the default value when indicating the distance from an Order’s trigger prices,
    /// and will always provide the default trigger condition when creating or modifying an Order. A special restriction applies when creating a Guaranteed Stop Loss Order.
    /// In this case the TriggerCondition value must either be “DEFAULT”, or the “natural” trigger side “DEFAULT” results in.
    /// So for a Guaranteed Stop Loss Order for a long trade valid values are “DEFAULT” and “BID”, and for short trades “DEFAULT” and “ASK” are valid.
    ///default = "OrderTriggerCondition::DEFAULT"
    #[serde(rename = "triggerCondition")]
    pub trigger_condition: OrderTriggerCondition,

    /// The client extensions to add to the Order. Do not set, modify, or delete clientExtensions if your account is associated with MT4.
    #[serde(rename = "clientExtensions")]
    pub client_extensions: Option<ClientExtensions>,
}

