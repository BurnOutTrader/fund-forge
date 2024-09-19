use crate::strategy_state::StrategyStartState;
use chrono::{DateTime, Datelike, Utc};
use ff_standard_lib::server_connections::{subscribe_primary_subscription_updates, set_warmup_complete, send_strategy_event_slice, SUBSCRIPTION_HANDLER, INDICATOR_HANDLER};
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
use tokio::sync::mpsc::{Receiver};
use tokio::sync::{mpsc, Notify, RwLock};
use tokio::sync::mpsc::error::TryRecvError;
use ff_standard_lib::indicators::indicator_handler::IndicatorEvents;
use ff_standard_lib::indicators::indicator_handler::IndicatorEvents::IndicatorTimeSlice;
use ff_standard_lib::indicators::values::IndicatorValues;
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

pub(crate) struct BackTestEngine {
    start_state: StrategyStartState,
    notify: Arc<Notify>, //DO not wait for permits outside data feed or we will have problems with freezing
    last_time: RwLock<DateTime<Utc>>,
    gui_enabled: bool,
    primary_subscription_updates: Receiver<Vec<DataSubscription>>,
}

// The date 2023-08-19 is in ISO week 33 of the year 2023
impl BackTestEngine {
    pub async fn new(
        notify: Arc<Notify>,
        start_state: StrategyStartState,
        gui_enabled: bool
    ) -> Self {
        let (sender, primary_subscription_updates) = mpsc::channel(1);
        subscribe_primary_subscription_updates("Back Test Engine".to_string(), sender).await;

        let engine = BackTestEngine {
            notify,
            start_state,
            last_time: RwLock::new(Default::default()),
            gui_enabled,
            primary_subscription_updates,
        };
        engine
    }

    /// Initializes the strategy, runs the warmup and then runs the strategy based on the mode.
    /// Calling this method will start the strategy running.
    pub async fn launch(mut self: Self) {
        println!("Engine: Initializing the strategy...");
        thread::spawn(move|| {
            if self.start_state.mode != StrategyMode::Backtest {
                panic!("Incorrect Engine instance for {:?}", self.start_state.mode)
            }
            // Run the engine logic on a dedicated OS thread
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                println!("Engine: Warming up the strategy...");
                self.warmup().await;

                println!("Engine: Start {:?} ", self.start_state.mode);
                self.run_backtest().await;

                println!("Engine: Backtest complete");
                let end_event = StrategyEvent::ShutdownEvent(String::from("Success"));
                let events = vec![end_event.clone()];
                send_strategy_event_slice(events).await;
            });
        });
    }

    async fn warmup(&mut self) {
        let end_time = self.start_state.start_date.to_utc();

        // we run the historical data feed from the start time minus the warmup duration until we reach the start date for the strategy
        let month_years = generate_file_dates(
            self.start_state.start_date - self.start_state.warmup_duration,
            end_time.clone(),
        );

        self.historical_data_feed(month_years, end_time.clone(), false)
            .await;

        set_warmup_complete().await;
        let warmup_complete_event = vec![StrategyEvent::WarmUpComplete];
        send_strategy_event_slice(warmup_complete_event).await;
        println!("Engine: Warm up complete")
    }

    /// Runs the strategy backtest
    async fn run_backtest(&mut self) {
        println!("Engine: Running the strategy backtest...");
        // we run the historical data feed from the start time until we reach the end date for the strategy
        let month_years = generate_file_dates(
            self.start_state.start_date,
            self.start_state.end_date.clone(),
        );
        self.historical_data_feed(month_years, self.start_state.end_date.clone(), true)
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
        warm_up_completed: bool,
    ) {
        let subscription_handler = SUBSCRIPTION_HANDLER.get().unwrap().clone();
        let indicator_handler = INDICATOR_HANDLER.get().unwrap().clone();
        // here we are looping through 1 month at a time, if the strategy updates its subscriptions we will stop the data feed, download the historical data again to include updated symbols, and resume from the next time to be processed.
        'main_loop: for (_, start) in &month_years {
            {
                *self.last_time.write().await = start.clone();
            }
            let mut last_time = start.clone();
            let mut primary_subscriptions = subscription_handler.primary_subscriptions().await;
            'month_loop: loop {
                let strategy_subscriptions = subscription_handler.strategy_subscriptions().await;
                //println!("Strategy Subscriptions: {:?}", strategy_subscriptions);
                let month_time_slices = match self.get_base_time_slices(start.clone(), &primary_subscriptions).await {
                    Ok(time_slices) => time_slices,
                    Err(e) => {
                        eprintln!("Engine: Error getting historical data for: {:?}: {:?}", start, e);
                        continue;
                    }
                };
                println!("Month slice length: {}", month_time_slices.len());

                let mut end_month = true;
                'time_instance_loop: loop {
                    let time = last_time + self.start_state.buffer_resolution;

                    if time > end_time {
                        *self.last_time.write().await = last_time.clone();
                        println!("End Time");
                        break 'main_loop;
                    }
                    if time.month() != start.month() {
                        //println!("Next Month Time");
                        break 'month_loop;
                    }

                    // we interrupt if we have a new subscription event so we can fetch the correct data, we will resume from the last time processed.
                   match self.primary_subscription_updates.try_recv() {
                        Ok(new_primary_subscriptions) => {
                            if primary_subscriptions != new_primary_subscriptions {
                                end_month = false;
                                primary_subscriptions = new_primary_subscriptions;
                                break 'time_instance_loop;
                            }
                        }
                        Err(_) => {}
                    }

                    // Collect data from the primary feeds simulating a buffering range
                    let mut time_slice: TimeSlice = month_time_slices
                        .range(last_time..=time)
                        .flat_map(|(_, value)| value.iter().cloned())
                        .collect();

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
                        self.notify.notified().await;
                    }
                    last_time = time.clone();
                }
                if end_month {
                    break 'month_loop;
                }
            }
            *self.last_time.write().await = last_time.clone();
        }
    }
}

pub struct RegistryHandler {}

impl RegistryHandler {
    pub async fn new() -> Self {
        Self {
        }
    }

    pub async fn register_strategy(&self, mode: StrategyMode, subscriptions: Vec<DataSubscription>) {
        todo!()
    }

    pub async fn deregister_strategy(&self, msg: String, last_time: DateTime<Utc>) {
        todo!()
    }

    pub async fn forward_events(&self, time: DateTime<Utc>, strategy_event_slice: Vec<StrategyEvent>) {
        todo!()
    }
}
