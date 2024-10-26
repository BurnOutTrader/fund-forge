use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::str::FromStr;
use chrono_tz::Tz;
use crate::standardized_types::enums::FuturesExchange;

/*#[derive(Debug, Deserialize)]
struct MarketConfig {
    #[serde(flatten)]
    symbols: HashMap<String, SymbolConfig>,
}*/

/*#[derive(Debug, Deserialize)]
struct SymbolConfig {
    #[serde(deserialize_with = "deserialize_exchange")]
    exchange: FuturesExchange,
    #[serde(deserialize_with = "deserialize_timezone")]
    timezone: Tz,
    schedule: Schedule,
    special_dates: SpecialDates,
}*/

#[derive(Debug, Deserialize)]
struct Schedule {
    pre_market: Vec<DaySchedule>,
    regular: Vec<DaySchedule>,
    post_market: Vec<DaySchedule>,
}

#[derive(Debug, Deserialize)]
struct DaySchedule {
    weekday: String,
    sessions: Vec<[String; 2]>,
}

#[derive(Debug, Deserialize)]
struct SpecialDates {
    holidays: Vec<String>,
    early_closes: Vec<SpecialTime>,
    late_opens: Vec<SpecialTime>,
}

#[derive(Debug, Deserialize)]
struct SpecialTime {
    date: String,
    time: String,
}
/*
// Custom deserializer for FuturesExchange enum
fn deserialize_exchange<'de, D>(deserializer: D) -> Result<FuturesExchange, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    FuturesExchange::from_str(&s).map_err(serde::de::Error::custom)
}

// Custom deserializer for chrono_tz::Tz
fn deserialize_timezone<'de, D>(deserializer: D) -> Result<Tz, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    s.parse::<Tz>().map_err(serde::de::Error::custom)
}*/