use std::collections::{HashMap, VecDeque};
use chrono::{DateTime, Timelike, Utc};
use tokio::sync::RwLock;
use crate::apis::vendor::client_requests::ClientSideDataVendor;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::base_data::quotebar::QuoteBar;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::data_server_messaging::FundForgeError;
use crate::standardized_types::enums::{Resolution};
use crate::standardized_types::subscriptions::{DataSubscription, Symbol};
use crate::standardized_types::time_slices::TimeSlice;


/// This Struct Handles when to consolidate data for a subscription from an existing subscription.
/// Alternatively if a subscription is of a lower resolution subscription, then the new subscription becomes the primary data source and the existing subscription becomes the secondary data source.
/// depending if the vendor has data available in that resolution.
pub struct SymbolSubscriptionHandler {
    /// The primary subscription is the subscription where data is coming directly from the `DataVendor`, In the event of bar data, it is pre-consolidated.
    primary_subscription: DataSubscription,
    /// The secondary subscriptions are consolidators that are used to consolidate data from the primary subscription.
    secondary_subscriptions: Vec<ConsolidatorEnum>,
    /// count the subscriptions so we can delete the object if it is no longer being used
    active_count : i32,
    symbol: Symbol,
    history_to_retain: usize,
}

impl SymbolSubscriptionHandler {
    pub async fn new(primary_subscription: DataSubscription, history_to_retain: usize) -> Self {
        let mut handler = SymbolSubscriptionHandler {
            primary_subscription: primary_subscription.clone(),
            secondary_subscriptions: vec![],
            active_count: 1,
            symbol: primary_subscription.symbol.clone(),
            history_to_retain,
        };
        // if we don't have the resolution available, we need to switch to a lower resolution for our primary subscription
        if !primary_subscription.symbol.data_vendor.resolutions_request(primary_subscription.market_type.clone()).await.unwrap().contains(&primary_subscription.resolution) {
            handler.select_primary_subscription(primary_subscription, history_to_retain).await;
        }
        handler
    }

    /// Updates the
    pub fn update(&mut self, base_data: &BaseDataEnum) -> Option<Vec<BaseDataEnum>> {
        // Ensure we only process if the symbol matches
        if &self.symbol != base_data.symbol() {
            return None;
        }
        
        if self.secondary_subscriptions.is_empty() {
            return None;
        }

        let mut consolidated_data = vec![];

        // Iterate over the secondary subscriptions and update them
        for consolidator in &mut self.secondary_subscriptions {
            if let Some(new_data) = consolidator.update(base_data) {
                consolidated_data.push(new_data);
            }
        }

        if consolidated_data.is_empty() {
            None
        } else {
            Some(consolidated_data)
        }
    }
    
    
    async fn select_primary_subscription(&mut self, new_subscription: DataSubscription, history_to_retain: usize) {
        let available_resolutions: Vec<Resolution> = new_subscription.symbol.data_vendor.resolutions_request(new_subscription.market_type.clone()).await.unwrap();
        println!("Available Resolutions: {:?}", available_resolutions);
        if available_resolutions.is_empty() {
            panic!("{} does not have any resolutions available", new_subscription.symbol.data_vendor);
        }
        let resolutions = self.resolutions(available_resolutions, new_subscription.resolution.clone());
        if resolutions.is_empty() {
            panic!("{} does not have any resolutions available", new_subscription.symbol.data_vendor);
        }
        if !resolutions.contains(&new_subscription.resolution) {
            self.secondary_subscriptions.push(ConsolidatorEnum::new_time_consolidator(new_subscription.clone(), new_subscription.base_data_type.clone(), new_subscription.resolution.clone(), history_to_retain).unwrap());
            let resolution = resolutions.iter().max().unwrap();
            self.primary_subscription = DataSubscription::new(new_subscription.symbol.name.clone(), new_subscription.symbol.data_vendor.clone(),  resolution.clone(), new_subscription.base_data_type.clone(),new_subscription.market_type.clone());
        }
        else {
            self.primary_subscription = new_subscription;
        }
    }

