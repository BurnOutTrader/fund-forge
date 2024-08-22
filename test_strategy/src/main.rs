use std::sync::Arc;
use chrono::{Duration, NaiveDate};
use chrono_tz::Australia;
use tokio::sync::{mpsc, Notify};
use ff_strategies::fund_forge_strategy::FundForgeStrategy;
use ff_standard_lib::apis::vendor::DataVendor;
use ff_standard_lib::server_connections::{PlatformMode};
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::{MarketType, Resolution, StrategyMode};
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
use ff_strategies::messages::strategy_events::{EventTimeSlice, StrategyEvent, StrategyInteractionMode};


/*fn defualt_broker_map() -> HashMap<DataVendor, Brokerage> {
    let mut broker_map = HashMap::new();
    broker_map.insert(DataVendor::Test, Brokerage::Test);
    broker_map
}*/

/// On initialization of a strategy we need to set the subscriptions function, this can be changed at any time to a new function, or the existing function can be used to recalibrate the subscriptions at runtime.
fn set_subscriptions_initial() -> Vec<DataSubscription> {
    let subscriptions: Vec<DataSubscription> = vec![
        DataSubscription::new("AUD-CAD".to_string(), DataVendor::Test, Resolution::Ticks(1), BaseDataType::Ticks, MarketType::Forex),
        DataSubscription::new("AUD-USD".to_string(), DataVendor::Test, Resolution::Ticks(1), BaseDataType::Ticks, MarketType::Forex),
    ];
    subscriptions
}

#[tokio::main]
async fn main() {
    let (strategy_event_sender, strategy_event_receiver) = mpsc::channel(1000000);
    let notify = Arc::new(Notify::new());
    // we initialize our strategy as a new strategy, meaning we are not loading drawing tools or existing data from previous runs.
    let strategy = FundForgeStrategy::initialize(
        Some(String::from("test")), //if none is passed in an id will be generated based on the executing program name, todo! this needs to be upgraded in the future to be more reliable in Single and Multi machine modes.
        notify.clone(),
        PlatformMode::SingleMachine, // SingleMachine or MultiMachine, multi machine mode for co-located servers, single machine for and MT5/Ninjatrader style platform
        StrategyMode::Backtest, // Backtest, Live, LivePaper
        StrategyInteractionMode::SemiAutomated,  // In semi-automated the strategy can interact with the user drawing tools and the user can change data subscriptions, in automated they cannot. // the base currency of the strategy
        NaiveDate::from_ymd_opt(2023, 03, 20).unwrap().and_hms_opt(0, 0, 0).unwrap(), // Starting date of the backtest is a NaiveDateTime not NaiveDate
        NaiveDate::from_ymd_opt(2023, 03, 30).unwrap().and_hms_opt(0, 0, 0).unwrap(), // Ending date of the backtest is a NaiveDateTime not NaiveDate
        Australia::Sydney, // the strategy time zone
        Duration::days(3), // the warmup duration, the duration of historical data we will pump through the strategy to warm up indicators etc before the strategy starts executing.
        set_subscriptions_initial(), //the closure or function used to set the subscriptions for the strategy. this allows us to have multiple subscription methods for more complex strategies
        strategy_event_sender, // the sender for the strategy events
        None,
        100
    ).await;

    on_data_received(strategy, notify, strategy_event_receiver).await;
}

