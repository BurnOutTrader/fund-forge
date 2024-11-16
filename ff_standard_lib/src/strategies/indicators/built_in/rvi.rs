use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use rust_decimal::{Decimal};
use rust_decimal::prelude::Zero;
use crate::gui_types::settings::Color;
use crate::helpers::decimal_calculators::round_to_tick_size;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::new_types::Price;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::indicators::indicator_values::{IndicatorPlot, IndicatorValues};
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};

/// Relative Vigor Index (RVI)
/// Measures market momentum based on the principle that prices close higher in uptrends and lower in downtrends.
///
/// # Plots
/// - "rvi": The main RVI line.
/// - "signal": The signal line, a smoothed version of the RVI line.
///
/// # Parameters
/// - period: Lookback period for RVI calculation.
/// - tick_rounding: Whether to round values to tick size.
///
/// # Usage
/// Useful for identifying momentum and potential reversals. Crossovers of RVI and signal line provide entry/exit signals.
#[derive(Clone, Debug)]
pub struct RelativeVigorIndex {
    name: IndicatorName,
    subscription: DataSubscription,
    history: RollingWindow<IndicatorValues>,
    base_data_history: RollingWindow<BaseDataEnum>,
    decimal_accuracy: u32,
    is_ready: bool,
    period: usize,
    tick_rounding: bool,
    tick_size: Decimal,
    plot_color_rvi: Color,
    plot_color_signal: Color,
    smoothed_rvi: Option<Decimal>,
    smoothed_signal: Option<Decimal>,
}

impl Display for RelativeVigorIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl RelativeVigorIndex {
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: usize,
        plot_color_rvi: Color,
        plot_color_signal: Color,
        tick_rounding: bool,
    ) -> Box<Self> {
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(subscription.symbol.name.clone())
            .await
            .unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(subscription.symbol.name.clone())
            .await
            .unwrap();

        Box::new(RelativeVigorIndex {
            name,
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period), // Store enough data for calculation
            decimal_accuracy,
            is_ready: false,
            period,
            tick_rounding,
            tick_size,
            plot_color_rvi,
            plot_color_signal,
            smoothed_rvi: None,
            smoothed_signal: None,
        })
    }

    /// Extract high, low, open, and close prices from BaseDataEnum.
    /// Specific to RVI.
    fn get_ohlc(&self, data: &BaseDataEnum) -> Option<(Price, Price, Price, Price)> {
        match data {
            BaseDataEnum::Candle(candle) => Some((candle.high, candle.low, candle.open, candle.close)),
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

    /// Smooth values using a simple moving average.
    fn smooth_values(values: &[Decimal]) -> Decimal {
        let sum: Decimal = values.iter().sum();
        sum / Decimal::from(values.len() as u64)
    }

    /// Calculate the RVI numerator and denominator.
    fn calculate_rvi_components(&self, history: &[BaseDataEnum]) -> Option<(Decimal, Decimal)> {
        let mut numerator = Decimal::zero();
        let mut denominator = Decimal::zero();

        for data in history.iter() {
            let (high, low, open, close) = self.get_ohlc(data)?;

            // Calculate close-open and high-low differences
            numerator += close - open;
            denominator += high - low;
        }

        if denominator.is_zero() {
            return None;
        }

        Some((numerator, denominator))
    }
}

impl Indicators for RelativeVigorIndex {
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

        self.base_data_history.add(base_data.clone());

        // Ensure sufficient data is available
        if self.base_data_history.len() < self.period {
            return None;
        }

        let history: Vec<BaseDataEnum> = self.base_data_history.history().to_vec();

        // Calculate RVI numerator and denominator
        let (numerator, denominator) = self.calculate_rvi_components(&history)?;

        // Calculate RVI value
        let rvi = self.round_value(numerator / denominator);

        // Smooth RVI value
        let smoothed_rvi = Self::smooth_values(&[rvi]);
        self.smoothed_rvi = Some(smoothed_rvi);

        // Calculate and smooth the signal line
        let signal = self.smoothed_signal.unwrap_or(smoothed_rvi);
        let smoothed_signal = self.round_value((smoothed_rvi + signal) / Decimal::from(2));
        self.smoothed_signal = Some(smoothed_signal);

        // Create plots
        let mut plots = BTreeMap::new();
        plots.insert(
            "rvi".to_string(),
            IndicatorPlot::new("RVI".to_string(), smoothed_rvi, self.plot_color_rvi.clone()),
        );
        plots.insert(
            "signal".to_string(),
            IndicatorPlot::new("Signal".to_string(), smoothed_signal, self.plot_color_signal.clone()),
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
        self.smoothed_rvi = None;
        self.smoothed_signal = None;
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