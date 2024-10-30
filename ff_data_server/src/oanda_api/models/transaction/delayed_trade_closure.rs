use serde::{Deserialize, Serialize};
use ff_standard_lib::standardized_types::accounts::AccountId;
use crate::oanda_api::models::primitives::DateTime;
use crate::oanda_api::models::trade::TradeID;
use crate::oanda_api::models::transaction_related::{MarketOrderReason, RequestID, TransactionID, TransactionType};

/// Represents a transaction indicating open trades that should have been closed but weren't due to untradeable instruments.
#[derive(Serialize, Deserialize, Debug)]
pub struct DelayedTradeClosureTransaction {
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

    /// The reason for the delayed trade closure.
    #[serde(rename = "reason")]
    reason: MarketOrderReason,

    /// List of Trade IDs identifying the open trades that will be closed when their respective instruments become tradeable.
    #[serde(rename = "tradeIDs")]
    trade_ids: Vec<TradeID>,
}