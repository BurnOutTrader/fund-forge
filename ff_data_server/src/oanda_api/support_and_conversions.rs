use std::fmt;
use std::str::FromStr;
use chrono::{DateTime, Duration, Utc};
use structopt::StructOpt;
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::{CandleType, Symbol, SymbolName};
use serde_json::Value;
use ff_standard_lib::standardized_types::base_data::quotebar::QuoteBar;
use rust_decimal::Decimal;
use ff_standard_lib::standardized_types::base_data::candle::Candle;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal_macros::dec;

///The resolutions supported by Oanda Brokerage oanda
#[derive(PartialEq, Ord, PartialOrd, Eq, Clone, Debug, StructOpt)]
pub(crate) enum Interval {
    S5,
    M1,
    H1,
}

impl Interval {
    #[allow(dead_code)]
    pub(crate) fn oanda_interval_to_resolution(&self) -> Resolution {
        match self {
            Interval::S5 => Resolution::Seconds(5),
            Interval::M1 => Resolution::Minutes(1),
            Interval::H1 => Resolution::Hours(1),
        }
    }
}

pub(crate) fn resolution_to_oanda_interval(resolution: &Resolution) -> Option<Interval> {
    match resolution {
        Resolution::Seconds(5) => Some(Interval::S5),
        Resolution::Minutes(1) => Some(Interval::M1),
        Resolution::Hours(1) => Some(Interval::H1),
        _ => None,
    }
}

pub(crate) fn add_time_to_date(resolution: &Interval) -> Duration {
    match resolution {
        Interval::S5 => Duration::hours(2),
        Interval::M1 => Duration::days(2),
        Interval::H1 => Duration::days(50),
    }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            Interval::S5 => "S5",
            Interval::M1 => "M1",
            Interval::H1 => "H1",
        })
    }
}

impl FromStr for Interval {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "S5" => Ok(Interval::S5),
            "M1" => Ok(Interval::M1),
            "H1" => Ok(Interval::H1),
            _ => Err(format!("{} is not a valid resolution", s)),
        }
    }
}

pub(crate) async fn oanda_clean_instrument(symbol_name: &SymbolName) -> SymbolName {
    symbol_name
        .replace("/", "_")
        .replace(":", "_")
        .replace("?", "_")
        .replace("-", "_")
        .replace(" ", "_")
        .to_uppercase()
}

pub(crate) fn oanda_quotebar_from_candle(candle: &Value, symbol: Symbol, resolution: Resolution) -> Result<QuoteBar, Box<dyn std::error::Error + Send + Sync>> {
    //println!("candle: {:?}", candle);
    let bid = candle["bid"].as_object().ok_or("Missing bid data")?;
    let ask = candle["ask"].as_object().ok_or("Missing ask data")?;

    let bid_high = Decimal::from_str(bid["h"].as_str().ok_or("Missing bid high")?)?;
    let bid_low = Decimal::from_str(bid["l"].as_str().ok_or("Missing bid low")?)?;
    let bid_open = Decimal::from_str(bid["o"].as_str().ok_or("Missing bid open")?)?;
    let bid_close = Decimal::from_str(bid["c"].as_str().ok_or("Missing bid close")?)?;
    let ask_high = Decimal::from_str(ask["h"].as_str().ok_or("Missing ask high")?)?;
    let ask_low = Decimal::from_str(ask["l"].as_str().ok_or("Missing ask low")?)?;
    let ask_open = Decimal::from_str(ask["o"].as_str().ok_or("Missing ask open")?)?;
    let ask_close = Decimal::from_str(ask["c"].as_str().ok_or("Missing ask close")?)?;

    // Changed to handle volume as a number
    let volume = candle["volume"].as_f64()
        .ok_or("Missing volume")?;
    let volume = Decimal::from_f64(volume)
        .ok_or("Failed to convert volume to Decimal")?;

    let time_str = candle["time"].as_str().ok_or("Missing time")?;
    let time = parse_oanda_time(time_str)?;

    Ok(QuoteBar::from_closed(
        symbol,
        bid_high,
        bid_low,
        bid_open,
        bid_close,
        ask_high,
        ask_low,
        ask_open,
        ask_close,
        volume,
        dec!(0),
        dec!(0),
        time,
        resolution,
        CandleType::CandleStick
    ))
}

fn parse_oanda_time(time: &str) -> Result<DateTime<Utc>, Box<dyn std::error::Error + Send + Sync>> {
    // Since the format is ISO 8601, we can use chrono `DateTime::parse_from_rfc3339`
    // which supports parsing the Zulu time (Z) directly.
    let parsed_time = DateTime::parse_from_rfc3339(time)?;
    //eprintln!("parsed_time: {:?}", parsed_time);

    // Convert the parsed DateTime to UTC
    Ok(parsed_time.with_timezone(&Utc))
}

pub fn candle_from_candle(candle: &Value, symbol: Symbol, resolution: Resolution) -> Result<Candle, Box<dyn std::error::Error + Send + Sync>> {
    let mid = candle["mid"].as_object().ok_or("Missing mid data")?;
    let high = Decimal::from_str(mid["h"].as_str().ok_or("Missing mid high")?)?;
    let low = Decimal::from_str(mid["l"].as_str().ok_or("Missing mid low")?)?;
    let open = Decimal::from_str(mid["o"].as_str().ok_or("Missing mid open")?)?;
    let close = Decimal::from_str(mid["c"].as_str().ok_or("Missing mid close")?)?;
    let time_str = candle["time"].as_str().ok_or("Missing time")?;
    let time = parse_oanda_time(time_str).unwrap();
    let volume = Decimal::from_str(candle["volume"].as_str().ok_or("Missing volume")?)?;

    Ok(Candle::from_closed(symbol, high, low, open, close, volume, dec!(0), dec!(0), time, resolution, CandleType::CandleStick))
}