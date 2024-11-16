use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use rust_decimal::{Decimal, MathematicalOps};
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

/// Bollinger Bands (BB)
/// A volatility-based indicator consisting of three bands: a middle simple moving average (SMA)
/// and two outer bands placed a specified number of standard deviations away from the middle band.
/// The bands expand and contract based on market volatility, creating a dynamic price envelope.
///
/// # Calculation Method
/// 1. Middle Band = SMA(price, period)
/// 2. Standard Deviation = √(Σ(price - SMA)² / period)
/// 3. Upper Band = Middle Band + (Standard Deviation × multiplier)
/// 4. Lower Band = Middle Band - (Standard Deviation × multiplier)
///
/// # Plots
/// - "middle": Middle band (SMA)
///   - Acts as the base trend line
///   - Often used as a support/resistance level
///
/// - "upper": Upper band
///   - Represents overbought territory
///   - Price tends to reverse near this level
///   - Breakouts above signal strong trends
///
/// - "lower": Lower band
///   - Represents oversold territory
///   - Price tends to reverse near this level
///   - Breakouts below signal strong downtrends
///
/// - "bandwidth": Shows the percentage spread between bands
///   - bandwidth = (upper - lower) / middle × 100
///   - High values indicate high volatility
///   - Low values indicate low volatility (squeeze)
///
/// - "percent_b": Shows where price is relative to the bands
///   - %B = (price - lower) / (upper - lower)
///   - Above 1.0: Price above upper band
///   - Below 0.0: Price below lower band
///   - 0.5: Price at middle band
///
/// # Parameters
/// - period: Number of periods for SMA (typically 20)
/// - num_std_dev: Number of standard deviations (typically 2)
/// - tick_rounding: Whether to round values to tick size
///
/// # Key Signals
/// 1. Mean Reversion
///    - Price tends to return to the middle band
///    - Extreme moves to outer bands often reverse
///
/// 2. Volatility Breakout
///    - Strong trends when price moves outside bands
///    - Sustained moves above/below outer bands
///
/// 3. Bollinger Squeeze
///    - Bands narrow (low bandwidth)
///    - Indicates potential upcoming volatility
///    - Often precedes significant moves
///
/// 4. W-Bottoms and M-Tops
///    - W-Bottom: Double bottom with second low above lower band
///    - M-Top: Double top with second high below upper band
///
/// # Common Usage Patterns
/// 1. Trend Identification
///    - Price above middle band = uptrend
///    - Price below middle band = downtrend
///    - Middle band slope indicates trend direction
///
/// 2. Volatility Analysis
///    - Wide bands = high volatility
///    - Narrow bands = low volatility
///    - Bandwidth helps identify squeezes
///
/// 3. Support/Resistance
///    - Middle band acts as dynamic support/resistance
///    - Outer bands mark price extremes
///
/// 4. Entry/Exit Signals
///    - Enter long: Price bounces off lower band
///    - Enter short: Price bounces off upper band
///    - Take profit: Price reaches opposite band
///
/// # Best Practices
/// 1. Multiple Time Frame Analysis
///    - Use longer time frames for trend
///    - Use shorter time frames for entry/exit
///
/// 2. Volume Confirmation
///    - High volume on band touches
///    - Volume expansion on breakouts
///
/// 3. Additional Indicators
///    - RSI for overbought/oversold
///    - MACD for trend confirmation
///    - Volume indicators for breakout confirmation
///
/// # Risk Management
/// 1. Breakout Trading
///    - False breakouts common
///    - Wait for confirmation candles
///    - Use volume for confirmation
///
/// 2. Mean Reversion Trading
///    - Don't fight strong trends
///    - Check higher time frame trend
///    - Use proper position sizing
///
/// # Known Limitations
/// - Lag due to moving average calculation
/// - Can generate false signals in ranging markets
/// - Less effective in strong trends
/// - Requires context from other indicators
///
/// # Advanced Concepts
/// 1. Keltner Channel Comparison
///    - BB uses standard deviation
///    - Keltner uses ATR
///    - Compare for confirmation
///
/// 2. Adaptive Bands
///    - Adjust standard deviations based on volatility
///    - Use different periods for different markets
///
/// 3. Multiple Band Sets
///    - Use 1, 2, and 3 standard deviations
///    - More bands for more precision
///
/// # Performance Considerations
/// - Updates efficiently with new data
/// - Maintains minimal state (SMA calculation)
/// - Handles tick rounding appropriately
/// - Uses accurate standard deviation calculation
#[derive(Clone, Debug)]
pub struct BollingerBands {
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
    middle_color: Color,
    upper_color: Color,
    lower_color: Color,
    period: u64,           // Typically 20 periods
    num_std_dev: Decimal,  // Typically 2.0 standard deviations
    tick_rounding: bool,
}

