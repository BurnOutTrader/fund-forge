use serde::{Deserialize, Serialize};
use ff_standard_lib::standardized_types::accounts::AccountId;
use crate::oanda_api::models::primitives::DateTime;
use crate::oanda_api::models::transaction_related::{RequestID, TransactionID, TransactionType};

/// Represents a transaction for resetting the Account’s resettable PL counters.
#[derive(Serialize, Deserialize, Debug)]
pub struct ResetResettablePLTransaction {
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
}