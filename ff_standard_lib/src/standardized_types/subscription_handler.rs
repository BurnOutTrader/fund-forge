use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::apis::vendor::client_requests::ClientSideDataVendor;
use crate::consolidators::consolidator_enum::{ConsolidatedData, ConsolidatorEnum};
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::enums::{Resolution, StrategyMode, SubscriptionResolutionType};
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions;
use crate::standardized_types::subscriptions::{DataSubscription, DataSubscriptionEvent, Symbol};
use crate::standardized_types::time_slices::TimeSlice;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use futures::channel::mpsc::Receiver;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;
use crate::standardized_types::strategy_events::EventTimeSlice;

/// Manages all subscriptions for a strategy. each strategy has its own subscription handler.
pub struct SubscriptionHandler {
    /// Manages the consolidators of specific symbols
    symbol_subscriptions: DashMap<Symbol, SymbolSubscriptionHandler>,
    /// fundamental data is not consolidated and so it does not need special handlers
    fundamental_subscriptions: RwLock<Vec<DataSubscription>>,
    /// Keeps a record when the strategy has updated its subscriptions, so we can pause the backtest to fetch new data.
    subscriptions_updated: AtomicBool,
    is_warmed_up: AtomicBool,
    strategy_mode: StrategyMode,
    // subscriptions which the strategy actually subscribed to, not the raw data needed to full-fill the subscription.
    strategy_subscriptions: RwLock<Vec<DataSubscription>>,
}

impl SubscriptionHandler {
    pub async fn new(strategy_mode: StrategyMode) -> Self {
        SubscriptionHandler {
            fundamental_subscriptions: Default::default(),
            symbol_subscriptions: Default::default(),
            subscriptions_updated: AtomicBool::new(true),
            is_warmed_up: AtomicBool::new(false),
            strategy_mode,
            strategy_subscriptions: Default::default(),
        }
    }

    /// Sets the SubscriptionHandler as warmed up, so we can start processing data.
    /// This lets the handler know that it needs to manually warm up any future subscriptions.
    pub async fn set_warmup_complete(&self) {
        self.is_warmed_up.store(true, Ordering::SeqCst);
        for symbol_handler in self.symbol_subscriptions.iter() {
            symbol_handler.value().set_warmed_up().await;
        }
    }

    /// Returns all the subscription events that have occurred since the last time this method was called.
    pub async fn subscription_events(&self) -> Vec<DataSubscriptionEvent> {
        let mut subscription_events = vec![];
        for symbol_handler in self.symbol_subscriptions.iter() {
            subscription_events.extend(symbol_handler.value().get_subscription_event_buffer().await);
        }
        subscription_events
    }

    pub async fn strategy_subscriptions(&self) -> Vec<DataSubscription> {
        let strategy_subscriptions = self.strategy_subscriptions.read().await;
        strategy_subscriptions.clone()
    }

    /// Subscribes to a new data subscription
    /// 'new_subscription: DataSubscription' The new subscription to subscribe to.
    /// 'history_to_retain: usize' The number of bars to retain in the history.
    /// 'current_time: DateTime<Utc>' The current time is used to warm up consolidator history if we have already done our initial strategy warm up.
    /// 'strategy_mode: StrategyMode' The strategy mode is used to determine how to warm up the history, in live mode we may not yet have a serialized history to the current time.
    pub async fn subscribe(
        &self,
        new_subscription: DataSubscription,
        history_to_retain: u64,
        current_time: DateTime<Utc>,
    ) {

        let mut strategy_subscriptions = self.strategy_subscriptions.write().await;
        if !strategy_subscriptions.contains(&new_subscription) {
            strategy_subscriptions.push(new_subscription.clone());
        }

        if new_subscription.base_data_type == BaseDataType::Fundamentals {
            //subscribe to fundamental
            let mut fundamental_subscriptions = self.fundamental_subscriptions.write().await;
            if !fundamental_subscriptions.contains(&new_subscription) {
                fundamental_subscriptions.push(new_subscription.clone());
            }
            self.subscriptions_updated.store(true, Ordering::SeqCst);
            return;
        }

        if !self.symbol_subscriptions.contains_key(&new_subscription.symbol) {
            let symbol_handler = SymbolSubscriptionHandler::new(
                new_subscription.clone(),
                self.is_warmed_up.load(Ordering::SeqCst),
                history_to_retain,
                current_time,
                self.strategy_mode,
            )
            .await;
            self.symbol_subscriptions.insert(new_subscription.symbol.clone(), symbol_handler);
        }

        self.symbol_subscriptions.get(&new_subscription.symbol).unwrap()
            .subscribe(
                new_subscription,
                history_to_retain,
                current_time,
                self.strategy_mode,
            )
            .await;

        self.subscriptions_updated.store(true, Ordering::SeqCst);
    }

