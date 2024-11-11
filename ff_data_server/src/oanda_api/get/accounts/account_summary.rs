use ff_standard_lib::messages::data_server_messaging::FundForgeError;
use serde_derive::{Deserialize, Serialize};
use ff_standard_lib::standardized_types::accounts::{AccountId, Currency};
use rust_decimal::Decimal;
use crate::oanda_api::api_client::OandaClient;
use crate::oanda_api::models::account::enums::{GuaranteedStopLossOrderMode, GuaranteedStopLossOrderMutability};
use crate::oanda_api::models::account::guaranteed_stop_loss_parameters::GuaranteedStopLossOrderParameters;
use crate::oanda_api::models::primitives::DateTime;
use crate::oanda_api::models::transaction_related::TransactionID;

pub(crate) async fn get_oanda_account_summary(oanda_client: &OandaClient, account_id: &str) -> Result<AccountSummary, FundForgeError> {
    let request_uri = format!("/accounts/{}/summary", account_id);

    let response = match oanda_client.send_rest_request(&request_uri).await {
        Ok(response) => response,
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(
                format!("Failed to get_requests account summary from server: {:?}", e)
            ));
        }
    };

    let content = match response.text().await {
        Ok(content) => content,
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(
                format!("Failed to read response content: {:?}", e)
            ));
        }
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(json) => json,
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(
                format!("Failed to parse JSON response: {:?}", e)
            ));
        }
    };

    // Assuming AccountSummary implements Deserialize
    match serde_json::from_value(json["account"].clone()) {
        Ok(summary) => Ok(summary),
        Err(e) => Err(FundForgeError::ServerErrorDebug(
            format!("Failed to deserialize account summary: {:?}", e)
        ))
    }
}

