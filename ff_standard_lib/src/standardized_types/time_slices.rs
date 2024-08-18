use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::enums::Resolution;
use crate::standardized_types::subscriptions::{Symbol};

/// A `UnstructuredSlice` is an unordered slice of data that is not yet consolidated into a `TimeSlice`.
pub struct UnstructuredSlice {
    pub data: Vec<u8>,
    pub price_data_type: BaseDataType,
    pub resolution: Resolution,
    pub symbol: Symbol,
}

/// A `TimeSlice` is a consolidated slice of data that is consolidated into a single point in time, you could have 1 hundred Ticks, 1 Quotebar and 3 Candles of different time frames,
/// if they all occurred at the same time, not all the data types will be the same time
pub type TimeSlice = Vec<BaseDataEnum>;



