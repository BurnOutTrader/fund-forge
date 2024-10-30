use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use ff_standard_lib::standardized_types::accounts::AccountId;
use ff_standard_lib::standardized_types::orders::OrderId;
use crate::oanda_api::models::pricing::ClientPrice;
use crate::oanda_api::models::primitives::{DateTime, HomeConversionFactors, InstrumentName};
use crate::oanda_api::models::transaction_related::{ClientExtensions, ClientID, OrderCancelReason, OrderFillReason, RequestID, TradeOpen, TradeReduce, TransactionID, TransactionRejectReason, TransactionType};

/// A transaction that represents the filling of an order.
#[derive(Serialize, Deserialize, Debug)]
pub struct OrderFillTransaction {
    /// The Transaction’s Identifier.
    #[serde(rename = "id")]
    id: TransactionID,

    /// The date/time when the Transaction was created.
    #[serde(rename = "time")]
    time: DateTime,

    /// The ID of the user that initiated the creation of the Transaction.
    #[serde(rename = "userID")]
    user_id: i32,

    /// The ID of the Account the Transaction was created for.
    #[serde(rename = "accountID")]
    account_id: AccountId,

    /// The ID of the “batch” that the Transaction belongs to.
    #[serde(rename = "batchID")]
    batch_id: TransactionID,

    /// The Request ID of the request which generated the transaction.
    #[serde(rename = "requestID")]
    request_id: RequestID,

    /// The Type of the Transaction.
    #[serde(rename = "type")]
    type_of: TransactionType,

    /// The ID of the Order filled.
    #[serde(rename = "orderID")]
    order_id: OrderId,

    /// The client Order ID of the Order filled.
    #[serde(rename = "clientOrderID")]
    client_order_id: Option<ClientID>,

    /// The name of the filled Order’s instrument.
    #[serde(rename = "instrument")]
    instrument: InstrumentName,

    /// The number of units filled by the OrderFill.
    #[serde(rename = "units")]
    units: Decimal,

    /// The conversion factor for gains. Deprecated.
    #[serde(rename = "gainQuoteHomeConversionFactor")]
    #[deprecated(note = "Will be removed in a future API update.")]
    gain_quote_home_conversion_factor: Option<Decimal>,

    /// The conversion factor for losses. Deprecated.
    #[serde(rename = "lossQuoteHomeConversionFactor")]
    #[deprecated(note = "Will be removed in a future API update.")]
    loss_quote_home_conversion_factor: Option<Decimal>,

    /// The HomeConversionFactors in effect at the time of the OrderFill.
    #[serde(rename = "homeConversionFactors")]
    home_conversion_factors: HomeConversionFactors,

    /// The prices of the OrderFill. Deprecated.
    #[serde(rename = "prices")]
    #[deprecated(note = "Will be removed in a future API update.")]
    price: Option<Decimal>,

    /// The full volume-weighted average prices.
    #[serde(rename = "fullVWAP")]
    full_vwap: Decimal,

    /// The prices in effect for the account at the time of the Order fill.
    #[serde(rename = "fullPrice")]
    full_price: ClientPrice,

    /// The reason that an Order was filled.
    #[serde(rename = "reason")]
    reason: OrderFillReason,

    /// The profit or loss incurred when the Order was filled.
    #[serde(rename = "pl")]
    pl: Decimal,

    /// The profit or loss incurred in the quote currency.
    #[serde(rename = "quotePL")]
    quote_pl: Decimal,

    /// The financing paid or collected when the Order was filled.
    #[serde(rename = "financing")]
    financing: Decimal,

    /// The base currency financing.
    #[serde(rename = "baseFinancing")]
    base_financing: Decimal,

    /// The quote currency financing.
    #[serde(rename = "quoteFinancing")]
    quote_financing: Decimal,

    /// The commission charged for filling the Order.
    #[serde(rename = "commission")]
    commission: Decimal,

    /// The total guaranteed execution fees.
    #[serde(rename = "guaranteedExecutionFee")]
    guaranteed_execution_fee: Decimal,

    /// The execution fees in the quote currency.
    #[serde(rename = "quoteGuaranteedExecutionFee")]
    quote_guaranteed_execution_fee: Decimal,

    /// The Account’s balance after the Order was filled.
    #[serde(rename = "accountBalance")]
    account_balance: Decimal,

    /// The Trade that was opened when the Order was filled.
    #[serde(rename = "tradeOpened")]
    trade_opened: Option<TradeOpen>,

    /// The Trades that were closed when the Order was filled.
    #[serde(rename = "tradesClosed")]
    trades_closed: Vec<TradeReduce>,

    /// The Trade that was reduced when the Order was filled.
    #[serde(rename = "tradeReduced")]
    trade_reduced: Option<TradeReduce>,

    /// The half spread cost for the OrderFill.
    #[serde(rename = "halfSpreadCost")]
    half_spread_cost: Decimal,
}

/// Represents a transaction that cancels an order.
#[derive(Serialize, Deserialize, Debug)]
pub struct OrderCancelTransaction {
    /// The Transaction’s Identifier.
    #[serde(rename = "id")]
    id: TransactionID,

    /// The date/time when the Transaction was created.
    #[serde(rename = "time")]
    time: DateTime,

    /// The ID of the user that initiated the creation of the Transaction.
    #[serde(rename = "userID")]
    user_id: i32,

    /// The ID of the Account the Transaction was created for.
    #[serde(rename = "accountID")]
    account_id: AccountId,

