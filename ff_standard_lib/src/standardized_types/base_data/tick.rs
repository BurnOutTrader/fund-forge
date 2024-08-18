use std::fmt;
use std::fmt::{Debug};
use std::str::FromStr;
use chrono::{DateTime, FixedOffset};
use chrono_tz::Tz;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::helpers::converters::time_local_from_str;
use crate::standardized_types::subscriptions::Symbol;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
/// A `Tick` is a single trade in a financial market.
///
/// # Parameters
/// 1. `symbol` - The symbol of the asset.
/// 2. `price` - The price of the asset.
/// 3. `time` - The time the price was recorded.
/// 4. `volume` - The volume of the trade.
pub struct Tick {
    pub symbol: Symbol,
    pub price: f64,
    pub time: String,
    pub volume: f64,
}

impl Tick {

    /// Create a new `Tick` instance.
    ///
    /// # Parameters
    /// 1. `symbol` - The symbol of the asset.
    /// 2. `price` - The price of the asset.
    /// 3. `time` - The time the price was recorded.
    /// 4. `volume` - The volume of the trade.
    /// 5. `side` - The side of the trade `Side` enum variant.
    /// 6. `data_vendor` - The data vendor of the trade.
    pub fn new(symbol: Symbol, price: f64, time: String, volume: f64) -> Self {
        Tick {
            symbol,
            price,
            time,
            volume,
        }
    }

    pub fn time_utc(&self) -> DateTime<chrono::Utc> {
        DateTime::from_str(&self.time).unwrap()
    }

    pub fn time_local(&self, time_zone: &Tz) -> DateTime<FixedOffset> {
        time_local_from_str(time_zone, &self.time)
    }
}

impl fmt::Display for Tick {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:?},{},{},{}",
            self.symbol, self.price, self.volume, self.time
        )
    }
}

impl fmt::Debug for Tick {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Tick {{ symbol: {:?}, price: {}, volume: {}, time: {}}}",
            self.symbol, self.price, self.volume, self.time
        )
    }
}


