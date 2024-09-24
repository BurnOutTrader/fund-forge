use chrono::{DateTime, Datelike, Utc,  Duration as ChronoDuration};
use ff_standard_lib::server_connections::{set_warmup_complete, send_strategy_event_slice, SUBSCRIPTION_HANDLER, MARKET_HANDLER, INDICATOR_HANDLER, subscribe_primary_subscription_updates, unsubscribe_primary_subscription_updates};
use ff_standard_lib::standardized_types::base_data::history::{
    generate_file_dates, get_historical_data,
};
use ff_standard_lib::standardized_types::data_server_messaging::FundForgeError;
use ff_standard_lib::standardized_types::enums::StrategyMode;
use ff_standard_lib::standardized_types::strategy_events::{EventTimeSlice, StrategyEvent};
use ff_standard_lib::standardized_types::time_slices::TimeSlice;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc::{Receiver};
use tokio::sync::{mpsc, Notify};
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;

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
pub(crate) struct HistoricalEngine {
    mode: StrategyMode,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    warmup_duration: ChronoDuration,
    buffer_resolution: ChronoDuration,
    notify: Arc<Notify>, //DO not wait for permits outside data feed or we will have problems with freezing
    gui_enabled: bool,
    primary_subscription_updates: Receiver<Vec<DataSubscription>>,
}

