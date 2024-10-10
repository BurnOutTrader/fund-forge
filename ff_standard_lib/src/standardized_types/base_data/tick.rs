use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::enums::{MarketType, OrderSide};
use crate::standardized_types::subscriptions::{DataSubscription, Symbol};
use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use std::fmt;
use std::fmt::Debug;
use std::str::FromStr;
use crate::standardized_types::datavendor_enum::DataVendor;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::new_types::{Price, TimeString, Volume};
use crate::standardized_types::resolution::Resolution;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq)]
#[archive(compare(PartialEq), check_bytes)]
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
    pub price: Price,
    pub time: TimeString,
    pub volume: Volume,
    pub side: Option<OrderSide>
}

impl BaseData for Tick {
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
        self.time_utc()
    }

    fn time_closed_local(&self, time_zone: &Tz) -> DateTime<Tz> {
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
            BaseDataType::Candles,
            symbol.market_type.clone(),
            candle_type,
        )
    }
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
    pub fn new(symbol: Symbol, price: Price, time: TimeString, volume: Volume, side: Option<OrderSide>) -> Self {
        Tick {
            symbol,
            price,
            time,
            volume,
            side,
        }
    }
}

impl fmt::Display for Tick {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let side = match self.side {
            None => "None".to_string(),
            Some(side) => side.to_string()
        };
        write!(
            f,
            "{:?},{},{},{},{}",
            self.symbol, self.price, self.volume, side, self.time
        )
    }
}

impl fmt::Debug for Tick {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let side = match self.side {
            None => "None".to_string(),
            Some(side) => side.to_string()
        };
        write!(
            f,
            "Tick {{ symbol: {:?}, price: {}, volume: {}, side: {}, time: {}}}",
            self.symbol, self.price, self.volume, side, self.time
        )
    }
}
