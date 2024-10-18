use std::cmp::PartialEq;
use chrono::{Duration, NaiveDate, Utc};
use chrono_tz::Australia;
use chrono_tz::Tz::UTC;
use colored::Colorize;
use ff_rithmic_api::systems::RithmicSystem;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::{FuturesExchange, MarketType, OrderSide, StrategyMode};
use ff_standard_lib::strategies::strategy_events::{StrategyEvent, StrategyEventBuffer};
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, SymbolName};
use ff_standard_lib::strategies::fund_forge_strategy::FundForgeStrategy;
use rust_decimal_macros::dec;
use tokio::sync::mpsc;
#[allow(unused_imports)]
use ff_standard_lib::gui_types::settings::Color;
use ff_standard_lib::standardized_types::accounts::{AccountId, Currency};
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::orders::{OrderUpdateEvent, TimeInForce};
use ff_standard_lib::standardized_types::position::PositionUpdateEvent;
use ff_standard_lib::standardized_types::resolution::Resolution;
#[allow(unused_imports)]
use ff_standard_lib::strategies::indicators::built_in::average_true_range::AverageTrueRange;
#[allow(unused_imports)]
use ff_standard_lib::strategies::indicators::indicator_enum::IndicatorEnum;
use ff_standard_lib::strategies::indicators::indicator_events::IndicatorEvents;
#[allow(unused_imports)]
use ff_standard_lib::strategies::indicators::indicators_trait::IndicatorName;


// TODO WARNING THIS IS LIVE TRADING
// to launch on separate machine
#[tokio::main]
async fn main() {
    let (strategy_event_sender, strategy_event_receiver) = mpsc::channel(1000);
    let strategy = FundForgeStrategy::initialize(
        StrategyMode::Live,
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
            /*DataSubscription::new(
                SymbolName::from("MNQ"),
                DataVendor::Rithmic(RithmicSystem::TopstepTrader),
                Resolution::Instant,
                BaseDataType::Quotes,
                MarketType::Futures(FuturesExchange::CME)
            ),*/
           DataSubscription::new(
               SymbolName::from("MNQ"),
               DataVendor::Rithmic(RithmicSystem::TopstepTrader),
               Resolution::Seconds(5),
               BaseDataType::Candles,
               MarketType::Futures(FuturesExchange::CME)
           )
        ],
        false,
        100,
        strategy_event_sender,
        core::time::Duration::from_millis(500),
        false,
        true,
        false
    ).await;

    on_data_received(strategy, strategy_event_receiver).await;
}

#[allow(dead_code, unused)]
#[derive(Clone, PartialEq, Debug)]
enum LastSide {
    Long,
    Flat,
    Short
}

