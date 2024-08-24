use std::cell::RefCell;
use std::collections::{BTreeMap};
use std::sync::{Arc};
use std::time::Duration as StdDuration;
use chrono::{DateTime, Datelike, Duration, Utc};
use tokio::sync::mpsc::{Sender};
use tokio::sync::{Notify};
use tokio::task;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::history::{generate_file_dates, get_historical_data};
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::data_server_messaging::FundForgeError;
use ff_standard_lib::standardized_types::enums::{StrategyMode};
use ff_standard_lib::standardized_types::OwnerId;
use ff_standard_lib::standardized_types::time_slices::TimeSlice;
use ff_standard_lib::subscription_handler::SubscriptionHandler;
use crate::interaction_handler::InteractionHandler;
use crate::market_handlers::MarketHandlerEnum;
use ff_standard_lib::strategy_events::{EventTimeSlice, StrategyEvent};
use crate::strategy_state::StrategyStartState;
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

pub(crate) struct Engine {
    owner_id: OwnerId,
    start_state: StrategyStartState,
    strategy_event_sender: Sender<EventTimeSlice>,
    subscription_handler: Arc<SubscriptionHandler>,
    market_event_handler: Arc<MarketHandlerEnum>,
    interaction_handler: Arc<InteractionHandler>,
    notify: Arc<Notify>, //DO not wait for permits outside data feed or we will have problems with freezing
}

// The date 2023-08-19 is in ISO week 33 of the year 2023
impl Engine{
    pub fn new(owner_id: OwnerId,
               notify: Arc<Notify>,
               start_state: StrategyStartState,
               strategy_event_sender: Sender<EventTimeSlice>,
               subscription_handler: Arc<SubscriptionHandler>,
               market_event_handler: Arc<MarketHandlerEnum>,
               interaction_handler: Arc<InteractionHandler>) -> Self {
        Engine {
            owner_id,
            notify,
            start_state,
            strategy_event_sender,
            subscription_handler,
            market_event_handler,
            interaction_handler,
        }
    }
    /// Initializes the strategy, runs the warmup and then runs the strategy based on the mode.
    /// Calling this method will start the strategy running.
    pub async fn launch(self: Self) {
        task::spawn(async move {
            println!("Engine: Initializing the strategy...");

            self.warmup().await;

            println!("Engine: Start {:?} ", self.start_state.mode);
            let msg = match self.start_state.mode {
                StrategyMode::Backtest => {
                    self.run_backtest().await;
                    "Engine: Backtest complete".to_string()
                }
                // All other modes use the live engine, just with different fns from the engine
                _ => {
                    self.run_live().await;
                    "Engine: Live complete".to_string()
                }
            };

            println!("{:?}", &msg);
            let end_event = StrategyEvent::ShutdownEvent(self.owner_id.clone(), msg);
            //self.notify.notified().await;
            match self.strategy_event_sender.send(vec![end_event]).await {
                Ok(_) => {},
                Err(e) => {
                    println!("Engine: Error forwarding event: {:?}", e);
                }
            }
        });
    }

    pub async fn run_live(&self) {
        todo!("Implement live mode");
        //need implementation for live plus live paper, where accounts are simulated locally. also make a hybrid mode so it trades live plus paper to test the engine functions accurately
    }


    /// Warm up runs on initialization to get the strategy ready for execution, this can be used to warm up indicators etc., to ensure the strategy has enough history to run from start to finish, without waiting for data.
    /// This is especially useful in live trading scenarios, else you could be waiting days for the strategy to warm up if you are dependent on a long history for your indicators etc. \
    /// \
    /// An alternative to the warmup, would be using the `history` method to get historical data for a specific subscription, and then manually warm up the indicators during re-balancing or adding new subscriptions.
    /// You don't need to be subscribed to an instrument to get the history, so that method is a good alternative for strategies that dynamically subscribe to instruments.
    async fn warmup(&self) {
        println!("Engine: Warming up the strategy...");
        let start_time = match self.start_state.mode {
            StrategyMode::Backtest => {
                self.start_state.start_date.to_utc()
            },
            //If Live we return the current time in utc time
            _ => Utc::now(),
        };

        // we run the historical data feed from the start time minus the warmup duration until we reach the start date for the strategy
        let month_years = generate_file_dates(self.start_state.start_date - self.start_state.warmup_duration , start_time.clone());
        self.historical_data_feed(month_years, start_time.clone(), false).await;

        self.subscription_handler.set_warmup_complete().await;
        self.market_event_handler.set_warm_up_complete().await;
        let warmup_complete_event = StrategyEvent::WarmUpComplete(self.owner_id.clone());
        //self.notify.notified().await;
        match self.strategy_event_sender.send(vec![warmup_complete_event]).await {
            Ok(_) => {},
            Err(e) => {
                println!("Engine: Error forwarding event: {:?}", e);
            }
        }
    }

    /// Runs the strategy backtest
    async fn run_backtest(&self) {
        println!("Engine: Running the strategy backtest...");
        self.interaction_handler.process_controls().await;
        // we run the historical data feed from the start time until we reach the end date for the strategy
        let month_years = generate_file_dates(self.start_state.start_date, self.start_state.end_date.clone());
        self.historical_data_feed(month_years, self.start_state.end_date.clone(), true).await;

        self.market_event_handler.process_ledgers().await;

        // If we have reached the end of the backtest, we check that the last time recorded is not in the future, if it is, we set it to the current time.
    }


