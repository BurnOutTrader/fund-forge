use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use ahash::AHashMap;
use chrono::{DateTime, Utc};
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

/// Volume Profile Distribution
/// =========================
///
/// Description:
/// A comprehensive volume analysis tool that creates a volume profile of price levels,
/// identifying significant areas of price acceptance and rejection.
///
/// Key Components:
/// 1. Price Level Analysis
///    - Volume nodes at each price
///    - Buying/selling pressure tracking
///    - Volume delta calculations
///    - Time-weighted analysis
///
/// 2. Key Levels
///    - Point of Control (POC) - Highest volume price
///    - Value Area High (VAH)
///    - Value Area Low (VAL)
///    - High Volume Nodes
///    - Low Volume Nodes
///
/// 3. Volume Analysis
///    - Volume distribution by price
///    - Buy/sell pressure analysis
///    - Cumulative delta tracking
///    - Volume node classification
///
/// Output Plots:
/// - POC, VAH, VAL
/// - Volume profile bars
/// - Pressure zones
/// - Cumulative delta
/// - Volume node markers
///
/// Use Cases:
/// - Identifying significant price levels
/// - Finding areas of support/resistance
/// - Understanding price acceptance/rejection
/// - Analyzing market structure
/// - Trading volume imbalances
///
/// Configuration Options:
/// - Value area percentage (typically 70%)
/// - Number of profile bars
/// - Update interval
/// - Color coding for different elements
/// - Volume node thresholds
#[derive(Clone, Debug)]
pub struct VolumeProfileDistribution {
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
    price_levels: AHashMap<Decimal, VolumeNode>,
    value_area_percentage: Decimal,  // Typically 70%
    num_profile_bars: u32,          // Number of horizontal volume bars
    poc_color: Color,               // Point of Control color
    vah_color: Color,               // Value Area High color
    val_color: Color,               // Value Area Low color
    profile_color: Color,           // Volume profile bars color
    high_volume_node_color: Color,  // High volume node color
    low_volume_node_color: Color,   // Low volume node color
    tick_rounding: bool,
    update_interval: u32,           // Bars between full recalculation
    bars_since_update: u32,
}

#[derive(Clone, Debug)]
struct VolumeNode {
    total_volume: Decimal,
    buy_volume: Decimal,
    sell_volume: Decimal,
    num_trades: u32,
    last_update: DateTime<Utc>,
}

impl VolumeNode {
    fn new(time: DateTime<Utc>) -> Self {
        VolumeNode {
            total_volume: dec!(0.0),
            buy_volume: dec!(0.0),
            sell_volume: dec!(0.0),
            num_trades: 0,
            last_update: time,
        }
    }

    fn delta(&self) -> Decimal {
        self.buy_volume - self.sell_volume
    }

    fn is_high_activity(&self, avg_volume: Decimal) -> bool {
        self.total_volume > avg_volume * dec!(1.5)
    }

    fn is_low_activity(&self, avg_volume: Decimal) -> bool {
        self.total_volume < avg_volume * dec!(0.5)
    }
}

impl Display for VolumeProfileDistribution {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl VolumeProfileDistribution {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        value_area_percentage: Decimal,
        num_profile_bars: u32,
        poc_color: Color,
        vah_color: Color,
        val_color: Color,
        profile_color: Color,
        high_volume_node_color: Color,
        low_volume_node_color: Color,
        tick_rounding: bool,
        update_interval: u32,
    ) -> Box<Self> {
        let symbol_name = match subscription.market_type {
            MarketType::Futures(_) => extract_symbol_from_contract(&subscription.symbol.name),
            _ => subscription.symbol.name.clone(),
        };
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(symbol_name.clone()).await.unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(symbol_name.clone()).await.unwrap();

        let vpd = VolumeProfileDistribution {
            name,
            market_type: subscription.symbol.market_type.clone(),
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(update_interval as usize),
            is_ready: false,
            tick_size,
            price_levels: AHashMap::new(),
            value_area_percentage,
            num_profile_bars,
            poc_color,
            vah_color,
            val_color,
            profile_color,
            high_volume_node_color,
            low_volume_node_color,
            decimal_accuracy,
            tick_rounding,
            update_interval,
            bars_since_update: 0,
        };
        Box::new(vpd)
    }

    #[allow(dead_code)]
    // Add new method to analyze buying/selling pressure
    fn analyze_price_levels(&self) -> Option<(Vec<(Decimal, String)>, Decimal)> {
        if self.price_levels.is_empty() {
            return None;
        }

        let mut delta_levels: Vec<(Decimal, Decimal)> = self.price_levels
            .iter()
            .map(|(price, node)| (*price, node.delta()))
            .collect();

        // Sort by absolute delta to find strongest imbalances
        delta_levels.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap());

        // Calculate cumulative delta
        let cumulative_delta: Decimal = self.price_levels
            .values()
            .map(|node| node.delta())
            .sum();

        // Generate pressure ratings for significant levels
        let pressure_levels: Vec<(Decimal, String)> = delta_levels
            .into_iter()
            .take(5)  // Top 5 most significant imbalances
            .map(|(price, delta)| {
                let pressure = if delta > dec!(0.0) {
                    format!("Strong Buying ({})", delta.round_dp(2))
                } else {
                    format!("Strong Selling ({})", delta.round_dp(2))
                };
                (price, pressure)
            })
            .collect();

