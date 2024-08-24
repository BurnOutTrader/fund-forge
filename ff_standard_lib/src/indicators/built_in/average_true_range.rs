
use crate::rolling_window::RollingWindow;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::subscriptions::DataSubscription;
/*
pub struct AverageTrueRange {
    subscription: DataSubscription,
    history: RollingWindow<IndicatorResults>,
    base_data_history: RollingWindow<BaseDataEnum>,
    is_ready: bool,
}

impl AverageTrueRange {
    fn new(subscription: DataSubscription, history_to_retain: usize, period: usize) -> Self {
        AverageTrueRange {
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period),
            is_ready: false,
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

impl AverageTrueRange {
    fn subscription(&self) -> &DataSubscription {
        &self.subscription
    }

    fn update(&mut self, base_data: BaseDataEnum) -> Option<IndicatorResults> {
        if !base_data.is_closed() {
            return None
        }
        
        self.base_data_history.add(base_data.clone());
        if !self.base_data_history.is_full() {
            return None
        } else if self.is_ready == false {
            self.is_ready = true;
        }
        
        let atr = self.calculate_true_range();
        if atr == 0.0 {
            return None
        }
 
        let result = IndicatorResult::new(atr, base_data.time_utc(), String::from("atr"));
        let mut results =IndicatorResults::new();
        results.push(result);
        
        if base_data.is_closed() {
            self.history.add(results.clone());
        }
        
        Some(results)
        
    }

    fn reset(&mut self) {
        self.history.clear();
        self.base_data_history.clear();
    }

    fn index(&self, index: usize) -> Option<IndicatorResults> {
        self.history.get(index).cloned()
    }

    fn plots(&self) -> RollingWindow<IndicatorResults> {
        self.history.clone()
    }
    
    fn is_ready(&self) -> bool {
        self.is_ready
    }
}*/