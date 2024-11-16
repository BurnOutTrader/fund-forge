use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use rust_decimal::Decimal;
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
/// Trend Direction Force Index (TDFI)
/// Combines price changes and volume to evaluate trend strength and direction.
///
/// # Plots
/// - "force_index": The main force index line
/// - "trend_strength": Smoothed trend strength
/// - "velocity": Price change (momentum)
/// - "momentum": Smoothed price momentum
///
/// # Parameters
/// - period: Number of periods for smoothing
/// - tick_rounding: Whether to round values to tick size
///
/// # Usage
/// Evaluates trend strength and direction, providing insights for momentum and trend-following strategies.

#[derive(Clone, Debug)]
pub struct TrendDirectionForceIndex {
    name: IndicatorName,
    subscription: DataSubscription,
    history: RollingWindow<IndicatorValues>,
    base_data_history: RollingWindow<BaseDataEnum>,
    decimal_accuracy: u32,
    is_ready: bool,
    period: u64,
    tick_rounding: bool,
    trend_color: Color,
    tick_size: Decimal,
    smoothed_trend: Option<Decimal>,
    smoothed_velocity: Option<Decimal>,
    smoothed_momentum: Option<Decimal>,
    last_close: Option<Decimal>,
}

impl Display for TrendDirectionForceIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl TrendDirectionForceIndex {
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: u64,
        trend_color: Color,
        tick_rounding: bool,
    ) -> Box<Self> {
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(subscription.symbol.name.clone())
            .await
            .unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(subscription.symbol.name.clone())
            .await
            .unwrap();

        Box::new(TrendDirectionForceIndex {
            name,
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period as usize),
            decimal_accuracy,
            is_ready: false,
            period,
            tick_rounding,
            trend_color,
            tick_size,
            smoothed_trend: None,
            smoothed_velocity: None,
            smoothed_momentum: None,
            last_close: None,
        })
    }

    /// Smooth a value using Wilder's smoothing method.
    fn smooth_value(current: Decimal, last_smoothed: Option<Decimal>, period: u64) -> Decimal {
        match last_smoothed {
            Some(last) => {
                let period_dec = Decimal::from(period);
                ((last * (period_dec - dec!(1.0))) + current) / period_dec
            }
            None => current,
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

    /// Calculate the force index from price and volume changes.
    fn calculate_force_index(
        &self,
        close: Decimal,
        last_close: Option<Decimal>,
        volume: Decimal,
    ) -> Decimal {
        match last_close {
            Some(last) => (close - last) * volume,
            None => dec!(0.0),
        }
    }

    /// Extract the closing price from base data.
    fn get_close_price(data: &BaseDataEnum) -> Option<Price> {
        match data {
            BaseDataEnum::Candle(candle) => Some(candle.close),
            BaseDataEnum::QuoteBar(bar) => Some(bar.bid_close),
            _ => None,
        }
    }
}

impl Indicators for TrendDirectionForceIndex {
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

        if !self.is_ready {
            if !self.base_data_history.is_full() {
                return None;
            }
            self.is_ready = true;
        }

        let close = Self::get_close_price(base_data)?;
        let volume = match base_data {
            BaseDataEnum::Candle(candle) => candle.volume,
            BaseDataEnum::QuoteBar(bar) => bar.volume,
            _ => dec!(0.0),
        };

        // Calculate force index
        let force_index = self.round_value(self.calculate_force_index(close, self.last_close, volume));

        // Smooth calculations
        self.smoothed_trend = Some(self.round_value(Self::smooth_value(force_index, self.smoothed_trend, self.period)));
        self.smoothed_velocity = Some(self.round_value(Self::smooth_value(
            close - self.last_close.unwrap_or(close),
            self.smoothed_velocity,
            self.period,
        )));
        self.smoothed_momentum = Some(self.round_value(Self::smooth_value(close, self.smoothed_momentum, self.period)));

        // Create plots
        let mut plots = BTreeMap::new();
        plots.insert(
            "force_index".to_string(),
            IndicatorPlot::new("Force Index".to_string(), force_index, self.trend_color.clone()),
        );
        if let Some(trend) = self.smoothed_trend {
            plots.insert(
                "trend_strength".to_string(),
                IndicatorPlot::new("Trend Strength".to_string(), trend, self.trend_color.clone()),
            );
        }
        if let Some(velocity) = self.smoothed_velocity {
            plots.insert(
                "velocity".to_string(),
                IndicatorPlot::new("Velocity".to_string(), velocity, self.trend_color.clone()),
            );
        }
        if let Some(momentum) = self.smoothed_momentum {
            plots.insert(
                "momentum".to_string(),
                IndicatorPlot::new("Momentum".to_string(), momentum, self.trend_color.clone()),
            );
        }

        // Update state
        self.last_close = Some(close);
        let values = IndicatorValues::new(
            self.name.clone(),
            self.subscription.clone(),
            plots,
            base_data.time_closed_utc(),
        );

        self.history.add(values.clone());
        Some(vec![values])
    }

    fn subscription(&self) -> &DataSubscription {
        &self.subscription
    }

    fn reset(&mut self) {
        self.history.clear();
        self.base_data_history.clear();
        self.is_ready = false;
        self.smoothed_trend = None;
        self.smoothed_velocity = None;
        self.smoothed_momentum = None;
        self.last_close = None;
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