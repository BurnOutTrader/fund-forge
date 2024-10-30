use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use ff_standard_lib::standardized_types::accounts::AccountId;
use crate::oanda_api::models::primitives::{DateTime, HomeConversionFactors, InstrumentName};
use crate::oanda_api::models::transaction_related::{OpenTradeDividendAdjustment, RequestID, TransactionID, TransactionType};

/// Represents a transaction for paying or collecting dividend adjustment amounts to or from an Account.
#[derive(Serialize, Deserialize, Debug)]
pub struct DividendAdjustmentTransaction {
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

    /// The name of the instrument for the dividendAdjustment transaction.
    #[serde(rename = "instrument")]
    instrument: InstrumentName,

    /// The total dividend adjustment amount paid or collected in the Account’s home currency.
    #[serde(rename = "dividendAdjustment")]
    dividend_adjustment: Decimal,

    /// The total dividend adjustment amount paid or collected in the Instrument’s quote currency.
    #[serde(rename = "quoteDividendAdjustment")]
    quote_dividend_adjustment: Decimal,

    /// The HomeConversionFactors in effect at the time of the DividendAdjustment.
    #[serde(rename = "homeConversionFactors")]
    home_conversion_factors: HomeConversionFactors,

    /// The Account balance after applying the DividendAdjustment Transaction.
    #[serde(rename = "accountBalance")]
    account_balance: Decimal,

    /// The dividend adjustment payment/collection details for each open Trade.
    #[serde(rename = "openTradeDividendAdjustments")]
    open_trade_dividend_adjustments: Vec<OpenTradeDividendAdjustment>,
}