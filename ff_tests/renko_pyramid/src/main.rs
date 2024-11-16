use chrono::{Duration, NaiveDate};
use chrono_tz::Australia;
use colored::Colorize;
use ff_standard_lib::apis::rithmic::rithmic_systems::RithmicSystem;
use rust_decimal::Decimal;
use ff_standard_lib::standardized_types::enums::{MarketType, OrderSide, PositionSide, StrategyMode};
use ff_standard_lib::strategies::strategy_events::{StrategyEvent};
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, SymbolName};
use ff_standard_lib::strategies::fund_forge_strategy::FundForgeStrategy;
use rust_decimal_macros::dec;
use tokio::sync::mpsc;
use ff_standard_lib::gui_types::settings::Color;
use ff_standard_lib::product_maps::rithmic::maps::{get_futures_exchange, get_futures_trading_hours};
use ff_standard_lib::product_maps::rithmic::rollover::get_front_month;
use ff_standard_lib::standardized_types::accounts::{Account, Currency};
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::orders::{OrderId, OrderUpdateEvent, TimeInForce};
use ff_standard_lib::standardized_types::position::PositionUpdateEvent;
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::strategies::indicators::built_in::renko::Renko;
use ff_standard_lib::strategies::indicators::indicator_events::IndicatorEvents;

#[tokio::main]
async fn main() {
    let (strategy_event_sender, strategy_event_receiver) = mpsc::channel(100);
    let account = Account::new(Brokerage::Rithmic(RithmicSystem::Apex), "APEX-3396-168".to_string());
    let symbol_name = SymbolName::from("MNQ");
    let exchange = get_futures_exchange(&symbol_name).unwrap();

    let subscription = DataSubscription::new(
        symbol_name.clone(),
        DataVendor::Rithmic,
        Resolution::Ticks(1),
        BaseDataType::Ticks,
        MarketType::Futures(exchange),
    );

    let strategy = FundForgeStrategy::initialize(
        StrategyMode::Backtest,
        dec!(100000),
        Currency::USD,
        NaiveDate::from_ymd_opt(2019, 11, 7).unwrap().and_hms_opt(0, 0, 0).unwrap(),
        NaiveDate::from_ymd_opt(2019, 11, 15).unwrap().and_hms_opt(0, 0, 0).unwrap(),
        Australia::Sydney,
        Duration::hours(1),
        vec![
            (None, subscription.clone())
        ],
        false,
        100,
        strategy_event_sender,
        core::time::Duration::from_millis(200),
        false,
        false,
        false,
        vec![account.clone()],
    ).await;

    on_data_received(strategy, strategy_event_receiver, subscription, symbol_name, account).await;
}

// This strategy is designed to pyramid into strong trends using renko. It will not work trading mean reverting markets or trading in both directions.
// It is a tool to help manage positions in fast trending markets. In the current state fund forge strategies should not be run without monitoring. It is possible strategies can lose sync with the actual broker account state.
// 1. enter if 1 bars ago was reversal against the trend and current bar is with the trend (assuming we are trading with the trend). Both modes enter with limit order at prior bar open.
// 2. It exits after 2 bearish renko bars. opposite for short.
// 3. It adds on repeat signals up to 4 times, only if it is in profit.
// 4. It takes profit after a certain amount of profit is made if it is at max size. It will do this with limit orders that expire in X seconds.
// 5. The limit order expiry is on the exchange/rithmic side.
// 6. It will cancel the take profit order if the position is closed.

const RENKO_RANGE: Decimal = dec!(5);
const MAX_SIZE: Decimal = dec!(20);
const SIZE: Decimal = dec!(5);
const INCREMENTAL_SCALP_PNL: Decimal = dec!(150);
const LIMIT_ORDER_EXPIRE_IN_SECS: i64 = 60 * 5;
const TRADING_LONG: bool = true;
const TRADING_SHORT: bool = false;

