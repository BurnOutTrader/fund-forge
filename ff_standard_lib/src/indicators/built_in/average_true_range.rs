use crate::indicators::indicator_trait::{Indicator, IndicatorResult};
use crate::rolling_window::RollingWindow;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::subscriptions::DataSubscription;

pub struct AverageTrueRange {
    name: String,
    subscription: DataSubscription,
    history: RollingWindow<IndicatorResult>,
    base_data_history: RollingWindow<BaseDataEnum>,
}

impl AverageTrueRange {
    fn new(subscription: DataSubscription, history_to_retain: usize, period: usize) -> Self {
        AverageTrueRange {
            name: format!("Average True Range: {:?} {} {}", subscription.symbol, subscription.resolution, subscription.symbol.data_vendor),
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period),
        }
    }

    fn calculate_true_range(&self) -> f64 {
        let base_data = self.base_data_history.history();
        let mut true_ranges = Vec::new();

        for i in 1..base_data.len() {
            match (&base_data[i - 1], &base_data[i]) {
                (BaseDataEnum::QuoteBar(prev_bar), BaseDataEnum::QuoteBar(curr_bar)) => {
                    let high_low = curr_bar.bid_high - curr_bar.bid_low;
                    let high_close = (curr_bar.bid_high - prev_bar.bid_close).abs();
                    let low_close = (curr_bar.bid_low - prev_bar.bid_close).abs();
                    true_ranges.push(high_low.max(high_close).max(low_close));
                },
                (BaseDataEnum::Candle(prev_candle), BaseDataEnum::Candle(curr_candle)) => {
                    let high_low = curr_candle.high - curr_candle.low;
                    let high_close = (curr_candle.high - prev_candle.close).abs();
                    let low_close = (curr_candle.low - prev_candle.close).abs();
                    true_ranges.push(high_low.max(high_close).max(low_close));
                },
                _ => panic!("Unsupported data type for AverageTrueRange"),
            }
        }

        // Calculate the average of true ranges (ATR)
        let atr = if !true_ranges.is_empty() {
            true_ranges.iter().sum::<f64>() / true_ranges.len() as f64
        } else {
            0.0
        };

        atr
    }
}

impl Indicator for AverageTrueRange {
    
    fn name(&self) -> &str {
        &self.name
    }

    fn subscription(&self) -> &DataSubscription {
        &self.subscription
    }

    fn update(&mut self, base_data: BaseDataEnum) -> Option<IndicatorResult> {
        match base_data {
            BaseDataEnum::QuoteBar(_) => (),
            BaseDataEnum::Candle(_) => (),
            _ => panic!("Unsupported data type for AverageTrueRange")
        }
        
        if base_data.subscription() != self.subscription {
            panic!("Subscription mismatch in AverageTrueRange")
        }
        
        self.base_data_history.add(base_data.clone());
        if !self.base_data_history.is_full() {
            return None
        }
        
        let atr = self.calculate_true_range();
        let result = IndicatorResult::new(atr, base_data.time_utc(), String::from("ATR"));
        
        if base_data.is_closed() {
            self.history.add(result.clone());
        }
        
        Some(result)
        
    }

    fn reset(&mut self) {
        self.history.clear();
        self.base_data_history.clear();
    }

    fn index(&self, index: usize) -> Option<IndicatorResult> {
        self.history.get(index).cloned()
    }

    fn plots(&self) -> RollingWindow<IndicatorResult> {
        self.history.clone()
    }
}