/// Here we listen for incoming data and build our custom strategy logic. this is where the magic happens.
pub async fn on_data_received(strategy: FundForgeStrategy, notify: Arc<Notify>, mut event_receiver: mpsc::Receiver<EventTimeSlice>)  {
    // Spawn a new task to listen for incoming data
    //println!("Subscriptions: {:? }", strategy.subscriptions().await);

    let aud_cad_15s = DataSubscription::new("AUD-CAD".to_string(), DataVendor::Test, Resolution::Minutes(15), BaseDataType::Candles, MarketType::Forex);
    let aud_usd_15s = DataSubscription::new("AUD-USD".to_string(), DataVendor::Test, Resolution::Minutes(15), BaseDataType::Candles, MarketType::Forex);
    strategy.subscribe(aud_cad_15s.clone(),100).await;
    strategy.subscribe(aud_usd_15s.clone(),100).await;

    //todo()! 2. NEXT TASK, SEPARATE fn for each event type. use some sort of strategy trait instead of an actual object.. this will probably solve sync problems
    //todo()! 3. speed up warm up.. need to get it 100% precise

    let mut warmup_complete = false;
    'strategy_loop: while let Some(event_slice) = event_receiver.recv().await {
        for strategy_event in event_slice {
            match strategy_event {
                // when a drawing tool is added from some external source the event will also show up here (the tool itself will be added to the strategy.drawing_objects HashMap behind the scenes)
                StrategyEvent::DrawingToolEvents(_, drawing_tool_event, _) => {
                    println!("Drawing Tool Event: {:?}", drawing_tool_event);
                }
                // our base data is received here and can be handled according to type
                // we may return data we didn't subscribe to here, if we subscribed to a data type which the vendor can not supply we will return the consolidated data + the data used to create the consolidated data.
                StrategyEvent::TimeSlice(_time, time_slice) => {
                    // here we would process the time slice events and update the strategy state accordingly.
                    for base_data in &time_slice {
                        match base_data {
                            BaseDataEnum::Price(_) => {}
                            BaseDataEnum::Candle(ref candle) => {
                               /* if !warmup_complete {
                                    // we could manually warm up indicators etc here.
                                    continue;
                                }*/
                                if candle.is_closed {
                                    println!("{}...Candle {}: close:{} at {}, is_closed: {}", strategy.time_utc().await, candle.symbol.name, candle.close, base_data.time_created_utc(), candle.is_closed); //note we automatically adjust for daylight savings based on historical daylight savings adjustments.
                                } else {
                                    //Todo Documents, Open bars get sent through with every tick, so you can always access the open bar using highest resolution.
                                    //println!("{}...Open Candle {}: close:{} at {}, is_closed: {}", strategy.time_utc().await, candle.symbol.name, candle.close, base_data.time_created_utc(), candle.is_closed); //note we automatically adjust for daylight savings based on historical daylight savings adjustments.
                                }

                                //println!("{}... time local {}", count, strategy.time_local().await);
                                //println!("{}... time utc {}", count, strategy.time_utc().await);

                                /* if count == 51 {
                                     // check subscription 1

                                     let three_bars_ago = &strategy.data_index(&subscription, 3).await;
                                     println!("{}...{} Three bars ago: {:?}", count, subscription.symbol.name, three_bars_ago);

                                     //let data_current = &strategy.data_current(&subscription).await;
                                     //println!("{}...{} Current data: {:?}", count, subscription.symbol.name, data_current);

                                     // check subcription 2

                                     let three_bars_ago = &strategy.data_index(&subscription_2, 3).await;
                                     println!("{}...{} Three bars ago: {:?}",  count, subscription_2.symbol.name, three_bars_ago);

                                     //let data_current = &strategy.data_current(&subscription_2).await;
                                     //println!("{}...{} Current data: {:?}", count, subscription_2.symbol.name, data_current);
                                 }*/



                                /*  if count /3 == 0 {
                                       strategy.enter_long("1".to_string(), candle.symbol.clone(), Brokerage::Test, 1, "Entry".to_string()).await;
                                  }
                                  if count / 5 == 0 {
                                   strategy.exit_long("1".to_string(), candle.symbol.clone(), Brokerage::Test, 1, "Entry".to_string()).await;
                                  }*/
                            }
                            BaseDataEnum::QuoteBar(_) => {}
                            BaseDataEnum::Tick(_tick) => {
                                //println!("{}...{} Tick: {}", strategy.time_utc().await, tick.symbol.name, base_data.time_created_utc());
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
                StrategyEvent::ShutdownEvent(_, _) => {
                    break 'strategy_loop
                } //Ok(()) // we could serialize data on shutdown here.
                StrategyEvent::WarmUpComplete(_) => {
                    warmup_complete = true;
                }
            }
            notify.notify_one();
            //simulate work
            //tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
    }
    event_receiver.close();
    println!("Strategy Event Loop Ended");
}