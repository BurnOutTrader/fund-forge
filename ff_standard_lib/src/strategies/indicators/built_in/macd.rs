use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::gui_types::settings::Color;
use crate::helpers::decimal_calculators::round_to_tick_size;
use crate::product_maps::rithmic::maps::extract_symbol_from_contract;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::enums::MarketType;
use crate::standardized_types::new_types::Price;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::indicators::indicator_values::{IndicatorPlot, IndicatorValues};
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};

/// Moving Average Convergence Divergence (MACD)
/// Trend-following momentum indicator showing relationship between two moving averages.
///
/// # Plots
/// - "macd": The MACD line (fast EMA - slow EMA)
/// - "signal": Signal line (EMA of MACD)
/// - "histogram": Difference between MACD and signal line
///
/// # Parameters
/// - fast_period: Shorter EMA period (typically 12)
/// - slow_period: Longer EMA period (typically 26)
/// - signal_period: Signal line EMA period (typically 9)
///
/// # Usage
/// Identifies trend changes, momentum, and potential entry/exit points.

/// Average Directional Index (ADX)
/// Measures trend strength regardless of direction.
///
/// # Plots
/// - "adx": Main ADX line showing trend strength (0-100)
/// - "plus_di": +DI line showing upward price movement
/// - "minus_di": -DI line showing downward price movement
///
/// # Parameters
/// - period: Calculation period (typically 14)
/// - tick_rounding: Whether to round values to tick size
///
/// # Usage
/// Determines trend strength and potential trend direction changes.
#[derive(Clone, Debug)]
pub struct MovingAverageConvergenceDivergence {
    name: IndicatorName,
    subscription: DataSubscription,
    history: RollingWindow<IndicatorValues>,
    base_data_history: RollingWindow<BaseDataEnum>,
    #[allow(unused)]
    market_type: MarketType,
    #[allow(unused)]
    tick_size: Decimal,
    decimal_accuracy: u32,
    is_ready: bool,
    plot_color: Color,
    hist_color: Color,
    signal_color: Color,
    fast_period: u64,
    slow_period: u64,
    signal_period: u64,
    tick_rounding: bool,
    fast_multiplier: Decimal,
    slow_multiplier: Decimal,
    signal_multiplier: Decimal,
    last_fast_ema: Option<Decimal>,
    last_slow_ema: Option<Decimal>,
    last_signal_ema: Option<Decimal>,
    last_macd: Option<Decimal>,
}

impl Display for MovingAverageConvergenceDivergence {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl MovingAverageConvergenceDivergence {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        fast_period: u64,
        slow_period: u64,
        signal_period: u64,
        plot_color: Color,
        signal_color: Color,
        hist_color: Color,
        tick_rounding: bool,
    ) -> Box<Self> {
        let symbol_name = match subscription.market_type {
            MarketType::Futures(_) => extract_symbol_from_contract(&subscription.symbol.name),
            _ => subscription.symbol.name.clone(),
        };
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(symbol_name.clone()).await.unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(symbol_name.clone()).await.unwrap();

        // Calculate EMA multipliers
        let fast_multiplier = Decimal::from(2) / (Decimal::from(fast_period) + Decimal::from(1));
        let slow_multiplier = Decimal::from(2) / (Decimal::from(slow_period) + Decimal::from(1));
        let signal_multiplier = Decimal::from(2) / (Decimal::from(signal_period) + Decimal::from(1));

        // Need enough history for the longest period (slow period) plus signal period for initialization
        let required_history = slow_period.max(fast_period + signal_period) as usize;

        let macd = MovingAverageConvergenceDivergence {
            name,
            market_type: subscription.symbol.market_type.clone(),
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(required_history),
            is_ready: false,
            tick_size,
            plot_color,
            signal_color,
            hist_color,
            fast_period,
            slow_period,
            signal_period,
            decimal_accuracy,
            tick_rounding,
            fast_multiplier,
            slow_multiplier,
            signal_multiplier,
            last_fast_ema: None,
            last_slow_ema: None,
            last_signal_ema: None,
            last_macd: None,
        };
        Box::new(macd)
    }

    fn get_close_price(data: &BaseDataEnum) -> Price {
        match data {
            BaseDataEnum::QuoteBar(bar) => bar.bid_close,
            BaseDataEnum::Candle(candle) => candle.close,
            _ => panic!("Unsupported data type for MACD"),
        }
    }

    fn calculate_initial_ema(&self, period: u64) -> Price {
        let base_data = self.base_data_history.history();
        let values: Vec<Decimal> = base_data.iter()
            .take(period as usize)
            .map(Self::get_close_price)
            .collect();

        if values.is_empty() {
            return dec!(0.0);
        }

        let sum: Decimal = values.iter().sum();

        match self.tick_rounding {
            true => round_to_tick_size(
                sum / Decimal::from(period),
                self.tick_size
            ),
            false => (sum / Decimal::from(period)).round_dp(self.decimal_accuracy),
        }
    }

    fn calculate_ema(&self, current_price: Decimal, prev_ema: Decimal, multiplier: Decimal) -> Price {
        let ema = multiplier * (current_price - prev_ema) + prev_ema;

        match self.tick_rounding {
            true => round_to_tick_size(ema, self.tick_size),
            false => ema.round_dp(self.decimal_accuracy),
        }
    }
}

impl Indicators for MovingAverageConvergenceDivergence {
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

        // Initialize EMAs if we're just starting
        if !self.is_ready {
            if !self.base_data_history.is_full() {
                return None;
            }

            // Calculate initial EMAs
            self.last_fast_ema = Some(self.calculate_initial_ema(self.fast_period));
            self.last_slow_ema = Some(self.calculate_initial_ema(self.slow_period));
            self.is_ready = true;
        }

        let current_price = Self::get_close_price(base_data);

        // Update EMAs
        let fast_ema = self.calculate_ema(
            current_price,
            self.last_fast_ema.unwrap(),
            self.fast_multiplier
        );
        let slow_ema = self.calculate_ema(
            current_price,
            self.last_slow_ema.unwrap(),
            self.slow_multiplier
        );

        // Calculate MACD line
        let macd = fast_ema - slow_ema;

        // Update signal line
        let signal = if self.last_signal_ema.is_none() {
            macd  // First MACD value becomes first signal value
        } else {
            self.calculate_ema(
                macd,
                self.last_signal_ema.unwrap(),
                self.signal_multiplier
            )
        };

        // Calculate histogram
        let histogram = macd - signal;

        // Update state
        self.last_fast_ema = Some(fast_ema);
        self.last_slow_ema = Some(slow_ema);
        self.last_signal_ema = Some(signal);
        self.last_macd = Some(macd);

        // Create plots
        let mut plots = BTreeMap::new();
        plots.insert(
            "macd".to_string(),
            IndicatorPlot::new("macd".to_string(), macd, self.plot_color.clone()),
        );
        plots.insert(
            "signal".to_string(),
            IndicatorPlot::new("signal".to_string(), signal, self.signal_color.clone()),
        );
        plots.insert(
            "histogram".to_string(),
            IndicatorPlot::new("histogram".to_string(), histogram, self.hist_color.clone()),
        );

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
        self.last_fast_ema = None;
        self.last_slow_ema = None;
        self.last_signal_ema = None;
        self.last_macd = None;
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
        self.history.len() as u64 + self.slow_period.max(self.fast_period + self.signal_period)
    }
}
