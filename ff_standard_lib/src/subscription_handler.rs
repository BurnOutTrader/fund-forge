use std::sync::Arc;
use chrono::{DateTime, Duration, Timelike, Utc};
use tokio::sync::{Mutex, RwLock};
use crate::apis::vendor::client_requests::ClientSideDataVendor;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::base_data::quotebar::QuoteBar;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::data_server_messaging::FundForgeError;
use crate::standardized_types::enums::{Resolution, StrategyMode};
use crate::standardized_types::subscriptions::{DataSubscription, DataSubscriptionEvent, Symbol};
use crate::standardized_types::time_slices::TimeSlice;
use ahash::AHashMap;
use futures::future::join_all;
use crate::rolling_window::RollingWindow;
use crate::standardized_types::base_data::history::range_data;

/// Manages all subscriptions for a strategy. each strategy has its own subscription handler.
pub struct SubscriptionHandler {
    /// Manages the subscriptions of specific symbols
    symbol_subscriptions: Arc<RwLock<AHashMap<Symbol, SymbolSubscriptionHandler>>>,
    fundamental_subscriptions: RwLock<Vec<DataSubscription>>,
    /// Keeps a record when the strategy has updated its subscriptions, so we can pause the backtest to fetch new data.
    subscriptions_updated: RwLock<bool>,
    is_warmed_up: Mutex<bool>,
    strategy_mode: StrategyMode,
}

impl SubscriptionHandler {
    pub async fn new(strategy_mode: StrategyMode) -> Self {
        SubscriptionHandler {
            fundamental_subscriptions: RwLock::new(vec![]),
            symbol_subscriptions: Arc::new(RwLock::new(Default::default())),
            subscriptions_updated: RwLock::new(true),
            is_warmed_up: Mutex::new(false),
            strategy_mode,
        }
    }

    /// Sets the SubscriptionHandler as warmed up, so we can start processing data.
    /// This lets the handler know that it needs to manually warm up any future subscriptions.
    pub async fn set_warmup_complete(&self) {
        *self.is_warmed_up.lock().await = true;
        for symbol_handler in self.symbol_subscriptions.write().await.values_mut() {
            symbol_handler.set_warmed_up();
        }
    }

    /// Returns all the subscription events that have occurred since the last time this method was called.
    pub async fn subscription_events(&self) -> Vec<DataSubscriptionEvent> {
        let mut subscription_events = vec![];
        for symbol_handler in self.symbol_subscriptions.write().await.values_mut() {
            subscription_events.extend(symbol_handler.get_subscription_event_buffer());
        }
        subscription_events
    }

    
    /// Subscribes to a new data subscription
    /// 'new_subscription: DataSubscription' The new subscription to subscribe to.
    /// 'history_to_retain: usize' The number of bars to retain in the history.
    /// 'current_time: DateTime<Utc>' The current time is used to warm up consolidator history if we have already done our initial strategy warm up.
    /// 'strategy_mode: StrategyMode' The strategy mode is used to determine how to warm up the history, in live mode we may not yet have a serialized history to the current time.
    pub async fn subscribe(&self, new_subscription: DataSubscription, history_to_retain: usize, current_time: DateTime<Utc>) -> Result<(), FundForgeError> {
        if new_subscription.base_data_type == BaseDataType::Fundamentals {
            //subscribe to fundamental
            let mut fundamental_subscriptions = self.fundamental_subscriptions.write().await;
            if !fundamental_subscriptions.contains(&new_subscription) {
                fundamental_subscriptions.push(new_subscription.clone());
            }
            *self.subscriptions_updated.write().await = true;
            return Ok(())
        }
        let mut symbol_subscriptions = self.symbol_subscriptions.write().await;
        if !symbol_subscriptions.contains_key(&new_subscription.symbol) {
            let symbol_handler = SymbolSubscriptionHandler::new(new_subscription.clone(), self.is_warmed_up.lock().await.clone(), history_to_retain, current_time, self.strategy_mode).await;
            symbol_subscriptions.insert(new_subscription.symbol.clone(), symbol_handler);
        }
        let symbol_handler = symbol_subscriptions.get_mut(&new_subscription.symbol).unwrap();
        symbol_handler.subscribe(new_subscription, history_to_retain, current_time, self.strategy_mode).await;

        *self.subscriptions_updated.write().await = true;
        Ok(())
    }

    /// Unsubscribes from a data subscription
    /// 'subscription: DataSubscription' The subscription to unsubscribe from.
    /// 'current_time: DateTime<Utc>' The current time is used to change our base data subscription and warm up any new consolidators if we are adjusting our base resolution.
    /// 'strategy_mode: StrategyMode' The strategy mode is used to determine how to warm up the history, in live mode we may not yet have a serialized history to the current time.
    pub async fn unsubscribe(&self, subscription: DataSubscription) -> Result<(), FundForgeError>  {
        if subscription.base_data_type == BaseDataType::Fundamentals {
            let mut fundamental_subscriptions = self.fundamental_subscriptions.write().await;
            if fundamental_subscriptions.contains(&subscription) {
                fundamental_subscriptions.retain(|fundamental_subscription| {
                    *fundamental_subscription != subscription
                });
            }
            *self.subscriptions_updated.write().await = true;
            return Ok(())
        }
        let mut handler = self.symbol_subscriptions.write().await;
        let symbol_handler = handler.get_mut(&subscription.symbol).unwrap();
        symbol_handler.unsubscribe(&subscription).await;
        if symbol_handler.active_count == 0 {
            handler.remove(&subscription.symbol);
        }
        *self.subscriptions_updated.write().await = true;
        Ok(())
    }

