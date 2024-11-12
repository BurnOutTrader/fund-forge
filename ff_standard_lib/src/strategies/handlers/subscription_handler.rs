use std::collections::BTreeMap;
use std::ops::Deref;
use std::sync::{Arc};
use ahash::AHashMap;
use crate::strategies::consolidators::consolidator_enum::{ConsolidatedData, ConsolidatorEnum};
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::enums::{StrategyMode, PrimarySubscription};
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::{DataSubscription, DataSubscriptionEvent, Symbol};
use crate::standardized_types::time_slices::TimeSlice;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use tokio::sync::RwLock;
use crate::strategies::client_features::server_connections::{is_warmup_complete};
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::base_data::fundamental::Fundamental;
use crate::standardized_types::base_data::quote::Quote;
use crate::standardized_types::base_data::quotebar::QuoteBar;
use crate::standardized_types::base_data::tick::Tick;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::resolution::Resolution;
use crate::strategies::strategy_events::StrategyEvent;
use tokio::sync::broadcast;
use tokio::sync::mpsc::Sender;
use crate::standardized_types::base_data::history::{get_compressed_historical_data};

/// Manages all subscriptions for a strategy. each strategy has its own subscription handler.
pub struct SubscriptionHandler {
    /// Manages the consolidators of specific symbols
    symbol_subscriptions: Arc<DashMap<Symbol, SymbolSubscriptionHandler>>,
    /// fundamental data is not consolidated and so it does not need special handlers
    fundamental_subscriptions: Arc<RwLock<Vec<DataSubscription>>>,
    strategy_mode: StrategyMode,
    // subscriptions which the strategy actually subscribed to, not the raw data needed to full-fill the subscription.
    strategy_subscriptions: Arc<RwLock<Vec<DataSubscription>>>,
    primary_subscriptions_broadcaster: tokio::sync::broadcast::Sender<Vec<DataSubscription>>,
    candle_history: DashMap<DataSubscription, RollingWindow<Candle>>,
    bar_history: DashMap<DataSubscription, RollingWindow<QuoteBar>>,
    tick_history: DashMap<DataSubscription, RollingWindow<Tick>>,
    quote_history: DashMap<DataSubscription, RollingWindow<Quote>>,
    fundamental_history: DashMap<DataSubscription, RollingWindow<Fundamental>>,
    open_candles: DashMap<DataSubscription, Candle>,
    open_bars: DashMap<DataSubscription, QuoteBar>,
    strategy_event_sender: Sender<StrategyEvent>
}

impl SubscriptionHandler {
    pub async fn new(strategy_mode: StrategyMode, strategy_event_sender: Sender<StrategyEvent>) -> Self {
        let (tx, _) = broadcast::channel(16);
        SubscriptionHandler {
            strategy_event_sender,
            fundamental_subscriptions: Default::default(),
            symbol_subscriptions: Default::default(),
            strategy_mode,
            strategy_subscriptions: Default::default(),
            primary_subscriptions_broadcaster: tx,
            candle_history: Default::default(),
            bar_history: Default::default(),
            tick_history: Default::default(),
            quote_history: Default::default(),
            fundamental_history: Default::default(),
            open_candles: Default::default(),
            open_bars: Default::default(),
        }
    }