    /// The ID of the “batch” that the Transaction belongs to.
    #[serde(rename = "batchID")]
    batch_id: TransactionID,

    /// The Request ID of the request which generated the transaction.
    #[serde(rename = "requestID")]
    request_id: RequestID,

    /// The Type of the Transaction.
    #[serde(rename = "type")]
    type_of: TransactionType,

    /// The ID of the Order cancelled.
    #[serde(rename = "orderID")]
    order_id: OrderId,

    /// The client ID of the Order cancelled.
    #[serde(rename = "clientOrderID")]
    client_order_id: Option<OrderId>,

    /// The reason that the Order was cancelled.
    #[serde(rename = "reason")]
    reason: OrderCancelReason,

    /// The ID of the Order that replaced this Order.
    #[serde(rename = "replacedByOrderID")]
    replaced_by_order_id: Option<OrderId>,
}

/// Represents a transaction that rejects an order cancellation.
#[derive(Serialize, Deserialize, Debug)]
pub struct OrderCancelRejectTransaction {
    /// The Transaction’s Identifier.
    #[serde(rename = "id")]
    id: TransactionID,

    /// The date/time when the Transaction was created.
    #[serde(rename = "time")]
    time: DateTime,

    /// The ID of the user that initiated the creation of the Transaction.
    #[serde(rename = "userID")]
    user_id: i32,

    /// The ID of the Account the Transaction was created for.
    #[serde(rename = "accountID")]
    account_id: AccountId,

    /// The ID of the “batch” that the Transaction belongs to.
    #[serde(rename = "batchID")]
    batch_id: TransactionID,

    /// The Request ID of the request which generated the transaction.
    #[serde(rename = "requestID")]
    request_id: RequestID,

    /// The Type of the Transaction.
    #[serde(rename = "type")]
    type_of: TransactionType,

    /// The ID of the Order intended to be cancelled.
    #[serde(rename = "orderID")]
    order_id: OrderId,

    /// The client ID of the Order intended to be cancelled.
    #[serde(rename = "clientOrderID")]
    client_order_id: Option<OrderId>,

    /// The reason that the Reject Transaction was created.
    #[serde(rename = "rejectReason")]
    reject_reason: TransactionRejectReason,
}

/// Represents a transaction that modifies client extensions of an order.
#[derive(Serialize, Deserialize, Debug)]
pub struct OrderClientExtensionsModifyTransaction {
    /// The Transaction’s Identifier.
    #[serde(rename = "id")]
    id: TransactionID,

    /// The date/time when the Transaction was created.
    #[serde(rename = "time")]
    time: DateTime,

    /// The ID of the user that initiated the creation of the Transaction.
    #[serde(rename = "userID")]
    user_id: i32,

    /// The ID of the Account the Transaction was created for.
    #[serde(rename = "accountID")]
    account_id: AccountId,

    /// The ID of the “batch” that the Transaction belongs to.
    #[serde(rename = "batchID")]
    batch_id: TransactionID,

    /// The Request ID of the request which generated the transaction.
    #[serde(rename = "requestID")]
    request_id: RequestID,

    /// The Type of the Transaction.
    #[serde(rename = "type")]
    type_of: TransactionType,

    /// The ID of the Order whose client extensions are to be modified.
    #[serde(rename = "orderID")]
    order_id: OrderId,

    /// The original Client ID of the Order whose client extensions are to be modified.
    #[serde(rename = "clientOrderID")]
    client_order_id: ClientID,

    /// The new Client Extensions for the Order.
    #[serde(rename = "clientExtensionsModify")]
    client_extensions_modify: ClientExtensions,

    /// The new Client Extensions for the Order’s Trade on fill.
    #[serde(rename = "tradeClientExtensionsModify")]
    trade_client_extensions_modify: ClientExtensions,
}

/// Represents a transaction that rejects the modification of client extensions of an order.
#[derive(Serialize, Deserialize, Debug)]
pub struct OrderClientExtensionsModifyRejectTransaction {
    /// The Transaction’s Identifier.
    #[serde(rename = "id")]
    id: TransactionID,

    /// The date/time when the Transaction was created.
    #[serde(rename = "time")]
    time: DateTime,

    /// The ID of the user that initiated the creation of the Transaction.
    #[serde(rename = "userID")]
    user_id: i32,

    /// The ID of the Account the Transaction was created for.
    #[serde(rename = "accountID")]
    account_id: AccountId,

    /// The ID of the “batch” that the Transaction belongs to.
    #[serde(rename = "batchID")]
    batch_id: TransactionID,

    /// The Request ID of the request which generated the transaction.
    #[serde(rename = "requestID")]
    request_id: RequestID,

    /// The Type of the Transaction.
    #[serde(rename = "type")]
    type_of: TransactionType,

    /// The ID of the Order whose client extensions are to be modified.
    #[serde(rename = "orderID")]
    order_id: OrderId,

    /// The original Client ID of the Order whose client extensions are to be modified.
    #[serde(rename = "clientOrderID")]
    client_order_id: ClientID,

    /// The new Client Extensions for the Order.
    #[serde(rename = "clientExtensionsModify")]
    client_extensions_modify: ClientExtensions,

    /// The new Client Extensions for the Order’s Trade on fill.
    #[serde(rename = "tradeClientExtensionsModify")]
    trade_client_extensions_modify: ClientExtensions,

    /// The reason that the Reject Transaction was created.
    #[serde(rename = "rejectReason")]
    reject_reason: TransactionRejectReason,
}

