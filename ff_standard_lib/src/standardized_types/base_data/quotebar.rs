use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::enums::MarketType;
use crate::standardized_types::subscriptions::{CandleType, DataSubscription, Symbol};
use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use std::fmt;
use std::fmt::{Debug, Display};
use std::str::FromStr;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::standardized_types::datavendor_enum::DataVendor;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::new_types::{Price, TimeString, Volume};
use crate::standardized_types::resolution::Resolution;

/// Represents a single quote bar in a financial chart, commonly used
/// in the financial technical analysis of price patterns.
///
/// # Fields
///
/// - `symbol`: The trading symbol of the asset.
/// - `bid_high`: The highest bid price.
/// - `bid_low`: The lowest bid price.
/// - `bid_open`: The opening bid price.
/// - `bid_close`: The closing bid price.
/// - `ask_high`: The highest ask price.
/// - `ask_low`: The lowest ask price.
/// - `ask_open`: The opening ask price.
/// - `ask_close`: The closing ask price.
/// - `volume`: The trading volume.
/// - `range`: The difference between the high and low prices.
/// - `time`: The opening time of the quote bar as a Unix timestamp.
/// - `spread`: The difference between the highest ask price and the lowest bid price.
/// - `is_closed`: Indicates whether the quote bar is closed.
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct QuoteBar {
    pub symbol: Symbol,
    pub bid_high: Price,
    pub bid_low: Price,
    pub bid_open: Price,
    pub bid_close: Price,
    pub ask_high: Price,
    pub ask_low: Price,
    pub ask_open: Price,
    pub ask_close: Price,
    pub volume: Volume,
    pub ask_volume: Volume,
    pub bid_volume: Volume,
    pub range: Price,
    pub time: TimeString,
    pub spread: Price,
    pub is_closed: bool,
    pub resolution: Resolution,
    pub candle_type: CandleType,
}


impl BaseData for QuoteBar {
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
        self.time_local(time_zone) + self.resolution.as_duration()
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
            BaseDataType::QuoteBars,
            symbol.market_type.clone(),
            candle_type,
        )
    }
}

impl QuoteBar {
    /// Creates a new `QuoteBar` instance that is open and has not yet closed.
    ///
    /// # Properties
    ///
    /// - `symbol`: The trading symbol of the asset.
    /// - `high`: The highest price.
    /// - `low`: The lowest price.
    /// - `open`: The opening price.
    /// - `close`: The closing price.
    /// - `bid_high`: The highest bid price.
    /// - `bid_low`: The lowest bid price.
    /// - `bid_open`: The opening bid price.
    /// - `bid_close`: The closing bid price.
    /// - `ask_high`: The highest ask price.
    /// - `ask_low`: The lowest ask price.
    /// - `ask_open`: The opening ask price.
    /// - `ask_close`: The closing ask price.
    /// - `volume`: The trading volume.
    /// - `range`: The difference between the high and low prices.
    /// - `time`: The opening time of the quote bar as a Unix timestamp.
    /// - `spread`: The difference between the highest ask price and the lowest bid price.
    /// - `is_closed`: Indicates whether the quote bar is closed.
    /// - `data_vendor`: The data vendor that provided the quote bar.
    /// - `resolution`: The resolution of the quote bar.
    /// - `candle_type`: The type of candlestick.
    pub fn new(
        symbol: Symbol,
        bid_open: Price,
        ask_open: Price,
        volume: Volume,
        ask_volume: Volume,
        bid_volume: Volume,
        time: String,
        resolution: Resolution,
        candle_type: CandleType,
    ) -> Self {
        Self {
            symbol,
            bid_high: bid_open,
            bid_low: bid_open,
            bid_open,
            bid_close: bid_open,
            ask_high: ask_open,
            ask_low: ask_open,
            ask_open,
            ask_close: ask_open,
            volume,
            ask_volume,
            bid_volume,
            range: dec!(0.0),
            time,
            spread: ask_open - bid_open,
            is_closed: false,
            resolution,
            candle_type,
        }
    }

    pub fn subscription(&self) -> DataSubscription {
        let symbol = self.symbol.clone();
        let resolution = self.resolution.clone();
        let candle_type = Some(self.candle_type.clone());
        DataSubscription::from_base_data(
            symbol.name.clone(),
            symbol.data_vendor.clone(),
            resolution,
            BaseDataType::QuoteBars,
            symbol.market_type.clone(),
            candle_type,
        )
    }

