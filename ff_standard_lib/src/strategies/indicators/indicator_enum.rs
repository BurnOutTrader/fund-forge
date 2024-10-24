use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::indicators::built_in::average_true_range::AverageTrueRange;
use crate::strategies::indicators::built_in::renko::Renko;
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};
use crate::strategies::indicators::indicator_values::IndicatorValues;

/// An enum for all indicators
/// Custom(Box<dyn Indicators + Send + Sync>) is for custom indicators which we want to handle automatically in the engine
#[derive(Clone, Debug)]
pub enum IndicatorEnum {
    AverageTrueRange(AverageTrueRange),
    Renko(Renko)
}

impl Indicators for IndicatorEnum {
    fn name(&self) -> IndicatorName {
        match self {
            IndicatorEnum::AverageTrueRange(indicator) => indicator.name(),
            IndicatorEnum::Renko(indicator) => indicator.name(),
        }
    }

    fn history_to_retain(&self) -> usize {
        match self {
            IndicatorEnum::AverageTrueRange(indicator) => indicator.history_to_retain(),
            IndicatorEnum::Renko(indicator) => indicator.history_to_retain(),
        }
    }

    /// We need to be sure to handle open and closed bars in our update method, for example AverageTrueRange will not update when historical.is_closed() == false
    fn update_base_data(&mut self, base_data: &BaseDataEnum) -> Option<Vec<IndicatorValues>> {
        match self {
            IndicatorEnum::AverageTrueRange(indicator) => indicator.update_base_data(base_data),
            IndicatorEnum::Renko(indicator) => indicator.update_base_data(base_data),
        }
    }

    fn subscription(&self) -> &DataSubscription {
        match self {
            IndicatorEnum::AverageTrueRange(indicator) => indicator.subscription(),
            IndicatorEnum::Renko(indicator) => indicator.subscription(),
        }
    }

    fn reset(&mut self) {
        match self {
            IndicatorEnum::AverageTrueRange(indicator) => indicator.reset(),
            IndicatorEnum::Renko(indicator) => indicator.reset(),
        }
    }

    fn index(&self, index: usize) -> Option<IndicatorValues> {
        match self {
            IndicatorEnum::AverageTrueRange(indicator) => indicator.index(index),
            IndicatorEnum::Renko(indicator) => indicator.index(index),
        }
    }

    fn current(&self) -> Option<IndicatorValues> {
        match self {
            IndicatorEnum::AverageTrueRange(indicator) => indicator.current(),
            IndicatorEnum::Renko(indicator) => indicator.current(),
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
            IndicatorEnum::AverageTrueRange(indicator) => indicator.plots(),
            IndicatorEnum::Renko(indicator) => indicator.plots(),
        }
    }

    fn is_ready(&self) -> bool {
        match self {
            IndicatorEnum::AverageTrueRange(indicator) => indicator.is_ready(),
            IndicatorEnum::Renko(indicator) => indicator.is_ready(),
        }
    }

    fn history(&self) -> RollingWindow<IndicatorValues> {
        match self {
            IndicatorEnum::AverageTrueRange(indicator) => indicator.history(),
            IndicatorEnum::Renko(indicator) => indicator.history(),
        }
    }

    fn data_required_warmup(&self) -> u64 {
        match self {
            IndicatorEnum::AverageTrueRange(indicator) => indicator.data_required_warmup(),
            IndicatorEnum::Renko(indicator) => indicator.data_required_warmup(),
        }
    }
}