    pub async fn subscriptions_updated(&self) -> bool {
        self.subscriptions_updated.read().await.clone()
    }
    
    pub async fn set_subscriptions_updated(&self, updated: bool) {
        *self.subscriptions_updated.write().await = updated;
    }

    /// Returns all the primary subscriptions
    /// These are subscriptions that come directly from the vendors own data source.
    /// They are not consolidators, but are the primary source of data for the consolidators.
    pub async fn primary_subscriptions(&self) -> Vec<DataSubscription> {
        let mut primary_subscriptions = vec![];
        for symbol_handler in self.symbol_subscriptions.read().await.values() {
            primary_subscriptions.push(symbol_handler.primary_subscription().await);
        }
        primary_subscriptions
    }

    /// Returns all the subscriptions including primary and consolidators
    pub async fn subscriptions(&self) -> Vec<DataSubscription> {
        let mut all_subscriptions = vec![];
        for symbol_handler in self.symbol_subscriptions.read().await.values() {
            all_subscriptions.append(&mut symbol_handler.all_subscriptions().await);
        }
        for subscription in self.fundamental_subscriptions.read().await.iter() {
            all_subscriptions.push(subscription.clone());
        }
        all_subscriptions
    }

    /// Updates any consolidators with primary data
    pub async fn update_consolidators(&self, time_slice: TimeSlice) -> TimeSlice {
        let mut tasks = vec![];

        for base_data in time_slice {
            let symbol_subscriptions = self.symbol_subscriptions.clone();
            let task = tokio::spawn(async move {
                let base_data = base_data.clone();
                let symbol = base_data.symbol();
                let mut symbol_subscriptions = symbol_subscriptions.write().await;
                if let Some(symbol_handler) =  symbol_subscriptions.get_mut(&symbol) {
                    symbol_handler.update(&base_data).await //todo we need to use interior mutability to update the consolidators across threads, add RWLock or mutex
                } else {
                    vec![]
                }
            });

            tasks.push(task);
        }

        // Await all tasks and collect the results
        let results: Vec<Vec<BaseDataEnum>> = join_all(tasks).await.into_iter().filter_map(|r| r.ok()).collect();

        // Flatten the results
        results.into_iter().flatten().collect()
    }
}


/// This Struct Handles when to consolidate data for a subscription from an existing subscription.
/// Alternatively if a subscription is of a lower resolution subscription, then the new subscription becomes the primary data source and the existing subscription becomes the secondary data source.
/// depending if the vendor has data available in that resolution.
pub struct SymbolSubscriptionHandler {
    /// The primary subscription is the subscription where data is coming directly from the `DataVendor`, In the event of bar data, it is pre-consolidated.
    primary_subscription: DataSubscription,
    /// The secondary subscriptions are consolidators that are used to consolidate data from the primary subscription.
    secondary_subscriptions: Vec<ConsolidatorEnum>,
    /// Count the subscriptions so we can delete the object if it is no longer being used
    active_count: i32,
    symbol: Symbol,
    subscription_event_buffer: Vec<DataSubscriptionEvent>,
    is_warmed_up: bool,
}

impl SymbolSubscriptionHandler {
    pub async fn new(primary_subscription: DataSubscription, is_warmed_up: bool, history_to_retain: usize, warm_up_to: DateTime<Utc>, strategy_mode: StrategyMode) -> Self {
        let mut handler = SymbolSubscriptionHandler {
            primary_subscription: primary_subscription.clone(),
            secondary_subscriptions: vec![],
            active_count: 1,
            symbol: primary_subscription.symbol.clone(),
            subscription_event_buffer: vec![],
            is_warmed_up
        };
        handler.select_primary_subscription(primary_subscription, history_to_retain, warm_up_to, strategy_mode).await;
        handler
    }

    pub async fn update(&mut self, base_data: &BaseDataEnum) -> Vec<BaseDataEnum> {
        // Ensure we only process if the symbol matches
        if &self.symbol != base_data.symbol() {
            panic!("Symbol mismatch: {:?} != {:?}", self.symbol, base_data.symbol());
        }

        let mut consolidated_data = vec![];

        // Read the secondary subscriptions

        if self.secondary_subscriptions.is_empty() {
            return vec![]
        }

        // Iterate over the secondary subscriptions and update them
        for consolidator in &mut self.secondary_subscriptions {
            let data = consolidator.update(base_data);
            consolidated_data.extend(data);
        }
        consolidated_data
    }

    pub fn set_warmed_up(&mut self) {
        self.is_warmed_up = true;
    }

