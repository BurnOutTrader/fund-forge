use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use ff_standard_lib::standardized_types::accounts::AccountId;
use crate::oanda_api::models::order::order_related::OrderPositionFill;
use crate::oanda_api::models::primitives::{DateTime, InstrumentName};
use crate::oanda_api::models::transaction_related::{ClientExtensions, FixedPriceOrderReason, GuaranteedStopLossDetails, RequestID, StopLossDetails, TakeProfitDetails, TrailingStopLossDetails, TransactionID, TransactionType};

/// A `FixedPriceOrderTransaction` represents the creation of a Fixed Price Order in the user’s account.
#[derive(Serialize, Deserialize, Debug)]
pub struct FixedPriceOrderTransaction {
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

    /// The Type of the Transaction. Always set to “FIXED_PRICE_ORDER”.
    #[serde(rename = "type")]
    pub type_of: TransactionType,

    /// The Fixed Price Order’s Instrument.
    #[serde(rename = "instrument")]
    pub instrument: InstrumentName,

    /// The quantity requested to be filled by the Fixed Price Order.
    #[serde(rename = "units")]
    pub units: Decimal,

    /// The prices specified for the Fixed Price Order.
    #[serde(rename = "prices")]
    pub price: Decimal,

    /// Specification of how Positions in the Account are modified when the Order is filled.
    #[serde(rename = "positionFill")]
    pub position_fill: OrderPositionFill,

    /// The state that the trade resulting from the Fixed Price Order should be set to.
    #[serde(rename = "tradeState")]
    pub trade_state: String,

    /// The reason that the Fixed Price Order was created.
    #[serde(rename = "reason")]
    pub reason: FixedPriceOrderReason,

    /// The client extensions for the Fixed Price Order.
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
}