    pub async fn set_subscriptions(
            &self,
            new_subscription: Vec<DataSubscription>,
            history_to_retain: u64,
            current_time: DateTime<Utc>,
    ) {
        let current_subscriptions = self.subscriptions().await;
        for sub in current_subscriptions {
            if !new_subscription.contains(&sub) {
                self.unsubscribe(sub.clone()).await;
            }
        }
        for sub in new_subscription {
           self.subscribe(sub.clone(),history_to_retain.clone(), current_time.clone()).await;
        }
        self.subscriptions_updated.store(true, Ordering::SeqCst);
    }

    /// Unsubscribes from a data subscription
    /// 'subscription: DataSubscription' The subscription to unsubscribe from.
    /// 'current_time: DateTime<Utc>' The current time is used to change our base data subscription and warm up any new consolidators if we are adjusting our base resolution.
    /// 'strategy_mode: StrategyMode' The strategy mode is used to determine how to warm up the history, in live mode we may not yet have a serialized history to the current time.
    pub async fn unsubscribe(&self, subscription: DataSubscription) {
        if subscription.base_data_type == BaseDataType::Fundamentals {
            let mut fundamental_subscriptions = self.fundamental_subscriptions.write().await;
            if fundamental_subscriptions.contains(&subscription) {
                fundamental_subscriptions
                    .retain(|fundamental_subscription| *fundamental_subscription != subscription);
            }
            let mut strategy_subscriptions = self.strategy_subscriptions.write().await;
            if strategy_subscriptions.contains(&subscription) {
                strategy_subscriptions.retain(|x| x != &subscription);
            }
            self.subscriptions_updated.store(true, Ordering::SeqCst);
            return;
        }

        self.symbol_subscriptions.get(&subscription.symbol).unwrap().unsubscribe(&subscription).await;
        if self.symbol_subscriptions.get(&subscription.symbol).unwrap().active_count() == 0 {
            self.symbol_subscriptions.remove(&subscription.symbol);
        }
        let mut strategy_subscriptions = self.strategy_subscriptions.write().await;
        if strategy_subscriptions.contains(&subscription) {
            strategy_subscriptions.retain(|x| x != &subscription);
        }
        self.subscriptions_updated.store(true, Ordering::SeqCst);
    }

    pub async fn get_subscriptions_updated(&self) -> bool {
        self.subscriptions_updated.load(Ordering::SeqCst)
    }

    pub async fn set_subscriptions_updated(&self, updated: bool) {
        self.subscriptions_updated.store(updated, Ordering::SeqCst);
    }

    /// Returns all the primary subscriptions
    /// These are subscriptions that come directly from the vendors own data source.
    /// They are not consolidators, but are the primary source of data for the consolidators.
    pub async fn primary_subscriptions(&self) -> Vec<DataSubscription> {
        let mut primary_subscriptions = vec![];
        for symbol_handler in self.symbol_subscriptions.iter() {
            primary_subscriptions.push(symbol_handler.value().primary_subscription().await);
        }
        primary_subscriptions
    }

    /// Returns all the subscriptions including primary and consolidators
    pub async fn subscriptions(&self) -> Vec<DataSubscription> {
        let mut all_subscriptions = vec![];
        for symbol_handler in self.symbol_subscriptions.iter() {
            all_subscriptions.append(&mut symbol_handler.value().all_subscriptions().await);
        }
        for subscription in self.fundamental_subscriptions.read().await.iter() {
            all_subscriptions.push(subscription.clone());
        }
        all_subscriptions
    }

