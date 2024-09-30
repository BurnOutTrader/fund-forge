use chrono::{Duration, NaiveDate};
use chrono_tz::{Australia};
use ff_standard_lib::indicators::indicator_handler::IndicatorEvents;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::{MarketType, Resolution, StrategyMode};
use ff_standard_lib::standardized_types::strategy_events::{StrategyEventBuffer, StrategyControls, StrategyEvent, StrategyInteractionMode};
use ff_standard_lib::standardized_types::subscriptions::{CandleType, DataSubscription, SymbolName};
use ff_strategies::fund_forge_strategy::FundForgeStrategy;
use rust_decimal_macros::dec;
use tokio::sync::{mpsc};
use ff_standard_lib::apis::brokerage::broker_enum::Brokerage;
use ff_standard_lib::apis::data_vendor::datavendor_enum::DataVendor;
use ff_standard_lib::indicators::built_in::average_true_range::AverageTrueRange;
use ff_standard_lib::indicators::indicator_enum::IndicatorEnum;
use ff_standard_lib::indicators::indicators_trait::IndicatorName;
use ff_standard_lib::server_connections::{GUI_DISABLED};
use ff_standard_lib::standardized_types::accounts::ledgers::{AccountId, Currency};
use ff_standard_lib::standardized_types::base_data::candle::Candle;
use ff_standard_lib::standardized_types::base_data::quotebar::QuoteBar;
use ff_standard_lib::standardized_types::Color;
use ff_standard_lib::standardized_types::orders::orders::OrderUpdateEvent;
use ff_standard_lib::standardized_types::rolling_window::RollingWindow;

