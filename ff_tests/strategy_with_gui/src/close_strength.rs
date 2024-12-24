use std::collections::BTreeMap;
use chrono::{DateTime, Duration, Utc};
use ff_standard_lib::gui_types::settings::Color;
use ff_standard_lib::product_maps::rithmic::maps::extract_symbol_from_contract;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::MarketType;
use ff_standard_lib::standardized_types::rolling_window::RollingWindow;
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
use ff_standard_lib::strategies::indicators::indicator_values::{IndicatorPlot, IndicatorValues};
use ff_standard_lib::strategies::indicators::indicators_trait::{IndicatorName, Indicators};
use rust_decimal::{Decimal, MathematicalOps};
use rust_decimal_macros::dec;

/// Calculates close strength as percentage of bar range in two ways:
/// 1. Single bar: Where did price close within the current bar's range?
/// 2. Average: What's the average close strength over N periods?
///
/// # Plots
/// - "strength": Single bar close strength
/// - "strength_average": Average close strength
/// - "bull_strength_average": Average close strength for bullish bars
/// - "bear_strength_average": Average close strength for bearish bars
///
/// For both calculations:
/// - 100% means closed at the high
/// - 0% means closed at the low
/// - 50% means closed in middle of range
///
#[derive(Clone, Debug)]
pub struct CloseStrength {
    name: IndicatorName,
    subscription: DataSubscription,
    history: RollingWindow<IndicatorValues>,
    base_data_history: RollingWindow<BaseDataEnum>,
    // Store both strength and timestamp for weighting calculations
    bull_bars: Vec<(Decimal, DateTime<Utc>)>,
    bear_bars: Vec<(Decimal, DateTime<Utc>)>,
    market_type: MarketType,
    decimal_accuracy: u32,
    is_ready: bool,
    plot_color: Color,
    period: u64,
}

impl CloseStrength {
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        plot_color: Color,
        period: u64,
    ) -> Box<Self> {
        let symbol_name = match subscription.market_type {
            MarketType::Futures(_) => extract_symbol_from_contract(&subscription.symbol.name),
            _ => subscription.symbol.name.clone(),
        };
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(symbol_name.clone()).await.unwrap();

        let indicator = CloseStrength {
            name,
            market_type: subscription.symbol.market_type.clone(),
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period as usize),
            bull_bars: Vec::with_capacity(period as usize),
            bear_bars: Vec::with_capacity(period as usize),
            is_ready: false,
            plot_color,
            decimal_accuracy,
            period,
        };
        Box::new(indicator)
    }

    fn calculate_single_bar_strength(&self, bar: &BaseDataEnum) -> Option<(Decimal, bool)> {
        match bar {
            BaseDataEnum::QuoteBar(quote_bar) => {
                let high = quote_bar.bid_high;
                let low = quote_bar.bid_low;
                let open = quote_bar.bid_open;
                let close = quote_bar.bid_close;

                let range = high - low;
                if range == dec!(0.0) {
                    return None;
                }

                let is_bullish = close > open;
                let strength = if close > open {
                    ((close - low) / range) * dec!(100.0)
                } else if close < open {
                    ((high - close) / range) * dec!(100.0)
                } else {
                    dec!(0.0)
                };

                Some((strength.round_dp(self.decimal_accuracy), is_bullish))
            },
            BaseDataEnum::Candle(candle) => {
                let high = candle.high;
                let low = candle.low;
                let open = candle.open;
                let close = candle.close;

                let range = high - low;
                if range == dec!(0.0) {
                    return None;
                }

                let is_bullish = close > open;
                let strength = if close > open {
                    ((close - low) / range) * dec!(100.0)
                } else if close < open {
                    ((high - close) / range) * dec!(100.0)
                } else {
                    dec!(0.0)
                };

                Some((strength.round_dp(self.decimal_accuracy), is_bullish))
            },
            _ => panic!("Unsupported data type for CloseStrength"),
        }
    }

    fn maintain_strength_histories(&mut self, strength: Decimal, is_bullish: bool, timestamp: DateTime<Utc>) {
        // Remove old entries based on timestamp
        let cutoff_time = timestamp - Duration::seconds((self.period * 60 * 60) as i64);

        if is_bullish {
            self.bull_bars.push((strength, timestamp));
            self.bull_bars.retain(|(_, time)| time >= &cutoff_time);
        } else {
            self.bear_bars.push((strength, timestamp));
            self.bear_bars.retain(|(_, time)| time >= &cutoff_time);
        }
    }

    fn calculate_weighted_strengths(&self) -> (Option<Decimal>, Option<Decimal>, Option<Decimal>) {
        let total_bars = self.bull_bars.len() + self.bear_bars.len();
        if total_bars == 0 {
            return (None, None, None);
        }

        // Calculate simple averages first
        let bull_avg = if !self.bull_bars.is_empty() {
            let sum: Decimal = self.bull_bars.iter().map(|(strength, _)| *strength).sum();
            let avg = (sum / Decimal::from(self.bull_bars.len())).round_dp(self.decimal_accuracy);
            Some(avg)
        } else {
            None
        };

        let bear_avg = if !self.bear_bars.is_empty() {
            let sum: Decimal = self.bear_bars.iter().map(|(strength, _)| *strength).sum();
            let avg = (sum / Decimal::from(self.bear_bars.len())).round_dp(self.decimal_accuracy);
            Some(avg)
        } else {
            None
        };

        // For overall average, weight by sample size
        let overall_avg = match (bull_avg, bear_avg) {
            (Some(bull), Some(bear)) => {
                let bull_size = Decimal::from(self.bull_bars.len());
                let bear_size = Decimal::from(self.bear_bars.len());
                let total_size = bull_size + bear_size;

                Some(((bull * bull_size + bear * bear_size) / total_size).round_dp(self.decimal_accuracy))
            },
            (Some(bull), None) => Some(bull),
            (None, Some(bear)) => Some(bear),
            (None, None) => None,
        };

        (overall_avg, bull_avg, bear_avg)
    }
}

