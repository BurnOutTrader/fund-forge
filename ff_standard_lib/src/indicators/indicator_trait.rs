use std::fmt::{Display, Formatter};
use std::str::FromStr;
use chrono::{DateTime, Utc};
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::subscriptions::DataSubscription;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::indicators::built_in::average_true_range::AverageTrueRange;
use crate::standardized_types::rolling_window::RollingWindow;

pub type IndicatorName = String;

/// An enum for all indicators
/// Custom(Box<dyn Indicators + Send + Sync>) is for custom indicators which we want to handle automatically in the engine
pub enum IndicatorEnum {
    //Custom(Box<dyn Indicators + Send + Sync>), if we use this then we cant use rkyv serialization
    AverageTrueRange(AverageTrueRange)
}

impl Indicators for IndicatorEnum {
    fn subscription(&self) -> &DataSubscription {
        match self {
            IndicatorEnum::AverageTrueRange(atr) => atr.subscription()
        }
    }

    fn update_base_data(&mut self, base_data: BaseDataEnum) -> Option<IndicatorValues> {
        match self {
            IndicatorEnum::AverageTrueRange(atr) => atr.update_base_data(base_data)
        }
    }

    fn reset(&mut self) {
        match self {
            IndicatorEnum::AverageTrueRange(atr) => atr.reset()
        }
    }

    fn index(&self, index: usize) -> Option<IndicatorValues> {
        match self {
            IndicatorEnum::AverageTrueRange(atr) => atr.index(index)
        }
    }

    /// returns a rolling window of the indicator, a value is:
    ///  ```rust
    /// use ahash::AHashMap;
    /// use chrono::{DateTime, Utc};
    ///
    /// pub struct IndicatorValue {
    ///     value: f64,
    ///     time: DateTime<Utc>,
    ///     plot_name: String,
    ///    }
    ///
    /// //Results are a AHashMap of results, where the plot can be identified by the IndicatorResult.plot_name name
    /// pub type IndicatorResults = Vec<IndicatorValue>;
    ///
    /// //if you have a rolling window of results for an ATR, you would have only 1 plot name "atr" but if you have a custom indicator with multiple plots like MACD, you would have multiple plot names
    /// ```
    fn plots(&self) -> RollingWindow<IndicatorValues> {
        match self {
            IndicatorEnum::AverageTrueRange(atr) => atr.plots()
        }
    }

    fn is_ready(&self) -> bool {
        match self {
            IndicatorEnum::AverageTrueRange(atr) => atr.is_ready()
        }
    }
}


pub type IndicatorValues = Vec<IndicatorValue>;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
    compare(PartialEq),
    check_bytes,
)]
#[archive_attr(derive(Debug))]
pub struct IndicatorValue {
    value: f64,
    time: String,
    plot_name: String,
}

impl Display for IndicatorValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "IndicatorValue: {{ value: {}, time: {}, label: {} }}", self.value, self.time, self.plot_name)
    }
}

impl IndicatorValue {
    pub fn new(value: f64, time: DateTime<Utc>, plot_name: String) -> IndicatorValue {
        IndicatorValue {
            value,
            time: time.to_string(),
            plot_name
        }
    }
    
    pub fn time_utc(&self) -> DateTime<Utc> {
        DateTime::from_str(&self.time).unwrap()
    }
    
    pub fn timestamp(&self) -> i64 {
        self.time_utc().timestamp()
    }
    
    pub fn value(&self) -> f64 {
        self.value
    }
    
    pub fn plot_name(&self) -> &str {
        &self.plot_name
    }
}

pub trait Indicators {
    /// Returns the name of the indicator
    fn name(&self) -> IndicatorName {
        let mut type_name = std::any::type_name::<Self>();
        type_name = type_name.split("::").last().unwrap();
        let subscription = self.subscription();
        match &subscription.candle_type {
            Some(candle_type) => {
                format!("Average True Range {} {} {} {}: {}", subscription.symbol.name, subscription.symbol.data_vendor, subscription.base_data_type, subscription.resolution, candle_type)
            },
            None => {
                format!("Average True Range {} {} {} {}", subscription.symbol.name, subscription.symbol.data_vendor, subscription.base_data_type, subscription.resolution)
            },
        }
    }
    
    /// Returns the subscription for the indicator
    fn subscription(&self) -> &DataSubscription;
    
    /// Updates the indicator with the new data point.
    fn update_base_data(&mut self, base_data: BaseDataEnum) -> Option<IndicatorValues>;
    
    /// Resets the indicator to its initial state
    fn reset(&mut self);
    
    /// Returns the indicator results at the given index
    fn index(&self, index: usize) -> Option<IndicatorValues>;

    /// returns a rolling window of the indicator, a result is: 
    ///  ```rust
    /// use ahash::AHashMap;
    /// use chrono::{DateTime, Utc};
    ///
    /// pub struct IndicatorResult {
    ///     value: f64,
    ///     time: DateTime<Utc>,
    ///     plot_name: String,
    ///    }
    /// //Results are a AHashMap of results, where the key is the plot name
    /// pub type IndicatorResults = AHashMap<String, IndicatorResult>;
    ///
    /// //if you have a rolling window of results for an ATR, you would have only 1 plot name "atr" but if you have a custom indicator with multiple plots, you would have multiple plot names
    /// ```
    fn plots(&self) -> RollingWindow<IndicatorValues>;
    
    /// Returns true if the indicator is ready
    fn is_ready(&self) -> bool;
}