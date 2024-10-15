use std::cmp::PartialEq;
use chrono::{Duration, NaiveDate};
use chrono_tz::Australia;
use colored::Colorize;
use ff_rithmic_api::systems::RithmicSystem;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::{FuturesExchange, MarketType, StrategyMode};
use ff_standard_lib::strategies::strategy_events::{StrategyEvent, StrategyEventBuffer};
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, SymbolName};
use ff_standard_lib::strategies::fund_forge_strategy::FundForgeStrategy;
use rust_decimal_macros::dec;
use tokio::sync::{mpsc};
use ff_standard_lib::gui_types::settings::Color;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::orders::OrderUpdateEvent;
use ff_standard_lib::strategies::ledgers::{AccountId, Currency};
use ff_standard_lib::standardized_types::position::PositionUpdateEvent;
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::strategies::indicators::built_in::average_true_range::AverageTrueRange;
use ff_standard_lib::strategies::indicators::indicator_enum::IndicatorEnum;
use ff_standard_lib::strategies::indicators::indicator_events::IndicatorEvents;
use ff_standard_lib::strategies::indicators::indicators_trait::IndicatorName;

// to launch on separate machine
#[tokio::main]
async fn main() {
    let (strategy_event_sender, strategy_event_receiver) = mpsc::channel(1000);
    let strategy = FundForgeStrategy::initialize(
        StrategyMode::LivePaperTrading,
        dec!(100000),
        Currency::USD,
        NaiveDate::from_ymd_opt(2024, 6, 5).unwrap().and_hms_opt(0, 0, 0).unwrap(),
        NaiveDate::from_ymd_opt(2024, 08, 28).unwrap().and_hms_opt(0, 0, 0).unwrap(),
        Australia::Sydney,
        Duration::hours(5),
        vec![
       /*     DataSubscription::new(
                SymbolName::from("MNQ"),
                DataVendor::Rithmic(RithmicSystem::TopstepTrader),
                Resolution::Ticks(1),
                BaseDataType::Ticks,
                MarketType::Futures(FuturesExchange::CME)
            ),*/
      /*      DataSubscription::new(
                SymbolName::from("MNQ"),
                DataVendor::Rithmic(RithmicSystem::TopstepTrader),
                Resolution::Instant,
                BaseDataType::Quotes,
                MarketType::Futures(FuturesExchange::CME)
            ),*/
           DataSubscription::new(
                SymbolName::from("MNQ"),
                DataVendor::Rithmic(RithmicSystem::TopstepTrader),
                Resolution::Seconds(1),
                BaseDataType::Candles,
                MarketType::Futures(FuturesExchange::CME)
            ),
      /*      DataSubscription::new(
                SymbolName::from("MNQ"),
                DataVendor::Rithmic(RithmicSystem::TopstepTrader),
                Resolution::Seconds(1),
                BaseDataType::QuoteBars,
                MarketType::Futures(FuturesExchange::CME)
            ),*/
        ],
        true,
        100,
        strategy_event_sender,
        core::time::Duration::from_millis(50),
        false,
        true,
    ).await;

    on_data_received(strategy, strategy_event_receiver).await;
}

#[derive(Clone, PartialEq, Debug)]
enum LastSide {
    Long,
    Flat,
    Short
}

