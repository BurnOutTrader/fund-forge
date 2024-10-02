use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::indicators::built_in::average_true_range::AverageTrueRange;
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};
use crate::strategies::indicators::indicator_values::IndicatorValues;

/// An enum for all indicators
/// Custom(Box<dyn Indicators + Send + Sync>) is for custom indicators which we want to handle automatically in the engine
#[derive(Clone, Debug)]
pub enum IndicatorEnum {
    AverageTrueRange(AverageTrueRange),
}

impl Indicators for IndicatorEnum {
    fn name(&self) -> IndicatorName {
        match self {
            IndicatorEnum::AverageTrueRange(atr) => atr.name(),
        }
    }

    fn history_to_retain(&self) -> usize {
        match self {
            IndicatorEnum::AverageTrueRange(indicator) => indicator.history_to_retain(),
        }
    }

    /// We need to be sure to handle open and closed bars in our update method, for example AverageTrueRange will not update when base_data.is_closed() == false
    fn update_base_data(&mut self, base_data: &BaseDataEnum) -> Option<IndicatorValues> {
        match self {
            IndicatorEnum::AverageTrueRange(atr) => atr.update_base_data(base_data),
        }
    }

    fn subscription(&self) -> DataSubscription {
        match self {
            IndicatorEnum::AverageTrueRange(atr) => atr.subscription(),
        }
    }

    fn reset(&mut self) {
        match self {
            IndicatorEnum::AverageTrueRange(atr) => atr.reset(),
        }
    }

    fn index(&self, index: usize) -> Option<IndicatorValues> {
        match self {
            IndicatorEnum::AverageTrueRange(atr) => atr.index(index),
        }
    }

    fn current(&self) -> Option<IndicatorValues> {
        match self {
            IndicatorEnum::AverageTrueRange(atr) => atr.current(),
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
            IndicatorEnum::AverageTrueRange(atr) => atr.plots(),
        }
    }

    fn is_ready(&self) -> bool {
        match self {
            IndicatorEnum::AverageTrueRange(atr) => atr.is_ready(),
        }
    }

    fn history(&self) -> RollingWindow<IndicatorValues> {
        match self {
            IndicatorEnum::AverageTrueRange(atr) => atr.history(),
        }
    }

    fn data_required_warmup(&self) -> u64 {
        match self {
            IndicatorEnum::AverageTrueRange(atr) => atr.data_required_warmup(),
        }
    }
}
