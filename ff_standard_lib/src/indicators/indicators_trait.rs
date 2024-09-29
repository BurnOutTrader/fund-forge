use crate::indicators::values::IndicatorValues;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;

pub type IndicatorLongName = String;
pub type IndicatorName = String;

pub trait Indicators {
    fn name(&self) -> IndicatorName;

    /// Returns the name of the indicator with the symbol and data vendor, resolution, base data type and candle type where applicable.
    /// example: "Average True Range EUR-USD Test QuoteBar 1D Candle Stick"
    fn long_name(&self) -> IndicatorLongName {
        let subscription = self.subscription();
        match &subscription.candle_type {
            Some(candle_type) => {
                format!(
                    "Average True Range {} {} {} {}: {}",
                    subscription.symbol.name,
                    subscription.symbol.data_vendor,
                    subscription.base_data_type,
                    subscription.resolution,
                    candle_type
                )
            }
            None => {
                format!(
                    "Average True Range {} {} {} {}",
                    subscription.symbol.name,
                    subscription.symbol.data_vendor,
                    subscription.base_data_type,
                    subscription.resolution
                )
            }
        }
    }
    fn history_to_retain(&self) -> usize;
    fn update_base_data(&mut self, base_data: &BaseDataEnum) -> Option<IndicatorValues>;

    /// Returns the subscription for the indicator.
    fn subscription(&self) -> DataSubscription;

    /// Resets the indicator to its initial state.
    fn reset(&mut self);

    /// Returns the indicator results at the given index.
    fn index(&self, index: usize) -> Option<IndicatorValues>;

    /// returns the crrent value, useful for update on tick or price change indicators.
    fn current(&self) -> Option<IndicatorValues>;

    fn plots(&self) -> RollingWindow<IndicatorValues>;

    /// Returns true if the indicator is ready.
    fn is_ready(&self) -> bool;

    fn history(&self) -> RollingWindow<IndicatorValues>;
}