    /// Creates a new `QuoteBar` instance representing a completed (closed) trading period.
    ///
    /// # Arguments
    ///
    /// - `symbol`: The trading symbol of the asset.
    /// - `bid_high`: The highest bid price during the quote bar's time.
    /// - `bid_low`: The lowest bid price during the quote bar's time.
    /// - `bid_open`: The opening bid price.
    /// - `bid_close`: The closing bid price.
    /// - `ask_high`: The highest ask price during the quote bar's time.
    /// - `ask_low`: The lowest ask price during the quote bar's time.
    /// - `ask_open`: The opening ask price.
    /// - `ask_close`: The closing ask price.
    /// - `volume`: The trading volume.
    /// - `time`: The opening time as a Unix timestamp.
    /// - `data_vendor`: The data vendor that provided the quote bar.
    pub fn from_closed(
        symbol: Symbol,
        bid_high: Price,
        bid_low: Price,
        bid_open: Price,
        bid_close: Price,
        ask_high: Price,
        ask_low: Price,
        ask_open: Price,
        ask_close: Price,
        volume: Volume,
        ask_volume: Volume,
        bid_volume: Volume,
        time: DateTime<chrono::Utc>,
        resolution: Resolution,
        candle_type: CandleType,
    ) -> Self {
        Self {
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
            ask_volume,
            bid_volume,
            range: (ask_high + bid_high) - (ask_low + bid_low),
            time: time.to_string(),
            spread: ask_close - bid_close,
            is_closed: true,
            resolution,
            candle_type,
        }
    }
}

impl Display for QuoteBar {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{},{:?},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            self.resolution,
            self.symbol,
            self.bid_high,
            self.bid_low,
            self.bid_open,
            self.bid_close,
            self.ask_high,
            self.ask_low,
            self.ask_open,
            self.ask_close,
            self.volume,
            self.ask_volume,
            self.bid_volume,
            self.range,
            self.spread,
            self.time,
            self.is_closed,
            self.candle_type
        )
    }
}

impl Debug for QuoteBar {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "QuoteBar {{ resolution: {}, symbol: {:?}, bid_high: {}, bid_low: {}, bid_open: {}, bid_close: {}, ask_high: {}, ask_low: {}, ask_open: {}, ask_close: {}, volume: {}, ask_volume: {}, bid_volume: {}, range: {}, spread: {}, time: {}, is_closed: {} }}",
            self.resolution , self.symbol, self.bid_high, self.bid_low, self.bid_open, self.bid_close, self.ask_high, self.ask_low, self.ask_open, self.ask_close, self.volume, self.ask_volume, self.bid_volume ,self.range, self.spread, self.time, self.is_closed
        )
    }
}

pub fn generate_5_day_quote_bar_data() -> Vec<QuoteBar> {
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

            // Simulate bid and ask values
            let bid_open = Decimal::from(100 + day as i32 * 10 + hour as i32);
            let bid_close = bid_open + Decimal::from(-5i32 + (hour % 2) as i32 * 10);
            let bid_high = bid_open.max(bid_close) + Decimal::from(2);
            let bid_low = bid_open.min(bid_close) - Decimal::from(2);

            let ask_open = bid_open + Decimal::from(1);
            let ask_close = bid_close + Decimal::from(1);
            let ask_high = ask_open.max(ask_close) + Decimal::from(2);
            let ask_low = ask_open.min(ask_close) - Decimal::from(2);

            // Simulate volumes
            let volume = Decimal::from(1000 + day as i32 * 100 + hour as i32 * 10);
            let ask_volume = volume / Decimal::from(2);
            let bid_volume = volume / Decimal::from(2);

            // Calculate range and spread
            let range = (ask_high + bid_high) - (ask_low + bid_low);
            let spread = ask_close - bid_close;

            test_data.push(QuoteBar {
                symbol: Symbol::new("TEST".to_string(), DataVendor::DataBento, MarketType::CFD), // Example symbol
                bid_high,
                bid_low,
                bid_open,
                bid_close,
                ask_high,
                ask_low,
                ask_open,
                ask_close,
                volume,
                ask_volume,
                bid_volume,
                range,
                time: utc_time.to_string(),         // Generate time string from DateTime<Utc>
                spread,
                is_closed: true,                    // Assume quote bars are closed
                resolution: Resolution::Hours(1),   // 1-hour resolution
                candle_type: CandleType::CandleStick,  // Quote bar type
            });
        }
    }
    test_data
}
