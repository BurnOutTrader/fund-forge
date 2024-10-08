use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::strategies::consolidators::consolidator_enum::ConsolidatedData;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::enums::SubscriptionResolutionType;
use crate::standardized_types::resolution::Resolution;
use crate::standardized_types::subscriptions::{DataSubscription};

//renko parameters will have to be strings so we can implement hash etc
//todo, just have different kinds of renko consolidators for the different kinds of renko
/// It is better to use RenkoType::PriceMovementValue to pass in a fixed value for atr size at run time.
/// You  can still pass in the atr range using the PriceMovementValue, if you use the ATR function, the Consolidator will need to first warm up the ATR indicator to get the ATR value.
/// If you have an ATR indicator already warmed up, it will be faster to use this instead.
#[allow(dead_code)]
pub struct Renko {
    current_data: Candle,
    pub(crate) subscription: DataSubscription,
    subscription_resolution_type: SubscriptionResolutionType,
    renko_range: Decimal,
    decimal_accuracy: u32,
    tick_size: Decimal
}

impl Renko {
    #[allow(dead_code)]
    pub(crate) async fn new(
        subscription: DataSubscription,
        subscription_resolution_type: SubscriptionResolutionType,
        renko_range: Option<Decimal>,
    ) -> Result<Self, String> {
        let current_data = match &subscription.base_data_type {
            BaseDataType::Ticks => Candle::new(
                subscription.symbol.clone(),
                dec!(0.0),
                dec!(0.0),
                dec!(0.0),
                dec!(0.0),
                "".to_string(),
                Resolution::Instant,
                subscription.candle_type.clone().unwrap(),
            ),
            _ => {
                return Err( format!(
                        "{} is an Invalid base data type for CountConsolidator",
                        subscription.base_data_type
                    ),
                )
            }
        };


        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(subscription.symbol.name.clone()).await.unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(subscription.symbol.name.clone()).await.unwrap();

        Ok(Renko {
            subscription_resolution_type,
            current_data,
            subscription,
            decimal_accuracy,
            renko_range: renko_range.unwrap(),
            tick_size,
        })
    }

    #[allow(dead_code)]
    /// Returns a candle if the count is reached
    pub(crate) fn update(&mut self, _base_data: &BaseDataEnum) -> ConsolidatedData {
        //let _lock = self.lock.lock().await; //to protect against race conditions where a time slice contains multiple data points of same subscrption
        todo!() //will need to be based on renko parameters
    }
}
