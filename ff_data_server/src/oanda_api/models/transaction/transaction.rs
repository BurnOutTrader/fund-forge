use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use ff_standard_lib::standardized_types::accounts::{AccountId, Currency};
use crate::oanda_api::models::primitives::DateTime;
use crate::oanda_api::models::transaction_related::{RequestID, TransactionID, TransactionRejectReason, TransactionType};

/// Represents the base Transaction specification.
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Transaction {
    /// The Transaction’s Identifier.
    id: TransactionID,

    /// The date/time when the Transaction was created.
    time: DateTime,

    /// The ID of the user that initiated the creation of the Transaction.
    #[serde(rename = "userID")]
    user_id: i32,

    /// The ID of the Account the Transaction was created for.
    #[serde(rename = "accountID")]
    account_id: AccountId,

    /// The ID of the “batch” that the Transaction belongs to. Transactions in
    /// the same batch are applied to the Account simultaneously.
    #[serde(rename = "batchID")]
    batch_id: TransactionID,

    /// The Request ID of the request which generated the transaction.
    #[serde(rename = "requestID")]
    request_id: RequestID,
}

/// Represents a CreateTransaction, extending the base Transaction.
#[derive(Serialize, Deserialize, Debug)]
struct CreateTransaction {
    /// Base Transaction fields.
    #[serde(flatten)]
    base: Transaction,

    /// The Type of the Transaction. Always set to “CREATE” in a CreateTransaction.
    #[serde(rename = "type", default = "default_create_transaction_type")]
    type_of: TransactionType,

    /// The ID of the Division that the Account is in.
    #[serde(rename = "divisionID")]
    division_id: i32,

    /// The ID of the Site that the Account was created at.
    #[serde(rename = "siteID")]
    site_id: i32,

    /// The ID of the user that the Account was created for.
    #[serde(rename = "accountUserID")]
    account_user_id: i32,

    /// The number of the Account within the site/division/user.
    #[serde(rename = "accountNumber")]
    account_number: i32,

    /// The home currency of the Account.
    #[serde(rename = "homeCurrency")]
    home_currency: Currency,
}

/// Provides a default value for the 'type' field of the `CreateTransaction` struct.
fn default_create_transaction_type() -> TransactionType {
    TransactionType::Create // Assuming this is a variant of the TransactionType enum
}

/// Represents a CloseTransaction, indicating the closing of an Account.
#[derive(Serialize, Deserialize, Debug)]
struct CloseTransaction {
    /// The Transaction’s Identifier.
    id: TransactionID,

    /// The date/time when the Transaction was created.
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

    /// The Type of the Transaction, always set to “CLOSE” in a CloseTransaction.
    #[serde(rename = "type", default = "default_close_transaction_type")]
    type_of: TransactionType,
}

/// Provides a default value for the 'type' field of the `CloseTransaction` struct.
fn default_close_transaction_type() -> TransactionType {
    TransactionType::Close // Assuming this is a variant of the TransactionType enum
}

/// Represents a ReopenTransaction, indicating the re-opening of a closed Account.
#[derive(Serialize, Deserialize, Debug)]
struct ReopenTransaction {
    /// The Transaction’s Identifier.
    id: TransactionID,

    /// The date/time when the Transaction was created.
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

    /// The Type of the Transaction, always set to “REOPEN” in a ReopenTransaction.
    #[serde(rename = "type", default = "default_reopen_transaction_type")]
    type_of: TransactionType,
}

/// Provides a default value for the 'type' field of the `ReopenTransaction` struct.
fn default_reopen_transaction_type() -> TransactionType {
    TransactionType::Reopen // Assuming this is a variant of the TransactionType enum
}

/// Represents a ClientConfigureTransaction, indicating the configuration of an Account by a client.
#[derive(Serialize, Deserialize, Debug)]
struct ClientConfigureTransaction {
    /// The Transaction’s Identifier.
    id: TransactionID,

    /// The date/time when the Transaction was created.
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

    /// The Type of the Transaction, always set to “CLIENT_CONFIGURE” in a ClientConfigureTransaction.
    #[serde(rename = "type", default = "default_client_configure_transaction_type")]
    type_of: TransactionType,

    /// The client-provided alias for the Account.
    alias: String,

    /// The margin rate override for the Account.
    #[serde(rename = "marginRate")]
    margin_rate: Decimal,
}

/// Provides a default value for the 'type' field of the `ClientConfigureTransaction` struct.
fn default_client_configure_transaction_type() -> TransactionType {
    TransactionType::ClientConfigure // Assuming this is a variant of the TransactionType enum
}

/// Represents a ClientConfigureRejectTransaction, indicating the rejection of configuration of an Account by a client.
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct ClientConfigureRejectTransaction {
    /// The Transaction’s Identifier.
    id: TransactionID,

    /// The date/time when the Transaction was created.
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

    /// The Type of the Transaction, always set to “CLIENT_CONFIGURE_REJECT” in a ClientConfigureRejectTransaction.
    #[serde(rename = "type", default = "default_client_configure_reject_transaction_type")]
    type_of: TransactionType,

    /// The client-provided alias for the Account.
    alias: String,

    /// The margin rate override for the Account.
    #[serde(rename = "marginRate")]
    margin_rate: Decimal,

    /// The reason that the Reject Transaction was created.
    #[serde(rename = "rejectReason")]
    reject_reason: TransactionRejectReason,
}

/// Provides a default value for the 'type' field of the `ClientConfigureRejectTransaction` struct.
fn default_client_configure_reject_transaction_type() -> TransactionType {
    TransactionType::ClientConfigureReject // Assuming this is a variant of the TransactionType enum
}

