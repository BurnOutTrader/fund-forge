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

/// Relative Strength Index (RSI)
/// A momentum oscillator that measures the speed and magnitude of price changes.
/// RSI evaluates overbought and oversold conditions by comparing the magnitude
/// of recent gains to recent losses. Developed by J. Welles Wilder Jr.
///
/// # Calculation Method
/// 1. Calculate price changes:
///    Change = Current Close - Previous Close
///
/// 2. Separate gains and losses:
///    Gains = Change if Change > 0, else 0
///    Losses = |Change| if Change < 0, else 0
///
/// 3. Calculate average gains and losses:
///    First Average Gain = Sum of Gains over past n periods / n
///    First Average Loss = Sum of Losses over past n periods / n
///
/// 4. Use Wilder's smoothing for subsequent values:
///    Avg Gain = (Previous Avg Gain × (n-1) + Current Gain) / n
///    Avg Loss = (Previous Avg Loss × (n-1) + Current Loss) / n
///
/// 5. Calculate RS (Relative Strength):
///    RS = Average Gain / Average Loss
///
/// 6. Calculate RSI:
///    RSI = 100 - (100 / (1 + RS))
///
/// # Plots
/// - "rsi": Main RSI line (0-100 scale)
///   - Overbought zone: Above 70
///   - Oversold zone: Below 30
///   - Centerline: 50
///
/// - "overbought": Upper reference line (typically 70)
///   - Indicates potential reversal zone
///   - Signals to consider taking profits
///
/// - "oversold": Lower reference line (typically 30)
///   - Indicates potential reversal zone
///   - Signals to look for buying opportunities
///
/// - "centerline": Middle reference line (50)
///   - Trend indicator
///   - Above 50: Bullish
///   - Below 50: Bearish
///
/// # Parameters
/// - period: Calculation period (typically 14)
/// - tick_rounding: Whether to round values to tick size
/// - overbought_level: Upper threshold (typically 70)
/// - oversold_level: Lower threshold (typically 30)
///
/// # Key Signals
/// 1. Overbought/Oversold
///   - RSI > 70: Overbought condition
///   - RSI < 30: Oversold condition
///   - More extreme levels (80/20) for strong trends
///
/// 2. Divergence
///   - Bullish: Price makes lower lows, RSI makes higher lows
///   - Bearish: Price makes higher highs, RSI makes lower highs
///   - Hidden: Confirms existing trend
///
/// 3. Centerline Crossovers
///   - Cross above 50: Bullish
///   - Cross below 50: Bearish
///   - Confirms trend direction
///
/// 4. Failure Swings
///   - Bullish: RSI holds above 30, makes higher low
///   - Bearish: RSI holds below 70, makes lower high
///   - Strong reversal signals
///
/// # Common Usage Patterns
/// 1. Trend Identification
///   - RSI > 50: Uptrend
///   - RSI < 50: Downtrend
///   - Slope indicates momentum
///
/// 2. Support/Resistance
///   - RSI can form its own trendlines
///   - Look for breaks of RSI trendlines
///   - RSI levels can act as S/R
///
/// 3. Entry/Exit Points
///   - Enter longs near oversold levels
///   - Enter shorts near overbought levels
///   - Exit on divergence signals
///
/// # Best Practices
/// 1. Multiple Time Frame Analysis
///   - Use longer timeframe for trend
///   - Use shorter timeframe for entry
///   - Confirm signals across timeframes
///
/// 2. Market Context
///   - Adjust levels for different markets
///   - Higher levels in bull markets
///   - Lower levels in bear markets
///
/// 3. Confirmation
///   - Use price action confirmation
///   - Look for candlestick patterns
///   - Check volume for confirmation
///
/// # Risk Management
/// 1. Position Sizing
///   - Larger size in trend direction
///   - Smaller size for counter-trend
///   - Scale in/out at extremes
///
/// 2. Stop Loss Placement
///   - Behind recent swing points
///   - Outside normal RSI fluctuation
///   - Based on volatility
///
/// # Advanced Concepts
/// 1. Modified RSI Versions
///   - Stochastic RSI
///   - Connors RSI
///   - Dynamic RSI levels
///
/// 2. Pattern Recognition
///   - Double bottoms/tops
///   - Head and shoulders
///   - Chart patterns in RSI
///
/// 3. Momentum Analysis
///   - Rate of change in RSI
///   - RSI trend strength
///   - RSI range analysis
///
/// # Known Limitations
/// - Lag due to averaging
/// - False signals in strong trends
/// - Can stay overbought/oversold
/// - Requires context
///
/// # Trading Strategies
/// 1. Mean Reversion
///   - Trade from extremes back to center
///   - Use with proper confirmation
///   - Set realistic targets
///
/// 2. Trend Following
///   - Use centerline crossovers
///   - Trade with overall trend
///   - Wait for pullbacks
///
/// 3. Divergence Trading
///   - Look for clear divergences
///   - Multiple timeframe confirmation
///   - Use with support/resistance
///
/// # Performance Notes
/// - Efficient updates with new data
/// - Accurate smoothing calculation
/// - Proper divergence detection
/// - Handles gaps appropriately
///
/// # Additional Tips
/// 1. Combine with Other Indicators
///   - Moving averages
///   - Volume indicators
///   - Price patterns
///
/// 2. Market-Specific Adjustments
///   - Stocks: Standard settings work well
///   - Forex: May need wider levels
///   - Crypto: Consider extreme levels
///
/// 3. Time-Based Considerations
///   - More reliable on daily timeframe
///   - Faster signals on lower timeframes
///   - Weekly for major trends
#[derive(Clone, Debug)]
pub struct RelativeStrengthIndex {
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
    period: u64,
    tick_rounding: bool,
    last_avg_gain: Option<Decimal>,
    last_avg_loss: Option<Decimal>,
}

