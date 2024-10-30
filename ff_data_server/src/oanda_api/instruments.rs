use serde_json::Value;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use std::str::FromStr;
use rust_decimal::Decimal;
use ff_standard_lib::helpers::converters::fund_forge_formatted_symbol_name;
use ff_standard_lib::standardized_types::enums::MarketType;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
/// The `OandaInstrument` struct is used to represent the instrument data from the Oanda API.
/// This is used to get the maximum benefit of the Oanda API but also treat all instruments in the same way in the engine.
///
/// # Parameters
/// * `symbol` - The trading symbol of the asset in fund forge format.
/// * `display_precision` - The number of decimal places that should be displayed for the instrument.
/// * `margin_rate` - The margin rate for the instrument.
/// * `maximum_order_units` - The maximum number of units that can be traded for the instrument.
/// * `maximum_position_size` - The maximum position size for the instrument.
/// * `maximum_trailing_stop_distance` - The maximum trailing stop distance for the instrument.
/// * `minimum_trade_size` - The minimum trade size for the instrument.
/// * `minimum_trailing_stop_distance` - The minimum trailing stop distance for the instrument.
/// * `instrument` - The trading symbol of the asset in the data vendor format.
/// * `pip_location` - The location of the pip for the instrument.
/// * `trade_units_precision` - The number of decimal places that should be used for the trade units.
/// * `instrument_type` - The type of the instrument.
pub struct OandaInstrument {
    pub display_name: String,
    pub symbol: String,
    pub display_precision: u32,
    pub margin_rate: Decimal,
    pub maximum_order_units: u64,
    pub maximum_position_size: u64,
    pub maximum_trailing_stop_distance: Decimal,
    pub minimum_trade_size: u64,
    pub minimum_trailing_stop_distance: Decimal,
    pub instrument_name: String,
    pub pip_location: u32,
    pub trade_units_precision: i32,
    pub market_type: MarketType,
}

impl OandaInstrument {
    pub fn from_json(instrument: &Value) -> Result<OandaInstrument, Box<dyn std::error::Error + Send + Sync>> {
        let symbol = instrument["name"].as_str().ok_or("Missing displayName")?.to_string();
        let display_precision = instrument["displayPrecision"].as_i64().ok_or("Missing displayPrecision")? as u32;
        let margin_rate = Decimal::from_str(instrument["marginRate"].as_str().ok_or("Missing marginRate")?)?;
        let maximum_order_units = u64::from_str(instrument["maximumOrderUnits"].as_str().ok_or("Missing maximumOrderUnits")?)?;
        let maximum_position_size = u64::from_str(instrument["maximumPositionSize"].as_str().ok_or("Missing maximumPositionSize")?)?;
        let maximum_trailing_stop_distance = Decimal::from_str(instrument["maximumTrailingStopDistance"].as_str().ok_or("Missing maximumTrailingStopDistance")?)?;
        let minimum_trade_size = u64::from_str(instrument["minimumTradeSize"].as_str().ok_or("Missing minimumTradeSize")?)?;
        let minimum_trailing_stop_distance = Decimal::from_str(instrument["minimumTrailingStopDistance"].as_str().ok_or("Missing minimumTrailingStopDistance")?)?;
        let display_name = instrument["displayName"].as_str().ok_or("Missing name")?.to_string();
        let pip_location = instrument["pipLocation"].as_i64().ok_or("Missing pipLocation")? as u32;
        let trade_units_precision = instrument["tradeUnitsPrecision"].as_i64().ok_or("Missing tradeUnitsPrecision")? as i32;
        let instrument_type = instrument["type"].as_str().ok_or("Missing type")?.to_string();
        
        let market_type: MarketType = match instrument_type.as_str() {
            "CURRENCY" => MarketType::Forex,
            _=> MarketType::CFD,
        };
        
        // the name property is the same property we use with teh api to get the instrument data
        let instrument_name =  instrument["name"].as_str().ok_or("Missing displayName")?.to_string();
        let symbol = fund_forge_formatted_symbol_name(&symbol);

        Ok(OandaInstrument {
            display_name,
            symbol,
            display_precision,
            margin_rate,
            maximum_order_units,
            maximum_position_size,
            maximum_trailing_stop_distance,
            minimum_trade_size,
            minimum_trailing_stop_distance,
            instrument_name,
            pip_location,
            trade_units_precision,
            market_type: market_type,
        })
    }
}