#[allow(clippy::const_err)]
pub async fn on_data_received(
    strategy: FundForgeStrategy,
    mut event_receiver: mpsc::Receiver<StrategyEvent>,
    subscription: DataSubscription,
    symbol_name: SymbolName,
    account: Account
) {
    println!("Starting Renko Pyramid Strategy with parameters: Renko Range: {}, Max Size: {}, Size: {}, Incremental Scalp PNL: {}, Limit Order Expire in Secs: {}, Trading Long: {}, Trading Short: {}", RENKO_RANGE, MAX_SIZE, SIZE, INCREMENTAL_SCALP_PNL, LIMIT_ORDER_EXPIRE_IN_SECS, TRADING_LONG, TRADING_SHORT);

    let renko = "renko".to_string();
    let renko_indicator = Renko::new(renko.clone(), subscription.clone(), RENKO_RANGE, Color::new(0, 128, 0), Color::new(128, 0, 0), 20).await;
    strategy.subscribe_indicator(renko_indicator).await;
    let open = "open".to_string();
    let close = "close".to_string();
    let mut warmup_complete = false;
    let mut entry_order_id: Option<OrderId> = None;
    let mut exit_order_id: Option<OrderId> = None;
    let mut tp_id: Option<OrderId> = None;
    let mut last_short_result = Result::BreakEven;
    let mut last_long_result = Result::BreakEven;
    let hours = get_futures_trading_hours(&symbol_name).unwrap();

    // The engine will send a buffer of strategy events at the specified buffer interval, it will send an empty buffer if no events were buffered in the period.
    'strategy_loop: while let Some(strategy_event) = event_receiver.recv().await {
        //println!("Strategy: Buffer Received Time: {}", strategy.time_local());
        //println!("Strategy: Buffer Event Time: {}", strategy.time_zone().from_utc_datetime(&time.naive_utc()));
        match strategy_event {
            StrategyEvent::IndicatorEvent(event) => {
                match event {
                    IndicatorEvents::IndicatorTimeSlice(slice) => {
                        let symbol_code = get_front_month(&symbol_name, strategy.time_utc()).unwrap();
                        for renko_value in slice {
                            if let (Some(block_open), Some(block_close)) = (renko_value.get_plot(&open), renko_value.get_plot(&close)) {
                                let msg = format!("Renko: Open: {}, Close: {} @ {}", block_open.value, block_close.value, strategy.time_local());
                                if block_close.value > block_open.value {
                                    println!("{}", msg.as_str().bright_green());
                                } else if block_close.value < block_open.value {
                                    println!("{}", msg.as_str().bright_red());
                                }

                                if !warmup_complete {
                                    continue;
                                }

                                if let Some(seconds_until_close) = hours.seconds_until_close(strategy.time_utc()) {
                                    if seconds_until_close < 500 {
                                        if let Some(entry_order_id) = &entry_order_id {
                                            strategy.cancel_order(entry_order_id.clone()).await;
                                        }
                                        if let Some(exit_order_id) = &exit_order_id {
                                            strategy.cancel_order(exit_order_id.clone()).await;
                                        }
                                        if let Some(tp_id) = &tp_id {
                                            strategy.cancel_order(tp_id.clone()).await;
                                        }
                                        if !strategy.is_flat(&account, &symbol_code) {
                                            strategy.flatten_all_for(account.clone()).await;
                                            println!("Flattening all positions for {} due to market close", symbol_code);
                                        }
                                        println!("Market is closing soon, waiting for next day: Time: {}", strategy.time_local());
                                        continue;
                                    }
                                }

                                if let (Some(last_block), Some(two_blocks_ago)) = (strategy.indicator_index(&renko, 1), strategy.indicator_index(&renko, 2)) {
                                    let last_close = last_block.get_plot(&close).unwrap().value;
                                    let last_open = last_block.get_plot(&open).unwrap().value;
                                    let two_blocks_ago_close = two_blocks_ago.get_plot(&close).unwrap().value;
                                    let two_blocks_ago_open = two_blocks_ago.get_plot(&open).unwrap().value;

                                    #[allow(clippy::const_err)]
                                    if TRADING_LONG {
                                        let is_long = strategy.is_long(&account, &symbol_code);
                                        let pnl = strategy.pnl(&account, &symbol_code);

                                        // Buy on 2 bullish renko blocks
                                        if two_blocks_ago_close < two_blocks_ago_open && block_close.value > block_open.value && last_close > last_open && entry_order_id == None && (!is_long || pnl > INCREMENTAL_SCALP_PNL) {
                                            let quantity = strategy.position_size(&account, &symbol_code);
                                            if quantity < MAX_SIZE {
                                                let tif = TimeInForce::Time((strategy.time_utc() + Duration::seconds(LIMIT_ORDER_EXPIRE_IN_SECS)).timestamp());
                                                entry_order_id = Some(strategy.limit_order(&symbol_name, Some(symbol_code.clone()), &account, None, SIZE, OrderSide::Buy, block_close.value, tif, String::from("Enter Long")).await);
                                            }
                                        }
                                        if is_long {
                                            //tp on 2 bearish renko blocks
                                            if last_close < last_open && block_close.value < block_open.value && exit_order_id == None {
                                                let quantity = strategy.position_size(&account, &symbol_code);
                                                exit_order_id = Some(strategy.exit_long(&symbol_name, Some(symbol_code.clone()), &account, None, quantity, String::from("Exit Long")).await);
                                            }

                                            let profit = strategy.pnl(&account, &symbol_code);
                                            let quantity = strategy.position_size(&account, &symbol_code);
                                            if profit > INCREMENTAL_SCALP_PNL && quantity == MAX_SIZE && exit_order_id == None && tp_id == None {
                                                let tif = TimeInForce::Time((strategy.time_utc() + Duration::seconds(LIMIT_ORDER_EXPIRE_IN_SECS)).timestamp());
                                                tp_id = Some(strategy.limit_order(&symbol_name, Some(symbol_code.clone()), &account, None, SIZE, OrderSide::Sell, last_close + RENKO_RANGE * dec!(4), tif, String::from("Partial TP Long")).await);
                                            }
                                        }
                                    }
                                    #[allow(clippy::const_err)]
                                    if TRADING_SHORT {
                                        let is_short = strategy.is_short(&account, &symbol_code);
                                        let pnl = strategy.pnl(&account, &symbol_code);

                                        // Buy on 2 bearish renko blocks
                                        if two_blocks_ago_close > two_blocks_ago_open && block_close.value < block_open.value && last_close < last_open && entry_order_id == None && (!is_short || pnl > INCREMENTAL_SCALP_PNL) {
                                            let quantity = strategy.position_size(&account, &symbol_code);
                                            if quantity < MAX_SIZE {
                                                let tif = TimeInForce::Time((strategy.time_utc() + Duration::seconds(LIMIT_ORDER_EXPIRE_IN_SECS)).timestamp());
                                                entry_order_id = Some(strategy.limit_order(&symbol_name, Some(symbol_code.clone()), &account, None, SIZE, OrderSide::Sell, block_close.value, tif, String::from("Enter Short")).await);
                                            }
                                        }
                                        if is_short {
                                            //tp on 2 bullish renko blocks
                                            if last_close > last_open && block_close.value > block_open.value && exit_order_id == None {
                                                let quantity = strategy.position_size(&account, &symbol_code);
                                                exit_order_id = Some(strategy.exit_short(&symbol_name, Some(symbol_code.clone()), &account, None, quantity, String::from("Exit Short")).await);
                                            }

                                            let profit = strategy.pnl(&account, &symbol_code);
                                            let quantity = strategy.position_size(&account, &symbol_code);
                                            if profit > INCREMENTAL_SCALP_PNL && quantity == MAX_SIZE && exit_order_id == None && tp_id == None {
                                                let tif = TimeInForce::Time((strategy.time_utc() + Duration::seconds(LIMIT_ORDER_EXPIRE_IN_SECS)).timestamp());
                                                tp_id = Some(strategy.limit_order(&symbol_name, Some(symbol_code.clone()), &account, None, SIZE, OrderSide::Buy, last_close - RENKO_RANGE * dec!(4), tif, String::from("Partial TP Short")).await);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        let pnl = strategy.pnl(&account, &symbol_code);
                        let quantity = strategy.position_size(&account, &symbol_code);
                        let msg = format!("{} Strategy: Open pnl: {}, Quantity: {}", symbol_code, pnl, quantity);
                        println!("{}", msg.as_str().bright_blue());
                    }
                    _ => {}
                }
            }
            StrategyEvent::TimeSlice(_slice) => {
                /*for data in slice.iter() {
                    match data {
                        BaseDataEnum::Tick(tick) => {
                            let msg = format!("Ticks: Time: {}, Price: {}", strategy.time_local(), tick.price);
                            println!("{}", msg.as_str().bright_cyan());
                        }
                        _ => {}
                    }
                }*/
            }
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
                    PositionUpdateEvent::PositionClosed { ref side, ref booked_pnl,.. } => {
                        strategy.print_ledger(event.account()).await;
                        exit_order_id = None;
                        entry_order_id = None;
                        let result = if *booked_pnl > dec!(0) {
                            Result::Win
                        } else if *booked_pnl < dec!(0) {
                            Result::Loss
                        } else {
                            Result::BreakEven
                        };
                        match side {
                            PositionSide::Long => {
                               last_long_result = result;
                            }
                            PositionSide::Short => {
                                last_short_result = result;
                            }
                        }
                    },
                }
                let msg = format!("{}, Time Local: {}", event, event.time_local(strategy.time_zone()));
                println!("{}", msg.as_str().purple());
            }
            StrategyEvent::OrderEvents(event) => {
                let msg = format!("Strategy: Order Event: {}, Time: {}", event, event.time_local(strategy.time_zone()));
                match event {
                    OrderUpdateEvent::OrderRejected { .. } => {
                        //strategy.print_ledger(event.account()).await;
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
                        if let Some(order_id) = &tp_id {
                            if event.order_id() == order_id {
                                tp_id = None;
                            }
                        }
                    },
                    OrderUpdateEvent::OrderCancelled { .. }  => {
                        //strategy.print_ledger(event.account()).await;
                        println!("{}", msg.as_str().bright_yellow());
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
                        if let Some(order_id) = &tp_id {
                            if event.order_id() == order_id {
                                tp_id = None;
                            }
                        }
                    },
                    OrderUpdateEvent::OrderFilled {..} => {
                        println!("{}", msg.as_str().bright_yellow());
                        if let Some(order_id) = &entry_order_id {
                            if event.order_id() == order_id {
                                entry_order_id = None;
                            }
                        }
                        if let Some(order_id) = &exit_order_id {
                            if event.order_id() == order_id {
                                exit_order_id = None;
                                if let Some(tp_order_id) = &tp_id {
                                    strategy.cancel_order(tp_order_id.clone()).await;
                                }
                            }
                        }
                        if let Some(order_id) = &tp_id {
                            if event.order_id() == order_id {
                                tp_id = None;
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

enum Result {
    Win,
    Loss,
    BreakEven
}