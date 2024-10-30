use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use ff_standard_lib::standardized_types::accounts::AccountId;
use ff_standard_lib::standardized_types::orders::OrderId;
use crate::oanda_api::models::order::order_related::{OrderPositionFill, OrderTriggerCondition, TimeInForce};
use crate::oanda_api::models::primitives::{DateTime, InstrumentName};
use crate::oanda_api::models::transaction_related::{ClientExtensions, GuaranteedStopLossDetails, MarketIfTouchedOrderReason, RequestID, StopLossDetails, TakeProfitDetails, TrailingStopLossDetails, TransactionID, TransactionRejectReason, TransactionType};

/// A `MarketIfTouchedOrderTransaction` represents the creation of a MarketIfTouched Order in the user’s Account.
#[derive(Serialize, Deserialize, Debug)]
#[allow(unused)]
pub struct MarketIfTouchedOrderTransaction {
    /// The Transaction’s Identifier.
    #[serde(rename = "id")]
    pub id: TransactionID,

    /// The date/time when the Transaction was created.
    #[serde(rename = "time")]
    pub time: DateTime,

    /// The ID of the user that initiated the creation of the Transaction.
    #[serde(rename = "userID")]
    pub user_id: i32,

    /// The ID of the Account the Transaction was created for.
    #[serde(rename = "accountID")]
    pub account_id: AccountId,

    /// The ID of the “batch” that the Transaction belongs to.
    #[serde(rename = "batchID")]
    pub batch_id: TransactionID,

    /// The Request ID of the request which generated the transaction.
    #[serde(rename = "requestID")]
    pub request_id: RequestID,

    /// The Type of the Transaction. Always set to “MARKET_IF_TOUCHED_ORDER”.
    #[serde(rename = "type")]
    pub type_of: TransactionType,

    /// The MarketIfTouched Order’s Instrument.
    #[serde(rename = "instrument")]
    pub instrument: InstrumentName,

    /// The quantity requested to be filled by the MarketIfTouched Order.
    #[serde(rename = "units")]
    pub units: Decimal,

    /// The prices threshold specified for the MarketIfTouched Order.
    #[serde(rename = "prices")]
    pub price: Decimal,

    /// The worst market prices that may be used to fill this MarketIfTouched Order.
    #[serde(rename = "priceBound")]
    pub price_bound: Option<Decimal>,

    /// The time-in-force requested for the MarketIfTouched Order.
    #[serde(rename = "timeInForce")]
    pub time_in_force: TimeInForce,

    /// The date/time when the MarketIfTouched Order will be cancelled if its timeInForce is “GTD”.
    #[serde(rename = "gtdTime")]
    pub gtd_time: Option<DateTime>,

    /// Specification of how Positions in the Account are modified when the Order is filled.
    #[serde(rename = "positionFill")]
    pub position_fill: OrderPositionFill,

    /// Specification of which prices component should be used when determining if an Order should be triggered and filled.
    #[serde(rename = "triggerCondition")]
    pub trigger_condition: OrderTriggerCondition,

    /// The reason that the Market-if-touched Order was initiated.
    #[serde(rename = "reason")]
    pub reason: MarketIfTouchedOrderReason,

    /// Client Extensions to add to the Order.
    #[serde(rename = "clientExtensions")]
    pub client_extensions: Option<ClientExtensions>,

    /// Specification of the Take Profit Order that should be created for a Trade.
    #[serde(rename = "takeProfitOnFill")]
    pub take_profit_on_fill: Option<TakeProfitDetails>,

    /// Specification of the Stop Loss Order that should be created for a Trade.
    #[serde(rename = "stopLossOnFill")]
    pub stop_loss_on_fill: Option<StopLossDetails>,

    /// Specification of the Trailing Stop Loss Order that should be created.
    #[serde(rename = "trailingStopLossOnFill")]
    pub trailing_stop_loss_on_fill: Option<TrailingStopLossDetails>,

    /// Specification of the Guaranteed Stop Loss Order that should be created.
    #[serde(rename = "guaranteedStopLossOnFill")]
    pub guaranteed_stop_loss_on_fill: Option<GuaranteedStopLossDetails>,

    /// Client Extensions to add to the Trade created when the Order is filled.
    #[serde(rename = "tradeClientExtensions")]
    pub trade_client_extensions: Option<ClientExtensions>,