    pub async fn clear_current_data(&mut self) {
        for consolidator in &mut self.secondary_subscriptions.iter_mut() {
            match consolidator {
                ConsolidatorEnum::Count(count_consolidator) => {
                    count_consolidator.clear_current_data();
                },
                ConsolidatorEnum::Time(time_consolidator) => {
                    time_consolidator.clear_current_data();
                },
            }
        }
    }
    
    pub fn get_subscription_event_buffer(&mut self) -> Vec<DataSubscriptionEvent> {
        let buffer = self.subscription_event_buffer.clone();
        self.subscription_event_buffer.clear();
        buffer
    }

    /// This is only used
    async fn select_primary_subscription(&mut self, new_subscription: DataSubscription, history_to_retain: usize, to_time: DateTime<Utc>, strategy_mode: StrategyMode) {
        let available_resolutions: Vec<Resolution> = new_subscription.symbol.data_vendor.resolutions(new_subscription.market_type.clone()).await.unwrap();
        //println!("Available Resolutions: {:?}", available_resolutions);
        if available_resolutions.is_empty() {
            panic!("{} does not have any resolutions available", new_subscription.symbol.data_vendor);
        }
        let resolutions = self.resolutions(available_resolutions, new_subscription.resolution.clone());
        if resolutions.is_empty() {
            panic!("{} does not have any resolutions available for {:?}", new_subscription.symbol.data_vendor, new_subscription);
        }
        if !resolutions.contains(&new_subscription.resolution) {
            match self.is_warmed_up {
                true => {
                    self.secondary_subscriptions.push(ConsolidatorEnum::new_time_consolidator_and_warmup(new_subscription.clone(), history_to_retain, to_time, strategy_mode).await.unwrap());
                },
                false => {
                    self.secondary_subscriptions.push(ConsolidatorEnum::new_time_consolidator(new_subscription.clone(), history_to_retain).unwrap());
                }
            }

            self.active_count += 1;
        }
        else {
            self.subscription_event_buffer.push(DataSubscriptionEvent::Subscribed(new_subscription.clone()));
            self.primary_subscription = new_subscription;
        }
    }

    fn resolutions(&self, available_resolutions: Vec<Resolution>, data_resolution: Resolution) -> Vec<Resolution> {
        available_resolutions
            .into_iter()
            .filter(|resolution| match (resolution, &data_resolution) {
                (Resolution::Ticks(num), Resolution::Ticks(num_2)) => {
                    if num <= num_2 {
                        true
                    } else {
                        false
                    }
                },
                (Resolution::Seconds(num), Resolution::Seconds(num_2)) => {
                    if num <= num_2 {
                        true
                    } else {
                        false
                    }
                },
                (Resolution::Minutes(num), Resolution::Minutes(num_2)) => {
                    if num <= num_2 {
                        true
                    } else {
                        false
                    }
                },
                (Resolution::Hours(num), Resolution::Hours(num_2)) => {
                    if num <= num_2 {
                        true
                    } else {
                        false
                    }
                },
                (Resolution::Ticks(1), Resolution::Seconds(_)) => {
                    true
                },
                (Resolution::Seconds(_), Resolution::Minutes(_)) => {
                    true
                },
                (Resolution::Ticks(1), Resolution::Minutes(_)) => {
                    true
                },
                (Resolution::Minutes(_), Resolution::Hours(_)) => {
                    true
                },
                (Resolution::Ticks(1), Resolution::Hours(_)) => {
                    true
                },
                (Resolution::Seconds(_), Resolution::Hours(_)) => {
                    true
                },
                _ => false,
            })
            .collect()
    }

