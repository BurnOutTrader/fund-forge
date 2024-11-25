use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::base_data::fundamental::Fundamental;
use crate::standardized_types::base_data::quote::Quote;
use crate::standardized_types::base_data::quotebar::QuoteBar;
use crate::standardized_types::base_data::tick::Tick;
use crate::standardized_types::base_data::traits::BaseData;
use crate::messages::data_server_messaging::FundForgeError;
use crate::standardized_types::enums::MarketType;
use crate::standardized_types::subscriptions::{DataSubscription, Symbol};
use crate::standardized_types::bytes_trait::Bytes;
use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use rkyv::ser::serializers::AllocSerializer;
use rkyv::ser::Serializer;
use rkyv::{AlignedVec, Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use std::fmt::{Display, Error, Formatter};
use std::str::FromStr;
use crate::standardized_types::datavendor_enum::DataVendor;
use crate::standardized_types::resolution::Resolution;

/// Enum for the different types of base data
/// This is the main_window enum for all the different types of base data
/// It is used to store all the different types of base data in a single type
/// This is useful for when you want to store all the different types of base data in a single collection
/// # Variants
/// * `Price`       see [`BaseDataEnum::Price`](ff_data_vendors::base_data_types::base_data_enum::BaseDataEnum::Price)
/// * `Candle`      see [`BaseDataEnum::Candle`](ff_data_vendors::base_data_types::base_data_enum::BaseDataEnum::Candle)
/// * `QuoteBar`    see [`BaseDataEnum::QuoteBar`](ff_data_vendors::base_data_types::base_data_enum::BaseDataEnum::QuoteBar)
/// * `Tick`        see [`BaseDataEnum::Tick`](ff_data_vendors::base_data_types::base_data_enum::BaseDataEnum::Tick)
/// * `Quote`       see [`BaseDataEnum::Quote`](ff_data_vendors::base_data_types::base_data_enum::BaseDataEnum::Quote)
/// * `Fundamental` see [`BaseDataEnum::Fundamental`](ff_data_vendors::base_data_types::base_data_enum::BaseDataEnum::Fundamental)
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum BaseDataEnum {
    /// The `Candle` struct is used to represent the properties of an asset at a given Resolution. see [`Candle`](ff_data_vendors::base_data_types::candle::Candle)
    ///
    /// # Properties
    /// * `symbol` - The symbol of the asset.
    /// * `open` - The opening price of the asset.
    /// * `high` - The highest price of the asset.
    /// * `low` - The lowest price of the asset.
    /// * `close` - The closing price of the asset.
    /// * `volume` - The volume of the asset.
    /// * `time` - The time the price was recorded.
    /// * `is_closed` - A boolean value indicating if the candle is closed.
    /// * `data_vendor` - The data vendor that provided the price.
    /// * `resolution` - The resolution of the candle.
    /// * `candle_type` - The type of candle.
    Candle(Candle),
    /// The `QuoteBar` struct is used to represent the more detailed properties of an asset at a given Resolution. see [`QuoteBar`](ff_data_vendors::base_data_types::quotebar::QuoteBar)
    /// The QuoteBar is both a mid-price and bid-ask specific view.
    /// # Properties
    /// * `symbol`: The trading symbol of the asset.
    /// * `high`: The highest price.
    /// * `low`: The lowest price.
    /// * `open`: The opening price.
    /// * `close`: The closing price.
    /// * `bid_high`: The highest bid price.
    /// * `bid_low`: The lowest bid price.
    /// * `bid_open`: The opening bid price.
    /// * `bid_close`: The closing bid price.
    /// * `ask_high`: The highest ask price.
    /// * `ask_low`: The lowest ask price.
    /// * `ask_open`: The opening ask price.
    /// * `ask_close`: The closing ask price.
    /// * `volume`: The trading volume.
    /// * `range`: The difference between the high and low prices.
    /// * `time`: The opening time of the quote bar as a Unix timestamp.
    /// * `spread`: The difference between the highest ask price and the lowest bid price.
    /// * `is_closed`: Indicates whether the quote bar is closed.
    /// * `data_vendor`: The data vendor that provided the quote bar.
    /// * `resolution`: The resolution of the quote bar.
    /// * `candle_type`: The type of candle.
    QuoteBar(QuoteBar),

    /// The `Tick` struct is used to represent the price of an asset at a given time. see [`Tick`](ff_data_vendors::base_data_types::tick::Tick)
    ///
    /// # Properties
    /// * `symbol` - The symbol of the asset.
    /// * `price` - The price of the asset.
    /// *. `time` - The time the price was recorded.
    /// * `volume` - The volume of the trade or trades executed.
    /// * `side` - The side of the trade `Side` enum variant.
    /// * `data_vendor` - The data vendor of the trade.
    Tick(Tick),

    /// The `Quote` struct is used to represent the quoted prices of an asset at a given time, it represents best bid as 'bid' and best offer as 'ask'. see [`Quote`](ff_data_vendors::base_data_types::quote::Quote)
    ///
    /// # Properties
    /// * `symbol` - The symbol of the asset.
    /// * `ask` - The current best `ask` price.
    /// * `bid` - The current best `bid` price.
    /// * `ask_volume` - The volume on offer at the `ask` price.
    /// * `bid_volume` - The volume of `bids` at the bid price.
    /// * `time` - The time of the quote.
    /// * `data_vendor` - The data vendor of the quote.
    Quote(Quote),

    /// The `Fundamental` struct is used to represent any other data that does not fit into the other variants. see [`Fundamental`](ff_data_vendors::base_data_types::fundamental::Fundamental)
    /// It is a versatile data type that can be used to store any other data that you wish.
    /// # Properties
    /// * `symbol` - `String` The symbol of the asset.
    /// * `time` - `i64` The time stamp the price was recorded.
    /// * `value` - `Option<f64>` The value of the fundamental data.
    /// * `value_string` - `Option<String>` The value of the fundamental data as a string, this can be used to pass in json objects etc to the `fn on_data_updates`.
    /// * `value_bytes` - `Option<Vec<u8>>` The value of the fundamental data as a byte array.
    /// * `name` - `String` The name of the fundamental data: This can be used in the `ff_data_server` to specify how the server is to pull the data from the specified broker, this allows max versatility with minimum hard coding, or re-coding of the engine.
    /// * `bias` - `Bias` enum The bias of the fundamental data `Bias` enum variant.
    /// * `data_vendor` - `DataVendor` enum The data vendor of the fundamental data `DataVendor` enum variant.
    Fundamental(Fundamental),
}

impl Display for BaseDataEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BaseDataEnum::Candle(candle) => write!(
                f,
                "{}: {}, {}: {}, {}, {}, {}, {}, {}",
                candle.symbol.name,
                candle.resolution,
                candle.symbol.data_vendor,
                candle.open,
                candle.high,
                candle.low,
                candle.close,
                candle.volume,
                candle.time
            ),
            BaseDataEnum::QuoteBar(bar) => write!(
                f,
                "{}: {}, {}: bid: {}, {}, {}, {}, ask: {}, {}, {}, {}, {}, {}",
                bar.symbol.name,
                bar.resolution,
                bar.symbol.data_vendor,
                bar.bid_open,
                bar.bid_high,
                bar.bid_low,
                bar.bid_close,
                bar.ask_open,
                bar.ask_high,
                bar.ask_low,
                bar.ask_close,
                bar.volume,
                bar.time
            ),
            BaseDataEnum::Tick(tick) => write!(
                f,
                "{}: {}, {}: {}, {}",
                tick.symbol.name, tick.symbol.data_vendor, tick.price, tick.time, tick.volume
            ),
            BaseDataEnum::Quote(quote) => write!(
                f,
                "{}: {}, {}: {}, {}",
                quote.symbol.name, quote.symbol.data_vendor, quote.bid, quote.ask, quote.time
            ),
            BaseDataEnum::Fundamental(fundamental) => write!(
                f,
                "{}: {}, {:?}: {}, {}",
                fundamental.symbol.name,
                fundamental.symbol.data_vendor,
                fundamental.values,
                fundamental.time,
                fundamental.name
            ),
        }
    }
}

