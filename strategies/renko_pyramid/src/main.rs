use std::cmp::PartialEq;
use chrono::{Duration, NaiveDate, Utc};
use chrono_tz::Australia;
use chrono_tz::Tz::UTC;
use colored::Colorize;
use ff_rithmic_api::systems::RithmicSystem;
use rust_decimal::Decimal;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::{FuturesExchange, MarketType, OrderSide, StrategyMode};
use ff_standard_lib::strategies::strategy_events::{StrategyEvent};
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, SymbolCode, SymbolName};
use ff_standard_lib::strategies::fund_forge_strategy::FundForgeStrategy;
use rust_decimal_macros::dec;
use tokio::sync::mpsc;
use ff_standard_lib::gui_types::settings::Color;
use ff_standard_lib::standardized_types::accounts::{Account, Currency};
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::orders::{OrderUpdateEvent, TimeInForce};
use ff_standard_lib::standardized_types::position::PositionUpdateEvent;
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::strategies::indicators::built_in::renko::Renko;
use ff_standard_lib::strategies::indicators::indicator_enum::IndicatorEnum;
use ff_standard_lib::strategies::indicators::indicator_events::IndicatorEvents;
use ff_standard_lib::strategies::indicators::indicator_values::IndicatorValues;

// to launch on separate machine
#[tokio::main]
async fn main() {
    let (strategy_event_sender, strategy_event_receiver) = mpsc::channel(100);
    let account = Account::new(Brokerage::Rithmic(RithmicSystem::Apex), "APEX-3396-168".to_string());
    let symbol_code = SymbolCode::from("MNQZ4");
    let symbol_name = SymbolName::from("MNQ");
    let subscription = DataSubscription::new(
        symbol_name.clone(),
        DataVendor::Rithmic(RithmicSystem::Apex),
        Resolution::Ticks(1),
        BaseDataType::Ticks,
        MarketType::Futures(FuturesExchange::CME),
    );

    let strategy = FundForgeStrategy::initialize(
        StrategyMode::Live,
        dec!(100000),
        Currency::USD,
        NaiveDate::from_ymd_opt(2024, 6, 5).unwrap().and_hms_opt(0, 0, 0).unwrap(),
        NaiveDate::from_ymd_opt(2024, 6, 15).unwrap().and_hms_opt(0, 0, 0).unwrap(),
        Australia::Sydney,
        Duration::hours(1),
        vec![
            subscription.clone()
        ],
        false,
        100,
        strategy_event_sender,
        core::time::Duration::from_millis(5),
        false,
        false,
        false,
        vec![account.clone()],
    ).await;

    on_data_received(strategy, strategy_event_receiver, subscription, symbol_code, symbol_name, account).await;
}

#[derive(Clone, PartialEq, Debug)]
enum LastSide {
    Long,
    Flat,
    Short
}

