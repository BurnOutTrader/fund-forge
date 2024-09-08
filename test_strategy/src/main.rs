use chrono::{Duration, NaiveDate};
use chrono_tz::Australia;
use ff_standard_lib::apis::vendor::DataVendor;
use ff_standard_lib::indicators::indicator_handler::IndicatorEvents;
use ff_standard_lib::server_connections::{initialize_clients, PlatformMode};
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::{MarketType, Resolution, StrategyMode};
use ff_standard_lib::standardized_types::strategy_events::{
    EventTimeSlice, StrategyEvent, StrategyInteractionMode,
};
use ff_standard_lib::standardized_types::subscriptions::{
    CandleType, DataSubscription,
};
use ff_strategies::fund_forge_strategy::FundForgeStrategy;
use std::sync::Arc;
use tokio::sync::{mpsc, Notify};
use ff_standard_lib::apis::brokerage::Brokerage;
use ff_standard_lib::standardized_types::accounts::ledgers::AccountId;
use ff_standard_lib::standardized_types::base_data::quotebar::QuoteBar;
use ff_standard_lib::standardized_types::rolling_window::RollingWindow;

// to launch on separate machine
#[tokio::main]
async fn main() {
    const GUI_ENABLED: bool = false;
    initialize_clients(&PlatformMode::SingleMachine, GUI_ENABLED).await.unwrap();

    let (strategy_event_sender, strategy_event_receiver) = mpsc::channel(1000);
    let notify = Arc::new(Notify::new());
    // we initialize our strategy as a new strategy, meaning we are not loading drawing tools or existing data from previous runs.
    let strategy = FundForgeStrategy::initialize(
        Some(String::from("test")), //if none is passed in an id will be generated based on the executing program name, todo! this needs to be upgraded in the future to be more reliable in Single and Multi machine modes.
        notify.clone(),
        StrategyMode::Backtest,                 // Backtest, Live, LivePaper
        StrategyInteractionMode::SemiAutomated, // In semi-automated the strategy can interact with the user drawing tools and the user can change data subscriptions, in automated they cannot. // the base currency of the strategy
        NaiveDate::from_ymd_opt(2023, 01, 4)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(), // Starting date of the backtest is a NaiveDateTime not NaiveDate
        NaiveDate::from_ymd_opt(2023, 04, 30)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(), // Ending date of the backtest is a NaiveDateTime not NaiveDate
        Australia::Sydney,                      // the strategy time zone
        Duration::days(2), // the warmup duration, the duration of historical data we will pump through the strategy to warm up indicators etc before the strategy starts executing.
        vec![DataSubscription::new_custom(
                "AUD-CAD".to_string(),
                DataVendor::Test,
                Resolution::Minutes(3),
                BaseDataType::QuoteBars,
                MarketType::Forex,
                CandleType::CandleStick,
            ),
             /*DataSubscription::new_custom(
                 "AUD-CAD".to_string(),
                 DataVendor::Test,
                 Resolution::Minutes(3),
                 BaseDataType::QuoteBars,
                 MarketType::Forex,
                 CandleType::CandleStick,
             ),
             DataSubscription::new_custom(
                 "AUD-CAD".to_string(),
                 DataVendor::Test,
                 Resolution::Seconds(15),
                 BaseDataType::QuoteBars,
                 MarketType::Forex,
                 CandleType::CandleStick,
             ),
             DataSubscription::new_custom(
                 "USD-HUF".to_string(),
                 DataVendor::Test,
                 Resolution::Minutes(15),
                 BaseDataType::QuoteBars,
                 MarketType::Forex,
                 CandleType::CandleStick,
             ),
             DataSubscription::new_custom(
                 "USD-HUF".to_string(),
                 DataVendor::Test,
                 Resolution::Minutes(3),
                 BaseDataType::QuoteBars,
                 MarketType::Forex,
                 CandleType::CandleStick,
             ),
             DataSubscription::new_custom(
                 "USD-HUF".to_string(),
                 DataVendor::Test,
                 Resolution::Seconds(15),
                 BaseDataType::QuoteBars,
                 MarketType::Forex,
                 CandleType::CandleStick,
             )*/], //the closure or function used to set the subscriptions for the strategy. this allows us to have multiple subscription methods for more complex strategies
        5,
        strategy_event_sender, // the sender for the strategy events
        None,
        //strategy resolution, all data at a lower resolution will be consolidated to this resolution, if using tick data, you will want to set this at 1 second or less depending on the data granularity
        //this allows us full control over how the strategy buffers data and how it processes data, in live trading .
        Some(Duration::seconds(1)),
        GUI_ENABLED
    ).await;

    on_data_received(strategy, notify, strategy_event_receiver).await;
}

