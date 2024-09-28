use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use ahash::AHashMap;
use async_std::task::block_on;
use crate::consolidators::consolidator_enum::{ConsolidatedData, ConsolidatorEnum};
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::enums::{Resolution, StrategyMode, SubscriptionResolutionType};
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::{DataSubscription, DataSubscriptionEvent, Symbol};
use crate::standardized_types::time_slices::TimeSlice;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use tokio::sync::mpsc::{Sender};
use tokio::sync::{RwLock};
use crate::servers::internal_broadcaster::StaticInternalBroadcaster;
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::base_data::fundamental::Fundamental;
use crate::standardized_types::base_data::history::range_data;
use crate::standardized_types::base_data::quote::Quote;
use crate::standardized_types::base_data::quotebar::QuoteBar;
use crate::standardized_types::base_data::tick::Tick;
use crate::standardized_types::base_data::traits::BaseData;

/// Manages all subscriptions for a strategy. each strategy has its own subscription handler.
pub struct SubscriptionHandler {
    /// Manages the consolidators of specific symbols
    symbol_subscriptions: Arc<DashMap<Symbol, SymbolSubscriptionHandler>>,
    /// fundamental data is not consolidated and so it does not need special handlers
    fundamental_subscriptions: Arc<RwLock<Vec<DataSubscription>>>,
    /// Keeps a record when the strategy has updated its subscriptions, so we can pause the backtest to fetch new data.
    is_warmed_up: AtomicBool,
    strategy_mode: StrategyMode,
    // subscriptions which the strategy actually subscribed to, not the raw data needed to full-fill the subscription.
    strategy_subscriptions: Arc<RwLock<Vec<DataSubscription>>>,
    primary_subscriptions_broadcaster: Arc<StaticInternalBroadcaster<Vec<DataSubscription>>>,
    candle_history: DashMap<DataSubscription, RollingWindow<Candle>>,
    bar_history: DashMap<DataSubscription, RollingWindow<QuoteBar>>,
    tick_history: DashMap<DataSubscription, RollingWindow<Tick>>,
    quote_history: DashMap<DataSubscription, RollingWindow<Quote>>,
    fundamental_history: DashMap<DataSubscription, RollingWindow<Fundamental>>,
    open_candles: DashMap<DataSubscription, Candle>,
    open_bars: DashMap<DataSubscription, QuoteBar>
}

impl SubscriptionHandler {
    pub async fn new(strategy_mode: StrategyMode) -> Self {
        let handler = SubscriptionHandler {
            fundamental_subscriptions: Default::default(),
            symbol_subscriptions: Default::default(),
            is_warmed_up: AtomicBool::new(false),
            strategy_mode,
            strategy_subscriptions: Default::default(),
            primary_subscriptions_broadcaster: Arc::new(StaticInternalBroadcaster::new()),
            candle_history: Default::default(),
            bar_history: Default::default(),
            tick_history: Default::default(),
            quote_history: Default::default(),
            fundamental_history: Default::default(),
            open_candles: Default::default(),
            open_bars: Default::default(),
        };
        handler
    }

    pub(crate) async fn subscribe_primary_subscription_updates(&self, name: String, sender: Sender<Vec<DataSubscription>>) {
        self.primary_subscriptions_broadcaster.subscribe(name, sender).await;
    }

    pub(crate) async fn unsubscribe_primary_subscription_updates(&self, name: String) {
        self.primary_subscriptions_broadcaster.unsubscribe(name).await;
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
        current_time: DateTime<Utc>,
        fill_forward: bool,
        history_to_retain: usize,
        broadcast: bool
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
            self.primary_subscriptions_broadcaster.broadcast(self.primary_subscriptions().await).await;
            return;
        }

        if !self.symbol_subscriptions.contains_key(&new_subscription.symbol) {
            let symbol_handler = SymbolSubscriptionHandler::new(
                new_subscription.symbol.clone(),
                self.is_warmed_up.load(Ordering::SeqCst),
            ).await;
            self.symbol_subscriptions.insert(new_subscription.symbol.clone(), symbol_handler);
            //println!("Handler: Subscribed: {}", new_subscription);
        }

        let windows = self.symbol_subscriptions.get(&new_subscription.symbol).unwrap()
            .subscribe(
                new_subscription.clone(),
                current_time,
                history_to_retain,
                self.strategy_mode,
                fill_forward
            )
            .await;

