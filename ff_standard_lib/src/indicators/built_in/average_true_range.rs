use std::fmt;
use std::fmt::{Display, Formatter};
use crate::indicators::indicator_trait::{IndicatorName, IndicatorValue, Indicators};
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::subscriptions::DataSubscription;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::apis::vendor::client_requests::ClientSideDataVendor;
use crate::helpers::decimal_calculators::round_to_decimals;

pub struct AverageTrueRange {
    name: IndicatorName,
    subscription: DataSubscription,
    history: RollingWindow<Vec<IndicatorValue>>,
    base_data_history: RollingWindow<BaseDataEnum>,
    is_ready: bool,
    period: u64,
    decimal_accuracy: u64,
}

impl Display for AverageTrueRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(values) => {
                let values_string = values.iter().map(|x| x.to_string()).collect::<Vec<String>>().join("\n");
                write!(f, "{}: {}\n{}", self.name(), self.period, values_string)

            },
            None => write!(f, "{}", format!("{}: {}: No values", self.name(), self.period)),
        }
    }
}

impl AverageTrueRange {
    pub async fn new(subscription: DataSubscription, history_to_retain: u64, period: u64) -> Self {
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(subscription.symbol.clone()).await;
        let mut atr = AverageTrueRange {
            name: String::from("Average True Range"),
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period),
            is_ready: false,
            period,
            decimal_accuracy: decimal_accuracy.unwrap(),
        };
        atr.name = atr.name();
        atr
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
            round_to_decimals(true_ranges.iter().sum::<f64>() / true_ranges.len() as f64, self.decimal_accuracy.clone()) 
        } else {
            0.0
        };

        atr
    }
}

impl Indicators for AverageTrueRange {
    fn subscription(&self) -> &DataSubscription {
        &self.subscription
    }

    fn update_base_data(&mut self, base_data: BaseDataEnum) -> Option<Vec<IndicatorValue>> {
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
 
        let result = IndicatorValue::new(atr, base_data.time_utc(), String::from("atr"));
        let mut results = Vec::new();
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

    fn index(&self, index: usize) -> Option<Vec<IndicatorValue>> {
        self.history.get(index).cloned()
    }

    fn plots(&self) -> RollingWindow<Vec<IndicatorValue>> {
        self.history.clone()
    }
    
    fn is_ready(&self) -> bool {
        self.is_ready
    }
}