    /// The ID of the Order that this Order replaces.
    #[serde(rename = "replacesOrderID")]
    pub replaces_order_id: Option<OrderId>,

    /// The ID of the Transaction that cancels the replaced Order.
    #[serde(rename = "cancellingTransactionID")]
    pub cancelling_transaction_id: Option<TransactionID>,
}

/// A `MarketIfTouchedOrderRejectTransaction` represents the rejection of the creation of a MarketIfTouched Order.
#[derive(Serialize, Deserialize, Debug)]
pub struct MarketIfTouchedOrderRejectTransaction {
    /// The Transaction’s Identifier.
    #[serde(rename = "id")]
    pub id: TransactionID,

    /// The date/time when the Transaction was created.
    #[serde(rename = "time")]
    pub time: DateTime,

    /// The ID of the user that initiated the creation of the Transaction.
    #[serde(rename = "userID")]
    pub user_id: i32,

    /// The ID of the Account the Transaction was created for.
    #[serde(rename = "accountID")]
    pub account_id: AccountId,

    /// The ID of the “batch” that the Transaction belongs to.
    #[serde(rename = "batchID")]
    pub batch_id: TransactionID,

    /// The Request ID of the request which generated the transaction.
    #[serde(rename = "requestID")]
    pub request_id: RequestID,

    /// The Type of the Transaction. Always set to “MARKET_IF_TOUCHED_ORDER_REJECT”.
    #[serde(rename = "type")]
    pub type_of: TransactionType,

    /// The MarketIfTouched Order’s Instrument.
    #[serde(rename = "instrument")]
    pub instrument: InstrumentName,

    /// The quantity requested to be filled by the MarketIfTouched Order.
    #[serde(rename = "units")]
    pub units: Decimal,

    /// The prices threshold specified for the MarketIfTouched Order.
    #[serde(rename = "prices")]
    pub price: Decimal,

    /// The worst market prices that may be used to fill this MarketIfTouched Order.
    #[serde(rename = "priceBound")]
    pub price_bound: Option<Decimal>,

    /// The time-in-force requested for the MarketIfTouched Order.
    #[serde(rename = "timeInForce")]
    pub time_in_force: TimeInForce,

    /// The date/time when the MarketIfTouched Order will be cancelled if its timeInForce is “GTD”.
    #[serde(rename = "gtdTime")]
    pub gtd_time: Option<DateTime>,

    /// Specification of how Positions in the Account are modified when the Order is filled.
    #[serde(rename = "positionFill")]
    pub position_fill: OrderPositionFill,

    /// Specification of which prices component should be used when determining if an Order should be triggered and filled.
    #[serde(rename = "triggerCondition")]
    pub trigger_condition: OrderTriggerCondition,

    /// The reason that the Market-if-touched Order was initiated.
    #[serde(rename = "reason")]
    #[allow(unused_variables)]
    pub reason: MarketIfTouchedOrderReason,

    /// Client Extensions to add to the Order.
    #[serde(rename = "clientExtensions")]
    pub client_extensions: Option<ClientExtensions>,

    /// Specification of the Take Profit Order that should be created for a Trade.
    #[serde(rename = "takeProfitOnFill")]
    pub take_profit_on_fill: Option<TakeProfitDetails>,

    /// Specification of the Stop Loss Order that should be created for a Trade.
    #[serde(rename = "stopLossOnFill")]
    pub stop_loss_on_fill: Option<StopLossDetails>,

    /// Specification of the Trailing Stop Loss Order that should be created.
    #[serde(rename = "trailingStopLossOnFill")]
    pub trailing_stop_loss_on_fill: Option<TrailingStopLossDetails>,

    /// Specification of the Guaranteed Stop Loss Order that should be created.
    #[serde(rename = "guaranteedStopLossOnFill")]
    pub guaranteed_stop_loss_on_fill: Option<GuaranteedStopLossDetails>,

    /// Client Extensions to add to the Trade created when the Order is filled.
    #[serde(rename = "tradeClientExtensions")]
    pub trade_client_extensions: Option<ClientExtensions>,

    /// The ID of the Order that this Order was intended to replace.
    #[serde(rename = "intendedReplacesOrderID")]
    pub intended_replaces_order_id: Option<OrderId>,

    /// The reason that the Reject Transaction was created.
    #[serde(rename = "rejectReason")]
    pub reject_reason: TransactionRejectReason,
}