    fn resolutions(&self, available_resolutions: Vec<Resolution>, data_resolution: Resolution) -> Vec<Resolution> {
        let resolutions: Vec<Resolution> = available_resolutions
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
                (Resolution::Minutes(_), Resolution::Seconds(_)) => {
                    true
                },
                (Resolution::Hours(_), Resolution::Minutes(_)) => {
                    true
                },
                _ => false,
            })
            .collect();
        if resolutions.is_empty() {
            panic!("vendor has no resolutions available");
        }
        resolutions
    }

    async fn subscribe(&mut self, new_subscription: DataSubscription, history_to_retain: usize) {
        match new_subscription.resolution {
            Resolution::Ticks(number) => {
                if !new_subscription.symbol.data_vendor.resolutions_request(new_subscription.market_type.clone()).await.unwrap().contains(&Resolution::Ticks(1)) {
                    panic!("{} does not have tick data available", new_subscription.symbol.data_vendor);
                }
                // we switch to tick data as base resolution for any tick subscription
                if number > 1  {
                    match self.primary_subscription.resolution {
                        Resolution::Ticks(existing_number) => {
                            self.secondary_subscriptions.push(ConsolidatorEnum::new_count_consolidator(self.primary_subscription.clone(), existing_number, self.primary_subscription.base_data_type.clone(), history_to_retain).unwrap())
                        },
                        _ => self.secondary_subscriptions.push(ConsolidatorEnum::new_time_consolidator(self.primary_subscription.clone(), self.primary_subscription.base_data_type.clone(), self.primary_subscription.resolution.clone(), history_to_retain).unwrap()),
                    }
                }
                
                if &self.primary_subscription.resolution != &Resolution::Ticks(1) {
                    self.primary_subscription = DataSubscription::new(new_subscription.symbol.name.clone(), new_subscription.symbol.data_vendor.clone(),  Resolution::Ticks(1), new_subscription.base_data_type.clone(),new_subscription.market_type.clone());
                }
               
                self.history_to_retain = history_to_retain * number as usize;
            },
            _ => {
                // if the new subscription is of a lower resolution
                if new_subscription.resolution < self.primary_subscription.resolution {
                    self.select_primary_subscription(new_subscription, history_to_retain).await;
                    self.history_to_retain = history_to_retain;
                }
                else { //if we have no problem with adding new the resolution we can just add the new subscription as a consolidator
                    self.secondary_subscriptions.push(ConsolidatorEnum::new_time_consolidator(new_subscription.clone(), new_subscription.base_data_type.clone(), new_subscription.resolution.clone(), history_to_retain).unwrap());
                }
            }
        }
        self.active_count += 1;
    }

    async fn unsubscribe(&mut self, subscription: &DataSubscription) {
        if subscription == &self.primary_subscription {
            if let Some(lowest_subscription) = self.secondary_subscriptions.iter().map(|consolidator| consolidator.subscription()).min() {
                self.select_primary_subscription(lowest_subscription, self.history_to_retain).await;
            }
        } else { //if subscription is not the primary subscription, then it must be a consolidator and can be removed without changing the primary subscription
            self.secondary_subscriptions.retain(|consolidator| {
                &consolidator.subscription() != subscription
            });
        }
        self.active_count -= 1;
    }

    pub fn all_subscriptions(&self) -> Vec<DataSubscription> {
        let mut all_subscriptions = vec![self.primary_subscription.clone()];
        for consolidator in &self.secondary_subscriptions {
            all_subscriptions.push(consolidator.subscription());
        }
        all_subscriptions
    }
    
    pub fn primary_subscription(&self) -> DataSubscription {
        self.primary_subscription.clone()
    }
}

/// Manages all subscriptions for a backtest strategy, in live 1 static handler is shared for all strategies and platform requirements.
pub struct SubscriptionHandler {
    /// Manages the subscriptions of specific symbols
    symbol_subscriptions: RwLock<HashMap<Symbol, SymbolSubscriptionHandler>>,
    fundamental_subscriptions: RwLock<Vec<DataSubscription>>,
    /// Keeps a record when the strategy has updated its subscriptions, so we can pause the backtest to fetch new data.
    subscriptions_updated: RwLock<bool>,
}

impl SubscriptionHandler {
    pub async fn new() -> Self {
        SubscriptionHandler {
            fundamental_subscriptions: RwLock::new(vec![]),
            symbol_subscriptions: RwLock::new(Default::default()),
            subscriptions_updated: RwLock::new(false),
        }
    }
    

    pub async fn subscribe(&self, new_subscription: DataSubscription, history_to_retain: usize) -> Result<(), FundForgeError> {
        if new_subscription.base_data_type == BaseDataType::Fundamentals {
            //subscribe to fundamental
            if !self.fundamental_subscriptions.read().await.contains(&new_subscription) {
                self.fundamental_subscriptions.write().await.push(new_subscription.clone());
            }
            *self.subscriptions_updated.write().await = true;
            return Ok(())
        }
        if !self.symbol_subscriptions.read().await.contains_key(&new_subscription.symbol) {
            let symbol_handler = SymbolSubscriptionHandler::new(new_subscription.clone(), history_to_retain).await;
            self.symbol_subscriptions.write().await.insert(new_subscription.symbol.clone(), symbol_handler);
        }
        else { 
            let mut symbol_handler = self.symbol_subscriptions.write().await; 
            let symbol_handler = symbol_handler.get_mut(&new_subscription.symbol).unwrap();
            symbol_handler.subscribe(new_subscription, history_to_retain).await;
        }
        *self.subscriptions_updated.write().await = true;
        Ok(())
    }

