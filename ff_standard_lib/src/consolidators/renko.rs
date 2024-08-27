use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use crate::apis::vendor::client_requests::ClientSideDataVendor;
use crate::consolidators::count::ConsolidatorError;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::enums::{Resolution};
use crate::standardized_types::subscriptions::{CandleType, DataSubscription};
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};



/// A consolidator that produces a new piece of data after a certain number of data points have been added.
/// Supports Ticks only.
pub struct RenkoConsolidator {
    current_data: Candle,
    pub(crate) subscription: DataSubscription,
    pub(crate) history: RollingWindow<BaseDataEnum>,
    tick_size: f64,
}

impl RenkoConsolidator
{
    pub(crate) async fn new(subscription: DataSubscription, history_to_retain: u64) -> Result<Self, ConsolidatorError> {
        let current_data = match &subscription.base_data_type {
            BaseDataType::Ticks => Candle::new(subscription.symbol.clone(), 0.0, 0.0, "".to_string(), Resolution::Instant, subscription.candle_type.clone().unwrap()),
            _ => return Err(ConsolidatorError { message: format!("{} is an Invalid base data type for CountConsolidator", subscription.base_data_type) }),
        };
        

        let tick_size = subscription.symbol.data_vendor.tick_size(subscription.symbol.clone()).await.unwrap();
        
        Ok(RenkoConsolidator {
            current_data,
            subscription,
            history: RollingWindow::new(history_to_retain),
            tick_size: tick_size.try_into().unwrap(),
        })
    }

    /// Returns a candle if the count is reached
    pub(crate) fn update(&mut self, base_data: &BaseDataEnum) -> Vec<BaseDataEnum> {
        //let _lock = self.lock.lock().await; //to protect against race conditions where a time slice contains multiple data points of same subscrption
        todo!() //will need to be based on renko parameters
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