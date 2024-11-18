use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::gui_types::settings::Color;
use crate::product_maps::rithmic::maps::extract_symbol_from_contract;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::enums::MarketType;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::indicators::indicator_values::{IndicatorPlot, IndicatorValues};
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};
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
    // Separate histories for bull and bear bars
    bull_strengths: Vec<Decimal>,
    bear_strengths: Vec<Decimal>,
    market_type: MarketType,
    decimal_accuracy: u32,
    is_ready: bool,
    plot_color: Color,
    period: u64,  // Number of bars to average
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
            bull_strengths: Vec::with_capacity(period as usize),
            bear_strengths: Vec::with_capacity(period as usize),
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

                let is_bullish = close >= open;
                let strength = if is_bullish {
                    ((close - low) / range) * dec!(100.0)
                } else {
                    ((high - close) / range) * dec!(100.0)
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

                let is_bullish = close >= open;
                let strength = if is_bullish {
                    ((close - low) / range) * dec!(100.0)
                } else {
                    ((high - close) / range) * dec!(100.0)
                };

                Some((strength.round_dp(self.decimal_accuracy), is_bullish))
            },
            _ => panic!("Unsupported data type for CloseStrength"),
        }
    }

    fn maintain_strength_histories(&mut self, strength: Decimal, is_bullish: bool) {
        // Update the appropriate history
        if is_bullish {
            self.bull_strengths.push(strength);
            if self.bull_strengths.len() > self.period as usize {
                self.bull_strengths.remove(0);
            }
        } else {
            self.bear_strengths.push(strength);
            if self.bear_strengths.len() > self.period as usize {
                self.bear_strengths.remove(0);
            }
        }
    }

    fn calculate_average_strengths(&self) -> (Option<Decimal>, Option<Decimal>, Option<Decimal>) {
        // Calculate overall average
        let all_strengths: Vec<Decimal> = self.bull_strengths.iter()
            .chain(self.bear_strengths.iter())
            .cloned()
            .collect();

        let overall_avg = if !all_strengths.is_empty() {
            let sum: Decimal = all_strengths.iter().sum();
            Some((sum / Decimal::from(all_strengths.len())).round_dp(self.decimal_accuracy))
        } else {
            None
        };

        // Calculate bull average
        let bull_avg = if !self.bull_strengths.is_empty() {
            let sum: Decimal = self.bull_strengths.iter().sum();
            Some((sum / Decimal::from(self.bull_strengths.len())).round_dp(self.decimal_accuracy))
        } else {
            None
        };

        // Calculate bear average
        let bear_avg = if !self.bear_strengths.is_empty() {
            let sum: Decimal = self.bear_strengths.iter().sum();
            Some((sum / Decimal::from(self.bear_strengths.len())).round_dp(self.decimal_accuracy))
        } else {
            None
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
        self.bull_strengths.clear();
        self.bear_strengths.clear();
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

        // Update strength histories
        self.maintain_strength_histories(single_strength, is_bullish);

        // Calculate averages
        let (avg_strength, bull_avg, bear_avg) = self.calculate_average_strengths();

        let avg_strength = avg_strength?;
        let bull_avg = bull_avg.unwrap_or_else(|| dec!(0.0));
        let bear_avg = bear_avg.unwrap_or_else(|| dec!(0.0));

        let mut plots = BTreeMap::new();
        plots.insert(
            "strength".to_string(),
            IndicatorPlot::new("strength".to_string(), single_strength, self.plot_color.clone()),
        );
        plots.insert(
            "strength_average".to_string(),
            IndicatorPlot::new("strength_average".to_string(), avg_strength, self.plot_color.clone()),
        );
        plots.insert(
            "bull_strength_average".to_string(),
            IndicatorPlot::new("bull_strength_average".to_string(), bull_avg, self.plot_color.clone()),
        );
        plots.insert(
            "bear_strength_average".to_string(),
            IndicatorPlot::new("bear_strength_average".to_string(), bear_avg, self.plot_color.clone()),
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