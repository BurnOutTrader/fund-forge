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

/// Swings Indicator
/// Identifies swing highs and lows over a specified period.
///
/// # Plots
/// - "swing_high": Swing high points.
/// - "swing_low": Swing low points.
///
/// # Parameters
/// - period: The number of bars to look back for determining swings.
/// - tick_rounding: Whether to round values to tick size.
///
/// # Usage
/// Useful for identifying turning points and potential support/resistance areas.
#[derive(Clone, Debug)]
pub struct Swings {
    name: IndicatorName,
    subscription: DataSubscription,
    history: RollingWindow<IndicatorValues>,
    base_data_history: RollingWindow<BaseDataEnum>,
    decimal_accuracy: u32,
    is_ready: bool,
    period: usize,
    tick_rounding: bool,
    tick_size: Decimal,
    plot_color_high: Color,
    plot_color_low: Color,
}

impl Display for Swings {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl Swings {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: usize,
        plot_color_high: Color,
        plot_color_low: Color,
        tick_rounding: bool,
    ) -> Self {
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(subscription.symbol.name.clone())
            .await
            .unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(subscription.symbol.name.clone())
            .await
            .unwrap();

        Swings {
            name,
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period * 2), // Store enough data for period comparison
            decimal_accuracy,
            is_ready: false,
            period,
            tick_rounding,
            tick_size,
            plot_color_high,
            plot_color_low,
        }
    }

    /// Extract high and low prices from BaseDataEnum.
    /// Specific to Swings Indicator.
    fn get_high_low(&self, data: &BaseDataEnum) -> Option<(Price, Price)> {
        match data {
            BaseDataEnum::QuoteBar(bar) => Some((bar.bid_high, bar.bid_low)),
            BaseDataEnum::Candle(candle) => Some((candle.high, candle.low)),
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

    /// Determines if the current bar is a swing high.
    fn is_swing_high(&self, highs: &[Decimal], current: Decimal) -> bool {
        let left = &highs[..self.period];
        let right = &highs[self.period + 1..];
        left.iter().all(|&h| current > h) && right.iter().all(|&h| current > h)
    }

    /// Determines if the current bar is a swing low.
    fn is_swing_low(&self, lows: &[Decimal], current: Decimal) -> bool {
        let left = &lows[..self.period];
        let right = &lows[self.period + 1..];
        left.iter().all(|&l| current < l) && right.iter().all(|&l| current < l)
    }
}

impl Indicators for Swings {
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

        // Extract high and low prices and add to history
        self.base_data_history.add(base_data.clone());

        // Ensure sufficient data is available
        if self.base_data_history.len() < self.period * 2 + 1 {
            return None;
        }

        let highs: Vec<Decimal> = self
            .base_data_history
            .history()
            .iter()
            .filter_map(|data| self.get_high_low(data).map(|(h, _)| h))
            .collect();

        let lows: Vec<Decimal> = self
            .base_data_history
            .history()
            .iter()
            .filter_map(|data| self.get_high_low(data).map(|(_, l)| l))
            .collect();

        let current_high = highs[self.period];
        let current_low = lows[self.period];

        let swing_high = if self.is_swing_high(&highs, current_high) {
            Some(self.round_value(current_high))
        } else {
            None
        };

        let swing_low = if self.is_swing_low(&lows, current_low) {
            Some(self.round_value(current_low))
        } else {
            None
        };

        // Create plots
        let mut plots = BTreeMap::new();
        if let Some(swing_high) = swing_high {
            plots.insert(
                "swing_high".to_string(),
                IndicatorPlot::new("Swing High".to_string(), swing_high, self.plot_color_high.clone()),
            );
        }
        if let Some(swing_low) = swing_low {
            plots.insert(
                "swing_low".to_string(),
                IndicatorPlot::new("Swing Low".to_string(), swing_low, self.plot_color_low.clone()),
            );
        }

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
        (self.period * 2) as u64
    }
}