const RENKO_RANGE: Decimal = dec!(3);
const MAX_SIZE: Decimal = dec!(20);
const SIZE: Decimal = dec!(5);
const INCREMENTAL_SCALP_PNL: Decimal = dec!(100);
pub async fn on_data_received(
    strategy: FundForgeStrategy,
    mut event_receiver: mpsc::Receiver<StrategyEvent>,
    subscription: DataSubscription,
    symbol_code: SymbolCode,
    symbol_name: SymbolName,
    account: Account
) {

    let renko = IndicatorEnum::Renko(Renko::new("renko".to_string(), subscription.clone(), RENKO_RANGE, Color::new(0, 128, 0), Color::new(128, 0, 0), 20).await);
    strategy.subscribe_indicator(renko, false).await;
    let open = "open".to_string();
    let close = "close".to_string();
    let mut last_block: Option<IndicatorValues> = None;
    let mut warmup_complete = false;
    let mut last_side = LastSide::Flat;
    let mut entry_order_id = None;
    let mut exit_order_id = None;
    // The engine will send a buffer of strategy events at the specified buffer interval, it will send an empty buffer if no events were buffered in the period.
    'strategy_loop: while let Some(strategy_event) = event_receiver.recv().await {
        //println!("Strategy: Buffer Received Time: {}", strategy.time_local());
        //println!("Strategy: Buffer Event Time: {}", strategy.time_zone().from_utc_datetime(&time.naive_utc()));
        match strategy_event {
            StrategyEvent::IndicatorEvent(event) => {
                match event {
                    IndicatorEvents::IndicatorTimeSlice(slice) => {
                        let mut no_entry = true;
                        let mut no_exit = true;
                        for renko_value in slice {
                            if let (Some(block_open), Some(block_close)) = (renko_value.get_plot(&open), renko_value.get_plot(&close)) {
                                let msg = format!("Renko: Open: {}, Close: {}", block_open.value, block_close.value);
                                if block_close.value > block_open.value {
                                    println!("{}", msg.as_str().bright_green());
                                } else if block_close.value < block_open.value {
                                    println!("{}", msg.as_str().bright_red());
                                }

                                if let Some(last_block) = last_block {
                                    let last_close = last_block.get_plot(&close).unwrap().value;
                                    let last_open = last_block.get_plot(&open).unwrap().value;

                                    if last_open < last_close && block_close.value > block_open.value && no_entry && entry_order_id == None {
                                        let quantity = strategy.position_size(&account, &symbol_code);

                                        if !strategy.is_long(&account, &symbol_code) && quantity < MAX_SIZE {
                                            entry_order_id = Some(strategy.enter_long(&symbol_name, None, &account, None, SIZE, String::from("Enter Long")).await);
                                            no_entry = false;
                                        }
                                    }
                                    if strategy.is_long(&account, &symbol_code) {
                                        if last_open > last_close && block_close.value < block_open.value && no_exit && exit_order_id == None {
                                            let quantity = strategy.position_size(&account, &symbol_code);
                                            exit_order_id = Some(strategy.exit_long(&symbol_name, None, &account, None, quantity, String::from("Exit Long")).await);
                                            no_exit = false;
                                        }

                                        let profit = strategy.pnl(&account, &symbol_code);
                                        let quantity = strategy.position_size(&account, &symbol_code);
                                        if profit > INCREMENTAL_SCALP_PNL && quantity == MAX_SIZE && exit_order_id == None && no_exit  {
                                            let tif = TimeInForce::Time((Utc::now() + Duration::seconds(30)).timestamp(), UTC.to_string());
                                            exit_order_id = Some(strategy.limit_order(&symbol_name, None, &account, None, SIZE, OrderSide::Sell, last_close + RENKO_RANGE, tif, String::from("TP Long")).await);
                                            no_exit = false;
                                        }
                                    }
                                }
                            }
                            last_block = Some(renko_value);
                        }
                    }
                    _ => {}
                }
            }
            StrategyEvent::TimeSlice(_) => {}
            StrategyEvent::ShutdownEvent(event) => {
                strategy.flatten_all_for(account).await;
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
                warmup_complete = true;
            }

            StrategyEvent::PositionEvents(event) => {
                match event {
                    PositionUpdateEvent::PositionOpened { .. } => {}
                    PositionUpdateEvent::Increased { .. } => {}
                    PositionUpdateEvent::PositionReduced { .. } => {
                        strategy.print_ledger(event.account()).await;
                    },
                    PositionUpdateEvent::PositionClosed { .. } => {
                        strategy.print_ledger(event.account()).await;
                        exit_order_id = None;
                        entry_order_id = None;
                    },
                }
                let quantity = strategy.position_size(&account, &symbol_code);
                let msg = format!("{}, Time Local: {}", event, event.time_local(strategy.time_zone()));
                println!("{}", msg.as_str().purple());
                println!("Strategy: Open Quantity: {}", quantity);
            }
            StrategyEvent::OrderEvents(event) => {
                let msg = format!("Strategy: Order Event: {}, Time: {}", event, event.time_local(strategy.time_zone()));
                match event {
                    OrderUpdateEvent::OrderRejected { .. } => {
                        strategy.print_ledger(event.account()).await;
                        println!("{}", msg.as_str().on_bright_magenta().on_bright_red());
                        if let Some(order_id) = &entry_order_id {
                            if event.order_id() == order_id {
                                entry_order_id = None;
                            }
                        }
                        if let Some(order_id) = &exit_order_id {
                            if event.order_id() == order_id {
                                exit_order_id = None;
                            }
                        }
                    },
                    OrderUpdateEvent::OrderCancelled { .. } | OrderUpdateEvent::OrderFilled {..} => {
                        strategy.print_ledger(event.account()).await;
                        println!("{}", msg.as_str().on_bright_magenta().on_bright_yellow());
                        if let Some(order_id) = &entry_order_id {
                            if event.order_id() == order_id {
                                entry_order_id = None;
                            }
                        }
                        if let Some(order_id) = &exit_order_id {
                            if event.order_id() == order_id {
                                exit_order_id = None;
                            }
                        }
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