impl Display for BollingerBands {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl BollingerBands {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: u64,
        num_std_dev: Decimal,
        middle_color: Color,
        upper_color: Color,
        lower_color: Color,
        tick_rounding: bool,
    ) -> Box<Self> {
        let symbol_name = match subscription.market_type {
            MarketType::Futures(_) => extract_symbol_from_contract(&subscription.symbol.name),
            _ => subscription.symbol.name.clone(),
        };
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(symbol_name.clone()).await.unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(symbol_name.clone()).await.unwrap();

        let bb = BollingerBands {
            name,
            market_type: subscription.symbol.market_type.clone(),
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period as usize),
            is_ready: false,
            tick_size,
            middle_color,
            upper_color,
            lower_color,
            period,
            num_std_dev,
            decimal_accuracy,
            tick_rounding,
        };
        Box::new(bb)
    }

    fn get_close_price(data: &BaseDataEnum) -> Price {
        match data {
            BaseDataEnum::QuoteBar(bar) => bar.bid_close,
            BaseDataEnum::Candle(candle) => candle.close,
            _ => panic!("Unsupported data type for Bollinger Bands"),
        }
    }

    fn calculate_bands(&self) -> Option<(Price, Price, Price)> {
        let base_data = self.base_data_history.history();
        if base_data.is_empty() {
            return None;
        }

        // Calculate simple moving average (middle band)
        let closes: Vec<Decimal> = base_data.iter()
            .map(Self::get_close_price)
            .collect();

        let period_dec = Decimal::from(self.period);
        let sma = closes.iter().sum::<Decimal>() / period_dec;

        // Calculate standard deviation
        let variance = closes.iter()
            .map(|close| (*close - sma) * (*close - sma))
            .sum::<Decimal>() / period_dec;

        let std_dev = variance.sqrt().unwrap_or(dec!(0.0));

        // Calculate bands
        let middle = match self.tick_rounding {
            true => round_to_tick_size(sma, self.tick_size),
            false => sma.round_dp(self.decimal_accuracy),
        };

        let upper = match self.tick_rounding {
            true => round_to_tick_size(sma + self.num_std_dev * std_dev, self.tick_size),
            false => (sma + self.num_std_dev * std_dev).round_dp(self.decimal_accuracy),
        };

        let lower = match self.tick_rounding {
            true => round_to_tick_size(sma - self.num_std_dev * std_dev, self.tick_size),
            false => (sma - self.num_std_dev * std_dev).round_dp(self.decimal_accuracy),
        };

        Some((upper, middle, lower))
    }

    pub fn get_bandwidth(&self) -> Option<Decimal> {
        if let Some((upper, middle, lower)) = self.calculate_bands() {
            if middle == dec!(0.0) {
                return None;
            }

            let bandwidth = (upper - lower) / middle * dec!(100.0);
            Some(match self.tick_rounding {
                true => round_to_tick_size(bandwidth, self.tick_size),
                false => bandwidth.round_dp(self.decimal_accuracy),
            })
        } else {
            None
        }
    }

    pub fn get_percent_b(&self) -> Option<Decimal> {
        if let Some((upper, _, lower)) = self.calculate_bands() {
            let current_price = Self::get_close_price(self.base_data_history.history().last()?);
            if upper == lower {
                return Some(dec!(50.0));
            }

            let percent_b = (current_price - lower) / (upper - lower) * dec!(100.0);
            Some(match self.tick_rounding {
                true => round_to_tick_size(percent_b, self.tick_size),
                false => percent_b.round_dp(self.decimal_accuracy),
            })
        } else {
            None
        }
    }
}

impl Indicators for BollingerBands {
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

        // Calculate bands
        let (upper, middle, lower) = self.calculate_bands()?;

        // Create plots
        let mut plots = BTreeMap::new();
        plots.insert(
            "middle".to_string(),
            IndicatorPlot::new("middle".to_string(), middle, self.middle_color.clone()),
        );
        plots.insert(
            "upper".to_string(),
            IndicatorPlot::new("upper".to_string(), upper, self.upper_color.clone()),
        );
        plots.insert(
            "lower".to_string(),
            IndicatorPlot::new("lower".to_string(), lower, self.lower_color.clone()),
        );

        // Optional: Add bandwidth and %B if needed
        if let Some(bandwidth) = self.get_bandwidth() {
            plots.insert(
                "bandwidth".to_string(),
                IndicatorPlot::new("bandwidth".to_string(), bandwidth, self.middle_color.clone()),
            );
        }

        if let Some(percent_b) = self.get_percent_b() {
            plots.insert(
                "percent_b".to_string(),
                IndicatorPlot::new("%B".to_string(), percent_b, self.middle_color.clone()),
            );
        }

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
        self.history.len() as u64 + self.period
    }
}
