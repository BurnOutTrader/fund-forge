use std::sync::Arc;
use chrono::{Duration, NaiveDate};
use chrono_tz::Australia;
use tokio::sync::{mpsc, Notify};
use tokio::sync::mpsc::Receiver;
use ff_strategies::fund_forge_strategy::FundForgeStrategy;
use ff_standard_lib::apis::vendor::DataVendor;
use ff_standard_lib::indicators::built_in::average_true_range::AverageTrueRange;
use ff_standard_lib::indicators::indicator_enum::IndicatorEnum;
use ff_standard_lib::indicators::indicator_handler::IndicatorEvents;
use ff_standard_lib::indicators::indicators_trait::Indicators;
use ff_standard_lib::indicators::values::IndicatorValues;
use ff_standard_lib::server_connections::{initialize_clients, PlatformMode};
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::{MarketType, Resolution, StrategyMode};
use ff_standard_lib::standardized_types::rolling_window::RollingWindow;
use ff_standard_lib::standardized_types::subscriptions::{CandleType, DataSubscription, DataSubscriptionEvent};
use ff_standard_lib::standardized_types::strategy_events::{EventTimeSlice, StrategyEvent, StrategyInteractionMode};


/*fn defualt_broker_map() -> HashMap<DataVendor, Brokerage> {
    let mut broker_map = HashMap::new();
    broker_map.insert(DataVendor::Test, Brokerage::Test);
    broker_map
}*/


fn set_subscriptions_initial() -> Vec<DataSubscription> {
    let subscriptions: Vec<DataSubscription> = vec![
        DataSubscription::new("AUD-CAD".to_string(), DataVendor::Test, Resolution::Ticks(1), BaseDataType::Ticks, MarketType::Forex),
        DataSubscription::new("AUD-USD".to_string(), DataVendor::Test, Resolution::Ticks(1), BaseDataType::Ticks, MarketType::Forex),
        DataSubscription::new("AUD-CAD".to_string(), DataVendor::Test, Resolution::Minutes(15), BaseDataType::Candles, MarketType::Forex)
    ];
    subscriptions
}

#[tokio::main]
async fn main() {
    initialize_clients(&PlatformMode::SingleMachine).await.unwrap();
    let (strategy_event_sender, strategy_event_receiver) = mpsc::channel(1000);
    let notify = Arc::new(Notify::new());
    // we initialize our strategy as a new strategy, meaning we are not loading drawing tools or existing data from previous runs.
    let strategy = FundForgeStrategy::initialize(
        Some(String::from("test")), //if none is passed in an id will be generated based on the executing program name, todo! this needs to be upgraded in the future to be more reliable in Single and Multi machine modes.
        notify.clone(),
        StrategyMode::Backtest, // Backtest, Live, LivePaper
        StrategyInteractionMode::SemiAutomated,  // In semi-automated the strategy can interact with the user drawing tools and the user can change data subscriptions, in automated they cannot. // the base currency of the strategy
        NaiveDate::from_ymd_opt(2023, 03, 20).unwrap().and_hms_opt(0, 0, 0).unwrap(), // Starting date of the backtest is a NaiveDateTime not NaiveDate
        NaiveDate::from_ymd_opt(2023, 03, 30).unwrap().and_hms_opt(0, 0, 0).unwrap(), // Ending date of the backtest is a NaiveDateTime not NaiveDate
        Australia::Sydney, // the strategy time zone
        Duration::days(3), // the warmup duration, the duration of historical data we will pump through the strategy to warm up indicators etc before the strategy starts executing.
        set_subscriptions_initial(), //the closure or function used to set the subscriptions for the strategy. this allows us to have multiple subscription methods for more complex strategies
        strategy_event_sender, // the sender for the strategy events
        None,
        100,

        //strategy resolution, all data at a lower resolution will be consolidated to this resolution, if using tick data, you will want to set this at 1 second or less depending on the data granularity
        //this allows us full control over how the strategy buffers data and how it processes data, in live trading .
        Some(Duration::seconds(1))
    ).await;

    on_data_received(strategy, notify, strategy_event_receiver).await;
}

