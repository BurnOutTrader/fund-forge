use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::traits::BaseData;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::messages::data_server_messaging::FundForgeError;
use crate::standardized_types::bytes_trait::Bytes;
use crate::standardized_types::subscriptions::DataSubscription;

/// A `TimeSlice` is a consolidated slice of data that is consolidated into a single point in time, you could have 1 hundred Ticks, 1 Quotebar and 3 Candles of different time frames,
/// if they all occurred at the same time, not all the data types will be the same time
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct TimeSlice {
    data: BTreeMap<i64, Vec<BaseDataEnum>>,
}

impl Bytes<Self> for TimeSlice {
    fn from_bytes(archived: &[u8]) -> Result<TimeSlice, FundForgeError> {
        match rkyv::from_bytes::<TimeSlice>(archived) {
            Ok(response) => Ok(response),
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(e.to_string())),
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        rkyv::to_bytes::<_, 1024>(self).unwrap().into()
    }
}

impl FromIterator<BaseDataEnum> for TimeSlice {
    fn from_iter<I: IntoIterator<Item = BaseDataEnum>>(iter: I) -> Self {
        let mut slice = TimeSlice::new();
        for item in iter {
            slice.add(item);
        }
        slice
    }
}

impl TimeSlice {
    pub fn new() -> Self {
        TimeSlice { data: BTreeMap::new() }
    }

    pub fn add(&mut self, item: BaseDataEnum) {
        let time = item.time_closed_utc().timestamp_nanos_opt().unwrap();
        self.data.entry(time).or_insert_with(Vec::new).push(item);
    }

    pub fn extend(&mut self, slice: TimeSlice) {
        for (time, mut items) in slice.data {
            self.data.entry(time).or_insert_with(Vec::new).append(&mut items);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &BaseDataEnum> {
        self.data.values().flat_map(|v| v.iter())
    }

    pub fn first(&self) -> Option<&BaseDataEnum> {
        self.data.values().next().and_then(|v| v.first())
    }

    pub fn last(&self) -> Option<&BaseDataEnum> {
        self.data.values().next_back().and_then(|v| v.last())
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn get_by_type(self, data_type: BaseDataType) -> impl Iterator<Item = BaseDataEnum> {
        self.data.into_iter().flat_map(move |(_, items)| {
            items.into_iter().filter(move |item| item.base_data_type() == data_type)
        })
    }

    pub fn get_by_subscription<'a>(
        self,
        subscription: &'a DataSubscription
    ) -> impl Iterator<Item = BaseDataEnum> + 'a {
        self.data.into_iter().flat_map(move |(_, items)| {
            let value = subscription.clone(); // Cloning helps avoid borrowing issues
            items.into_iter().filter(move |item| item.subscription() == value)
        })
    }

    pub fn get_by_type_borrowed(&self, data_type: BaseDataType) -> impl Iterator<Item = &BaseDataEnum> {
        self.data.values().flat_map(move |items| {
            items.iter().filter(move |item| item.base_data_type() == data_type)
        })
    }

    // New method to get an efficient iterator over all items
    pub fn iter_ordered(&self) -> impl Iterator<Item = (&i64, &BaseDataEnum)> {
        self.data.iter().flat_map(|(time, items)| {
            items.iter().map(move |item| (time, item))
        })
    }

    pub fn merge(&mut self, other: TimeSlice) {
        for (time, mut items) in other.data {
            match self.data.entry(time) {
                Entry::Vacant(entry) => {
                    // If we don't have data for this timestamp, insert directly
                    entry.insert(items);
                },
                Entry::Occupied(mut entry) => {
                    // If we have existing data, append the new items
                    entry.get_mut().append(&mut items);
                }
            }
        }
    }
}
