use std::collections::BTreeMap;
use rust_decimal::{Decimal, MathematicalOps};
use rust_decimal::prelude::{One};
use rust_decimal_macros::dec;
use crate::gui_types::settings::Color;
use crate::helpers::decimal_calculators::round_to_tick_size;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::indicators::indicator_values::{IndicatorPlot, IndicatorValues};
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};

/// Fisher Transform
/// A normalization technique to identify extremes and reversals.
///
/// # Plots
/// - "fisher": The main Fisher Transform line.
/// - "trigger": The trigger line, a lagged version of the Fisher Transform line.
///
/// # Parameters
/// - period: Lookback period for the high and low prices.
///
/// # Usage
/// Identifies overbought/oversold conditions and sharp reversals.
#[derive(Clone, Debug)]
pub struct FisherTransform {
    name: IndicatorName,
    subscription: DataSubscription,
    history: RollingWindow<IndicatorValues>,
    base_data_history: RollingWindow<BaseDataEnum>,
    decimal_accuracy: u32,
    is_ready: bool,
    period: usize,
    tick_rounding: bool,
    tick_size: Decimal,
    plot_color_fisher: Color,
    plot_color_trigger: Color,
    last_fisher: Option<Decimal>,
    last_trigger: Option<Decimal>,
}
impl FisherTransform {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: usize,
        plot_color_fisher: Color,
        plot_color_trigger: Color,
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

        Box::new(FisherTransform {
            name,
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period),
            decimal_accuracy,
            is_ready: false,
            period,
            tick_rounding,
            tick_size,
            plot_color_fisher,
            plot_color_trigger,
            last_fisher: None,
            last_trigger: None,
        })
    }

    /// Apply tick rounding or decimal precision rounding.
    fn round_value(&self, value: Decimal) -> Decimal {
        if self.tick_rounding {
            round_to_tick_size(value, self.tick_size)
        } else {
            value.round_dp(self.decimal_accuracy)
        }
    }

    /// Calculate the Fisher Transform value.
    fn calculate_fisher(&self, price: Decimal) -> Decimal {
        let clipped = price.clamp(dec!(-0.999), dec!(0.999));
        self.round_value(dec!(0.5) * ((Decimal::one() + clipped).ln() - (Decimal::one() - clipped).ln()))
    }

    /// Extract the close price from BaseDataEnum.
    fn get_close(&self, data: &BaseDataEnum) -> Option<Decimal> {
        match data {
            BaseDataEnum::Candle(candle) => Some(candle.close),
            BaseDataEnum::QuoteBar(bar) => Some(bar.bid_close),
            _ => None,
        }
    }
}

impl Indicators for FisherTransform {
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

        // Extract close price and add the base data to history.
        let close = self.get_close(base_data)?;
        self.base_data_history.add(base_data.clone());

        // Ensure sufficient data for calculations.
        if self.base_data_history.len() < self.period {
            return None;
        }

        // Gather all close prices for calculations.
        let prices: Vec<Decimal> = self
            .base_data_history
            .history()
            .iter()
            .filter_map(|data| self.get_close(data))
            .collect();

        let high = prices.iter().copied().max().unwrap_or(close);
        let low = prices.iter().copied().min().unwrap_or(close);

        let normalized_value = (close - low) / (high - low + dec!(1e-10)) * Decimal::from(2) - Decimal::one();
        let fisher = self.calculate_fisher(normalized_value);

        let trigger = self.last_fisher.unwrap_or(fisher);
        let smoothed_trigger = self.calculate_fisher(trigger);

        // Create plots.
        let mut plots = BTreeMap::new();
        plots.insert(
            "fisher".to_string(),
            IndicatorPlot::new("Fisher".to_string(), fisher, self.plot_color_fisher.clone()),
        );
        plots.insert(
            "trigger".to_string(),
            IndicatorPlot::new("Trigger".to_string(), smoothed_trigger, self.plot_color_trigger.clone()),
        );

        // Create and store indicator values.
        let values = IndicatorValues::new(
            self.name.clone(),
            self.subscription.clone(),
            plots,
            base_data.time_closed_utc(),
        );

        self.history.add(values.clone());
        self.last_fisher = Some(fisher);
        self.last_trigger = Some(smoothed_trigger);
        self.is_ready = true;
        Some(vec![values])
    }

    fn subscription(&self) -> &DataSubscription {
        &self.subscription
    }

    fn reset(&mut self) {
        self.history.clear();
        self.base_data_history.clear();
        self.last_fisher = None;
        self.last_trigger = None;
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