    pub async fn unsubscribe(&self, subscription: DataSubscription) -> Result<(), FundForgeError>  {
        if subscription.base_data_type == BaseDataType::Fundamentals {
            if self.fundamental_subscriptions.read().await.contains(&subscription) {
                self.fundamental_subscriptions.write().await.retain(|fundamental_subscription| {
                    *fundamental_subscription != subscription
                });
            }
            *self.subscriptions_updated.write().await = true;
            return Ok(())
        }
        let mut symbol_handler = self.symbol_subscriptions.write().await;
        let symbol_handler = symbol_handler.get_mut(&subscription.symbol).unwrap();
        symbol_handler.unsubscribe(&subscription).await;
        if symbol_handler.active_count == 0 {
            self.symbol_subscriptions.write().await.remove(&subscription.symbol);
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

    pub async fn primary_subscriptions(&self) -> Vec<DataSubscription> {
        let mut primary_subscriptions = vec![];
        for symbol_handler in self.symbol_subscriptions.read().await.values() {
            primary_subscriptions.push(symbol_handler.primary_subscription());
        }
        primary_subscriptions
    }

    pub async fn subscriptions(&self) -> Vec<DataSubscription> {
        let mut all_subscriptions = vec![];
        for symbol_handler in self.symbol_subscriptions.read().await.values() {
            all_subscriptions.append(&mut symbol_handler.all_subscriptions());
        }
        for subscription in self.fundamental_subscriptions.read().await.iter() {
            all_subscriptions.push(subscription.clone());
        }
        all_subscriptions
    }

    /// Updates any consolidators with primary data
    pub async fn update_consolidators(&self, time_slice: TimeSlice) -> Option<TimeSlice> {
        let mut new_data: Vec<BaseDataEnum> = vec![];
        let mut symbol_subscriptions = self.symbol_subscriptions.write().await;

        for base_data in time_slice {
            let symbol = base_data.symbol();
            if let Some(symbol_handler) = symbol_subscriptions.get_mut(&symbol) {
                let consolidated_data = symbol_handler.update(&base_data);
                match consolidated_data {
                    Some(data) => {
                        new_data.extend(data);
                    },
                    None => {},
                }
            }
        }

        if new_data.is_empty() {
            None
        } else {
            Some(new_data)
        }
    }
}

pub enum ConsolidatorEnum {
    Count(CountConsolidator),
    Time(TimeConsolidator),
}

impl ConsolidatorEnum {
    pub fn new_count_consolidator(subscription: DataSubscription, number: u64, base_data_type: BaseDataType, history_to_retain: usize) -> Result<Self, ConsolidatorError> {
        match CountConsolidator::new(subscription, number, base_data_type, history_to_retain) {
            Ok(consolidator) => Ok(ConsolidatorEnum::Count(consolidator)),
            Err(e) => Err(ConsolidatorError { message: e.message }),
        }
    }

    pub fn new_time_consolidator(subscription: DataSubscription, base_data_type: BaseDataType, resolution: Resolution, history_to_retain: usize) -> Result<Self, ConsolidatorError> {
        match TimeConsolidator::new(subscription, base_data_type, resolution, history_to_retain) {
            Ok(consolidator) => Ok(ConsolidatorEnum::Time(consolidator)),
            Err(e) => Err(ConsolidatorError { message: e.message }),
        }
    }

    pub fn update(&mut self, base_data: &BaseDataEnum) -> Option<BaseDataEnum> {
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
    pub fn new(subscription: DataSubscription, number: u64, base_data_type: BaseDataType, retain_last: usize) -> Result<Self, ConsolidatorError> {
        if base_data_type != BaseDataType::Ticks {
            return Err(ConsolidatorError { message: format!("{} is an Invalid base data type for CountConsolidator", base_data_type) });
        }

        let current_data = match base_data_type {
            BaseDataType::Ticks => Candle::new(subscription.symbol.clone(), 0.0, 0.0, "".to_string(), Resolution::Ticks(number)),
            _ => return Err(ConsolidatorError { message: format!("{} is an Invalid base data type for CountConsolidator", base_data_type) }),
        };

        //todo()! we should load the history from the server, run the consolidator until we have the bars we need for history
        Ok(CountConsolidator {
            number,
            counter: 0,
            current_data,
            subscription,
            history: RollingWindow::new(retain_last),
        })
    }

    /// Returns a candle if the count is reached
    pub fn update(&mut self, base_data: &BaseDataEnum) -> Option<BaseDataEnum> {
        match base_data {
            BaseDataEnum::Tick(tick) => {
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
                    let mut consolidated_bar = self.current_data.clone();
                    consolidated_bar.is_closed = true;
                    self.counter = 0;
                    self.history.add(BaseDataEnum::Candle(consolidated_bar.clone()));
                    return Some(BaseDataEnum::Candle(consolidated_bar));
                }
                None
            },
            _ => None
        }
    }

    pub fn bars(&self) -> &RollingWindow {
        &self.history
    }

    pub fn bars_index(&self, index: usize) -> Option<&BaseDataEnum> {
        self.history.get(index)
    }

    pub fn bar_current(&self) -> &Candle {
        &self.current_data
    }
}

pub struct RollingWindow {
    last: VecDeque<BaseDataEnum>,
    number: usize,
}

impl RollingWindow {
    pub fn new(number: usize) -> Self {
        RollingWindow {
            last: VecDeque::with_capacity(number),
            number,
        }
    }

    pub fn add(&mut self, data: BaseDataEnum) {
        if self.last.len() == self.number {
            self.last.pop_back(); // Remove the oldest data
        }
        self.last.push_front(data); // Add the latest data at the front
    }
    
    pub fn last(&self) -> Option<&BaseDataEnum> {
        self.last.front()
    }

    pub fn get(&self, index: usize) -> Option<&BaseDataEnum> {
        self.last.get(index)
    }

    pub fn len(&self) -> usize {
        self.last.len()
    }

    pub fn is_full(&self) -> bool {
        self.last.len() == self.number
    }
}

pub struct TimeConsolidator {
    resolution: Resolution,
    current_data: Option<BaseDataEnum>,
    base_data_type: BaseDataType,
    subscription: DataSubscription,
    history: RollingWindow,
}

impl TimeConsolidator {
    pub fn new(subscription: DataSubscription, base_data_type: BaseDataType, resolution: Resolution, retain_last: usize) -> Result<Self, ConsolidatorError> {
        if base_data_type == BaseDataType::Fundamentals {
            return Err(ConsolidatorError { message: format!("{} is an Invalid base data type for TimeConsolidator", base_data_type) });
        }

        if let Resolution::Ticks(_) = resolution {
            return Err(ConsolidatorError { message: format!("{:?} is an Invalid resolution for TimeConsolidator", resolution) });
        }

        //todo()! we should load the history from the server, run the consolidator until we have the bars we need for history
        Ok(TimeConsolidator {
            resolution,
            current_data: None,
            base_data_type,
            subscription,
            history: RollingWindow::new(retain_last),
        })
    }

    pub fn update(&mut self, base_data: &BaseDataEnum) -> Option<BaseDataEnum> {
        match base_data.base_data_type() {
            BaseDataType::Ticks => {
                if self.base_data_type == BaseDataType::Candles {
                    return self.update_candles(base_data);
                }
            },
            BaseDataType::Quotes => {
                if self.base_data_type == BaseDataType::QuoteBars {
                    return self.update_quote_bars(base_data);
                }
            },
            BaseDataType::Prices => {
                if self.base_data_type == BaseDataType::Candles {
                    return self.update_candles(base_data);
                }
            }
            BaseDataType::QuoteBars => {
                if self.base_data_type == BaseDataType::QuoteBars {
                    return self.update_quote_bars(base_data);
                }
            }
            BaseDataType::Candles => {
                if self.base_data_type == BaseDataType::Candles {
                    return self.update_candles(base_data);
                }
            }
            BaseDataType::Fundamentals => panic!("Fundamentals are not supported"),
        }
        None
    }

    fn new_quote_bar(&self, new_data: &BaseDataEnum) -> QuoteBar {
        let time = self.open_time(new_data.time_utc());
        match new_data {
            BaseDataEnum::QuoteBar(bar) => {
                let mut new_bar = QuoteBar::new(self.subscription.symbol.clone(), bar.bid_open, bar.ask_open, 0.0, time.to_string(), self.resolution.clone());
                new_bar.ask_high = bar.ask_high;
                new_bar.ask_low = bar.ask_low;
                new_bar.ask_close = bar.ask_close;
                new_bar.ask_open = bar.ask_open;
                new_bar.bid_high = bar.bid_high;
                new_bar.bid_low = bar.bid_low;
                new_bar.bid_close = bar.bid_close;
                new_bar.bid_open = bar.bid_open;
                new_bar
            },
            BaseDataEnum::Quote(quote) => QuoteBar::new(self.subscription.symbol.clone(), quote.bid, quote.ask, 0.0, time.to_string(), self.resolution.clone()),
            _ => panic!("Invalid base data type for QuoteBar consolidator"),
        }
    }
    
    pub fn bars(&self) -> &RollingWindow {
        &self.history
    }
    
    pub fn bars_index(&self, index: usize) -> Option<&BaseDataEnum> {
        self.history.get(index)
    }
    
    pub fn bar_current(&self) -> &BaseDataEnum {
        self.current_data.as_ref().unwrap()
    }
    
    /// We can use if time == some multiple of resolution then we can consolidate, we dont need to know the actual algo time, because we can get time from the base_data if self.last_time >
    fn update_quote_bars(&mut self, base_data: &BaseDataEnum) -> Option<BaseDataEnum> {
       if self.current_data.is_none() {
           let data = self.new_quote_bar(base_data);
           self.current_data = Some(BaseDataEnum::QuoteBar(data));
           return None
       } else if let Some(current_bar) = self.current_data.as_mut() {
           if base_data.time_created_utc() >= current_bar.time_created_utc() {
               let mut consolidated_bar = current_bar.clone();
               consolidated_bar.set_is_closed(true);

               let new_bar = self.new_quote_bar(base_data);
               self.current_data = Some(BaseDataEnum::QuoteBar(new_bar));
               self.history.add(consolidated_bar.clone());
               return Some(consolidated_bar)
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
                           quote_bar.ask_close = quote.ask;
                            quote_bar.bid_close = quote.bid;
                           return None 
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
                           quote_bar.ask_close = bar.ask_close;
                            quote_bar.bid_close = bar.bid_close;
                           return None
                       },
                       _ => panic!("Invalid base data type for QuoteBar consolidator")

                   }
               }
               _ => panic!("Invalid base data type for QuoteBar consolidator")
           }
       }
       panic!("Invalid base data type for QuoteBar consolidator")
    }

    fn new_candle(&self, new_data: &BaseDataEnum) -> Candle {
        let time = self.open_time(new_data.time_utc());
        match new_data {
            BaseDataEnum::Tick(tick) => Candle::new(self.subscription.symbol.clone(), tick.price, tick.volume, time.to_string(), self.resolution.clone()),
            BaseDataEnum::Candle(candle) => Candle::new(self.subscription.symbol.clone(), candle.open, candle.volume, time.to_string(), self.resolution.clone()),
            BaseDataEnum::Price(price) => Candle::new(self.subscription.symbol.clone(), price.price, 0.0, time.to_string(), self.resolution.clone()),
            _ => panic!("Invalid base data type for Candle consolidator")
        }
    }
    
    fn update_candles(&mut self, base_data: &BaseDataEnum) -> Option<BaseDataEnum> {
        if self.current_data.is_none() {
            let data = self.new_candle(base_data);
            self.current_data = Some(BaseDataEnum::Candle(data));
            return None
        } else if let Some(current_bar) = self.current_data.as_mut() {
            if base_data.time_created_utc() >= current_bar.time_created_utc() {
                let mut consolidated_bar = current_bar.clone();
                consolidated_bar.set_is_closed(true);

                let new_bar = self.new_candle(base_data);
                self.current_data = Some(BaseDataEnum::Candle(new_bar));
                self.history.add(consolidated_bar.clone());
                return Some(consolidated_bar)
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
                            return None
                        },
                        BaseDataEnum::Candle(new_candle) => {
                            if new_candle.high > candle.high {
                                candle.high = new_candle.high;
                            }
                            if new_candle.low < candle.low {
                                candle.low = new_candle.low;
                            }
                            candle.close = new_candle.close;
                            return None
                        },
                        BaseDataEnum::Price(price) => {
                            if price.price > candle.high {
                                candle.high = price.price;
                            }
                            if price.price < candle.low {
                                candle.low = price.price;
                            }
                            candle.close = price.price;
                            return None
                        },
                        _ => panic!("Invalid base data type for QuoteBar consolidator")
                    }
                },
                _ => panic!("Invalid base data type for QuoteBar consolidator")
            }
        }
        panic!("Invalid base data type for QuoteBar consolidator")
    }

    fn open_time(&self, time: DateTime<Utc>) -> DateTime<Utc> {
        match self.resolution {
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
}