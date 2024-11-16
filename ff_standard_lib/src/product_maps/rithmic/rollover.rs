use chrono::{DateTime, Datelike, TimeZone, Utc};
use chrono_tz::America::Chicago;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RolloverError {
    #[error("Symbol not recognized: {0}")]
    UnknownSymbol(String),
    #[error("Invalid month for rollover: {0}")]
    InvalidMonth(u32),
}

#[derive(Debug, Clone)]
struct ContractSpec {
    rollover_day: u32,
    is_quarterly: bool,
}

lazy_static::lazy_static! {
    static ref CONTRACT_SPECS: HashMap<&'static str, ContractSpec> = {
        let mut map = HashMap::new();

        // Energy
        map.insert("CL", ContractSpec { rollover_day: 18, is_quarterly: false }); // Crude Oil
        map.insert("HO", ContractSpec { rollover_day: 25, is_quarterly: false }); // Heating Oil
        map.insert("RB", ContractSpec { rollover_day: 25, is_quarterly: false }); // RBOB Gasoline
        map.insert("NG", ContractSpec { rollover_day: 28, is_quarterly: false }); // Natural Gas
        map.insert("MCL", ContractSpec { rollover_day: 18, is_quarterly: false }); // Micro Crude Oil
        map.insert("QM", ContractSpec { rollover_day: 18, is_quarterly: false }); // E-mini Crude Oil
        map.insert("QG", ContractSpec { rollover_day: 28, is_quarterly: false }); // E-mini Natural Gas

        // Metals
        map.insert("GC", ContractSpec { rollover_day: 26, is_quarterly: false }); // Gold
        map.insert("SI", ContractSpec { rollover_day: 25, is_quarterly: false }); // Silver
        map.insert("HG", ContractSpec { rollover_day: 28, is_quarterly: false }); // Copper
        map.insert("PA", ContractSpec { rollover_day: 25, is_quarterly: false }); // Palladium
        map.insert("PL", ContractSpec { rollover_day: 25, is_quarterly: false }); // Platinum
        map.insert("MGC", ContractSpec { rollover_day: 26, is_quarterly: false }); // Micro Gold
        map.insert("SIL", ContractSpec { rollover_day: 25, is_quarterly: false }); // Micro Silver

        // Equity Index
        map.insert("ES", ContractSpec { rollover_day: 9, is_quarterly: true }); // E-mini S&P 500
        map.insert("NQ", ContractSpec { rollover_day: 9, is_quarterly: true }); // E-mini NASDAQ-100
        map.insert("RTY", ContractSpec { rollover_day: 9, is_quarterly: true }); // E-mini Russell 2000
        map.insert("YM", ContractSpec { rollover_day: 9, is_quarterly: true }); // E-mini Dow
        map.insert("MES", ContractSpec { rollover_day: 9, is_quarterly: true }); // Micro E-mini S&P 500
        map.insert("MNQ", ContractSpec { rollover_day: 9, is_quarterly: true }); // Micro E-mini NASDAQ-100
        map.insert("M2K", ContractSpec { rollover_day: 9, is_quarterly: true }); // Micro E-mini Russell 2000
        map.insert("MYM", ContractSpec { rollover_day: 9, is_quarterly: true }); // Micro E-mini Dow

        // Interest Rates
        map.insert("ZN", ContractSpec { rollover_day: 21, is_quarterly: true }); // 10-Year T-Note
        map.insert("ZF", ContractSpec { rollover_day: 21, is_quarterly: true }); // 5-Year T-Note
        map.insert("ZB", ContractSpec { rollover_day: 21, is_quarterly: true }); // 30-Year T-Bond
        map.insert("ZT", ContractSpec { rollover_day: 21, is_quarterly: true }); // 2-Year T-Note
        map.insert("UB", ContractSpec { rollover_day: 21, is_quarterly: true }); // Ultra T-Bond
        map.insert("SR3", ContractSpec { rollover_day: 13, is_quarterly: true }); // 3-Month SOFR

        // Currencies
        map.insert("6E", ContractSpec { rollover_day: 9, is_quarterly: true }); // Euro FX
        map.insert("6B", ContractSpec { rollover_day: 9, is_quarterly: true }); // British Pound
        map.insert("6J", ContractSpec { rollover_day: 9, is_quarterly: true }); // Japanese Yen
        map.insert("6C", ContractSpec { rollover_day: 9, is_quarterly: true }); // Canadian Dollar
        map.insert("6A", ContractSpec { rollover_day: 9, is_quarterly: true }); // Australian Dollar
        map.insert("6N", ContractSpec { rollover_day: 9, is_quarterly: true }); // New Zealand Dollar
        map.insert("6S", ContractSpec { rollover_day: 9, is_quarterly: true }); // Swiss Franc
        map.insert("M6E", ContractSpec { rollover_day: 9, is_quarterly: true }); // Micro Euro FX
        map.insert("M6A", ContractSpec { rollover_day: 9, is_quarterly: true }); // Micro AUD/USD

        // Agricultural
        map.insert("ZC", ContractSpec { rollover_day: 15, is_quarterly: false }); // Corn
        map.insert("ZW", ContractSpec { rollover_day: 15, is_quarterly: false }); // Wheat
        map.insert("ZS", ContractSpec { rollover_day: 15, is_quarterly: false }); // Soybeans
        map.insert("ZM", ContractSpec { rollover_day: 15, is_quarterly: false }); // Soybean Meal
        map.insert("ZL", ContractSpec { rollover_day: 15, is_quarterly: false }); // Soybean Oil
        map.insert("KE", ContractSpec { rollover_day: 15, is_quarterly: false }); // KC Wheat
        map.insert("CT", ContractSpec { rollover_day: 7, is_quarterly: false });  // Cotton

        // VIX Products
        map.insert("VX", ContractSpec { rollover_day: 9, is_quarterly: false }); // VIX Futures

        map
    };
}