impl BaseDataEnum {


    /// Returns the `is_closed` property of the `BaseDataEnum` variant for variants that implement it, else just returns true.
    pub fn is_closed(&self) -> bool {
        match self {
            BaseDataEnum::Candle(candle) => candle.is_closed,
            BaseDataEnum::QuoteBar(quote_bar) => quote_bar.is_closed,
            _ => true,
        }
    }

    /// Links `BaseDataEnum` to a `BaseDataType`
    pub fn base_data_type(&self) -> BaseDataType {
        match self {
            BaseDataEnum::Candle(_) => BaseDataType::Candles,
            BaseDataEnum::QuoteBar(_) => BaseDataType::QuoteBars,
            BaseDataEnum::Tick(_) => BaseDataType::Ticks,
            BaseDataEnum::Quote(_) => BaseDataType::Quotes,
            BaseDataEnum::Fundamental(_) => BaseDataType::Fundamentals,
        }
    }

    pub(crate) fn set_is_closed(&mut self, is_closed: bool) {
        match self {
            BaseDataEnum::Candle(candle) => candle.is_closed = is_closed,
            BaseDataEnum::QuoteBar(bar) => bar.is_closed = is_closed,
            _ => {}
        }
    }

    /// Deserializes from `Vec<u8>` to `Vec<BaseDataEnum>`
    pub fn from_array_bytes(data: &Vec<u8>) -> Result<Vec<BaseDataEnum>, Error> {
        let archived_quotebars = match rkyv::check_archived_root::<Vec<BaseDataEnum>>(&data[..]) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Failed to deserialize data: {}", e);
                return Err(Error);
            }
        };

        // Assuming you want to work with the archived data directly, or you can deserialize it further
        Ok(archived_quotebars
            .deserialize(&mut rkyv::Infallible)
            .unwrap())
    }

    /// Serializes a `Vec<PriceDataEnum>` into `AlignedVec`
    pub fn vec_to_aligned(price_data: Vec<BaseDataEnum>) -> AlignedVec {
        // Create a new serializer
        let mut serializer = AllocSerializer::<20971520>::default();

        // Serialize the Vec<QuoteBar>
        serializer.serialize_value(&price_data).unwrap();

        // Get the serialized bytes
        let vec = serializer.into_serializer().into_inner();
        vec
    }

    /// Serializes a `Vec<PriceDataEnum>` into `Vec<u8>`
    /// This is the method used for quote bar request_response
    pub fn vec_to_bytes(price_data: Vec<BaseDataEnum>) -> Vec<u8> {
        // Get the serialized bytes
        let vec = BaseDataEnum::vec_to_aligned(price_data);
        vec.to_vec()
    }
}

