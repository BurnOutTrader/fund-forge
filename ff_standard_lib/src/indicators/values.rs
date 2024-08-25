use std::fmt::{Display, Formatter};
use std::str::FromStr;
use ahash::AHashMap;
use chrono::{DateTime, FixedOffset, Utc};
use chrono_tz::Tz;
use crate::helpers::converters::time_convert_utc_datetime_to_fixed_offset;
use crate::indicators::indicators_trait::IndicatorName;
use crate::standardized_types::subscriptions::DataSubscription;

pub type PlotName = String;

#[derive(Debug, Clone)]
pub struct IndicatorValues {
    time: String,
    pub indicator_name: IndicatorName,
    pub subscription: DataSubscription,
    values: AHashMap<PlotName, f64>
}

impl Display for IndicatorValues {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut values_string = String::new();
        for (key, value) in &self.values {
            values_string.push_str(&format!("{}: {}\n", key, value));
        }
        write!(f, "{}, {}, {}", self.indicator_name, self.subscription, values_string)
    }
}

impl IndicatorValues {
    pub fn new(indicator_name: IndicatorName, subscription: DataSubscription, values: AHashMap<PlotName, f64>, time: DateTime<Utc>) -> Self {
        Self {
            indicator_name,
            subscription,
            values,
            time: time.to_string()
        }
    }

    pub fn time_utc(&self) -> DateTime<Utc> {
        DateTime::from_str(&self.time).unwrap()
    }

    pub fn time_local(&self, time_zone: &Tz) -> DateTime<FixedOffset> {
        time_convert_utc_datetime_to_fixed_offset(time_zone, self.time_utc())
    }

    pub fn get_plot(&self, plot_name: &str) -> Option<f64> {
        self.values.get(plot_name).cloned()
    }

    pub fn insert(&mut self, plot_name: PlotName, value: f64) {
        self.values.insert(plot_name, value);
    }
}