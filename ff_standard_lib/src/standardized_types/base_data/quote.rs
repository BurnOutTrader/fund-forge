use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::enums::{MarketType, Resolution};
use crate::standardized_types::subscriptions::{DataSubscription, Symbol};
use crate::standardized_types::{Price, TimeString, Volume};
use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use std::fmt;
use std::fmt::{Debug, Display};
use std::str::FromStr;
use crate::apis::data_vendor::datavendor_enum::DataVendor;
use crate::standardized_types::base_data::traits::BaseData;

pub type BookLevel = u8;
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
/// A `Quote` is a snapshot of the current bid and ask prices for a given symbol. This is generally used to read the book or for cfd trading.
///
/// # Parameters
/// * `symbol` - The symbol of the asset.
/// * `ask` - The current ask price.
/// * `bid` - The current bid price.
/// * `ask_volume` - The volume of the ask price.
/// * `bid_volume` - The volume of the bid price.
/// * `time` - The time of the quote.
pub struct Quote {
    pub symbol: Symbol,
    pub ask: Price,
    pub bid: Price,
    pub ask_volume: Volume,
    pub bid_volume: Volume,
    pub time: TimeString,
}


impl BaseData for Quote {
    fn symbol_name(&self) -> Symbol {
        self.symbol.clone()
    }

    /// The actual candle object time, not adjusted for close etc, this is used when drawing the candle on charts.
    fn time_local(&self, time_zone: &Tz) -> DateTime<Tz> {
        let utc_time: DateTime<Utc> = DateTime::from_str(&self.time).unwrap();
        time_zone.from_utc_datetime(&utc_time.naive_utc())
    }

    /// The actual candle object time, not adjusted for close etc, this is used when drawing the candle on charts.
    fn time_utc(&self) -> DateTime<chrono::Utc> {
        DateTime::from_str(&self.time).unwrap()
    }

    fn time_created_utc(&self) -> DateTime<Utc> {
        self.time_utc()
    }

    fn time_created_local(&self, time_zone: &Tz) -> DateTime<Tz> {
        time_zone.from_utc_datetime(&self.time_utc().naive_utc())
    }

    fn data_vendor(&self) -> DataVendor {
        self.symbol.data_vendor.clone()
    }

    fn market_type(&self) -> MarketType {
        self.symbol.market_type.clone()
    }

    fn resolution(&self) -> Resolution {
        Resolution::Instant
    }

    fn symbol(&self) -> &Symbol {
        &self.symbol
    }

    fn subscription(&self) -> DataSubscription {
        let symbol = self.symbol.clone();
        let resolution = self.resolution();
        let candle_type = None;
        DataSubscription::from_base_data(
            symbol.name.clone(),
            symbol.data_vendor.clone(),
            resolution,
            BaseDataType::Quotes,
            symbol.market_type.clone(),
            candle_type,
        )
    }
}

impl Quote {
    /// Create a new `Quote` with the given parameters.
    ///
    /// # Parameters
    /// 1. `symbol` - The symbol of the asset.
    /// 2. `ask` - The current ask price.
    /// 3. `bid` - The current bid price.
    /// 4. `ask_volume` - The volume of the ask price.
    /// 5. `bid_volume` - The volume of the bid price.
    /// 6. `time` - The time of the quote.
    /// 7. `book_level` - The level in the order book where 0 is best bid or best ask
    pub fn new(
        symbol: Symbol,
        ask: Price,
        bid: Price,
        ask_volume: Price,
        bid_volume: Price,
        time: TimeString,
    ) -> Self {
        Quote {
            symbol,
            ask,
            bid,
            ask_volume,
            bid_volume,
            time,
        }
    }
}

impl Display for Quote {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:?},{},{},{},{},{}",
            self.symbol,
            self.ask,
            self.bid,
            self.ask_volume,
            self.bid_volume,
            self.time,
        )
    }
}

impl Debug for Quote {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Quote {{ symbol: {:?}, ask: {}, bid: {}, ask_volume: {}, bid_volume: {}, time: {}",
            self.symbol, self.ask, self.bid, self.ask_volume, self.bid_volume, self.time
        )
    }
}
