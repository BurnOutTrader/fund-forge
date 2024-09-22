use chrono::{DateTime, Utc};
use ff_standard_lib::standardized_types::enums::StrategyMode;
use ff_standard_lib::standardized_types::strategy_events::{EventTimeSlice};
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
#[allow(unused_variables)]
pub struct RegistryHandler {}

#[allow(unused_variables)]
impl RegistryHandler {
    pub async fn new() -> Self {
        Self {
        }
    }

    #[allow(unused_variables)]
    pub async fn register_strategy(&self, _mode: StrategyMode,_subscriptionsns: Vec<DataSubscription>) {
        todo!()
    }

    #[allow(unused_variables)]
    pub async fn deregister_strategy(&self, _msg: String, _last_time: DateTime<Utc>) {
        todo!()
    }

    #[allow(unused_variables)]
    pub async fn forward_events(&self, _time: DateTime<Utc>, _strategy_event_slice: EventTimeSlice) {
        todo!()
    }
}