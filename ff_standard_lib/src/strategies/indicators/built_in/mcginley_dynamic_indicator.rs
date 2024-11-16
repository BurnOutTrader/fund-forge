use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use rust_decimal::{Decimal};
use crate::gui_types::settings::Color;
use crate::helpers::decimal_calculators::round_to_tick_size;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::new_types::Price;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::indicators::indicator_values::{IndicatorPlot, IndicatorValues};
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};

/// McGinley Dynamic Indicator
/// A smoothing mechanism that adapts dynamically to market conditions.
///
/// # Plots
/// - "mcginley_dynamic": The main McGinley Dynamic line.
///
/// # Parameters
/// - period: Lookback period for smoothing.
/// - tick_rounding: Whether to round values to tick size.
///
/// # Usage
/// Provides a smooth and responsive trend line with minimal lag, suitable for trend-following strategies.
#[derive(Clone, Debug)]
pub struct McGinleyDynamic {
    name: IndicatorName,
    subscription: DataSubscription,
    history: RollingWindow<IndicatorValues>,
    base_data_history: RollingWindow<BaseDataEnum>,
    decimal_accuracy: u32,
    is_ready: bool,
    period: u64,
    tick_rounding: bool,
    tick_size: Decimal,
    plot_color: Color,
    last_mcginley: Option<Decimal>,
}

impl Display for McGinleyDynamic {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl McGinleyDynamic {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: u64,
        plot_color: Color,
        tick_rounding: bool,
    ) -> Box<Self> {
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(subscription.symbol.name.clone())
            .await
            .unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(subscription.symbol.name.clone())
            .await
            .unwrap();

        Box::new(McGinleyDynamic {
            name,
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period as usize),
            decimal_accuracy,
            is_ready: false,
            period,
            tick_rounding,
            tick_size,
            plot_color,
            last_mcginley: None,
        })
    }

    /// Extract close price from BaseDataEnum.
    /// Specific to McGinley Dynamic Indicator.
    fn get_price(&self, data: &BaseDataEnum) -> Option<Price> {
        match data {
            BaseDataEnum::QuoteBar(bar) => Some(bar.bid_close),
            BaseDataEnum::Candle(candle) => Some(candle.close),
            _ => None,
        }
    }

    /// Apply tick rounding if enabled, otherwise round to decimal accuracy.
    fn round_value(&self, value: Decimal) -> Decimal {
        if self.tick_rounding {
            round_to_tick_size(value, self.tick_size)
        } else {
            value.round_dp(self.decimal_accuracy)
        }
    }

    /// Calculate the McGinley Dynamic value.
    fn calculate_mcginley(
        &self,
        current_price: Decimal,
        last_mcginley: Decimal,
    ) -> Decimal {
        let adjustment = Decimal::from(self.period)
            * ((current_price / last_mcginley).abs() - Decimal::from(1));
        last_mcginley + (current_price - last_mcginley) / (adjustment + Decimal::from(self.period))
    }
}

impl Indicators for McGinleyDynamic {
    fn name(&self) -> IndicatorName {
        self.name.clone()
    }

    fn history_to_retain(&self) -> usize {
        self.history.number.clone() as usize
    }

    fn update_base_data(&mut self, base_data: &BaseDataEnum) -> Option<Vec<IndicatorValues>> {
        if !base_data.is_closed() {
            return None;
        }

        // Extract close price from the base data
        let close = self.get_price(base_data)?;
        self.base_data_history.add(base_data.clone());

        // Ensure sufficient data is available
        if self.base_data_history.len() < self.period as usize {
            return None;
        }

        // Calculate the McGinley Dynamic
        let last_mcginley = self.last_mcginley.unwrap_or(close); // Initialize with the first close price if not set
        let mcginley_dynamic = self.round_value(self.calculate_mcginley(close, last_mcginley));

        // Update the last McGinley Dynamic value
        self.last_mcginley = Some(mcginley_dynamic);

        // Create plots
        let mut plots = BTreeMap::new();
        plots.insert(
            "mcginley_dynamic".to_string(),
            IndicatorPlot::new("McGinley Dynamic".to_string(), mcginley_dynamic, self.plot_color.clone()),
        );

        // Create and store indicator values
        let values = IndicatorValues::new(
            self.name.clone(),
            self.subscription.clone(),
            plots,
            base_data.time_closed_utc(),
        );

        self.history.add(values.clone());
        self.is_ready = true;
        Some(vec![values])
    }

    fn subscription(&self) -> &DataSubscription {
        &self.subscription
    }

    fn reset(&mut self) {
        self.history.clear();
        self.base_data_history.clear();
        self.last_mcginley = None;
        self.is_ready = false;
    }

    fn index(&self, index: usize) -> Option<IndicatorValues> {
        if !self.is_ready {
            return None;
        }
        self.history.get(index).cloned()
    }

    fn current(&self) -> Option<IndicatorValues> {
        if !self.is_ready {
            return None;
        }
        self.history.last().cloned()
    }

    fn plots(&self) -> RollingWindow<IndicatorValues> {
        self.history.clone()
    }

    fn is_ready(&self) -> bool {
        self.is_ready
    }

    fn history(&self) -> RollingWindow<IndicatorValues> {
        self.history.clone()
    }

    fn data_required_warmup(&self) -> u64 {
        self.period
    }
}