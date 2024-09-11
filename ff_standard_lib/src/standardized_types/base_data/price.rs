use crate::helpers::converters::time_local_from_str;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::enums::Resolution;
use crate::standardized_types::subscriptions::{DataSubscription, Symbol};
use crate::standardized_types::Price;
use chrono::{DateTime, FixedOffset};
use chrono_tz::Tz;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use std::fmt::{Debug, Display};
use std::str::FromStr;

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

    pub fn resolution(&self) -> Resolution {
        Resolution::Instant
    }

    pub fn subscription(&self) -> DataSubscription {
        let symbol = self.symbol.clone();
        DataSubscription::from_base_data(
            symbol.name.clone(),
            symbol.data_vendor.clone(),
            Resolution::Instant,
            BaseDataType::TradePrices,
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
