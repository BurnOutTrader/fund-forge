use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use rust_decimal::Decimal;
use rust_decimal::prelude::Signed;
use rust_decimal_macros::dec;
use crate::gui_types::settings::Color;
use crate::helpers::decimal_calculators::round_to_tick_size;
use crate::product_maps::rithmic::maps::extract_symbol_from_contract;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::enums::MarketType;
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::indicators::indicator_values::{IndicatorPlot, IndicatorValues};
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};

/// Market Exhaustion Indicator
/// =========================
///
/// Description:
/// An indicator that identifies potential market reversal points by analyzing volume thrust,
/// price extensions, and momentum shifts. Helps detect when a market move may be exhausted
/// and due for a reversal.
///
/// # Plots
/// 1. "volume_ratio" - Volume Thrust Measurement
///    - Current volume relative to moving average
///    - Values > threshold indicate climactic volume
///    - Used for volume spike detection
///    - Higher values suggest stronger exhaustion
///
/// 2. "price_extension" - Price Extension Analysis
///    - Distance of price from moving average
///    - Normalized as percentage
///    - Identifies overextended moves
///    - Higher values indicate extreme moves
///
/// 3. "momentum" - Price Momentum
///    - Rate of price change
///    - Used for momentum divergence
///    - Helps confirm exhaustion
///    - Shows momentum shifts
///
/// 4. "signal" (conditional) - Exhaustion Signals
///    Types:
///    - "Bullish Exhaustion": Upward price extension with high volume
///    - "Bearish Exhaustion": Downward price extension with high volume
///    - "Bullish Climax": High volume with upward momentum shift
///    - "Bearish Climax": High volume with downward momentum shift
///    - "Bullish Divergence": Price extension with positive momentum shift
///    - "Bearish Divergence": Price extension with negative momentum shift
///    - "Volume Climax": Exceptional volume spike
///
/// # Parameters
/// - volume_ma_period: Period for volume moving average
/// - momentum_period: Momentum calculation period
/// - price_ma_period: Price moving average period
/// - thrust_threshold: Volume multiple for climax detection
/// - price_extension: Price extension threshold
///
/// # Signal Generation
/// 1. Volume-Based Signals
///    - Volume/MA ratio exceeds thrust threshold
///    - Indicates potential exhaustion
///    - Stronger signals with higher ratios
///    - Used for climax detection
///
/// 2. Price Extension Signals
///    - Price moves far from moving average
///    - Indicates overextended condition
///    - More significant at extremes
///    - Used for divergence detection
///
/// 3. Momentum Signals
///    - Momentum shifts and divergences
///    - Confirms exhaustion patterns
///    - Helps time reversals
///    - Validates other signals
///
/// # Pattern Recognition
/// 1. Climactic Exhaustion
///    - Very high volume
///    - Extended price
///    - Momentum shift
///
/// 2. Price Exhaustion
///    - Extreme price extension
///    - Momentum divergence
///    - Normal to high volume
///
/// 3. Volume Climax
///    - Exceptional volume
///    - Any price condition
///    - Potential reversal point
///
/// # Use Cases
/// 1. Reversal Trading
///    - Identify potential reversal points
///    - Time counter-trend entries
///    - Spot market extremes
///
/// 2. Trend Trading
///    - Identify trend exhaustion
///    - Exit before reversals
///    - Scale out positions
///
/// 3. Risk Management
///    - Warning of potential reversals
///    - Position size adjustment
///    - Stop placement
///
/// # Best Practices
/// 1. Signal Confirmation
///    - Wait for multiple conditions
///    - Check price action
///    - Verify volume pattern
///
/// 2. Context Consideration
///    - Overall trend direction
///    - Support/resistance levels
///    - Time of day/session
///
/// 3. Risk Control
///    - Don't counter-trend trade strong moves
///    - Use appropriate position sizing
///    - Have clear exit plans
///
/// # Example Settings
/// 1. Aggressive
///    ```rust
///    volume_ma_period: 10,
///    momentum_period: 5,
///    price_ma_period: 10,
///    thrust_threshold: dec!(2.0),
///    price_extension: dec!(0.02),
///    ```
///
/// 2. Conservative
///    ```rust
///    volume_ma_period: 20,
///    momentum_period: 10,
///    price_ma_period: 20,
///    thrust_threshold: dec!(3.0),
///    price_extension: dec!(0.03),
///    ```
///
/// # Implementation Notes
/// 1. Calculations
///    - Volume MA: Simple moving average
///    - Price MA: Simple moving average
///    - Momentum: Rate of change
///    - Extensions: Percentage from MA
///
/// 2. Signal Priority
///    - Multiple condition signals strongest
///    - Volume climax signals next
///    - Extension/momentum signals last
///
/// 3. Performance
///    - Efficient moving average updates
///    - Minimal state maintenance
///    - Fast signal generation
///
/// # Common Patterns
/// 1. End of Trend
///    - High volume
///    - Extended price
///    - Momentum divergence
///
/// 2. Buying/Selling Climax
///    - Extreme volume
///    - Price extension
///    - Sharp momentum shift
///
/// 3. Exhaustion Gaps
///    - Price extension
///    - Above average volume
///    - Momentum extreme
///
/// # Additional Considerations
/// 1. Market Type
///    - More reliable in trending markets
///    - Adjust thresholds for volatility
///    - Consider instrument characteristics
///
/// 2. Timeframe Alignment
///    - Check multiple timeframes
///    - Confirm on higher timeframe
///    - Enter on lower timeframe
///
/// 3. Volume Profile
///    - Consider normal volume patterns
///    - Adjust for time of day
///    - Account for seasonal patterns
///
/// # Potential Enhancements
/// 1. Additional Signals
///    - Multiple timeframe signals
///    - Pattern recognition
///    - Accumulation/distribution
///
/// 2. Filtering
///    - Time-based filters
///    - Trend filters
///    - Volume profile analysis
///
/// 3. Risk Management
///    - Dynamic position sizing
///    - Automatic stop placement
///    - Scale-out levels
#[derive(Clone, Debug)]
pub struct MarketExhaustion {
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
    volume_ma_period: u64,        // Volume moving average period
    momentum_period: u64,         // Momentum calculation period
    thrust_threshold: Decimal,    // Volume thrust multiplier
    price_extension: Decimal,     // Price extension threshold
    exhaustion_color: Color,
    thrust_color: Color,
    signal_color: Color,
    tick_rounding: bool,
    last_volume_ma: Option<Decimal>,
    last_momentum: Option<Decimal>,
    price_ma_period: u64,        // Price moving average period
    last_price_ma: Option<Decimal>,
}

