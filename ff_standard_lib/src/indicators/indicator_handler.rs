use ahash::AHashMap;
use chrono::{DateTime, Duration, Utc};
use tokio::sync::RwLock;
use crate::indicators::indicator_enum::IndicatorEnum;
use crate::indicators::indicators_trait::{IndicatorName, Indicators};
use crate::indicators::values::{IndicatorValues};
use crate::standardized_types::OwnerId;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::standardized_types::time_slices::TimeSlice;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::apis::vendor::client_requests::ClientSideDataVendor;
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

pub struct IndicatorHandler {
    indicators: RwLock<AHashMap<DataSubscription, AHashMap<IndicatorName, IndicatorEnum>>>,
    is_warm_up_complete: RwLock<bool>,
    owner_id: OwnerId,
    strategy_mode: StrategyMode,
    event_buffer: RwLock<Vec<StrategyEvent>>,
}

impl IndicatorHandler {
    pub fn new(owner_id: OwnerId, strategy_mode: StrategyMode) -> Self {
        Self {
            indicators: Default::default(),
            is_warm_up_complete: RwLock::new(false),
            owner_id,
            strategy_mode,
            event_buffer: Default::default(),
        }
    }
    
    pub async fn get_event_buffer(&self) -> Vec<StrategyEvent> {
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
        
        match indicators.insert(name.clone(), indicator) {
            Some(indicator) => self.event_buffer.write().await.push(StrategyEvent::IndicatorEvent(self.owner_id.clone(), IndicatorEvents::Replaced(indicator.name()))),
            None => self.event_buffer.write().await.push(StrategyEvent::IndicatorEvent(self.owner_id.clone(), IndicatorEvents::IndicatorAdded(name.clone())))
        }
    }

    pub async fn remove_indicator(&self, indicator: &IndicatorName) {
        let mut indicators = self.indicators.write().await;
        for (_, map) in indicators.iter_mut() {
            if map.remove(indicator).is_some() {
                self.event_buffer.write().await.push(StrategyEvent::IndicatorEvent(self.owner_id.clone(), IndicatorEvents::IndicatorRemoved(indicator.clone())));
                return
            }
        }
    }

    pub async fn indicators_unsubscribe(&self, subscription: &DataSubscription) {
        self.indicators.write().await.remove(subscription);
    }


    pub async fn update_time_slice(&self, time_slice: &TimeSlice) {
        let mut indicators = self.indicators.write().await;
        let mut values = Vec::new();
        for data in time_slice {
            let subscription = data.subscription();
            if let Some(indicators) = indicators.get_mut(&subscription) {
                for indicator in indicators.values_mut() {
                    if indicator.subscription() != subscription {
                        continue
                    }
                    let value = indicator.update_base_data(data);
                    if let Some(value) = value {
                        values.push(value);
                    }
                }
            }
        }
        if !values.is_empty() {
            self.event_buffer.write().await.push(StrategyEvent::IndicatorEvent(self.owner_id.clone(), IndicatorEvents::IndicatorTimeSlice(values)));
        }
    }
    
    pub async fn history(&self, name: &IndicatorName) -> Option<RollingWindow<IndicatorValues>> {
        let indicators = self.indicators.read().await;
        for (_, map) in indicators.iter() {
            for indicator in map.values() {
                if &indicator.name() == name {
                    let history = indicator.history();
                    return match history.is_empty() {
                        true => None,
                        false => Some(history),
                    }
                }
            }
        }
        None
    }
    
    pub async fn current(&self, name: &IndicatorName) -> Option<IndicatorValues> {
        let indicators = self.indicators.read().await;
        for (_, map) in indicators.iter() {
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
        for (_, map) in indicators.iter() {
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
    for resolution in vendor_resolutions {
        if minimum_resolution.is_none() {
            minimum_resolution = Some(resolution);
        } else {
            if resolution > minimum_resolution.unwrap() && resolution < subscription.resolution {
                minimum_resolution = Some(resolution);
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

    for (time, slice) in &base_data {
        if time > &to_time {
            break;
        }
        for base_data in slice {
            indicator.update_base_data(base_data);
        }
    }
    if strategy_mode != StrategyMode::Backtest {
        //todo() we will get any bars which are not in out serialized history here
    }
    indicator
}