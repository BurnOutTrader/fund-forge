use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::indicators::indicators_trait::IndicatorName;
use crate::indicators::values::IndicatorValues;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum IndicatorEvents {
    IndicatorAdded(IndicatorName),
    IndicatorRemoved(IndicatorName),
    IndicatorTimeSlice(Vec<IndicatorValues>),
    Replaced(IndicatorName),
}