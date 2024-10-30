use serde_json::Value;
use std::str::FromStr;
use rust_decimal::Decimal;
use serde_derive::{Deserialize, Serialize};
use ff_standard_lib::helpers::converters::fund_forge_formatted_symbol_name;
use ff_standard_lib::standardized_types::enums::MarketType;
use ff_standard_lib::standardized_types::subscriptions::SymbolName;
use crate::oanda_api::models::account::enums::GuaranteedStopLossOrderMode;

use crate::oanda_api::models::primitives::{GuaranteedStopLossOrderLevelRestriction, InstrumentCommission, InstrumentFinancing};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OandaInstrument {
    pub symbol_name: SymbolName,
    pub display_name: String,
    pub name: String,  // This is the API instrument name
    #[serde(rename = "type")]
    pub instrument_type: String,
    pub display_precision: u32,
    pub pip_location: u32,
    pub trade_units_precision: i32,
    pub minimum_trade_size: Decimal,
    pub maximum_trailing_stop_distance: Decimal,
    pub minimum_guaranteed_stop_loss_distance: Decimal,
    pub minimum_trailing_stop_distance: Decimal,
    pub maximum_position_size: Decimal,
    pub maximum_order_units: Decimal,
    pub margin_rate: Decimal,
    pub commission: InstrumentCommission,
    pub guaranteed_stop_loss_order_mode: GuaranteedStopLossOrderMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guaranteed_stop_loss_order_execution_premium: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guaranteed_stop_loss_order_level_restriction: Option<GuaranteedStopLossOrderLevelRestriction>,
    pub financing: InstrumentFinancing,
    pub tags: Vec<String>,
    pub market_type: MarketType,
}

impl OandaInstrument {
    pub fn from_json(instrument: &Value) -> Result<OandaInstrument, Box<dyn std::error::Error + Send + Sync>> {
        let name = instrument["name"].as_str().ok_or("Missing name")?.to_string();
        let instrument_type = instrument["type"].as_str().ok_or("Missing type")?.to_string();
        let display_name = instrument["displayName"].as_str().ok_or("Missing displayName")?.to_string();
        let pip_location = instrument["pipLocation"].as_i64().ok_or("Missing pipLocation")? as u32;
        let display_precision = instrument["displayPrecision"].as_i64().ok_or("Missing displayPrecision")? as u32;
        let trade_units_precision = instrument["tradeUnitsPrecision"].as_i64().ok_or("Missing tradeUnitsPrecision")? as i32;

        // Decimal conversions
        let minimum_trade_size = Decimal::from_str(instrument["minimumTradeSize"].as_str().ok_or("Missing minimumTradeSize")?)?;
        let maximum_trailing_stop_distance = Decimal::from_str(instrument["maximumTrailingStopDistance"].as_str().ok_or("Missing maximumTrailingStopDistance")?)?;
        let minimum_trailing_stop_distance = Decimal::from_str(instrument["minimumTrailingStopDistance"].as_str().ok_or("Missing minimumTrailingStopDistance")?)?;
        let maximum_position_size = Decimal::from_str(instrument["maximumPositionSize"].as_str().ok_or("Missing maximumPositionSize")?)?;
        let maximum_order_units = Decimal::from_str(instrument["maximumOrderUnits"].as_str().ok_or("Missing maximumOrderUnits")?)?;
        let margin_rate = Decimal::from_str(instrument["marginRate"].as_str().ok_or("Missing marginRate")?)?;

        // Default commission since it's not in the JSON
        let commission = InstrumentCommission {
            commission: Decimal::ZERO,
            units_traded: Decimal::ZERO,
            minimum_commission: Decimal::ZERO,
        };

        // Handle GuaranteedStopLossOrderMode
        let guaranteed_stop_loss_order_mode = match instrument["guaranteedStopLossOrderMode"].as_str() {
            Some("ALLOWED") => GuaranteedStopLossOrderMode::Allowed,
            _ => GuaranteedStopLossOrderMode::Disabled,
        };

        // Optional fields
        let guaranteed_stop_loss_order_execution_premium = instrument["guaranteedStopLossOrderExecutionPremium"]
            .as_str()
            .map(|s| Decimal::from_str(s))
            .transpose()?;

        // Parse tags
        let tags = instrument["tags"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|tag| tag["name"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let market_type = match instrument_type.as_str() {
            "CURRENCY" => MarketType::Forex,
            _ => MarketType::CFD,
        };

        // Format the symbol
        let symbol_name = fund_forge_formatted_symbol_name(&name);

        let minimum_guaranteed_stop_loss_distance = instrument["minimumGuaranteedStopLossDistance"]
            .as_str()
            .map(Decimal::from_str)
            .transpose()?
            .unwrap_or_default();

        let financing: InstrumentFinancing = serde_json::from_value(instrument["financing"].clone())?;

        Ok(OandaInstrument {
            display_name,
            name,
            symbol_name,
            instrument_type,
            display_precision,
            pip_location,
            trade_units_precision,
            minimum_trade_size,
            maximum_trailing_stop_distance,
            minimum_guaranteed_stop_loss_distance,
            minimum_trailing_stop_distance,
            maximum_position_size,
            maximum_order_units,
            margin_rate,
            commission,
            guaranteed_stop_loss_order_mode,
            guaranteed_stop_loss_order_execution_premium,
            guaranteed_stop_loss_order_level_restriction: None,
            financing,
            tags,
            market_type,
        })
    }
}
