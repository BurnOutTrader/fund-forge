use crate::strategy_state::StrategyStartState;
use chrono::{DateTime, Datelike, Utc};
use ff_standard_lib::server_connections::{broadcast_time, get_strategy_sender, subscribe_all_subscription_updates, subscribe_indicator_events, subscribe_markets, subscribe_primary_subscription_updates, set_warmup_complete, broadcast_primary_data, subscribe_indicator_values, subscribe_consolidated_time_slice};
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
    consolidated_updates: Receiver<TimeSlice>,
    primary_subscription_updates: Receiver<Vec<DataSubscription>>,
    all_subscription_updates: Receiver<Vec<DataSubscription>>,
    indicator_value_updates: Receiver<Vec<IndicatorValues>>,
    indicator_event_updates: Receiver<Vec<IndicatorEvents>>,
    market_updates: Receiver<EventTimeSlice>
}

// The date 2023-08-19 is in ISO week 33 of the year 2023
impl BackTestEngine {
    pub async fn new(
        notify: Arc<Notify>,
        start_state: StrategyStartState,
        gui_enabled: bool
    ) -> Self {
        let (sender, consolidated_updates) = mpsc::channel(1);
        subscribe_consolidated_time_slice(sender).await;

        let (sender, primary_subscription_updates) = mpsc::channel(1);
        subscribe_primary_subscription_updates(sender).await;

        let (sender, all_subscription_updates) = mpsc::channel(1);
        subscribe_all_subscription_updates(sender).await;

        let (sender, indicator_value_updates) = mpsc::channel(1);
        subscribe_indicator_values(sender).await;

        let (sender, indicator_event_updates) = mpsc::channel(1);
        subscribe_indicator_events(sender).await;

        let (sender, market_updates) = mpsc::channel(1);
        subscribe_markets(sender).await;

        let engine = BackTestEngine {
            notify,
            start_state,
            last_time: RwLock::new(Default::default()),
            gui_enabled,
            consolidated_updates,
            primary_subscription_updates,
            all_subscription_updates,
            indicator_value_updates,
            indicator_event_updates,
            market_updates
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
                match get_strategy_sender().lock().await.send(events).await {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Engine: Error forwarding event: {:?}", e);
                    }
                }
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
        match get_strategy_sender().lock().await.send(warmup_complete_event).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Engine: Error forwarding event: {:?}", e);
            }
        }
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
        // here we are looping through 1 month at a time, if the strategy updates its subscriptions we will stop the data feed, download the historical data again to include updated symbols, and resume from the next time to be processed.
        'main_loop: for (_, start) in &month_years {
            *self.last_time.write().await = start.clone();

            broadcast_time(start.clone()).await;
            let mut last_time = start.clone();
            let mut primary_subscriptions = self.primary_subscription_updates.recv().await.unwrap();
            'month_loop: loop {
                let strategy_subscriptions = self.all_subscription_updates.recv().await.unwrap();
                //println!("Strategy Subscriptions: {:?}", strategy_subscriptions);
                //println!("Primary Subscriptions: {:?}", primary_subscriptions);
                let month_time_slices = match self.get_base_time_slices(start.clone(), &primary_subscriptions).await {
                    Ok(time_slices) => time_slices,
                    Err(e) => {
                        eprintln!("Engine: Error getting historical data for: {:?}: {:?}", start, e);
                        continue;
                    }
                };

                let mut end_month = true;
                'time_loop: loop {
                    let time = last_time + self.start_state.buffer_resolution;

                    if time > end_time {
                        *self.last_time.write().await = last_time.clone();
                        break 'main_loop;
                    }
                    if time.month() != start.month() {
                        break 'month_loop;
                    }

                    // we interrupt if we have a new subscription event so we can fetch the correct data, we will resume from the last time processed.
                    if let Some(new_primary_subscriptions) = self.primary_subscription_updates.recv().await {
                        if primary_subscriptions != new_primary_subscriptions {
                            end_month = false;
                            primary_subscriptions = new_primary_subscriptions;
                            break 'time_loop;
                        }
                    }


                    // Collect data from the range
                    let mut time_slice: TimeSlice = month_time_slices
                        .range(last_time..=time)
                        .flat_map(|(_, value)| value.iter().cloned())
                        .collect();

                    let mut strategy_event_slice: EventTimeSlice = EventTimeSlice::new();
                    // if we don't have base data we update any objects which need to be updated with the time
                    if time_slice.is_empty() {
                        broadcast_time(time.clone()).await;
                        let consolidated_data = self.consolidated_updates.recv().await.unwrap();
                        let indicator_slice_event = self.indicator_value_updates.recv().await.unwrap();
                        strategy_event_slice.push(StrategyEvent::IndicatorEvent(IndicatorTimeSlice(indicator_slice_event)));
                        strategy_event_slice.push(StrategyEvent::TimeSlice(
                            time.to_string(),
                            consolidated_data,
                        ));
                        let indicator_buffer =  self.indicator_event_updates.recv().await.unwrap();
                        for event in indicator_buffer {
                            strategy_event_slice.push(StrategyEvent::IndicatorEvent(event));
                        }
                        if !strategy_event_slice.is_empty() {
                            if !self.send_and_continue(strategy_event_slice, warm_up_completed).await {
                                *self.last_time.write().await = last_time.clone();
                                break 'main_loop;
                            }
                        }
                        last_time = time.clone();
                        continue 'time_loop;
                    }
                    broadcast_primary_data(time_slice.clone()).await;
                    broadcast_time(time).await;

                    // need multiple event loops for market handler, 1 for data 1 for orders
                    //self.market_handler_update_sender.send(MarketHandlerUpdate::TimeSlice(time_slice.clone())).await.unwrap();

                    let consolidated_data = self.consolidated_updates.recv().await.unwrap();
                    time_slice.extend(consolidated_data);

                    let mut strategy_time_slice: TimeSlice = TimeSlice::new();
                    for base_data in time_slice {
                        if strategy_subscriptions.contains(&base_data.subscription()) {
                            strategy_time_slice.push(base_data);
                        }
                    }

                    let market_event_handler_events = self.market_updates.recv().await.unwrap();
                    strategy_event_slice.extend(market_event_handler_events);

                    let indicator_buffer= self.indicator_event_updates.recv().await.unwrap();
                    for event in indicator_buffer {
                        strategy_event_slice.push(StrategyEvent::IndicatorEvent(event));
                    }

                   let indicator_slice_event = self.indicator_value_updates.recv().await.unwrap();
                    strategy_event_slice.push(StrategyEvent::IndicatorEvent(IndicatorEvents::IndicatorTimeSlice(indicator_slice_event)));


                    strategy_event_slice.push(StrategyEvent::TimeSlice(
                        time.to_string(),
                        strategy_time_slice,
                    ));

                    if !self
                        .send_and_continue(strategy_event_slice, warm_up_completed)
                        .await
                    {
                        *self.last_time.write().await = last_time.clone();
                        break 'main_loop;
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

    async fn send_and_continue(
        &self,
        strategy_event_slice: EventTimeSlice,
        warm_up_completed: bool,
    ) -> bool {
        match get_strategy_sender().lock().await.send(strategy_event_slice).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Engine: Error forwarding event: {:?}", e);
            }
        }
        // We check if the user has requested a delay between time slices for market replay style backtesting.
        // need to subscribe to updates for these handlers
  /*      if warm_up_completed {
            if self.interaction_handler.process_controls().await == true {
                return false;
            }

            match self.interaction_handler.replay_delay_ms().await {
                Some(delay) => tokio::time::sleep(StdDuration::from_millis(delay)).await,
                None => {}
            }
        }*/
        self.notify.notified().await;
        true
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
