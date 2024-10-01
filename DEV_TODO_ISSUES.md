### Problems 
The static buffer is an issue, some events don't need to be buffered and cause deadlock when added after warm up
This issue is directly related to warming up subscriptions and indicators, somehow adding a buffer event to the public static buffer causes Mutex dead lock...
 - subscription events in subscription handler
 - Possibly indicator events

## Solutions
- All 3 fns are called directly by the strategy instance's associated fns, this is likely the cause of the deadlock on the buffer mutex,
The obvious solution is to just return the events to the calling functions and the strategy struct can add to the buffer after the functions have run.
- If above still deadlocks, send events directly to the strategy without buffering, subscription events are not time sensitive so synchronization is not an issue.. 
- these events are more of a debug issue and could possibly be moved to a simple `Result<>` return type... they are only considered strategy events for the sake of the strategy registry GUI connections.
- There is the option to just use the strategy sender directly and not buffer events, but then synchronization could become an issue.


## Problem fns

in indicator handler this fn causes deadlock on warm up of indicators if using add_buffer,  events should be returned to the strategy calling fn then added to the buffer
```rust
// Todo Indicator dead lock is caused when warming up indicators and adding buffer, might need this fn to return the event and add to buffer after fn is ran.
pub async fn add_indicator(&self, indicator: IndicatorEnum, time: DateTime<Utc>) {
        let subscription = indicator.subscription();

        if !self.indicators.contains_key(&subscription) {
            self.indicators.insert(subscription.clone(), DashMap::new());
        }

        let name = indicator.name().clone();

        let indicator = match is_warmup_complete() {
            true => warmup(time, self.strategy_mode.clone(), indicator).await,
            false => indicator,
        };

        if !self.subscription_map.contains_key(&name) {
            //add_buffer(time, StrategyEvent::IndicatorEvent(IndicatorEvents::IndicatorAdded(name.clone()))).await;
        } else {
           // add_buffer(time, StrategyEvent::IndicatorEvent(IndicatorEvents::Replaced(name.clone()))).await;
        }
        if let Some(map) = self.indicators.get(&subscription) {
            map.insert(indicator.name(), indicator);
        }
        self.subscription_map.insert(name.clone(), subscription.clone());
    }
```

In the subscription handler adding to buffer in this fn causes dead lock. events should be returned to the strategy calling fn then added to the buffer
```rust
pub async fn subscribe( //todo, when subscribing to data, if we already have a lower resolution subscribed for a symbol and it has a history window big enough to warm up, we could pass it in to speed warm ups
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
    } else {
        let msg = format!("{}: Already subscribed: {}", new_subscription.symbol.data_vendor, new_subscription.symbol.name);
        add_buffer(current_time, StrategyEvent::DataSubscriptionEvent(DataSubscriptionEvent::FailedToSubscribe(new_subscription.clone(), msg))).await;
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
        ).await;
        self.symbol_subscriptions.insert(new_subscription.symbol.clone(), symbol_handler);
        //println!("Handler: Subscribed: {}", new_subscription);
        //todo if have added a new symbol handler we need to register the symbol with the market handler
        let register_msg = MarketMessageEnum::RegisterSymbol(new_subscription.symbol.clone());
        self.market_event_sender.send(register_msg).await.unwrap();
    }

    let symbol_subscriptions = self.symbol_subscriptions.get(&new_subscription.symbol).unwrap();
    let windows = symbol_subscriptions.value().subscribe(
        current_time,
        new_subscription.clone(),
        current_time,
        history_to_retain,
        self.strategy_mode,
        fill_forward
    ).await;

    match windows {
        Ok(windows) => {
            for (subscription, window) in windows {
                //todo need to iter windows and get out the correct type of data
                match new_subscription.base_data_type {
                    BaseDataType::Ticks => {
                        self.tick_history.insert(subscription.clone(), RollingWindow::new(history_to_retain));
                        if let (mut tick_window) = self.tick_history.get_mut(&subscription).unwrap() {
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
                        if let (mut quote_window) = self.quote_history.get_mut(&subscription).unwrap() {
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
                        if let (mut bar_window) = self.bar_history.get_mut(&subscription).unwrap() {
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
                        if let (mut candle_window) = self.candle_history.get_mut(&subscription).unwrap() {
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
                        if let (mut fundamental_window) = self.fundamental_history.get_mut(&subscription).unwrap() {
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
            // this causes a dead-lock
            //add_buffer(current_time, StrategyEvent::DataSubscriptionEvent(DataSubscriptionEvent::Subscribed(new_subscription))).await;
        }
        Err(e) => {
            //add_buffer(current_time, StrategyEvent::DataSubscriptionEvent(DataSubscriptionEvent::FailedToSubscribe(new_subscription, e))).await;
        }
    }

    if broadcast {
        self.primary_subscriptions_broadcaster.broadcast(self.primary_subscriptions().await).await;
    }
}



```

