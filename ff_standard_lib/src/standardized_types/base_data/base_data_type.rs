use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::base_data::quote::Quote;
use crate::standardized_types::base_data::quotebar::QuoteBar;
use crate::standardized_types::base_data::tick::Tick;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::fmt;
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
    Quotes = 1,
    QuoteBars = 2,
    Candles = 3,
    Fundamentals = 4,
}
impl BaseDataType {
    // Function to get_requests the TypeId of the associated data type
    pub fn get_type_id(&self) -> TypeId {
        match self {
            BaseDataType::Ticks => TypeId::of::<Tick>(),
            BaseDataType::Quotes => TypeId::of::<Quote>(),
            BaseDataType::QuoteBars => TypeId::of::<QuoteBar>(),
            BaseDataType::Candles => TypeId::of::<Candle>(),
            BaseDataType::Fundamentals => TypeId::of::<String>(),
            //BaseDataType::OrderBooks => TypeId::of::<OrderBook>(),
        }
    }

    // Convert from string to BaseDataType
    pub fn from_str(string_ref: &str) -> Result<Self, String> {
        match string_ref.to_lowercase().as_str() {
            "Ticks" => Ok(BaseDataType::Ticks),
            "Quotes" => Ok(BaseDataType::Quotes),
            "Quotebars" => Ok(BaseDataType::QuoteBars),
            "Candles" => Ok(BaseDataType::Candles),
            "Fundamentals" => Ok(BaseDataType::Fundamentals),
            // "order books" => Ok(BaseDataType::OrderBooks),
            _ => Err(format!("Unknown BaseDataType: {}", string_ref)),
        }
    }

    // Convert from BaseDataType to string
    pub fn to_string(&self) -> String {
        match self {
            BaseDataType::Ticks => "Ticks".to_string(),
            BaseDataType::Quotes => "Quotes".to_string(),
            BaseDataType::QuoteBars => "Quotebars".to_string(),
            BaseDataType::Candles => "Candles".to_string(),
            BaseDataType::Fundamentals => "Fundamentals".to_string(),
            //BaseDataType::OrderBooks => "order books".to_string(),
        }
    }
}

// Implement Display for BaseDataType
impl Display for BaseDataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}