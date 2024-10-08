use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::strategies::consolidators::consolidator_enum::ConsolidatedData;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::enums::SubscriptionResolutionType;
use crate::standardized_types::resolution::Resolution;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::handlers::market_handlers::SYMBOL_INFO;

//renko parameters will have to be strings so we can implement hash etc
//todo, just have different kinds of renko consolidators for the different kinds of renko
/// A consolidator that produces a new piece of data after a certain number of data points have been added.
/// Supports Ticks only.
#[allow(dead_code)]
pub struct RenkoConsolidator {
    current_data: Candle,
    pub(crate) subscription: DataSubscription,
    subscription_resolution_type: SubscriptionResolutionType,
    range: Decimal,
    decimal_accuracy: u32 //todo, might need to use a dual calculation option, for futures use tick size, for fx use decimal accuracy.
}

impl RenkoConsolidator {
    #[allow(dead_code)]
    pub(crate) async fn new(
        subscription: DataSubscription,
        subscription_resolution_type: SubscriptionResolutionType,
        range: Decimal
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


        let decimal_accuracy = if let Some(info) = SYMBOL_INFO.get(&subscription.symbol.name) {
            info.decimal_accuracy
        } else {
            subscription.symbol.data_vendor.decimal_accuracy(subscription.symbol.name.clone()).await.unwrap()
        };

        Ok(RenkoConsolidator {
            subscription_resolution_type,
            current_data,
            subscription,
            decimal_accuracy,
            range
        })
    }

    #[allow(dead_code)]
    /// Returns a candle if the count is reached
    pub(crate) fn update(&mut self, _base_data: &BaseDataEnum) -> ConsolidatedData {
        //let _lock = self.lock.lock().await; //to protect against race conditions where a time slice contains multiple data points of same subscrption
        todo!() //will need to be based on renko parameters
    }
}
