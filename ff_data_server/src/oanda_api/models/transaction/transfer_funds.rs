use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use ff_standard_lib::standardized_types::accounts::AccountId;
use crate::oanda_api::models::primitives::{DateTime};
use crate::oanda_api::models::transaction_related::{FundingReason, RequestID, TransactionID, TransactionRejectReason, TransactionType};

/// Represents a TransferFundsTransaction, indicating the transfer of funds in/out of an Account.
#[derive(Serialize, Deserialize, Debug)]
struct TransferFundsTransaction {
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

    /// The Type of the Transaction, always set to “TRANSFER_FUNDS” in a TransferFundsTransaction.
    #[serde(rename = "type", default = "default_transfer_funds_transaction_type")]
    type_of: TransactionType,

    /// The amount to deposit/withdraw from the Account in the Account’s home currency.
    /// A positive value indicates a deposit, a negative value indicates a withdrawal.
    amount: Decimal,

    /// The reason that an Account is being funded.
    #[serde(rename = "fundingReason")]
    funding_reason: FundingReason,

    /// An optional comment that may be attached to a fund transfer for audit purposes.
    comment: String,

    /// The Account’s balance after funds are transferred.
    #[serde(rename = "accountBalance")]
    account_balance: Decimal,
}

/// Provides a default value for the 'type_of' field of the `TransferFundsTransaction` struct.
fn default_transfer_funds_transaction_type() -> TransactionType {
    TransactionType::TransferFunds // Assuming this is a variant of the TransactionType enum
}

/// Represents a TransferFundsRejectTransaction, indicating the rejection of the transfer of funds in/out of an Account.
#[derive(Serialize, Deserialize, Debug)]
struct TransferFundsRejectTransaction {
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

    /// The Type of the Transaction, always set to “TRANSFER_FUNDS_REJECT” in a TransferFundsRejectTransaction.
    #[serde(rename = "type", default = "default_transfer_funds_reject_transaction_type")]
    type_of: TransactionType,

    /// The amount to deposit/withdraw from the Account in the Account’s home currency.
    /// A positive value indicates a deposit, a negative value indicates a withdrawal.
    amount: Decimal,

    /// The reason that an Account is being funded.
    #[serde(rename = "fundingReason")]
    funding_reason: FundingReason,

    /// An optional comment that may be attached to a fund transfer for audit purposes.
    comment: String,

    /// The reason that the Reject Transaction was created.
    #[serde(rename = "rejectReason")]
    reject_reason: TransactionRejectReason,
}

/// Provides a default value for the 'type_of' field of the `TransferFundsRejectTransaction` struct.
fn default_transfer_funds_reject_transaction_type() -> TransactionType {
    TransactionType::TransferFundsReject // Assuming this is a variant of the TransactionType enum
}


