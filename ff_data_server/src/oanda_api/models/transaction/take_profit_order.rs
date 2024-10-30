use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use ff_standard_lib::standardized_types::accounts::AccountId;
use ff_standard_lib::standardized_types::orders::OrderId;
use crate::oanda_api::models::order::order_related::{OrderTriggerCondition, TimeInForce};
use crate::oanda_api::models::primitives::DateTime;
use crate::oanda_api::models::trade::TradeID;
use crate::oanda_api::models::transaction_related::{ClientExtensions, ClientID, RequestID, TakeProfitOrderReason, TransactionID, TransactionRejectReason, TransactionType};

/// A `TakeProfitOrderTransaction` represents the creation of a TakeProfit Order in the user’s Account.
#[derive(Serialize, Deserialize, Debug)]
pub struct TakeProfitOrderTransaction {
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

    /// The Type of the Transaction. Always set to “TAKE_PROFIT_ORDER”.
    #[serde(rename = "type")]
    pub type_of: TransactionType,

    /// The ID of the Trade to close when the prices threshold is breached.
    #[serde(rename = "tradeID")]
    pub trade_id: TradeID,

    /// The client ID of the Trade to be closed when the prices threshold is breached.
    #[serde(rename = "clientTradeID")]
    pub client_trade_id: Option<ClientID>,

    /// The prices threshold specified for the TakeProfit Order.
    #[serde(rename = "prices")]
    pub price: Decimal,

    /// The time-in-force requested for the TakeProfit Order.
    #[serde(rename = "timeInForce")]
    pub time_in_force: TimeInForce,

    /// The date/time when the TakeProfit Order will be cancelled if its timeInForce is “GTD”.
    #[serde(rename = "gtdTime")]
    pub gtd_time: Option<DateTime>,

    /// Specification of which prices component should be used when determining if an Order should be triggered and filled.
    #[serde(rename = "triggerCondition")]
    pub trigger_condition: OrderTriggerCondition,

    /// The reason that the Take Profit Order was initiated.
    #[serde(rename = "reason")]
    pub reason: TakeProfitOrderReason,

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

/// A `TakeProfitOrderRejectTransaction` represents the rejection of the creation of a TakeProfit Order.
#[derive(Serialize, Deserialize, Debug)]
pub struct TakeProfitOrderRejectTransaction {
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

    /// The Type of the Transaction. Always set to “TAKE_PROFIT_ORDER_REJECT”.
    #[serde(rename = "type")]
    pub type_of: TransactionType,

    /// The ID of the Trade to close when the prices threshold is breached.
    #[serde(rename = "tradeID")]
    pub trade_id: TradeID,

    /// The client ID of the Trade to be closed when the prices threshold is breached.
    #[serde(rename = "clientTradeID")]
    pub client_trade_id: Option<ClientID>,

    /// The prices threshold specified for the TakeProfit Order.
    #[serde(rename = "prices")]
    pub price: Decimal,

    /// The time-in-force requested for the TakeProfit Order.
    #[serde(rename = "timeInForce")]
    pub time_in_force: TimeInForce,

    /// The date/time when the TakeProfit Order will be cancelled if its timeInForce is “GTD”.
    #[serde(rename = "gtdTime")]
    pub gtd_time: Option<DateTime>,

    /// Specification of which prices component should be used when determining if an Order should be triggered and filled.
    #[serde(rename = "triggerCondition")]
    pub trigger_condition: OrderTriggerCondition,

    /// The reason that the Take Profit Order was initiated.
    #[serde(rename = "reason")]
    pub reason: TakeProfitOrderReason,

    /// Client Extensions to add to the Order.
    #[serde(rename = "clientExtensions")]
    pub client_extensions: Option<ClientExtensions>,

    /// The ID of the OrderFill Transaction that caused this Order to be created.
    #[serde(rename = "orderFillTransactionID")]
    pub order_fill_transaction_id: Option<TransactionID>,

    /// The ID of the Order that this Order was intended to replace.
    #[serde(rename = "intendedReplacesOrderID")]
    pub intended_replaces_order_id: Option<OrderId>,

    /// The reason that the Reject Transaction was created.
    #[serde(rename = "rejectReason")]
    pub reject_reason: TransactionRejectReason,
}
