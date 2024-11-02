use chrono::{DateTime, Utc};
use crate::standardized_types::enums::StrategyMode;
use crate::strategies::strategy_events::{StrategyEventBuffer};
use crate::standardized_types::subscriptions::DataSubscription;
#[allow(unused_variables, dead_code)]
pub struct RegistryHandler {}

#[allow(unused_variables, dead_code)]
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
    pub async fn forward_events(&self, _time: DateTime<Utc>, _strategy_event_slice: StrategyEventBuffer) {
        todo!()
    }
}