    async fn get_base_time_slices(&self, month_start: DateTime<Utc>) -> Result<BTreeMap<DateTime<Utc>, TimeSlice>, FundForgeError> {
        let subscriptions = self.subscription_handler.primary_subscriptions().await;
        //println!("Month Loop Subscriptions: {:?}", subscriptions);
        match get_historical_data(subscriptions.clone(), month_start).await {
            Ok(time_slices) => {
               Ok(time_slices)
            },
            Err(e) => Err(e)
        }
    }

    /// Feeds the historical data to the strategy, along with any events that were created.
    async fn historical_data_feed(&self, month_years: BTreeMap<i32, DateTime<Utc>>, end_time: DateTime<Utc>, warm_up_completed: bool) {
        // here we are looping through 1 month at a time, if the strategy updates its subscriptions we will stop the data feed, download the historical data again to include updated symbols, and resume from the next time to be processed.
        'main_loop: for (_, start) in &month_years {
            let mut last_time = start.clone();
            'month_loop: loop {
                self.subscription_handler.set_subscriptions_updated(false).await;
                let mut time_slices = match self.get_base_time_slices(start.clone()).await {
                    Ok(time_slices) => time_slices,
                    Err(e) => {
                        println!("Error getting historical data for: {:?}: {:?}", start, e);
                        break 'month_loop
                    }
                };

                let mut end_month = true;
                'time_loop: loop {
                    let time = last_time + self.start_state.backtest_resolution;

                    if time > end_time {
                        break 'main_loop
                    }
                    if time.month() != start.month() {
                        break 'month_loop
                    }

                    // we interrupt if we have a new subscription event so we can fetch the correct data, we will resume from the last time processed.
                    if self.subscriptions_updated(last_time).await {
                        end_month = false;
                        break 'time_loop
                    }

                    //check if we have any base data for the time
                    let time_slice = time_slices.get(&time);

                    // if we don't have base data we update any objects which need to be updated with the time
                    if time_slice.is_none() {
                        let mut strategy_event_slice : EventTimeSlice = EventTimeSlice::new();
                        let consolidated_data = self.subscription_handler.update_consolidators_time(time.clone()).await;
                        
                        if !consolidated_data.is_empty() {
                            strategy_event_slice.push(StrategyEvent::TimeSlice(self.owner_id.clone(), consolidated_data));
                        }
                        if !strategy_event_slice.is_empty() {
                            if !self.send_and_continue(time.clone(), strategy_event_slice, warm_up_completed).await {
                                break 'main_loop
                            }
                        }
                        last_time = time.clone();
                        continue 'time_loop
                    }
                    
                    if time_slice.is_none() {
                        continue 'time_loop
                    }

                    let mut time_slice = time_slice.unwrap().clone();

                    let mut strategy_event = EventTimeSlice::new();

                    if let Some(market_event_handler_events) = self.market_event_handler.base_data_upate(time_slice.clone()).await {
                        strategy_event.extend(market_event_handler_events);
                    }

                    let consolidated_data = self.subscription_handler.update_consolidators(time_slice.clone()).await;
                    time_slice.extend(consolidated_data);
                    strategy_event.push(StrategyEvent::TimeSlice(self.owner_id.clone(), time_slice));

                    
                    if !self.send_and_continue(time.clone(), strategy_event, warm_up_completed).await {
                        break 'main_loop
                    }
                    
                    time_slices.remove(&time);
                    last_time = time.clone();
                }
                if end_month {
                    break 'month_loop
                }
            }
        }
    }
    
    async fn subscriptions_updated(&self, last_time: DateTime<Utc>) -> bool {
        if self.subscription_handler.subscriptions_updated().await {
            let subscription_events = self.subscription_handler.subscription_events().await;
            let strategy_event = vec![StrategyEvent::DataSubscriptionEvents(self.owner_id.clone(), subscription_events, last_time.timestamp())];
            match self.strategy_event_sender.send(strategy_event).await {
                Ok(_) => {},
                Err(e) => {
                    println!("Error forwarding event: {:?}", e);
                }
            }
            return true
        }
        false
    }
    
    async fn send_and_continue(&self, time: DateTime<Utc>, strategy_event_slice: EventTimeSlice, warm_up_completed: bool) -> bool {
        self.market_event_handler.update_time(time.clone()).await;
        match self.strategy_event_sender.send(strategy_event_slice).await {
            Ok(_) => {},
            Err(e) => {
                println!("Error forwarding event: {:?}", e);
            }
        }
        // We check if the user has requested a delay between time slices for market replay style backtesting.
        if warm_up_completed {
            match self.interaction_handler.replay_delay_ms().await {
                Some(delay) => tokio::time::sleep(StdDuration::from_millis(delay)).await,
                None => {},
            }
            if self.interaction_handler.process_controls().await == true {
                return false
            }
        }
        self.notify.notified().await;
        
        true
    }
}