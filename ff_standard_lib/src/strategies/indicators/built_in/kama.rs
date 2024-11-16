use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use rust_decimal::{Decimal};
use rust_decimal_macros::dec;
use crate::gui_types::settings::Color;
use crate::helpers::decimal_calculators::round_to_tick_size;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::new_types::Price;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::indicators::indicator_values::{IndicatorPlot, IndicatorValues};
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};

/// Kaufman Adaptive Moving Average (KAMA)
/// Adjusts its smoothing factor dynamically based on market noise and trend.
///
/// # Plots
/// - "kama": The main KAMA line.
///
/// # Parameters
/// - period: Lookback period for efficiency ratio (ER) calculation.
/// - fast_period: Fast smoothing period (e.g., 2).
/// - slow_period: Slow smoothing period (e.g., 30).
/// - tick_rounding: Whether to round values to tick size.
///
/// # Usage
/// Smooths price data while adapting to volatility. Suitable for identifying trends with reduced lag.
#[derive(Clone, Debug)]
pub struct KaufmanAdaptiveMovingAverage {
    name: IndicatorName,
    subscription: DataSubscription,
    history: RollingWindow<IndicatorValues>,
    base_data_history: RollingWindow<BaseDataEnum>,
    decimal_accuracy: u32,
    is_ready: bool,
    period: u64,
    fast_period: u64,
    slow_period: u64,
    tick_rounding: bool,
    tick_size: Decimal,
    plot_color: Color,
    last_kama: Option<Decimal>,
}

impl Display for KaufmanAdaptiveMovingAverage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl KaufmanAdaptiveMovingAverage {
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: u64,
        fast_period: u64,
        slow_period: u64,
        plot_color: Color,
        tick_rounding: bool,
    ) -> Box<Self> {
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(subscription.symbol.name.clone())
            .await
            .unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(subscription.symbol.name.clone())
            .await
            .unwrap();

        Box::new(KaufmanAdaptiveMovingAverage {
            name,
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period as usize), // Collect historical base data for calculation
            decimal_accuracy,
            is_ready: false,
            period,
            fast_period,
            slow_period,
            tick_rounding,
            tick_size,
            plot_color,
            last_kama: None,
        })
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

    /// Calculate the Efficiency Ratio (ER).
    fn calculate_efficiency_ratio(&self, prices: &[Decimal]) -> Decimal {
        if prices.is_empty() {
            return dec!(0.0);
        }

        let price_change = (prices.last().unwrap() - prices.first().unwrap()).abs();
        let volatility: Decimal = prices
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .sum();

        if volatility == dec!(0.0) {
            dec!(0.0)
        } else {
            price_change / volatility
        }
    }

    /// Calculate the Smoothing Constant (SC).
    fn calculate_smoothing_constant(&self, efficiency_ratio: Decimal) -> Decimal {
        let fast_smoothing = Decimal::from(2) / (Decimal::from(self.fast_period) + dec!(1.0));
        let slow_smoothing = Decimal::from(2) / (Decimal::from(self.slow_period) + dec!(1.0));

        let sc = (efficiency_ratio * (fast_smoothing - slow_smoothing)) + slow_smoothing;
        sc * sc // Squared smoothing constant
    }

    /// Calculate the next KAMA value.
    fn calculate_kama(&self, current_price: Decimal, last_kama: Decimal, smoothing_constant: Decimal) -> Decimal {
        last_kama + smoothing_constant * (current_price - last_kama)
    }
}

impl Indicators for KaufmanAdaptiveMovingAverage {
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
        let close = self.get_price(base_data)?;
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

        // Calculate Efficiency Ratio (ER)
        let efficiency_ratio = self.calculate_efficiency_ratio(&prices);

        // Calculate Smoothing Constant (SC)
        let smoothing_constant = self.calculate_smoothing_constant(efficiency_ratio);

        // Calculate KAMA
        let last_kama = self.last_kama.unwrap_or_else(|| prices.last().cloned().unwrap_or_default());
        let kama = self.round_value(self.calculate_kama(close, last_kama, smoothing_constant));

        // Update the last KAMA value
        self.last_kama = Some(kama);

        // Create plots
        let mut plots = BTreeMap::new();
        plots.insert(
            "kama".to_string(),
            IndicatorPlot::new("Kaufman Adaptive Moving Average".to_string(), kama, self.plot_color.clone()),
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
        self.last_kama = None;
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