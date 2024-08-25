use std::fmt::{Display};
use crate::indicators::values::IndicatorValues;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;

pub type IndicatorLongName = String;
pub type IndicatorName = String;

pub trait Indicators {
    
    /// Returns the short name of the indicator.
    /// example: "AverageTrueRange"
    fn short_name(&self) -> IndicatorName {
        let mut type_name = std::any::type_name::<Self>();
        type_name = type_name.split("::").last().unwrap();
        type_name.to_string()
    }

    /// Returns the name of the indicator with the symbol and data vendor, resolution, base data type and candle type where applicable.
    /// example: "Average True Range EUR-USD Test QuoteBar 1D Candle Stick"
    fn long_name(&self) -> IndicatorLongName {
        let mut type_name = std::any::type_name::<Self>();
        type_name = self.short_name().as_str();
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

    /// Returns the subscription for the indicator.
    fn subscription(&self) -> DataSubscription;

    /// Updates the indicator with the new data point.
    fn update_base_data(&mut self, base_data: BaseDataEnum) -> Option<IndicatorValues>;

    /// Resets the indicator to its initial state.
    fn reset(&mut self);

    /// Returns the indicator results at the given index.
    fn index(&self, index: u64) -> Option<IndicatorValues>;

    /// returns the crrent value, useful for update on tick or price change indicators.
    fn current(&self) -> Option<IndicatorValues>;

    /// returns a rolling window of the indicator, a result is: 
    ///  ```rust
    /// use ahash::AHashMap;
    /// use chrono::{DateTime, Utc};
    ///
    /// pub struct IndicatorValue {
    ///     value: f64,
    ///     time: DateTime<Utc>,
    ///     plot_name: String,
    ///    }
    /// //Results are a AHashMap of results, where the key is the plot name
    /// pub type IndicatorResults = AHashMap<String, IndicatorValue>;
    ///
    /// //if you have a rolling window of results for an ATR, you would have only 1 plot name "atr" but if you have a custom indicator with multiple plots, you would have multiple plot names
    /// ```
    fn plots(&self) -> RollingWindow<IndicatorValues>;

    /// Returns true if the indicator is ready.
    fn is_ready(&self) -> bool;
}

