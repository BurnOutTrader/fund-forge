use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::strategies::consolidators::consolidator_enum::ConsolidatedData;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::enums::{MarketType};
use crate::standardized_types::base_data::traits::BaseData;
use crate::messages::data_server_messaging::FundForgeError;
use crate::standardized_types::resolution::Resolution;
use crate::standardized_types::subscriptions::DataSubscription;

//Todo Replace all quantity and volume with Volume aka Decimal, same for price.
/// A consolidator that produces a new piece of data after a certain number of data points have been added.
/// Supports Ticks only.
pub struct CountConsolidator {
    number: u64,
    counter: u64,
    current_data: Candle,
    pub(crate) subscription: DataSubscription,
    decimal_accuracy: u32,
    tick_size: Decimal,
    market_type: MarketType,
}

impl CountConsolidator {
    pub(crate) async fn new(
        subscription: DataSubscription,
    ) -> Result<Self, FundForgeError> {
        let number = match subscription.resolution {
            Resolution::Ticks(num) => num,
            _ => {
                return Err(FundForgeError::ClientSideErrorDebug(format!("{:?} is an Invalid resolution for CountConsolidator", subscription.resolution)))
            }
        };

        let current_data = match subscription.base_data_type {
            BaseDataType::Ticks => Candle::new(
                subscription.symbol.clone(),
                dec!(0.0),
                dec!(0.0),
                dec!(0.0),
                dec!(0.0),
                "".to_string(),
                Resolution::Ticks(number),
                subscription.candle_type.clone().unwrap(),
            ),
            _ => {
                return Err(FundForgeError::ClientSideErrorDebug(format!("{} is an Invalid base data type for CountConsolidator", subscription.base_data_type)))
            }
        };

        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(subscription.symbol.name.clone()).await?;
        let tick_size = subscription.symbol.data_vendor.tick_size(subscription.symbol.name.clone()).await?;

        let market_type = subscription.symbol.market_type.clone();

        Ok(CountConsolidator {
            number,
            counter: 0,
            market_type,
            current_data,
            subscription,
            decimal_accuracy,
            tick_size,
        })
    }

    /// Returns a candle if the count is reached
    pub(crate) fn update(&mut self, base_data: &BaseDataEnum) -> ConsolidatedData {
        match base_data {
            BaseDataEnum::Tick(tick) => {
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
                self.current_data.range = self.market_type.round_price(self.current_data.high - self.current_data.low, self.tick_size, self.decimal_accuracy);
                self.current_data.close = tick.price;
                self.current_data.volume += tick.volume;
                if self.counter == self.number {
                    let mut consolidated_candle = self.current_data.clone();
                    consolidated_candle.is_closed = true;
                    self.counter = 0;
                    self.current_data = match self.subscription.base_data_type {
                        BaseDataType::Ticks => Candle::new(
                            self.subscription.symbol.clone(),
                            dec!(0.0),
                            dec!(0.0),
                            dec!(0.0),
                            dec!(0.0),
                            base_data.time_utc().to_string(),
                            Resolution::Ticks(self.number),
                            self.subscription.candle_type.clone().unwrap(),
                        ),
                        _ => panic!(
                            "Invalid base data type for CountConsolidator: {}",
                            self.subscription.base_data_type
                        ),
                    };
                    ConsolidatedData::with_closed(BaseDataEnum::Candle(self.current_data.clone()), BaseDataEnum::Candle(consolidated_candle))
                } else {
                    ConsolidatedData::with_open(BaseDataEnum::Candle(self.current_data.clone()))
                }
            }
            _ => panic!(
                "Invalid base data type for CountConsolidator: {}",
                base_data.base_data_type()
            ),
        }
    }
}