    /// Updates any consolidators with primary data
    pub async fn update_time_slice(&self, time_slice: &TimeSlice) -> Option<TimeSlice> {
        let mut open_bars: BTreeMap<DataSubscription, BaseDataEnum> = BTreeMap::new();
        let mut closed_bars = Vec::new();

        // Clone the Arc to the symbol subscriptions.
        //let symbol_subscriptions = self.symbol_subscriptions.clone();

        // Create a FuturesUnordered to collect all futures and run them concurrently.
        let mut update_futures = FuturesUnordered::new();

        for base_data in time_slice.iter() {
            let symbol = base_data.symbol();
           // let symbol_subscriptions = symbol_subscriptions.clone(); // Clone the Arc for each task.
            let base_data = base_data.clone(); // Clone base_data to avoid borrowing issues.

            // Add the future to the FuturesUnordered.
            update_futures.push(async move {
                // Get a read guard inside the async block to avoid lifetime issues.
                if let Some(handler) = self.symbol_subscriptions.get(&symbol) {
                    handler.update(&base_data).await
                } else {
                    Vec::new() // Return empty if handler is not found.
                }
            });
        }

        // Process all the updates concurrently.
        while let Some(data) = update_futures.next().await {
            for consolidated_bars in data {
                if let Some(consolidated_bar) = consolidated_bars.closed_data {
                    closed_bars.push(consolidated_bar);
                }
                open_bars.insert(consolidated_bars.open_data.subscription(), consolidated_bars.open_data);
            }
        }

        // Combine open and closed bars.
        for (_, data) in open_bars {
            closed_bars.push(data);
        }

        if closed_bars.is_empty() {
            None
        } else {
            Some(closed_bars)
        }
    }

    pub async fn update_consolidators_time(&self, time: DateTime<Utc>) -> Option<TimeSlice> {
        let futures: Vec<_> = self.symbol_subscriptions.iter().map(|(symbol_handler)| {
            let time = time.clone();
            // Creating async blocks that will run concurrently
            async move {
                symbol_handler.value().update_time(time).await
            }
        }).collect();

        // Execute all futures concurrently
        let results = futures::future::join_all(futures).await;

        // Collect the results into a TimeSlice
        let mut time_slice = TimeSlice::new();
        for result in results {
            if let Some(data) = result {
                time_slice.extend(data);
            }
        }

        if time_slice.is_empty() {
            None
        } else {
            Some(time_slice)
        }
    }

    pub async fn history(
        &self,
        subscription: &DataSubscription,
    ) -> Option<RollingWindow<BaseDataEnum>> {
        if subscription.base_data_type == BaseDataType::Fundamentals {
            return None;
        }
        if let Some(symbol_subscription) = self
            .symbol_subscriptions
            .get(&subscription.symbol)
        {
            if let Some(consolidator) = symbol_subscription
                .secondary_subscriptions
                .get(subscription)
            {
                return Some(consolidator.history());
            }
        }
        None
    }

    pub async fn bar_index(
        &self,
        subscription: &DataSubscription,
        index: usize,
    ) -> Option<BaseDataEnum> {
        if subscription.base_data_type == BaseDataType::Fundamentals {
            return None;
        }
        if let Some(symbol_subscription) = self
            .symbol_subscriptions
            .get(&subscription.symbol)
        {
            if let Some(consolidator) = symbol_subscription
                .secondary_subscriptions
                .get(subscription)
            {
                if consolidator.subscription() == subscription {
                    return consolidator.index(index);
                }
            }
        }
        None
    }

    pub async fn bar_current(&self, subscription: &DataSubscription) -> Option<BaseDataEnum> {
        if subscription.base_data_type == BaseDataType::Fundamentals {
            return None;
        }

        if let Some(symbol_subscription) = self.symbol_subscriptions.get(&subscription.symbol) {
            let primary_subscription = symbol_subscription.primary_subscription().await;
            if &primary_subscription == subscription {
                return None;
            }
            if let Some(consolidator) = symbol_subscription
                .secondary_subscriptions
                .get(subscription)
            {
                if consolidator.subscription() == subscription {
                    return consolidator.current();
                }
            }
        }
        None
    }
}

/// This Struct Handles when to consolidate data for a subscription from an existing subscription.
/// Alternatively if a subscription is of a lower resolution subscription, then the new subscription becomes the primary data source and the existing subscription becomes the secondary data source.
/// depending if the vendor has data available in that resolution.
pub struct SymbolSubscriptionHandler {
    /// The primary subscription is the subscription where data is coming directly from the `DataVendor`, In the event of bar data, it is pre-consolidated.
    primary_subscription: RwLock<DataSubscription>,
    /// The secondary subscriptions are consolidators that are used to consolidate data from the primary subscription.
    secondary_subscriptions: DashMap<DataSubscription, ConsolidatorEnum>,
    symbol: Symbol,
    subscription_event_buffer: RwLock<Vec<DataSubscriptionEvent>>,
    is_warmed_up: AtomicBool,
    primary_history: RwLock<RollingWindow<BaseDataEnum>>,
}