/// A summary representation of a client's Account.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AccountSummary {
    /// The Account's identifier.
    #[serde(rename = "id")]
    pub id: AccountId,

    /// Client-assigned alias for the Account. Only provided if the Account has an alias set.
    #[serde(rename = "alias")]
    pub alias: String,

    /// The home currency of the Account.
    #[serde(rename = "currency")]
    pub currency: Currency,

    /// ID of the user that created the Account.
    #[serde(rename = "createdByUserID")]
    pub created_by_user_id: i64,

    /// The date/time when the Account was created.
    #[serde(rename = "createdTime")]
    pub created_time: DateTime,

    /// The current guaranteed Stop Loss Order settings of the Account.
    /// This field will only be present if the guaranteedStopLossOrderMode is not 'DISABLED'.
    #[serde(rename = "guaranteedStopLossOrderParameters", skip_serializing_if = "Option::is_none")]
    pub guaranteed_stop_loss_order_parameters: Option<GuaranteedStopLossOrderParameters>,

    /// The current guaranteed Stop Loss Order mode of the Account.
    #[serde(rename = "guaranteedStopLossOrderMode")]
    pub guaranteed_stop_loss_order_mode: GuaranteedStopLossOrderMode,

    /// The current guaranteed Stop Loss Order mutability setting of the Account.
    /// This field will only be present if the guaranteedStopLossOrderMode is not 'DISABLED'.
    #[serde(rename = "guaranteedStopLossOrderMutability")]
    pub guaranteed_stop_loss_order_mutability: Option<GuaranteedStopLossOrderMutability>,

    /// The date/time that the Account's resettablePL was last reset.
    #[serde(rename = "resettablePLTime")]
    pub resettable_pl_time: DateTime,

    /// Client-provided margin rate override for the Account.
    /// The effective margin rate of the Account is the lesser of this value and the OANDA
    /// margin rate for the Account's division. This value is only provided if a margin rate
    /// override exists for the Account.
    #[serde(rename = "marginRate")]
    pub margin_rate: Option<Decimal>,

    /// The number of Trades currently open in the Account.
    #[serde(rename = "openTradeCount")]
    pub open_trade_count: i32,

    /// The number of Positions currently open in the Account.
    #[serde(rename = "openPositionCount")]
    pub open_position_count: i32,

    /// The number of Orders currently pending in the Account.
    #[serde(rename = "pendingOrderCount")]
    pub pending_order_count: i32,

    /// Flag indicating that the Account has hedging enabled.
    #[serde(rename = "hedgingEnabled")]
    pub hedging_enabled: bool,

    /// The total unrealized profit/loss for all Trades currently open in the Account.
    #[serde(rename = "unrealizedPL")]
    pub unrealized_pl: Decimal,

    /// The net asset value of the Account. Equal to Account balance + unrealizedPL.
    #[serde(rename = "NAV")]
    pub nav: Decimal,

    /// Margin currently used for the Account.
    #[serde(rename = "marginUsed")]
    pub margin_used: Decimal,

    /// Margin available for Account currency.
    #[serde(rename = "marginAvailable")]
    pub margin_available: Decimal,

    /// The value of the Account's open positions represented in the Account's home currency.
    #[serde(rename = "positionValue")]
    pub position_value: Decimal,

    /// The Account's margin closeout unrealized PL.
    #[serde(rename = "marginCloseoutUnrealizedPL")]
    pub margin_closeout_unrealized_pl: Decimal,

    /// The Account's margin closeout NAV.
    #[serde(rename = "marginCloseoutNAV")]
    pub margin_closeout_nav: Decimal,

    /// The Account's margin closeout margin used.
    #[serde(rename = "marginCloseoutMarginUsed")]
    pub margin_closeout_margin_used: Decimal,

    /// The Account's margin closeout percentage.
    /// When this value is 1.0 or above, the Account is in a margin closeout situation.
    #[serde(rename = "marginCloseoutPercent")]
    pub margin_closeout_percent: Decimal,

    /// The value of the Account's open positions as used for margin closeout calculations
    /// represented in the Account's home currency.
    #[serde(rename = "marginCloseoutPositionValue")]
    pub margin_closeout_position_value: Decimal,

    /// The current WithdrawalLimit for the account which will be zero or a positive value
    /// indicating how much can be withdrawn from the account.
    #[serde(rename = "withdrawalLimit")]
    pub withdrawal_limit: Decimal,

    /// The Account's margin call margin used.
    #[serde(rename = "marginCallMarginUsed")]
    pub margin_call_margin_used: Decimal,

    /// The Account's margin call percentage.
    /// When this value is 1.0 or above, the Account is in a margin call situation.
    #[serde(rename = "marginCallPercent")]
    pub margin_call_percent: Decimal,

    /// The current balance of the account.
    #[serde(rename = "balance")]
    pub balance: Decimal,

    /// The total profit/loss realized over the lifetime of the Account.
    #[serde(rename = "pl")]
    pub pl: Decimal,

    /// The total realized profit/loss for the account since it was last reset by the client.
    #[serde(rename = "resettablePL")]
    pub resettable_pl: Decimal,

    /// The total amount of financing paid/collected over the lifetime of the account.
    #[serde(rename = "financing")]
    pub financing: Decimal,

    /// The total amount of commission paid over the lifetime of the Account.
    #[serde(rename = "commission")]
    pub commission: Decimal,

    /// The total amount of dividend adjustment paid over the lifetime of the Account in
    /// the Account's home currency.
    #[serde(rename = "dividendAdjustment")]
    pub dividend_adjustment: Decimal,

    /// The total amount of fees charged over the lifetime of the Account for the execution
    /// of guaranteed Stop Loss Orders.
    #[serde(rename = "guaranteedExecutionFees")]
    pub guaranteed_execution_fees: Decimal,

    /// The date/time when the Account entered a margin call state. Only provided if the
    /// Account is in a margin call.
    #[serde(rename = "marginCallEnterTime")]
    pub margin_call_enter_time: Option<DateTime>,

    /// The number of times that the Account's current margin call was extended.
    #[serde(rename = "marginCallExtensionCount", skip_serializing_if = "Option::is_none")]
    pub margin_call_extension_count: Option<i32>,

    /// The date/time of the Account's last margin call extension.
    #[serde(rename = "lastMarginCallExtensionTime")]
    pub last_margin_call_extension_time: Option<DateTime>,

    /// The ID of the last Transaction created for the Account.
    #[serde(rename = "lastTransactionID")]
    pub last_transaction_id: TransactionID,
}