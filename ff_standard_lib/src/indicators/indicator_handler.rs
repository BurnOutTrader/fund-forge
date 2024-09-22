use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::consolidators::consolidator_enum::ConsolidatorEnum;
use crate::indicators::indicator_enum::IndicatorEnum;
use crate::indicators::indicators_trait::{IndicatorName, Indicators};
use crate::indicators::values::IndicatorValues;
use crate::standardized_types::base_data::history::range_data;
use crate::standardized_types::enums::StrategyMode;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::strategy_events::StrategyEvent;
use crate::standardized_types::subscriptions::{filter_resolutions, DataSubscription};
use crate::standardized_types::time_slices::TimeSlice;
use chrono::{DateTime, Duration, Utc};
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use dashmap::DashMap;
use tokio::sync::{RwLock};
use crate::standardized_types::base_data::traits::BaseData;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum IndicatorEvents {
    IndicatorAdded(IndicatorName),
    IndicatorRemoved(IndicatorName),
    IndicatorTimeSlice(Vec<IndicatorValues>),
    Replaced(IndicatorName),
}

pub enum IndicatorRequest {}

pub struct IndicatorHandler {
    indicators: Arc<DashMap<DataSubscription, DashMap<IndicatorName, IndicatorEnum>>>,
    is_warm_up_complete: AtomicBool,
    strategy_mode: StrategyMode,
    event_buffer: RwLock<Vec<StrategyEvent>>,
    subscription_map: DashMap<IndicatorName, DataSubscription>, //used to quickly find the subscription of an indicator by name.
}

impl IndicatorHandler {
    pub async fn new(strategy_mode: StrategyMode) -> Self {
        let handler =Self {
            indicators: Default::default(),
            is_warm_up_complete: AtomicBool::new(false),
            strategy_mode,
            event_buffer: Default::default(),
            subscription_map: Default::default(),
        };
        handler
    }

    pub async fn get_event_buffer(&self) -> Option<Vec<StrategyEvent>> {
        let mut buffer = self.event_buffer.write().await;
        let buffer_cached = buffer.clone();
        buffer.clear();

        match buffer.is_empty() {
            true => None,
            false => Some(buffer_cached)
        }
    }

    pub async fn set_warmup_complete(&self) {
        self.is_warm_up_complete.store(true, Ordering::SeqCst);
    }

    pub async fn add_indicator(&self, indicator: IndicatorEnum, time: DateTime<Utc>) {
        let subscription = indicator.subscription();

        if !self.indicators.contains_key(&subscription) {
            self.indicators.insert(subscription.clone(), DashMap::new());
        }

        let name = indicator.name().clone();

        let indicator = match self.is_warm_up_complete.load(Ordering::SeqCst) {
            true => warmup(time, self.strategy_mode.clone(), indicator).await,
            false => indicator,
        };

        if !self.subscription_map.contains_key(&name) {
            self
                .event_buffer
                .write()
                .await
                .push(StrategyEvent::IndicatorEvent(
                IndicatorEvents::IndicatorAdded(name.clone())));
        } else {
            self
                .event_buffer
                .write()
                .await
                .push(StrategyEvent::IndicatorEvent(
                IndicatorEvents::Replaced(name.clone())));
        }
        if let Some(map) = self.indicators.get(&subscription) {
            map.insert(indicator.name(), indicator);
        }
        self.subscription_map.insert(name.clone(), subscription.clone());
    }

    pub async fn remove_indicator(&self, indicator: &IndicatorName) {
        if let Some(map) =
            self.indicators.get_mut(&self.subscription_map.get(indicator).unwrap())
        {
            if map.remove(indicator).is_some() {
                self.event_buffer
                    .write()
                    .await
                    .push(StrategyEvent::IndicatorEvent(
                        IndicatorEvents::IndicatorRemoved(indicator.clone()),
                    ));
                self.subscription_map.remove(indicator);
            }
        }
        self.subscription_map.remove(indicator);
    }

    pub async fn indicators_unsubscribe_subscription(&self, subscription: &DataSubscription) {
        self.indicators.remove(subscription);
        for sub in self.subscription_map.iter() {
            if sub.value() == subscription {
                self.subscription_map.remove(sub.key());
            }
        }
    }