    async fn subscribe(&mut self, new_subscription: DataSubscription, history_to_retain: usize, to_time: DateTime<Utc>, strategy_mode: StrategyMode) {
        if self.all_subscriptions().await.contains(&new_subscription) {
            return
        }
        match new_subscription.resolution {
            Resolution::Ticks(number) => {
                if !new_subscription.symbol.data_vendor.resolutions(new_subscription.market_type.clone()).await.unwrap().contains(&Resolution::Ticks(1)) {
                    panic!("{} does not have tick data available", new_subscription.symbol.data_vendor);
                }
                // we switch to tick data as base resolution for any tick subscription
                if number > 1  {
                    match self.primary_subscription.resolution {
                        Resolution::Ticks(_) => {
                            let consolidator = match self.is_warmed_up {
                                true => ConsolidatorEnum::new_count_consolidator_and_warmup(self.primary_subscription.clone(), history_to_retain, to_time, strategy_mode).await.unwrap(),
                                false => ConsolidatorEnum::new_count_consolidator(self.primary_subscription.clone(), history_to_retain).unwrap(),
                            };
                            self.subscription_event_buffer.push(DataSubscriptionEvent::Subscribed(consolidator.subscription().clone()));
                            self.secondary_subscriptions.push(consolidator);

                        },
                        _ => {
                            let consolidator = match self.is_warmed_up {
                                true => ConsolidatorEnum::new_time_consolidator_and_warmup(self.primary_subscription.clone(), history_to_retain, to_time, strategy_mode).await.unwrap(),
                                false => ConsolidatorEnum::new_time_consolidator(self.primary_subscription.clone(), history_to_retain).unwrap(),
                            };
                            self.subscription_event_buffer.push(DataSubscriptionEvent::Subscribed(consolidator.subscription().clone()));
                            self.secondary_subscriptions.push(consolidator);
                        },
                    }
                }

                if self.primary_subscription.resolution != Resolution::Ticks(1) {
                    let new_primary_subscription = DataSubscription::new(new_subscription.symbol.name.clone(), new_subscription.symbol.data_vendor.clone(),  Resolution::Ticks(1), new_subscription.base_data_type.clone(),new_subscription.market_type.clone());
                    self.primary_subscription = new_primary_subscription.clone();
                    self.subscription_event_buffer.push(DataSubscriptionEvent::Subscribed(new_primary_subscription.clone()));
                }
            },
            _ => {
                // if the new subscription is of a lower resolution
                if new_subscription.resolution < self.primary_subscription.resolution {
                    self.select_primary_subscription(new_subscription, history_to_retain, to_time, strategy_mode).await;
                }
                else { //if we have no problem with adding new the resolution we can just add the new subscription as a consolidator
                    let consolidator = match self.is_warmed_up {
                        true => ConsolidatorEnum::new_time_consolidator_and_warmup(new_subscription.clone(), history_to_retain, to_time, strategy_mode).await.unwrap(),
                        false => ConsolidatorEnum::new_time_consolidator(new_subscription.clone(), history_to_retain).unwrap(),
                    };
                    self.secondary_subscriptions.push(consolidator);
                    self.subscription_event_buffer.push(DataSubscriptionEvent::Subscribed(new_subscription.clone()));
                }
            }
        }
        self.active_count += 1;
    }

    async fn unsubscribe(&mut self, subscription: &DataSubscription) {
        if subscription == &self.primary_subscription {
            if self.secondary_subscriptions.is_empty() {
                self.subscription_event_buffer.push(DataSubscriptionEvent::Unsubscribed(subscription.clone()));
                self.active_count -= 1;
                return;
            }
        } else { //if subscription is not the primary subscription, then it must be a consolidator and can be removed without changing the primary subscription
            self.secondary_subscriptions.retain(|consolidator| {
                &consolidator.subscription() != subscription
            });
            self.subscription_event_buffer.push(DataSubscriptionEvent::Unsubscribed(subscription.clone()));
            self.active_count -= 1;
        }
    }

    pub async fn all_subscriptions(&self) -> Vec<DataSubscription> {
        let mut all_subscriptions = vec![self.primary_subscription.clone()];
        for consolidator in self.secondary_subscriptions.iter() {
            all_subscriptions.push(consolidator.subscription());
        }
        all_subscriptions
    }

    pub async fn primary_subscription(&self) -> DataSubscription {
        self.primary_subscription.clone()
    }
}

pub enum ConsolidatorEnum {
    Count(CountConsolidator),
    Time(TimeConsolidator),
}

impl ConsolidatorEnum {
    pub async fn new_count_consolidator_and_warmup(subscription: DataSubscription, history_to_retain: usize, to_time: DateTime<Utc>, strategy_mode: StrategyMode) -> Result<Self, ConsolidatorError> {
        match CountConsolidator::new_and_warmup(subscription, history_to_retain, to_time, strategy_mode).await {
            Ok(consolidator) => Ok(ConsolidatorEnum::Count(consolidator)),
            Err(e) => Err(ConsolidatorError { message: e.message }),
        }
    }

    pub async fn new_time_consolidator_and_warmup(subscription: DataSubscription, history_to_retain: usize, to_time: DateTime<Utc>, strategy_mode: StrategyMode) -> Result<Self, ConsolidatorError> {
        match TimeConsolidator::new_and_warmup(subscription, history_to_retain, to_time, strategy_mode).await {
            Ok(consolidator) => Ok(ConsolidatorEnum::Time(consolidator)),
            Err(e) => Err(ConsolidatorError { message: e.message }),
        }
    }

    pub fn new_count_consolidator(subscription: DataSubscription, history_to_retain: usize) -> Result<Self, ConsolidatorError> {
        match CountConsolidator::new(subscription, history_to_retain) {
            Ok(consolidator) => Ok(ConsolidatorEnum::Count(consolidator)),
            Err(e) => Err(ConsolidatorError { message: e.message }),
        }
    }

    pub fn new_time_consolidator(subscription: DataSubscription,  history_to_retain: usize) -> Result<Self, ConsolidatorError> {
        match TimeConsolidator::new(subscription, history_to_retain) {
            Ok(consolidator) => Ok(ConsolidatorEnum::Time(consolidator)),
            Err(e) => Err(ConsolidatorError { message: e.message }),
        }
    }

    pub fn update(&mut self, base_data: &BaseDataEnum) -> Vec<BaseDataEnum> {
        match self {
            ConsolidatorEnum::Count(count_consolidator) => {
                count_consolidator.update(base_data)
            },
            ConsolidatorEnum::Time(time_consolidator) => {
                time_consolidator.update(base_data)
            },
        }
    }

