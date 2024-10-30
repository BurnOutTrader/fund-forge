use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use ff_standard_lib::standardized_types::accounts::AccountId;
use crate::oanda_api::models::order::order_related::{OrderPositionFill, TimeInForce};
use crate::oanda_api::models::primitives::{DateTime, InstrumentName};
use crate::oanda_api::models::trade::TradeID;
use crate::oanda_api::models::transaction_related::{ClientExtensions, GuaranteedStopLossDetails, MarketOrderReason, RequestID, StopLossDetails, TakeProfitDetails, TrailingStopLossDetails, TransactionID, TransactionRejectReason, TransactionType};

/// MarketOrderTransaction represents the creation of a Market Order in the user’s account.
/// A Market Order Transaction represents the creation of a Market Order in the user's account.
/// A Market Order is an Order that is filled immediately at the current market prices.
#[derive(Debug, Serialize, Deserialize)]
struct MarketOrderTransaction {
    /// The Transaction’s Identifier.
    #[serde(rename = "id")]
    transaction_id: TransactionID,

    /// The date/time when the Transaction was created.
    #[serde(rename = "time")]
    created_time: DateTime,

    /// The ID of the user that initiated the creation of the Transaction.
    #[serde(rename = "userID")]
    user_id: i64,

    /// The ID of the Account the Transaction was created for.
    #[serde(rename = "accountID")]
    account_id: AccountId,

    /// The ID of the “batch” that the Transaction belongs to.
    #[serde(rename = "batchID")]
    batch_id: TransactionID,

    /// The Request ID of the request which generated the transaction.
    #[serde(rename = "requestID")]
    request_id: RequestID,

    /// The Type of the Transaction. Always set to “MARKET_ORDER” in a MarketOrderTransaction.
    #[serde(rename = "type")]
    type_of: TransactionType,

    /// The Market Order’s Instrument.
    #[serde(rename = "instrument")]
    instrument: InstrumentName,

    /// The quantity requested to be filled by the Market Order.
    #[serde(rename = "units")]
    quantity: Decimal,

    /// The time-in-force requested for the Market Order. Restricted to FOK or IOC for a MarketOrder.
    #[serde(rename = "timeInForce")]
    time_in_force: TimeInForce,

    /// The worst prices that the client is willing to have the Market Order filled at.
    #[serde(rename = "priceBound")]
    price_bound: Option<Decimal>,

    /// Specification of how Positions in the Account are modified when the Order is filled.
    #[serde(rename = "positionFill")]
    position_fill: OrderPositionFill,

    /// Details of the Trade requested to be closed, only provided when the Market Order is being used to explicitly close a Trade.
    #[serde(rename = "tradeClose")]
    trade_close: Option<MarketOrderTradeClose>,

    /// Details of the long Position requested to be closed out, only provided when a Market Order is being used to explicitly closeout a long Position.
    #[serde(rename = "longPositionCloseout")]
    long_position_closeout: Option<MarketOrderPositionCloseout>,

    /// Details of the short Position requested to be closed out, only provided when a Market Order is being used to explicitly closeout a short Position.
    #[serde(rename = "shortPositionCloseout")]
    short_position_closeout: Option<MarketOrderPositionCloseout>,

    /// Details of the Margin Closeout that this Market Order was created for.
    #[serde(rename = "marginCloseout")]
    margin_closeout: MarketOrderMarginCloseout,

    /// Details of the delayed Trade close that this Market Order was created for.
    #[serde(rename = "delayedTradeClose")]
    delayed_trade_close: MarketOrderDelayedTradeClose,

    /// The reason that the Market Order was created.
    #[serde(rename = "reason")]
    reason: MarketOrderReason,

    /// Client Extensions to add to the Order (only provided if the Order is being created with client extensions).
    #[serde(rename = "clientExtensions")]
    client_extensions: Option<ClientExtensions>,

    /// The specification of the Take Profit Order that should be created for a Trade opened when the Order is filled (if such a Trade is created).
    #[serde(rename = "takeProfitOnFill")]
    take_profit_on_fill: Option<TakeProfitDetails>,

    /// The specification of the Stop Loss Order that should be created for a Trade opened when the Order is filled (if such a Trade is created).
    #[serde(rename = "stopLossOnFill")]
    stop_loss_on_fill: Option<StopLossDetails>,

    /// The specification of the Trailing Stop Loss Order that should be created for a Trade that is opened when the Order is filled (if such a Trade is created).
    #[serde(rename = "trailingStopLossOnFill")]
    trailing_stop_loss_on_fill: Option<TrailingStopLossDetails>,

    /// The specification of the Guaranteed Stop Loss Order that should be created for a Trade that is opened when the Order is filled (if such a Trade is created).
    #[serde(rename = "guaranteedStopLossOnFill")]
    guaranteed_stop_loss_on_fill: Option<GuaranteedStopLossDetails>,

    /// Client Extensions to add to the Trade created when the Order is filled (if such a Trade is created).
    #[serde(rename = "tradeClientExtensions")]
    trade_client_extensions: Option<ClientExtensions>,
}

/// A `MarketOrderRejectTransaction` represents the rejection of the creation of a Market Order.
#[derive(Serialize, Deserialize, Debug)]
pub struct MarketOrderRejectTransaction {
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

