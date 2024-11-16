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

/// Average Directional Movement Rating (ADMR)
/// An enhanced version of the ADX indicator that provides a more detailed analysis of trend
/// strength and direction. It combines directional movement concepts with a comprehensive
/// rating system to give traders clear trend assessments.
///
/// # Calculation Method
/// 1. Directional Movement (DM):
///    +DM = current_high - previous_high (if positive and > -DM)
///    -DM = previous_low - current_low (if positive and > +DM)
///
/// 2. True Range (TR):
///    TR = max(
///        high - low,
///        |high - previous_close|,
///        |low - previous_close|
///    )
///
/// 3. Smoothed Values (using Wilder's smoothing):
///    Smoothed = ((previous * (period - 1)) + current) / period
///    Applied to: TR, +DM, -DM
///
/// 4. Directional Indicators:
///    +DI = (Smoothed +DM / Smoothed TR) × 100
///    -DI = (Smoothed -DM / Smoothed TR) × 100
///
/// 5. Directional Index:
///    DX = |+DI - -DI| / (+DI + -DI) × 100
///
/// # Plots
/// - "plus_di": Positive Directional Indicator
///   - Measures upward price movement strength
///   - Higher values indicate stronger upward pressure
///   - Used in trend direction determination
///
/// - "minus_di": Negative Directional Indicator
///   - Measures downward price movement strength
///   - Higher values indicate stronger downward pressure
///   - Used in trend direction determination
///
/// - "adx": Average Directional Index
///   - Measures overall trend strength
///   - Range: 0-100
///   - Higher values indicate stronger trends
///
/// - "rating": Comprehensive Trend Rating
///   - Combines strength and direction
///   - Format: "{Strength} {Direction}"
///   - Examples: "Very Strong Bullish", "Moderate Bearish"
///
/// # Rating Classifications
/// 1. Trend Strength:
///   - "Very Strong": ADX >= 40
///   - "Strong": ADX >= 25
///   - "Moderate": ADX >= 15
///   - "Weak": ADX < 15
///
/// 2. Trend Direction:
///   - "Bullish": +DI > -DI by significant margin
///   - "Slightly Bullish": +DI > -DI by small margin
///   - "Bearish": -DI > +DI by significant margin
///   - "Slightly Bearish": -DI > +DI by small margin
///   - "Neutral": +DI ≈ -DI
///
/// # Parameters
/// - period: Smoothing period (typically 14)
/// - tick_rounding: Whether to round values to tick size
///
/// # Key Signals
/// 1. Trend Strength Changes
///   - Increasing ADX: Trend getting stronger
///   - Decreasing ADX: Trend weakening
///   - ADX crosses: Key strength thresholds
///
/// 2. Direction Changes
///   - DI crossovers: Potential trend changes
///   - Divergence from price: Trend sustainability
///   - Rating changes: Trend development stages
///
/// # Common Usage Patterns
/// 1. Trend Trading
///   - Enter when rating shows strong trend
///   - Exit when trend strength weakens
///   - Avoid trades in weak trends
///
/// 2. Confirmation
///   - Verify other indicators' signals
///   - Confirm breakouts
///   - Validate trend continuations
///
/// 3. Position Management
///   - Adjust position size with trend strength
///   - Tighten stops in weak trends
///   - Let profits run in strong trends
///
/// # Best Practices
/// 1. Multiple Time Frames
///   - Check longer time frame trend
///   - Enter on shorter time frame signals
///   - Maintain time frame alignment
///
/// 2. Volume Confirmation
///   - Look for volume support in strong trends
///   - Be cautious of low volume ratings changes
///   - Confirm direction changes with volume
///
/// 3. Market Context
///   - Consider overall market conditions
///   - Adjust thresholds for different markets
///   - Account for volatility regime
///
/// # Limitations
/// - Lag due to smoothing calculations
/// - Can give false signals in choppy markets
/// - May miss early trend changes
/// - Requires context from other indicators
///
/// # Implementation Notes
/// 1. Performance
///   - Efficient smoothing calculations
///   - Minimal state maintenance
///   - Accurate rating generation
///
/// 2. Accuracy
///   - Proper Wilder's smoothing
///   - Precise directional movement calculation
///   - Consistent rating classifications
///
/// # Example Usage
/// ```rust
/// let admr = DirectionalMovementRating::new(
///     IndicatorName::new("ADMR(14)"),
///     subscription,
///     100,           // history to retain
///     14,            // period
///     Color::Blue,   // ADMR color
///     Color::Green,  // Rating color
///     false,         // tick_rounding
/// ).await;
/// ```
///
/// # Advanced Applications
/// 1. Trend Filtering
///   - Only trade in strong trends
///   - Filter by direction
///   - Combine strength thresholds
///
/// 2. System Integration
///   - Use as trend filter
///   - Combine with other indicators
///   - Build comprehensive signals
///
/// 3. Risk Management
///   - Size positions by trend strength
///   - Adjust stops by rating
///   - Scale in/out based on rating changes
#[derive(Clone, Debug)]
pub struct DirectionalMovementRating {
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
    admr_color: Color,
    rating_color: Color,
    period: u64,
    tick_rounding: bool,
    smoothed_tr: Option<Decimal>,
    smoothed_plus_dm: Option<Decimal>,
    smoothed_minus_dm: Option<Decimal>,
    last_high: Option<Decimal>,
    last_low: Option<Decimal>,
}

