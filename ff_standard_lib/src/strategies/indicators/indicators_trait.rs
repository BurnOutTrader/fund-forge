use crate::strategies::indicators::indicator_values::IndicatorValues;
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
    /// The number of Indicator data points 'IndicatorValues` to retain in history, this is passed in on creating a new indicator
    fn history_to_retain(&self) -> usize;

    /// This is where the engine sends data
    /// The engine will send open and closed bars so you might want to check  if the base_data.is_closed()
    /// depending on your indicators intentions and logic, open bars will make logic more complex,
    /// Using a BtreeMap<DateTime, BaseDataEnum> where the key time is the open time of the data is a good way to ensure you only have 1 open bar in your history.
    /// Since the open time of a bar never changes, each time the same open bar updates it will replace the last version of itself in the map.
    /// if !base_data.is_closed() {
    ///     return None;
    ///  }
    fn update_base_data(&mut self, base_data: &BaseDataEnum) -> Option<IndicatorValues>;

    /// Returns the subscription for the indicator.
    fn subscription(&self) -> DataSubscription;

    /// Resets the indicator to its initial state.
    fn reset(&mut self);

    /// Returns the indicator results at the given history index.
    fn index(&self, index: usize) -> Option<IndicatorValues>;

    /// returns the current value, useful for update on tick or price change indicators.
    fn current(&self) -> Option<IndicatorValues>;

    fn plots(&self) -> RollingWindow<IndicatorValues>;

    /// Returns true if the indicator is ready.
    fn is_ready(&self) -> bool;

    /// Returns the indicators history, we can still use another data structure to store history like a BTreeMap,
    /// this function would be expensive to use and doesn't server much purpose
    /// since we can just use strategy.indicator_index(u64) to get the historical data.
    fn history(&self) -> RollingWindow<IndicatorValues>;

    /// the number of base data points we need to fill the history on warm up, for example an 5 period ATR indicator that keeps a history of 12 data points will require 17 base data enums to warm up
    fn data_required_warmup(&self) -> u64;
}
