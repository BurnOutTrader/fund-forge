use std::fmt::{Display, Formatter};
use std::str::FromStr;
use ahash::AHashMap;
use chrono::{DateTime, FixedOffset, Utc};
use chrono_tz::Tz;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::helpers::converters::time_convert_utc_datetime_to_fixed_offset;
use crate::indicators::indicators_trait::IndicatorName;
use crate::standardized_types::subscriptions::DataSubscription;

pub type PlotName = String;
pub struct ArchivedAHashMap(AHashMap<String, f64>);

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
    compare(PartialEq),
    check_bytes,
)]
#[archive_attr(derive(Debug))]
pub struct IndicatorValue {
    pub name: PlotName,
    pub value: f64,
}

/// A struct that represents the values of an indicator at a specific time.
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
    compare(PartialEq),
    check_bytes,
)]
#[archive_attr(derive(Debug))]
pub struct IndicatorValues {
    name: IndicatorName,
    time: String,
    subscription: DataSubscription,
    values: Vec<IndicatorValue>
}

impl Display for IndicatorValues {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut values_string = String::new();
        for plot in &self.values {
            values_string.push_str(&format!("{}: {}\n", plot.name, plot.value));
        }
        write!(f, "{}, {}, {}", self.name, self.subscription, values_string)
    }
}

impl IndicatorValues {
    pub fn new(name: IndicatorName, subscription: DataSubscription, values: AHashMap<PlotName, f64>, time: DateTime<Utc>) -> Self {
        let mut modified_values = Vec::new();
        for (key, value) in values {
            let plot = IndicatorValue {
                name: key,
                value
            };
            modified_values.push(plot);
        }
        Self {
            name,
            subscription,
            values: modified_values,
            time: time.to_string()
        }
    }
    
    pub fn name(&self) -> &IndicatorName {
        &self.name
    }

    /// get the time in the UTC time zone
    pub fn time_utc(&self) -> DateTime<Utc> {
        DateTime::from_str(&self.time).unwrap()
    }

    /// get the time in the local time zone
    pub fn time_local(&self, time_zone: &Tz) -> DateTime<FixedOffset> {
        time_convert_utc_datetime_to_fixed_offset(time_zone, self.time_utc())
    }

    /// get the value of a plot by name
    pub fn get_plot(&self, plot_name: &PlotName) -> Option<f64> {
        for plot in &self.values {
            if plot.name == *plot_name {
                return Some(plot.value);
            }
        }
        None
    }
    
    /// get all the values `values: &AHashMap<PlotName, f64>`
    pub fn values(&self) -> AHashMap<PlotName, f64> {
        let mut values = AHashMap::new();
        for plot in &self.values {
            values.insert(plot.name.clone(), plot.value);
        }
        values
    }

    /// insert a value into the values
    pub(crate) fn insert(&mut self, plot_name: PlotName, value: f64) {
        let plot = IndicatorValue {
            name: plot_name,
            value
        };
        self.values.push(plot);
    }
}