impl Bytes<Self> for BaseDataEnum {
    fn from_bytes(archived: &[u8]) -> Result<BaseDataEnum, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<BaseDataEnum>(archived) {
            //Ignore this warning: Trait `Deserialize<ResponseType, SharedDeserializeMap>` is not implemented for `ArchivedRequestType` [E0277]
            Ok(response) => Ok(response),
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(e.to_string())),
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let vec = rkyv::to_bytes::<_, 1024>(self).unwrap();
        vec.into()
    }
}

impl BaseData for BaseDataEnum {
    /// Returns the symbol of the `BaseDataEnum` variant
    fn symbol_name(&self) -> Symbol {
        match self {
            BaseDataEnum::Candle(candle) => candle.symbol.clone(),
            BaseDataEnum::QuoteBar(quote_bar) => quote_bar.symbol.clone(),
            BaseDataEnum::Tick(tick) => tick.symbol.clone(),
            BaseDataEnum::Quote(quote) => quote.symbol.clone(),
            BaseDataEnum::Fundamental(fundamental) => fundamental.symbol.clone(),
        }
    }

    /// Returns the data time at the specified time zone offset as a `DateTime<Tz>`
    fn time_local(&self, time_zone: &Tz) -> DateTime<Tz> {
        time_zone.from_utc_datetime(&self.time_utc().naive_utc())
    }