/// Here we listen for incoming data and build our custom strategy logic. this is where the magic happens.
pub async fn on_data_received(strategy: FundForgeStrategy, notify: Arc<Notify>, mut event_receiver: mpsc::Receiver<EventTimeSlice>)  {
    //notify.notify_one();
    // Spawn a new task to listen for incoming data
    //println!("Subscriptions: {:? }", strategy.subscriptions().await);
    let aud_cad_60m = DataSubscription::new_custom("AUD-CAD".to_string(), DataVendor::Test, Resolution::Minutes(60), BaseDataType::Candles, MarketType::Forex, CandleType::HeikinAshi);
    let aud_usd_15m = DataSubscription::new("AUD-USD".to_string(), DataVendor::Test, Resolution::Minutes(15), BaseDataType::Candles, MarketType::Forex);
    
    // Create a manually managed indicator directly in the on_data_received function (14 period ATR, which retains 100 historical IndicatorValues)
    let mut heikin_atr = AverageTrueRange::new(String::from("heikin_atr"), aud_usd_15m.clone(), 100, 14).await;
    let mut heikin_atr_history: RollingWindow<IndicatorValues> = RollingWindow::new(100);

    
    
    let mut warmup_complete = false;
    let mut count = 0;
    'strategy_loop: while let Some(event_slice) = event_receiver.recv().await {
        //println!("{}... time local {}", count, strategy.time_local().await);
        //println!("{}... time utc {}", count, strategy.time_utc().await);
        if warmup_complete {
            count += 1;
            if count == 100 {
                strategy.subscriptions_update(vec![aud_usd_15m.clone(), aud_cad_60m.clone()],100).await;
                // lets make another indicator to be handled by the IndicatorHandler, we need to wrap this as an indicator enum variat of the same name.
                let heikin_atr_20 = IndicatorEnum::AverageTrueRange(AverageTrueRange::new(String::from("heikin_atr_20"), aud_usd_15m.clone(), 100, 20).await);
                strategy.indicator_subscribe(heikin_atr_20).await;
            }
        }
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
                    'base_data_loop: for base_data in &time_slice {
                        match base_data {
                            BaseDataEnum::Price(_) => {}
                            BaseDataEnum::Candle(ref candle) => {
                                if base_data.subscription() == aud_cad_60m && candle.is_closed {
                                    /*heikin_atr.update_base_data(base_data);
                                    if heikin_atr.is_ready() {
                                        let atr = heikin_atr.current();
                                        println!("{}...{} ATR: {}", strategy.time_utc().await, aud_cad_60m.symbol.name, atr.unwrap());
                                    }*/
                                }
                                if warmup_complete {
                                    if candle.is_closed == true {
                                        //println!("{}...Candle {}, {}: close price:{} at {}, closed: {}, {}", strategy.time_utc().await, candle.resolution, candle.symbol.name, candle.close, base_data.time_created_utc(), candle.is_closed, candle.candle_type); //note we automatically adjust for daylight savings based on historical daylight savings adjustments.
                                        if count > 2000 {
                                            /*let three_bars_ago = &strategy.bar_index(&subscription, 3).await;
                                            println!("{}...{} Three bars ago: {:?}", count, subscription.symbol.name, three_bars_ago);
                                            //let data_current = &strategy.data_current(&subscription).await;
                                            //println!("{}...{} Current data: {:?}", count, subscription.symbol.name, data_current);


                                            let three_bars_ago = &strategy.bar_index(&subscription, 10).await;
                                            println!("{}...{} Three bars ago: {:?}", count, subscription.symbol.name, three_bars_ago);
                                            let data_current = &strategy.bar_current(&subscription).await;
                                            println!("{}...{} Current data: {:?}", count, subscription.symbol.name, data_current);*/
                                        }

                                    } else if candle.is_closed == false {
                                        //Todo Documents, Open bars get sent through with every tick, so you can always access the open bar using highest resolution.
                                        //println!("{}...Open Candle {}: close:{} at {}, is_closed: {}, candle_type: {:?}", strategy.time_utc().await, candle.symbol.name, candle.close, base_data.time_created_utc(), candle.is_closed, candle.candle_type); //note we automatically adjust for daylight savings based on historical daylight savings adjustments.
                                    }


                                   /*   if count /3 == 0 {
                                           strategy.enter_long("1".to_string(), candle.symbol.clone(), Brokerage::Test, 1, "Entry".to_string()).await;
                                      }
                                      if count / 5 == 0 {
                                       strategy.exit_long("1".to_string(), candle.symbol.clone(), Brokerage::Test, 1, "Entry".to_string()).await;
                                      }*/
                                }
                            }
                            BaseDataEnum::QuoteBar(_) => {}
                            BaseDataEnum::Tick(tick) => {
                                if !warmup_complete {
                                    // we could manually warm up indicators etc here.
                                    println!("{}...{} Tick: {}", strategy.time_utc().await, tick.symbol.name, base_data.time_created_utc());
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
                        IndicatorEvents::IndicatorTimeSlice(slice_event) => {
                            // we can see our auto manged indicator values for here.
                            for indicator_values in slice_event {
                                println!("{}: \n {:?}", indicator_values.name(), indicator_values.values());
                            }
                        }
                        IndicatorEvents::Replaced(replace_event) => {
                            println!("Indicator Replaced: {:?}", replace_event);
                        }
                    }
                }
            }
            notify.notify_one();
            //simulate work
            //tokio::time::sleep(tokio::time::Duration::from_nanos(50)).await;
        }
    }
    event_receiver.close();
    println!("Strategy Event Loop Ended");
}