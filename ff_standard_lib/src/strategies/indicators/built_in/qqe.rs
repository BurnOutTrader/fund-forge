use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use rust_decimal::{Decimal};
use rust_decimal::prelude::{One, Zero};
use crate::gui_types::settings::Color;
use crate::helpers::decimal_calculators::round_to_tick_size;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::new_types::Price;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::indicators::indicator_values::{IndicatorPlot, IndicatorValues};
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};

/// Quantitative Qualitative Estimation (QQE)
/// Enhances RSI with smoothing and volatility bands to improve signal quality.
///
/// # Plots
/// - "qqe": The main QQE line.
/// - "signal": The signal line, smoothed from the QQE line.
///
/// # Parameters
/// - period: Lookback period for the RSI calculation.
/// - smoothing: Smoothing period for the QQE line.
/// - tick_rounding: Whether to round values to tick size.
///
/// # Usage
/// Combines trend and momentum analysis. Crossovers of QQE and signal lines provide entry/exit signals.
#[derive(Clone, Debug)]
pub struct QQE {
    name: IndicatorName,
    subscription: DataSubscription,
    history: RollingWindow<IndicatorValues>,
    base_data_history: RollingWindow<BaseDataEnum>,
    decimal_accuracy: u32,
    is_ready: bool,
    period: usize,
    smoothing: usize,
    tick_rounding: bool,
    tick_size: Decimal,
    plot_color_qqe: Color,
    plot_color_signal: Color,
    last_qqe: Option<Decimal>,
    last_signal: Option<Decimal>,
}

impl Display for QQE {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl QQE {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: usize,
        smoothing: usize,
        plot_color_qqe: Color,
        plot_color_signal: Color,
        tick_rounding: bool,
    ) -> Self {
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(subscription.symbol.name.clone())
            .await
            .unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(subscription.symbol.name.clone())
            .await
            .unwrap();

        QQE {
            name,
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period),
            decimal_accuracy,
            is_ready: false,
            period,
            smoothing,
            tick_rounding,
            tick_size,
            plot_color_qqe,
            plot_color_signal,
            last_qqe: None,
            last_signal: None,
        }
    }

    /// Extract close price from BaseDataEnum.
    /// Specific to QQE.
    fn get_close(&self, data: &BaseDataEnum) -> Option<Price> {
        match data {
            BaseDataEnum::Candle(candle) => Some(candle.close),
            BaseDataEnum::QuoteBar(bar) => Some(bar.bid_close),
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

    /// Smooth the values using a simple moving average.
    fn smooth_values(&self, prev: Decimal, current: Decimal) -> Decimal {
        prev + (current - prev) / Decimal::from(self.smoothing as u64)
    }

    /// Calculate RSI.
    fn calculate_rsi(&self, closes: &[Decimal]) -> Option<Decimal> {
        let mut gains = Decimal::zero();
        let mut losses = Decimal::zero();

        for i in 1..closes.len() {
            let change = closes[i] - closes[i - 1];
            if change > Decimal::zero() {
                gains += change;
            } else {
                losses += -change;
            }
        }

        if gains + losses == Decimal::zero() {
            return None;
        }

        let rs = gains / losses;
        Some(Decimal::from(100) - (Decimal::from(100) / (Decimal::one() + rs)))
    }
}

impl Indicators for QQE {
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

        // Extract close price and add to history
        self.base_data_history.add(base_data.clone());

        // Ensure sufficient data is available
        if self.base_data_history.len() < self.period {
            return None;
        }

        let closes: Vec<Decimal> = self
            .base_data_history
            .history()
            .iter()
            .filter_map(|data| self.get_close(data))
            .collect();

        // Calculate RSI
        let rsi = self.calculate_rsi(&closes)?;

        // Calculate QQE
        let last_qqe = self.last_qqe.unwrap_or(rsi);
        let qqe = self.round_value(self.smooth_values(last_qqe, rsi));
        self.last_qqe = Some(qqe);

        // Calculate Signal
        let last_signal = self.last_signal.unwrap_or(qqe);
        let signal = self.round_value(self.smooth_values(last_signal, qqe));
        self.last_signal = Some(signal);

        // Create plots
        let mut plots = BTreeMap::new();
        plots.insert(
            "qqe".to_string(),
            IndicatorPlot::new("QQE".to_string(), qqe, self.plot_color_qqe.clone()),
        );
        plots.insert(
            "signal".to_string(),
            IndicatorPlot::new("Signal".to_string(), signal, self.plot_color_signal.clone()),
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
        self.last_qqe = None;
        self.last_signal = None;
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
        self.period as u64
    }
}