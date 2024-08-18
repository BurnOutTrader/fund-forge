use std::collections::HashMap;
use std::sync::Arc;
use chrono::{Duration, NaiveDate};
use chrono_tz::Australia;
use tokio::sync::{mpsc};
use ff_strategies::fund_forge_strategy::FundForgeStrategy;
use ff_standard_lib::apis::brokerage::Brokerage;
use ff_standard_lib::apis::vendor::DataVendor;
use ff_standard_lib::server_connections::{PlatformMode};
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::{MarketType, Resolution, StrategyMode};
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
use ff_strategies::messages::strategy_events::{StrategyEvent, StrategyInteractionMode};


/*fn defualt_broker_map() -> HashMap<DataVendor, Brokerage> {
    let mut broker_map = HashMap::new();
    broker_map.insert(DataVendor::Test, Brokerage::Test);
    broker_map
}*/

/// On initialization of a strategy we need to set the subscriptions function, this can be changed at any time to a new function, or the existing function can be used to recalibrate the subscriptions at runtime.
fn set_subscriptions_initial() -> Vec<DataSubscription> {
    let subscriptions: Vec<DataSubscription> = vec![
        DataSubscription::new("AUD-CAD".to_string(), DataVendor::Test, Resolution::Ticks(1), BaseDataType::Ticks, MarketType::Forex),
        DataSubscription::new("AUD-CAD".to_string(), DataVendor::Test, Resolution::Seconds(15), BaseDataType::Candles, MarketType::Forex),
    ];
    subscriptions
}

#[tokio::main]
async fn main() {
    let (strategy_event_sender, strategy_event_receiver) = mpsc::channel(10000);
    // we initialize our strategy as a new strategy, meaning we are not loading drawing tools or existing data from previous runs.
    let strategy = FundForgeStrategy::initialize(
        Some(String::from("test")), //if none is passed in an id will be generated based on the executing program name, todo! this needs to be upgraded in the future to be more reliable in Single and Multi machine modes.
        PlatformMode::SingleMachine, // SingleMachine or MultiMachine, multi machine mode for co-located servers, single machine for and MT5/Ninjatrader style platform
        StrategyMode::Backtest, // Backtest, Live, LivePaper
        StrategyInteractionMode::SemiAutomated,  // In semi-automated the strategy can interact with the user drawing tools and the user can change data subscriptions, in automated they cannot. // the base currency of the strategy
        NaiveDate::from_ymd_opt(2023, 03, 8).unwrap().and_hms_opt(0, 0, 0).unwrap(), // Starting date of the backtest is a NaiveDateTime not NaiveDate
        NaiveDate::from_ymd_opt(2023, 03, 10).unwrap().and_hms_opt(0, 0, 0).unwrap(), // Ending date of the backtest is a NaiveDateTime not NaiveDate
        Australia::Sydney, // the strategy time zone
        Duration::days(3), // the warmup duration, the duration of historical data we will pump through the strategy to warm up indicators etc before the strategy starts executing.
        set_subscriptions_initial(), //the closure or function used to set the subscriptions for the strategy. this allows us to have multiple subscription methods for more complex strategies
        strategy_event_sender, // the sender for the strategy events
        None,
        100
    ).await;

    on_strategy_events(strategy, strategy_event_receiver).await;
}