impl Indicators for CloseStrength {
    fn name(&self) -> IndicatorName {
        self.name.clone()
    }

    fn history_to_retain(&self) -> usize {
        self.history.number.clone() as usize
    }

    fn subscription(&self) -> &DataSubscription {
        &self.subscription
    }

    fn reset(&mut self) {
        self.history.clear();
        self.base_data_history.clear();
        self.bull_bars.clear();
        self.bear_bars.clear();
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
        self.period  // Need full period for averaging
    }

    fn update_base_data(&mut self, base_data: &BaseDataEnum) -> Option<Vec<IndicatorValues>> {
        if !base_data.is_closed() {
            return None;
        }

        self.base_data_history.add(base_data.clone());

        if !self.is_ready {
            if self.base_data_history.is_full() {
                self.is_ready = true;
            } else {
                return None;
            }
        }

        // Calculate single bar strength and determine if bullish/bearish
        let (single_strength, is_bullish) = match self.calculate_single_bar_strength(base_data) {
            Some(result) => result,
            None => return None,
        };

        // Update strength histories with timestamp
        self.maintain_strength_histories(single_strength, is_bullish, base_data.time_closed_utc());

        // Calculate weighted averages
        let (avg_strength, bull_avg, bear_avg) = self.calculate_weighted_strengths();

        let avg_strength = avg_strength?;
        let bull_avg = bull_avg.unwrap_or_else(|| dec!(0.0));
        let bear_avg = bear_avg.unwrap_or_else(|| dec!(0.0));

        // Calculate bar type ratios for additional plots
        let total_bars = self.bull_bars.len() + self.bear_bars.len();
        let bull_ratio = if total_bars > 0 {
            (Decimal::from(self.bull_bars.len()) / Decimal::from(total_bars)) * dec!(100.0)
        } else {
            dec!(0.0)
        };
        let bear_ratio = if total_bars > 0 {
            (Decimal::from(self.bear_bars.len()) / Decimal::from(total_bars)) * dec!(100.0)
        } else {
            dec!(0.0)
        };

        let mut plots = BTreeMap::new();
        plots.insert(
            "strength".to_string(),
            IndicatorPlot::new("strength".to_string(), single_strength, self.plot_color.clone()),
        );
        plots.insert(
            "weighted_strength".to_string(),
            IndicatorPlot::new("weighted_strength".to_string(), avg_strength, self.plot_color.clone()),
        );
        plots.insert(
            "bull_strength".to_string(),
            IndicatorPlot::new("bull_strength".to_string(), bull_avg, self.plot_color.clone()),
        );
        plots.insert(
            "bear_strength".to_string(),
            IndicatorPlot::new("bear_strength".to_string(), bear_avg, self.plot_color.clone()),
        );
        plots.insert(
            "bull_ratio".to_string(),
            IndicatorPlot::new("bull_ratio".to_string(), bull_ratio, self.plot_color.clone()),
        );
        plots.insert(
            "bear_ratio".to_string(),
            IndicatorPlot::new("bear_ratio".to_string(), bear_ratio, self.plot_color.clone()),
        );

        let values = IndicatorValues::new(
            self.name.clone(),
            self.subscription.clone(),
            plots,
            base_data.time_closed_utc()
        );

        self.history.add(values.clone());
        Some(vec![values])
    }
}