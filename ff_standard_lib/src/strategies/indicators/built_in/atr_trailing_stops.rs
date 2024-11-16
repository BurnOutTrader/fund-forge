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

/// ATR Trailing Stop
/// A dynamic stop-loss indicator that adjusts based on market volatility using the Average True Range (ATR).
/// Provides separate stops for long and short positions, automatically trailing price movement to protect profits
/// while allowing room for normal market fluctuations.
///
/// # Calculation Method
/// 1. True Range (TR) = max(
///    current_high - current_low,
///    |current_high - previous_close|,
///    |current_low - previous_close|
/// )
/// 2. ATR = Exponential Moving Average of True Range
/// 3. Long Stop = Highest High - (ATR × multiplier)
/// 4. Short Stop = Lowest Low + (ATR × multiplier)
///
/// # Plots
/// - "long_stop": Stop level for long positions
///   - Trails below the price during uptrends
///   - Only moves up, never down
///   - Exit long position if price closes below this level
///
/// - "short_stop": Stop level for short positions
///   - Trails above the price during downtrends
///   - Only moves down, never up
///   - Exit short position if price closes above this level
///
/// # Parameters
/// - period: Number of periods for ATR calculation (typically 14-21)
/// - multiplier: ATR multiplier for stop distance (typically 2-3)
///   - Higher multiplier = wider stops, fewer whipsaws
///   - Lower multiplier = tighter stops, more whipsaws
/// - tick_rounding: Whether to round values to tick size
///
/// # Key Features
/// 1. Volatility-Based Adaptation
///    - Stops widen in volatile markets
///    - Stops tighten in quiet markets
///    - Adjusts automatically to market conditions
///
/// 2. Trend Following
///    - Long stops trail uptrends
///    - Short stops trail downtrends
///    - Helps ride larger trends
///
/// 3. Position Management
///    - Clear exit points for positions
///    - Automatic profit protection
///    - Risk management built-in
///
/// # Common Usage
/// 1. Stop Loss Placement
///    - Initial stop placement for new positions
///    - Dynamic adjustment as trade progresses
///    - Clear exit points for risk management
///
/// 2. Trend Trading
///    - Ride trends until stop is hit
///    - Stay in position while trend continues
///    - Exit when trend weakens
///
/// 3. Position Sizing
///    - Use stop distance for position sizing
///    - Calculate risk per trade
///    - Maintain consistent risk levels
///
/// # Best Practices
/// 1. Entry Management
///    - Wait for price to move away from stop
///    - Avoid entering near the stop level
///    - Consider stop distance in entry timing
///
/// 2. Multiple Time Frames
///    - Longer timeframe for trend direction
///    - Shorter timeframe for entry/exit timing
///    - Match ATR period to trading timeframe
///
/// 3. Stop Adjustment
///    - Never widen stops
///    - Allow stops to tighten in profit
///    - Consider partial position exits
///
/// # Risk Management
/// 1. Position Sizing
///    - Calculate risk based on stop distance
///    - Account for volatility changes
///    - Adjust position size accordingly
///
/// 2. Multiple Stops
///    - Initial stop for risk control
///    - Trailing stop for profit protection
///    - Time-based stops for dead trades
///
/// 3. Gap Risk
///    - Be aware of overnight gaps
///    - Consider market hours
///    - Use additional protective measures
///
/// # Trade Management
/// 1. Entry Confirmation
///    - Price moving away from stop
///    - Volume confirmation
///    - Trend alignment
///
/// 2. Stop Adjustment
///    - Move to breakeven when possible
///    - Tighten stops in profit
///    - Never widen initial stop
///
/// 3. Exit Strategy
///    - Full exit at stop hit
///    - Partial exits at targets
///    - Scale out in trends
///
/// # Known Limitations
/// - Can be whipsawed in ranging markets
/// - Gap risk in overnight positions
/// - May trail too far in strong trends
/// - Requires context from other indicators
///
/// # Implementation Notes
/// 1. Performance
///    - Efficient updates with new data
///    - Minimal state maintenance
///    - Accurate tick rounding
///
/// 2. Calculation Accuracy
///    - True Range includes gaps
///    - Proper ATR smoothing
///    - Accurate stop trailing
///
/// # Advanced Concepts
/// 1. Multiple ATR Stops
///    - Different multipliers for different purposes
///    - Combine with other indicators
///    - Create stop zones
///
/// 2. Breakout Confirmation
///    - Use stops for breakout validation
///    - Combine with volume
///    - Monitor stop violations
///
/// 3. Volatility Analysis
///    - ATR trends indicate volatility changes
///    - Stop width indicates market condition
///    - Use for position sizing
#[derive(Clone, Debug)]
pub struct AtrTrailingStop {
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
    long_stop_color: Color,
    short_stop_color: Color,
    period: u64,
    multiplier: Decimal,
    tick_rounding: bool,
    last_atr: Option<Decimal>,
    highest_high: Option<Decimal>,
    lowest_low: Option<Decimal>,
    last_long_stop: Option<Decimal>,
    last_short_stop: Option<Decimal>,
}