#### Same problem as above if using add_buffer here
```rust
async fn subscribe(
        &self,
        current_time: DateTime<Utc>,
        new_subscription: DataSubscription,
        warm_up_to_time: DateTime<Utc>,
        history_to_retain: usize,
        strategy_mode: StrategyMode,
        fill_forward: bool
    ) -> Result<AHashMap<DataSubscription, RollingWindow<BaseDataEnum>>, String> {
        if new_subscription.base_data_type == BaseDataType::Fundamentals {
            return Err("Symbol handler does not handle Fundamental subcsriptions".to_string());
        }

        if let Some(subscription) = self.primary_subscriptions.get(&new_subscription.subscription_resolution_type()) {
            if *subscription.value() == new_subscription {
                return Err(format!("{}: Already subscribed: {}", new_subscription.symbol.data_vendor, new_subscription.symbol.name))
            }
        }
        if let Some(subscriptions) = self.secondary_subscriptions.get(&new_subscription.subscription_resolution_type()) {
            if let Some(_subscription) = subscriptions.get(&new_subscription) {
                return Err(format!("{}: Already subscribed: {}", new_subscription.symbol.data_vendor, new_subscription.symbol.name))
            }
        }
        let is_warmed_up =   is_warmup_complete();

        let mut returned_windows = AHashMap::new();
        let load_data_closure = |closure_subscription: &DataSubscription| -> Result<AHashMap<DataSubscription, RollingWindow<BaseDataEnum>>, String>{
            if is_warmed_up {
                let from_time = match closure_subscription.resolution == Resolution::Instant {
                    true => {
                        let subtract_duration: Duration = Duration::seconds(2) * history_to_retain as i32;
                        warm_up_to_time - subtract_duration - Duration::days(5)
                    }
                    false => {
                        let subtract_duration: Duration = closure_subscription.resolution.as_duration() * history_to_retain as i32;
                        warm_up_to_time - subtract_duration - Duration::days(2)
                    }
                };
                let primary_history = block_on(range_data(from_time, warm_up_to_time, closure_subscription.clone()));
                let mut history = RollingWindow::new(history_to_retain);
                for (_, slice) in primary_history {
                    for data in slice.iter() {
                        history.add(data.clone());
                    }
                }
                returned_windows.insert(closure_subscription.clone(), history);
                return Ok(returned_windows)
            }
            returned_windows.insert(closure_subscription.clone(), RollingWindow::new(history_to_retain));
            Ok(returned_windows)
        };

        // we need to determine if the data vendor has this kind of primary data
        let is_primary_capable = self.vendor_primary_resolutions.contains(&new_subscription.subscription_resolution_type());
        if is_primary_capable && strategy_mode == StrategyMode::Backtest  {
            //In backtest mode we can just use the historical data so no need to reconsolidate
            self.primary_subscriptions.insert(new_subscription.subscription_resolution_type(), new_subscription.clone());
            return load_data_closure(&new_subscription)
        }

        let sub_res_type = new_subscription.subscription_resolution_type();
        // if the vendor doesn't supply this data we need to determine if we can atleast consolidate it from some source they do supply
        match new_subscription.base_data_type {
            BaseDataType::Ticks => {
                if !self.vendor_primary_resolutions.contains(&SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks)) {
                        Err( format!("{}: Does not support this subscription: {}", new_subscription.symbol.data_vendor, new_subscription))
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
                    Err( format!("{}: Does not support this subscription: {}", new_subscription.symbol.data_vendor, new_subscription))
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
                            return Err( format!("{}: Does not support this subscription: {}", new_subscription.symbol.data_vendor, new_subscription))
                        }
                        SubscriptionResolutionType::new(Resolution::Instant, BaseDataType::Quotes)
                    }
                    BaseDataType::Candles => {
                        if !self.vendor_primary_resolutions.contains(&SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks)) && !self.vendor_primary_resolutions.contains(&SubscriptionResolutionType::new(Resolution::Instant, BaseDataType::Quotes)) && !!self.vendor_data_types.contains(&BaseDataType::Candles) {
                            return Err(format!("{}: Does not support this subscription: {}", new_subscription.symbol.data_vendor, new_subscription))
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
                                return load_data_closure(&new_subscription)
                            }
                            false => {
                                return Err(format!("{}: Does not have low enough resolution data to consolidate: {}", new_subscription.symbol.data_vendor, new_subscription))
                            }
                        }
                    }
                    let lowest_possible_primary = DataSubscription::new(new_subscription.symbol.name.clone(), new_subscription.symbol.data_vendor.clone(), lowest_res, new_subscription.base_data_type.clone(), new_subscription.market_type.clone());
                    let primary_res_sub_type = lowest_possible_primary.subscription_resolution_type();
                    let mut returned_windows = AHashMap::new();
                    if !self.primary_subscriptions.contains_key(&primary_res_sub_type) {
                        self.primary_subscriptions.insert(primary_res_sub_type.clone(), lowest_possible_primary.clone());
                        if is_warmed_up {
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
                                for data in slice.iter() {
                                    history.add(data.clone());
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
                    let consolidator = ConsolidatorEnum::create_consolidator(new_subscription.clone(), fill_forward.clone(), primary_res_sub_type).await;
                    let (consolidator, window) = match is_warmed_up {
                        true => ConsolidatorEnum::warmup(consolidator, warm_up_to_time, history_to_retain as i32, strategy_mode).await,
                        false => (consolidator, RollingWindow::new(history_to_retain))
                    };
                    if let Some(mut map) = self.secondary_subscriptions.get_mut(&primary_res_sub_type) {
                        map.value_mut().insert(consolidator.subscription().clone(), consolidator);
                    }
                    returned_windows.insert(new_subscription.clone(), window);
                    add_buffer(current_time, StrategyEvent::DataSubscriptionEvent(DataSubscriptionEvent::Subscribed(new_subscription.clone()))).await;
                    Ok(returned_windows)
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
                                for data in slice.iter() {
                                    history.add(data.clone());
                                }
                            }
                            returned_windows.insert(new_primary.clone(), history);
                        } else {
                            returned_windows.insert(new_primary.clone(), RollingWindow::new(history_to_retain));
                        }
                    }
                    let consolidator = ConsolidatorEnum::create_consolidator(new_subscription.clone(), fill_forward.clone(), new_primary.subscription_resolution_type()).await;
                    let (final_consolidator, window) = match is_warmed_up {
                        true => {
                            let (final_consolidator, window) = ConsolidatorEnum::warmup(consolidator, warm_up_to_time, history_to_retain as i32, strategy_mode).await;
                            (final_consolidator, window)
                        },
                        false => (consolidator, RollingWindow::new(history_to_retain))
                    };
                    self.secondary_subscriptions
                        .entry(new_primary.subscription_resolution_type())
                        .or_insert_with(AHashMap::new)
                        .insert(new_subscription.clone(), final_consolidator);
                    returned_windows.insert(new_subscription.clone(), window);
                    Ok(returned_windows)
                }
            }
            _ => panic!("This shouldnt be possible")
        }
    }
```