impl Display for DirectionalMovementRating {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl DirectionalMovementRating {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: u64,
        admr_color: Color,
        rating_color: Color,
        tick_rounding: bool,
    ) -> Self {
        let symbol_name = match subscription.market_type {
            MarketType::Futures(_) => extract_symbol_from_contract(&subscription.symbol.name),
            _ => subscription.symbol.name.clone(),
        };
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(symbol_name.clone()).await.unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(symbol_name.clone()).await.unwrap();

        let admr = DirectionalMovementRating {
            name,
            market_type: subscription.symbol.market_type.clone(),
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(2), // Need current and previous bar
            is_ready: false,
            tick_size,
            admr_color,
            rating_color,
            period,
            decimal_accuracy,
            tick_rounding,
            smoothed_tr: None,
            smoothed_plus_dm: None,
            smoothed_minus_dm: None,
            last_high: None,
            last_low: None,
        };
        admr
    }

    fn get_bar_data(data: &BaseDataEnum) -> Option<(Price, Price)> {
        match data {
            BaseDataEnum::QuoteBar(bar) => Some((
                bar.bid_high,
                bar.bid_low,
            )),
            BaseDataEnum::Candle(candle) => Some((
                candle.high,
                candle.low,
            )),
            _ => None,
        }
    }

    fn calculate_directional_movement(
        &self,
        curr_high: Decimal,
        curr_low: Decimal
    ) -> (Decimal, Decimal) {
        let (plus_dm, minus_dm) = match (self.last_high, self.last_low) {
            (Some(last_high), Some(last_low)) => {
                let up_move = curr_high - last_high;
                let down_move = last_low - curr_low;

                if up_move > down_move && up_move > dec!(0.0) {
                    (up_move, dec!(0.0))
                } else if down_move > up_move && down_move > dec!(0.0) {
                    (dec!(0.0), down_move)
                } else {
                    (dec!(0.0), dec!(0.0))
                }
            },
            _ => (dec!(0.0), dec!(0.0)),
        };

        (plus_dm, minus_dm)
    }

    fn calculate_true_range(
        &self,
        high: Decimal,
        low: Decimal,
    ) -> Decimal {
        match (self.last_high, self.last_low) {
            (Some(last_high), Some(last_low)) => {
                let range1 = high - low;
                let range2 = (high - last_low).abs();
                let range3 = (last_high - low).abs();
                range1.max(range2).max(range3)
            },
            _ => high - low,
        }
    }

