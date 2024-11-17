use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::quotebar::QuoteBar;
use crate::standardized_types::enums::MarketType;
use crate::standardized_types::subscriptions::{CandleType, DataSubscription, Symbol};
use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use std::fmt;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::standardized_types::datavendor_enum::DataVendor;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::new_types::{Price, TimeString, Volume};
use crate::standardized_types::resolution::Resolution;

#[derive(
    Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Eq, PartialOrd, Ord,
)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum CandleCalculationType {
    HeikenAshi,
    Candle,
}

impl Default for CandleCalculationType {
    fn default() -> Self {
        CandleCalculationType::Candle
    }
}

/// Represents a single candlestick in a candlestick chart, commonly used
/// in the financial technical analysis of price patterns.
///
/// # Fields
///
/// - `symbol`: The trading symbol of the asset.
/// - `high`: The highest price.
/// - `low`: The lowest price.
/// - `open`: The opening price.
/// - `close`: The closing price.
/// - `volume`: The trading volume.
/// - `range`: The difference between the high and low prices.
/// - `time`: The opening time of the candles as a Unix timestamp.
/// - `is_closed`: Indicates whether the candles is closed.
/// - `data_vendor`: The data vendor that provided the candles.
/// - `resolution`: The resolution of the candles.
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq)]
#[archive(
// This will generate a PartialEq impl between our unarchived and archived
// types:
compare(PartialEq),
// bytecheck can be used to validate your data if you want. To use the safe
// API, you have to derive CheckBytes for the archived type:
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub struct Candle {
    pub symbol: Symbol,
    pub high: Price,
    pub low: Price,
    pub open: Price,
    pub close: Price,
    pub volume: Volume,
    pub ask_volume: Volume,
    pub bid_volume: Volume,
    pub range: Price,
    pub time: TimeString,
    pub is_closed: bool,
    pub resolution: Resolution,
    pub candle_type: CandleType,
}

impl Display for Candle {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?},{},{},{},{},{},{},{},{},{},{},{},{}",
            self.symbol,
            self.resolution,
            self.high,
            self.low,
            self.open,
            self.close,
            self.volume,
            self.ask_volume,
            self.bid_volume,
            self.range,
            self.time,
            self.is_closed,
            self.candle_type
        )
    }
}

impl BaseData for Candle {
    fn symbol_name(&self) -> Symbol {
        self.symbol.clone()
    }

    /// The actual candle object time, not adjusted for close etc, this is used when drawing the candle on charts.
    fn time_local(&self, time_zone: &Tz) -> DateTime<Tz> {
        time_zone.from_utc_datetime(&self.time_utc().naive_utc())
    }

    /// The actual candle object time, not adjusted for close etc, this is used when drawing the candle on charts.
    fn time_utc(&self) -> DateTime<chrono::Utc> {
        DateTime::from_str(&self.time).unwrap()
    }

    fn time_closed_utc(&self) -> DateTime<Utc> {
        self.time_utc() + self.resolution.as_duration()
    }

    fn time_closed_local(&self, time_zone: &Tz) -> DateTime<Tz> {
        time_zone.from_utc_datetime(&self.time_utc().naive_utc()) + self.resolution.as_duration()
    }

    fn data_vendor(&self) -> DataVendor {
        self.symbol.data_vendor.clone()
    }

    fn market_type(&self) -> MarketType {
        self.symbol.market_type.clone()
    }

    fn resolution(&self) -> Resolution {
        self.resolution.clone()
    }

    fn symbol(&self) -> &Symbol {
        &self.symbol
    }

    fn subscription(&self) -> DataSubscription {
        let symbol = self.symbol.clone();
        let resolution = self.resolution();
        let candle_type = Some(self.candle_type.clone());
        DataSubscription::from_base_data(
            symbol.name.clone(),
            symbol.data_vendor.clone(),
            resolution,
            BaseDataType::Candles,
            symbol.market_type.clone(),
            candle_type,
        )
    }
}

impl Candle {

