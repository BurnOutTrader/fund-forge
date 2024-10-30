use serde::{Deserialize, Serialize};
use ff_standard_lib::standardized_types::accounts::AccountId;
use crate::oanda_api::models::primitives::DateTime;
use crate::oanda_api::models::trade::TradeID;
use crate::oanda_api::models::transaction_related::{ClientExtensions, ClientID, RequestID, TransactionID, TransactionRejectReason, TransactionType};

/// Represents a transaction that modifies the client extensions of a trade.
#[derive(Serialize, Deserialize, Debug)]
pub struct TradeClientExtensionsModifyTransaction {
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

    /// The ID of the Trade whose client extensions are to be modified.
    #[serde(rename = "tradeID")]
    trade_id: TradeID,

    /// The original Client ID of the Trade whose client extensions are to be modified.
    #[serde(rename = "clientTradeID")]
    client_trade_id: ClientID,

    /// The new Client Extensions for the Trade.
    #[serde(rename = "tradeClientExtensionsModify")]
    trade_client_extensions_modify: ClientExtensions,
}

/// Represents a transaction that rejects the modification of a trade's client extensions.
#[derive(Serialize, Deserialize, Debug)]
pub struct TradeClientExtensionsModifyRejectTransaction {
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

    /// The ID of the Trade whose client extensions are to be modified.
    #[serde(rename = "tradeID")]
    trade_id: TradeID,

    /// The original Client ID of the Trade whose client extensions are to be modified.
    #[serde(rename = "clientTradeID")]
    client_trade_id: ClientID,

    /// The new Client Extensions for the Trade.
    #[serde(rename = "tradeClientExtensionsModify")]
    trade_client_extensions_modify: ClientExtensions,

    /// The reason that the Reject Transaction was created.
    #[serde(rename = "rejectReason")]
    reject_reason: TransactionRejectReason,
}

