use crate::interaction_handler::InteractionHandler;
use crate::market_handlers::{HistoricalMarketHandler, MarketHandlerEnum};
use crate::strategy_state::StrategyStartState;
use chrono::{DateTime, Datelike, Utc};
use ff_standard_lib::indicators::indicator_handler::IndicatorHandler;
use ff_standard_lib::server_connections::{get_async_reader, get_async_sender, ConnectionType};
use ff_standard_lib::servers::communications_async::{SecondaryDataReceiver, SecondaryDataSender};
use ff_standard_lib::standardized_types::base_data::history::{
    generate_file_dates, get_historical_data,
};
use ff_standard_lib::standardized_types::data_server_messaging::FundForgeError;
use ff_standard_lib::standardized_types::enums::StrategyMode;
use ff_standard_lib::standardized_types::strategy_events::{EventTimeSlice, StrategyEvent};
use ff_standard_lib::standardized_types::subscription_handler::SubscriptionHandler;
use ff_standard_lib::standardized_types::time_slices::TimeSlice;
use ff_standard_lib::standardized_types::OwnerId;
use ff_standard_lib::strategy_registry::strategies::{StrategyRegistryForward, StrategyResponse};
use ff_standard_lib::strategy_registry::{RegistrationRequest, RegistrationResponse};
use ff_standard_lib::timed_events_handler::TimedEventHandler;
use ff_standard_lib::traits::bytes::Bytes;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration as StdDuration;
use tokio::sync::mpsc::Sender;
use tokio::sync::{Mutex, Notify, RwLock};
use tokio::task;
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
    owner_id: OwnerId,
    start_state: StrategyStartState,
    strategy_event_sender: Sender<EventTimeSlice>,
    subscription_handler: Arc<SubscriptionHandler>,
    market_event_handler: Arc<MarketHandlerEnum>,
    interaction_handler: Arc<InteractionHandler>,
    indicator_handler: Arc<IndicatorHandler>,
    timed_event_handler: Arc<TimedEventHandler>,
    notify: Arc<Notify>, //DO not wait for permits outside data feed or we will have problems with freezing
    last_time: RwLock<DateTime<Utc>>,
    gui_enabled: bool,
}

// The date 2023-08-19 is in ISO week 33 of the year 2023
impl BackTestEngine {
    pub async fn new(
        owner_id: OwnerId,
        notify: Arc<Notify>,
        start_state: StrategyStartState,
        strategy_event_sender: Sender<EventTimeSlice>,
        subscription_handler: Arc<SubscriptionHandler>, //todo, since we are on a seperate thread the engine should take ownership of these and the strategy should just send message requests through a synchronous channel and await responses
        market_event_handler: Arc<MarketHandlerEnum>,
        interaction_handler: Arc<InteractionHandler>,
        indicator_handler: Arc<IndicatorHandler>,
        timed_event_handler: Arc<TimedEventHandler>,
        gui_enabled: bool
    ) -> Self {
        BackTestEngine {
            owner_id,
            notify,
            start_state,
            strategy_event_sender,
            subscription_handler,
            market_event_handler,
            interaction_handler,
            indicator_handler,
            timed_event_handler,
            last_time: RwLock::new(Default::default()),
            gui_enabled
        }
    }

