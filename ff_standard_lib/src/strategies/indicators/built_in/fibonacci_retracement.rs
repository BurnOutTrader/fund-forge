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

#[derive(Clone, Debug)]
pub struct FibonacciRetracement {
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
    level_colors: Vec<Color>,
    tick_rounding: bool,
    lookback_period: u64,          // Period to identify swings
    swing_threshold: Decimal,      // Minimum price move to identify swing
    last_swing_high: Option<(DateTime<Utc>, Decimal)>,  // (time, price)
    last_swing_low: Option<(DateTime<Utc>, Decimal)>,
    trend_direction: Option<bool>, // true for uptrend
    fib_levels: Vec<Decimal>,     // Standard Fibonacci levels
}

impl Display for FibonacciRetracement {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl FibonacciRetracement {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        lookback_period: u64,
        swing_threshold: Decimal,
        level_colors: Vec<Color>,
        tick_rounding: bool,
    ) -> Self {
        let symbol_name = match subscription.market_type {
            MarketType::Futures(_) => extract_symbol_from_contract(&subscription.symbol.name),
            _ => subscription.symbol.name.clone(),
        };
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(symbol_name.clone()).await.unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(symbol_name.clone()).await.unwrap();

        let fib = FibonacciRetracement {
            name,
            market_type: subscription.symbol.market_type.clone(),
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(lookback_period as usize),
            is_ready: false,
            tick_size,
            level_colors,
            decimal_accuracy,
            tick_rounding,
            lookback_period,
            swing_threshold,
            last_swing_high: None,
            last_swing_low: None,
            trend_direction: None,
            fib_levels: vec![
                dec!(0.0),    // 0%
                dec!(0.236),  // 23.6%
                dec!(0.382),  // 38.2%
                dec!(0.5),    // 50%
                dec!(0.618),  // 61.8%
                dec!(0.786),  // 78.6%
                dec!(1.0),    // 100%
                dec!(1.618),  // 161.8% extension
                dec!(2.618),  // 261.8% extension
            ],
        };
        fib
    }

    fn get_high_low_close(data: &BaseDataEnum) -> Option<(Price, Price, Price)> {
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

    fn identify_swings(&mut self) -> Option<(bool, Decimal, Decimal)> {
        let history = self.base_data_history.history();
        if history.len() < 3 {
            return None;
        }

        let mut highs: Vec<(usize, DateTime<Utc>, Decimal)> = Vec::new();
        let mut lows: Vec<(usize, DateTime<Utc>, Decimal)> = Vec::new();

        // First pass: identify potential swing points
        for i in 1..history.len() - 1 {
            let (prev_high, prev_low, _) = Self::get_high_low_close(&history[i - 1])?;
            let (curr_high, curr_low, _) = Self::get_high_low_close(&history[i])?;
            let (next_high, next_low, _) = Self::get_high_low_close(&history[i + 1])?;

            // Potential swing high
            if curr_high > prev_high && curr_high > next_high {
                highs.push((i, history[i].time_closed_utc(), curr_high));
            }

            // Potential swing low
            if curr_low < prev_low && curr_low < next_low {
                lows.push((i, history[i].time_closed_utc(), curr_low));
            }
        }

        // Second pass: filter significant swings
        if highs.is_empty() || lows.is_empty() {
            return None;
        }

        // Find most recent significant swing high and low
        let significant_high = highs.iter()
            .rev()
            .find(|&&(_, _, price)| {
                lows.iter()
                    .any(|&(_, _, low_price)|
                        (price - low_price).abs() >= self.swing_threshold
                    )
            });

        let significant_low = lows.iter()
            .rev()
            .find(|&&(_, _, price)| {
                highs.iter()
                    .any(|&(_, _, high_price)|
                        (high_price - price).abs() >= self.swing_threshold
                    )
            });

        match (significant_high, significant_low) {
            (Some(&(high_idx, high_time, high_price)),
                Some(&(low_idx, low_time, low_price))) => {
                // Determine trend direction based on most recent swing
                let trend_up = if high_idx != low_idx {
                    high_idx > low_idx
                } else {
                    high_time > low_time
                };

                self.trend_direction = Some(trend_up);
                if trend_up {
                    self.last_swing_low = Some((low_time, low_price));
                    self.last_swing_high = Some((high_time, high_price));
                } else {
                    self.last_swing_high = Some((high_time, high_price));
                    self.last_swing_low = Some((low_time, low_price));
                }

                Some((trend_up, high_price, low_price))
            },
            _ => None,
        }
    }

    fn calculate_levels(
        &self,
        start_price: Decimal,
        end_price: Decimal,
        trend_up: bool,
    ) -> Vec<(String, Decimal)> {
        let price_range = end_price - start_price;

        self.fib_levels.iter()
            .map(|&level| {
                let level_price = if trend_up {
                    end_price - (price_range * level)
                } else {
                    start_price + (price_range * level)
                };

                let level_price = match self.tick_rounding {
                    true => round_to_tick_size(level_price, self.tick_size),
                    false => level_price.round_dp(self.decimal_accuracy),
                };

                // Convert level to percentage string
                let level_str = format!("{}%", (level * dec!(100.0)).round_dp(1));
                (level_str, level_price)
            })
            .collect()
    }

    fn get_level_description(&self, level: Decimal) -> String {
        match level {
            l if l == dec!(0.0) => "Strong Support/Resistance",
            l if l == dec!(0.236) => "Weak Retracement",
            l if l == dec!(0.382) => "Key Retracement",
            l if l == dec!(0.5) => "Medium Retracement",
            l if l == dec!(0.618) => "Golden Ratio",
            l if l == dec!(0.786) => "Deep Retracement",
            l if l == dec!(1.0) => "Full Retracement",
            l if l == dec!(1.618) => "Golden Extension",
            l if l == dec!(2.618) => "Deep Extension",
            _ => "Custom Level",
        }.to_string()
    }
}

impl Indicators for FibonacciRetracement {
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

        // Identify new swing points and calculate levels
        if let Some((trend_up, high, low)) = self.identify_swings() {
            let (start_price, end_price) = if trend_up {
                (low, high)
            } else {
                (high, low)
            };

            let levels = self.calculate_levels(start_price, end_price, trend_up);

            // Create plots
            let mut plots = BTreeMap::new();

            // Add each Fibonacci level
            for (i, (level_str, price)) in levels.iter().enumerate() {
                let color = &self.level_colors[i % self.level_colors.len()];
                let description = self.get_level_description(self.fib_levels[i]);

                plots.insert(
                    format!("fib_{}", level_str),
                    IndicatorPlot::new(
                        format!("{} ({})", description, level_str),
                        *price,
                        color.clone(),
                    ),
                );
            }

            // Add trend direction
            plots.insert(
                "trend".to_string(),
                IndicatorPlot::new(
                    if trend_up { "Uptrend" } else { "Downtrend" }.to_string(),
                    end_price,
                    self.level_colors[0].clone(),
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
        } else {
            None
        }
    }

    // ... implement other required trait methods ...
    fn subscription(&self) -> &DataSubscription {
        &self.subscription
    }

    fn reset(&mut self) {
        self.history.clear();
        self.base_data_history.clear();
        self.is_ready = false;
        self.last_swing_high = None;
        self.last_swing_low = None;
        self.trend_direction = None;
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
        self.lookback_period
    }
}