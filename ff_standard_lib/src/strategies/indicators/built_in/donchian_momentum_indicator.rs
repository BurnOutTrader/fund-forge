use std::collections::BTreeMap;
use std::fmt;
use std::fmt::{Display, Formatter};
use chrono::{DateTime, Utc};
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

/// Donchian Momentum Indicator
/// ==========================
///
/// Description:
/// A comprehensive trend and momentum indicator that combines Donchian Channels with
/// momentum and volatility analysis to identify high-probability breakout opportunities
/// and trend strength.
///
/// # Plots
/// 1. "upper" - Upper Donchian Channel
///    - Highest high over channel period
///    - Potential resistance level
///    - Breakout reference for longs
///
/// 2. "lower" - Lower Donchian Channel
///    - Lowest low over channel period
///    - Potential support level
///    - Breakout reference for shorts
///
/// 3. "momentum" - Price Momentum
///    - Rate of price change
///    - Positive values: upward momentum
///    - Negative values: downward momentum
///    - Used for trend confirmation
///
/// 4. "volatility" - Price Volatility
///    - Average true range over period
///    - Used for breakout confirmation
///    - Helps filter false signals
///    - Adjusts breakout thresholds
///
/// 5. "signal" - Combined Signal
///    Format: "{Strength} {Trend} {Position}"
///    - Strength: "Strong", "Moderate", "Weak", "Neutral"
///    - Trend: "Bullish", "Bearish"
///    - Position: "Overbought", "Middle", "Oversold"
///
/// 6. "breakout" (conditional)
///    - Only appears on breakout conditions
///    - Shows "Upward Breakout" or "Downward Breakout"
///    - Includes volatility confirmation
///
/// # Parameters
/// - channel_period: Donchian channel calculation period (typically 20)
/// - momentum_period: Momentum measurement period (typically 10)
/// - volatility_period: Volatility calculation period (typically 14)
/// - breakout_threshold: Minimum volatility multiple for valid breakout
/// - tick_rounding: Whether to round values to tick size
///
/// # Signal Generation
/// 1. Breakout Signals
///    - Price exceeds channel by volatility threshold
///    - Momentum confirms direction
///    - Volatility confirms significance
///
/// 2. Trend Strength
///    - Based on momentum/volatility ratio
///    - Strong: > 75% of volatility
///    - Moderate: 50-75% of volatility
///    - Weak: 25-50% of volatility
///    - Neutral: < 25% of volatility
///
/// 3. Market Position
///    - Overbought: > 90% of channel range
///    - Middle: 10-90% of channel range
///    - Oversold: < 10% of channel range
///
/// # Use Cases
/// 1. Trend Following
///    - Breakout trade entries
///    - Trend confirmation
///    - Exit signal generation
///
/// 2. Range Trading
///    - Channel boundary identification
///    - Support/resistance levels
///    - Range expansion/contraction
///
/// 3. Momentum Trading
///    - Trend strength assessment
///    - Entry timing
///    - Position sizing
///
/// # Best Practices
/// 1. Entry Conditions
///    - Wait for breakout confirmation
///    - Check momentum alignment
///    - Verify volatility context
///
/// 2. Position Management
///    - Size based on volatility
///    - Trail stops with channel
///    - Scale using momentum
///
/// 3. Risk Management
///    - Use opposite channel for stops
///    - Consider volatility for position size
///    - Monitor strength changes
///
/// # Calculation Notes
/// 1. Channels
///    - Upper = Highest high over period
///    - Lower = Lowest low over period
///    - Updates with each new bar
///
/// 2. Momentum
///    - Average price change over period
///    - Normalized by period length
///    - Direction and magnitude significant
///
/// 3. Volatility
///    - Average true range calculation
///    - Used for breakout threshold
///    - Adapts to market conditions
///
/// # Common Patterns
/// 1. Strong Breakout
///    - Price exceeds channel significantly
///    - Strong momentum in breakout direction
///    - Increased volatility
///
/// 2. Failed Breakout
///    - Price barely exceeds channel
///    - Weak or contrary momentum
///    - Low volatility
///
/// 3. Range Expansion
///    - Increasing channel width
///    - Rising volatility
///    - Strong momentum building
///
/// # Implementation Details
/// - Efficient updates on new data
/// - Minimal state maintenance
/// - Accurate signal generation
/// - Clear plot organization
///
/// # Additional Considerations
/// 1. Market Context
///    - Different for trending/ranging markets
///    - Adjust periods for volatility
///    - Consider time of day
///
/// 2. Timeframe Alignment
///    - Use longer period for trend
///    - Shorter for entry timing
///    - Match to trading style
///
/// 3. Volume Confirmation
///    - Higher volume on breakouts
///    - Lower volume in ranges
///    - Volume trend alignment
///
/// # Common Settings
/// 1. Aggressive
///    - Shorter periods
///    - Lower breakout threshold
///    - More signals
///
/// 2. Conservative
///    - Longer periods
///    - Higher breakout threshold
///    - Fewer, higher quality signals
#[derive(Clone, Debug)]
pub struct DonchianMomentum {
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
    channel_period: u64,           // Donchian channel period
    momentum_period: u64,          // Momentum calculation period
    volatility_period: u64,        // Volatility measurement period
    breakout_threshold: Decimal,   // Minimum volatility multiple for breakout
    channel_color: Color,
    momentum_color: Color,
    signal_color: Color,
    tick_rounding: bool,
    last_breakout: Option<(DateTime<Utc>, bool)>, // (time, is_upward)
}