    /// The Type of the Transaction. Always set to “MARKET_ORDER_REJECT”.
    #[serde(rename = "type")]
    pub type_of: TransactionType,

    /// The Market Order’s Instrument.
    #[serde(rename = "instrument")]
    pub instrument: InstrumentName,

    /// The quantity requested to be filled by the Market Order.
    #[serde(rename = "units")]
    pub units: Decimal,

    /// The time-in-force requested for the Market Order.
    #[serde(rename = "timeInForce")]
    pub time_in_force: TimeInForce,

    /// The worst prices that the client is willing to have the Market Order filled at.
    #[serde(rename = "priceBound")]
    pub price_bound: Option<Decimal>,

    /// Specification of how Positions in the Account are modified when the Order is filled.
    #[serde(rename = "positionFill")]
    pub position_fill: OrderPositionFill,

    /// Details of the Trade requested to be closed.
    #[serde(rename = "tradeClose")]
    pub trade_close: Option<MarketOrderTradeClose>,

    /// Details of the long Position requested to be closed out.
    #[serde(rename = "longPositionCloseout")]
    pub long_position_closeout: Option<MarketOrderPositionCloseout>,

    /// Details of the short Position requested to be closed out.
    #[serde(rename = "shortPositionCloseout")]
    pub short_position_closeout: Option<MarketOrderPositionCloseout>,

    /// Details of the Margin Closeout that this Market Order was created for.
    #[serde(rename = "marginCloseout")]
    pub margin_closeout: Option<MarketOrderMarginCloseout>,

    /// Details of the delayed Trade close that this Market Order was created for.
    #[serde(rename = "delayedTradeClose")]
    pub delayed_trade_close: Option<MarketOrderDelayedTradeClose>,

    /// The reason that the Market Order was created.
    #[serde(rename = "reason")]
    pub reason: MarketOrderReason,

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

    /// Client Extensions to add to the Trade.
    #[serde(rename = "tradeClientExtensions")]
    pub trade_client_extensions: Option<ClientExtensions>,

    /// The reason that the Reject Transaction was created.
    #[serde(rename = "rejectReason")]
    pub reject_reason: TransactionRejectReason,
}

/// A MarketOrderTradeClose specifies the extensions to a Market Order that has been created specifically to close a Trade.
#[derive(Serialize, Deserialize, Debug)]
pub struct MarketOrderTradeClose {
    /// The ID of the Trade requested to be closed
    #[serde(rename = "tradeID")]
    pub trade_id: TradeID,

    /// The client ID of the Trade requested to be closed
    #[serde(rename = "clientTradeID")]
    pub client_trade_id: String,

    /// Indication of how much of the Trade to close. Either “ALL”, or a Decimal reflecting a partial close of the Trade.
    #[serde(rename = "units")]
    pub units: String,
}

/// Details for the Market Order extensions specific to a Market Order placed that is part of a Market Order Margin Closeout in a client’s account.
#[derive(Serialize, Deserialize, Debug)]
pub struct MarketOrderMarginCloseout {
    /// The reason the Market Order was created to perform a margin closeout
    #[serde(rename = "reason")]
    pub reason: MarketOrderMarginCloseoutReason,
}

/// The reason that the Market Order was created to perform a margin closeout.
#[derive(Serialize, Deserialize, Debug)]
pub enum MarketOrderMarginCloseoutReason {
    /// Trade closures resulted from violating OANDA’s margin policy
    #[serde(rename = "MARGIN_CHECK_VIOLATION")]
    MarginCheckViolation,

    /// Trade closures came from a margin closeout event resulting from regulatory conditions placed on the Account’s margin call
    #[serde(rename = "REGULATORY_MARGIN_CALL_VIOLATION")]
    RegulatoryMarginCallViolation,

    /// Trade closures resulted from violating the margin policy imposed by regulatory requirements
    #[serde(rename = "REGULATORY_MARGIN_CHECK_VIOLATION")]
    RegulatoryMarginCheckViolation,
}

/// Details for the Market Order extensions specific to a Market Order placed with the intent of fully closing a specific open trade that should have already been closed but wasn’t due to halted market conditions.
#[derive(Serialize, Deserialize, Debug)]
pub struct MarketOrderDelayedTradeClose {
    /// The ID of the Trade being closed
    #[serde(rename = "tradeID")]
    pub trade_id: TradeID,

    /// The Client ID of the Trade being closed
    #[serde(rename = "clientTradeID")]
    pub client_trade_id: TradeID,

    /// The Transaction ID of the DelayedTradeClosure transaction to which this Delayed Trade Close belongs to
    #[serde(rename = "sourceTransactionID")]
    pub source_transaction_id: TransactionID,
}

/// A MarketOrderPositionCloseout specifies the extensions to a Market Order when it has been created to close out a specific Position.
#[derive(Serialize, Deserialize, Debug)]
pub struct MarketOrderPositionCloseout {
    /// The instrument of the Position being closed out.
    #[serde(rename = "instrument")]
    pub instrument: InstrumentName,

    /// Indication of how much of the Position to close. Either “ALL”, or a Decimal reflecting a partial close of the Position.
    #[serde(rename = "units")]
    pub units: String,
}




