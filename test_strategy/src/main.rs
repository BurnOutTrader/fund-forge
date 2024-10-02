use chrono::{Duration, NaiveDate};
use chrono_tz::Australia;
use colored::Colorize;
use ff_standard_lib::strategies::indicators::events::IndicatorEvents;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::{MarketType, StrategyMode};
use ff_standard_lib::strategies::strategy_events::{StrategyControls, StrategyEvent, StrategyEventBuffer};
use ff_standard_lib::standardized_types::subscriptions::{CandleType, DataSubscription, SymbolName};
use ff_standard_lib::strategies::fund_forge_strategy::FundForgeStrategy;
use rust_decimal_macros::dec;
use tokio::sync::mpsc;
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::strategies::indicators::built_in::average_true_range::AverageTrueRange;
use ff_standard_lib::strategies::indicators::indicator_enum::IndicatorEnum;
use ff_standard_lib::strategies::indicators::indicators_trait::IndicatorName;
use ff_standard_lib::strategies::ledgers::{AccountId, Currency};
use ff_standard_lib::standardized_types::base_data::candle::Candle;
use ff_standard_lib::standardized_types::base_data::quotebar::QuoteBar;
use ff_standard_lib::gui_types::settings::Color;
use ff_standard_lib::client_features::connection_types::GUI_DISABLED;
use ff_standard_lib::standardized_types::orders::OrderId;
use ff_standard_lib::standardized_types::resolution::Resolution;

// to launch on separate machine
#[tokio::main]
async fn main() {
    let (strategy_event_sender, strategy_event_receiver) = mpsc::channel(1000);
    // we initialize our strategy as a new strategy, meaning we are not loading drawing tools or existing data from previous runs.
    let strategy = FundForgeStrategy::initialize(
        //ToDo: You can Test Live paper using the simulated data feed. Live paper might panic subscribing in event loop due to live warm up not being complete and accessing history to early.
        StrategyMode::Backtest, // Backtest, Live, LivePaper
        dec!(100000),
        Currency::USD,
        NaiveDate::from_ymd_opt(2024, 6, 19).unwrap().and_hms_opt(0, 0, 0).unwrap(), // Starting date of the backtest is a NaiveDateTime not NaiveDate
        NaiveDate::from_ymd_opt(2024, 06, 21).unwrap().and_hms_opt(0, 0, 0).unwrap(), // Ending date of the backtest is a NaiveDateTime not NaiveDate
        Australia::Sydney,                      // the strategy time zone
        Duration::days(1), // the warmup duration, the duration of historical data we will pump through the strategy to warm up indicators etc before the strategy starts executing.
        vec![
            /*DataSubscription::new(
                SymbolName::from("EUR-USD"),
                DataVendor::Test,
                Resolution::Instant,
                BaseDataType::Quotes,
                MarketType::Forex,
            ),
            DataSubscription::new(
                SymbolName::from("AUD-CAD"),
                DataVendor::Test,
                Resolution::Instant,
                BaseDataType::Quotes,
                MarketType::Forex,
            ),*/
            DataSubscription::new(
                SymbolName::from("EUR-USD"),
                DataVendor::Test,
                Resolution::Minutes(3),
                BaseDataType::QuoteBars,
                MarketType::Forex,
            ),
            DataSubscription::new_custom(
                 SymbolName::from("AUD-CAD"),
                 DataVendor::Test,
                 Resolution::Minutes(3),
                 MarketType::Forex,
                 CandleType::HeikinAshi
             ),],
        false,
        100,
        strategy_event_sender, // the sender for the strategy events
        //strategy resolution in milliseconds, all data at a lower resolution will be consolidated to this resolution, if using tick data, you will want to set this at 100 or less depending on the data granularity
        //this allows us full control over how the strategy buffers data and how it processes data, in live trading and backtesting.
        //ToDo: Test Un-Buffered engines (None) vs Buffered Some(Duration)
        Some(core::time::Duration::from_millis(100)),
        //None,

        GUI_DISABLED
    ).await;

    on_data_received(strategy, strategy_event_receiver).await;
}

