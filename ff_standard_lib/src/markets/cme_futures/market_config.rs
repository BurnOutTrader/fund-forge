use std::collections::HashMap;
use serde::{Deserialize, Deserializer};
use chrono_tz::Tz;
use crate::standardized_types::enums::FuturesExchange;
use crate::standardized_types::symbol_info::SymbolInfo;
use chrono::{DateTime, NaiveTime};
use crate::helpers::converters::resolve_market_datetime_in_timezone;

#[derive(Debug, Deserialize)]
struct MarketConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    symbols: HashMap<String, SymbolConfig>,
}

#[derive(Debug, Deserialize)]
struct SymbolConfig {
    #[serde(deserialize_with = "deserialize_exchange")]
    exchange: FuturesExchange,
    #[serde(with = "timezone_deserializer")]
    timezone: Tz,
    symbol_info: SymbolInfo,
    schedule: Schedule,
    special_dates: SpecialDates,
}

// Custom deserializer for FuturesExchange enum
fn deserialize_exchange<'de, D>(deserializer: D) -> Result<FuturesExchange, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    FuturesExchange::from_string(&s).map_err(serde::de::Error::custom)
}

// Module for timezone deserialization
mod timezone_deserializer {
    use super::*;
    use serde::{self, Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Tz, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<Tz>().map_err(serde::de::Error::custom)
    }
}

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

impl SpecialTime {
    pub fn time(&self, tz: Tz) -> DateTime<Tz> {
        let naive_date = chrono::NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").unwrap();
        let naive_time = NaiveTime::parse_from_str(&self.time, "%H:%M:%S").unwrap();
        let naive_dt = naive_date.and_time(naive_time);
        resolve_market_datetime_in_timezone(tz, naive_dt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::America::New_York;

    #[test]
    fn test_special_time_conversion() {
        let special_time = SpecialTime {
            date: "2024-01-15".to_string(),
            time: "13:00:00".to_string(),
        };

        let dt = special_time.time(New_York);
        assert_eq!(dt.format("%Y-%m-%d %H:%M:%S %z").to_string(), "2024-01-15 13:00:00 -0500");
    }

    #[test]
    fn test_dst_spring_forward() {
        // Test during spring forward DST transition
        let special_time = SpecialTime {
            date: "2024-03-10".to_string(), // DST starts
            time: "02:30:00".to_string(),   // This time doesn't exist
        };

        let dt = special_time.time(New_York);
        // Should be adjusted to 3:30 AM as 2:30 AM doesn't exist
        assert_eq!(dt.format("%Y-%m-%d %H:%M:%S %z").to_string(), "2024-03-10 03:30:00 -0400");
    }

    #[test]
    fn test_dst_fall_back() {
        // Test during fall back DST transition
        let special_time = SpecialTime {
            date: "2024-11-03".to_string(), // DST ends
            time: "01:30:00".to_string(),   // This time happens twice
        };

        let dt = special_time.time(New_York);
        // Should take the earlier time (EDT)
        assert_eq!(dt.format("%Y-%m-%d %H:%M:%S %z").to_string(), "2024-11-03 01:30:00 -0400");
    }

    #[test]
    fn test_normal_time() {
        let special_time = SpecialTime {
            date: "2024-07-15".to_string(),
            time: "14:30:00".to_string(),
        };

        let dt = special_time.time(New_York);
        assert_eq!(dt.format("%Y-%m-%d %H:%M:%S %z").to_string(), "2024-07-15 14:30:00 -0400");
    }
}