use crate::helpers::converters::time_local_from_str;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::enums::{MarketType, Resolution};
use crate::standardized_types::subscriptions::{DataSubscription, Symbol};
use crate::standardized_types::Price;
use chrono::{DateTime, FixedOffset, Utc};
use chrono_tz::Tz;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use std::fmt::{Debug, Display};
use std::str::FromStr;
use crate::apis::data_vendor::datavendor_enum::DataVendor;
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::base_data::traits::BaseData;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
/// The `Price` struct is used to represent the price of an asset at a given time.
///
/// # Parameters
/// 1. `symbol` - The symbol of the asset.
/// 2. `price` - The price of the asset.
/// 3. `time` - The time the price was recorded.
pub struct TradePrice {
    pub symbol: Symbol,
    pub price: Price,
    pub time: String,
}

impl BaseData for TradePrice {
    fn symbol_name(&self) -> Symbol {
        self.symbol.clone()
    }

    /// The actual candle object time, not adjusted for close etc, this is used when drawing the candle on charts.
    fn time_local(&self, time_zone: &Tz) -> DateTime<FixedOffset> {
        time_local_from_str(time_zone, &self.time)
    }

    /// The actual candle object time, not adjusted for close etc, this is used when drawing the candle on charts.
    fn time_utc(&self) -> DateTime<chrono::Utc> {
        DateTime::from_str(&self.time).unwrap()
    }

    fn time_created_utc(&self) -> DateTime<Utc> {
        self.time_utc()
    }

    fn time_created_local(&self, time_zone: &Tz) -> DateTime<FixedOffset> {
        self.time_local(time_zone)
    }

    fn data_vendor(&self) -> DataVendor {
        self.data_vendor()
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
            BaseDataType::TradePrices,
            symbol.market_type.clone(),
            candle_type,
        )
    }
}

impl TradePrice {
    /// Create a new `Price` instance.
    /// # Parameters
    /// 1. `symbol` - The symbol of the asset.
    /// 2. `price` - The price of the asset.
    /// 3. `time` - The time the price was recorded.
    /// 4. `vendor` - The data vendor that provided the price.
    pub fn new(symbol: Symbol, price: Price, time: String) -> Self {
        TradePrice {
            symbol,
            price,
            time,
        }
    }
}

impl Display for TradePrice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Symbol: {:?}, Price: {}, Time: {}",
            self.symbol, self.price, self.time
        )
    }
}

impl Debug for TradePrice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Price")
            .field("symbol", &self.symbol)
            .field("price", &self.price)
            .field("time", &self.time)
            .finish()
    }
}
