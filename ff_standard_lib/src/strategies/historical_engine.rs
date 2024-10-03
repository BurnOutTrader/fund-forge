use chrono::{DateTime, Datelike, Utc,  Duration as ChronoDuration};
use crate::strategies::client_features::server_connections::{set_warmup_complete, SUBSCRIPTION_HANDLER, INDICATOR_HANDLER, subscribe_primary_subscription_updates, unsubscribe_primary_subscription_updates, add_buffer, forward_buffer};
use crate::standardized_types::base_data::history::{
    generate_file_dates, get_historical_data,
};
use crate::messages::data_server_messaging::FundForgeError;
use crate::standardized_types::enums::StrategyMode;
use crate::strategies::strategy_events::{StrategyEvent};
use crate::standardized_types::time_slices::TimeSlice;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, Notify};
use crate::strategies::handlers::market_handlers::MarketMessageEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::historical_time::{get_backtest_time, update_backtest_time};
//Possibly more accurate engine
/*todo Use this for saving and loading data, it will make smaller file sizes and be less handling for consolidator, we can then just update historical data once per week on sunday and load last week from broker.
  use Create a date (you can use DateTime<Utc>, Local, or NaiveDate)
  let date = Utc.ymd(2023, 8, 19); // Year, Month, Date
  let iso_week = date.iso_week();
  let week_number = iso_week.week(); // Get the week number (1-53)
  let week_year = iso_week.year(); // Get the year of the week (ISO week year)
  println!("The date {:?} is in ISO week {} of the year {}", date, week_number, week_year);
  The date 2023-08-19 is in ISO week 33 of the year 2023
*/

#[allow(dead_code)]
pub struct HistoricalEngine {
    mode: StrategyMode,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    warmup_duration: ChronoDuration,
    buffer_resolution: Option<Duration>,
    gui_enabled: bool,
    primary_subscription_updates: Receiver<Vec<DataSubscription>>,
    market_event_sender: Sender<MarketMessageEnum>,
    notify: Arc<Notify>
}