/// Here we listen for incoming data and build our custom strategy logic. this is where the magic happens.
pub async fn on_strategy_events(strategy: Arc<FundForgeStrategy>, mut event_receiver: mpsc::Receiver<StrategyEvent>)  {
    // Spawn a new task to listen for incoming data
    println!("Subscriptions: {:? }", strategy.subscriptions().await);
    // get history for specific date range for some analysis etc
    /*let history = history(set_subscriptions(), strategy.read().await.time_local().await - Duration::days(3), strategy.read().await.time_local().await).await;
    println!("History: {:?}", history);*/

    // get the strategy time in utc time
    // let time_utc = strategy.read().await.time_utc();
    // println!("UTC Time: {:?}", time_utc);
    // get the strategy time in local time
    // let local_time = strategy.read().await.time_local();
    // println!("Local Time: {:?}", local_time);
    let mut count = 0;
    while let Some(strategy_event) = event_receiver.recv().await {
        if !strategy.is_warmup_complete().await {
            continue;
        }

        match strategy_event {
            // when a drawing tool is added from some external source the event will also show up here (the tool itself will be added to the strategy.drawing_objects HashMap behind the scenes)
            StrategyEvent::DrawingToolEvents(_, drawing_tool_event, _) => {
                println!("Drawing Tool Event: {:?}", drawing_tool_event);
            }
            // our base data is received here and can be handled according to type
            StrategyEvent::TimeSlice(_, time_slice) => {
                // here we would process the time slice events and update the strategy state accordingly.

                for base_data in &time_slice {
                    match base_data {
                        BaseDataEnum::Price(_) => {}
                        BaseDataEnum::Candle(ref candle) => {
                            //println!("Time Slice: {:?}", &time_slice.len());
                            count += 1;
                            println!("{}...Candle {}: close:{} at {}", count, candle.symbol.name, candle.close, base_data.time_created_utc()); //note we automatically adjust for daylight savings based on historical daylight savings adjustments.
                            //println!("{}... time local {}", count, strategy.time_local().await);
                            println!("{}... time utc {}", count, strategy.time_utc().await);
                        }
                        BaseDataEnum::QuoteBar(_) => {}
                        BaseDataEnum::Tick(ref _tick) => {
                            //println!("{}... {}: close:{} at {}", count, tick.symbol.name, tick.price, base_data.time_created_utc()); //note we automatically adjust for daylight savings based on historical daylight savings adjustments.
                            //println!("{:?}", bar);

                            // for demo purposes: after 10 bars we will change our subscriptions closure and then calibrate our strategy subscriptions to its requirements.


                            // we can change the start-up set_subscriptions function at any time by calling state.set_subscriptions() and passing in a new closure or fn.
                            // here we could use advanced filtering logic to select assets based on some criteria.
                            /*
                              let subscriptions = vec![
                                  DataSubscription::new("AUD-USD".to_string(), DataVendor::Test, Resolution::Ticks(1), BaseDataType::Ticks, MarketType::Forex),
                                  DataSubscription::new("AUD-CAD".to_string(), DataVendor::Test, Resolution::Ticks(1), BaseDataType::Ticks, MarketType::Forex),
                              ];

                              // we can change the subscriptions at any time
                              strategy.subscriptions_update(subscriptions).await;

                              for subscription in strategy.state().subscriptions().await {
                                  println!("Subscription: {:? }", subscription);
                              }

                              //sleep for 10 seconds to allow us to read the console and check the new subscriptions updated correctly.
                              //tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;


                          if count /3 == 0 {
                                 strategy.enter_long(tick.symbol.clone(), Brokerage::Test, 1, "Entry".to_string(), "1".to_string()).await;
                             }

                             if count / 5 == 0 {
                                 strategy.exit_long(tick.symbol.clone(), Brokerage::Test, 1, "Entry".to_string(), "1".to_string()).await;
                             }
                           */
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
            StrategyEvent::DataSubscriptionEvents(_, event, _) => {
                println!("Data Subscription Event: {:?}", event);
            }
            // strategy controls are received here, this is useful for SemiAutomated mode. we could close all positions on a pause of the strategy, or custom handle other user inputs.
            StrategyEvent::StrategyControls(_, _, _) => {}
            StrategyEvent::ShutdownEvent(_, _) => break //Ok(()) // we could serialize data on shutdown here.
        }
        //simulate work
        //tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    event_receiver.close();
    println!("Strategy Event Loop Ended");
    //Ok(())
}