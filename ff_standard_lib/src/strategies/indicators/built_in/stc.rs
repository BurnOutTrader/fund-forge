use std::collections::BTreeMap;
use rust_decimal::{Decimal};
use rust_decimal::prelude::{One, Zero};
use crate::gui_types::settings::Color;
use crate::helpers::decimal_calculators::round_to_tick_size;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::indicators::indicator_values::{IndicatorPlot, IndicatorValues};
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};

/// Schaff Trend Cycle (STC)
/// Combines MACD and cycle analysis for early trend detection.
///
/// # Plots
/// - "stc": The main Schaff Trend Cycle line.
///
/// # Parameters
/// - macd_fast: Fast EMA period for MACD.
/// - macd_slow: Slow EMA period for MACD.
/// - cycle_period: Lookback period for cycle analysis.
///
/// # Usage
/// Detects trends early with reduced lag.
#[derive(Clone, Debug)]
pub struct SchaffTrendCycle {
    name: IndicatorName,
    subscription: DataSubscription,
    history: RollingWindow<IndicatorValues>,
    base_data_history: RollingWindow<BaseDataEnum>,
    decimal_accuracy: u32,
    is_ready: bool,
    macd_fast: usize,
    macd_slow: usize,
    cycle_period: usize,
    tick_rounding: bool,
    tick_size: Decimal,
    plot_color: Color,
    macd_values: Vec<Decimal>,
    stc_values: Vec<Decimal>,
}
impl SchaffTrendCycle {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        macd_fast: usize,
        macd_slow: usize,
        cycle_period: usize,
        plot_color: Color,
        tick_rounding: bool,
    ) -> Box<Self> {
        let decimal_accuracy = subscription
            .symbol
            .data_vendor
            .decimal_accuracy(subscription.symbol.name.clone())
            .await
            .unwrap();
        let tick_size = subscription
            .symbol
            .data_vendor
            .tick_size(subscription.symbol.name.clone())
            .await
            .unwrap();

        Box::new(SchaffTrendCycle {
            name,
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(macd_slow),
            decimal_accuracy,
            is_ready: false,
            macd_fast,
            macd_slow,
            cycle_period,
            tick_rounding,
            tick_size,
            plot_color,
            macd_values: Vec::new(),
            stc_values: Vec::new(),
        })
    }

    /// Round a value to the tick size or decimal accuracy, depending on configuration.
    fn round_value(&self, value: Decimal) -> Decimal {
        if self.tick_rounding {
            round_to_tick_size(value, self.tick_size)
        } else {
            value.round_dp(self.decimal_accuracy)
        }
    }

    /// Extract the close price from BaseDataEnum.
    fn get_close(&self, data: &BaseDataEnum) -> Option<Decimal> {
        match data {
            BaseDataEnum::Candle(candle) => Some(candle.close),
            BaseDataEnum::QuoteBar(bar) => Some(bar.bid_close),
            _ => None,
        }
    }

    /// Calculate the MACD value (EMA fast - EMA slow).
    fn calculate_macd(&self, closes: &[Decimal]) -> Decimal {
        let fast_period = self.macd_fast;
        let slow_period = self.macd_slow;

        let fast_ema = Self::calculate_ema(closes, fast_period);
        let slow_ema = Self::calculate_ema(closes, slow_period);

        self.round_value(fast_ema - slow_ema)
    }

    /// Helper function to calculate an EMA (Exponential Moving Average).
    fn calculate_ema(prices: &[Decimal], period: usize) -> Decimal {
        if prices.len() < period {
            return Decimal::zero();
        }

        let multiplier = Decimal::from(2) / (Decimal::from(period) + Decimal::one());
        let mut ema = prices[0];

        for price in prices.iter().skip(1) {
            ema = (price - ema) * multiplier + ema;
        }

        ema
    }

    /// Calculate the Schaff Trend Cycle (STC).
    fn calculate_stc(&mut self, macd: Decimal) -> Decimal {
        // Append MACD value to macd_values.
        self.macd_values.push(macd);
        if self.macd_values.len() > self.cycle_period {
            self.macd_values.remove(0);
        }

        // Calculate stochastic on MACD values.
        let stoch_high = self.macd_values.iter().cloned().max().unwrap_or(macd);
        let stoch_low = self.macd_values.iter().cloned().min().unwrap_or(macd);

        if stoch_high == stoch_low {
            return Decimal::zero();
        }

        let stochastic = (macd - stoch_low) / (stoch_high - stoch_low) * Decimal::from(100);

        // Smooth the stochastic value and append to stc_values.
        self.stc_values.push(stochastic);
        if self.stc_values.len() > self.cycle_period {
            self.stc_values.remove(0);
        }

        // Return the smoothed STC value.
        let smoothed_stc = Self::calculate_ema(&self.stc_values, self.cycle_period);
        self.round_value(smoothed_stc)
    }
}

impl Indicators for SchaffTrendCycle {
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

        // Add the current base data to history.
        self.base_data_history.add(base_data.clone());

        // Ensure there is enough data for calculation.
        if self.base_data_history.len() < self.macd_slow + self.cycle_period {
            return None;
        }

        // Gather all close prices for MACD calculation.
        let closes: Vec<Decimal> = self
            .base_data_history
            .history()
            .iter()
            .filter_map(|data| self.get_close(data))
            .collect();

        let macd = self.calculate_macd(&closes);
        let stc = self.calculate_stc(macd);

        // Create plots.
        let mut plots = BTreeMap::new();
        plots.insert(
            "stc".to_string(),
            IndicatorPlot::new("STC".to_string(), stc, self.plot_color.clone()),
        );

        // Create and store indicator values.
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
        self.macd_values.clear();
        self.stc_values.clear();
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
        (self.macd_slow + self.cycle_period) as u64
    }
}