impl SymbolSubscriptionHandler {
    pub async fn new(
        primary_subscription: DataSubscription,
        is_warmed_up: bool,
        history_to_retain: u64,
        warm_up_to: DateTime<Utc>,
        strategy_mode: StrategyMode,
    ) -> Self {
        let mut handler = SymbolSubscriptionHandler {
            primary_subscription: RwLock::new(primary_subscription.clone()),
            secondary_subscriptions: Default::default(),
            symbol: primary_subscription.symbol.clone(),
            subscription_event_buffer: Default::default(),
            is_warmed_up: AtomicBool::new(is_warmed_up),
            primary_history: RwLock::new(RollingWindow::new(history_to_retain)),
        };
        handler
            .select_primary_non_tick_subscription(
                primary_subscription,
                history_to_retain,
                warm_up_to,
                strategy_mode,
            )
            .await;
        handler
    }

    pub fn active_count(&self) -> usize {
        self.secondary_subscriptions.len()
    }

    pub async fn update(&self, base_data_enum: &BaseDataEnum) -> Vec<ConsolidatedData> {
         // Read the secondary subscriptions
        if self.secondary_subscriptions.is_empty() {
            return vec![];
        }

        if base_data_enum.subscription() != *self.primary_subscription.read().await {
            return vec![]
        }

        self.primary_history.write().await.add(base_data_enum.clone());

        let mut data = vec![];
        //todo this doesnt work because the data subscription is never the consolidator subscription
        for mut consolidator in self.secondary_subscriptions.iter_mut() {
            data.push(consolidator.value_mut().update(base_data_enum))
        }
        data
    }

    pub async fn update_time(&self, time: DateTime<Utc>) -> Option<Vec<BaseDataEnum>> {
        let mut consolidated_data = vec![];
        // Iterate over the secondary subscriptions and update them
        for mut consolidator in self.secondary_subscriptions.iter_mut() {
            let data = consolidator.value_mut().update_time(time.clone());
            consolidated_data.extend(data);
        }
        match consolidated_data.is_empty() {
            true => None,
            false => Some(consolidated_data),
        }
    }

    pub async fn set_warmed_up(&self) {
        self.is_warmed_up.store(true, Ordering::SeqCst);
    }

    pub async fn get_subscription_event_buffer(&self) -> Vec<DataSubscriptionEvent> {
        let mut buffer = self.subscription_event_buffer.write().await;
        let return_buffer = buffer.clone();
        buffer.clear();
        return_buffer
    }

    /// This is only used
    async fn select_primary_non_tick_subscription(
        &self,
        new_subscription: DataSubscription,
        history_to_retain: u64,
        to_time: DateTime<Utc>,
        strategy_mode: StrategyMode,
    ) {
        let available_resolutions: Vec<SubscriptionResolutionType> = new_subscription
            .symbol
            .data_vendor
            .resolutions(new_subscription.market_type.clone())
            .await
            .unwrap();
        //println!("Available Resolutions: {:?}", available_resolutions);
        if available_resolutions.is_empty() {
            panic!(
                "{} does not have any resolutions available",
                new_subscription.symbol.data_vendor
            );
        }
        let resolution_types = subscriptions::filter_resolutions(
            available_resolutions,
            new_subscription.resolution.clone(),
        );
        if resolution_types.is_empty() {
            panic!("{} does not have any resolutions available for {:?}, Problem in select_primary_non_tick_subscription or vendor.resolutions() fn", new_subscription.symbol.data_vendor, new_subscription);
        }

        let mut subscription_set = false;
        let mut primary_subscription = self.primary_subscription.write().await;
        //if we have the resolution avaialable from the vendor, just use it.
        for subscription_resolution_type in &resolution_types {
            if subscription_resolution_type.resolution == new_subscription.resolution && subscription_resolution_type.base_data_type == new_subscription.base_data_type {
                self.subscription_event_buffer.write().await.push(DataSubscriptionEvent::Subscribed(new_subscription.clone()));
                *primary_subscription = new_subscription.clone();
                self.primary_history.write().await.history.clear();
                subscription_set = true;
                break;
            }
        }
        if !subscription_set {
            self.secondary_subscriptions.insert(
                new_subscription.clone(),
                ConsolidatorEnum::create_consolidator(
                    self.is_warmed_up.load(Ordering::SeqCst),
                    new_subscription.clone(),
                    history_to_retain,
                    to_time,
                    strategy_mode,
                )
                .await,
            );

            let max_resolution = resolution_types.iter().max_by_key(|r| r.resolution);
            if let Some(resolution_type) = max_resolution {
                let subscription = DataSubscription::new(
                    new_subscription.symbol.name.clone(),
                    new_subscription.symbol.data_vendor.clone(),
                    resolution_type.resolution,
                    resolution_type.base_data_type,
                    new_subscription.market_type.clone(),
                );

                *primary_subscription = subscription
            }
        }
    }

