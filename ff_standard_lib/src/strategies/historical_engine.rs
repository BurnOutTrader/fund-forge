use std::collections::BTreeMap;
use std::sync::Arc;
use chrono::{DateTime, Duration as ChronoDuration, NaiveTime, TimeZone, Utc};
use crate::strategies::client_features::server_connections::{set_warmup_complete};
use crate::standardized_types::base_data::history::{get_compressed_historical_data};
use crate::standardized_types::enums::StrategyMode;
use crate::strategies::strategy_events::StrategyEvent;
use crate::standardized_types::time_slices::TimeSlice;
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use crate::standardized_types::subscriptions::DataSubscription;
use tokio::sync::{broadcast, mpsc, Notify};
use crate::strategies::handlers::indicator_handler::IndicatorHandler;
use crate::strategies::handlers::market_handler::backtest_matching_engine::BackTestEngineMessage;
use crate::strategies::handlers::market_handler::price_service::{get_price_service_sender, PriceServiceMessage};
use crate::strategies::handlers::subscription_handler::SubscriptionHandler;
use crate::strategies::handlers::timed_events_handler::TimedEventHandler;
use crate::strategies::historical_time::update_backtest_time;
use crate::strategies::ledgers::ledger_service::LedgerService;

#[allow(dead_code)]
pub(crate) struct HistoricalEngine {
    mode: StrategyMode,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    warmup_duration: ChronoDuration,
    buffer_resolution: Duration,
    gui_enabled: bool,
    primary_subscription_updates: broadcast::Receiver<Vec<DataSubscription>>,
    tick_over_no_data: bool,
    strategy_event_sender: mpsc::Sender<StrategyEvent>,
    notified: Arc<Notify>,
    historical_message_sender: Option<Sender<BackTestEngineMessage>>,
    ledger_service: Arc<LedgerService>,
    timed_event_handler: Arc<TimedEventHandler>,
    indicator_handler: Arc<IndicatorHandler>,
    subscription_handler: Arc<SubscriptionHandler>
}

// The date 2023-08-19 is in ISO week 33 of the year 2023
impl HistoricalEngine {
    pub(crate) async fn new(
        mode: StrategyMode,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        warmup_duration: ChronoDuration,
        buffer_resolution: Duration,
        gui_enabled: bool,
        tick_over_no_data: bool,
        strategy_event_sender: mpsc::Sender<StrategyEvent>,
        notified: Arc<Notify>,
        historical_message_sender: Option<Sender<BackTestEngineMessage>>,
        ledger_service: Arc<LedgerService>,
        timed_event_handler: Arc<TimedEventHandler>,
        indicator_handler: Arc<IndicatorHandler>,
        subscription_handler: Arc<SubscriptionHandler>
    ) -> Self {
        let rx = subscription_handler.subscribe_primary_subscription_updates();
        let engine = HistoricalEngine {
            mode,
            start_time: start_date,
            end_time: end_date,
            warmup_duration,
            buffer_resolution,
            gui_enabled,
            primary_subscription_updates: rx,
            tick_over_no_data,
            strategy_event_sender,
            notified,
            historical_message_sender,
            ledger_service,
            timed_event_handler,
            indicator_handler,
            subscription_handler
        };
        engine
    }