    pub fn subscription(&self) -> DataSubscription {
        match self {
            ConsolidatorEnum::Count(count_consolidator) => count_consolidator.subscription.clone(),
            ConsolidatorEnum::Time(time_consolidator) => time_consolidator.subscription.clone(),
        }
    }
    
    pub fn resolution(&self) -> Resolution {
        match self {
            ConsolidatorEnum::Count(count_consolidator) => count_consolidator.subscription.resolution.clone(),
            ConsolidatorEnum::Time(time_consolidator) => time_consolidator.subscription.resolution.clone(),
        }
    }
    
    pub fn history_to_retain(&self) -> usize {
        match self {
            ConsolidatorEnum::Count(count_consolidator) => count_consolidator.history.number,
            ConsolidatorEnum::Time(time_consolidator) => time_consolidator.history.number,
        }
    }
}

#[derive(Debug)]
pub struct ConsolidatorError {
    message: String,
}

/// A consolidator that produces a new piece of data after a certain number of data points have been added.
/// Supports Ticks only.
pub struct CountConsolidator {
    number: u64,
    counter: u64,
    current_data: Candle,
    subscription: DataSubscription,
    history: RollingWindow,
}

impl CountConsolidator {
    pub async fn new_and_warmup(subscription: DataSubscription, history_to_retain: usize, warm_up_to_time: DateTime<Utc>, strategy_mode: StrategyMode) -> Result<Self, ConsolidatorError> {
        let number = match subscription.resolution {
            Resolution::Ticks(num) => num,
            _ => return Err(ConsolidatorError { message: format!("{:?} is an Invalid resolution for CountConsolidator", subscription.resolution) }),
        };

        let current_data = match subscription.base_data_type {
            BaseDataType::Ticks => Candle::new(subscription.symbol.clone(), 0.0, 0.0, "".to_string(), Resolution::Ticks(number)),
            _ => return Err(ConsolidatorError { message: format!("{} is an Invalid base data type for CountConsolidator", subscription.base_data_type) }),
        };

        let mut consolidator = CountConsolidator {
            number,
            counter: 0,
            current_data: current_data,
            subscription,
            history: RollingWindow::new(history_to_retain),
        };
        consolidator.warmup(warm_up_to_time, strategy_mode).await;
        Ok(consolidator)
    }

    pub fn new(subscription: DataSubscription, history_to_retain: usize) -> Result<Self, ConsolidatorError> {
        let number = match subscription.resolution {
            Resolution::Ticks(num) => num,
            _ => return Err(ConsolidatorError { message: format!("{:?} is an Invalid resolution for CountConsolidator", subscription.resolution) }),
        };

        let current_data = match subscription.base_data_type {
            BaseDataType::Ticks => Candle::new(subscription.symbol.clone(), 0.0, 0.0, "".to_string(), Resolution::Ticks(number)),
            _ => return Err(ConsolidatorError { message: format!("{} is an Invalid base data type for CountConsolidator", subscription.base_data_type) }),
        };

        Ok(CountConsolidator {
            number,
            counter: 0,
            current_data: current_data,
            subscription,
            history: RollingWindow::new(history_to_retain),
        })
    }


    async fn warmup(&mut self, to_time: DateTime<Utc>, strategy_mode: StrategyMode) {
        match strategy_mode {
            StrategyMode::Backtest => {
                //let historical_duration =
                let vendor_resolutions = self.subscription.symbol.data_vendor.resolutions(self.subscription.market_type.clone()).await.unwrap();
                let mut minimum_resolution: Option<Resolution> = None;
                if vendor_resolutions.contains(&Resolution::Ticks(1)) {
                    minimum_resolution = Some(Resolution::Ticks(1));
                }
                else {
                    return;
                }

                let minimum_resolution = match minimum_resolution.is_none() {
                    true => panic!("{} does not have any resolutions available", self.subscription.symbol.data_vendor),
                    false => minimum_resolution.unwrap()
                };

                let data_type = match minimum_resolution {
                    Resolution::Ticks(_) => BaseDataType::Ticks,
                    _ => self.subscription.base_data_type.clone()
                };

                //todo we need a way to calulate the numebr of weekends etc to add to the from_time
                let from_time = (to_time - (self.subscription.resolution.as_duration() * self.history.number as i32) - Duration::days(4)); //we go back a bit further in case of holidays or weekends

                let base_subscription = DataSubscription::new(self.subscription.symbol.name.clone(), self.subscription.symbol.data_vendor.clone(), minimum_resolution, data_type, self.subscription.market_type.clone());
                let base_data = range_data(from_time, to_time, base_subscription.clone()).await;

                for (_, slice) in &base_data {
                    for base_data in slice {
                        self.update(base_data);
                    }
                }
            },
            _ => {
                todo!("Finish implementing warmup for live trading");
            }
        }
    }

    pub(crate) fn clear_current_data(&mut self) {
        self.current_data = match self.subscription.base_data_type {
            BaseDataType::Ticks => {
                self.history.clear();
                self.counter = 0;
                Candle::new(self.subscription.symbol.clone(), 0.0, 0.0, "".to_string(), Resolution::Ticks(self.number))
            },
            _ => panic!("Invalid base data type for CountConsolidator: {}", self.subscription.base_data_type),
        };
    }

