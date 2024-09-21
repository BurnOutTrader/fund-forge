use chrono::{DateTime, Utc};
use ff_standard_lib::standardized_types::enums::StrategyMode;
use ff_standard_lib::standardized_types::strategy_events::StrategyEvent;
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;

pub struct RegistryHandler {}

impl RegistryHandler {
    pub async fn new() -> Self {
        Self {
        }
    }

    pub async fn register_strategy(&self, mode: StrategyMode, subscriptions: Vec<DataSubscription>) {
        todo!()
    }

    pub async fn deregister_strategy(&self, msg: String, last_time: DateTime<Utc>) {
        todo!()
    }

    pub async fn forward_events(&self, time: DateTime<Utc>, strategy_event_slice: Vec<StrategyEvent>) {
        todo!()
    }
}