// The date 2023-08-19 is in ISO week 33 of the year 2023
impl HistoricalEngine {
    pub async fn new(
        mode: StrategyMode,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        warmup_duration: ChronoDuration,
        buffer_resolution: ChronoDuration,
        notify: Arc<Notify>,
        gui_enabled: bool,
    ) -> Self {
        let (tx, rx) = mpsc::channel(10);
        subscribe_primary_subscription_updates("Historical Engine".to_string(), tx).await;
        let engine = HistoricalEngine {
            notify,
            mode,
            start_time: start_date,
            end_time: end_date,
            warmup_duration,
            buffer_resolution,
            gui_enabled,
            primary_subscription_updates: rx,
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
                self.warmup().await;
                if self.mode == StrategyMode::Backtest {
                    self.run_backtest().await;
                    println!("Engine: Backtest complete");
                    let end_event = StrategyEvent::ShutdownEvent(String::from("Success"));
                    let events = vec![end_event.clone()];
                    send_strategy_event_slice(events).await;
                }
            });
        });
    }

    pub async fn warmup(&mut self) {
        println!("Engine: Warming up the strategy...");
        let end_time = self.start_time;

        // we run the historical data feed from the start time minus the warmup duration until we reach the start date for the strategy
        let month_years = generate_file_dates(
            self.start_time - self.warmup_duration,
            end_time.clone(),
        );

        self.historical_data_feed(month_years, end_time.clone())
            .await;

        set_warmup_complete().await;
        let warmup_complete_event = vec![StrategyEvent::WarmUpComplete];
        send_strategy_event_slice(warmup_complete_event).await;
        if self.mode != StrategyMode::Backtest {
            unsubscribe_primary_subscription_updates("Historical Engine".to_string()).await;
        }
        println!("Engine: Warm up complete")
    }

    /// Runs the strategy backtest
    async fn run_backtest(&mut self) {
        println!("Engine: Start {:?} ", self.mode);
        // we run the historical data feed from the start time until we reach the end date for the strategy
        let month_years = generate_file_dates(
            self.start_time,
            self.end_time.clone(),
        );
        self.historical_data_feed(month_years, self.end_time.clone())
            .await;
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
    async fn historical_data_feed(
        &mut self,
        month_years: BTreeMap<i32, DateTime<Utc>>,
        end_time: DateTime<Utc>,
    ) {
        let subscription_handler = SUBSCRIPTION_HANDLER.get().unwrap().clone();
        let indicator_handler = INDICATOR_HANDLER.get().unwrap().clone();
        let market_handler = MARKET_HANDLER.get().unwrap().clone();
        let notify = self.notify.clone();
        // here we are looping through 1 month at a time, if the strategy updates its subscriptions we will stop the data feed, download the historical data again to include updated symbols, and resume from the next time to be processed.
        'main_loop: for (_, start) in &month_years {
            let mut last_time = start.clone();
            'month_loop: loop {
                let mut primary_subscriptions = subscription_handler.primary_subscriptions().await;
                let mut subscription_changes = true;
                while subscription_changes {
                    match self.primary_subscription_updates.try_recv() {
                        Ok(new_primary_subscriptions) => {
                            if primary_subscriptions != new_primary_subscriptions {
                                primary_subscriptions = new_primary_subscriptions;
                            }
                        }
                        Err(_) => {
                            subscription_changes = false
                        }
                    }
                }
                let strategy_subscriptions = subscription_handler.strategy_subscriptions().await;
                println!("Engine: Strategy Subscriptions: {:?}", strategy_subscriptions);

                println!("Engine: Primary resolution subscriptions: {:?}", primary_subscriptions);
                let month_time_slices = match self.get_base_time_slices(start.clone(), &primary_subscriptions).await {
                    Ok(time_slices) => time_slices,
                    Err(e) => {
                        eprintln!("Engine: Error getting historical data for: {:?}: {:?}", start, e);
                        continue;
                    }
                };
                println!("{} Data Points Recovered from Server: {}", start.date_naive(), month_time_slices.len());

                let mut end_month = true;
                'time_instance_loop: loop {
                    let time = last_time + self.buffer_resolution;
                    if time > end_time {
                        println!("Engine: End Time: {}", end_time);
                        break 'main_loop;
                    }
                    if time.month() != start.month() {
                        //println!("Next Month Time");
                        break 'month_loop;
                    }

                    // we interrupt if we have a new subscription event so we can fetch the correct data, we will resume from the last time processed.
                   match self.primary_subscription_updates.try_recv() {
                        Ok(_) => break 'time_instance_loop,
                        Err(_) => {}
                    }

                    // Collect data from the primary feeds simulating a buffering range
                    let time_slice: TimeSlice = month_time_slices
                        .range(last_time..=time)
                        .flat_map(|(_, value)| value.iter().cloned())
                        .collect();
                    market_handler.update_time_slice(time.clone(), &time_slice).await;

                    let mut strategy_event_slice: EventTimeSlice = EventTimeSlice::new();
                    let mut strategy_time_slice: TimeSlice = TimeSlice::new();
                    // update our consolidators and create the strategies time slice with any new data or just create empty slice.
                    if !time_slice.is_empty() {
                        // Add only primary data which the strategy has subscribed to into the strategies time slice
                        if let Some(consolidated_data) = subscription_handler.update_time_slice(time_slice.clone()).await {
                            strategy_time_slice.extend(consolidated_data);
                        }

                        for base_data in time_slice {
                            if strategy_subscriptions.contains(&base_data.subscription()) {
                                strategy_time_slice.push(base_data);
                            }
                        }
                    }

                    // update the consolidators time and see if that generates new data, in case we didn't have primary data to update with.
                    if let Some(consolidated_data) = subscription_handler.update_consolidators_time(time.clone()).await {
                        strategy_time_slice.extend(consolidated_data);
                    }

                    if !strategy_time_slice.is_empty() {
                        // Update indicators and get any generated events.
                        if let Some(events) = indicator_handler.update_time_slice(&strategy_time_slice).await {
                            strategy_event_slice.extend(events);
                        }

                        // add the strategy time slice to the new events.
                        strategy_event_slice.push(StrategyEvent::TimeSlice(
                            time.to_string(),
                            strategy_time_slice,
                        ));
                    }

                    // send the buffered strategy events to the strategy
                    if !strategy_event_slice.is_empty() {
                        send_strategy_event_slice(strategy_event_slice).await;
                        notify.notified().await;
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