    /// Returns a candle if the count is reached
    pub fn update(&mut self, base_data: &BaseDataEnum) -> Vec<BaseDataEnum> {
        match base_data {
            BaseDataEnum::Tick(tick) => {
                let mut candles = vec![];
                if self.counter == 0 {
                    self.current_data.symbol = base_data.symbol().clone();
                    self.current_data.time = tick.time.clone();
                    self.current_data.open = tick.price;
                    self.current_data.volume = tick.volume;
                }
                self.counter += 1;
                if tick.price > self.current_data.high {
                    self.current_data.high = tick.price;
                }
                if tick.price < self.current_data.low {
                    self.current_data.low = tick.price;
                }
                self.current_data.close = tick.price;
                self.current_data.volume += tick.volume;
                if self.counter == self.number {
                    let mut consolidated_candle = self.current_data.clone();
                    consolidated_candle.is_closed = true;
                    self.counter = 0;
                    let consolidated_data = BaseDataEnum::Candle(consolidated_candle.clone());
                    self.history.add(consolidated_data.clone());
                    candles.push(consolidated_data);
                    self.current_data = match self.subscription.base_data_type {
                        BaseDataType::Ticks => Candle::new(self.subscription.symbol.clone(), 0.0, 0.0, "".to_string(), Resolution::Ticks(self.number)),
                        _ => panic!("Invalid base data type for CountConsolidator: {}", self.subscription.base_data_type),
                    };
                }
                else {
                    candles.push(BaseDataEnum::Candle(self.current_data.clone()));
                }
                candles
            },
            _ => panic!("Invalid base data type for CountConsolidator: {}", base_data.base_data_type()),
        }
    }
}


pub struct TimeConsolidator {
    current_data: Option<BaseDataEnum>,
    subscription: DataSubscription,
    history: RollingWindow,
}

impl TimeConsolidator {
    /// Creates a new TimeConsolidator
    /// 'subscription: DataSubscription' The TimeConsolidator will consolidate data based on the resolution of the subscription.
    /// 'history_to_retain: usize' will retain the last `history_to_retain` bars.
    /// 'warm_up_to_time: DateTime<Utc>' will warm up the history to the specified time.
    /// 'strategy_mode: StrategyMode' will use the specified strategy mode to warm up the history, if Live mode then we will need to get the most recent data from the vendor directly as we may not yet have the data in the serialized history.
    pub async fn new_and_warmup(subscription: DataSubscription, history_to_retain: usize, warm_up_to_time: DateTime<Utc>, strategy_mode: StrategyMode) -> Result<Self, ConsolidatorError> {
        if subscription.base_data_type == BaseDataType::Fundamentals {
            return Err(ConsolidatorError { message: format!("{} is an Invalid base data type for TimeConsolidator", subscription.base_data_type) });
        }

        if let Resolution::Ticks(_) = subscription.resolution {
            return Err(ConsolidatorError { message: format!("{:?} is an Invalid resolution for TimeConsolidator", subscription.resolution) });
        }

        //todo()! we should load the history from the server, run the consolidator until we have the bars we need for history
        let mut consolidator = TimeConsolidator {
            current_data: None,
            subscription,
            history: RollingWindow::new(history_to_retain),
        };
        consolidator.warmup(warm_up_to_time, strategy_mode).await;
        Ok(consolidator)
    }

    pub fn new(subscription: DataSubscription, history_to_retain: usize) -> Result<Self, ConsolidatorError> {
        if subscription.base_data_type == BaseDataType::Fundamentals {
            return Err(ConsolidatorError { message: format!("{} is an Invalid base data type for TimeConsolidator", subscription.base_data_type) });
        }

        if let Resolution::Ticks(_) = subscription.resolution {
            return Err(ConsolidatorError { message: format!("{:?} is an Invalid resolution for TimeConsolidator", subscription.resolution) });
        }

        Ok(TimeConsolidator {
            current_data: None,
            subscription,
            history: RollingWindow::new(history_to_retain),
        })
    }

    pub fn update(&mut self, base_data: &BaseDataEnum) -> Vec<BaseDataEnum> {
        match base_data.base_data_type() {
            BaseDataType::Ticks => {
                if self.subscription.base_data_type == BaseDataType::Candles {
                    return self.update_candles(base_data);
                }
            },
            BaseDataType::Quotes => {
                if self.subscription.base_data_type == BaseDataType::QuoteBars {
                    return self.update_quote_bars(base_data);
                }
            },
            BaseDataType::Prices => {
                if self.subscription.base_data_type == BaseDataType::Candles {
                    return self.update_candles(base_data);
                }
            }
            BaseDataType::QuoteBars => {
                if self.subscription.base_data_type == BaseDataType::QuoteBars {
                    return self.update_quote_bars(base_data);
                }
            }
            BaseDataType::Candles => {
                if self.subscription.base_data_type == BaseDataType::Candles {
                    return self.update_candles(base_data);
                }
            }
            BaseDataType::Fundamentals => panic!("Fundamentals are not supported"),
        }
        vec![]
    }