#[allow(dead_code, unused)]
pub async fn on_data_received(
    strategy: FundForgeStrategy,
    mut event_receiver: mpsc::Receiver<StrategyEventBuffer>,
) {
    let subscription = DataSubscription::new(
        SymbolName::from("MNQ"),
        DataVendor::Rithmic(RithmicSystem::TopstepTrader),
        Resolution::Seconds(5),
        BaseDataType::Candles,
        MarketType::Futures(FuturesExchange::CME)
    );
    /*
    let atr_5 = IndicatorEnum::AverageTrueRange(
        AverageTrueRange::new(
            IndicatorName::from("atr_5"),
            // The subscription for the indicator
            subscription.clone(),

            // history to retain
            10,

            // atr period
            5,

            // Plot color for GUI or println!()
            Color::new (128, 0, 128)
        ).await,
    );*/

    //if you set auto subscribe to false and change the resolution, the strategy will intentionally panic to let you know you won't have data for the indicator
    //strategy.subscribe_indicator(atr_5, true).await;

    let mut warmup_complete = false;
    let mut last_side = LastSide::Flat;
    let account: AccountId = "S1Sep246906077".to_string();
    let brokerage = Brokerage::Rithmic(RithmicSystem::TopstepTrader);
    let mut symbol_code = "".to_string();
    println!("Staring Strategy Loop");
    let symbol = "MNQ".to_string();
    let mut count = 0;
    let mut bars_since_entry = 0;
    let mut entry_order_id = None;
    // The engine will send a buffer of strategy events at the specified buffer interval, it will send an empty buffer if no events were buffered in the period.
    'strategy_loop: while let Some(event_slice) = event_receiver.recv().await {
        //println!("Strategy: Buffer Received Time: {}", strategy.time_local());
        for (_time, strategy_event) in event_slice.iter() {
            //println!("Strategy: Buffer Event Time: {}", strategy.time_zone().from_utc_datetime(&time.naive_utc()));
            match strategy_event {
                StrategyEvent::TimeSlice(time_slice) => {
                    // here we would process the time slice events and update the strategy state accordingly.
                    for (_, base_data) in time_slice.iter_ordered() {
                        match base_data {
                            BaseDataEnum::Tick(tick) => {
                               let msg =  format!("{} Tick: {} @ {}", tick.symbol.name, tick.price, tick.time_local(strategy.time_zone()));
                                println!("{}", msg.as_str().purple());
                            }
                            BaseDataEnum::Candle(candle) => {
                                // Place trades based on the AUD-CAD Heikin Ashi Candles
                                if candle.is_closed == true  {
                                    let msg = format!("{} {} {} Close: {}, {}", candle.symbol.name, candle.resolution, candle.candle_type, candle.close, candle.time_closed_local(strategy.time_zone()));
                                    if candle.close == candle.open {
                                        println!("{}", msg.as_str().blue())
                                    } else {
                                        match candle.close > candle.open {
                                            true => println!("{}", msg.as_str().bright_green()),
                                            false => println!("{}", msg.as_str().bright_red()),
                                        }
                                    }

                                    count += 1;

                                    let last_candle = strategy.candle_index(&subscription, 1);

                                    if last_candle.is_none() || candle.resolution != Resolution::Seconds(5) {
                                        println!("Last Candle Is None");
                                        continue;
                                    }

                                    let last_candle = last_candle.unwrap();
                                    // entry orders
                                    if candle.close > last_candle.high && entry_order_id == None && bars_since_entry == 0 {
                                        println!("Submitting long limit");
                                        let cancel_order_time = Utc::now() + Duration::seconds(30);
                                        let order_id = strategy.limit_order(&symbol, None, &account, &Brokerage::Rithmic(RithmicSystem::TopstepTrader), None,dec!(1), OrderSide::Buy, last_candle.low, TimeInForce::Time(cancel_order_time.naive_utc().to_string(), UTC.to_string()), String::from("Enter Long Limit")).await;
                                        entry_order_id = Some(order_id);
                                    }
                                    else if candle.close < last_candle.low && entry_order_id == None && bars_since_entry == 0  {
                                        println!("Submitting short limit");
                                        let cancel_order_time = Utc::now() + Duration::seconds(30);
                                        let order_id = strategy.limit_order(&symbol, None, &account, &Brokerage::Rithmic(RithmicSystem::TopstepTrader), None,dec!(1), OrderSide::Sell, last_candle.high, TimeInForce::Time(cancel_order_time.naive_utc().to_string(), UTC.to_string()), String::from("Enter Short Limit")).await;
                                        entry_order_id = Some(order_id);
                                    }


                                    // exit orders
                                    let is_long = strategy.is_long(&brokerage, &account, &symbol_code);
                                    let is_short = strategy.is_short(&brokerage, &account, &symbol_code);
                                    if is_long || is_short {
                                        bars_since_entry +=1;
                                    }

                                    if is_long && bars_since_entry > 5 {
                                        let exit_id = strategy.exit_long(&symbol, None, &account, &Brokerage::Rithmic(RithmicSystem::TopstepTrader), None,dec!(1), String::from("Exit Long")).await;
                                        bars_since_entry = 0;
                                        entry_order_id = None;
                                    } else if is_short && bars_since_entry > 5 {
                                        let exit_id = strategy.exit_short(&symbol,None, &account, &Brokerage::Rithmic(RithmicSystem::TopstepTrader), None,dec!(1), String::from("Exit Short")).await;
                                        bars_since_entry= 0;
                                        entry_order_id = None;
                                    }
                                }
                            }
                            BaseDataEnum::Quote(quote) => {
                                let msg = format!("{} Quote: {} @ {}", quote.symbol.name, quote.bid, quote.time_local(strategy.time_zone()));
                                println!("{}", msg.as_str().yellow());
                            }
                            BaseDataEnum::QuoteBar(quotebar) => {
                                if quotebar.is_closed == true {
                                    let msg = format!("{} {} QuoteBar Bid Close: {}, {}", quotebar.symbol.name, quotebar.resolution, quotebar.bid_close, quotebar.time_closed_local(strategy.time_zone()));
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
                    strategy.flatten_all_for(brokerage, &account).await;
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
                        PositionUpdateEvent::PositionOpened { .. } => {},
                        PositionUpdateEvent::Increased { .. } => {},
                        PositionUpdateEvent::PositionReduced { .. } => strategy.print_ledger(event.brokerage(), event.account_id()),
                        PositionUpdateEvent::PositionClosed { .. } => strategy.print_ledger(event.brokerage(), event.account_id()),
                    }

                    let msg = format!("{}, Time Local: {}", event, event.time_local(strategy.time_zone()));
                    println!("{}", msg.as_str().purple());

                    let quantity = strategy.position_size(&brokerage, &account, &symbol_code);
                    println!("Strategy: Open Quantity: {}", quantity);
                }
                StrategyEvent::OrderEvents(event) => {
                    match event.symbol_code() {
                        None => {}
                        Some(code) => {
                            if code.starts_with("MNQ") {
                                symbol_code = code;
                            }
                        }
                    }
                    strategy.print_ledger(event.brokerage(), event.order_id());
                    let msg = format!("Strategy: Order Event: {}, Time: {}", event, event.time_local(strategy.time_zone()));
                    match event {
                        OrderUpdateEvent::OrderRejected { .. } | OrderUpdateEvent::OrderUpdateRejected { .. } => {
                            println!("{}", msg.as_str().on_bright_magenta().on_bright_red());
                            entry_order_id = None;
                        },
                        OrderUpdateEvent::OrderCancelled {order_id, ..} => {
                            println!("{}", msg.as_str().bright_cyan());
                            let mut cancelled = false;
                            if let Some(entry_order_id) = &entry_order_id {
                                if *order_id == *entry_order_id {
                                    cancelled = true;
                                }
                            }
                            if cancelled {
                                entry_order_id = None;
                            }
                        }
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