/// Here we listen for incoming data and build our custom strategy logic. this is where the magic happens.
pub async fn on_data_received(
    strategy: FundForgeStrategy,
    mut event_receiver: mpsc::Receiver<StrategyEventBuffer>,
) {
    let heikin_atr_5 = IndicatorEnum::AverageTrueRange(
        AverageTrueRange::new(IndicatorName::from("heikin_atr_5"),
              DataSubscription::new(
                  SymbolName::from("EUR-USD"),
                  DataVendor::Test,
                  Resolution::Minutes(3),
                  BaseDataType::QuoteBars,
                  MarketType::Forex,
              ),
            100,
            5,
            Some(Color::new(50,50,50))
        ).await,
    );
    //if you set auto subscribe to false and change the resolution, the strategy will intentionally panic to let you know you won't have data for the indicator
    strategy.subscribe_indicator(heikin_atr_5, true).await;
    let mut count = 0;
    let brokerage = Brokerage::Test;
    let mut warmup_complete = false;
    let mut bars_since_entry_1 = 0;
    let mut bars_since_entry_2 = 0;
    let account_name = AccountId::from("TestAccount");
    // The engine will send a buffer of strategy events at the specified buffer interval, it will send an empty buffer if no events were buffered in the period.
    'strategy_loop: while let Some(event_slice) = event_receiver.recv().await {
        for (_time, strategy_event) in event_slice.iter() {
            match strategy_event {
                // when a drawing tool is added from some external source the event will also show up here (the tool itself will be added to the strategy.drawing_objects HashMap behind the scenes)
                StrategyEvent::DrawingToolEvents(event) => {
                    println!("Strategy: Drawing Tool Event: {:?}", event);
                }

                StrategyEvent::TimeSlice(time_slice) => {
                    // here we would process the time slice events and update the strategy state accordingly.
                    for base_data in time_slice.iter() {
                        // only data we specifically subscribe to show up here, if the data is building from ticks but we didn't subscribe to ticks specifically, ticks won't show up but the subscribed resolution will.
                        match base_data {
                            BaseDataEnum::Candle(candle) => {
                                // Place trades based on the AUD-CAD Heikin Ashi Candles
                                if candle.is_closed == true {
                                    let msg = format!("{} {} {} Close: {}, {}", candle.symbol.name, candle.resolution, candle.candle_type, candle.close, candle.time_closed_local(strategy.time_zone()));
                                    if candle.close == candle.open {
                                        println!("{}", msg.as_str().blue())
                                    } else {
                                        match candle.close > candle.open {
                                            true => println!("{}", msg.as_str().green()),
                                            false => println!("{}", msg.as_str().red()),
                                        }
                                    }

                                    if !warmup_complete {
                                        continue;
                                    }

                                    //index 0 is the same candle we are currently unwrapping in the base data enum, since closed candles are added to history before we get them
                                  /*  let candle_10_ago = strategy.candle_index(&base_data.subscription(), 11).unwrap();
                                    let msg = format!("{} {} 10 Candles Ago Close: {}, {}", candle_10_ago.symbol.name, candle_10_ago.resolution, candle_10_ago.close, candle_10_ago.time_closed_local(strategy.time_zone()));
                                    println!("{}", msg.as_str().on_bright_black());*/

                                    if candle.resolution == Resolution::Minutes(20) && candle.symbol.name == "AUD-CAD" && candle.symbol.data_vendor == DataVendor::Test {
                                        let last_candle: Candle = strategy.candle_index(&base_data.subscription(), 1).unwrap();
                                        let is_short = strategy.is_short(&brokerage, &account_name, &candle.symbol.name);

                                        if candle.close < last_candle.low
                                            && !is_short {
                                            let _entry_order_id = strategy.enter_short(&candle.symbol.name, &account_name, &brokerage, dec!(10), String::from("Enter Short")).await;
                                            bars_since_entry_2 = 0;
                                        }

                                        if is_short {
                                            bars_since_entry_2 += 1;
                                        }

                                        if bars_since_entry_2 > 10
                                            && is_short {
                                            let _exit_order_id = strategy.exit_short(&candle.symbol.name, &account_name, &brokerage, dec!(10), String::from("Exit Short")).await;
                                            bars_since_entry_2 = 0;
                                        }
                                    }

                                    if count == 20 {
                                        let msg = "Subscribing to new Candle Resolution and warming subscription, this will take time as we don't have warm up data in memory, in backtesting we have to pause, in live we will do this as a background task".to_string();
                                        println!("{}",msg.as_str().purple());
                                        let heikin_20_min = DataSubscription::new_custom(
                                            SymbolName::from("AUD-CAD"),
                                            DataVendor::Test,
                                            Resolution::Minutes(20),
                                            MarketType::Forex,
                                            CandleType::HeikinAshi
                                        );

                                        // subscribing to data subscriptions returns a result, the result is a DataSubscription event, Ok(FailedToSubscribe) or Err(Subscribed)
                                        // In live or live paper, we could have 2 failures, 1 here on client side, and another event that comes from server side, if it fails on the api for any reason.
                                        // The subscription handler should catch problems before the server, but there is always a possibility that a server side failure to subscribe occurs.
                                        match strategy.subscribe(heikin_20_min, 3, true).await {
                                            Ok(ok) => {
                                                let msg = format!("{}", ok.to_string());
                                                println!("{}", msg.as_str().bright_magenta())
                                            }
                                            Err(e) =>  {
                                                let msg = format!("{}", e.to_string());
                                                println!("{}", msg.as_str().on_bright_white())
                                            }
                                        }
                                    }
                                }
                            }
                            BaseDataEnum::QuoteBar(quotebar) => {
                                // Place trades based on the EUR-USD QuoteBars
                                if quotebar.is_closed == true {
                                    let msg = format!("{} {} QuoteBar Close: {}, {}", quotebar.symbol.name, quotebar.resolution, quotebar.bid_close, quotebar.time_closed_local(strategy.time_zone()));
                                    if quotebar.bid_close == quotebar.bid_open {
                                        println!("{}", msg.as_str().blue())
                                    } else {
                                        match quotebar.bid_close > quotebar.bid_open {
                                            true => println!("{}", msg.as_str().green()),
                                            false => println!("{}", msg.as_str().red()),
                                        }
                                    }

                                    if !warmup_complete {
                                        continue;
                                    }

                                    if quotebar.resolution == Resolution::Minutes(3) && quotebar.symbol.name == "EUR-USD" && quotebar.symbol.data_vendor == DataVendor::Test {
                                        let last_bar: QuoteBar = strategy.bar_index(&base_data.subscription(), 1).unwrap();
                                        let is_long: bool = strategy.is_long(&brokerage, &account_name, &quotebar.symbol.name);

                                        if quotebar.bid_close > last_bar.bid_high
                                            && !is_long {
                                            let _entry_order_id: OrderId = strategy.enter_long(&quotebar.symbol.name, &account_name, &brokerage, dec!(10), String::from("Enter Long")).await;
                                            bars_since_entry_1 = 0;
                                        }

                                        if is_long {
                                            bars_since_entry_1 += 1;
                                        }

                                        if bars_since_entry_1 > 10
                                            && is_long {
                                            let _exit_order_id: OrderId = strategy.exit_long(&quotebar.symbol.name, &account_name, &brokerage, dec!(10), String::from("Exit Long")).await;
                                            bars_since_entry_1 = 0;
                                        }
                                    }

                                    count += 1;

                                    if count == 50 {
                                        let msg = "Subscribing to new indicator heikin_atr3_5min and warming up subscriptions".to_string();
                                        println!("{}",msg.as_str().purple());
                                        // this will test both our auto warm up for indicators and data subscriptions
                                        let heikin_atr3_15min = IndicatorEnum::AverageTrueRange(
                                            AverageTrueRange::new(
                                                IndicatorName::from("heikin_atr3_15min"),
                                                DataSubscription::new(
                                                    SymbolName::from("EUR-USD"),
                                                    DataVendor::Test,
                                                    Resolution::Minutes(15),
                                                    BaseDataType::QuoteBars,
                                                    MarketType::Forex,
                                                ),
                                                5,
                                                3,
                                                Some(Color::new(30,30,50))
                                            ).await,
                                        );
                                        // we auto subscribe to the subscription, this will warm up the data subscription, which the indicator will then use to warm up.
                                        // the indicator would still warm up if this was false, but if we  don't have the data subscription already subscribed the strategy will deliberately panic
                                        let event = strategy.subscribe_indicator(heikin_atr3_15min, true).await;
                                        let msg = format!("{}", event);
                                        println!("{}", msg.as_str().bright_purple());
                                    }
                                }
                                //do something with the current open bar
                                if !quotebar.is_closed {
                                    //println!("Open bar time: {}", time)
                                }
                            }
                            BaseDataEnum::Tick(_tick) => {}
                            BaseDataEnum::Quote(quote) => {
                                // primary data feed won't show up in event loop unless specifically subscribed by the strategy
                                let msg = format!("{} Quote: {}, Local Time {}", quote.symbol.name, base_data.time_closed_utc(), quote.time_local(strategy.time_zone()));
                                println!("{}", msg.as_str().purple());
                            }
                            BaseDataEnum::Fundamental(_fundamental) => {}
                        }
                    }
                }

                // order updates are received here, excluding order creation events, the event loop here starts with an OrderEvent::Accepted event and ends with the last fill, rejection or cancellation events.
                StrategyEvent::OrderEvents(event) => {
                    strategy.print_ledgers();
                    let msg = format!("{}, Strategy: Order Event: {}", strategy.time_utc(), event);
                    println!("{}", msg.as_str().bright_yellow());
                }

                // if an external source adds or removes a data subscription it will show up here, this is useful for SemiAutomated mode
                StrategyEvent::DataSubscriptionEvent(event) => {
                        let msg = format!("Strategy: Data Subscription Event: {}", event);
                        println!("{}", msg.as_str().bright_magenta());
                }

                // strategy controls are received here, this is useful for SemiAutomated mode. we could close all positions on a pause of the strategy, or custom handle other user inputs.
                StrategyEvent::StrategyControls(control) => {
                    match control {
                        StrategyControls::Continue => {}
                        StrategyControls::Pause => {}
                        StrategyControls::Stop => {}
                        StrategyControls::Start => {}
                        StrategyControls::Delay(_) => {}
                        StrategyControls::Custom(_) => {}
                        StrategyControls::CustomBytes(_, _) => {}
                    }
                }

                StrategyEvent::ShutdownEvent(event) => {
                    let msg = format!("{}",event);
                    println!("{}", msg.as_str().bright_magenta());
                    strategy.export_trades(&String::from("./trades exports"));
                    strategy.print_ledgers();
                    //we should handle shutdown gracefully by first ending the strategy loop.
                    break 'strategy_loop
                },

                StrategyEvent::WarmUpComplete => {
                    let msg = String::from("Strategy: Warmup Complete");
                    println!("{}", msg.as_str().bright_magenta());
                    warmup_complete = true;
                }

                StrategyEvent::IndicatorEvent(indicator_event) => {
                    //we can handle indicator events here, this is useful for debugging and monitoring the state of the indicators.
                    match indicator_event {
                        IndicatorEvents::IndicatorAdded(added_event) => {
                            let msg = format!("Strategy:Indicator Added: {:?}", added_event);
                            println!("{}", msg.as_str().yellow());
                        }
                        IndicatorEvents::IndicatorRemoved(removed_event) => {
                            let msg = format!("Strategy:Indicator Removed: {:?}", removed_event);
                            println!("{}", msg.as_str().yellow());
                        }
                        IndicatorEvents::IndicatorTimeSlice(slice_event) => {
                            // we can see our auto manged indicator values for here.
                            for indicator_values in slice_event {
                                for (_name, plot) in indicator_values.plots(){
                                    let msg = format!("{}: {}: {:?}, {}", indicator_values.name, plot.name, plot.value, strategy.time_local());
                                    println!("{}", msg.as_str().bright_cyan());
                                }
                            }
                        }
                        IndicatorEvents::Replaced(replace_event) => {
                            let msg = format!("Strategy:Indicator Replaced: {:?}", replace_event);
                            println!("{}", msg.as_str().yellow());
                        }
                    }

                    // we could also get the automanaged indicator values from teh strategy at any time.
                }

                StrategyEvent::PositionEvents(event) => {
                    let msg = format!("{}", event);
                    println!("{}", msg.as_str().yellow());
                }
            }
        }
    }
    event_receiver.close();
    println!("Strategy: Event Loop Ended");
}
