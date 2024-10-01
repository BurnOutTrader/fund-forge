use rust_decimal_macros::dec;
use crate::consolidators::consolidator_enum::ConsolidatedData;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::enums::{Resolution, SubscriptionResolutionType};
use crate::standardized_types::subscriptions::DataSubscription;

//renko parameters will have to be strings so we can implement hash etc
//todo, just have different kinds of renko consolidators for the different kinds of renko
/// A consolidator that produces a new piece of data after a certain number of data points have been added.
/// Supports Ticks only.
#[allow(dead_code)]
pub struct RenkoConsolidator {
    current_data: Candle,
    pub(crate) subscription: DataSubscription,
    tick_size: f64,
    subscription_resolution_type: SubscriptionResolutionType
}

impl RenkoConsolidator {
    pub(crate) async fn new(
        subscription: DataSubscription,
        subscription_resolution_type: SubscriptionResolutionType
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

        let tick_size = subscription
            .symbol
            .tick_size()
            .await
            .unwrap();

        Ok(RenkoConsolidator {
            subscription_resolution_type,
            current_data,
            subscription,
            tick_size: tick_size.try_into().unwrap(),
        })
    }

    /// Returns a candle if the count is reached
    pub(crate) fn update(&mut self, _base_data: &BaseDataEnum) -> ConsolidatedData {
        //let _lock = self.lock.lock().await; //to protect against race conditions where a time slice contains multiple data points of same subscrption
        todo!() //will need to be based on renko parameters
    }
}
