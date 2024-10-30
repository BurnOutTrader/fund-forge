use serde::{Deserialize, Serialize};
use ff_standard_lib::standardized_types::accounts::AccountId;
use crate::oanda_api::models::primitives::DateTime;
use crate::oanda_api::models::transaction_related::{RequestID, TransactionID, TransactionType};

/// Represents a transaction when an Account enters the margin call state.
#[derive(Serialize, Deserialize, Debug)]
pub struct MarginCallEnterTransaction {
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

/// Represents a transaction when the margin call state for an Account has been extended.
#[derive(Serialize, Deserialize, Debug)]
pub struct MarginCallExtendTransaction {
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

    /// The number of extensions to the Account’s current margin call that have been applied.
    /// This value will be set to 1 for the first MarginCallExtend Transaction.
    #[serde(rename = "extensionNumber")]
    extension_number: i32,
}

/// Represents a transaction when an Account exits the margin call state.
#[derive(Serialize, Deserialize, Debug)]
pub struct MarginCallExitTransaction {
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