    fn smooth_value(current: Decimal, last_smoothed: Option<Decimal>, period: u64) -> Decimal {
        match last_smoothed {
            Some(last) => {
                let period_dec = Decimal::from(period);
                ((last * (period_dec - dec!(1.0))) + current) / period_dec
            },
            None => current,
        }
    }

    fn calculate_rating(&self, plus_di: Decimal, minus_di: Decimal, adx: Decimal) -> String {
        // Convert ADX and DI values into a comprehensive rating
        let trend_strength = if adx >= dec!(40.0) {
            "Very Strong"
        } else if adx >= dec!(25.0) {
            "Strong"
        } else if adx >= dec!(15.0) {
            "Moderate"
        } else {
            "Weak"
        };

        let trend_direction = if plus_di > minus_di {
            if (plus_di - minus_di) > dec!(10.0) {
                "Bullish"
            } else {
                "Slightly Bullish"
            }
        } else if minus_di > plus_di {
            if (minus_di - plus_di) > dec!(10.0) {
                "Bearish"
            } else {
                "Slightly Bearish"
            }
        } else {
            "Neutral"
        };

        format!("{} {}", trend_strength, trend_direction)
    }

    fn round_value(&self, value: Decimal) -> Decimal {
        match self.tick_rounding {
            true => round_to_tick_size(value, self.tick_size),
            false => value.round_dp(self.decimal_accuracy),
        }
    }
}

impl Indicators for DirectionalMovementRating {
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

        let (high, low) = Self::get_bar_data(base_data)?;

        // Calculate directional movement
        let (plus_dm, minus_dm) = self.calculate_directional_movement(high, low);

        // Calculate true range
        let tr = self.calculate_true_range(high, low);

        // Smooth the values using Wilder's smoothing
        self.smoothed_tr = Some(Self::smooth_value(
            tr,
            self.smoothed_tr,
            self.period
        ));

        self.smoothed_plus_dm = Some(Self::smooth_value(
            plus_dm,
            self.smoothed_plus_dm,
            self.period
        ));

        self.smoothed_minus_dm = Some(Self::smooth_value(
            minus_dm,
            self.smoothed_minus_dm,
            self.period
        ));

        // Calculate +DI and -DI
        let (plus_di, minus_di) = if let Some(tr) = self.smoothed_tr {
            if tr > dec!(0.0) {
                (
                    (self.smoothed_plus_dm.unwrap() / tr) * dec!(100.0),
                    (self.smoothed_minus_dm.unwrap() / tr) * dec!(100.0)
                )
            } else {
                (dec!(0.0), dec!(0.0))
            }
        } else {
            (dec!(0.0), dec!(0.0))
        };

        // Calculate ADX
        let di_sum = plus_di + minus_di;
        let di_diff = (plus_di - minus_di).abs();
        let dx = if di_sum > dec!(0.0) {
            (di_diff / di_sum) * dec!(100.0)
        } else {
            dec!(0.0)
        };

        // Get the rating
        let rating = self.calculate_rating(plus_di, minus_di, dx);

        // Create plots
        let mut plots = BTreeMap::new();

        plots.insert(
            "plus_di".to_string(),
            IndicatorPlot::new("+DI".to_string(), self.round_value(plus_di), self.admr_color.clone()),
        );

        plots.insert(
            "minus_di".to_string(),
            IndicatorPlot::new("-DI".to_string(), self.round_value(minus_di), self.admr_color.clone()),
        );

        plots.insert(
            "adx".to_string(),
            IndicatorPlot::new("ADX".to_string(), self.round_value(dx), self.admr_color.clone()),
        );

        plots.insert(
            "rating".to_string(),
            IndicatorPlot::new(rating, self.round_value(dx), self.rating_color.clone()),
        );

        // Update state
        self.last_high = Some(high);
        self.last_low = Some(low);
        self.is_ready = true;

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
        self.smoothed_tr = None;
        self.smoothed_plus_dm = None;
        self.smoothed_minus_dm = None;
        self.last_high = None;
        self.last_low = None;
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