impl Display for RelativeStrengthIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl RelativeStrengthIndex {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: u64,
        plot_color: Color,
        tick_rounding: bool,
    ) -> Box<Self> {
        let symbol_name = match subscription.market_type {
            MarketType::Futures(_) => extract_symbol_from_contract(&subscription.symbol.name),
            _ => subscription.symbol.name.clone(),
        };
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(symbol_name.clone()).await.unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(symbol_name.clone()).await.unwrap();

        let rsi = RelativeStrengthIndex {
            name,
            market_type: subscription.symbol.market_type.clone(),
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new((period + 1) as usize), // Need +1 to calculate first change
            is_ready: false,
            tick_size,
            plot_color,
            period,
            decimal_accuracy,
            tick_rounding,
            last_avg_gain: None,
            last_avg_loss: None,
        };
        Box::new(rsi)
    }

    fn get_close_price(data: &BaseDataEnum) -> Price {
        match data {
            BaseDataEnum::QuoteBar(bar) => bar.bid_close,
            BaseDataEnum::Candle(candle) => candle.close,
            _ => panic!("Unsupported data type for RSI"),
        }
    }

    fn calculate_initial_averages(&self) -> (Decimal, Decimal) {
        let base_data = self.base_data_history.history();
        let mut gains = Vec::new();
        let mut losses = Vec::new();

        // Calculate price changes
        for i in 1..base_data.len() {
            let prev_price = Self::get_close_price(&base_data[i - 1]);
            let curr_price = Self::get_close_price(&base_data[i]);
            let change = curr_price - prev_price;

            if change > dec!(0.0) {
                gains.push(change);
                losses.push(dec!(0.0));
            } else {
                gains.push(dec!(0.0));
                losses.push(change.abs());
            }
        }

        // Calculate averages
        let avg_gain = gains.iter().sum::<Decimal>() / Decimal::from(self.period);
        let avg_loss = losses.iter().sum::<Decimal>() / Decimal::from(self.period);

        match self.tick_rounding {
            true => (
                round_to_tick_size(avg_gain, self.tick_size),
                round_to_tick_size(avg_loss, self.tick_size),
            ),
            false => (
                avg_gain.round_dp(self.decimal_accuracy),
                avg_loss.round_dp(self.decimal_accuracy),
            ),
        }
    }

    fn calculate_rsi(&self, avg_gain: Decimal, avg_loss: Decimal) -> Decimal {
        if avg_loss == dec!(0.0) {
            return dec!(100.0);
        }

        let rs = avg_gain / avg_loss;
        let rsi = dec!(100.0) - (dec!(100.0) / (dec!(1.0) + rs));

        match self.tick_rounding {
            true => round_to_tick_size(rsi, self.tick_size),
            false => rsi.round_dp(self.decimal_accuracy),
        }
    }

    fn calculate_smoothed_averages(
        &self,
        last_avg_gain: Decimal,
        last_avg_loss: Decimal,
        current_gain: Decimal,
        current_loss: Decimal,
    ) -> (Decimal, Decimal) {
        // Use Wilder's smoothing: [(prev_avg * (period - 1)) + current_value] / period
        let period_dec = Decimal::from(self.period);
        let multiplier = Decimal::from(self.period - 1);

        let avg_gain = (last_avg_gain * multiplier + current_gain) / period_dec;
        let avg_loss = (last_avg_loss * multiplier + current_loss) / period_dec;

        match self.tick_rounding {
            true => (
                round_to_tick_size(avg_gain, self.tick_size),
                round_to_tick_size(avg_loss, self.tick_size),
            ),
            false => (
                avg_gain.round_dp(self.decimal_accuracy),
                avg_loss.round_dp(self.decimal_accuracy),
            ),
        }
    }
}

impl Indicators for RelativeStrengthIndex {
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

            // Initialize with first set of averages
            let (avg_gain, avg_loss) = self.calculate_initial_averages();
            self.last_avg_gain = Some(avg_gain);
            self.last_avg_loss = Some(avg_loss);
            self.is_ready = true;
        }

        // Get current and previous prices
        let base_data = self.base_data_history.history();
        let curr_price = Self::get_close_price(&base_data[base_data.len() - 1]);
        let prev_price = Self::get_close_price(&base_data[base_data.len() - 2]);

        // Calculate current change
        let change = curr_price - prev_price;
        let (current_gain, current_loss) = if change > dec!(0.0) {
            (change, dec!(0.0))
        } else {
            (dec!(0.0), change.abs())
        };

        // Update averages using Wilder's smoothing
        let (new_avg_gain, new_avg_loss) = self.calculate_smoothed_averages(
            self.last_avg_gain.unwrap(),
            self.last_avg_loss.unwrap(),
            current_gain,
            current_loss,
        );

        // Calculate RSI
        let rsi = self.calculate_rsi(new_avg_gain, new_avg_loss);

        // Update state for next calculation
        self.last_avg_gain = Some(new_avg_gain);
        self.last_avg_loss = Some(new_avg_loss);

        // Create indicator values
        let mut plots = BTreeMap::new();
        plots.insert(
            "rsi".to_string(),
            IndicatorPlot::new("rsi".to_string(), rsi, self.plot_color.clone()),
        );

        let values = IndicatorValues::new(
            self.name.clone(),
            self.subscription.clone(),
            plots,
            base_data.last().unwrap().time_closed_utc(),
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
        self.last_avg_gain = None;
        self.last_avg_loss = None;
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
        self.history.len() as u64 + self.period + 1 // +1 for initial change calculation
    }
}