fn month_to_code(month: u32) -> Result<char, RolloverError> {
    match month {
        1 => Ok('F'),  2 => Ok('G'),  3 => Ok('H'),
        4 => Ok('J'),  5 => Ok('K'),  6 => Ok('M'),
        7 => Ok('N'),  8 => Ok('Q'),  9 => Ok('U'),
        10 => Ok('V'), 11 => Ok('X'), 12 => Ok('Z'),
        _ => Err(RolloverError::InvalidMonth(month)),
    }
}

fn get_next_month(current_month: u32, is_quarterly: bool) -> (u32, bool) {
    if is_quarterly {
        match current_month {
            3 => (6, false),   // March -> June
            6 => (9, false),   // June -> September
            9 => (12, false),  // September -> December
            12 => (3, true),   // December -> March (next year)
            1..=2 => (3, false),
            4..=5 => (6, false),
            7..=8 => (9, false),
            10..=11 => (12, false),
            _ => unreachable!(),
        }
    } else {
        if current_month == 12 {
            (1, true)  // December -> January (next year)
        } else {
            (current_month + 1, false)  // Normal monthly increment
        }
    }
}

pub fn get_front_month(symbol: &str, utc_time: DateTime<Utc>) -> Result<String, RolloverError> {
    let spec = CONTRACT_SPECS
        .get(symbol)
        .ok_or_else(|| RolloverError::UnknownSymbol(symbol.to_string()))?;

    // Convert to Chicago time
    let chicago_time = utc_time.with_timezone(&Chicago);
    let mut month = chicago_time.month();
    let mut year = chicago_time.year();

    // Special handling for quarterly contracts
    if spec.is_quarterly {
        if [3, 6, 9, 12].contains(&month) {
            // In delivery quarter - check rollover
            if chicago_time.day() >= spec.rollover_day {
                let (next_month, year_increment) = get_next_month(month, true);
                month = next_month;
                if year_increment {
                    year += 1;
                }
            }
        } else {
            // Not in delivery quarter - use next quarterly month
            let (next_month, year_increment) = get_next_month(month, true);
            month = next_month;
            if year_increment {
                year += 1;
            }
        }
    } else {
        // Monthly contracts
        // Special handling for year boundary (December)
        if month == 12 && chicago_time.day() >= spec.rollover_day {
            month = 1;
            year += 1;
        } else {
            // Get next month as base contract
            let (next_month, year_increment) = get_next_month(month, false);
            month = next_month;
            if year_increment {
                year += 1;
            }

            // If past rollover, get one more month
            if chicago_time.day() >= spec.rollover_day {
                let (next_month, year_increment) = get_next_month(month, false);
                month = next_month;
                if year_increment {
                    year += 1;
                }
            }
        }
    }

    // Format contract symbol
    let year_code = year % 100;
    let month_code = month_to_code(month)?;

    Ok(format!("{}{}{:02}", symbol, month_code, year_code))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn test_case(symbol: &str, date_str: &str, expected: &str) {
        let utc_time = Utc.datetime_from_str(date_str, "%Y-%m-%d %H:%M:%S")
            .expect("Invalid test date");
        let result = get_front_month(symbol, utc_time).unwrap();
        assert_eq!(
            result, expected,
            "Test failed for {} on {}. Expected {}, got {}",
            symbol, date_str, expected, result
        );
    }

    #[test]
    fn test_cl_rollover() {
        test_case("CL", "2024-01-17 14:30:00", "CLG24"); // February contract before rollover
        test_case("CL", "2024-01-18 14:30:00", "CLH24"); // March contract after rollover
    }

    #[test]
    fn test_gc_rollover() {
        test_case("GC", "2024-01-25 14:30:00", "GCG24"); // February contract before rollover
        test_case("GC", "2024-01-26 14:30:00", "GCH24"); // March contract after rollover
    }

    #[test]
    fn test_es_quarterly() {
        test_case("ES", "2024-03-08 14:30:00", "ESH24"); // March contract before rollover
        test_case("ES", "2024-03-09 14:30:00", "ESM24"); // June contract after rollover
    }

    #[test]
    fn test_year_boundary() {
        test_case("CL", "2024-12-18 14:30:00", "CLF25"); // January contract after December rollover
        test_case("ES", "2024-12-13 14:30:00", "ESH25"); // March contract after December rollover
    }
}