use std::cmp::PartialEq;
use std::collections::HashMap;
use chrono::{Duration, NaiveDate};
use chrono_tz::Australia;
use colored::Colorize;
use rust_decimal::Decimal;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::{MarketType, PrimarySubscription, StrategyMode};
use ff_standard_lib::strategies::strategy_events::{StrategyEvent};
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, SymbolName};
use ff_standard_lib::strategies::fund_forge_strategy::FundForgeStrategy;
use rust_decimal_macros::dec;
use tokio::sync::mpsc;
use ff_standard_lib::product_maps::rithmic::maps::CME_HOURS;
use ff_standard_lib::standardized_types::accounts::{Account, Currency};
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::orders::OrderUpdateEvent;
use ff_standard_lib::standardized_types::position::PositionUpdateEvent;
use ff_standard_lib::standardized_types::resolution::Resolution;

// to launch on separate machine
#[tokio::main]
async fn main() {
    let (strategy_event_sender, strategy_event_receiver) = mpsc::channel(1000);

    let strategy = FundForgeStrategy::initialize(
        StrategyMode::Backtest, // Backtest, Live, LivePaper
        dec!(100000),
        Currency::AUD,
        NaiveDate::from_ymd_opt(2005, 02, 01).unwrap().and_hms_opt(0, 0, 0).unwrap(), // Starting date of the backtest is a NaiveDateTime not NaiveDate
        NaiveDate::from_ymd_opt(2024, 11, 11).unwrap().and_hms_opt(0, 0, 0).unwrap(), // Ending date of the backtest is a NaiveDateTime not NaiveDate
        Australia::Sydney,                      // the strategy time zone
        Duration::hours(10), // the warmup duration, the duration of historical data we will pump through the strategy to warm up indicators etc before the strategy starts executing.
        vec![
            // Since we only have quote level test data, the 2 subscriptions will be created by consolidating the quote feed. Quote data will automatically be subscribed as primary data source.
            (Some(PrimarySubscription::new(Resolution::Hours(1), BaseDataType::QuoteBars)),
             DataSubscription::new(
                SymbolName::from("EUR-USD"),
                DataVendor::Oanda,
                Resolution::Hours(4),
                BaseDataType::QuoteBars,
                MarketType::Forex
            ), Some(CME_HOURS)),
            /*(Some(PrimarySubscription::new(Resolution::Hours(1), BaseDataType::QuoteBars)),
             DataSubscription::new(
                SymbolName::from("USD-CAD"),
                DataVendor::Oanda,
                Resolution::Hours(4),
                BaseDataType::QuoteBars,
                MarketType::Forex
            ), None),
            (Some(PrimarySubscription::new(Resolution::Hours(1), BaseDataType::QuoteBars)),
             DataSubscription::new(
                SymbolName::from("EUR-CAD"),
                DataVendor::Oanda,
                Resolution::Hours(4),
                BaseDataType::QuoteBars,
                MarketType::Forex
            ), None),
            (Some(PrimarySubscription::new(Resolution::Hours(1), BaseDataType::QuoteBars)),
             DataSubscription::new(
                SymbolName::from("EUR-JPY"),
                DataVendor::Oanda,
                Resolution::Hours(4),
                BaseDataType::QuoteBars,
                MarketType::Forex
            ), None),
            (Some(PrimarySubscription::new(Resolution::Hours(1), BaseDataType::QuoteBars)),
             DataSubscription::new(
                SymbolName::from("AUD-JPY"),
                DataVendor::Oanda,
                Resolution::Hours(4),
                BaseDataType::QuoteBars,
                MarketType::Forex
            ), None),*/
        ],

        //fill forward
        false,

        // history to retain for our initial subscriptions
        100,

        // the sender for the strategy events
        strategy_event_sender,

        // Buffer Duration
        //strategy resolution in milliseconds, all data at a lower resolution will be consolidated to this resolution, if using tick data, you will want to set this at 100 or less depending on the data granularity
        core::time::Duration::from_secs(60), //use a giant buffer since we are only using 1 hour data and not actually buffering anything

        // Enabled will launch the strategy registry handler to connect to a GUI, currently will crash if enabled
        false,

        //tick over no data, strategy will run at buffer resolution speed to simulate weekends and holidays, if false we will just skip over them to the next data point.
        false,
        false,
        vec![Account::new(Brokerage::Oanda, "101-011-24767836-001".to_string())]
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
    mut event_receiver: mpsc::Receiver<StrategyEvent>,
) {

    let mut warmup_complete = false;
    let account_1 = Account::new(Brokerage::Oanda, "101-011-24767836-001".to_string());
    let mut last_side = LastSide::Flat;

    let mut exit_orders = HashMap::new();
    let mut entry_orders = HashMap::new();
    'strategy_loop: while let Some(strategy_event) = event_receiver.recv().await {
        match strategy_event {
            StrategyEvent::TimeSlice(time_slice) => {
                for base_data in time_slice.iter() {
                    match base_data {
                        BaseDataEnum::QuoteBar(qb) => {
                            if qb.is_closed == true && qb.resolution == Resolution::Day {
                                let msg = format!("{} {} {} Close: {}, {}", qb.symbol.name, qb.resolution, qb.candle_type, qb.bid_close, qb.time_closed_local(strategy.time_zone()));
                                if qb.bid_close == qb.bid_open {
                                    println!("{}", msg.as_str().blue())
                                } else {
                                    match qb.bid_close > qb.bid_open {
                                        true => println!("{}", msg.as_str().bright_green()),
                                        false => println!("{}", msg.as_str().bright_red()),
                                    }
                                }
                                if !warmup_complete {
                                    continue;
                                }

                                //LONG CONDITIONS
                                {
                                    // ENTER LONG
                                    if !entry_orders.contains_key(&qb.symbol.name) {
                                        let is_flat = strategy.is_flat(&account_1, &qb.symbol.name);
                                        // buy AUD-CAD if consecutive green HA candles if our other account is long on EUR
                                        if is_flat
                                            && qb.bid_close > qb.bid_open
                                        {
                                            println!("Strategy: {} Enter Long, Time {}", qb.symbol.name, strategy.time_local());
                                            entry_orders.insert(qb.symbol.name.clone(), strategy.enter_long(&qb.symbol.name, None, &account_1, None, dec!(10000), String::from("Enter Long")).await);
                                            last_side = LastSide::Long;
                                        }
                                    }

                                    // ADD LONG
                                    let is_short = strategy.is_short(&account_1, &qb.symbol.name);
                                    let is_long = strategy.is_long(&account_1, &qb.symbol.name);
                                    let long_pnl = strategy.pnl(&account_1, &qb.symbol.name);
                                    println!("Open pnl: {}, Is_short: {}, is_long:{} ", long_pnl, is_short, is_long);

                                    // LONG SL+TP
                                    if !exit_orders.contains_key(&qb.symbol.name) {
                                        if is_long && long_pnl > dec!(1000.0) {
                                            println!("Strategy: {} Exit Long Take Profit, Time {}", qb.symbol.name, strategy.time_local());  // Fixed message
                                            let position_size = strategy.position_size(&account_1, &qb.symbol.name);
                                            exit_orders.insert(qb.symbol.name.clone(), strategy.exit_long(&qb.symbol.name, None, &account_1, None, position_size, String::from("Exit Long Take Profit")).await);
                                        } else if is_long
                                            && long_pnl <= dec!(-100.0)
                                        {
                                            println!("Strategy: {} Exit Long Take Loss, Time {}", qb.symbol.name,  strategy.time_local());
                                            let position_size: Decimal = strategy.position_size(&account_1, &qb.symbol.name);
                                            exit_orders.insert(qb.symbol.name.clone(), strategy.exit_long(&qb.symbol.name, None, &account_1, None, position_size, String::from("Exit Long Take Loss")).await);
                                        }
                                    }
                                }

                                //crash the strategy if short order is detected
                                if strategy.is_short(&account_1, &qb.symbol.name) {
                                    println!("Short position detected");
                                    break 'strategy_loop;
                                }
                            }
                        }

                        BaseDataEnum::Quote(q) => {
                            println!("{}", q);
                        }
                        BaseDataEnum::Tick(t) => {
                            println!("{}", t);
                        }
                        _ => {}
                    }
                }
            }
            StrategyEvent::ShutdownEvent(event) => {
                strategy.flatten_all_for(account_1.clone()).await;
                let msg = format!("{}",event);
                println!("{}", msg.as_str().bright_magenta());
                strategy.export_positions_to_csv(&String::from("./trades exports"));
                strategy.export_trades_to_csv(&account_1, &String::from("./trades exports"));
                strategy.print_ledgers();
                //we should handle shutdown gracefully by first ending the strategy loop.
                break 'strategy_loop
            },

            StrategyEvent::WarmUpComplete => {
                let msg = String::from("Strategy: Warmup Complete");
                println!("{}", msg.as_str().bright_magenta());
                warmup_complete = true;
             /*   let sub = DataSubscription::new(
                    SymbolName::from("NAS100-USD"),
                    DataVendor::Oanda,
                    Resolution::Minutes(5),
                    BaseDataType::QuoteBars,
                    MarketType::CFD
                );
                strategy.subscribe(sub.clone(), 100, false).await;
                let data = strategy.bar_index(&sub, 0);
                println!("Strategy: Bar Index: {:?}", data);*/
            }

            StrategyEvent::PositionEvents(event) => {
                match event {
                    PositionUpdateEvent::PositionOpened { .. } => {}
                    PositionUpdateEvent::Increased { .. } => {}
                    PositionUpdateEvent::PositionReduced { .. } => {
                        strategy.print_ledger(event.account())
                    },
                    PositionUpdateEvent::PositionClosed { .. } => {
                        strategy.print_ledger(event.account())
                    },
                }
                let msg = format!("{}, Time Local: {}", event, event.time_local(strategy.time_zone()));
                println!("{}", msg.as_str().purple());
            }
            StrategyEvent::OrderEvents(event) => {
                let msg = format!("Strategy: Order Event: {}, Time: {}", event, event.time_local(strategy.time_zone()));
                match event {
                     OrderUpdateEvent::OrderUpdateRejected { .. } => {
                        strategy.print_ledger(event.account());
                        println!("{}", msg.as_str().on_bright_magenta().on_bright_red())
                    },
                    OrderUpdateEvent::OrderRejected { ref symbol_name, ref order_id,.. } => {
                        strategy.print_ledger(event.account());
                        println!("{}", msg.as_str().on_bright_magenta().on_bright_red());
                        if let Some(exit_order) = exit_orders.get(symbol_name) {
                            if order_id == exit_order {
                                exit_orders.remove(symbol_name);
                            }
                        }
                        if let Some(entry_order) = entry_orders.get(symbol_name) {
                            if *order_id == *entry_order {
                                entry_orders.remove(symbol_name);
                            }
                        }
                    }
                    OrderUpdateEvent::OrderFilled {symbol_name, order_id, ..} | OrderUpdateEvent::OrderCancelled {symbol_name, order_id, ..} => {
                        println!("{}", msg.as_str().bright_yellow());
                        if let Some(exit_order) = exit_orders.get(&symbol_name) {
                            if order_id == *exit_order {
                                exit_orders.remove(&symbol_name);
                            }
                        }
                        if let Some(entry_order) = entry_orders.get(&symbol_name) {
                            if *order_id == *entry_order {
                                entry_orders.remove(&symbol_name);
                            }
                        }
                    }

                    _ =>  println!("{}", msg.as_str().bright_yellow())
                }
            }
            StrategyEvent::TimedEvent(name) => {
                println!("{} has triggered", name);
            }
            _ => {}
        }
    }
    event_receiver.close();
    println!("Strategy: Event Loop Ended");
}
