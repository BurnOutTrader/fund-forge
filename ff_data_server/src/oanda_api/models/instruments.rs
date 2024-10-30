use rust_decimal::Decimal;
use std::fmt;
use std::str::FromStr;
use chrono::Duration;
use serde::{Serialize, Deserialize};
use crate::oanda_api::models::primitives::{DateTime, InstrumentName};

/// Summary: The granularity of a candlestick
#[derive(Serialize, Deserialize, PartialEq, Ord, PartialOrd, Eq, Clone, Debug)]
pub enum CandlestickGranularity {
    S5,
    S10,
    S15,
    S30,
    M1,
    M2,
    M4,
    M5,
    M10,
    M15,
    M30,
    H1,
    H2,
    H3,
    H4,
    H6,
    H8,
    H12,
    D,
    W,
    M
}

impl FromStr for CandlestickGranularity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "S5" => Ok(CandlestickGranularity::S5),
            "S10" => Ok(CandlestickGranularity::S10),
            "S15" => Ok(CandlestickGranularity::S15),
            "S30" => Ok(CandlestickGranularity::S30),
            "M1" => Ok(CandlestickGranularity::M1),
            "M2" => Ok(CandlestickGranularity::M2),
            "M4" => Ok(CandlestickGranularity::M4),
            "M5" => Ok(CandlestickGranularity::M5),
            "M10" => Ok(CandlestickGranularity::M10),
            "M15" => Ok(CandlestickGranularity::M15),
            "M30" => Ok(CandlestickGranularity::M30),
            "H1" => Ok(CandlestickGranularity::H1),
            "H2" => Ok(CandlestickGranularity::H2),
            "H3" => Ok(CandlestickGranularity::H3),
            "H4" => Ok(CandlestickGranularity::H4),
            "H6" => Ok(CandlestickGranularity::H6),
            "H8" => Ok(CandlestickGranularity::H8),
            "H12" => Ok(CandlestickGranularity::H12),
            "D" => Ok(CandlestickGranularity::D),
            "W" => Ok(CandlestickGranularity::W),
            _ => Err(format!("{} is not a valid resolution", s)),
        }
    }
}

#[allow(dead_code)]
pub(crate) fn granularity_to_duration(resolution: &CandlestickGranularity) -> Duration {
    match resolution {
        CandlestickGranularity::S5 => Duration::seconds(5),
        CandlestickGranularity::S10 => Duration::seconds(10),
        CandlestickGranularity::S15 => Duration::seconds(15),
        CandlestickGranularity::S30 => Duration::seconds(30),
        CandlestickGranularity::M1 => Duration::minutes(1),
        CandlestickGranularity::M2 => Duration::minutes(2),
        CandlestickGranularity::M4 => Duration::minutes(4),
        CandlestickGranularity::M5 => Duration::minutes(5),
        CandlestickGranularity::M10 => Duration::minutes(10),
        CandlestickGranularity::M15 => Duration::minutes(15),
        CandlestickGranularity::M30 => Duration::minutes(30),
        CandlestickGranularity::H1 => Duration::hours(1),
        CandlestickGranularity::H2 => Duration::hours(2),
        CandlestickGranularity::H3 => Duration::hours(3),
        CandlestickGranularity::H4 => Duration::hours(4),
        CandlestickGranularity::H6 => Duration::hours(6),
        CandlestickGranularity::H8 => Duration::hours(8),
        CandlestickGranularity::H12 => Duration::hours(12),
        CandlestickGranularity::D => Duration::days(1),
        CandlestickGranularity::W => Duration::weeks(1),
        CandlestickGranularity::M => Duration::weeks(4),
    }
}

#[allow(dead_code)]
pub(crate) fn add_time_to_date(resolution: &CandlestickGranularity) -> Duration {
    match resolution {
        CandlestickGranularity::S5 => Duration::hours(2),
        CandlestickGranularity::S10 | CandlestickGranularity::S15 | CandlestickGranularity::S30 => Duration::hours(4),
        CandlestickGranularity::M1 | CandlestickGranularity::M2 |  CandlestickGranularity::M4 | CandlestickGranularity::M5 => Duration::hours(8),
        CandlestickGranularity::M10 | CandlestickGranularity::M15 | CandlestickGranularity::M30 => Duration::hours(24),
        CandlestickGranularity::H1 | CandlestickGranularity::H2 | CandlestickGranularity::H3 | CandlestickGranularity::H4 | CandlestickGranularity::H6 | CandlestickGranularity::H8 | CandlestickGranularity::H12 => Duration::days(100),
        CandlestickGranularity::D => Duration::days(1000),
        CandlestickGranularity::W => Duration::days(3000),
        CandlestickGranularity::M => Duration::days(3000),
    }
}