    pub fn from_quotebar(quotebar: QuoteBar, bid: bool) -> Self {
        let high = match bid {
            true => quotebar.bid_high,
            false => quotebar.ask_high,
        };
        let low = match bid {
            true => quotebar.bid_low,
            false => quotebar.ask_low,
        };
        let open = match bid {
            true => quotebar.bid_open,
            false => quotebar.ask_open,
        };
        let close = match bid {
            true => quotebar.bid_close,
            false => quotebar.ask_close,
        };

        Candle {
            symbol: quotebar.symbol,
            time: quotebar.time,
            is_closed: quotebar.is_closed,
            open,
            high,
            low,
            close,
            volume: quotebar.volume,
            ask_volume: quotebar.ask_volume,
            bid_volume: quotebar.bid_volume,
            range: high - low,
            resolution: quotebar.resolution,
            candle_type: CandleType::CandleStick,
        }
    }

    pub fn new(
        symbol: Symbol,
        open: Price,
        volume: Volume,
        ask_volume: Volume,
        bid_volume: Volume,
        time: TimeString,
        resolution: Resolution,
        candle_type: CandleType,
    ) -> Self {
        Self {
            symbol,
            high: open,
            low: open,
            open,
            close: open,
            volume,
            ask_volume,
            bid_volume,
            range: dec!(0.0),
            time,
            is_closed: false,
            resolution,
            candle_type,
        }
    }



    /// Creates a new `candles` instance representing a completed (closed) trading period.
    pub fn from_closed(
        symbol: Symbol,
        high: Price,
        low: Price,
        open: Price,
        close: Price,
        volume: Volume,
        ask_volume: Volume,
        bid_volume: Volume,
        time: DateTime<chrono::Utc>,
        resolution: Resolution,
        candle_type: CandleType,
    ) -> Self {
        Self {
            symbol,
            high,
            low,
            open,
            close,
            volume,
            ask_volume,
            bid_volume,
            range: high - low,
            time: time.to_string(),
            is_closed: true,
            resolution,
            candle_type,
        }
    }

    /// Updates the candles with new price and volume information. Typically used
    /// during the trading period before the candles closes.
    pub fn update(&mut self, price: Price, volume: Volume, is_closed: bool) {
        self.high = self.high.max(price);
        self.low = self.low.min(price);
        self.close = price;
        self.volume += volume;
        self.range = self.high - self.low;

        if is_closed {
            self.is_closed = true;
        }
    }
}

impl fmt::Debug for Candle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Candle {{ resolution {}, symbol: {:?}, high: {}, low: {}, open: {}, close: {}, volume: {}, ask_volume: {}, bid_volume: {} range: {}, time: {}, is_closed: {}, candle_type {} }}",
            self.resolution, self.symbol, self.high, self.low, self.open, self.close, self.volume, self.ask_volume, self.bid_volume, self.range, self.time, self.is_closed, self.candle_type
        )
    }
}

pub fn generate_5_day_candle_data() -> Vec<Candle> {
    let mut test_data = Vec::new();
    let base_date = NaiveDate::from_ymd_opt(2024, 11, 10).unwrap();

    for day in 0..5 {
        for hour in 0..24 {
            // Generate time using DateTime<Utc>
            let naive_time = NaiveDateTime::new(
                base_date + chrono::Duration::days(day as i64),
                chrono::NaiveTime::from_hms_opt(hour, 0, 0).unwrap(),
            );
            let utc_time: DateTime<Utc> = Utc.from_utc_datetime(&naive_time);

            // Simulate values
            let open = Decimal::from(100 + day as i32 * 10 + hour as i32);
            let close = open + Decimal::from(-5i32 + (hour % 2) as i32 * 10); // Fluctuate up and down
            let high = open.max(close) + Decimal::from(2); // Simulate high
            let low = open.min(close) - Decimal::from(2);  // Simulate low
            let volume = Decimal::from(1000 + day as i32 * 100 + hour as i32 * 10);
            let ask_volume = volume / Decimal::from(2);
            let bid_volume = volume / Decimal::from(2);
            let range = high - low;

            test_data.push(Candle {
                symbol: Symbol::new("TEST".to_string(), DataVendor::Test, MarketType::CFD), // Example symbol
                high,
                low,
                open,
                close,
                volume,
                ask_volume,
                bid_volume,
                range,
                time: utc_time.to_string(),        // Generate time string from DateTime<Utc>
                is_closed: true,                   // Assume candles are closed
                resolution: Resolution::Hours(1),  // 1-hour resolution
                candle_type: CandleType::CandleStick, // Assume standard candles
            });
        }
    }
    test_data
}