    async fn subscribe(
        &self,
        new_subscription: DataSubscription,
        history_to_retain: u64,
        to_time: DateTime<Utc>,
        strategy_mode: StrategyMode,
    ) {
        if self.all_subscriptions().await.contains(&new_subscription) {
            return;
        }

        let mut subscription_event_buffer = self.subscription_event_buffer.write().await;
        match new_subscription.resolution {
            Resolution::Ticks(number) => {

                let res_type_tick =
                    SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks);
                if !new_subscription
                    .symbol
                    .data_vendor
                    .resolutions(new_subscription.market_type.clone())
                    .await
                    .unwrap()
                    .contains(&res_type_tick)
                {
                    panic!(
                        "{} does not have tick data available",
                        new_subscription.symbol.data_vendor
                    );
                }
                // we switch to tick data as base resolution for any tick subscription
                if number > 1 {
                    let consolidator = ConsolidatorEnum::create_consolidator(
                        self.is_warmed_up.load(Ordering::SeqCst),
                        new_subscription.clone(),
                        history_to_retain,
                        to_time,
                        strategy_mode,
                    )
                    .await;
                    subscription_event_buffer
                        .push(DataSubscriptionEvent::Subscribed(
                            consolidator.subscription().clone(),
                        ));

                    self.secondary_subscriptions
                        .insert(consolidator.subscription().clone(), consolidator);
                }
                let mut primary_subscription = self.primary_subscription.write().await;
                if primary_subscription.resolution != Resolution::Ticks(1) {
                    let new_primary_subscription = DataSubscription::new(
                        new_subscription.symbol.name.clone(),
                        new_subscription.symbol.data_vendor.clone(),
                        Resolution::Ticks(1),
                        new_subscription.base_data_type.clone(),
                        new_subscription.market_type.clone(),
                    );
                    *primary_subscription = new_primary_subscription.clone();
                    subscription_event_buffer
                        .push(DataSubscriptionEvent::Subscribed(
                            new_primary_subscription.clone(),
                        ));
                }
            }
            _ => {
                // if the new subscription is of a lower resolution
                if new_subscription.resolution < self.primary_subscription.read().await.resolution {
                    self.select_primary_non_tick_subscription(
                        new_subscription,
                        history_to_retain,
                        to_time,
                        strategy_mode,
                    )
                    .await;
                } else {
                    //if we have no problem with adding new the resolution we can just add the new subscription as a consolidator
                    let consolidator = ConsolidatorEnum::create_consolidator(
                        self.is_warmed_up.load(Ordering::SeqCst),
                        new_subscription.clone(),
                        history_to_retain,
                        to_time,
                        strategy_mode,
                    )
                    .await;
                    self.secondary_subscriptions
                        .insert(new_subscription.clone(), consolidator);
                    subscription_event_buffer
                        .push(DataSubscriptionEvent::Subscribed(new_subscription.clone()));
                }
            }
        }
    }

    async fn unsubscribe(&self, subscription: &DataSubscription) {
        let mut subscription_event_buffer = self.subscription_event_buffer.write().await;

        if subscription == &*self.primary_subscription.read().await {
            if self.secondary_subscriptions.is_empty() {
                subscription_event_buffer
                    .push(DataSubscriptionEvent::Unsubscribed(subscription.clone()));
                return;
            }
        } else {
            //if subscription is not the primary subscription, then it must be a consolidator and can be removed without changing the primary subscription
            self.secondary_subscriptions.remove(subscription);
            subscription_event_buffer
                .push(DataSubscriptionEvent::Unsubscribed(subscription.clone()));
        }
    }

    pub async fn all_subscriptions(&self) -> Vec<DataSubscription> {
        let mut all_subscriptions = vec![self.primary_subscription.read().await.clone()];
        for consolidator in self.secondary_subscriptions.iter() {
            all_subscriptions.push(consolidator.subscription().clone());
        }
        all_subscriptions
    }

    pub async fn primary_subscription(&self) -> DataSubscription {
        self.primary_subscription.read().await.clone()
    }
}