// The date 2023-08-19 is in ISO week 33 of the year 2023
impl HistoricalEngine {
    pub async fn new(
        mode: StrategyMode,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        warmup_duration: ChronoDuration,
        buffer_resolution: Option<Duration>,
        gui_enabled: bool,
        market_event_sender: Sender<MarketMessageEnum>,
        notify: Arc<Notify>
    ) -> Self {
        let (tx, rx) = mpsc::channel(10);
        subscribe_primary_subscription_updates("Historical Engine".to_string(), tx);
        let engine = HistoricalEngine {
            mode,
            start_time: start_date,
            end_time: end_date,
            warmup_duration,
            buffer_resolution,
            gui_enabled,
            primary_subscription_updates: rx,
            market_event_sender,
            notify
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
                let month_years = generate_file_dates(
                    warm_up_start_time,
                    end_time,
                );

                match self.buffer_resolution {
                    None => self.historical_data_feed_unbuffered(month_years, warm_up_start_time, end_time, self.mode).await,
                    Some(res) => self.historical_data_feed_buffered(month_years, warm_up_start_time, end_time, res, self.mode).await,
                }

                match self.mode {
                    StrategyMode::Backtest => {
                        add_buffer(end_time, StrategyEvent::ShutdownEvent("Backtest Complete".to_string()) ).await;
                        forward_buffer().await;
                    }
                    StrategyMode::Live => {}
                    StrategyMode::LivePaperTrading => {}
                }
            });
        });
    }


    async fn get_base_time_slices(
        &self,
        month_start: DateTime<Utc>,
        primary_subscriptions: &Vec<DataSubscription>
    ) -> Result<BTreeMap<DateTime<Utc>, TimeSlice>, FundForgeError> {
        match get_historical_data(primary_subscriptions.clone(), month_start).await {
            Ok(time_slices) => Ok(time_slices),
            Err(e) => Err(e),
        }
    }

    /// Feeds the historical data to the strategy, along with any events that were created.
    /// Tries to simulate trading without the buffer, where events and data are forwarded directly to the engine
    async fn historical_data_feed_unbuffered(
        &mut self,
        month_years: BTreeMap<i32, DateTime<Utc>>,
        warm_up_start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        mode: StrategyMode
    ) {
        println!("Un-Buffered Engine: Warming up the strategy...");
        let subscription_handler = SUBSCRIPTION_HANDLER.get().unwrap().clone();
        let indicator_handler = INDICATOR_HANDLER.get().unwrap().clone();
        let mut primary_subscriptions = subscription_handler.primary_subscriptions().await;
        for subscription in &primary_subscriptions {
            println!("Un-Buffered Engine: Primary Subscription: {}", subscription);
        }
        let strategy_subscriptions = subscription_handler.strategy_subscriptions().await;
        for subscription in &strategy_subscriptions {
            println!("Un-Buffered Engine: Strategy Subscription: {}", subscription);
        }
        let mut warm_up_complete = false;
        // here we are looping through 1 month at a time, if the strategy updates its subscriptions we will stop the data feed, download the historical data again to include updated symbols, and resume from the next time to be processed.
        'main_loop: for (_, start) in &month_years {
            'month_loop: loop {
                println!("Un-Buffered Engine: Preparing TimeSlices For: {}", start.date_naive().format("%B %Y"));
                let month_time_slices = match self.get_base_time_slices(start.clone(), &primary_subscriptions).await {
                    Ok(time_slices) => time_slices,
                    Err(e) => {
                        eprintln!("Engine: Error getting historical data for: {:?}: {:?}", start, e);
                        continue;
                    }
                };
                println!("{} Data Points Recovered from Server: {} for {}", start.date_naive(), month_time_slices.len(), start.date_naive());
                let mut end_month = true;
                'time_instance_loop: for (time, time_slice) in month_time_slices {
                    if time > end_time {
                        println!("Un-Buffered Engine: End Time: {}", end_time);
                        break 'main_loop;
                    }
                    if time < warm_up_start_time {
                        continue
                    }
                    if !warm_up_complete {
                        if time >= self.start_time {
                            warm_up_complete = true;
                            set_warmup_complete();
                            add_buffer(get_backtest_time(), StrategyEvent::WarmUpComplete).await;
                            forward_buffer().await;
                            if mode == StrategyMode::Live || mode == StrategyMode::LivePaperTrading {
                                unsubscribe_primary_subscription_updates("Historical Engine");
                                break 'main_loop
                            }
                            println!("Un-Buffered Engine: Start Backtest");
                        }
                    }
                    update_backtest_time(time.clone());
                    self.market_event_sender.send(MarketMessageEnum::TimeSliceUpdate(time_slice.clone())).await.unwrap();
                    let mut strategy_time_slice: TimeSlice = TimeSlice::new();
                    // we interrupt if we have a new subscription event so we can fetch the correct data, we will resume from the last time processed.
                    match self.primary_subscription_updates.try_recv() {
                        Ok(updates) => {
                            if updates != primary_subscriptions {
                                primary_subscriptions = updates;
                                end_month = false;
                                break 'time_instance_loop
                            }
                        }
                        Err(_) => {}
                    }

                    // update our consolidators and create the strategies time slice with any new data or just create empty slice.
                    // Add only primary data which the strategy has subscribed to into the strategies time slice
                    if let Some(consolidated_data) = subscription_handler.update_time_slice(time_slice.clone()).await {
                        strategy_time_slice.extend(consolidated_data);
                    }

                    // update the consolidators time and see if that generates new data, in case we didn't have primary data to update with.
                    if let Some(consolidated_data) = subscription_handler.update_consolidators_time(time.clone()).await {
                        strategy_time_slice.extend(consolidated_data);
                    }

                    for data in time_slice.iter() {
                        if strategy_subscriptions.contains(&data.subscription()) {
                            strategy_time_slice.add(data.clone());
                        }
                    }

                    if !strategy_time_slice.is_empty() {
                        // Update indicators and get any generated events.
                        indicator_handler.update_time_slice(time, &strategy_time_slice).await;
                        // add the strategy time slice to the new events.
                        let slice_event = StrategyEvent::TimeSlice(strategy_time_slice);
                        add_buffer(time, slice_event).await;
                        forward_buffer().await;
                        //self.notify.notified().await;
                    }
                }
                if end_month {
                    break 'month_loop;
                }
            }
        }
    }


    /// Feeds the historical data to the strategy, along with any events that were created.
    /// Simulates trading with a live buffer, where we catch events for x duration before forwarding to the strategy
    #[allow(unused_assignments)]
    async fn historical_data_feed_buffered(
        &mut self,
        month_years: BTreeMap<i32, DateTime<Utc>>,
        warm_up_start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        buffer_duration: Duration,
        mode: StrategyMode
    ) {
        println!("Buffered Engine: Warming up the strategy...");
        let subscription_handler = SUBSCRIPTION_HANDLER.get().unwrap().clone();
        let indicator_handler = INDICATOR_HANDLER.get().unwrap().clone();
        // here we are looping through 1 month at a time, if the strategy updates its subscriptions we will stop the data feed, download the historical data again to include updated symbols, and resume from the next time to be processed.
        let mut warm_up_complete = false;
        let mut primary_subscriptions = subscription_handler.primary_subscriptions().await;
        for subscription in &primary_subscriptions {
            println!("Buffered Engine: Primary Subscription: {}", subscription);
        }
        let strategy_subscriptions = subscription_handler.strategy_subscriptions().await;
        for subscription in &strategy_subscriptions {
            println!("Buffered Engine: Strategy Subscription: {}", subscription);
        }
        'main_loop: for (_, start) in &month_years {
            let mut last_time = start.clone();
            'month_loop: loop {
                println!("Buffered Engine: Preparing TimeSlices For: {}", start.date_naive().format("%B %Y"));
                let month_time_slices = match self.get_base_time_slices(start.clone(), &primary_subscriptions).await {
                    Ok(time_slices) => time_slices,
                    Err(e) => {
                        eprintln!("Buffered Engine: Error getting historical data for: {:?}: {:?}", start, e);
                        continue;
                    }
                };
                println!("{} Data Points Recovered from Server: {} for {}", start.date_naive(), month_time_slices.len(), start.date_naive());
                let mut end_month = true;
                'time_instance_loop: loop {
                    let time = last_time + buffer_duration;
                    if time > end_time {
                        println!("Buffered Engine: End Time: {}", end_time);
                        break 'main_loop;
                    }
                    if time < warm_up_start_time {
                        continue
                    }
                    if !warm_up_complete {
                        if time >= self.start_time {
                            warm_up_complete = true;
                            set_warmup_complete();
                            add_buffer(get_backtest_time(), StrategyEvent::WarmUpComplete).await;
                            forward_buffer().await;
                            if mode == StrategyMode::Live || mode == StrategyMode::LivePaperTrading {
                                unsubscribe_primary_subscription_updates("Historical Engine");
                                break 'main_loop
                            }
                            println!("Buffered Engine: Start Backtest");
                        }
                    }

                    if time.month() != start.month() {
                        //println!("Next Month Time");
                        break 'month_loop;
                    }

                    update_backtest_time(time.clone());
                    // we interrupt if we have a new subscription event so we can fetch the correct data, we will resume from the last time processed.
                    match self.primary_subscription_updates.try_recv() {
                        Ok(updates) => {
                            if updates != primary_subscriptions {
                                primary_subscriptions = updates;
                                end_month = false;
                                break 'time_instance_loop
                            }
                        }
                        Err(_) => {}
                    }

                    // Collect data from the primary feeds simulating a buffering range
                    let time_slice: TimeSlice = month_time_slices
                        .range(last_time..=time)
                        .flat_map(|(_, value)| value.iter().cloned())
                        .collect();
                    self.market_event_sender.send(MarketMessageEnum::TimeSliceUpdate(time_slice.clone())).await.unwrap();


                    let mut strategy_time_slice: TimeSlice = TimeSlice::new();
                    // update our consolidators and create the strategies time slice with any new data or just create empty slice.
                    if !time_slice.is_empty() {
                        // Add only primary data which the strategy has subscribed to into the strategies time slice
                        if let Some(consolidated_data) = subscription_handler.update_time_slice(time_slice.clone()).await {
                            strategy_time_slice.extend(consolidated_data);
                        }

                        for base_data in time_slice.iter() {
                            if strategy_subscriptions.contains(&base_data.subscription()) {
                                strategy_time_slice.add(base_data.clone());
                            }
                        }
                    }

                    // update the consolidators time and see if that generates new data, in case we didn't have primary data to update with.
                    if let Some(consolidated_data) = subscription_handler.update_consolidators_time(time.clone()).await {
                        strategy_time_slice.extend(consolidated_data);
                    }

                    if !strategy_time_slice.is_empty() {
                        // Update indicators and get any generated events.
                        indicator_handler.update_time_slice(time, &strategy_time_slice).await;

                        // add the strategy time slice to the new events.
                        let slice_event = StrategyEvent::TimeSlice(
                            strategy_time_slice,
                        );
                        add_buffer(time, slice_event).await;
                        forward_buffer().await;
                        //self.notify.notified().await;
                    }
                    last_time = time.clone();

                }
                if end_month {
                    break 'month_loop;
                }
            }
        }
    }
}