    /// Initializes the strategy, runs the warmup and then runs the strategy based on the mode.
    /// Calling this method will start the strategy running.
    pub async fn launch(self: Self) {
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
                let end_event = StrategyEvent::ShutdownEvent(self.owner_id.clone(), String::from("Success"));
                let events = vec![end_event.clone()];
                match self.strategy_event_sender.send(events).await {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Engine: Error forwarding event: {:?}", e);
                    }
                }
            });
        });
    }

    async fn warmup(&self) {
        let end_time = self.start_state.start_date.to_utc();

        // we run the historical data feed from the start time minus the warmup duration until we reach the start date for the strategy
        let month_years = generate_file_dates(
            self.start_state.start_date - self.start_state.warmup_duration,
            end_time.clone(),
        );

        self.historical_data_feed(month_years, end_time.clone(), false)
            .await;

        self.subscription_handler.set_warmup_complete().await;
        self.market_event_handler.set_warm_up_complete().await;
        self.indicator_handler.set_warmup_complete().await;
        self.timed_event_handler.set_warmup_complete().await;
        let warmup_complete_event = vec![StrategyEvent::WarmUpComplete(self.owner_id.clone())];
        //self.notify.notified().await;
        match self
            .strategy_event_sender
            .send(warmup_complete_event.clone())
            .await
        {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Engine: Error forwarding event: {:?}", e);
            }
        }
        println!("Engine: Strategy Warm up complete")
    }

    /// Runs the strategy backtest
    async fn run_backtest(&self) {
        println!("Engine: Running the strategy backtest...");
        self.interaction_handler.process_controls().await;
        // we run the historical data feed from the start time until we reach the end date for the strategy
        let month_years = generate_file_dates(
            self.start_state.start_date,
            self.start_state.end_date.clone(),
        );
        self.historical_data_feed(month_years, self.start_state.end_date.clone(), true)
            .await;

        self.market_event_handler.process_ledgers().await;
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
        &self,
        month_years: BTreeMap<i32, DateTime<Utc>>,
        end_time: DateTime<Utc>,
        warm_up_completed: bool,
    ) {
        // here we are looping through 1 month at a time, if the strategy updates its subscriptions we will stop the data feed, download the historical data again to include updated symbols, and resume from the next time to be processed.
        'main_loop: for (_, start) in &month_years {
            *self.last_time.write().await = start.clone();
            self.market_event_handler.update_time(start.clone()).await;
            let mut last_time = start.clone();
            'month_loop: loop {
                let strategy_subscriptions = self.subscription_handler.strategy_subscriptions().await;
                //println!("Strategy Subscriptions: {:?}", strategy_subscriptions);
                let primary_subscriptions = self.subscription_handler.primary_subscriptions().await;
                //println!("Primary Subscriptions: {:?}", primary_subscriptions);
                let mut month_time_slices = match self.get_base_time_slices(start.clone(), &primary_subscriptions).await {
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
                    if self.subscriptions_updated(last_time.clone(), &primary_subscriptions).await {
                        end_month = false;
                        break 'time_loop;
                    }

                    self.timed_event_handler.update_time(time.clone()).await;
                    // Collect data from the range
                    let mut time_slice: TimeSlice = month_time_slices
                        .range(last_time..=time)
                        .flat_map(|(_, value)| value.iter().cloned())
                        .collect();

                    let mut strategy_event_slice: EventTimeSlice = EventTimeSlice::new();
                    // if we don't have base data we update any objects which need to be updated with the time
                    if time_slice.is_empty() {
                        if let Some(consolidated_data) = self.subscription_handler.update_consolidators_time(time.clone()).await {
                            if let Some(indicator_slice_event) = self.indicator_handler.update_time_slice(time.clone(), &consolidated_data).await {
                                strategy_event_slice.push(indicator_slice_event);
                            }
                            strategy_event_slice.push(StrategyEvent::TimeSlice(
                                self.owner_id.clone(),
                                time.to_string(),
                                consolidated_data,
                            ));
                        }
                        if let Some(indicator_buffer) = self.indicator_handler.get_event_buffer().await {
                            strategy_event_slice.extend(indicator_buffer);
                        }
                        if !strategy_event_slice.is_empty() {
                            if !self.send_and_continue(time.clone(), strategy_event_slice, warm_up_completed).await {
                                *self.last_time.write().await = last_time.clone();
                                break 'main_loop;
                            }
                        }
                        last_time = time.clone();
                        continue 'time_loop;
                    }

                    let (market_event_handler_events, consolidated_data) = tokio::join!(
                        self.market_event_handler.update_time_slice(&time_slice),
                        self.subscription_handler.update_time_slice(&time_slice),
                    );

                    if let Some(data) = consolidated_data {
                        time_slice.extend(data);
                    }

                    // we also need to check if we have any consolidated data by time, in case the time slice doesn't contain data for the specific subscription.
                    if let Some(consolidated_data) = self.subscription_handler.update_consolidators_time(time.clone()).await {
                        time_slice.extend(consolidated_data)
                    }

                    let mut strategy_time_slice: TimeSlice = TimeSlice::new();
                    for base_data in time_slice {
                        if strategy_subscriptions.contains(&base_data.subscription()) {
                            strategy_time_slice.push(base_data);
                        }
                    }

                    if let Some(events) = market_event_handler_events {
                        strategy_event_slice.extend(events);
                    }

                    if let Some(indicator_slice_event) = self.indicator_handler.update_time_slice(time.clone(), &strategy_time_slice).await {
                        strategy_event_slice.push(indicator_slice_event);
                    }

                    if let Some(indicator_buffer) = self.indicator_handler.get_event_buffer().await {
                        strategy_event_slice.extend(indicator_buffer);
                    }

                    strategy_event_slice.push(StrategyEvent::TimeSlice(
                        self.owner_id.clone(),
                        time.to_string(),
                        strategy_time_slice,
                    ));

                    if !self
                        .send_and_continue(time.clone(), strategy_event_slice, warm_up_completed)
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

    async fn subscriptions_updated(&self, last_time: DateTime<Utc>, primary_subscriptions: &Vec<DataSubscription>) -> bool {
        if self.subscription_handler.get_subscriptions_updated().await {
            self.subscription_handler
                .set_subscriptions_updated(false)
                .await;

            let subscription_events = self.subscription_handler.subscription_events().await;
            let strategy_event = vec![StrategyEvent::DataSubscriptionEvents(
                self.owner_id.clone(),
                subscription_events,
                last_time.timestamp(),
            )];

            match self.strategy_event_sender.send(strategy_event.clone()).await {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Engine: Error forwarding event: {:?}", e);
                }
            }
            // if the primary subscriptions havent changed then we don't need to break the backtest event loop
            let new_primary_subscriptions = self.subscription_handler.primary_subscriptions().await;
            return match *primary_subscriptions == new_primary_subscriptions {
                true => false,
                false => true
            }
        }
        false
    }

    async fn send_and_continue(
        &self,
        time: DateTime<Utc>,
        strategy_event_slice: EventTimeSlice,
        warm_up_completed: bool,
    ) -> bool {
        self.market_event_handler.update_time(time.clone()).await;
        match self.strategy_event_sender.send(strategy_event_slice.clone()).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Engine: Error forwarding event: {:?}", e);
            }
        }
        // We check if the user has requested a delay between time slices for market replay style backtesting.
        if warm_up_completed {
            if self.interaction_handler.process_controls().await == true {
                return false;
            }

            match self.interaction_handler.replay_delay_ms().await {
                Some(delay) => tokio::time::sleep(StdDuration::from_millis(delay)).await,
                None => {}
            }
        }
        self.notify.notified().await;
        true
    }
}

