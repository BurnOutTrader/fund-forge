use crate::helpers::decimal_calculators::round_to_tick_size;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::standardized_types::{Color, Price};
use std::collections::BTreeMap;
use std::fmt;
use std::fmt::{Display, Formatter};
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive};
use rust_decimal_macros::dec;
use crate::indicators::indicators_trait::{IndicatorName, Indicators};
use crate::indicators::values::{IndicatorPlot, IndicatorValues};
use crate::standardized_types::base_data::traits::BaseData;

pub struct AverageTrueRange {
    name: IndicatorName,
    subscription: DataSubscription,
    history: RollingWindow<IndicatorValues>,
    base_data_history: RollingWindow<BaseDataEnum>,
    is_ready: bool,
    tick_size: Price,
    plot_color: Option<Color>,
}

impl Display for AverageTrueRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl AverageTrueRange {
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: u64,
        plot_color: Option<Color>,
    ) -> Self {
        let tick_size = subscription
            .symbol
            .tick_size()
            .await.unwrap();
        let atr = AverageTrueRange {
            name,
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period as usize),
            is_ready: false,
            tick_size,
            plot_color,
        };
        atr
    }

    fn calculate_true_range(&self) -> Price {
        let base_data = self.base_data_history.history();
        let mut true_ranges = Vec::new();

        for i in 1..base_data.len() {
            match (&base_data[i - 1], &base_data[i]) {
                (BaseDataEnum::QuoteBar(prev_bar), BaseDataEnum::QuoteBar(curr_bar)) => {
                    let high_low = curr_bar.bid_high - curr_bar.bid_low;
                    let high_close = (curr_bar.bid_high - prev_bar.bid_close).abs();
                    let low_close = (curr_bar.bid_low - prev_bar.bid_close).abs();
                    true_ranges.push(high_low.max(high_close).max(low_close));
                }
                (BaseDataEnum::Candle(prev_candle), BaseDataEnum::Candle(curr_candle)) => {
                    let high_low = curr_candle.high - curr_candle.low;
                    let high_close = (curr_candle.high - prev_candle.close).abs();
                    let low_close = (curr_candle.low - prev_candle.close).abs();
                    true_ranges.push(high_low.max(high_close).max(low_close));
                }
                _ => panic!("Unsupported data type for AverageTrueRange"),
            }
        }

        // Calculate the average of true ranges (ATR)
        let atr = if !true_ranges.is_empty() {
            let sum = true_ranges.iter().sum::<Decimal>();
            if sum == dec!(0.0) {
                return dec!(0.0)
            }
            round_to_tick_size(
                 sum / Decimal::from_usize(true_ranges.len()).unwrap(),
                self.tick_size.clone(),
            )
        } else {
            dec!(0.0)
        };
        atr
    }
}

impl Indicators for AverageTrueRange {
    fn name(&self) -> IndicatorName {
        self.name.clone()
    }

    fn history_to_retain(&self) -> usize {
        self.history.number.clone() as usize
    }

    fn update_base_data(&mut self, base_data: &BaseDataEnum) -> Option<IndicatorValues> {
        if !base_data.is_closed() {
            return None;
        }

        self.base_data_history.add(base_data.clone());
        if self.is_ready == false {
            if !self.base_data_history.is_full() {
                return None;
            } else {
                self.is_ready = true;
            }
        }

        let atr = self.calculate_true_range();
        if atr == dec!(0.0) {
            return None;
        }

        let mut plots = BTreeMap::new();
        let name = "atr".to_string();
        plots.insert(
            name,
            IndicatorPlot::new("atr".to_string(), atr, self.plot_color.clone()),
        );
        let values = IndicatorValues::new(
            self.name.clone(),
            self.subscription.clone(),
            plots,
            base_data.time_closed_utc()
        );

        self.history.add(values.clone());
        Some(values)
    }

    fn subscription(&self) -> DataSubscription {
        self.subscription.clone()
    }

    fn reset(&mut self) {
        self.history.clear();
        self.base_data_history.clear();
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
}