pub async fn on_data_received(
    strategy: FundForgeStrategy,
    mut event_receiver: mpsc::Receiver<StrategyEventBuffer>,
) {
 /*   let atr_5 = IndicatorEnum::AverageTrueRange(
        AverageTrueRange::new(
            IndicatorName::from("atr_5"),
            // The subscription for the indicator
            DataSubscription::new(
                SymbolName::from("MNQ"),
                DataVendor::Rithmic(RithmicSystem::TopstepTrader),
                Resolution::Seconds(1),
                BaseDataType::Candles,
                MarketType::Futures(FuturesExchange::CME)
            ),

            // history to retain
            10,

            // atr period
            5,

            // Plot color for GUI or println!()
            Color::new (128, 0, 128)
        ).await,
    );

    //if you set auto subscribe to false and change the resolution, the strategy will intentionally panic to let you know you won't have data for the indicator
    strategy.subscribe_indicator(atr_5, true).await;*/

    let brokerage = Brokerage::Test;
    let mut warmup_complete = false;
    let account_1 = AccountId::from("Test_Account_1");
    let mut last_side = LastSide::Flat;
    println!("Staring Strategy Loop");
    // The engine will send a buffer of strategy events at the specified buffer interval, it will send an empty buffer if no events were buffered in the period.
    'strategy_loop: while let Some(event_slice) = event_receiver.recv().await {
        //println!("Strategy: Buffer Received Time: {}", strategy.time_local());
        for (_time, strategy_event) in event_slice.iter() {
            //println!("Strategy: Buffer Event Time: {}", strategy.time_zone().from_utc_datetime(&time.naive_utc()));
            match strategy_event {
                StrategyEvent::TimeSlice(time_slice) => {
                    // here we would process the time slice events and update the strategy state accordingly.
                    for base_data in time_slice.iter() {
                        // only data we specifically subscribe to show up here, if the data is building from ticks but we didn't subscribe to ticks specifically, ticks won't show up but the subscribed resolution will.
                        match base_data {
                            BaseDataEnum::Tick(tick) => {
                              // println!("{}", tick);
                            }
                            BaseDataEnum::Candle(candle) => {
                                // Place trades based on the AUD-CAD Heikin Ashi Candles
                                if candle.is_closed == true {
                                    let msg = format!("{} {} {} Close: {}, {}", candle.symbol.name, candle.resolution, candle.candle_type, candle.close, candle.time_closed_local(strategy.time_zone()));
                                    if candle.close == candle.open {
                                        println!("{}", msg.as_str().blue())
                                    } else {
                                        match candle.close > candle.open {
                                            true => println!("{}", msg.as_str().bright_green()),
                                            false => println!("{}", msg.as_str().bright_red()),
                                        }
                                    }

                                  /*  if !warmup_complete {
                                        continue;
                                    }

                                    //LONG CONDITIONS
                                    {
                                        // ENTER LONG
                                        let is_flat = strategy.is_flat(&brokerage, &account_1, &candle.symbol.name);
                                        // buy AUD-CAD if consecutive green HA candles if our other account is long on EUR
                                        if is_flat
                                            && candle.close > candle.open
                                            && last_side != LastSide::Long
                                        {
                                            let _entry_order_id = strategy.enter_long(&candle.symbol.name, &account_1, &brokerage, dec!(7), String::from("Enter Long")).await;
                                            println!("Strategy: Enter Long, Time {}", strategy.time_local());
                                            last_side = LastSide::Long;
                                        }

                                        // ADD LONG
                                        let is_long = strategy.is_long(&brokerage, &account_1, &candle.symbol.name);
                                        let long_pnl = strategy.pnl(&brokerage, &account_1, &candle.symbol.name);
                                        let position_size: Decimal = strategy.position_size(&brokerage, &account_1, &candle.symbol.name);
                                        if is_long
                                            && long_pnl > dec!(150.0)
                                            && long_pnl < dec!(300.0)
                                            && position_size < dec!(21)
                                        {
                                            let _add_order_id = strategy.enter_long(&candle.symbol.name, &account_1, &brokerage, dec!( 7), String::from("Add Long")).await;
                                            println!("Strategy: Add Long, Time {}", strategy.time_local());
                                        }

                                        // LONG SL+TP
                                        if is_long && long_pnl > dec!(500.0)
                                        {
                                            let _exit_order_id = strategy.exit_long(&candle.symbol.name, &account_1, &brokerage, position_size, String::from("Exit Long Take Profit")).await;
                                            println!("Strategy: Add Short, Time {}", strategy.time_local());
                                        }
                                        else if is_long
                                            && long_pnl <= dec!(-500.0)
                                        {
                                            let _exit_order_id = strategy.exit_long(&candle.symbol.name, &account_1, &brokerage, position_size, String::from("Exit Long Take Loss")).await;
                                            println!("Strategy: Exit Long Take Loss, Time {}", strategy.time_local());
                                        }
                                    }

                                    //SHORT CONDITIONS
                                    {
                                        // ENTER SHORT
                                        let is_flat = strategy.is_flat(&brokerage, &account_1, &candle.symbol.name);
                                        // test short ledger
                                        if is_flat
                                            && candle.close < candle.open
                                            && last_side != LastSide::Short
                                        {
                                            let _entry_order_id = strategy.enter_short(&candle.symbol.name, &account_1, &brokerage, dec!(7), String::from("Enter Short")).await;
                                            println!("Strategy: Enter Short, Time {}", strategy.time_local());
                                            last_side = LastSide::Short;
                                        }

                                        // ADD SHORT
                                        let is_short = strategy.is_short(&brokerage, &account_1, &candle.symbol.name);
                                        let short_pnl = strategy.pnl(&brokerage, &account_1, &candle.symbol.name);
                                        let position_size_short: Decimal = strategy.position_size(&brokerage, &account_1, &candle.symbol.name);
                                        if is_short
                                            && short_pnl > dec!(150.0)
                                            && short_pnl < dec!(300.0)
                                            && position_size_short < dec!(21)
                                        {
                                            let _add_order_id = strategy.enter_short(&candle.symbol.name, &account_1, &brokerage, dec!(7), String::from("Add Short")).await;
                                            println!("Strategy: Add Short, Time {}", strategy.time_local());
                                        }

                                        // SHORT SL+TP
                                        let position_size_short: Decimal = strategy.position_size(&brokerage, &account_1, &candle.symbol.name);
                                        if is_short
                                            && short_pnl > dec!(500.0)
                                        {
                                            let _exit_order_id = strategy.exit_short(&candle.symbol.name, &account_1, &brokerage, position_size_short, String::from("Exit Short Take Profit")).await;
                                            println!("Strategy: Exit Short Take Profit, Time {}", strategy.time_local());
                                        }
                                        else if is_short
                                            && short_pnl < dec!(-500.0)
                                        {
                                            let _exit_order_id = strategy.exit_short(&candle.symbol.name, &account_1, &brokerage, position_size_short, String::from("Exit Short Take Loss")).await;
                                            println!("Strategy: Exit Short Take Loss, Time {}", strategy.time_local());
                                        }


                                    }
                                        */

                                }
                            }
                            BaseDataEnum::Quote(quote) => {
                                println!("{}", quote);
                            }
                            BaseDataEnum::QuoteBar(quotebar) => {
                                if quotebar.is_closed == true {
                                    let msg = format!("{} {} QuoteBar Bid Close: {}, Ask Close: {}, {}", quotebar.symbol.name, quotebar.resolution, quotebar.bid_close, quotebar.ask_close, quotebar.time_closed_local(strategy.time_zone()));
                                    if quotebar.bid_close == quotebar.bid_open {
                                        println!("{}", msg.as_str().blue())
                                    } else {
                                        match quotebar.bid_close > quotebar.bid_open {
                                            true => println!("{}", msg.as_str().bright_green()),
                                            false => println!("{}", msg.as_str().bright_red()),
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                StrategyEvent::ShutdownEvent(event) => {
                    strategy.flatten_all_for(brokerage, &account_1).await;
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

                StrategyEvent::PositionEvents(event) => {
                    match event {
                        PositionUpdateEvent::PositionOpened { .. } => {}
                        PositionUpdateEvent::Increased { .. } => {}
                        PositionUpdateEvent::PositionReduced { .. } => strategy.print_ledger(event.brokerage(), event.account_id()),
                        PositionUpdateEvent::PositionClosed { .. } => strategy.print_ledger(event.brokerage(), event.account_id()),
                    }
                    let quantity = strategy.position_size(&brokerage, &account_1, &"EUR-USD".to_string());
                    let msg = format!("{}, Time Local: {}", event, event.time_local(strategy.time_zone()));
                    println!("{}", msg.as_str().purple());
                    println!("Strategy: Open Quantity: {}", quantity);
                }
                StrategyEvent::OrderEvents(event) => {
                    let msg = format!("Strategy: Order Event: {}, Time: {}", event, event.time_local(strategy.time_zone()));
                    match event {
                        OrderUpdateEvent::OrderRejected { .. } | OrderUpdateEvent::OrderUpdateRejected { .. } => println!("{}", msg.as_str().on_bright_magenta().on_bright_red()),
                        _ =>  println!("{}", msg.as_str().bright_yellow())
                    }
                }
                StrategyEvent::TimedEvent(name) => {
                    println!("{} has triggered", name);
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
                                //we could access the exact plot we want using its name, Average True Range only has 1 plot but MACD would have multiple
                                let plot = indicator_values.get_plot(&"atr".to_string());

                                //or we can access all values as a single collection
                                let indicator_values = format!("{}", indicator_values);

                                //if we have a plot named atr we will print it
                                if let Some(plot) = plot {
                                    // the plot color is in rgb, so we can convert to any gui styled coloring and we will print all the values in this color
                                    println!("{}", indicator_values.as_str().truecolor(plot.color.red, plot.color.green, plot.color.blue));
                                }
                            }
                        }
                        IndicatorEvents::Replaced(replace_event) => {
                            let msg = format!("Strategy:Indicator Replaced: {:?}", replace_event);
                            println!("{}", msg.as_str().yellow());
                        }
                    }
                }
                _ => {}
            }
        }
    }
    event_receiver.close();
    println!("Strategy: Event Loop Ended");
}