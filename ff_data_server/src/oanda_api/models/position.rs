use std::str::FromStr;
use chrono::Utc;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Serialize, Deserialize, Deserializer};
use serde_json::Value;
use uuid::Uuid;
use ff_standard_lib::helpers::converters::fund_forge_formatted_symbol_name;
use ff_standard_lib::product_maps::oanda::maps::OANDA_SYMBOL_INFO;
use ff_standard_lib::standardized_types::accounts::{Account};
use ff_standard_lib::standardized_types::position::Position;
use crate::oanda_api::models::primitives::{InstrumentName};
use crate::oanda_api::models::transaction_related::TransactionID;


/// A filter that can be used when fetching Transactions.
#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionFilter {
    /// Type of Transactions to filter.
    #[serde(rename = "type")]
    pub type_of: String,

    /// The ID of the most recent Transaction created for the Account.
    #[serde(rename = "lastTransactionID")]
    pub last_transaction_id: TransactionID,

    /// The date/time when the TransactionHeartbeat was created.
    pub time: String,
}

/// A TransactionHeartbeat object is injected into the Transaction stream to ensure that the HTTP connection remains active.
#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionHeartbeat {
    /// The string “HEARTBEAT”.
    #[serde(rename = "type")]
    pub type_of: String,

    /// The ID of the most recent Transaction created for the Account.
    #[serde(rename = "lastTransactionID")]
    pub last_transaction_id: TransactionID,

    /// The date/time when the TransactionHeartbeat was created.
    pub time: String,
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct PositionSide {
    pub units: Decimal,
    pub pl: Decimal,
    pub resettable_pl: Decimal,
    pub unrealized_pl: Decimal,
    pub financing: Decimal,
    pub dividend_adjustment: Decimal,
    pub guaranteed_execution_fees: Decimal,
    pub average_price: Option<Decimal>,
    pub trade_ids: Vec<String>,
}

impl<'de> Deserialize<'de> for PositionSide {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;

        Ok(PositionSide {
            units: value.get("units")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .unwrap_or_default(),
            pl: value.get("pl")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .unwrap_or_default(),
            resettable_pl: value.get("resettablePL")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .unwrap_or_default(),
            unrealized_pl: value.get("unrealizedPL")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .unwrap_or_default(),
            financing: value.get("financing")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .unwrap_or_default(),
            dividend_adjustment: value.get("dividendAdjustment")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .unwrap_or_default(),
            guaranteed_execution_fees: value.get("guaranteedExecutionFees")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .unwrap_or_default(),
            average_price: value.get("averagePrice")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok()),
            trade_ids: value.get("tradeIDs")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect())
                .unwrap_or_default(),
        })
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct OandaPosition {
    pub instrument: String,
    #[serde(default)]
    pub pl: Decimal,
    #[serde(default, rename = "resettablePL")]
    pub resettable_pl: Decimal,
    #[serde(default, rename = "unrealizedPL")]
    pub unrealized_pl: Decimal,
    #[serde(default, rename = "marginUsed")]
    pub margin_used: Decimal,
    #[serde(default)]
    pub commission: Decimal,
    #[serde(default, rename = "dividendAdjustment")]
    pub dividend_adjustment: Decimal,
    #[serde(default)]
    pub financing: Decimal,
    #[serde(default, rename = "guaranteedExecutionFees")]
    pub guaranteed_execution_fees: Decimal,
    pub long: PositionSide,
    pub short: PositionSide,
}

/// The dynamic (calculated) state of a Position.
#[derive(Serialize, Deserialize, Debug)]
pub struct CalculatedPositionState {
    /// The Position’s Instrument.
    pub instrument: InstrumentName,

    /// The Position’s net unrealized profit/loss.
    #[serde(rename = "netUnrealizedPL")]
    pub net_unrealized_pl: Decimal,

    /// The unrealized profit/loss of the Position’s long open Trades.
    #[serde(rename = "longUnrealizedPL")]
    pub long_unrealized_pl: Decimal,

    /// The unrealized profit/loss of the Position’s short open Trades.
    #[serde(rename = "shortUnrealizedPL")]
    pub short_unrealized_pl: Decimal,

    /// Margin currently used by the Position.
    #[serde(rename = "marginUsed")]
    pub margin_used: Decimal,
}


pub(crate) fn parse_oanda_position(position: OandaPosition, account: Account) -> Option<Position> {
    let symbol_name = fund_forge_formatted_symbol_name(&position.instrument);
    let (side, quantity, average_price, open_pnl) = match position.long.units > dec!(0) {
        true => (
            ff_standard_lib::standardized_types::enums::PositionSide::Long,
            position.long.units,
            position.long.average_price.unwrap_or_default(),
            position.long.unrealized_pl,
        ),
        false => (
            ff_standard_lib::standardized_types::enums::PositionSide::Short,
            position.short.units,
            position.short.average_price.unwrap_or_default(),
            position.short.unrealized_pl,
        ),
    };
    let symbol_info = match OANDA_SYMBOL_INFO.get(&symbol_name) {
        Some(info) => info.clone(),
        None => return None
    };

    let mut position = Position::new(
        symbol_name.clone(),
        symbol_name,
        account.clone(),
        side,
        quantity,
        average_price,
        Uuid::new_v4().to_string(),
        symbol_info.clone(),
        dec!(1),
        "Existing Order".to_string(),
        Utc::now()
    );
    position.open_pnl = open_pnl;
    Some(position)
}
