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

/// Relative Momentum Index (RMI)
/// A momentum oscillator that builds on RSI by incorporating a momentum period.
///
/// # Plots
/// - "rmi": The main RMI line.
///
/// # Parameters
/// - period: Lookback period for RMI calculation.
/// - momentum: Momentum period (number of consecutive up/down days).
/// - tick_rounding: Whether to round values to tick size.
///
/// # Usage
/// Identifies overbought/oversold conditions and trend strength. Crossovers of RMI thresholds provide signals.
#[derive(Clone, Debug)]
pub struct RelativeMomentumIndex {
    name: IndicatorName,
    subscription: DataSubscription,
    history: RollingWindow<IndicatorValues>,
    base_data_history: RollingWindow<BaseDataEnum>,
    decimal_accuracy: u32,
    is_ready: bool,
    period: usize,
    momentum: usize,
    tick_rounding: bool,
    tick_size: Decimal,
    plot_color_rmi: Color,
}

impl Display for RelativeMomentumIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl RelativeMomentumIndex {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: usize,
        momentum: usize,
        plot_color_rmi: Color,
        tick_rounding: bool,
    ) -> Self {
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(subscription.symbol.name.clone())
            .await
            .unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(subscription.symbol.name.clone())
            .await
            .unwrap();

        RelativeMomentumIndex {
            name,
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period + momentum), // Store enough data for calculation
            decimal_accuracy,
            is_ready: false,
            period,
            momentum,
            tick_rounding,
            tick_size,
            plot_color_rmi,
        }
    }

    /// Extract close price from BaseDataEnum.
    /// Specific to RMI.
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

    /// Calculate the RMI numerator and denominator.
    fn calculate_rmi_components(&self, closes: &[Decimal]) -> (Decimal, Decimal) {
        let mut gains = Decimal::zero();
        let mut losses = Decimal::zero();

        for i in self.momentum..closes.len() {
            let change = closes[i] - closes[i - self.momentum];
            if change > Decimal::zero() {
                gains += change;
            } else {
                losses += -change;
            }
        }

        (gains, losses)
    }
}

impl Indicators for RelativeMomentumIndex {
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
        if self.base_data_history.len() < self.period + self.momentum {
            return None;
        }

        let closes: Vec<Decimal> = self
            .base_data_history
            .history()
            .iter()
            .filter_map(|data| self.get_close(data))
            .collect();

        // Calculate RMI components
        let (gains, losses) = self.calculate_rmi_components(&closes);

        // Calculate RMI
        let rmi = if gains + losses == Decimal::zero() {
            Decimal::zero()
        } else {
            Decimal::from(100) * (gains / (gains + losses))
        };

        let rmi = self.round_value(rmi);

        // Create plots
        let mut plots = BTreeMap::new();
        plots.insert(
            "rmi".to_string(),
            IndicatorPlot::new("RMI".to_string(), rmi, self.plot_color_rmi.clone()),
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
        (self.period + self.momentum) as u64
    }
}