    /// Initializes the strategy, runs the warmup and then runs the strategy based on the mode.
    /// Calling this method will start the strategy running.
    pub async fn launch(mut self: Self) {
        if self.mode != StrategyMode::Backtest {
            panic!("Engine: Trying to launch backtest engine in live mode");
        }
        println!("Engine: Initializing the strategy...");
        thread::spawn(move|| {
            // Run the engine logic on a dedicated OS thread
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let warm_up_start_time = self.start_time - self.warmup_duration;
                let end_time = match self.mode {
                    StrategyMode::Backtest => self.end_time,
                    StrategyMode::Live | StrategyMode::LivePaperTrading => self.start_time
                };

                match self.mode {
                    StrategyMode::Backtest => self.historical_data_feed(warm_up_start_time, end_time, self.buffer_resolution, self.mode).await,
                    StrategyMode::Live | StrategyMode::LivePaperTrading  => panic!("Incorrect engine for Live modes"),
                }

                match self.mode {
                    StrategyMode::Backtest => {
                        let event = StrategyEvent::ShutdownEvent("Backtest Complete".to_string());
                        match self.strategy_event_sender.send(event).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("Historical Engine: Failed to send event: {}", e)
                        }
                    }
                    _ => {
                        panic!("Incorrect engine for Live modes")
                    }
                }
            });
        });
    }

    /// Feeds the historical data to the strategy, along with any events that were created.
    /// Simulates trading with a live buffer, where we catch events for x duration before forwarding to the strategy
    #[allow(unused_assignments)]
    async fn historical_data_feed(
        &mut self,
        warm_up_start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        buffer_duration: Duration,
        mode: StrategyMode,
    ) {
        println!("Historical Engine: Warming up the strategy...");
        let market_price_sender = get_price_service_sender();
        // here we are looping through 1 day at a time, if the strategy updates its subscriptions we will stop the data feed, download the historical data again to include updated symbols, and resume from the next time to be processed.
        let mut warm_up_complete = false;
        let mut primary_subscriptions = loop {
            let subscriptions = self.subscription_handler.primary_subscriptions().await;
            if !subscriptions.is_empty() {
                break subscriptions;
            }
            println!("Historical Engine: Waiting for primary subscriptions...");
            tokio::time::sleep(Duration::from_millis(200)).await;
        };

        for subscription in &primary_subscriptions {
            println!("Historical Engine: Primary Subscription: {}", subscription);
        }
        let strategy_subscriptions = self.subscription_handler.strategy_subscriptions().await;
        for subscription in &strategy_subscriptions {
            println!("Historical Engine: Strategy Subscription: {}", subscription);
        }
        let mut last_time = warm_up_start_time.clone();
        let mut first_iteration = true;
        'main_loop: while last_time <= end_time {
            if !first_iteration {
                last_time += Duration::from_nanos(1)
            }
            let to_time: DateTime<Utc> = {
                let end_of_day_naive = last_time.date_naive().and_time(NaiveTime::from_hms_nano_opt(23, 59, 59, 999_999_999).unwrap());
                Utc.from_utc_datetime(&end_of_day_naive).max(last_time)
            };
            if first_iteration {
                first_iteration = false;
            }
            let mut time_slices = match get_compressed_historical_data(primary_subscriptions.clone(), last_time.clone(), to_time).await {
                Ok(time_slices) => {
                    if time_slices.is_empty() && self.tick_over_no_data {
                        println!("Historical Engine: No data period, weekend or holiday: ticking through at buffering resolution, data will resume shortly");
                    } else if time_slices.is_empty() && !self.tick_over_no_data {
                        last_time = to_time + buffer_duration;
                        continue 'main_loop
                    }
                    time_slices
                },
                Err(e) => {
                    if self.tick_over_no_data {
                        println!("Historical Engine: Error getting data: {}", e);
                    } else if !self.tick_over_no_data {
                        last_time = to_time + buffer_duration;
                        continue 'main_loop
                    }
                    BTreeMap::new()
                }
            };

            //eprintln!("Time Slices: {:?}", time_slices);

            let mut time = last_time;
            'day_loop: while time <= to_time {
                time += buffer_duration;
                if !warm_up_complete {
                    if time >= self.start_time {
                        warm_up_complete = true;
                        set_warmup_complete();
                        let event = StrategyEvent::WarmUpComplete;
                        match self.strategy_event_sender.send(event).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("Historical Engine: Failed to send event: {}", e)
                        }
                        if mode == StrategyMode::Live || mode == StrategyMode::LivePaperTrading {
                            break 'main_loop
                        }
                        println!("Historical Engine: Start Backtest");
                    }
                }

                // we interrupt if we have a new subscription event so we can fetch the correct data, we will resume from the last time processed.
                match self.primary_subscription_updates.try_recv() {
                    Ok(updates) => {
                        if updates != primary_subscriptions {
                            primary_subscriptions = updates;
                            break 'day_loop
                        }
                    }
                    Err(_) => {}
                }
                update_backtest_time(time);
                self.timed_event_handler.update_time(time.clone()).await;

                let time_range = last_time.timestamp_nanos_opt().unwrap()..=time.timestamp_nanos_opt().unwrap();
                let mut time_slice: TimeSlice = TimeSlice::new();

                // Extract and remove data points in this range
                let keys_to_remove: Vec<i64> = time_slices
                    .range(time_range)
                    .map(|(k, _)| *k)
                    .collect();

                for key in keys_to_remove {
                    if let Some(data) = time_slices.remove(&key) {
                        time_slice.extend(data);
                    }
                }

                let mut strategy_time_slice: TimeSlice = TimeSlice::new();
                // update our consolidators and create the strategies time slice with any new data or just create empty slice.
                if !time_slice.is_empty() {
                    let arc_slice = Arc::new(time_slice.clone());
                    match market_price_sender.send(PriceServiceMessage::TimeSliceUpdate(arc_slice.clone())).await {
                        Ok(_) => {}
                        Err(e) => panic!("Market Handler: Error sending backtest message: {}", e)
                    }
                    self.ledger_service.timeslice_updates(arc_slice.clone()).await;

                    // Add only primary data which the strategy has subscribed to into the strategies time slice
                    if let Some(consolidated_data) = self.subscription_handler.update_time_slice(arc_slice.clone()).await {
                        strategy_time_slice.extend(consolidated_data);
                    }

                    strategy_time_slice.extend(time_slice);
                }

                if let Some(backtest_message_sender) = &self.historical_message_sender {
                    let message = BackTestEngineMessage::Time(time);
                    match backtest_message_sender.send(message).await {
                        Ok(_) => {}
                        Err(e) => panic!("Market Handler: Error sending backtest message: {}", e)
                    }
                    self.notified.notified().await;
                }

                // update the consolidators time and see if that generates new data, in case we didn't have primary data to update with.
                if let Some(consolidated_data) = self.subscription_handler.update_consolidators_time(time.clone()).await {
                    strategy_time_slice.extend(consolidated_data);
                }

                if !strategy_time_slice.is_empty() {
                    // Update indicators and get_requests any generated events.
                    if let Some(events) = self.indicator_handler.update_time_slice(&strategy_time_slice).await {
                        match self.strategy_event_sender.send(StrategyEvent::IndicatorEvent(events)).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("Historical Engine: Failed to send event: {}", e)
                        }
                    }

                    let slice_event = StrategyEvent::TimeSlice(
                        strategy_time_slice,
                    );
                    match self.strategy_event_sender.send(slice_event).await {
                        Ok(_) => {}
                        Err(e) => eprintln!("Historical Engine: Failed to send event: {}", e)
                    }
                }

                last_time = time.clone();
            }
        }
    }
}