    fn new_quote_bar(&self, new_data: &BaseDataEnum) -> QuoteBar {
        let time = self.open_time(new_data.time_utc());
        match new_data {
            BaseDataEnum::QuoteBar(bar) => {
                let mut new_bar = bar.clone();
                new_bar.is_closed = false;
                new_bar.time = time.to_string();
                new_bar.resolution = self.subscription.resolution.clone();
                new_bar
            },
            BaseDataEnum::Quote(quote) => QuoteBar::new(self.subscription.symbol.clone(), quote.bid, quote.ask, 0.0, time.to_string(), self.subscription.resolution.clone()),
            _ => panic!("Invalid base data type for QuoteBar consolidator"),
        }
    }
    
    /// We can use if time == some multiple of resolution then we can consolidate, we dont need to know the actual algo time, because we can get time from the base_data if self.last_time >
    fn update_quote_bars(&mut self, base_data: &BaseDataEnum) -> Vec<BaseDataEnum> {
       if self.current_data.is_none() {
           let data = self.new_quote_bar(base_data);
           self.current_data = Some(BaseDataEnum::QuoteBar(data));
           return vec![self.current_data.clone().unwrap()]
       } else if let Some(current_bar) = self.current_data.as_mut() {
           if base_data.time_created_utc() >= current_bar.time_created_utc() {
               let mut consolidated_bar = current_bar.clone();
               consolidated_bar.set_is_closed(true);
                self.history.add(consolidated_bar.clone());
               let new_bar = self.new_quote_bar(base_data);
               self.current_data = Some(BaseDataEnum::QuoteBar(new_bar.clone()));
               return vec![consolidated_bar, BaseDataEnum::QuoteBar(new_bar)]
           }
           match current_bar {
               BaseDataEnum::QuoteBar(quote_bar) => {
                   match base_data {
                       BaseDataEnum::Quote(quote) => {
                           if quote.ask > quote_bar.ask_high {
                               quote_bar.ask_high = quote.ask;
                           }
                           if quote.ask < quote_bar.ask_low {
                               quote_bar.ask_low = quote.ask;
                           }
                           if quote.bid > quote_bar.bid_high {
                               quote_bar.bid_high = quote.bid;
                           }
                           if quote.bid < quote_bar.bid_low {
                               quote_bar.bid_low = quote.bid;
                           }
                           quote_bar.range = quote_bar.ask_high - quote_bar.bid_low;
                           quote_bar.ask_close = quote.ask;
                            quote_bar.bid_close = quote.bid;
                           return vec![BaseDataEnum::QuoteBar(quote_bar.clone())]
                       },
                       BaseDataEnum::QuoteBar(bar) => {
                            if bar.ask_high > quote_bar.ask_high {
                                 quote_bar.ask_high = bar.ask_high;
                            }
                            if bar.ask_low < quote_bar.ask_low {
                                 quote_bar.ask_low = bar.ask_low;
                            }
                            if bar.bid_high > quote_bar.bid_high {
                                 quote_bar.bid_high = bar.bid_high;
                            }
                            if bar.bid_low < quote_bar.bid_low {
                                 quote_bar.bid_low = bar.bid_low;
                            }
                           quote_bar.range = quote_bar.ask_high - quote_bar.bid_low;
                           quote_bar.ask_close = bar.ask_close;
                            quote_bar.bid_close = bar.bid_close;
                           quote_bar.volume += bar.volume;
                           return vec![BaseDataEnum::QuoteBar(quote_bar.clone())]
                       },
                       _ =>  panic!("Invalid base data type for QuoteBar consolidator: {}", base_data.base_data_type())

                   }
               }
               _ =>  panic!("Invalid base data type for QuoteBar consolidator: {}", base_data.base_data_type())
           }
       }
        panic!("Invalid base data type for QuoteBar consolidator: {}", base_data.base_data_type())
    }

    fn new_candle(&self, new_data: &BaseDataEnum) -> Candle {
        let time = self.open_time(new_data.time_utc());
        match new_data {
            BaseDataEnum::Tick(tick) => Candle::new(self.subscription.symbol.clone(), tick.price, tick.volume, time.to_string(), self.subscription.resolution.clone()),
            BaseDataEnum::Candle(candle) => {
                let mut consolidated_candle = candle.clone();
                consolidated_candle.is_closed = false;
                consolidated_candle.resolution = self.subscription.resolution.clone();
                consolidated_candle.time = time.to_string();
                consolidated_candle
            },
            BaseDataEnum::Price(price) => Candle::new(self.subscription.symbol.clone(), price.price, 0.0, time.to_string(), self.subscription.resolution.clone()),
            _ => panic!("Invalid base data type for Candle consolidator")
        }
    }

    pub(crate) fn clear_current_data(&mut self) {
        self.current_data = None;
        self.history.clear();
    }

