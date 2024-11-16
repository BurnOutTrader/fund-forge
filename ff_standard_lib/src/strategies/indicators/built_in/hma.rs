use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use rust_decimal::{Decimal};
use crate::gui_types::settings::Color;
use crate::helpers::decimal_calculators::round_to_tick_size;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::new_types::{Price};
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::indicators::indicator_values::{IndicatorPlot, IndicatorValues};
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};

/// Hull Moving Average (HMA)
/// A weighted moving average designed to be smooth and responsive, reducing lag.
///
/// # Plots
/// - "hma": The main Hull Moving Average line.
///
/// # Parameters
/// - period: Number of periods for the HMA calculation.
/// - tick_rounding: Whether to round values to tick size.
///
/// # Usage
/// Provides smooth trend-following signals with minimal lag, suitable for identifying reversals or continuation patterns.
#[derive(Clone, Debug)]
pub struct HullMovingAverage {
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
}

impl Display for HullMovingAverage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl HullMovingAverage {
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: u64,
        plot_color: Color,
        tick_rounding: bool,
    ) -> Self {
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(subscription.symbol.name.clone())
            .await
            .unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(subscription.symbol.name.clone())
            .await
            .unwrap();

        HullMovingAverage {
            name,
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period as usize), // Collect historical base data for calculation
            decimal_accuracy,
            is_ready: false,
            period,
            tick_rounding,
            tick_size,
            plot_color,
        }
    }

    /// Extract price from BaseDataEnum (unique to this indicator).
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

    /// Calculate a weighted moving average (WMA).
    fn calculate_wma(data: &[Decimal]) -> Decimal {
        let total_weights: Decimal = (1..=data.len()).map(|x| Decimal::from(x as u64)).sum();
        let weighted_sum: Decimal = data
            .iter()
            .enumerate()
            .map(|(i, value)| value * Decimal::from((i + 1) as u64))
            .sum();
        weighted_sum / total_weights
    }

    /// Calculate the Hull Moving Average (HMA).
    fn calculate_hma(&self, prices: &[Decimal]) -> Decimal {
        // Calculate WMA for half the period
        let half_period = (self.period as usize / 2).max(1);
        let wma_half = Self::calculate_wma(&prices[prices.len() - half_period..]);

        // Calculate WMA for the full period
        let wma_full = Self::calculate_wma(&prices[prices.len() - self.period as usize..]);

        // Adjusted WMA
        let diff = (Decimal::from(2) * wma_half) - wma_full;

        // Final HMA calculation (square root of period)
        let hma_period = ((self.period as f64).sqrt().ceil() as usize).max(1);
        let hma = Self::calculate_wma(&vec![diff; hma_period]);

        hma
    }
}

impl Indicators for HullMovingAverage {
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

        // Extract price and add to history
        self.base_data_history.add(base_data.clone());

        // Ensure sufficient data is available
        if self.base_data_history.len() < self.period as usize {
            return None;
        }

        let prices: Vec<Decimal> = self.base_data_history
            .history()
            .iter()
            .filter_map(|data| self.get_price(data))
            .collect();

        // Calculate HMA
        let hma = self.round_value(self.calculate_hma(&prices));

        // Create plots
        let mut plots = BTreeMap::new();
        plots.insert(
            "hma".to_string(),
            IndicatorPlot::new("Hull Moving Average".to_string(), hma, self.plot_color.clone()),
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