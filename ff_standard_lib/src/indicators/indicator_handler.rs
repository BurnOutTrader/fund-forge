use std::sync::Arc;
use ahash::AHashMap;
use chrono::{DateTime, Duration, Utc};
use tokio::sync::RwLock;
use crate::indicators::indicator_enum::IndicatorEnum;
use crate::indicators::indicators_trait::{AsyncIndicators, IndicatorName, Indicators};
use crate::indicators::values::{IndicatorValues};
use crate::standardized_types::OwnerId;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::standardized_types::time_slices::TimeSlice;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::apis::vendor::client_requests::ClientSideDataVendor;
use crate::consolidators::consolidator_enum::ConsolidatorEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::history::range_data;
use crate::standardized_types::enums::{Resolution, StrategyMode};
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::strategy_events::StrategyEvent;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
    compare(PartialEq),
    check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum IndicatorEvents {
    IndicatorAdded(IndicatorName),
    IndicatorRemoved(IndicatorName),
    IndicatorTimeSlice(Vec<IndicatorValues>),
    Replaced(IndicatorName),
}

pub enum IndicatorRequest {

}

pub struct IndicatorHandler {
    indicators: Arc<RwLock<AHashMap<DataSubscription, AHashMap<IndicatorName, IndicatorEnum>>>>,
    is_warm_up_complete: RwLock<bool>,
    owner_id: OwnerId,
    strategy_mode: StrategyMode,
    event_buffer: RwLock<Vec<StrategyEvent>>,
    subscription_map: RwLock<AHashMap<IndicatorName, DataSubscription>>, //used to quickly find the subscription of an indicator by name.
    //receiver: Receiver<StrategyEvent>,
    //todo add broadcaster and receiver to decouple handlers. this will allow the handlers to
    //1. Have interior mutability and remove async locks.
    //2. Be seperated and insulated from the engine and the strategy.
    //3. This will allow us to have multiple strategies running in parallel.
    //4. Live strategies can share the same handler.
    //5. You could async send all data... run all handlers at the same time, then async collect all buffers.

    // Downside:
    // 1. Each handler will require its own messaging.

    // How:
    // strategies starting in backtest mode will launch a function which will generate a new handler.
    // strategies in live mode will subscribe to an existing handler and launch there own listener function for receiving requests from the strategy.
    // each handler will have a buffer for events which will be sent to the strategies.
}

impl IndicatorHandler {
    pub fn new(owner_id: OwnerId, strategy_mode: StrategyMode) -> Self {
        Self {
            indicators: Default::default(),
            is_warm_up_complete: RwLock::new(false),
            owner_id,
            strategy_mode,
            event_buffer: Default::default(),
            subscription_map: Default::default(),
        }
    }

    async fn get_event_buffer(&self) -> Vec<StrategyEvent> {
        let mut buffer  = self.event_buffer.write().await;
        let buffer_cached = buffer.clone();
        buffer.clear();
        buffer_cached
    }

    pub async fn set_warmup_complete(&self) {
        *self.is_warm_up_complete.write().await = true;
    }

    pub async fn add_indicator(&self, indicator: IndicatorEnum, time: DateTime<Utc>) {
        let mut indicators = self.indicators.write().await;
        let indicators = indicators.entry(indicator.subscription()).or_insert_with(|| AHashMap::new());
        let name = indicator.name().clone();
        let warm_up_complete = *self.is_warm_up_complete.read().await;
        let indicator = match warm_up_complete {
            true => warmup(time, self.strategy_mode.clone(), indicator).await,
            false => indicator
        };
        self.subscription_map.write().await.insert(name.clone(), indicator.subscription().clone());
        match indicators.insert(name.clone(), indicator) {
            Some(_) => self.event_buffer.write().await.push(StrategyEvent::IndicatorEvent(self.owner_id.clone(), IndicatorEvents::Replaced(name.clone()))),
            None => self.event_buffer.write().await.push(StrategyEvent::IndicatorEvent(self.owner_id.clone(), IndicatorEvents::IndicatorAdded(name.clone())))
        }
    }

    pub async fn remove_indicator(&self, indicator: &IndicatorName) {
        let mut indicators = self.indicators.write().await;
        if let Some(map) = indicators.get_mut(&self.subscription_map.read().await.get(indicator).unwrap()) {
            if map.remove(indicator).is_some() {
                self.event_buffer.write().await.push(StrategyEvent::IndicatorEvent(self.owner_id.clone(), IndicatorEvents::IndicatorRemoved(indicator.clone())));
                self.subscription_map.write().await.remove(indicator);
            }
        }
        self.subscription_map.write().await.remove(indicator);
    }

    pub async fn indicators_unsubscribe(&self, subscription: &DataSubscription) {
        self.indicators.write().await.remove(subscription);
        for (name, sub) in self.subscription_map.read().await.iter() {
            if sub == subscription {
                self.subscription_map.write().await.remove(name);
            }
        }
    }

