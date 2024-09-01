use crate::helpers::converters::time_local_from_str;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::enums::Resolution;
use crate::standardized_types::subscriptions::{DataSubscription, Symbol};
use crate::standardized_types::{Price, TimeString};
use chrono::{DateTime, FixedOffset};
use chrono_tz::Tz;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use std::fmt;
use std::fmt::{Debug, Display};
use std::str::FromStr;
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
    pub ask_volume: f64,
    pub bid_volume: f64,
    pub time: TimeString,
    pub book_level: BookLevel,
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
        book_level: BookLevel,
    ) -> Self {
        Quote {
            symbol,
            ask,
            bid,
            ask_volume,
            bid_volume,
            time,
            book_level,
        }
    }

    pub fn resolution(&self) -> Resolution {
        Resolution::Instant
    }

    pub fn subscription(&self) -> DataSubscription {
        let symbol = self.symbol.clone();
        DataSubscription::from_base_data(
            symbol.name.clone(),
            symbol.data_vendor.clone(),
            Resolution::Instant,
            BaseDataType::Quotes,
            symbol.market_type.clone(),
            None,
        )
    }

    pub fn time_utc(&self) -> DateTime<chrono::Utc> {
        DateTime::from_str(&self.time).unwrap()
    }

    pub fn time_local(&self, time_zone: &Tz) -> DateTime<FixedOffset> {
        time_local_from_str(time_zone, &self.time)
    }
}

impl Display for Quote {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:?},{},{},{},{},{},{}",
            self.symbol,
            self.ask,
            self.bid,
            self.ask_volume,
            self.bid_volume,
            self.time,
            self.book_level
        )
    }
}

impl Debug for Quote {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Quote {{ symbol: {:?}, ask: {}, bid: {}, ask_volume: {}, bid_volume: {}, time: {}, book_level: {}",
            self.symbol, self.ask, self.bid, self.ask_volume, self.bid_volume, self.time, self.book_level
        )
    }
}