    /// This is the closing time of the `BaseDataEnum` variant, so for 1 hour quotebar, if the bar opens at 5pm, the closing time will be 6pm and the time_utc will be 6pm. time_object_unchanged(&self) will return the unaltered value.
    fn time_utc(&self) -> DateTime<Utc> {
        match self {
            BaseDataEnum::Candle(candle) => DateTime::from_str(&candle.time).unwrap(),
            BaseDataEnum::QuoteBar(quote_bar) => DateTime::from_str(&quote_bar.time).unwrap(),
            BaseDataEnum::Tick(tick) => DateTime::from_str(&tick.time).unwrap(),
            BaseDataEnum::Quote(quote) => DateTime::from_str(&quote.time).unwrap(),
            BaseDataEnum::Fundamental(fundamental) => {
                DateTime::from_str(&fundamental.time).unwrap()
            }
        }
    }

    fn time_closed_utc(&self) -> DateTime<Utc> {
        match self {
            BaseDataEnum::Candle(candle) => candle.time_utc() + candle.resolution.as_duration(),
            BaseDataEnum::QuoteBar(quote_bar) => {
                quote_bar.time_utc() + quote_bar.resolution.as_duration()
            }
            BaseDataEnum::Tick(tick) => tick.time_utc(),
            BaseDataEnum::Quote(quote) => quote.time_utc(),
            BaseDataEnum::Fundamental(fundamental) => fundamental.time_utc(),
        }
    }

    fn time_closed_local(&self, time_zone: &Tz) -> DateTime<Tz> {
        time_zone.from_utc_datetime(&self.time_closed_utc().naive_utc())
    }

    /// Returns the `DataVendor` enum variant of the `BaseDataEnum` variant object
    fn data_vendor(&self) -> DataVendor {
        match self {
            BaseDataEnum::Candle(candle) => candle.symbol.data_vendor.clone(),
            BaseDataEnum::QuoteBar(quote_bar) => quote_bar.symbol.data_vendor.clone(),
            BaseDataEnum::Tick(tick) => tick.symbol.data_vendor.clone(),
            BaseDataEnum::Quote(quote) => quote.symbol.data_vendor.clone(),
            BaseDataEnum::Fundamental(fundamental) => fundamental.symbol.data_vendor.clone(),
        }
    }

    fn market_type(&self) -> MarketType {
        match self {
            BaseDataEnum::Candle(candle) => candle.symbol.market_type.clone(),
            BaseDataEnum::QuoteBar(quote_bar) => quote_bar.symbol.market_type.clone(),
            BaseDataEnum::Tick(tick) => tick.symbol.market_type.clone(),
            BaseDataEnum::Quote(quote) => quote.symbol.market_type.clone(),
            BaseDataEnum::Fundamental(fundamental) => fundamental.symbol.market_type.clone(),
        }
    }

    fn resolution(&self) -> Resolution {
        match self {
            BaseDataEnum::Candle(candle) => candle.resolution,
            BaseDataEnum::QuoteBar(bar) => bar.resolution,
            // this works because tick candles will be candles not ticks so number is always 1
            BaseDataEnum::Tick(_) => Resolution::Ticks(1),
            BaseDataEnum::Fundamental(data) => data.resolution ,
            _ => Resolution::Instant,
        }
    }

    fn symbol(&self) -> &Symbol {
        match self {
            BaseDataEnum::Candle(candle) => &candle.symbol,
            BaseDataEnum::QuoteBar(quote_bar) => &quote_bar.symbol,
            BaseDataEnum::Tick(tick) => &tick.symbol,
            BaseDataEnum::Quote(quote) => &quote.symbol,
            BaseDataEnum::Fundamental(fundamental) => &fundamental.symbol,
        }
    }

    fn subscription(&self) -> DataSubscription {
        let symbol = self.symbol();
        let resolution = self.resolution();
        let candle_type = match self {
            BaseDataEnum::Candle(candle) => Some(candle.candle_type.clone()),
            BaseDataEnum::QuoteBar(bar) => Some(bar.candle_type.clone()),
            _ => None,
        };
        DataSubscription::from_base_data(
            symbol.name.clone(),
            symbol.data_vendor.clone(),
            resolution,
            self.base_data_type(),
            symbol.market_type.clone(),
            candle_type,
        )
    }
}
