use crate::apis::vendor::client_requests::ClientSideDataVendor;
use crate::helpers::decimal_calculators::round_to_tick_size;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::enums::Resolution;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;

#[derive(Debug)]
pub struct ConsolidatorError {
    pub(crate) message: String,
}

/// A consolidator that produces a new piece of data after a certain number of data points have been added.
/// Supports Ticks only.
pub struct CountConsolidator {
    number: u64,
    counter: u64,
    current_data: Candle,
    pub(crate) subscription: DataSubscription,
    pub(crate) history: RollingWindow<BaseDataEnum>,
    tick_size: f64, //need to add this
}

impl CountConsolidator {
    pub(crate) async fn new(
        subscription: DataSubscription,
        history_to_retain: u64,
    ) -> Result<Self, ConsolidatorError> {
        let number = match subscription.resolution {
            Resolution::Ticks(num) => num,
            _ => {
                return Err(ConsolidatorError {
                    message: format!(
                        "{:?} is an Invalid resolution for CountConsolidator",
                        subscription.resolution
                    ),
                })
            }
        };

        let current_data = match subscription.base_data_type {
            BaseDataType::Ticks => Candle::new(
                subscription.symbol.clone(),
                0.0,
                0.0,
                "".to_string(),
                Resolution::Ticks(number),
                subscription.candle_type.clone().unwrap(),
            ),
            _ => {
                return Err(ConsolidatorError {
                    message: format!(
                        "{} is an Invalid base data type for CountConsolidator",
                        subscription.base_data_type
                    ),
                })
            }
        };

        let tick_size = match subscription
            .symbol
            .data_vendor
            .tick_size(subscription.symbol.clone())
            .await
        {
            Ok(size) => size,
            Err(e) => {
                return Err(ConsolidatorError {
                    message: format!("Error getting tick size: {}", e),
                })
            }
        };

        Ok(CountConsolidator {
            number,
            counter: 0,
            current_data,
            subscription,
            history: RollingWindow::new(history_to_retain),
            tick_size,
        })
    }

    /// Returns a candle if the count is reached
    pub(crate) fn update(&mut self, base_data: &BaseDataEnum) -> Vec<BaseDataEnum> {
        match base_data {
            BaseDataEnum::Tick(tick) => {
                let mut candles = vec![];
                if self.counter == 0 {
                    self.current_data.symbol = base_data.symbol().clone();
                    self.current_data.time = tick.time.clone();
                    self.current_data.open = tick.price;
                    self.current_data.volume = tick.volume;
                    self.current_data.high = tick.price;
                    self.current_data.low = tick.price;
                }
                self.counter += 1;
                self.current_data.high = self.current_data.high.max(tick.price);
                self.current_data.low = self.current_data.low.min(tick.price);
                self.current_data.range = round_to_tick_size(
                    self.current_data.high - self.current_data.low,
                    self.tick_size,
                );
                self.current_data.close = tick.price;
                self.current_data.volume += tick.volume;
                if self.counter == self.number {
                    let mut consolidated_candle = self.current_data.clone();
                    consolidated_candle.is_closed = true;
                    self.counter = 0;
                    let consolidated_data = BaseDataEnum::Candle(consolidated_candle.clone());
                    self.history.add(consolidated_data.clone());
                    candles.push(consolidated_data);
                    self.current_data = match self.subscription.base_data_type {
                        BaseDataType::Ticks => Candle::new(
                            self.subscription.symbol.clone(),
                            0.0,
                            0.0,
                            "".to_string(),
                            Resolution::Ticks(self.number),
                            self.subscription.candle_type.clone().unwrap(),
                        ),
                        _ => panic!(
                            "Invalid base data type for CountConsolidator: {}",
                            self.subscription.base_data_type
                        ),
                    };
                } else {
                    candles.push(BaseDataEnum::Candle(self.current_data.clone()));
                }
                candles
            }
            _ => panic!(
                "Invalid base data type for CountConsolidator: {}",
                base_data.base_data_type()
            ),
        }
    }

    pub(crate) fn history(&self) -> RollingWindow<BaseDataEnum> {
        self.history.clone()
    }

    pub(crate) fn index(&self, index: u64) -> Option<BaseDataEnum> {
        match self.history.get(index) {
            Some(data) => Some(data.clone()),
            None => None,
        }
    }

    pub(crate) fn current(&self) -> Option<BaseDataEnum> {
        Some(BaseDataEnum::Candle(self.current_data.clone()))
    }
}
