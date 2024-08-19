use std::collections::{BTreeMap};
use std::sync::Arc;
use std::time::Duration as StdDuration;
use chrono::{DateTime, Utc};
use ff_standard_lib::standardized_types::base_data::history::{generate_file_dates, get_historical_data};
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::{StrategyMode};
use ff_standard_lib::standardized_types::time_slices::TimeSlice;
use crate::fund_forge_strategy::{FundForgeStrategy};
use crate::messages::strategy_events::{StrategyEvent};


impl FundForgeStrategy {
    /// Initializes the strategy, runs the warmup and then runs the strategy based on the mode.
    /// Calling this method will start the strategy running.
    pub async fn launch(strategy: Arc<FundForgeStrategy>) {
        println!("Initializing the strategy...");
        let mode = {
            println!("Subscriptions updated: {:?}", strategy.subscription_handler.subscriptions().await);
            strategy.state.mode.clone()
        };

        strategy.warmup(mode).await;

        println!("Start Mode: {:?} ", mode);
        let msg = match mode {
            StrategyMode::Backtest => {
                strategy.run_backtest().await;
                "Backtest complete".to_string()
            }
            // All other modes use the live engine, just with different fns from the engine
            _ => {
                strategy.run_live().await;
                "Live complete".to_string()
            }
        };

        println!("{:?}", &msg);
        let end_event = StrategyEvent::ShutdownEvent(strategy.owner_id.clone(), msg);
        strategy.fwd_event(end_event).await;
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
    async fn warmup(&self, mode: StrategyMode) {
        let start_time = match mode {
            StrategyMode::Backtest => {
                self.state.start_date.to_utc()
            },
            //If Live we return the current time in utc time
            _ => Utc::now(),
        };

        // we run the historical data feed from the start time minus the warmup duration until we reach the start date for the strategy
        let warmup_start_time = start_time - self.state.warmup_duration;

        let month_years = generate_file_dates(warmup_start_time, start_time.clone());
        self.historical_data_feed(month_years, start_time.clone()).await;

        self.state.set_warmup_complete(true).await;
    }

    /// Runs the strategy backtest
    async fn run_backtest(&self) {
        println!("Running the strategy backtest...");
        self.interaction_handler.process_controls().await;

        // we run the historical data feed from the start time until we reach the end date for the strategy
        let start_time = self.state.start_date.clone();

        let end_date = self.state.end_date.clone();
        let month_years = generate_file_dates(start_time, end_date.clone());

        //println!("file dates: {:?}", month_years);

        self.historical_data_feed(month_years, end_date).await;

        self.market_event_handler.process_ledgers().await;

        // If we have reached the end of the backtest, we check that the last time recorded is not in the future, if it is, we set it to the current time.
    }


    /// Used by historical data feed to determine how to proceed with the historical data feed.
    /// Feeds the historical data to the strategy fixing
    async fn historical_data_feed(&self, month_years: BTreeMap<i32, DateTime<Utc>>, end_time: DateTime<Utc>) {
        let owner_id = self.owner_id.clone();
        let mut last_time = self.time_utc().await;
        // here we are looping through 1 month at a time, if the strategy updates its subscriptions we will stop the data feed, download the historical data again to include updated symbols, and resume from the next time to be processed.
        'main_loop: for (_, month_start) in month_years {
            'month_loop: loop {
                let subscriptions = self.subscription_handler.primary_subscriptions().await;
                let time_slices = match get_historical_data(subscriptions.clone(), month_start).await {
                    Ok(time_slices) => {
                        time_slices
                    },
                    Err(e) => {
                        println!("Error getting historical data for: {:?}. Error: {:?}", month_start, e);
                        break 'month_loop
                    }
                };

                if time_slices.len() == 0 {
                    println!("No data found for: {:?}", month_start);
                    break 'month_loop
                }

                'slice_loop: for (time, time_slice) in time_slices {
                    if time > end_time {
                        break 'main_loop
                    }

                    if self.subscription_handler.subscriptions_updated().await {
                        self.subscription_handler.set_subscriptions_updated(false).await;
                        last_time = time;
                        break 'slice_loop
                    }

                    if time < last_time {
                        continue;
                    }

                    // Here we check if we have any consildated data and feed it into the engine first, this is because time consoldated data forms 1 base data point late and so must be fed into the strategy before the next base data
                    let consolidated_data = self.subscription_handler.update_consolidators(time_slice.clone()).await;
                    if let Some(consolidated_data) = consolidated_data {
                        let mut consolidated_timeslices : BTreeMap<DateTime<Utc>, TimeSlice> = BTreeMap::new();
                        for data in consolidated_data {
                            if !consolidated_timeslices.contains_key(&data.time_created_utc()) {
                                consolidated_timeslices.insert(data.time_created_utc(), vec![data]);
                            }
                            else {
                                consolidated_timeslices.get_mut(&data.time_created_utc()).unwrap().push(data);
                            }
                        }
                        for (consolidation_time, time_slice) in consolidated_timeslices {
                            let time_slice_event = StrategyEvent::TimeSlice(owner_id.clone(), time_slice);
                            self.market_event_handler.update_time(consolidation_time).await;
                            self.fwd_event(time_slice_event).await;
                        }
                    }

                    // The market event handler response with any order events etc that may have been returned from the base data update
                    let market_event_handler_events = self.market_event_handler.base_data_upate(time_slice.clone()).await;
                    if let Some(event_handler_events) = market_event_handler_events {
                        self.market_event_handler.update_time(time.clone()).await;
                        for event in event_handler_events {
                            self.fwd_event(event).await;
                        }
                    }

                    let time_slice_event = StrategyEvent::TimeSlice(owner_id.clone(), time_slice);
                    self.fwd_event(time_slice_event).await;

                   // We check if the user has requested a delay between time slices for market replay style backtesting.
                    match self.interaction_handler.replay_delay_ms().await {
                        Some(delay) => tokio::time::sleep(StdDuration::from_millis(delay)).await,
                        None => {},
                    }

                    // We check if the user has input any strategy commands, pause, play, stop etc.
                    if self.interaction_handler.process_controls().await == true {
                        break 'main_loop
                    }
                }
            };
        }
    }
}


