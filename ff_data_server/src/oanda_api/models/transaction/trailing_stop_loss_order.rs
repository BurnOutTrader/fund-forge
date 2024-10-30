use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use ff_standard_lib::standardized_types::accounts::AccountId;
use ff_standard_lib::standardized_types::orders::OrderId;
use crate::oanda_api::models::order::order_related::{OrderTriggerCondition, TimeInForce};
use crate::oanda_api::models::primitives::{DateTime};
use crate::oanda_api::models::trade::TradeID;
use crate::oanda_api::models::transaction_related::{ClientExtensions, ClientID, RequestID, TrailingStopLossOrderReason, TransactionID, TransactionRejectReason, TransactionType};

/// A `TrailingStopLossOrderTransaction` represents the creation of a TrailingStopLoss Order in the user’s Account.
#[derive(Serialize, Deserialize, Debug)]
pub struct TrailingStopLossOrderTransaction {
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

    /// The Type of the Transaction. Always set to “TRAILING_STOP_LOSS_ORDER”.
    #[serde(rename = "type")]
    pub type_of: TransactionType,

    /// The ID of the Trade to close when the prices threshold is breached.
    #[serde(rename = "tradeID")]
    pub trade_id: TradeID,

    /// The client ID of the Trade to be closed when the prices threshold is breached.
    #[serde(rename = "clientTradeID")]
    pub client_trade_id: Option<ClientID>,

    /// The prices distance (in prices units) specified for the TrailingStopLoss Order.
    #[serde(rename = "distance")]
    pub distance: Decimal,

    /// The time-in-force requested for the TrailingStopLoss Order.
    #[serde(rename = "timeInForce")]
    pub time_in_force: TimeInForce,

    /// The date/time when the StopLoss Order will be cancelled if its timeInForce is “GTD”.
    #[serde(rename = "gtdTime")]
    pub gtd_time: Option<DateTime>,

    /// Specification of which prices component should be used when determining if an Order should be triggered and filled.
    #[serde(rename = "triggerCondition")]
    pub trigger_condition: OrderTriggerCondition,

    /// The reason that the Trailing Stop Loss Order was initiated.
    #[serde(rename = "reason")]
    pub reason: TrailingStopLossOrderReason,

    /// Client Extensions to add to the Order.
    #[serde(rename = "clientExtensions")]
    pub client_extensions: Option<ClientExtensions>,

    /// The ID of the OrderFill Transaction that caused this Order to be created.
    #[serde(rename = "orderFillTransactionID")]
    pub order_fill_transaction_id: Option<TransactionID>,

    /// The ID of the Order that this Order replaces.
    #[serde(rename = "replacesOrderID")]
    pub replaces_order_id: Option<OrderId>,

    /// The ID of the Transaction that cancels the replaced Order.
    #[serde(rename = "cancellingTransactionID")]
    pub cancelling_transaction_id: Option<TransactionID>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TrailingStopLossOrderRejectTransaction {
    /// The Transaction’s Identifier.
    #[serde(rename = "id")]
    pub transaction_id: TransactionID,

    /// The date/time when the Transaction was created.
    pub time: DateTime,

    /// The ID of the user that initiated the creation of the Transaction.
    #[serde(rename = "userID")]
    pub user_id: i32,

    /// The ID of the Account the Transaction was created for.
    #[serde(rename = "accountID")]
    pub account_id: AccountId,

    /// The ID of the “batch” that the Transaction belongs to.
    // Transactions in the same batch are applied to the Account simultaneously.
    #[serde(rename = "batchID")]
    pub batch_transaction_id: TransactionID,

    /// The Request ID of the request which generated the transaction.
    #[serde(rename = "requestID")]
    pub request_id: RequestID,

    /// The Type of the Transaction. Always set to “TRAILING_STOP_LOSS_ORDER_REJECT” in a
    ///TrailingStopLossOrderRejectTransaction.
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,

    /// The ID of the Trade to close when the prices threshold is breached.
    #[serde(rename = "tradeID")]
    pub trade_id: TradeID,

    /// The client ID of the Trade to be closed when the prices threshold is breached.
    #[serde(rename = "clientTradeID")]
    pub client_trade_id: ClientID,

    /// The prices distance (in prices units) specified for the TrailingStopLoss Order.
    #[serde(rename = "distance")]
    pub distance: Decimal,

    /// The time-in-force requested for the TrailingStopLoss Order. Restricted to “GTC”, “GFD” and “GTD” for
    ///TrailingStopLoss Orders.
    #[serde(rename = "timeInForce")]
    pub time_in_force: TimeInForce,

    /// The date/time when the StopLoss Order will be cancelled if its timeInForce is “GTD”.
    #[serde(rename = "gtdTime")]
    pub gtd_time: DateTime,

    /// Specification of which prices component should be used when determining if an Order should be triggered and
    ///filled. This allows Orders to be triggered based on the bid, ask, mid, default (ask for buy, bid for sell) or i bid
    ///for buy) prices depending on the desired behaviour. Orders are always filled using their default prices component.
    /// This feature is only provided through the REST API. Clients who choose to specify a non-default trigger
    ///condition will not see it reflected in any of OANDA’s proprietary or partner trading platforms, their transaction
    ///history or their account statements. OANDA platforms always assume that an Order’s trigger condition is set to the
    ///default value when indicating the distance from an Order’s trigger prices, and will always provide the default
    ///trigger condition when creating or modifying an Order. A special restriction applies when creating a Guaranteed Stop
    ///Loss Order. In this case the TriggerCondition value must either be “DEFAULT”, or the “natural” trigger s
    ///“DEFAULT” results in. So for a Guaranteed Stop Loss Order for a long trade valid values are “DEFAULT” and “BID”,
    ///and for short trades “DEFAULT” and “ASK” are valid.
    #[serde(rename = "triggerCondition")]
    pub trigger_condition: OrderTriggerCondition,

    /// The reason that the Trailing Stop Loss Order was initiated
    #[serde(rename = "reason")]
    pub reason: TrailingStopLossOrderReason,

    /// Client Extensions to add to the Order (only provided if the Order is being created with client extensions).
    #[serde(rename = "clientExtensions")]
    pub client_extensions: ClientExtensions,

    /// The ID of the OrderFill Transaction that caused this Order to be created (only provided if this Order was
    ///created automatically when another Order was filled).
    #[serde(rename = "orderFillTransactionID")]
    pub order_fill_transaction_id: TransactionID,

    /// The ID of the Order that this Order was intended to replace (only provided if this Order was intended to
    ///replace an existing Order).
    #[serde(rename = "intendedReplacesOrderID")]
    pub intended_replaces_order_id: OrderId,

    /// The reason that the Reject Transaction was created
    #[serde(rename = "rejectReason")]
    pub reject_reason: TransactionRejectReason,
}