impl fmt::Display for CandlestickGranularity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            CandlestickGranularity::S5 => "S5",
            CandlestickGranularity::S10 => "S10",
            CandlestickGranularity::S15 => "S15",
            CandlestickGranularity::S30 => "S30",
            CandlestickGranularity::M1 => "M1",
            CandlestickGranularity::M2 => "M2",
            CandlestickGranularity::M4 => "M4",
            CandlestickGranularity::M5 => "M5",
            CandlestickGranularity::M10 => "M10",
            CandlestickGranularity::M15 => "M15",
            CandlestickGranularity::M30 => "M30",
            CandlestickGranularity::H1 => "H1",
            CandlestickGranularity::H2 => "H2",
            CandlestickGranularity::H3 => "H3",
            CandlestickGranularity::H4 => "H4",
            CandlestickGranularity::H6 => "H6",
            CandlestickGranularity::H8 => "H8",
            CandlestickGranularity::H12 => "H12",
            CandlestickGranularity::D => "D",
            CandlestickGranularity::W => "W",
            CandlestickGranularity::M => "M",
        })
    }
}

/// Summary: The day of the week to use for candlestick granularities with weekly alignment.
#[derive(Serialize, Deserialize, Debug)]
pub enum WeeklyAlignment {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

/// Summary: The prices data (open, high, low, close) for the Candlestick representation.
#[derive(Serialize, Deserialize, Debug)]
pub struct CandlestickData {
    #[serde(rename = "o")]
    pub open: Decimal,
    #[serde(rename = "h")]
    pub high: Decimal,
    #[serde(rename = "l")]
    pub low: Decimal,
    #[serde(rename = "c")]
    pub close: Decimal,
}

/// Summary: The Candlestick representation
#[derive(Serialize, Deserialize, Debug)]
pub struct Candlestick {
    pub time: DateTime,
    pub bid: Option<CandlestickData>,
    pub ask: Option<CandlestickData>,
    pub mid: Option<CandlestickData>,
    pub volume: i32,
    pub complete: bool,
}

/// Summary: Response containing instrument, granularity, and a list of quotebars.
#[derive(Serialize, Deserialize, Debug)]
pub struct CandlestickResponse {
    pub instrument: InstrumentName,
    pub granularity: CandlestickGranularity,
    pub candles: Vec<Candlestick>,
}

/// Summary: The representation of an instrument’s order book at a point in time
#[derive(Serialize, Deserialize, Debug)]
pub struct OrderBook {
    pub instrument: InstrumentName,
    pub time: DateTime,
    pub price: Decimal,
    #[serde(rename = "bucketWidth")]
    pub bucket_width: Decimal,
    pub buckets: Vec<OrderBookBucket>,
}

/// Summary: The order book data for a partition of the instrument’s prices.
#[derive(Serialize, Deserialize, Debug)]
pub struct OrderBookBucket {
    pub price: Decimal,
    #[serde(rename = "longCountPercent")]
    pub long_count_percent: Decimal,
    #[serde(rename = "shortCountPercent")]
    pub short_count_percent: Decimal,
}

/// Summary: The representation of an instrument’s position book at a point in time
#[derive(Serialize, Deserialize, Debug)]
pub struct PositionBook {
    pub instrument: InstrumentName,
    pub time: DateTime,
    pub price: Decimal,
    #[serde(rename = "bucketWidth")]
    pub bucket_width: Decimal,
    pub buckets: Vec<PositionBookBucket>,
}

/// Summary: The position book data for a partition of the instrument’s prices.
#[derive(Serialize, Deserialize, Debug)]
pub struct PositionBookBucket {
    pub price: Decimal,
    #[serde(rename = "longCountPercent")]
    pub long_count_percent: Decimal,
    #[serde(rename = "shortCountPercent")]
    pub short_count_percent: Decimal,
}