pub struct RegistryHandler {
    owner_id: OwnerId,
    registry_sender: Arc<SecondaryDataSender>,
    registry_receiver: Arc<Mutex<SecondaryDataReceiver>>,
}

impl RegistryHandler {
    pub async fn new(owner_id: OwnerId) -> Self {
        Self {
            owner_id,
            registry_sender: get_async_sender(ConnectionType::StrategyRegistry).await.unwrap(),
            registry_receiver: get_async_reader(ConnectionType::StrategyRegistry).await.unwrap(),
        }
    }

    pub async fn register_strategy(&self, mode: StrategyMode, subscriptions: Vec<DataSubscription>) {
        let strategy_register_event =
            RegistrationRequest::Strategy(self.owner_id.clone(), mode, subscriptions).to_bytes();
        self.registry_sender
            .send(&strategy_register_event)
            .await
            .unwrap();
        match self.registry_receiver.lock().await.receive().await {
            Some(response) => {
                let registration_response =
                    RegistrationResponse::from_bytes(&response).unwrap();
                match registration_response {
                    RegistrationResponse::Success => println!("Engine: Registered with data server"),
                    RegistrationResponse::Error(e) => panic!("Engine: Failed to register strategy: {:?}", e)
                }
            }
            None => {
                panic!("Engine: No response from the strategy registry")
            }
        }
    }

    pub async fn deregister_strategy(&self, msg: String, last_time: DateTime<Utc>) {
        let end_event = StrategyEvent::ShutdownEvent(self.owner_id.clone(), msg);
        let sender = self.registry_sender.clone();
        let shutdown_slice_event = StrategyRegistryForward::StrategyEventUpdates(
            last_time.timestamp(),
            vec![end_event],
        );
        match sender.send(&shutdown_slice_event.to_bytes()).await {
            Ok(_) => {}
            Err(_) => {} }

        let shutdown_warning_event =
            StrategyRegistryForward::ShutDown(last_time.timestamp());
        self.registry_sender
            .send(&shutdown_warning_event.to_bytes())
            .await
            .unwrap();
        match self.registry_receiver.lock().await.receive().await {
            Some(response) => {
                let shutdown_response = StrategyResponse::from_bytes(&response).unwrap();
                match shutdown_response {
                    StrategyResponse::ShutDownAcknowledged(owner_id) => {
                        println!("Engine: Registry Shut Down Acknowledged {}", owner_id)
                    }
                }
            }
            None => {
                eprintln!("Engine: No response from the strategy registry")
            }
        }
    }

    pub async fn forward_events(&self, time: DateTime<Utc>, strategy_event_slice: Vec<StrategyEvent>) {
        let event_bytes = StrategyRegistryForward::StrategyEventUpdates(time.timestamp(), strategy_event_slice).to_bytes();
        let sender = self.registry_sender.clone();
        task::spawn(async move {
            match sender.send(&event_bytes).await {
                Ok(_) => {}
                Err(_) => {}
            }
        });
    }
}
