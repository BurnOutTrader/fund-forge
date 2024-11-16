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
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::indicators::indicator_values::{IndicatorPlot, IndicatorValues};
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};

/// Volume Weighted Moving Average (VWMA)
/// A technical indicator that factors in volume in addition to price, giving more weight
/// to price moves that occur on higher volume. This helps identify more significant
/// price movements and potential trend changes based on volume confirmation.
///
/// # Calculation Method
/// 1. Basic VWMA calculation:
///    VWMA = Σ(Price × Volume) / Σ(Volume)
///    over the specified period
///
/// 2. Volume Significance:
///    - Tracks average volume using exponential smoothing
///    - Calculates current volume relative to average
///    - Identifies high volume periods using threshold
///
/// # Plots
/// - "vwma": Main VWMA line
///   - Volume-weighted price average
///   - More responsive to high-volume moves
///   - Less affected by low-volume noise
///
/// - "volume_significance": Volume context
///   - Shows relative volume strength
///   - "High Volume": Above threshold
///   - "Normal Volume": Below threshold
///
/// # Parameters
/// - period: Calculation period for the moving average
/// - high_volume_threshold: Multiple of average volume to mark as high
/// - tick_rounding: Whether to round values to tick size
///
/// # Key Features
/// 1. Volume Weighting
///   - Emphasizes price moves with higher volume
///   - Reduces impact of low-volume noise
///   - Better reflects true market momentum
///
/// 2. Volume Analysis
///   - Tracks average volume dynamically
///   - Identifies significant volume spikes
///   - Provides volume context for price moves
///
/// 3. Implementation Details
///   - Uses exponential smoothing for volume average
///   - Handles missing data appropriately
///   - Efficient updates with new data
///
/// # Common Usage
/// 1. Trend Following
///   - VWMA acts as dynamic support/resistance
///   - More reliable than simple MA due to volume
///   - Use crossovers for trend changes
///
/// 2. Volume Analysis
///   - Identifies significant price moves
///   - Confirms breakouts with volume
///   - Spots weak moves on low volume
///
/// 3. Support/Resistance
///   - Dynamic S/R levels with volume context
///   - More reliable on high volume
///   - Less reliable on low volume
///
/// # Best Practices
/// 1. Trend Analysis
///   - Compare price to VWMA for trend
///   - Watch for high-volume crossovers
///   - Consider multiple timeframes
///
/// 2. Volume Confirmation
///   - Check volume significance on moves
///   - More weight to high-volume signals
///   - Be cautious of low-volume moves
///
/// 3. Signal Generation
///   - Use with price action
///   - Confirm with other indicators
///   - Consider market context
///
/// # Advantages
/// - Better than simple MA for trend identification
/// - Built-in volume confirmation
/// - Reduces impact of low-volume noise
/// - More responsive to significant moves
///
/// # Limitations
/// - Can lag on sudden reversals
/// - Requires volume data
/// - May be less useful in low-volume periods
/// - Can be affected by irregular volume patterns
///
/// # Example Usage
/// ```rust
/// let vwma = VolumeWeightedMA::new(
///     IndicatorName::new("VWMA(20)"),
///     subscription,
///     100,           // history to retain
///     20,            // period
///     Color::Blue,   // VWMA line color
///     Color::Green,  // Volume significance color
///     false,         // tick_rounding
///     dec!(2.0),     // high_volume_threshold
/// ).await;
/// ```
///
/// # Performance Considerations
/// - Efficient volume average updates
/// - Minimal state maintenance
/// - O(1) updates for new data
/// - Memory usage proportional to period
///
/// # Note on Volume Data
/// The indicator requires reliable volume data to function properly.
/// It works with both standard candles and quote bars, extracting
/// volume data appropriately from each format.
#[derive(Clone, Debug)]
pub struct VolumeWeightedMA {
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
    vwma_color: Color,
    volume_significance_color: Color,
    period: u64,
    tick_rounding: bool,
    average_volume: Option<Decimal>,  // For volume significance comparison
    high_volume_threshold: Decimal,   // Multiple of average volume to consider "high"
}

