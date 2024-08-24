use std::str::FromStr;
use ahash::AHashMap;
use chrono::{DateTime, Utc};

use crate::rolling_window::RollingWindow;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::subscriptions::DataSubscription;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
/*
pub type IndicatorResults = Vec<IndicatorResult>;
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
    compare(PartialEq),
    check_bytes,
)]
#[archive_attr(derive(Debug))]
pub struct IndicatorResult {
    value: f64,
    time: String,
    plot_name: String,
}

impl IndicatorResult {
    pub fn new(value: f64, time: DateTime<Utc>, plot_name: String) -> IndicatorResult {
        IndicatorResult {
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

/// An enum for all indicators
/// Custom(Box<dyn Indicators + Send + Sync>) is for custom indicators which we want to handle automatically in the engine
pub enum IndicatorEnum {
    //Custom(Box<dyn Indicators + Send + Sync>),
    AverageTrueRange(AverageTrueRange)
}

impl IndicatorEnum {
    fn subscription(&self) -> &DataSubscription {
        match self {
            //IndicatorEnum::Custom(dyn_indicator) => dyn_indicator.subscription(),
            IndicatorEnum::AverageTrueRange(atr) => atr.subscription()
        }
    }

    fn update(&mut self, base_data: BaseDataEnum) -> Option<IndicatorResults> {
        match self {
            //IndicatorEnum::Custom(dyn_indicator) => dyn_indicator.update(base_data),
            IndicatorEnum::AverageTrueRange(atr) => atr.update(base_data)
        }
    }

    fn reset(&mut self) {
        match self {
            //IndicatorEnum::Custom(dyn_indicator) => dyn_indicator.reset(),
            IndicatorEnum::AverageTrueRange(atr) => atr.reset()
        }
    }

    fn index(&self, index: usize) -> Option<IndicatorResults> {
        match self {
            //IndicatorEnum::Custom(dyn_indicator) => dyn_indicator.index(index),
            IndicatorEnum::AverageTrueRange(atr) => atr.index(index)
        }
    }

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
    /// //Results are a AHashMap of results, where the plot can be identified by the IndicatorResult.plot_name name
    /// pub type IndicatorResults = Vec<IndicatorResult>;
    /// 
    /// //if you have a rolling window of results for an ATR, you would have only 1 plot name "atr" but if you have a custom indicator with multiple plots, you would have multiple plot names
    /// ```
    fn plots(&self) -> RollingWindow<IndicatorResults> {
        match self {
            //IndicatorEnum::Custom(dyn_indicator) => dyn_indicator.plots(),
            IndicatorEnum::AverageTrueRange(atr) => atr.plots()
        }
    }

    fn is_ready(&self) -> bool {
        match self {
            //IndicatorEnum::Custom(dyn_indicator) => dyn_indicator.is_ready(),
            IndicatorEnum::AverageTrueRange(atr) => atr.is_ready()
        }
    }
}*/
/*

pub trait Indicators {
    
    /// Returns the name of the indicator
    fn name(&self) -> String {
        let mut type_name = std::any::type_name::<Self>();
        type_name = type_name.split("::").last().unwrap();
        let subscription = self.subscription();
        let symbol = subscription.symbol.clone();
        let resolution = subscription.resolution.to_string();
        let vendor = symbol.data_vendor.clone();
        format!("{}-{}-{}-{}", &type_name, &symbol.name, &resolution, &vendor)
    }
    
    /// Returns the subscription for the indicator
    fn subscription(&self) -> &DataSubscription;
    
    /// Updates the indicator with the new data point.
    fn update(&mut self, base_data: BaseDataEnum) -> Option<IndicatorResults>;
    
    /// Resets the indicator to its initial state
    fn reset(&mut self);
    
    /// Returns the indicator results at the given index
    fn index(&self, index: usize) -> Option<IndicatorResults>;

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
    fn plots(&self) -> RollingWindow<IndicatorResults>;
    
    /// Returns true if the indicator is ready
    fn is_ready(&self) -> bool;
}*/