impl Display for DonchianMomentum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl DonchianMomentum {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        channel_period: u64,
        momentum_period: u64,
        volatility_period: u64,
        breakout_threshold: Decimal,
        channel_color: Color,
        momentum_color: Color,
        signal_color: Color,
        tick_rounding: bool,
    ) -> Box<Self> {
        let symbol_name = match subscription.market_type {
            MarketType::Futures(_) => extract_symbol_from_contract(&subscription.symbol.name),
            _ => subscription.symbol.name.clone(),
        };
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(symbol_name.clone()).await.unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(symbol_name.clone()).await.unwrap();

        let dm = DonchianMomentum {
            name,
            market_type: subscription.symbol.market_type.clone(),
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(channel_period.max(momentum_period.max(volatility_period)) as usize),
            is_ready: false,
            tick_size,
            channel_period,
            momentum_period,
            volatility_period,
            breakout_threshold,
            channel_color,
            momentum_color,
            signal_color,
            decimal_accuracy,
            tick_rounding,
            last_breakout: None,
        };
        Box::new(dm)
    }

    fn get_bar_data(data: &BaseDataEnum) -> Option<(Price, Price, Price)> {
        match data {
            BaseDataEnum::QuoteBar(bar) => Some((
                bar.bid_high,
                bar.bid_low,
                bar.bid_close,
            )),
            BaseDataEnum::Candle(candle) => Some((
                candle.high,
                candle.low,
                candle.close,
            )),
            _ => None,
        }
    }

    fn calculate_channels(&self, history: &[BaseDataEnum]) -> Option<(Price, Price)> {
        let channel_data: Vec<(Decimal, Decimal)> = history.iter()
            .rev()
            .take(self.channel_period as usize)
            .filter_map(|bar| {
                Self::get_bar_data(bar).map(|(high, low, _)| (high, low))
            })
            .collect();

        if channel_data.is_empty() {
            return None;
        }

        let upper = channel_data.iter().map(|(h, _)| *h).max()?;
        let lower = channel_data.iter().map(|(_, l)| *l).min()?;

        Some((
            self.round_value(upper),
            self.round_value(lower)
        ))
    }

    fn calculate_momentum(&self, history: &[BaseDataEnum]) -> Option<Price> {
        let closes: Vec<Decimal> = history.iter()
            .rev()
            .take(self.momentum_period as usize)
            .filter_map(|bar| {
                Self::get_bar_data(bar).map(|(_, _, close)| close)
            })
            .collect();

        if closes.len() < 2 {
            return None;
        }

        let current_close = *closes.first()?;
        let period_change = current_close - *closes.last()?;
        let avg_change = period_change / Decimal::from(closes.len() - 1);

        Some(self.round_value(avg_change))
    }

    fn calculate_volatility(&self, history: &[BaseDataEnum]) -> Option<Price> {
        let ranges: Vec<Decimal> = history.iter()
            .rev()
            .take(self.volatility_period as usize)
            .filter_map(|bar| {
                Self::get_bar_data(bar).map(|(high, low, _)| high - low)
            })
            .collect();

        if ranges.is_empty() {
            return None;
        }

        let avg_range = ranges.iter().sum::<Decimal>() / Decimal::from(ranges.len());
        Some(self.round_value(avg_range))
    }

    fn check_breakout(
        &self,
        close: Decimal,
        upper: Decimal,
        lower: Decimal,
        volatility: Decimal,
        momentum: Decimal,
    ) -> Option<bool> {
        // Require breakout to exceed channel by volatility threshold
        let breakout_margin = volatility * self.breakout_threshold;

        if close > upper + breakout_margin && momentum > dec!(0.0) {
            Some(true)  // Upward breakout
        } else if close < lower - breakout_margin && momentum < dec!(0.0) {
            Some(false) // Downward breakout
        } else {
            None
        }
    }

    fn get_signal_description(
        &self,
        close: Decimal,
        upper: Decimal,
        lower: Decimal,
        momentum: Decimal,
        volatility: Decimal,
    ) -> String {
        let channel_position = (close - lower) / (upper - lower) * dec!(100.0);
        let momentum_strength = momentum.abs() / volatility * dec!(100.0);

        let trend = if close > (upper + lower) / dec!(2.0) {
            "Bullish"
        } else {
            "Bearish"
        };

        let strength = if momentum_strength > dec!(75.0) {
            "Strong"
        } else if momentum_strength > dec!(50.0) {
            "Moderate"
        } else if momentum_strength > dec!(25.0) {
            "Weak"
        } else {
            "Neutral"
        };

        let position = if channel_position > dec!(90.0) {
            "Overbought"
        } else if channel_position < dec!(10.0) {
            "Oversold"
        } else {
            "Middle"
        };

        format!("{} {} {}", strength, trend, position)
    }

    fn round_value(&self, value: Decimal) -> Price {
        match self.tick_rounding {
            true => round_to_tick_size(value, self.tick_size),
            false => value.round_dp(self.decimal_accuracy),
        }
    }
}