        if let Some(windows) = windows {
            for (subscription, window) in windows {
                match new_subscription.base_data_type {
                    BaseDataType::Ticks => {
                        self.tick_history.insert(subscription, RollingWindow::new(history_to_retain));
                    }
                    BaseDataType::Quotes => {
                        self.quote_history.insert(subscription, RollingWindow::new(history_to_retain));
                    }
                    BaseDataType::QuoteBars => {
                        self.bar_history.insert(subscription, RollingWindow::new(history_to_retain));
                    }
                    BaseDataType::Candles => {
                        self.candle_history.insert(subscription, RollingWindow::new(history_to_retain));
                    }
                    BaseDataType::Fundamentals => {
                        self.fundamental_history.insert(subscription, RollingWindow::new(history_to_retain));
                    }
                }
            }
        }
        if broadcast {
            self.primary_subscriptions_broadcaster.broadcast(self.primary_subscriptions().await).await;
        }
    }

    pub async fn set_subscriptions(
            &self,
            new_subscription: Vec<DataSubscription>,
            history_to_retain: usize,
            current_time: DateTime<Utc>,
            fill_forward: bool,
            broadcast: bool,
    ) {
        let current_subscriptions = self.subscriptions().await;
        for sub in current_subscriptions {
            if !new_subscription.contains(&sub) {
                self.unsubscribe(sub.clone(), false).await;
            }
        }
        for sub in new_subscription {
           self.subscribe(sub.clone(), current_time.clone(), fill_forward, history_to_retain, false).await;
        }
        if broadcast {
            self.primary_subscriptions_broadcaster.broadcast(self.primary_subscriptions().await).await;
        }
    }

    /// Unsubscribes from a data subscription
    /// 'subscription: DataSubscription' The subscription to unsubscribe from.
    /// 'current_time: DateTime<Utc>' The current time is used to change our base data subscription and warm up any new consolidators if we are adjusting our base resolution.
    /// 'strategy_mode: StrategyMode' The strategy mode is used to determine how to warm up the history, in live mode we may not yet have a serialized history to the current time.
    pub async fn unsubscribe(&self, subscription: DataSubscription, broadcast: bool) {
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
            self.primary_subscriptions_broadcaster.broadcast(self.primary_subscriptions().await).await;
            //println!("Handler: Unsubscribed: {}", subscription);
            return;
        }

        self.symbol_subscriptions.get(&subscription.symbol).unwrap().unsubscribe(&subscription).await;
        let mut strategy_subscriptions = self.strategy_subscriptions.write().await;
        strategy_subscriptions.retain(|x| x != &subscription);
        if self.symbol_subscriptions.get(&subscription.symbol).unwrap().active_count() == 0 {
            self.symbol_subscriptions.remove(&subscription.symbol);
        }
        match subscription.base_data_type {
            BaseDataType::Ticks => {
                self.tick_history.remove(&subscription);
            }
            BaseDataType::Quotes => {
                self.quote_history.remove(&subscription);
            }
            BaseDataType::QuoteBars => {
                self.bar_history.remove(&subscription);
            }
            BaseDataType::Candles => {
                self.candle_history.remove(&subscription);
            }
            BaseDataType::Fundamentals => {
                self.fundamental_history.remove(&subscription);
            }
        }
        if broadcast {
            self.primary_subscriptions_broadcaster.broadcast(self.primary_subscriptions().await).await;
        }
    }


    /// Returns all the primary subscriptions
    /// These are subscriptions that come directly from the vendors own data source.
    /// They are not consolidators, but are the primary source of data for the consolidators.
    pub async fn primary_subscriptions(&self) -> Vec<DataSubscription> {
        let mut primary_subscriptions = vec![];
        for symbol_handler in self.symbol_subscriptions.iter() {
            primary_subscriptions.extend(symbol_handler.value().primary_subscriptions().await);
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
    pub async fn update_time_slice(&self, time_slice: TimeSlice) -> Option<TimeSlice> {
        let symbol_subscriptions = self.symbol_subscriptions.clone();
        let mut open_bars: BTreeMap<DataSubscription, BaseDataEnum> = BTreeMap::new();
        let mut time_slice_bars = Vec::new();

        // Create a FuturesUnordered to collect all futures and run them concurrently.
        let mut update_futures = FuturesUnordered::new();

        for base_data in time_slice.iter() {
            let symbol = base_data.symbol();
            // let symbol_subscriptions = symbol_subscriptions.clone(); // Clone the Arc for each task.
            let base_data = base_data.clone(); // Clone base_data to avoid borrowing issues.

            let symbol_subscriptions = symbol_subscriptions.clone();
            // Add the future to the FuturesUnordered.
            update_futures.push(async move {
                // Get a read guard inside the async block to avoid lifetime issues.
                if let Some(handler) = symbol_subscriptions.get(&symbol) {
                    handler.update(&base_data).await
                } else {
                    println!("No handler: {:?}", symbol);
                    Vec::new() // Return empty if handler is not found.
                }
            });
        }

        // Process all the updates concurrently.
        while let Some(data) = update_futures.next().await {
            for consolidated_bars in data {
                if let Some(consolidated_bar) = consolidated_bars.closed_data {
                    time_slice_bars.push(consolidated_bar.clone());
                    let subscription = consolidated_bar.subscription();
                    match consolidated_bar {
                        BaseDataEnum::Tick(tick) => {
                            if let Some(mut rolling_window) = self.tick_history.get_mut(&subscription) {
                                rolling_window.add(tick);
                            }
                        }
                        BaseDataEnum::Quote(quote) => {
                            if let Some(mut rolling_window) = self.quote_history.get_mut(&subscription) {
                                rolling_window.add(quote);
                            }
                        }
                        BaseDataEnum::QuoteBar(qb) => {
                            if let Some(mut rolling_window) = self.bar_history.get_mut(&subscription) {
                                rolling_window.add(qb);
                            }
                        }
                        BaseDataEnum::Candle(candle) => {
                            if let Some(mut rolling_window) = self.candle_history.get_mut(&subscription) {
                                rolling_window.add(candle);
                            }
                        }
                        BaseDataEnum::Fundamental(fund) => {
                            if let Some(mut rolling_window) = self.fundamental_history.get_mut(&subscription) {
                                rolling_window.add(fund);
                            }
                        }
                    }
                }
                open_bars.insert(consolidated_bars.open_data.subscription(), consolidated_bars.open_data);
            }
        }

        // Combine open and closed bars.
        for (_, data) in open_bars {
            match &data {
                BaseDataEnum::Candle(ref candle) => {
                    self.open_candles.insert(data.subscription(), candle.clone());
                }
                BaseDataEnum::QuoteBar(ref qb) => {
                    self.open_bars.insert(data.subscription(), qb.clone());
                }
                _ => {}
            }
            time_slice_bars.push(data);
        }

        match time_slice_bars.is_empty() {
            true => None,
            false => Some(time_slice_bars)
        }
    }

    pub fn bar_history(&self, subscription: &DataSubscription) -> Option<RollingWindow<QuoteBar>> {
        if let Some(window) = self.bar_history.get(subscription) {
            return Some(window.value().clone())
        }
        None
    }

    pub fn candle_history(&self, subscription: &DataSubscription) -> Option<RollingWindow<Candle>> {
        if let Some(window) = self.candle_history.get(subscription) {
            return Some(window.value().clone())
        }
        None
    }

    pub fn tick_history(&self, subscription: &DataSubscription) -> Option<RollingWindow<Tick>> {
        if let Some(window) = self.tick_history.get(subscription) {
            return Some(window.value().clone())
        }
        None
    }

    pub fn quote_history(&self, subscription: &DataSubscription) -> Option<RollingWindow<Quote>> {
        if let Some(window) = self.quote_history.get(subscription) {
            return Some(window.value().clone())
        }
        None
    }

    pub fn open_bar(&self, subscription: &DataSubscription) -> Option<QuoteBar> {
        match self.open_bars.get(subscription) {
            None => None,
            Some(data) => Some(data.value().clone())
        }
    }

    pub fn open_candle(&self, subscription: &DataSubscription) -> Option<Candle> {
        match self.open_candles.get(subscription) {
            None => None,
            Some(data) => Some(data.value().clone())
        }
    }

    pub fn candle_index(&self, subscription: &DataSubscription, index: usize) -> Option<Candle> {
        if let Some(window) = self.candle_history.get(subscription) {
            return match window.get(index) {
                None => None,
                Some(data) => Some(data.clone())
            }
        }
        None
    }

    pub fn bar_index(&self, subscription: &DataSubscription, index: usize) -> Option<QuoteBar> {
        if let Some(window) = self.bar_history.get(subscription) {
            return match window.get(index) {
                None => None,
                Some(data) => Some(data.clone())
            }
        }
        None
    }

    pub fn tick_index(&self, subscription: &DataSubscription, index: usize) -> Option<Tick> {
        if let Some(window) = self.tick_history.get(subscription) {
            return match window.get(index) {
                None => None,
                Some(data) => Some(data.clone())
            }
        }
        None
    }

    pub fn quote_index(&self, subscription: &DataSubscription, index: usize) -> Option<Quote> {
        if let Some(window) = self.quote_history.get(subscription) {
            return match window.get(index) {
                None => None,
                Some(data) => Some(data.clone())
            }
        }
        None
    }

    pub async fn update_consolidators_time(&self, time: DateTime<Utc>) -> Option<TimeSlice> {
        let symbol_subscriptions = self.symbol_subscriptions.clone();
        let futures: Vec<_> = symbol_subscriptions.iter().map(|symbol_handler| {
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
                for consolidated_data in &data {
                    let subscription = consolidated_data.subscription();
                    match consolidated_data {
                        BaseDataEnum::Tick(ref tick) => {
                            if let Some(mut rolling_window) = self.tick_history.get_mut(&subscription) {
                                rolling_window.add(tick.clone());
                            }
                        }
                        BaseDataEnum::Quote(ref quote) => {
                            if let Some(mut rolling_window) = self.quote_history.get_mut(&subscription) {
                                rolling_window.add(quote.clone());
                            }
                        }
                        BaseDataEnum::QuoteBar(ref qb) => {
                            if let Some(mut rolling_window) = self.bar_history.get_mut(&subscription) {
                                rolling_window.add(qb.clone());
                            }
                        }
                        BaseDataEnum::Candle(ref candle) => {
                            if let Some(mut rolling_window) = self.candle_history.get_mut(&subscription) {
                                rolling_window.add(candle.clone());
                            }
                        }
                        BaseDataEnum::Fundamental(ref fund) => {
                            if let Some(mut rolling_window) = self.fundamental_history.get_mut(&subscription) {
                                rolling_window.add(fund.clone());
                            }
                        }
                    }
                }
                time_slice.extend(data);
            }
        }

        match time_slice.is_empty() {
            true => None,
            false => Some(time_slice)
        }
    }
}

/// This Struct Handles when to consolidate data for a subscription from an existing subscription.
/// Alternatively if a subscription is of a lower resolution subscription, then the new subscription becomes the primary data source and the existing subscription becomes the secondary data source.
/// depending if the vendor has data available in that resolution.
pub struct SymbolSubscriptionHandler {
    symbol: Symbol,
    /// The primary subscription is the subscription where data is coming directly from the `DataVendor`, In the event of bar data, it is pre-consolidated.
    primary_subscriptions: DashMap<SubscriptionResolutionType, DataSubscription>,
    /// The secondary subscriptions are consolidators that are used to consolidate data from the primary subscription. the first key is the primary subscription for each consolidator
    secondary_subscriptions: DashMap<SubscriptionResolutionType, AHashMap<DataSubscription, ConsolidatorEnum>>,
    subscription_event_buffer: RwLock<Vec<DataSubscriptionEvent>>,
    vendor_primary_resolutions: Vec<SubscriptionResolutionType>,
    vendor_data_types: Vec<BaseDataType>,
    is_warmed_up: AtomicBool,
}

impl SymbolSubscriptionHandler {
    pub async fn new(
        symbol: Symbol,
        is_warmed_up: bool,
    ) -> Self {
        let vendor_primary_resolutions = symbol.data_vendor.resolutions(symbol.market_type.clone()).await.unwrap();
        let vendor_data_types = symbol.data_vendor.base_data_types().await.unwrap();
        let handler = SymbolSubscriptionHandler {
            symbol,
            primary_subscriptions: DashMap::with_capacity(5),
            secondary_subscriptions: DashMap::with_capacity(5),
            subscription_event_buffer: RwLock::new(Vec::new()),
            is_warmed_up: AtomicBool::new(is_warmed_up),
            vendor_primary_resolutions,
            vendor_data_types
        };
 /*       for sub in initial_primary_subscriptions {
            if sub.base_data_type == BaseDataType::Fundamentals {
                continue;
            }
            handler.subscribe(sub, warm_up_to.clone(), strategy_mode.clone(), fill_forward.clone()).await;
        }*/
        handler
    }

    pub fn active_count(&self) -> usize {
        let mut count = 0;
        for map in self.secondary_subscriptions.iter() {
            for sub in map.value() {
                count += 1;
            }
        }
        count
    }

    pub async fn update(&self, base_data_enum: &BaseDataEnum) -> Vec<ConsolidatedData> {
         // Read the secondary subscriptions
        if self.secondary_subscriptions.is_empty() {
            return vec![];
        }

        let sub_res = SubscriptionResolutionType::new(base_data_enum.resolution().clone(), base_data_enum.base_data_type());
        if let Some(mut base_data_consoldiators) = self.secondary_subscriptions.get_mut(&sub_res){
            let mut data = vec![];
            for (_, consolidator) in base_data_consoldiators.iter_mut() {
                let consolidated_data = consolidator.update(&base_data_enum);
                data.push(consolidated_data);
            }
            return data
        }
        vec![]
    }

    pub async fn update_time(&self, time: DateTime<Utc>) -> Option<Vec<BaseDataEnum>> {
        let mut consolidated_data = vec![];
        // Iterate over the secondary subscriptions and update them
        for mut consolidator_map in self.secondary_subscriptions.iter_mut() {
            for (_, consolidator) in consolidator_map.iter_mut() {
                let data = consolidator.update_time(time.clone());
                consolidated_data.extend(data);
            }
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

    async fn subscribe(
        &self,
        new_subscription: DataSubscription,
        warm_up_to_time: DateTime<Utc>,
        history_to_retain: usize,
        strategy_mode: StrategyMode,
        fill_forward: bool
    ) -> Option<AHashMap<DataSubscription, RollingWindow<BaseDataEnum>>> {
        if new_subscription.base_data_type == BaseDataType::Fundamentals {
            return None;
        }
        let mut subscription_event_buffer = self.subscription_event_buffer.write().await;
        if let Some(subscription) = self.primary_subscriptions.get(&new_subscription.subscription_resolution_type()) {
            if *subscription.value() == new_subscription {
                let msg = format!("{}: Already subscribed: {}", new_subscription.symbol.data_vendor, new_subscription.symbol.name);
                subscription_event_buffer.push(DataSubscriptionEvent::FailedSubscribed(new_subscription, msg));
                return None
            }
        }
        if let Some(subscriptions) = self.secondary_subscriptions.get(&new_subscription.subscription_resolution_type()) {
            if let Some(subscription) = subscriptions.get(&new_subscription) {
                let msg = format!("{}: Already subscribed: {}", new_subscription.symbol.data_vendor, new_subscription.symbol.name);
                subscription_event_buffer.push(DataSubscriptionEvent::FailedSubscribed(new_subscription, msg));
                return None
            }
        }
        let is_warmed_up = self.is_warmed_up.load(Ordering::SeqCst);

        let mut returned_windows = AHashMap::new();
        let load_data_closure = |closure_subscription: &DataSubscription| -> Option<AHashMap<DataSubscription, RollingWindow<BaseDataEnum>>>{
            if is_warmed_up {
                let from_time = match closure_subscription.resolution == Resolution::Instant {
                    true => {
                        let subtract_duration: Duration = Duration::seconds(2) * history_to_retain as i32;
                        warm_up_to_time - subtract_duration - Duration::days(5)
                    }
                    false => {
                        let subtract_duration: Duration = closure_subscription.resolution.as_duration() * history_to_retain as i32;
                        warm_up_to_time - subtract_duration - Duration::days(5)
                    }
                };
                let primary_history = block_on(range_data(from_time, warm_up_to_time, closure_subscription.clone()));
                let mut history = RollingWindow::new(history_to_retain);
                for (_, slice) in primary_history {
                    for data in slice {
                        history.add(data);
                    }
                }
                returned_windows.insert(closure_subscription.clone(), history);
                return Some(returned_windows)
            }
            returned_windows.insert(closure_subscription.clone(), RollingWindow::new(history_to_retain));
            Some(returned_windows)
        };

        // we need to determine if the data vendor has this kind of primary data
        let is_primary_capable = self.vendor_primary_resolutions.contains(&new_subscription.subscription_resolution_type());
        if is_primary_capable && strategy_mode == StrategyMode::Backtest  {
            //In backtest mode we can just use the historical data so no need to reconsolidate
            self.primary_subscriptions.insert(new_subscription.subscription_resolution_type(), new_subscription.clone());
            subscription_event_buffer.push(DataSubscriptionEvent::Subscribed(new_subscription.clone()));
            return load_data_closure(&new_subscription)
        }

        let sub_res_type = new_subscription.subscription_resolution_type();
        // if the vendor doesn't supply this data we need to determine if we can atleast consolidate it from some source they do supply
        match new_subscription.base_data_type {
            BaseDataType::Ticks => {
                if !self.vendor_primary_resolutions.contains(&SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks)) {
                    let msg = format!("{}: Does not support this subscription: {}", new_subscription.symbol.data_vendor, new_subscription);
                    subscription_event_buffer.push(DataSubscriptionEvent::FailedSubscribed(new_subscription, msg));
                    return None
                } else {
                    if !self.primary_subscriptions.contains_key(&sub_res_type) {
                        self.primary_subscriptions.insert(sub_res_type.clone(), new_subscription.clone());
                    }
                    if !self.secondary_subscriptions.contains_key(&sub_res_type) {
                        self.secondary_subscriptions.insert(sub_res_type, AHashMap::new());
                    }
                    return load_data_closure(&new_subscription)
                }
            }
            BaseDataType::Quotes => {
                if !self.vendor_primary_resolutions.contains(&SubscriptionResolutionType::new(Resolution::Instant, BaseDataType::Quotes)) {
                    let msg = format!("{}: Does not support this subscription: {}", new_subscription.symbol.data_vendor, new_subscription);
                    subscription_event_buffer.push(DataSubscriptionEvent::FailedSubscribed(new_subscription.clone(), msg));
                    return None
                } else {
                    if !self.primary_subscriptions.contains_key(&sub_res_type) {
                        self.primary_subscriptions.insert(sub_res_type.clone(), new_subscription.clone());
                    }
                    if !self.secondary_subscriptions.contains_key(&sub_res_type) {
                        self.secondary_subscriptions.insert(sub_res_type, AHashMap::new());
                    }
                    return load_data_closure(&new_subscription)
                }
            }
            BaseDataType::QuoteBars | BaseDataType::Candles => {
                let ideal_subscription = match new_subscription.base_data_type {
                    BaseDataType::QuoteBars => {
                        if !self.vendor_primary_resolutions.contains(&SubscriptionResolutionType::new(Resolution::Instant, BaseDataType::Quotes)) && !!self.vendor_data_types.contains(&BaseDataType::QuoteBars) {
                            let msg = format!("{}: Does not support this subscription: {}", new_subscription.symbol.data_vendor, new_subscription);
                            subscription_event_buffer.push(DataSubscriptionEvent::FailedSubscribed(new_subscription, msg));
                            return None
                        }
                        SubscriptionResolutionType::new(Resolution::Instant, BaseDataType::Quotes)
                    }
                    BaseDataType::Candles => {
                        if !self.vendor_primary_resolutions.contains(&SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks)) && !self.vendor_primary_resolutions.contains(&SubscriptionResolutionType::new(Resolution::Instant, BaseDataType::Quotes)) && !!self.vendor_data_types.contains(&BaseDataType::Candles) {
                            let msg = format!("{}: Does not support this subscription: {}", new_subscription.symbol.data_vendor, new_subscription);
                            subscription_event_buffer.push(DataSubscriptionEvent::FailedSubscribed(new_subscription, msg));
                            return None
                        }
                        if self.vendor_primary_resolutions.contains(&SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks)) {
                            SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks)
                        } else {
                            if self.vendor_primary_resolutions.contains(&SubscriptionResolutionType::new(Resolution::Seconds(1), BaseDataType::Candles)) && new_subscription.resolution >= Resolution::Seconds(1) {
                                SubscriptionResolutionType::new(Resolution::Seconds(1), BaseDataType::Candles)
                            } else {
                                SubscriptionResolutionType::new(Resolution::Instant, BaseDataType::Quotes)
                            }
                        }
                    }
                    _ => panic!("This cant happen")
                };

                //if we don't have quotes we subscribe to the lowest possible resolution
                if !self.vendor_primary_resolutions.contains(&ideal_subscription) {
                    //if we haven't subscribe to quotebars to consolidate from we will need to
                    let mut has_lower_resolution = false;
                    let mut lowest_res = new_subscription.resolution;
                    for kind in &self.vendor_primary_resolutions {
                        if kind.resolution < new_subscription.resolution && kind.base_data_type == BaseDataType::QuoteBars {
                            has_lower_resolution = true;
                            if kind.resolution < lowest_res {
                                lowest_res = kind.resolution.clone()
                            }
                        }
                    }
                    if !has_lower_resolution {
                        match self.vendor_primary_resolutions.contains(&new_subscription.subscription_resolution_type()) {
                            true => {
                                self.primary_subscriptions.insert(new_subscription.subscription_resolution_type(), new_subscription.clone());
                                subscription_event_buffer.push(DataSubscriptionEvent::Subscribed(new_subscription.clone()));
                                return load_data_closure(&new_subscription)
                            }
                            false => {
                                let message = format!("{}: Does not have low enough resolution data to consolidate: {}", new_subscription.symbol.data_vendor, new_subscription);
                                subscription_event_buffer.push(DataSubscriptionEvent::FailedSubscribed(new_subscription, message));
                                return None
                            }
                        }
                    }
                    let lowest_possible_primary = DataSubscription::new(new_subscription.symbol.name.clone(), new_subscription.symbol.data_vendor.clone(), lowest_res, new_subscription.base_data_type.clone(), new_subscription.market_type.clone());
                    let primary_res_sub_type = lowest_possible_primary.subscription_resolution_type();
                    let mut returned_windows = AHashMap::new();
                    if !self.primary_subscriptions.contains_key(&primary_res_sub_type) {
                        self.primary_subscriptions.insert(primary_res_sub_type.clone(), lowest_possible_primary.clone());
                        if self.is_warmed_up.load(Ordering::SeqCst) {
                            let from_time = match lowest_possible_primary.resolution == Resolution::Instant {
                                true => {
                                    let subtract_duration: Duration = Duration::seconds(2) * history_to_retain as i32;
                                    warm_up_to_time - subtract_duration - Duration::days(5)
                                }
                                false => {
                                    let subtract_duration: Duration = lowest_possible_primary.resolution.as_duration() * history_to_retain as i32;
                                    warm_up_to_time - subtract_duration - Duration::days(5)
                                }
                            };
                            let primary_history = range_data(from_time, warm_up_to_time, lowest_possible_primary.clone()).await;
                            let mut history = RollingWindow::new(history_to_retain);
                            for (_, slice) in primary_history {
                                for data in slice {
                                    history.add(data);
                                }
                            }
                            returned_windows.insert(lowest_possible_primary.clone(), history);
                        } else {
                            returned_windows.insert(lowest_possible_primary.clone(), RollingWindow::new(history_to_retain));
                        }
                        if !self.secondary_subscriptions.contains_key(&primary_res_sub_type) {
                            self.secondary_subscriptions.insert(primary_res_sub_type.clone(), AHashMap::new());
                        }
                    }
                    let consolidator = ConsolidatorEnum::create_consolidator(new_subscription.clone(), fill_forward.clone()).await;
                    let (consolidator, window) = match is_warmed_up {
                        true => ConsolidatorEnum::warmup(consolidator, warm_up_to_time, history_to_retain as i32, strategy_mode).await,
                        false => (consolidator, RollingWindow::new(history_to_retain))
                    };
                    if let Some(mut map) = self.secondary_subscriptions.get_mut(&primary_res_sub_type) {
                        map.value_mut().insert(consolidator.subscription().clone(), consolidator);
                    }
                    returned_windows.insert(new_subscription.clone(), window);
                    subscription_event_buffer.push(DataSubscriptionEvent::Subscribed(new_subscription));
                    return Some(returned_windows)
                } else {
                    let mut returned_windows = AHashMap::new();
                    //if we have quotes we subscribe to quotes
                    let new_primary = DataSubscription::new(new_subscription.symbol.name.clone(), new_subscription.symbol.data_vendor.clone(), ideal_subscription.resolution, ideal_subscription.base_data_type, new_subscription.market_type.clone());
                    if !self.primary_subscriptions.contains_key(&ideal_subscription) {
                        self.primary_subscriptions.insert(new_primary.subscription_resolution_type(), new_primary.clone());
                        if is_warmed_up {
                            let from_time = match new_primary.resolution == Resolution::Instant {
                                true => {
                                    let subtract_duration: Duration = Duration::seconds(2) * history_to_retain as i32;
                                    warm_up_to_time - subtract_duration - Duration::days(5)
                                }
                                false => {
                                    let subtract_duration: Duration = new_primary.resolution.as_duration() * history_to_retain as i32;
                                    warm_up_to_time - subtract_duration - Duration::days(5)
                                }
                            };
                            let primary_history = range_data(from_time, warm_up_to_time, new_primary.clone()).await;
                            let mut history = RollingWindow::new(history_to_retain);
                            for (_, slice) in primary_history {
                                for data in slice {
                                    history.add(data);
                                }
                            }
                            returned_windows.insert(new_primary.clone(), history);
                        } else {
                            returned_windows.insert(new_primary.clone(), RollingWindow::new(history_to_retain));
                        }
                    }
                    let consolidator = ConsolidatorEnum::create_consolidator(new_subscription.clone(), fill_forward.clone()).await;
                    let (consolidator, window) = match is_warmed_up {
                        true => ConsolidatorEnum::warmup(consolidator, warm_up_to_time, history_to_retain as i32, strategy_mode).await,
                        false => (consolidator, RollingWindow::new(history_to_retain))
                    };
                    if let Some(mut consolidator_map) = self.secondary_subscriptions.get_mut(&new_primary.subscription_resolution_type()) {
                        consolidator_map.insert(new_subscription.clone(), consolidator);
                    } else {
                        let mut new_map = AHashMap::new();
                        new_map.insert(new_subscription.clone(), consolidator);
                        self.secondary_subscriptions.insert(new_primary.subscription_resolution_type(), new_map);
                    }
                    returned_windows.insert(new_subscription.clone(), window);
                    subscription_event_buffer.push(DataSubscriptionEvent::Subscribed(new_subscription.clone()));
                    return Some(returned_windows)
                }
            }
            BaseDataType::Fundamentals => {
                let msg = format!("{}: Does not support this subscription: {}", new_subscription.symbol.data_vendor, new_subscription);
                subscription_event_buffer.push(DataSubscriptionEvent::FailedSubscribed(new_subscription, msg));
                return None
            }
        }
    }

    async fn unsubscribe(&self, subscription: &DataSubscription) {
        let mut subscription_event_buffer = self.subscription_event_buffer.write().await;
        let sub_res_type = subscription.subscription_resolution_type();
        if self.primary_subscriptions.contains_key(&sub_res_type) {
            //determine if we have secondaries for this
            if let Some(mut map) = self.secondary_subscriptions.get_mut(&sub_res_type) {
                if map.is_empty() {
                    self.primary_subscriptions.remove(&sub_res_type);
                }
                self.secondary_subscriptions.remove(&sub_res_type);
            }
            subscription_event_buffer.push(DataSubscriptionEvent::Unsubscribed(subscription.clone()))
        } else if let Some(mut map) = self.secondary_subscriptions.get_mut(&sub_res_type) {
            let sub = map.remove(&subscription);
            match sub {
                None => subscription_event_buffer.push(DataSubscriptionEvent::FailedUnSubscribed(subscription.clone(), "No subscription to unsubscribe".to_string())),
                Some(sub) => subscription_event_buffer.push(DataSubscriptionEvent::Unsubscribed(subscription.clone())),
            }
        } else {
            subscription_event_buffer.push(DataSubscriptionEvent::Unsubscribed(subscription.clone()));
        }
    }

    pub async fn all_subscriptions(&self) -> Vec<DataSubscription> {
        // Collect primary subscriptions
        let mut all_subscriptions: Vec<DataSubscription> = self
            .primary_subscriptions
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

        // Collect secondary subscriptions by iterating over the inner AHashMap keys
        for entry in self.secondary_subscriptions.iter() {
            let secondary_subs = entry
                .value()
                .keys()
                .cloned() // Clone the keys (which are DataSubscriptions)
                .collect::<Vec<DataSubscription>>();

            all_subscriptions.extend(secondary_subs);
        }

        all_subscriptions
    }

    pub async fn primary_subscriptions(&self) -> Vec<DataSubscription> {
        self.primary_subscriptions.iter().map(|entry| entry.value().clone()).collect()
    }
}