// to launch on separate machine
#[tokio::main]
async fn main() {
    let (strategy_event_sender, strategy_event_receiver) = mpsc::channel(1000);
    // we initialize our strategy as a new strategy, meaning we are not loading drawing tools or existing data from previous runs.
    let strategy = FundForgeStrategy::initialize(
        StrategyMode::Backtest, // Backtest, Live, LivePaper
        dec!(100000),
        Currency::USD,
        StrategyInteractionMode::SemiAutomated, // In semi-automated the strategy can interact with the user drawing tools and the user can change data subscriptions, in automated they cannot. // the base currency of the strategy
        NaiveDate::from_ymd_opt(2024, 6, 19)
            .unwrap()
            .and_hms_opt(0, 0, 0).unwrap(), // Starting date of the backtest is a NaiveDateTime not NaiveDate
        NaiveDate::from_ymd_opt(2024, 06, 21)
            .unwrap()
            .and_hms_opt(0, 0, 0).unwrap(), // Ending date of the backtest is a NaiveDateTime not NaiveDate
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
        None,
        //strategy resolution in milliseconds, all data at a lower resolution will be consolidated to this resolution, if using tick data, you will want to set this at 100 or less depending on the data granularity
        //this allows us full control over how the strategy buffers data and how it processes data, in live trading and backtesting.
        Some(core::time::Duration::from_millis(100)),
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
    strategy.indicator_subscribe(heikin_atr_5, false).await;

    let brokerage = Brokerage::Test;
    let mut warmup_complete = false;
    let mut bars_since_entry_1 = 0;
    let mut bars_since_entry_2 = 0;
    let mut history_1 : RollingWindow<QuoteBar> = RollingWindow::new(10);
    let mut history_2 : RollingWindow<Candle> = RollingWindow::new(10);
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
                                    println!("Candle {}: {}, Local Time: {}", candle.symbol.name, candle.time_created_utc(), candle.time_created_local(strategy.time_zone()));
                                    history_2.add(candle.clone());
                                    let last_bar =
                                        match history_2.get(1) {
                                            None => {
                                                continue;
                                            },
                                            Some(bar) => bar
                                        };

                                    if !warmup_complete {
                                        continue;
                                    }

                                    let is_long = strategy.is_long(&brokerage, &account_name, &candle.symbol.name);

                                    if candle.close > last_bar.high
                                        && !is_long
                                    {
                                        let _entry_order_id = strategy.enter_long(&candle.symbol.name, &account_name, &brokerage, dec!(1), String::from("Enter Long")).await;
                                        bars_since_entry_2 = 0;
                                    }

                                    if bars_since_entry_2 > 10
                                        &&is_long
                                    {
                                        let _exit_order_id = strategy.exit_long(&candle.symbol.name, &account_name, &brokerage,dec!(1), String::from("Exit Long")).await;
                                        bars_since_entry_2 = 0;
                                    }

                                    if is_long {
                                        bars_since_entry_2 += 1;
                                    }
                                }
                            }
                            BaseDataEnum::QuoteBar(quotebar) => {
                                // Place trades based on the EUR-USD QuoteBars
                                if quotebar.is_closed == true {
                                    println!("QuoteBar {}: {}, Local Time {}", quotebar.symbol.name, quotebar.time_created_utc(), quotebar.time_created_local(strategy.time_zone()));
                                    history_1.add(quotebar.clone());
                                    let last_bar =
                                    match history_1.get(1) {
                                        None => {
                                            continue;
                                        },
                                        Some(bar) => bar
                                    };

                                    if !warmup_complete {
                                        continue;
                                    }
                                    //todo, make a candle_index and quote_bar_index to get specific data types and save pattern matching

                                    let is_long = strategy.is_long(&brokerage, &account_name, &quotebar.symbol.name);

                                    if quotebar.bid_close > last_bar.bid_high
                                        && !is_long
                                    {
                                        let _entry_order_id = strategy.enter_long(&quotebar.symbol.name, &account_name, &brokerage, dec!(1), String::from("Enter Long")).await;
                                        bars_since_entry_1 = 0;
                                    }

                                    if bars_since_entry_1 > 10
                                        //&& last_bar.bid_close < two_bars_ago.bid_low
                                        && is_long
                                    {
                                        let _exit_order_id = strategy.exit_long(&quotebar.symbol.name, &account_name, &brokerage,dec!(1), String::from("Exit Long")).await;
                                        bars_since_entry_1 = 0;
                                    }

                                    if is_long {
                                        bars_since_entry_1 += 1;
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
                                println!("{} Quote: {}, Local Time {}", quote.symbol.name, base_data.time_created_utc(), quote.time_local(strategy.time_zone()));
                            }
                            BaseDataEnum::Fundamental(_fundamental) => {}
                        }
                    }
                }

                // order updates are received here, excluding order creation events, the event loop here starts with an OrderEvent::Accepted event and ends with the last fill, rejection or cancellation events.
                StrategyEvent::OrderEvents(event) => {
                    //strategy.print_ledgers();
                    match &event {
                        OrderUpdateEvent::OrderAccepted { brokerage, account_id, order_id } => {
                            println!("{}: {:?}", order_id, strategy.print_ledger(brokerage.clone(), account_id));
                        }
                        OrderUpdateEvent::OrderFilled { brokerage, account_id, order_id } => {
                            println!("{}: {:?}", order_id, strategy.print_ledger(brokerage.clone(), account_id));
                        },
                        OrderUpdateEvent::OrderPartiallyFilled { brokerage, account_id, order_id } => {
                            println!("{}: {:?}", order_id, strategy.print_ledger(brokerage.clone(), account_id));
                        }
                        OrderUpdateEvent::OrderCancelled { brokerage, account_id, order_id } => {
                            println!("{}: {:?}", order_id, strategy.print_ledger(brokerage.clone(), account_id));
                        }
                        OrderUpdateEvent::OrderRejected { brokerage, account_id, order_id, reason } => {
                            println!("{}: {:?}: {}", order_id, strategy.print_ledger(brokerage.clone(), account_id), reason);
                        }
                        OrderUpdateEvent::OrderUpdated { brokerage, account_id, order_id, order} => {
                            println!("{}: {:?}", order_id, strategy.print_ledger(brokerage.clone(), account_id));
                        }
                        OrderUpdateEvent::OrderUpdateRejected { brokerage, account_id, order_id, reason } => {
                            println!("{}: {:?}: {}", order_id, strategy.print_ledger(brokerage.clone(), account_id), reason);
                        }
                    };
                    println!("{}, Strategy: Order Event: {:?}", strategy.time_utc(), event);
                }

                // if an external source adds or removes a data subscription it will show up here, this is useful for SemiAutomated mode
                StrategyEvent::DataSubscriptionEvent(event) => {
                        println!("Strategy: Data Subscription Event: {:?}", event);
                }

                // strategy controls are received here, this is useful for SemiAutomated mode. we could close all positions on a pause of the strategy, or custom handle other user inputs.
                StrategyEvent::StrategyControls(control) => {
                    match control {
                        StrategyControls::Continue => {}
                        StrategyControls::Pause => {}
                        StrategyControls::Stop => {}
                        StrategyControls::Start => {}
                        StrategyControls::Delay(_) => {}
                    }
                }

                StrategyEvent::ShutdownEvent(event) => {
                    println!("{}",event);
                    strategy.export_trades(&String::from("./trades exports"));
                    strategy.print_ledgers().await;
                    //we should handle shutdown gracefully by first ending the strategy loop.
                    break 'strategy_loop
                },

                StrategyEvent::WarmUpComplete{} => {
                    println!("Strategy: Warmup Complete");
                    warmup_complete = true;
                }

                StrategyEvent::IndicatorEvent(indicator_event) => {
                    //we can handle indicator events here, this is useful for debugging and monitoring the state of the indicators.
                    match indicator_event {
                        IndicatorEvents::IndicatorAdded(added_event) => {
                            println!("Strategy:Indicator Added: {:?}", added_event);
                        }
                        IndicatorEvents::IndicatorRemoved(removed_event) => {
                            println!("Strategy:Indicator Removed: {:?}", removed_event);
                        }
                        IndicatorEvents::IndicatorTimeSlice(slice_event) => {
                            // we can see our auto manged indicator values for here.
                            for indicator_values in slice_event {
                                for (_name, plot) in indicator_values.plots(){
                                    println!("{}: {}: {:?}", indicator_values.name, plot.name, plot.value);
                                }
                            }
                        }
                        IndicatorEvents::Replaced(replace_event) => {
                            println!("Strategy:Indicator Replaced: {:?}", replace_event);
                        }
                    }

                    // we could also get the automanaged indicator values from teh strategy at any time.
                }

                StrategyEvent::PositionEvents(event) => {
                    println!("{:?}", event);
                }
            }
        }
    }
    event_receiver.close();
    println!("Strategy: Event Loop Ended");
}