    fn update_candles(&mut self, base_data: &BaseDataEnum) -> Vec<BaseDataEnum> {
        if self.current_data.is_none() {
            let data = self.new_candle(base_data);
            self.current_data = Some(BaseDataEnum::Candle(data));
            let candles = vec![self.current_data.clone().unwrap()];
            return candles
        } else if let Some(current_bar) = self.current_data.as_mut() {
            if base_data.time_created_utc() >= current_bar.time_created_utc() {
                let mut consolidated_bar = current_bar.clone();
                consolidated_bar.set_is_closed(true);
                self.history.add(consolidated_bar.clone());

                let new_bar = self.new_candle(base_data);
                self.current_data = Some(BaseDataEnum::Candle(new_bar.clone()));
                return vec![consolidated_bar, BaseDataEnum::Candle(new_bar)]
            }
            match current_bar {
                BaseDataEnum::Candle(candle) => {
                    match base_data {
                        BaseDataEnum::Tick(tick) => {
                            if tick.price > candle.high {
                                candle.high = tick.price;
                            }
                            if tick.price < candle.low {
                                candle.low = tick.price;
                            }
                            candle.close = tick.price;
                            candle.range = candle.high - candle.low;
                            candle.volume += tick.volume;
                            return vec![BaseDataEnum::Candle(candle.clone())]
                        },
                        BaseDataEnum::Candle(new_candle) => {
                            if new_candle.high > candle.high {
                                candle.high = new_candle.high;
                            }
                            if new_candle.low < candle.low {
                                candle.low = new_candle.low;
                            }
                            candle.range = candle.high - candle.low;
                            candle.close = new_candle.close;
                            candle.volume += new_candle.volume;
                            return vec![BaseDataEnum::Candle(candle.clone())]
                        },
                        BaseDataEnum::Price(price) => {
                            if price.price > candle.high {
                                candle.high = price.price;
                            }
                            if price.price < candle.low {
                                candle.low = price.price;
                            }
                            candle.range = candle.high - candle.low;
                            candle.close = price.price;
                            return vec![BaseDataEnum::Candle(candle.clone())]
                        },
                        _ => panic!("Invalid base data type for Candle consolidator: {}", base_data.base_data_type())
                    }
                },
                _ =>  panic!("Invalid base data type for Candle consolidator: {}", base_data.base_data_type())
            }
        }
        panic!("Invalid base data type for Candle consolidator: {}", base_data.base_data_type())
    }

    fn open_time(&self, time: DateTime<Utc>) -> DateTime<Utc> {
        match self.subscription.resolution {
            Resolution::Seconds(interval) => {
                let timestamp = time.timestamp();
                let rounded_timestamp = timestamp - (timestamp % interval as i64);
                DateTime::from_timestamp(rounded_timestamp, 0).unwrap()
            }
            Resolution::Minutes(interval) => {
                let minute = time.minute() as i64;
                let rounded_minute = (minute / interval as i64) * interval as i64;
                let rounded_time = time
                    .with_minute(rounded_minute as u32)
                    .unwrap()
                    .with_second(0)
                    .unwrap()
                    .with_nanosecond(0)
                    .unwrap();
                rounded_time
            }
            Resolution::Hours(interval) => {
                let hour = time.hour() as i64;
                let rounded_hour = (hour / interval as i64) * interval as i64;
                let rounded_time = time
                    .with_hour(rounded_hour as u32)
                    .unwrap()
                    .with_minute(0)
                    .unwrap()
                    .with_second(0)
                    .unwrap()
                    .with_nanosecond(0)
                    .unwrap();
                rounded_time
            }
            _ => time, // Handle other resolutions if necessary
        }
    }

    async fn warmup(&mut self, to_time: DateTime<Utc>, strategy_mode: StrategyMode) {
        match strategy_mode {
            StrategyMode::Backtest => {
                //let historical_duration =
                let vendor_resolutions = self.subscription.symbol.data_vendor.resolutions(self.subscription.market_type.clone()).await.unwrap();
                let mut minimum_resolution: Option<Resolution> = None;
                for resolution in vendor_resolutions {
                    if minimum_resolution.is_none() {
                        minimum_resolution = Some(resolution);
                    } else {
                        if resolution > minimum_resolution.unwrap() && resolution < self.subscription.resolution {
                            minimum_resolution = Some(resolution);
                        }
                    }
                }

                let minimum_resolution = match minimum_resolution.is_none() {
                    true => panic!("{} does not have any resolutions available", self.subscription.symbol.data_vendor),
                    false => minimum_resolution.unwrap()
                };

                let data_type = match minimum_resolution {
                    Resolution::Ticks(_) => BaseDataType::Ticks,
                    _ => self.subscription.base_data_type.clone()
                };

                //todo we need a way to calulate the numebr of weekends etc to add to the from_time
                let from_time = to_time - (self.subscription.resolution.as_duration() * self.history.number as i32) - Duration::days(4); //we go back a bit further in case of holidays or weekends

                let base_subscription = DataSubscription::new(self.subscription.symbol.name.clone(), self.subscription.symbol.data_vendor.clone(), minimum_resolution, data_type, self.subscription.market_type.clone());
                let base_data = range_data(from_time, to_time, base_subscription.clone()).await;

                for (_, slice) in &base_data {
                    for base_data in slice {
                        self.update(base_data);
                    }
                }
            },
            _ => {
                todo!("Finish implementing warmup for live trading");
            }
        }
    }
}