impl Display for MarketExhaustion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl MarketExhaustion {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        volume_ma_period: u64,
        momentum_period: u64,
        price_ma_period: u64,
        thrust_threshold: Decimal,
        price_extension: Decimal,
        exhaustion_color: Color,
        thrust_color: Color,
        signal_color: Color,
        tick_rounding: bool,
    ) -> Self {
        let symbol_name = match subscription.market_type {
            MarketType::Futures(_) => extract_symbol_from_contract(&subscription.symbol.name),
            _ => subscription.symbol.name.clone(),
        };
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(symbol_name.clone()).await.unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(symbol_name.clone()).await.unwrap();

        let max_period = volume_ma_period.max(momentum_period.max(price_ma_period));

        let me = MarketExhaustion {
            name,
            market_type: subscription.symbol.market_type.clone(),
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(max_period as usize),
            is_ready: false,
            tick_size,
            volume_ma_period,
            momentum_period,
            price_ma_period,
            thrust_threshold,
            price_extension,
            exhaustion_color,
            thrust_color,
            signal_color,
            decimal_accuracy,
            tick_rounding,
            last_volume_ma: None,
            last_momentum: None,
            last_price_ma: None,
        };
        me
    }

    fn get_bar_data(data: &BaseDataEnum) -> Option<(Price, Price, Price, Price, Volume)> {
        match data {
            BaseDataEnum::QuoteBar(bar) => Some((
                bar.bid_high,
                bar.bid_low,
                bar.bid_open,
                bar.bid_close,
                bar.bid_volume,
            )),
            BaseDataEnum::Candle(candle) => Some((
                candle.high,
                candle.low,
                candle.open,
                candle.close,
                Decimal::from(candle.volume),
            )),
            _ => None,
        }
    }

    fn calculate_volume_ma(&self, history: &[BaseDataEnum]) -> Price {
        let volumes: Vec<Decimal> = history.iter()
            .rev()
            .take(self.volume_ma_period as usize)
            .filter_map(|bar| {
                Self::get_bar_data(bar).map(|(_, _, _, _, volume)| volume)
            })
            .collect();

        let ma = if volumes.is_empty() {
            dec!(0.0)
        } else {
            volumes.iter().sum::<Decimal>() / Decimal::from(volumes.len())
        };

        self.round_value(ma)
    }

    fn calculate_price_ma(&self, history: &[BaseDataEnum]) -> Price {
        let closes: Vec<Decimal> = history.iter()
            .rev()
            .take(self.price_ma_period as usize)
            .filter_map(|bar| {
                Self::get_bar_data(bar).map(|(_, _, _, close, _)| close)
            })
            .collect();

        let ma = if closes.is_empty() {
            dec!(0.0)
        } else {
            closes.iter().sum::<Decimal>() / Decimal::from(closes.len())
        };

        self.round_value(ma)
    }

    fn calculate_momentum(&self, history: &[BaseDataEnum]) -> Price {
        let closes: Vec<Decimal> = history.iter()
            .rev()
            .take(self.momentum_period as usize)
            .filter_map(|bar| {
                Self::get_bar_data(bar).map(|(_, _, _, close, _)| close)
            })
            .collect();

        if closes.len() < 2 {
            return dec!(0.0);
        }

        let first = closes[0];
        let last = closes[closes.len() - 1];
        self.round_value((first - last) / last * dec!(100.0))
    }

    fn detect_exhaustion(
        &self,
        current_volume: Decimal,
        volume_ma: Decimal,
        current_price: Decimal,
        price_ma: Decimal,
        momentum: Decimal,
        is_up: bool,
    ) -> Option<String> {
        // Volume thrust check
        let volume_thrust = current_volume / volume_ma;
        let price_distance = ((current_price - price_ma) / price_ma).abs();

        // Check previous momentum
        let momentum_shift = match self.last_momentum {
            Some(last_mom) => momentum.signum() != last_mom.signum(),
            None => false,
        };

        if volume_thrust >= self.thrust_threshold && price_distance >= self.price_extension {
            // Strong volume and extended price
            if is_up {
                Some("Bullish Exhaustion".to_string())
            } else {
                Some("Bearish Exhaustion".to_string())
            }
        } else if volume_thrust >= self.thrust_threshold && momentum_shift {
            // Strong volume with momentum shift
            if is_up {
                Some("Bullish Climax".to_string())
            } else {
                Some("Bearish Climax".to_string())
            }
        } else if price_distance >= self.price_extension && momentum_shift {
            // Extended price with momentum shift
            if is_up {
                Some("Bullish Divergence".to_string())
            } else {
                Some("Bearish Divergence".to_string())
            }
        } else if volume_thrust >= self.thrust_threshold {
            // Just volume thrust
            Some("Volume Climax".to_string())
        } else {
            None
        }
    }

    fn round_value(&self, value: Decimal) -> Price {
        match self.tick_rounding {
            true => round_to_tick_size(value, self.tick_size),
            false => value.round_dp(self.decimal_accuracy),
        }
    }
}

