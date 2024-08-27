use async_trait::async_trait;
use crate::indicators::values::IndicatorValues;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;

pub type IndicatorLongName = String;
pub type IndicatorName = String;



#[async_trait]
pub trait AsyncIndicators: Indicators {
    /// Updates the indicator with the new data point.
    /// be aware you need to make update_base_data().await and async fn and use a tokio::Mutex to lock to the type when locking, since this type depends upon interior mutability
    /// without a lock we have a potential race condition where the timeslice contains multiple data points for a subscription, 
    /// to cheaply resolve this we simply use a mutex lock inside our type to prevent multple calls to update being ran at same moment
    ///
    /// ### Example
    /// ```rust
    /// use ff_standard_lib::indicators::indicators_trait::IndicatorName;
    /// use ff_standard_lib::indicators::values::IndicatorValues;
    /// use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
    /// use ff_standard_lib::standardized_types::rolling_window::RollingWindow;
    /// use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
    ///
    /// pub struct AverageTrueRange {
    ///     name: IndicatorName,
    ///     subscription: DataSubscription,
    ///    history: RollingWindow<IndicatorValues>,
    ///     base_data_history: RollingWindow<BaseDataEnum>,
    ///     is_ready: bool,
    ///     period: u64,
    ///     tick_size: f64,
    ///     lock: tokio::sync::Mutex<()>
    /// }
    /// ```
    async fn update_base_data(&mut self, base_data: &BaseDataEnum) -> Option<IndicatorValues>;
}

pub trait Indicators {
    
    fn name(&self) -> IndicatorName;

    /// Returns the name of the indicator with the symbol and data vendor, resolution, base data type and candle type where applicable.
    /// example: "Average True Range EUR-USD Test QuoteBar 1D Candle Stick"
    fn long_name(&self) -> IndicatorLongName {
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

    fn history(&self) -> RollingWindow<IndicatorValues>;
}

