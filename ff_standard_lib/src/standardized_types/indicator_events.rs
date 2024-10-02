use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::strategies::indicators::indicators_trait::IndicatorName;
use crate::standardized_types::indicator_values::IndicatorValues;
use std::fmt;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum IndicatorEvents {
    IndicatorAdded(IndicatorName),
    IndicatorRemoved(IndicatorName),
    IndicatorTimeSlice(Vec<IndicatorValues>),
    Replaced(IndicatorName),
}

impl fmt::Display for IndicatorEvents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IndicatorEvents::IndicatorAdded(name) => write!(f, "Indicator added: {}", name),
            IndicatorEvents::IndicatorRemoved(name) => write!(f, "Indicator removed: {}", name),
            IndicatorEvents::IndicatorTimeSlice(values) => {
                for value in values {
                    write!(f, "{}", value)?;
                }
                Ok(())
            },
            IndicatorEvents::Replaced(name) => write!(f, "Indicator replaced: {}", name),
        }
    }
}