    pub async fn update_time_slice(&self, time_slice: &TimeSlice) -> Option<Vec<StrategyEvent>> {
        //this could potentially have a race condition if we have 2x the same data subscription in the same time slice. but this would only happen in back-tests using an incorrect strategy resolution or in 
        // fast markets where indicators using tick or price data... in should not generally be possible unless done deliberately. I think it is worth keeping simple concurrent performance gain for the risk.
        let mut tasks = vec![];
        for data in time_slice.clone() {
            let indicators = self.indicators.clone();
            let task = tokio::spawn(async move {
                let subscription = data.subscription();
                let mut indicators = indicators.write().await;
                let mut values = Vec::new();
                if let Some(indicators) = indicators.get_mut(&subscription) {
                    for indicator in indicators.values_mut() {
                        let value = indicator.update_base_data(&data).await;
                        if let Some(value) = value {
                            values.push(value);
                        }
                    }
                }
                values
            });

            tasks.push(task);
        }

        // Await all tasks and collect the results
        let results: Vec<Vec<IndicatorValues>> = futures::future::join_all(tasks).await.into_iter().filter_map(|r| r.ok()).collect();
        let values = results.into_iter().flatten().collect::<Vec<_>>();

        if !values.is_empty() {
            self.event_buffer.write().await.push(StrategyEvent::IndicatorEvent(self.owner_id.clone(), IndicatorEvents::IndicatorTimeSlice(values)));
        }

        let events = self.get_event_buffer().await;
        match events.is_empty() {
            true => None,
            false => Some(events),
        }
    }

    pub async fn history(&self, name: IndicatorName) -> Option<RollingWindow<IndicatorValues>> {
        let indicators = self.indicators.write().await;
        let subscription = match self.subscription_map.read().await.get(&name) {
            Some(sub) => sub.clone(),
            None => return None
        };
        if let Some(map) = indicators.get(&subscription) {
            if let Some(indicator) = map.get(&name) {
                let history = indicator.history();
                return match history.is_empty() {
                    true => None,
                    false => Some(history),
                }
            }
        }
        None
    }

    pub async fn current(&self, name: &IndicatorName) -> Option<IndicatorValues> {
        let indicators = self.indicators.read().await;
        let subscription = match self.subscription_map.read().await.get(name) {
            Some(sub) => sub.clone(),
            None => return None
        };
        if let Some(map) = indicators.get(&subscription) {
            for indicator in map.values() {
                if &indicator.name() == name {
                    return indicator.current();
                }
            }
        }
        None
    }

    pub async fn index(&self, name: &IndicatorName, index: u64) -> Option<IndicatorValues> {
        let indicators = self.indicators.read().await;
        let subscription = match self.subscription_map.read().await.get(name) {
            Some(sub) => sub.clone(),
            None => return None
        };
        if let Some(map) = indicators.get(&subscription) {
            for indicator in map.values() {
                if &indicator.name() == name {
                    return indicator.index(index);
                }
            }
        }
        None
    }
}

async fn warmup(to_time: DateTime<Utc>, strategy_mode: StrategyMode, mut indicator: IndicatorEnum) -> IndicatorEnum {
    let subscription = indicator.subscription();
    let vendor_resolutions = subscription.symbol.data_vendor.resolutions(subscription.market_type.clone()).await.unwrap();
    let mut minimum_resolution: Option<Resolution> = None;
    for resolution in &vendor_resolutions {
        if minimum_resolution.is_none() {
            minimum_resolution = Some(resolution.clone());
        } else {
            if resolution > &minimum_resolution.unwrap() && resolution < &subscription.resolution {
                minimum_resolution = Some(resolution.clone());
            }
        }
    }

    let minimum_resolution = match minimum_resolution.is_none() {
        true => panic!("{} does not have any resolutions available", subscription.symbol.data_vendor),
        false => minimum_resolution.unwrap()
    };

    let data_type = match minimum_resolution {
        Resolution::Ticks(_) => BaseDataType::Ticks,
        _ => subscription.base_data_type.clone()
    };

    let from_time = to_time - (subscription.resolution.as_duration() * indicator.history().number as i32) - Duration::days(4); //we go back a bit further in case of holidays or weekends

    let base_subscription = DataSubscription::new(subscription.symbol.name.clone(), subscription.symbol.data_vendor.clone(), minimum_resolution, data_type, subscription.market_type.clone());
    let base_data = range_data(from_time, to_time, base_subscription.clone()).await;

    match base_subscription == subscription {
        true => {
            for (time, slice) in &base_data {
                if time > &to_time {
                    break;
                }
                for base_data in slice {
                    indicator.update_base_data(base_data).await;
                }
            }
        }
        false => {
            let mut consolidator = ConsolidatorEnum::create_consolidator(true, indicator.subscription().clone(), indicator.history().number, to_time, strategy_mode).await;
            for (time, slice) in &base_data {
                if time > &to_time {
                    break;
                }
                for base_data in slice {
                    let consolidated = consolidator.update(base_data).await;
                    if consolidated.is_empty() {
                        continue
                    }
                    for data in consolidated {
                        indicator.update_base_data(&data).await;
                    }
                }
            }
        }
    }
    if strategy_mode != StrategyMode::Backtest {
        //todo() we will get any bars which are not in out serialized history here
    }
    indicator
}