    pub(crate) fn subscribe_primary_subscription_updates(&self) -> broadcast::Receiver<Vec<DataSubscription>> {
        self.primary_subscriptions_broadcaster.subscribe()
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
        primary_source: Option<PrimarySubscription>,
        new_subscription: DataSubscription,
        current_time: DateTime<Utc>,
        fill_forward: bool,
        history_to_retain: usize,
        broadcast: bool
    ) {
        let mut strategy_subscriptions = self.strategy_subscriptions.write().await;
        if !strategy_subscriptions.contains(&new_subscription) {
            strategy_subscriptions.push(new_subscription.clone());
        } else {
            let msg = format!("{}: Already subscribed: {}", new_subscription.symbol.data_vendor, new_subscription.symbol.name);
            let event = DataSubscriptionEvent::FailedToSubscribe(new_subscription.clone(), msg);
            let _ = self.strategy_event_sender.send(StrategyEvent::DataSubscriptionEvent(event)).await;
        }

        let _ = strategy_subscriptions.deref();
        if new_subscription.base_data_type == BaseDataType::Fundamentals {
            //subscribe to fundamental
            let mut fundamental_subscriptions = self.fundamental_subscriptions.write().await;
            if !fundamental_subscriptions.contains(&new_subscription) {
                fundamental_subscriptions.push(new_subscription.clone());
            }
            let subscriptions = self.primary_subscriptions().await;
            match self.primary_subscriptions_broadcaster.send(subscriptions) {
                Ok(_) => {}
                Err(_) => {}
            }
            let event = DataSubscriptionEvent::Subscribed(new_subscription.clone());
            let _ = self.strategy_event_sender.send(StrategyEvent::DataSubscriptionEvent(event)).await;
        }

        if !self.symbol_subscriptions.contains_key(&new_subscription.symbol) {
            let symbol_handler = SymbolSubscriptionHandler::new(
                new_subscription.symbol.clone(),
            ).await;
            self.symbol_subscriptions.insert(new_subscription.symbol.clone(), symbol_handler);
        }

        let symbol_subscriptions = self.symbol_subscriptions.get(&new_subscription.symbol).unwrap();
        let windows = symbol_subscriptions.value().subscribe(
                primary_source,
                new_subscription.clone(),
                current_time,
                history_to_retain,
                self.strategy_mode,
                fill_forward
            ).await;

        match windows {
            Ok(windows) => {
                for (subscription, window) in windows {
                    //todo need to iter windows and get_requests out the correct type of data
                    match new_subscription.base_data_type {
                        BaseDataType::Ticks => {
                            self.tick_history.insert(subscription.clone(), RollingWindow::new(history_to_retain));
                            if let Some(mut tick_window) = self.tick_history.get_mut(&subscription) {
                                for data in window.history {
                                    match data {
                                        BaseDataEnum::Tick(tick) => tick_window.value_mut().add(tick),
                                        _ => {}
                                    }
                                }
                            }
                        }
                        BaseDataType::Quotes => {
                            self.quote_history.insert(subscription.clone(), RollingWindow::new(history_to_retain));
                            if let Some(mut quote_window) = self.quote_history.get_mut(&subscription) {
                                for data in window.history {
                                    match data {
                                        BaseDataEnum::Quote(quote) => quote_window.value_mut().add(quote),
                                        _ => {}
                                    }
                                }
                            }
                        }
                        BaseDataType::QuoteBars => {
                            self.bar_history.insert(subscription.clone(), RollingWindow::new(history_to_retain));
                            if let Some(mut bar_window) = self.bar_history.get_mut(&subscription) {
                                for data in window.history {
                                    match data {
                                        BaseDataEnum::QuoteBar(quote) => bar_window.value_mut().add(quote),
                                        _ => {}
                                    }
                                }
                            }
                        }
                        BaseDataType::Candles => {
                            self.candle_history.insert(subscription.clone(), RollingWindow::new(history_to_retain));
                            if let Some(mut candle_window) = self.candle_history.get_mut(&subscription) {
                                for data in window.history {
                                    match data {
                                        BaseDataEnum::Candle(candle) => candle_window.value_mut().add(candle),
                                        _ => {}
                                    }
                                }
                            }
                        }
                        BaseDataType::Fundamentals => {
                            self.fundamental_history.insert(subscription.clone(), RollingWindow::new(history_to_retain));
                            if let Some(mut fundamental_window) = self.fundamental_history.get_mut(&subscription) {
                                for data in window.history {
                                    match data {
                                        BaseDataEnum::Fundamental(funda) => fundamental_window.value_mut().add(funda),
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
                if broadcast {
                    let subscriptions = self.primary_subscriptions().await;
                    match self.primary_subscriptions_broadcaster.send(subscriptions) {
                        Ok(_) => {}
                        Err(_) => {}
                    }
                }
                let event = DataSubscriptionEvent::Subscribed(new_subscription.clone());
                let _ = self.strategy_event_sender.send(StrategyEvent::DataSubscriptionEvent(event)).await;
            }
            Err(e) => {
                let _ = self.strategy_event_sender.send(StrategyEvent::DataSubscriptionEvent(e)).await;
            }
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
            let subscriptions = self.primary_subscriptions().await;
            match self.primary_subscriptions_broadcaster.send(subscriptions) {
                Ok(_) => {}
                Err(_) => {}
            }
            //println!("Handler: Unsubscribed: {}", subscription);
            return;
        }

        let event =  self.symbol_subscriptions.get(&subscription.symbol).unwrap().unsubscribe(&subscription).await;
        let _ = self.strategy_event_sender.send(StrategyEvent::DataSubscriptionEvent(event)).await;

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
            let subscriptions = self.primary_subscriptions().await;
            match self.primary_subscriptions_broadcaster.send(subscriptions) {
                Ok(_) => {}
                Err(_) => {}
            }
        }
    }


    /// Returns all the primary subscriptions
    /// These are subscriptions that come directly from the vendors own data source.
    /// They are not consolidators, but are the primary source of data for the consolidators.
    pub async fn primary_subscriptions(&self) -> Vec<DataSubscription> {
        let mut primary_subscriptions = vec![];
        for symbol_handler in self.symbol_subscriptions.iter() {
            primary_subscriptions.extend(symbol_handler.value().primary_subscriptions());
        }
        let fundamentals = self.fundamental_subscriptions.read().await.clone();
        if !fundamentals.is_empty() {
            primary_subscriptions.extend(fundamentals);
        }
        primary_subscriptions
    }

    /// Returns all the subscriptions including primary and consolidators
    pub async fn subscriptions(&self) -> Vec<DataSubscription> {
        let mut all_subscriptions = vec![];
        for symbol_handler in self.symbol_subscriptions.iter() {
            all_subscriptions.append(&mut symbol_handler.value().all_subscriptions());
        }
        all_subscriptions.extend(self.fundamental_subscriptions.read().await.clone());
        all_subscriptions
    }

    /// Updates any consolidators with primary data
    pub async fn update_time_slice(&self, time_slice: Arc<TimeSlice>) -> Option<TimeSlice> {
        let symbol_subscriptions = self.symbol_subscriptions.clone();
        let mut open_bars: BTreeMap<DataSubscription, BaseDataEnum> = BTreeMap::new();
        let mut time_slice_bars = TimeSlice::new();

        let mut update_futures = FuturesUnordered::new();

        for base_data in time_slice.iter() {
            let symbol = base_data.symbol();
            let base_data = base_data.clone();
            let symbol_subscriptions = symbol_subscriptions.clone();
            match &base_data {
                BaseDataEnum::Candle(candle) => {
                    if let Some(mut history) = self.candle_history.get_mut(&candle.subscription()) {
                        history.add(candle.clone());
                    }
                }
                BaseDataEnum::QuoteBar(qb) => {
                    if let Some(mut history) = self.bar_history.get_mut(&qb.subscription()) {
                        history.add(qb.clone());
                    }
                }
                BaseDataEnum::Tick(tick) => {
                    if let Some(mut history) = self.tick_history.get_mut(&tick.subscription()) {
                        history.add(tick.clone());
                    }
                }
                BaseDataEnum::Quote(q) => {
                    if let Some(mut history) = self.quote_history.get_mut(&q.subscription()) {
                        history.add(q.clone());
                    }
                }
                BaseDataEnum::Fundamental(_) => {}
            }

            update_futures.push(async move {
                if let Some(handler) = symbol_subscriptions.get(&symbol) {
                    handler.update(&base_data).await
                } else {
                    Vec::new()
                }
            });
        }

        let mut all_bars: BTreeMap<(DataSubscription, DateTime<Utc>), BaseDataEnum> = BTreeMap::new();
        while let Some(data) = update_futures.next().await {
            for consolidated_bars in data {
                if let Some(consolidated_bar) = consolidated_bars.closed_data {
                    let key = (consolidated_bar.subscription(), consolidated_bar.time_utc());
                    all_bars.entry(key).or_insert(consolidated_bar);
                }
                let open_key = consolidated_bars.open_data.subscription();
                open_bars.entry(open_key).or_insert(consolidated_bars.open_data);
            }
        }

        for ((subscription, _), data) in all_bars {
            match &data {
                BaseDataEnum::Tick(tick) => {
                    if let Some(mut rolling_window) = self.tick_history.get_mut(&subscription) {
                        rolling_window.add(tick.clone());
                    }
                }
                BaseDataEnum::Quote(quote) => {
                    if let Some(mut rolling_window) = self.quote_history.get_mut(&subscription) {
                        rolling_window.add(quote.clone());
                    }
                }
                BaseDataEnum::QuoteBar(qb) => {
                    if let Some(mut rolling_window) = self.bar_history.get_mut(&subscription) {
                        rolling_window.add(qb.clone());
                    }
                }
                BaseDataEnum::Candle(candle) => {
                    if let Some(mut rolling_window) = self.candle_history.get_mut(&subscription) {
                        rolling_window.add(candle.clone());
                    }
                }
                BaseDataEnum::Fundamental(fund) => {
                    if let Some(mut rolling_window) = self.fundamental_history.get_mut(&subscription) {
                        rolling_window.add(fund.clone());
                    }
                }
            }
            time_slice_bars.add(data);
        }

        for (subscription, data) in open_bars {
            match &data {
                BaseDataEnum::Candle(candle) => {
                    self.open_candles.insert(subscription.clone(), candle.clone());
                }
                BaseDataEnum::QuoteBar(qb) => {
                    self.open_bars.insert(subscription.clone(), qb.clone());
                }
                _ => {}
            }
            time_slice_bars.add(data);
        }

        if time_slice_bars.is_empty() {
            None
        } else {
            Some(time_slice_bars)
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

    //todo need a live version of this, where we record which consolidators had data and which didnt, we update time for thise that didn't
    pub async fn update_consolidators_time(&self, time: DateTime<Utc>) -> Option<TimeSlice> {
        let symbol_subscriptions = self.symbol_subscriptions.clone();
        let futures: Vec<_> = symbol_subscriptions.iter().map(|symbol_handler| {
            let time = time.clone();
            // Creating async blocks that will run concurrently
            async move {
                symbol_handler.value().update_time(time).await
            }
        }).collect();

        // Execute all cme_futures concurrently
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
                for base_data in data {
                    time_slice.add(base_data);
                }
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
    /// The primary subscription is the subscription where data is coming directly from the `DataVendor`, In the event of bar data, it is pre-consolidated.
    primary_subscriptions: DashMap<PrimarySubscription, DataSubscription>,
    /// The secondary subscriptions are consolidators that are used to consolidate data from the primary subscription. the first key is the primary subscription for each consolidator
    secondary_subscriptions: DashMap<PrimarySubscription, AHashMap<DataSubscription, ConsolidatorEnum>>,
    vendor_primary_resolutions: Vec<PrimarySubscription>,
}

impl SymbolSubscriptionHandler {
    pub async fn new(
        symbol: Symbol,
    ) -> Self {
        let vendor_primary_resolutions = symbol.data_vendor.resolutions(symbol.market_type.clone()).await.unwrap();
        let handler = SymbolSubscriptionHandler {
            primary_subscriptions: DashMap::new(),
            secondary_subscriptions: DashMap::new(),
            vendor_primary_resolutions,
        };
        handler
    }

    pub fn active_count(&self) -> usize {
        let mut count = 0;
        for map in self.secondary_subscriptions.iter() {
            for _sub in map.value() {
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

        let sub_res = PrimarySubscription::new(base_data_enum.resolution().clone(), base_data_enum.base_data_type());
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

    async fn subscribe(
        &self,
        primary_source: Option<PrimarySubscription>,
        new_subscription: DataSubscription,
        warm_up_to_time: DateTime<Utc>,
        history_to_retain: usize,
        strategy_mode: StrategyMode,
        fill_forward: bool
    ) -> Result<AHashMap<DataSubscription, RollingWindow<BaseDataEnum>>, DataSubscriptionEvent> {
        if new_subscription.base_data_type == BaseDataType::Fundamentals {
            return Err(DataSubscriptionEvent::FailedToSubscribe(new_subscription, "Symbol handler does not handle Fundamental subscriptions".to_string()));
        }

        if let Some(subscription) = self.primary_subscriptions.get(&new_subscription.subscription_resolution_type()) {
            if *subscription.value() == new_subscription {
                return Err(DataSubscriptionEvent::FailedToSubscribe(new_subscription.clone(), format!("{}: Already subscribed: {}", new_subscription.symbol.data_vendor, new_subscription.symbol.name)))
            }
        }

        if let Some(subscriptions) = self.secondary_subscriptions.get(&new_subscription.subscription_resolution_type()) {
            if let Some(_subscription) = subscriptions.get(&new_subscription) {
                return Err(DataSubscriptionEvent::FailedToSubscribe(new_subscription.clone(), format!("{}: Already subscribed: {}", new_subscription.symbol.data_vendor, new_subscription.symbol.name)))
            }
        }
        let is_warmed_up =   is_warmup_complete();

        let mut returned_windows = AHashMap::new();
        if let Some(primary) = primary_source {
            if !self.vendor_primary_resolutions.contains(&primary) {
                return Err(DataSubscriptionEvent::FailedToSubscribe(new_subscription.clone(), format!("{}: Does not support this subscription: {}", new_subscription.symbol.data_vendor, new_subscription)))
            }
            if !self.primary_subscriptions.contains_key(&primary) {
                if !self.primary_subscriptions.contains_key(&primary) {
                    let new_primary = DataSubscription::new(new_subscription.symbol.name.clone(), new_subscription.symbol.data_vendor.clone(), primary.resolution, primary.base_data_type, new_subscription.market_type.clone());
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
                        let data = get_compressed_historical_data(vec![new_primary.clone()], from_time, warm_up_to_time).await.unwrap_or_else(|_e| BTreeMap::new());
                        let mut history = RollingWindow::new(history_to_retain);
                        for (_, slice) in data {
                            for data in slice.iter() {
                                history.add(data.clone());
                            }
                        }
                        returned_windows.insert(new_primary.clone(), history);
                    } else {
                        returned_windows.insert(new_primary.clone(), RollingWindow::new(history_to_retain));
                    }
                }
            }
            if !self.secondary_subscriptions.contains_key(&primary) {
                let consolidator = ConsolidatorEnum::create_consolidator(new_subscription.clone(), fill_forward.clone()).await;
                let (final_consolidator, window) = match is_warmed_up {
                    true => {
                        let (final_consolidator, window) = ConsolidatorEnum::warmup(consolidator, warm_up_to_time, history_to_retain as i32, strategy_mode).await;
                        (final_consolidator, window)
                    },
                    false => (consolidator, RollingWindow::new(history_to_retain))
                };
                self.secondary_subscriptions
                    .entry(primary)
                    .or_insert_with(AHashMap::new)
                    .insert(new_subscription.clone(), final_consolidator);
                returned_windows.insert(new_subscription.clone(), window);
            }
            Ok(returned_windows)
        } else {
            if !self.primary_subscriptions.contains_key(&new_subscription.subscription_resolution_type()) {
                self.primary_subscriptions.insert(new_subscription.subscription_resolution_type(), new_subscription.clone());
                if is_warmed_up {
                    let from_time = match new_subscription.resolution == Resolution::Instant {
                        true => {
                            let subtract_duration: Duration = chrono::Duration::seconds(2) * history_to_retain as i32;
                            warm_up_to_time - subtract_duration - Duration::days(5)
                        }
                        false => {
                            let subtract_duration: Duration = new_subscription.resolution.as_duration() * history_to_retain as i32;
                            warm_up_to_time - subtract_duration - Duration::days(5)
                        }
                    };
                    let data = get_compressed_historical_data(vec![new_subscription.clone()], from_time, warm_up_to_time).await.unwrap_or_else(|_e| BTreeMap::new());
                    let mut history = RollingWindow::new(history_to_retain);
                    for (_, slice) in data {
                        for data in slice.iter() {
                            history.add(data.clone());
                        }
                    }
                    returned_windows.insert(new_subscription.clone(), history);
                } else {
                    returned_windows.insert(new_subscription.clone(), RollingWindow::new(history_to_retain));
                }
            }
            Ok(returned_windows)
        }
    }

    async fn unsubscribe(&self, subscription: &DataSubscription) -> DataSubscriptionEvent {
        let sub_res_type = subscription.subscription_resolution_type();
        if self.primary_subscriptions.contains_key(&sub_res_type) {
            //determine if we have secondaries for this
            if let Some(map) = self.secondary_subscriptions.get(&sub_res_type) {
                if map.is_empty() {
                    self.primary_subscriptions.remove(&sub_res_type);
                }
                self.secondary_subscriptions.remove(&sub_res_type);
            }
            DataSubscriptionEvent::Unsubscribed(subscription.clone())
        } else if let Some(mut map) = self.secondary_subscriptions.get_mut(&sub_res_type) {
            let sub = map.remove(&subscription);
            match sub {
                None => DataSubscriptionEvent::FailedUnSubscribed(subscription.clone(), "No subscription to unsubscribe".to_string()),
                Some(_consolidator) => DataSubscriptionEvent::Unsubscribed(subscription.clone()),
            }
        } else {
            DataSubscriptionEvent::Unsubscribed(subscription.clone())
        }
    }

    pub fn all_subscriptions(&self) -> Vec<DataSubscription> {
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

    pub fn primary_subscriptions(&self) -> Vec<DataSubscription> {
        self.primary_subscriptions.iter().map(|entry| entry.value().clone()).collect()
    }
}