        Some((pressure_levels, cumulative_delta))
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

    fn round_to_profile_level(&self, price: Decimal) -> Decimal {
        let rounded = match self.tick_rounding {
            true => round_to_tick_size(price, self.tick_size),
            false => price.round_dp(self.decimal_accuracy),
        };
        rounded
    }

    fn update_volume_node(
        &mut self,
        price: Decimal,
        volume: Decimal,
        is_buyer_initiated: bool,
        time: DateTime<Utc>,
    ) {
        let level = self.round_to_profile_level(price);
        let node = self.price_levels
            .entry(level)
            .or_insert_with(|| VolumeNode::new(time));

        node.total_volume += volume;
        if is_buyer_initiated {
            node.buy_volume += volume;
        } else {
            node.sell_volume += volume;
        }
        node.num_trades += 1;
        node.last_update = time;
    }

    fn calculate_value_area(&self) -> Option<(Decimal, Decimal, Decimal)> {
        if self.price_levels.is_empty() {
            return None;
        }

        // Find Point of Control (POC)
        let poc = self.price_levels
            .iter()
            .max_by(|a, b| a.1.total_volume.partial_cmp(&b.1.total_volume).unwrap())?
            .0
            .clone();

        // Calculate total volume
        let total_volume: Decimal = self.price_levels
            .values()
            .map(|node| node.total_volume)
            .sum();

        let target_volume = total_volume * self.value_area_percentage;
        let mut current_volume = self.price_levels[&poc].total_volume;
        let mut vah = poc;
        let mut val = poc;

        // Expand value area until target volume is reached
        while current_volume < target_volume {
            let above_price = vah + self.tick_size;
            let below_price = val - self.tick_size;

            let above_volume = self.price_levels.get(&above_price)
                .map_or(dec!(0.0), |node| node.total_volume);
            let below_volume = self.price_levels.get(&below_price)
                .map_or(dec!(0.0), |node| node.total_volume);

            if above_volume >= below_volume {
                vah = above_price;
                current_volume += above_volume;
            } else {
                val = below_price;
                current_volume += below_volume;
            }
        }

        Some((poc, vah, val))
    }

    fn get_volume_at_price_level(&self, price: Decimal) -> Option<Decimal> {
        let rounded_price = self.round_to_profile_level(price);
        Some(self.price_levels
            .get(&rounded_price)
            .map_or(dec!(0.0), |node| node.total_volume))
    }

    fn identify_volume_nodes(&self) -> Vec<(Decimal, bool)> {
        let avg_volume: Decimal = self.price_levels
            .values()
            .map(|node| node.total_volume)
            .sum::<Decimal>() / Decimal::from(self.price_levels.len());

        self.price_levels
            .iter()
            .filter_map(|(price, node)| {
                if node.is_high_activity(avg_volume) {
                    Some((*price, true))  // High volume node
                } else if node.is_low_activity(avg_volume) {
                    Some((*price, false)) // Low volume node
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Indicators for VolumeProfileDistribution {
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

        let (high, low, open, close, volume) = Self::get_bar_data(base_data)?;
        let time = base_data.time_closed_utc();

        // Update volume nodes
        let avg_price = (high + low) / dec!(2.0);
        let is_buyer_initiated = close > open;
        self.update_volume_node(avg_price, volume, is_buyer_initiated, time);

        self.bars_since_update += 1;
        if self.bars_since_update < self.update_interval {
            return None;
        }

        // Reset counter and recalculate full profile
        self.bars_since_update = 0;

        // Calculate value area and significant levels
        let (poc, vah, val) = self.calculate_value_area()?;
        let volume_nodes = self.identify_volume_nodes();

        // Create plots
        let mut plots = BTreeMap::new();

        // Add main levels
        plots.insert(
            "poc".to_string(),
            IndicatorPlot::new("Point of Control".to_string(), poc, self.poc_color.clone()),
        );

        plots.insert(
            "vah".to_string(),
            IndicatorPlot::new("Value Area High".to_string(), vah, self.vah_color.clone()),
        );

        plots.insert(
            "val".to_string(),
            IndicatorPlot::new("Value Area Low".to_string(), val, self.val_color.clone()),
        );

        // Add volume profile bars
        let price_range = high - low;
        let bar_size = price_range / Decimal::from(self.num_profile_bars);

        for i in 0..self.num_profile_bars {
            let bar_price = low + (bar_size * Decimal::from(i));
            let bar_volume = self.get_volume_at_price_level(bar_price)?;

            plots.insert(
                format!("profile_bar_{}", i),
                IndicatorPlot::new(
                    format!("Volume at {}", self.round_to_profile_level(bar_price)),
                    bar_volume,
                    self.profile_color.clone(),
                ),
            );
        }

        // Add volume nodes
        for (i, (price, is_high)) in volume_nodes.iter().enumerate() {
            let (name, color) = if *is_high {
                ("High Volume Node", self.high_volume_node_color.clone())
            } else {
                ("Low Volume Node", self.low_volume_node_color.clone())
            };

            plots.insert(
                format!("node_{}", i),
                IndicatorPlot::new(name.to_string(), *price, color),
            );
        }

        let values = IndicatorValues::new(
            self.name.clone(),
            self.subscription.clone(),
            plots,
            time,
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
        self.price_levels.clear();
        self.bars_since_update = 0;
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
        self.update_interval as u64
    }
}