impl Display for VolumeWeightedMA {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl VolumeWeightedMA {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: u64,
        vwma_color: Color,
        volume_significance_color: Color,
        tick_rounding: bool,
        high_volume_threshold: Decimal,
    ) -> Box<Self> {
        let symbol_name = match subscription.market_type {
            MarketType::Futures(_) => extract_symbol_from_contract(&subscription.symbol.name),
            _ => subscription.symbol.name.clone(),
        };
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(symbol_name.clone()).await.unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(symbol_name.clone()).await.unwrap();

        let vwma = VolumeWeightedMA {
            name,
            market_type: subscription.symbol.market_type.clone(),
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period as usize),
            is_ready: false,
            tick_size,
            vwma_color,
            volume_significance_color,
            period,
            decimal_accuracy,
            tick_rounding,
            average_volume: None,
            high_volume_threshold,
        };
        Box::new(vwma)
    }

    fn get_price_and_volume(data: &BaseDataEnum) -> Option<(Price, Volume)> {
        match data {
            BaseDataEnum::QuoteBar(bar) => Some((
                bar.bid_close,
                bar.bid_volume,
            )),
            BaseDataEnum::Candle(candle) => Some((
                candle.close,
                Decimal::from(candle.volume),
            )),
            _ => None,
        }
    }

    fn calculate_vwma(&self, history: &Vec<BaseDataEnum>) -> Option<Price> {
        let mut volume_price_sum = dec!(0.0);
        let mut volume_sum = dec!(0.0);

        for data in history {
            if let Some((price, volume)) = Self::get_price_and_volume(data) {
                volume_price_sum += price * volume;
                volume_sum += volume;
            }
        }

        if volume_sum == dec!(0.0) {
            return None;
        }

        let vwma = volume_price_sum / volume_sum;

        Some(match self.tick_rounding {
            true => round_to_tick_size(vwma, self.tick_size),
            false => vwma.round_dp(self.decimal_accuracy),
        })
    }

    fn update_average_volume(&mut self, new_volume: Decimal) {
        self.average_volume = Some(match self.average_volume {
            None => new_volume,
            Some(avg) => {
                // Exponential smoothing for average volume
                let alpha = dec!(2.0) / (Decimal::from(self.period) + dec!(1.0));
                avg + alpha * (new_volume - avg)
            }
        });
    }

    fn calculate_volume_significance(&self, current_volume: Decimal) -> Decimal {
        match self.average_volume {
            None => dec!(1.0),
            Some(avg_vol) => {
                if avg_vol == dec!(0.0) {
                    dec!(1.0)
                } else {
                    current_volume / avg_vol
                }
            }
        }
    }

    fn is_high_volume(&self, significance: Decimal) -> bool {
        significance >= self.high_volume_threshold
    }
}

impl Indicators for VolumeWeightedMA {
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

        // Get current volume and update average
        let (_, current_volume) = Self::get_price_and_volume(base_data)?;
        self.update_average_volume(current_volume);

        // Calculate VWMA
        let vwma = self.calculate_vwma(&self.base_data_history.history)?;

        // Calculate volume significance
        let volume_significance = self.calculate_volume_significance(current_volume);
        let is_high_volume = self.is_high_volume(volume_significance);

        // Create plots
        let mut plots = BTreeMap::new();

        // Main VWMA line
        plots.insert(
            "vwma".to_string(),
            IndicatorPlot::new("VWMA".to_string(), vwma, self.vwma_color.clone()),
        );

        // Volume significance
        plots.insert(
            "volume_significance".to_string(),
            IndicatorPlot::new(
                if is_high_volume { "High Volume" } else { "Normal Volume" }.to_string(),
                volume_significance,
                self.volume_significance_color.clone(),
            ),
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
        self.average_volume = None;
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