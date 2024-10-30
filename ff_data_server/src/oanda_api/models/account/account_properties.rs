use serde::{Deserialize, Serialize};
use ff_standard_lib::standardized_types::accounts::AccountId;

/// Represents properties related to an Account.
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountProperties {
    /// The Account's identifier.
    #[serde(rename = "id")]
    id: AccountId,

    /// The Account's associated MT4 Account ID.
    /// This field will not be present if the Account is not an MT4 account.
    #[serde(rename = "mt4AccountID")]
    mt4_account_id: Option<i32>,

    /// The Account's tags.
    #[serde(rename = "tags")]
    tags: Vec<String>,
}