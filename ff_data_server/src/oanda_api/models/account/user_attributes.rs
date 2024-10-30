use serde::{Deserialize, Serialize};
use ff_standard_lib::standardized_types::accounts::Currency;

/// Contains the attributes of a user.
#[derive(Debug, Serialize, Deserialize)]
struct UserAttributes {
    /// The user’s OANDA-assigned user ID.
    #[serde(rename = "userID")]
    user_id: i64,

    /// The user-provided username.
    #[serde(rename = "username")]
    username: String,

    /// The user’s title.
    #[serde(rename = "title")]
    title: String,

    /// The user’s name.
    #[serde(rename = "name")]
    name: String,

    /// The user’s email address.
    #[serde(rename = "email")]
    email: String,

    /// The OANDA division the user belongs to.
    #[serde(rename = "divisionAbbreviation")]
    division_abbreviation: String,

    /// The user’s preferred language.
    #[serde(rename = "languageAbbreviation")]
    language_abbreviation: String,

    /// The home currency of the Account.
    #[serde(rename = "homeCurrency")]
    home_currency: Currency,
}