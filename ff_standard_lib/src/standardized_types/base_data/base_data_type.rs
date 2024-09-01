use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::base_data::price::TradePrice;
use crate::standardized_types::base_data::quote::Quote;
use crate::standardized_types::base_data::quotebar::QuoteBar;
use crate::standardized_types::base_data::tick::Tick;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::fmt::Display;

#[derive(
    Serialize,
    Deserialize,
    Clone,
    Serialize_rkyv,
    Deserialize_rkyv,
    Archive,
    PartialEq,
    Debug,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Copy,
)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
/// The `BaseDataType` enum is used to specify the type of data that is being fed into the algorithm. \
/// Not to be confused with the `DataType` enum which is used to specify the type of data that is being requested from the data provider. \
/// This enum is used to form `NetworkMessage` objects which are used to request and send data to and from your running `ff_data_server` instance.
pub enum BaseDataType {
    Ticks = 0,
    /// A `Quote` is a data point that represents the current bid and ask prices for a given symbol.
    Quotes = 1,
    /// A `Price` is a data point that represents the current price or last traded of a given symbol.
    TradePrices = 2,
    QuoteBars = 3,
    Candles = 4,
    Fundamentals = 5,
    //OrderBooks = 6,
}

impl BaseDataType {
    pub fn get_type_id(&self) -> TypeId {
        match self {
            BaseDataType::Ticks => TypeId::of::<Tick>(),
            BaseDataType::Quotes => TypeId::of::<Quote>(),
            BaseDataType::TradePrices => TypeId::of::<TradePrice>(),
            BaseDataType::QuoteBars => TypeId::of::<QuoteBar>(),
            BaseDataType::Candles => TypeId::of::<Candle>(),
            BaseDataType::Fundamentals => TypeId::of::<String>(),
            //BaseDataType::OrderBooks => TypeId::of::<OrderBook>(),
        }
    }
}

impl Display for BaseDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BaseDataType::Ticks => write!(f, "ticks"),
            BaseDataType::Quotes => write!(f, "quotes"),
            BaseDataType::TradePrices => write!(f, "prices"),
            BaseDataType::QuoteBars => write!(f, "quotebars"),
            BaseDataType::Candles => write!(f, "candles"),
            BaseDataType::Fundamentals => write!(f, "fundamentals"),
            //BaseDataType::OrderBooks => write!(f, "order books"),
        }
    }
}