    pub async fn update_time_slice(&self, time_slice: &TimeSlice) -> Option<Vec<StrategyEvent>>   {
        let mut results: BTreeMap<IndicatorName, IndicatorValues> = BTreeMap::new();
        let indicators = self.indicators.clone();

        for data in time_slice {
            let subscription = data.subscription();
            if let Some(indicators_by_sub) = indicators.get_mut(&subscription) {
                for mut indicators_dash_map in indicators_by_sub.iter_mut() {
                    let data = indicators_dash_map.value_mut().update_base_data(data);
                    if let Some(indicator_data) = data {
                        results.insert(indicators_dash_map.key().clone(), indicator_data);
                    }
                }
            }
        }

        let mut event_buffer: Vec<StrategyEvent>= vec![];

        let results_vec: Vec<IndicatorValues> = results.values().cloned().collect();
        if !results_vec.is_empty() {
            event_buffer.push(StrategyEvent::IndicatorEvent(IndicatorEvents::IndicatorTimeSlice(results_vec)));
        }

        let buffered = self.event_buffer.read().await.clone();
        if !buffered.is_empty() {
            event_buffer.extend(buffered);
            self.event_buffer.write().await.clear();
        }

        match event_buffer.is_empty() {
            true => None,
            false => Some(event_buffer)
        }
    }

    pub async fn history(&self, name: IndicatorName) -> Option<RollingWindow<IndicatorValues>> {
        let subscription = match self.subscription_map.get(&name) {
            Some(sub) => sub.clone(),
            None => return None,
        };
        if let Some(map) = self.indicators.get(&subscription) {
            if let Some(indicator) = map.get(&name) {
                let history = indicator.history();
                return match history.is_empty() {
                    true => None,
                    false => Some(history),
                };
            }
        }
        None
    }

    pub async fn current(&self, name: &IndicatorName) -> Option<IndicatorValues> {
        let subscription = match self.subscription_map.get(name) {
            Some(sub) => sub.clone(),
            None => return None,
        };
        if let Some(map) = self.indicators.get(&subscription) {
            for indicator in map.value() {
                if &indicator.name() == name {
                    return indicator.current();
                }
            }
        }
        None
    }

    pub async fn index(&self, name: &IndicatorName, index: usize) -> Option<IndicatorValues> {
        let subscription = match self.subscription_map.get(name) {
            Some(sub) => sub.clone(),
            None => return None,
        };
        if let Some(map) = self.indicators.get(&subscription) {
            for indicator in map.value() {
                if &indicator.name() == name {
                    return indicator.index(index);
                }
            }
        }
        None
    }
}

async fn warmup(
    to_time: DateTime<Utc>,
    strategy_mode: StrategyMode,
    mut indicator: IndicatorEnum,
) -> IndicatorEnum {
    let subscription = indicator.subscription();
    let vendor_resolutions = filter_resolutions(
        subscription
            .symbol
            .data_vendor
            .resolutions(subscription.market_type.clone())
            .await
            .unwrap(),
        subscription.resolution.clone(),
    );
    let max_resolution = vendor_resolutions.iter().max_by_key(|r| r.resolution);
    let min_resolution = match max_resolution.is_none() {
        true => panic!(
            "{} does not have any resolutions available",
            subscription.symbol.data_vendor
        ),
        false => max_resolution.unwrap(),
    };

    let from_time = to_time
        - (subscription.resolution.as_duration() * indicator.history().number as i32)
        - Duration::days(4); //we go back a bit further in case of holidays or weekends

    let base_subscription = DataSubscription::new(
        subscription.symbol.name.clone(),
        subscription.symbol.data_vendor.clone(),
        min_resolution.resolution,
        min_resolution.base_data_type,
        subscription.market_type.clone(),
    );
    let base_data = range_data(from_time, to_time, base_subscription.clone()).await;

    match base_subscription == subscription {
        true => {
            for (time, slice) in base_data {
                if time > to_time {
                    break;
                }
                for base_data in slice {
                    indicator.update_base_data(&base_data);
                }
            }
        }
        false => {
            let consolidator = ConsolidatorEnum::create_consolidator(
                true,
                indicator.subscription().clone(),
                (indicator.history().number * 2) as u64,
                to_time,
                strategy_mode,
            )
            .await;
            for data in consolidator.history().history {
                indicator.update_base_data(&data);
            }
        }
    }
    if strategy_mode != StrategyMode::Backtest {
        //todo() we will get any bars which are not in out serialized history here
    }
    indicator
}