impl Indicators for MarketExhaustion {
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

        let (_, _, _, close, volume) = Self::get_bar_data(base_data)?;

        // Calculate indicators
        let volume_ma = self.calculate_volume_ma(&self.base_data_history.history);
        let price_ma = self.calculate_price_ma(&self.base_data_history.history);
        let momentum = self.calculate_momentum(&self.base_data_history.history);

        // Determine trend direction
        let is_up = close > price_ma;

        // Detect exhaustion
        let exhaustion = self.detect_exhaustion(
            volume,
            volume_ma,
            close,
            price_ma,
            momentum,
            is_up,
        );

        // Create plots
        let mut plots = BTreeMap::new();

        // Volume analysis
        plots.insert(
            "volume_ratio".to_string(),
            IndicatorPlot::new(
                "Volume/MA Ratio".to_string(),
                self.round_value(volume / volume_ma),
                self.thrust_color.clone(),
            ),
        );

        // Price extension
        plots.insert(
            "price_extension".to_string(),
            IndicatorPlot::new(
                "Price Extension".to_string(),
                self.round_value(((close - price_ma) / price_ma).abs()),
                self.exhaustion_color.clone(),
            ),
        );

        // Momentum
        plots.insert(
            "momentum".to_string(),
            IndicatorPlot::new(
                "Momentum".to_string(),
                momentum,
                self.exhaustion_color.clone(),
            ),
        );

        // Exhaustion signal
        if let Some(signal) = exhaustion {
            plots.insert(
                "signal".to_string(),
                IndicatorPlot::new(
                    signal,
                    close,
                    self.signal_color.clone(),
                ),
            );
        }

        // Update state
        self.last_volume_ma = Some(volume_ma);
        self.last_momentum = Some(momentum);
        self.last_price_ma = Some(price_ma);

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
        self.last_volume_ma = None;
        self.last_momentum = None;
        self.last_price_ma = None;
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
        self.volume_ma_period.max(self.momentum_period.max(self.price_ma_period))
    }
}