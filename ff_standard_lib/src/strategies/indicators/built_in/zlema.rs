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

/// Zero Lag Exponential Moving Average (ZLEMA)
/// A variation of EMA that reduces lag by adjusting for price displacement.
///
/// # Plots
/// - "zlema": The main ZLEMA line.
///
/// # Parameters
/// - period: Lookback period for the ZLEMA calculation.
/// - tick_rounding: Whether to round values to tick size.
///
/// # Usage
/// Provides a smooth, lag-reduced trend line for identifying trends and reversals.
#[derive(Clone, Debug)]
pub struct ZeroLagExponentialMovingAverage {
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
    last_zlema: Option<Decimal>,
    multiplier: Decimal,
}

impl Display for ZeroLagExponentialMovingAverage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl ZeroLagExponentialMovingAverage {
    #[allow(dead_code)]
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

        let multiplier = Decimal::from(2) / (Decimal::from(period) + Decimal::from(1));

        ZeroLagExponentialMovingAverage {
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
            last_zlema: None,
            multiplier,
        }
    }

    /// Extract close price from BaseDataEnum.
    /// Specific to ZLEMA.
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

    /// Calculate the displaced price for ZLEMA.
    fn calculate_displaced_price(&self, prices: &[Decimal]) -> Decimal {
        let displacement = (self.period as usize / 2).max(1);
        let last_index = prices.len().saturating_sub(displacement);
        let displaced_price = if last_index < prices.len() {
            prices[last_index]
        } else {
            prices.last().cloned().unwrap_or_default()
        };

        displaced_price
    }

    /// Update the ZLEMA using the displaced price.
    fn calculate_zlema(&self, displaced_price: Decimal, last_zlema: Decimal) -> Decimal {
        last_zlema + self.multiplier * (displaced_price - last_zlema)
    }
}

impl Indicators for ZeroLagExponentialMovingAverage {
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
        if self.base_data_history.len() < self.period as usize {
            return None;
        }

        let prices: Vec<Decimal> = self.base_data_history
            .history()
            .iter()
            .filter_map(|data| self.get_price(data))
            .collect();

        // Calculate displaced price
        let displaced_price = self.calculate_displaced_price(&prices);

        // Initialize ZLEMA with the first displaced price if not already set
        let last_zlema = self.last_zlema.unwrap_or(displaced_price);
        let zlema = self.round_value(self.calculate_zlema(displaced_price, last_zlema));

        // Update the last ZLEMA value
        self.last_zlema = Some(zlema);

        // Create plots
        let mut plots = BTreeMap::new();
        plots.insert(
            "zlema".to_string(),
            IndicatorPlot::new("Zero Lag EMA".to_string(), zlema, self.plot_color.clone()),
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
        self.last_zlema = None;
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