impl Indicators for DonchianMomentum {
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

        let (_, _, close) = Self::get_bar_data(base_data)?;

        // Calculate components
        let (upper, lower) = self.calculate_channels(&self.base_data_history.history)?;
        let momentum = self.calculate_momentum(&self.base_data_history.history)?;
        let volatility = self.calculate_volatility(&self.base_data_history.history)?;

        // Check for breakout
        let breakout = self.check_breakout(close, upper, lower, volatility, momentum);
        if let Some(is_upward) = breakout {
            self.last_breakout = Some((base_data.time_closed_utc(), is_upward));
        }

        // Generate signal description
        let signal = self.get_signal_description(close, upper, lower, momentum, volatility);

        // Create plots
        let mut plots = BTreeMap::new();

        plots.insert(
            "upper".to_string(),
            IndicatorPlot::new("Upper Channel".to_string(), upper, self.channel_color.clone()),
        );

        plots.insert(
            "lower".to_string(),
            IndicatorPlot::new("Lower Channel".to_string(), lower, self.channel_color.clone()),
        );

        plots.insert(
            "momentum".to_string(),
            IndicatorPlot::new("Momentum".to_string(), momentum, self.momentum_color.clone()),
        );

        plots.insert(
            "volatility".to_string(),
            IndicatorPlot::new("Volatility".to_string(), volatility, self.momentum_color.clone()),
        );

        plots.insert(
            "signal".to_string(),
            IndicatorPlot::new(signal, close, self.signal_color.clone()),
        );

        // Add breakout marker if detected
        if let Some(is_upward) = breakout {
            plots.insert(
                "breakout".to_string(),
                IndicatorPlot::new(
                    if is_upward { "Upward Breakout" } else { "Downward Breakout" }.to_string(),
                    close,
                    self.signal_color.clone(),
                ),
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
        self.last_breakout = None;
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
        self.channel_period.max(self.momentum_period.max(self.volatility_period))
    }
}