/// Here we listen for incoming data and build our custom strategy logic. this is where the magic happens.
pub async fn on_data_received(
    strategy: FundForgeStrategy,
    notify: Arc<Notify>,
    mut event_receiver: mpsc::Receiver<EventTimeSlice>,
) {
 /*   let heikin_atr_20 = IndicatorEnum::AverageTrueRange(
        AverageTrueRange::new(IndicatorName::from("heikin_atr_20"), DataSubscription::new(
                "AUD-CAD".to_string(),
                DataVendor::Test,
                Resolution::Minutes(15),
                BaseDataType::QuoteBars,
                MarketType::Forex,
            ),
            100,
            20,
            Some(Color::new(50,50,50))
        )
            .await,
    );
    strategy.indicator_subscribe(heikin_atr_20).await;*/

    let brokerage = Brokerage::Test;
    let account = AccountId::from("1");

    let mut warmup_complete = false;
    let mut count = 0;
    let mut history : RollingWindow<QuoteBar> = RollingWindow::new(10);

    'strategy_loop: while let Some(event_slice) = event_receiver.recv().await {
        for strategy_event in event_slice {
            match strategy_event {
                // when a drawing tool is added from some external source the event will also show up here (the tool itself will be added to the strategy.drawing_objects HashMap behind the scenes)
                StrategyEvent::DrawingToolEvents(_, drawing_tool_event, _) => {
                    println!("Drawing Tool Event: {:?}", drawing_tool_event);
                }
                StrategyEvent::TimeSlice(_owner , time, time_slice) => {
                    // here we would process the time slice events and update the strategy state accordingly.
                    'base_data_loop: for base_data in &time_slice {
                        // only data we specifically subscribe to show up here, if the data is building from ticks but we didn't subscribe to ticks specifically, ticks won't show up but the subscribed resolution will.
                        match base_data {
                            BaseDataEnum::Price(_) => {}
                            BaseDataEnum::Candle(candle) => {
                                if candle.is_closed == true {
                                    println!("{}", candle);
                                }
                            }
                            BaseDataEnum::QuoteBar(quotebar) => {
                                if !warmup_complete {
                                    continue;
                                }
                                if quotebar.is_closed == true {
                                    println!("Closed bar time {}", time); //note we automatically adjust for daylight savings based on historical daylight savings adjustments.
                                    history.add(quotebar.clone());

                                    //todo, make a candle_index and quote_bar_index to get specific data types and save pattern matching
                                    let last_bar = match history.get(1) {
                                        None => continue,
                                        Some(bar) => bar
                                    };

                                    let bars_2 = match history.get(2) {
                                        None => continue,
                                        Some(bar) => bar
                                    };

                                    if quotebar.bid_close > last_bar.bid_high && last_bar.bid_close > bars_2.bid_high {
                                        strategy.enter_long(quotebar.symbol.name.clone(), account.clone(), brokerage.clone(), 1, String::from("Enter Long")).await;
                                        println!("Enter Long");
                                    } else if quotebar.bid_close < last_bar.bid_low && last_bar.bid_close < bars_2.bid_low {
                                        strategy.enter_short(quotebar.symbol.name.clone(), account.clone(), brokerage.clone(), 1, String::from("Enter Long")).await;
                                        println!("Enter Short");
                                    }
                                } else if !quotebar.is_closed {
                                    //println!("Open bar time: {}", time)
                                }
                            }
                            BaseDataEnum::Tick(tick) => {
                                if !warmup_complete {
                                    // we could manually warm up indicators etc here.
                                    println!(
                                        "{}...{} Tick: {}",
                                        strategy.time_utc().await,
                                        tick.symbol.name,
                                        base_data.time_created_utc()
                                    );
                                }
                            }
                            BaseDataEnum::Quote(_) => {}
                            BaseDataEnum::Fundamental(_) => {}
                        }
                    }
                }
                // order updates are received here, excluding order creation events, the event loop here starts with an OrderEvent::Accepted event and ends with the last fill, rejection or cancellation events.
                StrategyEvent::OrderEvents(_, event) => {
                    println!("Order Event: {:?}", event);
                }
                // if an external source adds or removes a data subscription it will show up here, this is useful for SemiAutomated mode
                StrategyEvent::DataSubscriptionEvents(_, events, _) => {
                    for event in events {
                        println!("Data Subscription Event: {:?}", event);
                    }
                }
                // strategy controls are received here, this is useful for SemiAutomated mode. we could close all positions on a pause of the strategy, or custom handle other user inputs.
                StrategyEvent::StrategyControls(_, _, _) => {}
                StrategyEvent::ShutdownEvent(_, _) => break 'strategy_loop, //we should handle shutdown gracefully by first ending the strategy loop.
                StrategyEvent::WarmUpComplete(_) => {
                    println!("Strategy Warmup Complete");
                    warmup_complete = true;
                }
                StrategyEvent::IndicatorEvent(_, indicator_event) => {
                    //we can handle indicator events here, this is useful for debugging and monitoring the state of the indicators.
                    match indicator_event {
                        IndicatorEvents::IndicatorAdded(added_event) => {
                            println!("Indicator Added: {:?}", added_event);
                        }
                        IndicatorEvents::IndicatorRemoved(removed_event) => {
                            println!("Indicator Removed: {:?}", removed_event);
                        }
                        IndicatorEvents::IndicatorTimeSlice(_time, slice_event) => {
                            // we can see our auto manged indicator values for here.
                            for indicator_values in slice_event {
                                println!(
                                    "{}: \n {:?}",
                                    indicator_values.name(),
                                    indicator_values.values()
                                );
                            }

                            /*let history: Option<RollingWindow<IndicatorValues>> = strategy.indicator_history(IndicatorName::from("heikin_atr_20")).await;
                            if let Some(history) = history {
                                println!("History: {:?}", history.history());
                            }

                            let current: Option<IndicatorValues> = strategy.indicator_current(&IndicatorName::from("heikin_atr_20")).await;
                            if let Some(current) = current {
                                println!("Current: {:?}", current.values());
                            }

                            let index: Option<IndicatorValues> = strategy.indicator_index(&IndicatorName::from("heikin_atr_20"), 3).await;
                            if let Some(index) = index {
                                println!("Index: {:?}", index.values());
                            }*/
                        }
                        IndicatorEvents::Replaced(replace_event) => {
                            println!("Indicator Replaced: {:?}", replace_event);
                        }
                    }

                    // we could also get the automanaged indicator values from teh strategy at any time.
                }
                StrategyEvent::PositionEvents(_) => {}
            }
            notify.notify_one();
            //simulate work
            //tokio::time::sleep(tokio::time::Duration::from_nanos(50)).await;
        }
    }
    event_receiver.close();
    println!("Strategy Event Loop Ended");
}
