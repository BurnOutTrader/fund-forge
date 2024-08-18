use std::fmt::{Debug, Display};
use std::str::FromStr;
use chrono::{DateTime, FixedOffset};
use chrono_tz::Tz;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::standardized_types::enums::Bias;
use crate::helpers::converters::{time_local_from_str};
use crate::standardized_types::subscriptions::Symbol;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
/// The `Fundamental` struct can be used to store any other data that is not a `Price`, `Quote`, `Tick`, `QuoteBar` or `Candle` and feed it into the algorithm `fn on_data_updates()` \
/// \
/// The value properties are optional to allow flexibility in the data that can be stored, the most versatile of these is `Vec<u8>` which can store any binary data which can then be cast to any concrete object using the crate `rkyv`. \
/// therefore there is no need to modify the base data enum with more types unless you find some limitation to this approach. \
/// \
/// the value_string could also be used to hold json objects etc, so there really shouldnt be any limitations as to what data you can feed into your algorithm. \
/// Just remember that the time for for the data should be the UTC time that it was created in its final form, in live situations this isn't critical as the data will be fed into the engine upon receipt. \
/// For backtesting it is critical that the time is the time that the data was created in its final form, this is because the engine will use the time to determine which `TimeSlice` to feed the data into.
///
/// # Parameters
/// 1. `symbol` - `String` The symbol of the asset.
/// 2. `time` - `i64` The time stamp the price was recorded.
/// 3. `value` - The value of the fundamental data.
/// 4. `value_string` - `Option<f64>` The value of the fundamental data as a string, this can be used to pass in json objects etc to the `fn on_data_updates`.
/// 5. `value_bytes` - `Option<Vec<u8>>` The value of the fundamental data as a byte array.
/// 6. `name` - `String` The name of the fundamental data: This can be used in the `ff_data_server` to specify how the server is to pull the data from the specified broker, this allows max versatility with minimum hard coding, or re-coding of the engine.
/// 7. `bias` - `Bias` enum The bias of the fundamental data `Bias` enum variant.
pub struct Fundamental {
    pub symbol: Symbol,
    pub time: String,
    pub value: Option<f64>,
    pub value_string: Option<String>,
    pub value_bytes: Option<Vec<u8>>,
    pub name: String,
    pub bias: Bias,
}

impl Fundamental {
    /// Create a new `Fundamental` instance.
    ///
    /// # Parameters
    /// 1. `symbol` - `Symbol` The symbol of the asset.
    /// 2. `time` - `i64` The time stamp the price was recorded.
    /// 3. `value` - `Option<f64>` The value of the fundamental data.
    /// 4. `value_string` - `Option<String>` The value of the fundamental data as a string, this can be used to pass in json objects etc to the `fn on_data_updates`. using regular time slice objects built into the engine.
    /// 5. `value_bytes` - `Option<Vec<u8>>` The value of the fundamental data as a byte array.
    /// 6. `name` - `String` The name of the fundamental data: This can be used in the `ff_data_server` to specify how the server is to pull the data from the specified broker, this allows max versatility with minimum hard coding, or re-coding of the engine.
    /// 7. `bias` - `Bias` enum The bias of the fundamental data `Bias` enum variant.
    /// 8. `data_vendor` - `DataVendor` enum The data vendor of the fundamental data `DataVendor` enum variant.
    pub fn new(symbol: Symbol, time: String, value: Option<f64>, value_string: Option<String>, value_bytes: Option<Vec<u8>>, name: String, bias: Bias) -> Self {
        Self {
            symbol,
            time,
            value,
            value_string,
            value_bytes,
            name,
            bias,
        }
    }

    pub fn time_utc(&self) -> DateTime<chrono::Utc> {
        DateTime::from_str(&self.time).unwrap()
    }

    pub fn time_local(&self, time_zone: &Tz) -> DateTime<FixedOffset> {
        time_local_from_str(time_zone, &self.time)
    }
}

impl Debug for Fundamental {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fundamental {{ symbol: {:?}, time: {}, value: {:?}, value_string: {:?}, name: {}, bias: {}}}", self.symbol, self.time, self.value, self.value_string, self.name, self.bias)
    }
}

impl Display for Fundamental {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fundamental {{ symbol: {:?}, time: {}, value: {:?}, value_string: {:?}, name: {}, bias: {}}}", self.symbol, self.time, self.value, self.value_string, self.name, self.bias)
    }
}