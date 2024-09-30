use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::enums::Resolution;
use crate::standardized_types::subscriptions::Symbol;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};

/// A `UnstructuredSlice` is an unordered slice of data that is not yet consolidated into a `TimeSlice`.
#[derive(Clone)]
pub struct UnstructuredSlice {
    pub data: Vec<u8>,
    pub price_data_type: BaseDataType,
    pub resolution: Resolution,
    pub symbol: Symbol,
}

/// A `TimeSlice` is a consolidated slice of data that is consolidated into a single point in time, you could have 1 hundred Ticks, 1 Quotebar and 3 Candles of different time frames,
/// if they all occurred at the same time, not all the data types will be the same time
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct TimeSlice {
    data: Vec<BaseDataEnum>,  // Internal vector holding the data
}


impl FromIterator<BaseDataEnum> for TimeSlice {
    fn from_iter<I: IntoIterator<Item = BaseDataEnum>>(iter: I) -> Self {
        let mut slice = TimeSlice {
            data: Vec::from_iter(iter),
        };
        // Sort the slice by timestamp after collecting
        slice.data.sort_by_key(|base_data| base_data.time_closed_utc());
        slice
    }
}

impl TimeSlice {
    // Create a new empty TimeSlice
    pub fn new() -> Self {
        TimeSlice { data: Vec::new() }
    }

    // Insert a new BaseDataEnum and keep the slice sorted
    pub fn add(&mut self, item: BaseDataEnum) {
        self.data.push(item);
        self.data.sort_by_key(|base_data| base_data.time_closed_utc()); // Sort by the timestamp after insertion
    }

    // Insert a collection of BaseDataEnum, ensuring sorted order
    pub fn extend(&mut self, slice: TimeSlice) {
        self.data.extend(slice.data);
        self.data.sort_by_key(|base_data| base_data.time_closed_utc()); // Sort after adding multiple items
    }

    // Access the sorted data as an iterator
    pub fn iter(&self) -> impl Iterator<Item = &BaseDataEnum> {
        self.data.iter()  // Return an iterator over the sorted data
    }

    // Get the earliest (first) item, if it exists
    pub fn first(&self) -> Option<&BaseDataEnum> {
        self.data.first()
    }

    // Get the latest (last) item, if it exists
    pub fn last(&self) -> Option<&BaseDataEnum> {
        self.data.last()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Returns an owned iterator for all items of the specified `BaseDataType`.
    pub fn get_by_type(self, data_type: BaseDataType) -> impl Iterator<Item = BaseDataEnum> {
        self.data.into_iter().filter(move |item| match (data_type, item) {
            (BaseDataType::Ticks, BaseDataEnum::Tick(_)) => true,
            (BaseDataType::Quotes, BaseDataEnum::Quote(_)) => true,
            (BaseDataType::QuoteBars, BaseDataEnum::QuoteBar(_)) => true,
            (BaseDataType::Candles, BaseDataEnum::Candle(_)) => true,
            (BaseDataType::Fundamentals, BaseDataEnum::Fundamental(_)) => true,
            _ => false,
        })
    }

    /// Returns a borrowed iterator for all items of the specified `BaseDataType`.
    pub fn get_by_type_borrowed(&self, data_type: BaseDataType) -> impl Iterator<Item = &BaseDataEnum> {
        self.data.iter().filter(move |item| match (data_type, item) {
            (BaseDataType::Ticks, BaseDataEnum::Tick(_)) => true,
            (BaseDataType::Quotes, BaseDataEnum::Quote(_)) => true,
            (BaseDataType::QuoteBars, BaseDataEnum::QuoteBar(_)) => true,
            (BaseDataType::Candles, BaseDataEnum::Candle(_)) => true,
            (BaseDataType::Fundamentals, BaseDataEnum::Fundamental(_)) => true,
            _ => false,
        })
    }
}