impl Display for AtrTrailingStop {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl AtrTrailingStop {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: u64,
        multiplier: Decimal,
        long_stop_color: Color,
        short_stop_color: Color,
        tick_rounding: bool,
    ) -> Self {
        let symbol_name = match subscription.market_type {
            MarketType::Futures(_) => extract_symbol_from_contract(&subscription.symbol.name),
            _ => subscription.symbol.name.clone(),
        };
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(symbol_name.clone()).await.unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(symbol_name.clone()).await.unwrap();

        let atr_stop = AtrTrailingStop {
            name,
            market_type: subscription.symbol.market_type.clone(),
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period as usize),
            is_ready: false,
            tick_size,
            long_stop_color,
            short_stop_color,
            period,
            multiplier,
            decimal_accuracy,
            tick_rounding,
            last_atr: None,
            highest_high: None,
            lowest_low: None,
            last_long_stop: None,
            last_short_stop: None,
        };
        atr_stop
    }

    fn get_price_data(data: &BaseDataEnum) -> Option<(Price, Price, Price)> {
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

    fn calculate_true_range(&self, current: &BaseDataEnum, previous: Option<&BaseDataEnum>) -> Price {
        let (current_high, current_low, current_close) = Self::get_price_data(current).unwrap();

        let prev_close = previous
            .and_then(Self::get_price_data)
            .map(|(_, _, close)| close)
            .unwrap_or(current_close);

        // Calculate the three differences
        let high_low = current_high - current_low;
        let high_close = (current_high - prev_close).abs();
        let low_close = (current_low - prev_close).abs();

        // True Range is the greatest of the three
        high_low.max(high_close).max(low_close)
    }

    fn calculate_atr(&self, true_range: Decimal) -> Price {
        match self.last_atr {
            None => true_range,
            Some(last_atr) => {
                // Wilder's smoothing
                let period_dec = Decimal::from(self.period);
                (last_atr * (period_dec - dec!(1.0)) + true_range) / period_dec
            }
        }
    }

    fn update_stops(&mut self, high: Decimal, low: Decimal, close: Decimal, atr: Decimal) -> (Price, Price) {
        // Update highest high and lowest low
        self.highest_high = Some(self.highest_high.map_or(high, |h| h.max(high)));
        self.lowest_low = Some(self.lowest_low.map_or(low, |l| l.min(low)));

        // Calculate basic stops
        let atr_offset = self.multiplier * atr;
        let basic_long_stop = self.highest_high.unwrap() - atr_offset;
        let basic_short_stop = self.lowest_low.unwrap() + atr_offset;

        // Long stop can only increase if price is rising
        let long_stop = match self.last_long_stop {
            None => basic_long_stop,
            Some(last_stop) => {
                if close > last_stop {
                    basic_long_stop.max(last_stop)
                } else {
                    last_stop
                }
            }
        };

        // Short stop can only decrease if price is falling
        let short_stop = match self.last_short_stop {
            None => basic_short_stop,
            Some(last_stop) => {
                if close < last_stop {
                    basic_short_stop.min(last_stop)
                } else {
                    last_stop
                }
            }
        };

        // Round if needed
        let (long_stop, short_stop) = match self.tick_rounding {
            true => (
                round_to_tick_size(long_stop, self.tick_size),
                round_to_tick_size(short_stop, self.tick_size),
            ),
            false => (
                long_stop.round_dp(self.decimal_accuracy),
                short_stop.round_dp(self.decimal_accuracy),
            ),
        };

        self.last_long_stop = Some(long_stop);
        self.last_short_stop = Some(short_stop);

        (long_stop, short_stop)
    }
}

impl Indicators for AtrTrailingStop {
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

        let history = self.base_data_history.history();
        let (high, low, close) = Self::get_price_data(base_data)?;

        // Calculate true range and ATR
        let true_range = self.calculate_true_range(
            base_data,
            history.get(history.len() - 2),
        );

        let atr = self.calculate_atr(true_range);
        self.last_atr = Some(atr);

        // Update and get stops
        let (long_stop, short_stop) = self.update_stops(high, low, close, atr);

        // Create plots
        let mut plots = BTreeMap::new();
        plots.insert(
            "long_stop".to_string(),
            IndicatorPlot::new("Long Stop".to_string(), long_stop, self.long_stop_color.clone()),
        );
        plots.insert(
            "short_stop".to_string(),
            IndicatorPlot::new("Short Stop".to_string(), short_stop, self.short_stop_color.clone()),
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
        self.last_atr = None;
        self.highest_high = None;
        self.lowest_low = None;
        self.last_long_stop = None;
        self.last_short_stop = None;
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
        self.period + 1 // Need period + 1 bars for initial calculation
    }
}
