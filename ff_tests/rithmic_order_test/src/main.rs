use std::cmp::PartialEq;
use chrono::{Duration, NaiveDate};
use chrono_tz::Australia;
use colored::Colorize;
use ff_rithmic_api::systems::RithmicSystem;
use rust_decimal::Decimal;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::{FuturesExchange, MarketType, StrategyMode};
use ff_standard_lib::strategies::strategy_events::{StrategyEvent};
use ff_standard_lib::standardized_types::subscriptions::{CandleType, DataSubscription, SymbolName};
use ff_standard_lib::strategies::fund_forge_strategy::FundForgeStrategy;
use rust_decimal_macros::dec;
use tokio::sync::mpsc;
use ff_standard_lib::standardized_types::accounts::{Account, AccountId, Currency};
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::orders::OrderUpdateEvent;
use ff_standard_lib::standardized_types::position::PositionUpdateEvent;
use ff_standard_lib::standardized_types::resolution::Resolution;

// to launch on separate machine
#[tokio::main]
async fn main() {
    let (strategy_event_sender, strategy_event_receiver) = mpsc::channel(1000);

    let symbol_name = SymbolName::from("MNQ");
    let mut symbol_code = symbol_name.clone();
    symbol_code.push_str("Z24");

    let strategy = FundForgeStrategy::initialize(
        StrategyMode::Live,
        dec!(100000),
        Currency::USD,
        NaiveDate::from_ymd_opt(2024, 6, 5).unwrap().and_hms_opt(0, 0, 0).unwrap(),
        NaiveDate::from_ymd_opt(2024, 6, 15).unwrap().and_hms_opt(0, 0, 0).unwrap(),
        Australia::Sydney,
        Duration::hours(1),
        vec![
            DataSubscription::new_custom(
                symbol_name,
                DataVendor::Rithmic(RithmicSystem::Apex),
                Resolution::Seconds(1),
                MarketType::Futures(FuturesExchange::CME),
                CandleType::CandleStick
            ),
        ],
        false,
        100,
        strategy_event_sender,
        core::time::Duration::from_millis(100),
        false,
        false,
        false,
        vec![Account::new(Brokerage::Rithmic(RithmicSystem::Apex), "APEX-3396-168".to_string())]
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
    let account_1 = Account::new(Brokerage::Rithmic(RithmicSystem::Apex), AccountId::from("APEX-3396-168"));
    let mut last_side = LastSide::Flat;

    let symbol_name = SymbolName::from("MNQ");
    let mut symbol_code = symbol_name.clone();
    symbol_code.push_str("Z24");

    let mut order_palaced = false;
    // The engine will send a buffer of strategy events at the specified buffer interval, it will send an empty buffer if no events were buffered in the period.
    'strategy_loop: while let Some(strategy_event) = event_receiver.recv().await {
        //println!("Strategy: Buffer Received Time: {}", strategy.time_local());
        //println!("Strategy: Buffer Event Time: {}", strategy.time_zone().from_utc_datetime(&time.naive_utc()));
        match strategy_event {
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
                                        true => println!("{}", msg.as_str().bright_green()),
                                        false => println!("{}", msg.as_str().bright_red()),
                                    }
                                }

                                //LONG CONDITIONS
                                {
                                    // ENTER LONG
                                    let is_flat = strategy.is_flat(&account_1, &symbol_code);
                                    // buy AUD-CAD if consecutive green HA candles if our other account is long on EUR
                                    if !order_palaced
                                    {
                                        println!("Strategy: Enter Long, Time {}", strategy.time_local());
                                        let _entry_order_id = strategy.enter_long(&candle.symbol.name, None ,&account_1, None, dec!(1), String::from("Enter Long")).await;
                                        last_side = LastSide::Long;
                                        order_palaced = true;
                                    }

                                    // ADD LONG
                                    let is_short = strategy.is_short(&account_1, &symbol_code);
                                    let is_long = strategy.is_long(&account_1, &candle.symbol.name);
                                    let long_pnl = strategy.pnl(&account_1, &symbol_code);
                                    println!("Open pnl: {}, Is_short: {}, is_long: {} ", long_pnl, is_short, is_long);

                                    // LONG SL+TP
                                    if is_long && long_pnl > dec!(10.0)
                                    {
                                        let position_size: Decimal = strategy.position_size(&account_1, &symbol_code);
                                        let _exit_order_id = strategy.exit_long(&candle.symbol.name, None, &account_1, None, position_size, String::from("Exit Long Take Profit")).await;
                                        println!("Strategy: Add Short, Time {}", strategy.time_local());
                                    }
                                    else if is_long
                                        && long_pnl <= dec!(-100)
                                    {
                                        let position_size: Decimal = strategy.position_size(&account_1, &symbol_code);
                                        let _exit_order_id = strategy.exit_long(&candle.symbol.name, None, &account_1, None, position_size, String::from("Exit Long Take Loss")).await;
                                        println!("Strategy: Exit Long Take Loss, Time {}", strategy.time_local());
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            StrategyEvent::ShutdownEvent(event) => {
                strategy.flatten_all_for(account_1).await;
                let msg = format!("{}",event);
                println!("{}", msg.as_str().bright_magenta());
                strategy.export_trades(&String::from("./trades exports"));
                strategy.print_ledgers().await;
                //we should handle shutdown gracefully by first ending the strategy loop.
                break 'strategy_loop
            },

            StrategyEvent::WarmUpComplete => {
                let msg = String::from("Strategy: Warmup Complete");
                println!("{}", msg.as_str().bright_magenta());
            }

            StrategyEvent::PositionEvents(event) => {
                match event {
                    PositionUpdateEvent::PositionOpened { .. } => {}
                    PositionUpdateEvent::Increased { .. } => {}
                    PositionUpdateEvent::PositionReduced { .. } => {
                        strategy.print_ledger(event.account()).await
                    },
                    PositionUpdateEvent::PositionClosed { .. } => {
                        strategy.print_ledger(event.account()).await
                    },
                }
                let quantity = strategy.position_size(&account_1, &symbol_code);
                let msg = format!("{}, Time Local: {}", event, event.time_local(strategy.time_zone()));
                println!("{}", msg.as_str().purple());
                println!("Strategy: Open Quantity: {}", quantity);
            }
            StrategyEvent::OrderEvents(event) => {
                let msg = format!("Strategy: Order Event: {}, Time: {}", event, event.time_local(strategy.time_zone()));
                match event {
                    OrderUpdateEvent::OrderRejected { .. } | OrderUpdateEvent::OrderUpdateRejected { .. } => {
                        strategy.print_ledger(event.account()).await;
                        println!("{}", msg.as_str().on_bright_magenta().on_bright_red())
                    },
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