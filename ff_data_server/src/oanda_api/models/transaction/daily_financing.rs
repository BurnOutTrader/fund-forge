use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use ff_standard_lib::standardized_types::accounts::AccountId;
use crate::oanda_api::models::account::enums::AccountFinancingMode;
use crate::oanda_api::models::primitives::{DateTime};
use crate::oanda_api::models::transaction_related::{PositionFinancing, RequestID, TransactionID, TransactionType};

/// Represents a transaction for the daily payment/collection of financing for an Account.
#[derive(Serialize, Deserialize, Debug)]
pub struct DailyFinancingTransaction {
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

    /// The amount of financing paid/collected for the Account.
    #[serde(rename = "financing")]
    financing: Decimal,

    /// The Account’s balance after daily financing.
    #[serde(rename = "accountBalance")]
    account_balance: Decimal,

    /// Deprecated: Will be removed in a future API update.
    #[serde(rename = "accountFinancingMode")]
    account_financing_mode: AccountFinancingMode,

    /// The financing paid/collected for each Position in the Account.
    #[serde(rename = "positionFinancings")]
    position_financings: Vec<PositionFinancing>,
}

