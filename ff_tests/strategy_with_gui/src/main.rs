mod close_strength;

use std::cmp::{max, min};
use std::sync::Arc;
use std::thread::sleep;
use chrono::{Duration, NaiveDate, Timelike};
use chrono_tz::{Australia, US};
use chrono_tz::America::Chicago;
use chrono_tz::Tz::Australia__Brisbane;
use colored::Colorize;
use ff_gui::control_panel::panel::{window_settings, StrategyControlPanel};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tokio::sync::mpsc;
use ff_standard_lib::apis::rithmic::rithmic_systems::RithmicSystem;
use ff_standard_lib::gui_types::settings::Color;
use ff_standard_lib::product_maps::rithmic::maps::{get_futures_exchange, get_futures_symbol_info, get_futures_trading_hours};
use ff_standard_lib::standardized_types::accounts::{Account, Currency};
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::enums::{MarketType, OrderSide, PositionSide, PrimarySubscription, StrategyMode};
use ff_standard_lib::standardized_types::orders::{OrderId, OrderUpdateEvent, TimeInForce};
use ff_standard_lib::standardized_types::position::PositionUpdateEvent;
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, SymbolName};
use ff_standard_lib::standardized_types::symbol_info::SymbolInfo;
use ff_standard_lib::strategies::fund_forge_strategy::FundForgeStrategy;
use ff_standard_lib::strategies::indicators::built_in::average_true_range::AverageTrueRange;
use ff_standard_lib::strategies::indicators::built_in::exponential_moving_average::ExponentialMovingAverage;
use ff_standard_lib::strategies::indicators::built_in::renko::Renko;
use ff_standard_lib::strategies::indicators::built_in::rmi::RelativeMomentumIndex;
use ff_standard_lib::strategies::indicators::built_in::rsi::RelativeStrengthIndex;
use ff_standard_lib::strategies::indicators::indicator_events::IndicatorEvents;
use ff_standard_lib::strategies::indicators::indicators_trait::IndicatorName;
use ff_standard_lib::strategies::strategy_events::{StrategyControls, StrategyEvent};
use iced::{Task, Theme};
use crate::close_strength::CloseStrength;
use tokio::task;
// This strategy uses an average measure of price action, combined with renko trend, its primary edge is that it adds to winners and does not add to losers.
// It is designed to be run as a semi-automated strategy, managed by the trader.
// You can increase and decrease or close the position, using the gui tool.

// This strategy will accumulate and doesn't like to take profits, human management is mandatory, if it catches a trend, let it ride until trend is over or set max balance.

const MAX_BALANCE: Decimal = dec!(65000);
const MIN_BALANCE: Decimal = dec!(48000);
const RENKO_RANGE: Decimal = dec!(20);
const SIZE: Decimal = dec!(1);
const MAX_SIZE: Decimal = dec!(10);
const MAX_SIZE_MULTIPLIER: Decimal = dec!(4); //used for dynamic position sizing
const MAX_ENTRIES: i32 = 10;
const MAX_RISK_PER_TRADE: Decimal = dec!(300);
const HAS_NO_TRADE_HOURS: bool = false;
const NO_TRADE_HOURS: u32 = 13; //will not trade before this hour if HAS_NO_TRADE_HOURS == true
const SAFTEY_LEVEL: Decimal = dec!(2640); // the strategy will not trade if price is below this level

#[tokio::main]
async fn main() -> iced::Result {
    let (strategy_event_sender, strategy_event_receiver) = mpsc::channel(100);
    let strategy_event_sender_clone= strategy_event_sender.clone();
    let account = Account::new(Brokerage::Rithmic(RithmicSystem::Apex), "PA-APEX-3396-17".to_string()); //S1Nov228450257 PA-APEX-3396-18
    let account_clone = account.clone();

    task::spawn(async move {

        let symbol_name = SymbolName::from("MNQ");
        let exchange = get_futures_exchange(&symbol_name).unwrap();

        let subscription = DataSubscription::new(
            symbol_name.clone(),
            DataVendor::Rithmic,
            Resolution::Ticks(1),
            BaseDataType::Ticks,
            MarketType::Futures(exchange),
        );

        let candle_subscription = DataSubscription::new(
            symbol_name.clone(),
            DataVendor::Rithmic,
            Resolution::Minutes(5),
            BaseDataType::Candles,
            MarketType::Futures(exchange),
        );

        //let correlation = DataSubscription::new("MES".to_string(), DataVendor::Rithmic, Resolution::Minutes(1), BaseDataType::Candles, MarketType::Futures(exchange));
        let strategy = FundForgeStrategy::initialize(
            StrategyMode::Live,
            dec!(50000),
            Currency::USD,
            NaiveDate::from_ymd_opt(2024, 12, 10).unwrap().and_hms_opt(0, 0, 0).unwrap(),
            NaiveDate::from_ymd_opt(2024, 12, 25).unwrap().and_hms_opt(0, 0, 0).unwrap(),
            Australia__Brisbane,
            Duration::hours(24),
            vec![
                (None, subscription.clone(), None),
                (Some(PrimarySubscription::new(Resolution::Ticks(1), BaseDataType::Ticks)), candle_subscription.clone(), None),
            ],
            false,
            100,
            strategy_event_sender,
            core::time::Duration::from_millis(50),
            false,
            false,
            false,
            vec![account_clone.clone()],
        ).await;

        let renko_indicator = Renko::new("renko".to_string(), subscription.clone(), RENKO_RANGE, Color::new(0, 128, 0), Color::new(128, 0, 0), 20).await;
        strategy.subscribe_indicator(renko_indicator, None).await;

        let bar_strength = CloseStrength::new(IndicatorName::from("Close Strength"), candle_subscription.clone(), 20, Color::new(128, 0, 128), 20).await;
        strategy.subscribe_indicator(bar_strength, None).await;

        let atr = AverageTrueRange::new(IndicatorName::from("ATR"), candle_subscription.clone(), 14, 50, Color::new(0, 0, 128), true).await;
        strategy.subscribe_indicator(atr, None).await;

        on_data_received(Arc::new(strategy), strategy_event_receiver, subscription, candle_subscription, symbol_name, account_clone).await;
    });

    let mut control = StrategyControlPanel {
        strategy_sender: strategy_event_sender_clone,
        current_state: StrategyControls::Continue,
        theme: Theme::default()
    };

    iced::application(
        "Price Action",
        StrategyControlPanel::update,
        StrategyControlPanel::view,
    )
    .theme(StrategyControlPanel::theme)
    .window(window_settings(&account))
    .run_with(move || {
        (control, Task::none())
    })
}


#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
enum Trend {
    Bullish,
    Bearish,
    None
}

#[allow(clippy::const_err)]
pub async fn on_data_received(
    strategy: Arc<FundForgeStrategy>,
    mut event_receiver: mpsc::Receiver<StrategyEvent>,
    subscription: DataSubscription,
    candle_subscription: DataSubscription,
    symbol_name: SymbolName,
    account: Account,
) {
    let symbol_info = get_futures_symbol_info(&symbol_name).unwrap();
    let mut metrics = TradingMetrics::new(SIZE, MAX_SIZE, SIZE, dec!(2), MAX_RISK_PER_TRADE, symbol_info.clone());
    let tp_value = MAX_RISK_PER_TRADE * dec!(2) * SIZE; //* dec!(4);
    let add_value = SIZE * (MAX_RISK_PER_TRADE / dec!(3));
    let absolute_sl_value = MAX_RISK_PER_TRADE * MAX_SIZE;
    let sl_value = (MAX_RISK_PER_TRADE / dec!(2)) * SIZE;
    println!("Starting Price Action Strategy with parameters: Renko Range: {}, Max Size: {}, Size: {}, Min Balance: {}, Max Balance: {}", RENKO_RANGE, MAX_SIZE, SIZE, MIN_BALANCE, MAX_BALANCE);


    let renko = "renko".to_string();
    let strength_indicator = IndicatorName::from("Close Strength");
    let strength = "strength".to_string();
    let average_strength = "weighted_strength".to_string();
    let bear_strength = "bear_strength".to_string();
    let bull_strength = "bull_strength".to_string();
    let open = "open".to_string();
    let close = "close".to_string();

    let ema = "ema".to_string();
    let mut warmup_complete = false;
    let mut entry_order_id: Option<OrderId> = None;
    let mut exit_order_id: Option<OrderId> = None;
    let mut tp_id: Option<OrderId> = None;
    let mut hard_stop: Option<OrderId> = None;
    let mut last_short_result = Result::BreakEven;
    let mut last_long_result = Result::BreakEven;

    let trading_hours = get_futures_trading_hours(&symbol_name).unwrap();
    let exchange = match subscription.market_type {
        MarketType::Futures(exchange) => exchange,
        _ => panic!("Invalid Market Type")
    };

    let symbol_code = strategy.get_front_month(account.brokerage, symbol_name.clone(), exchange).await.unwrap();
    let mut trend = Trend::None;
    let mut entries = 0;
    let mut state = StrategyControls::Continue;

    // The engine will send a buffer of strategy events at the specified buffer interval, it will send an empty buffer if no events were buffered in the period.
    'strategy_loop: while let Some(strategy_event) = event_receiver.recv().await {
        match strategy_event {
            StrategyEvent::IndicatorEvent(event) => {
                match event {
                    IndicatorEvents::IndicatorTimeSlice(slice) => {
                        for renko_value in slice {
                            if let (Some(block_open), Some(block_close)) = (renko_value.get_plot(&open), renko_value.get_plot(&close)) {
                                if let Some(last_block) = strategy.indicator_index(&renko, 1)
                                {
                                    let last_open = last_block.get_plot(&open).unwrap().value;
                                    let last_close = last_block.get_plot(&close).unwrap().value;
                                    if trend != Trend::Bullish && block_close.value > block_open.value && last_close > last_open {
                                        trend = Trend::Bullish;
                                        println!("Trend is Bullish");
                                    }
                                    else if trend != Trend::Bearish && block_close.value < block_open.value && last_close < last_open {
                                        trend = Trend::Bearish;
                                        println!("Trend is Bearish");
                                    }

                                    let is_long = strategy.is_long(&account, &symbol_code);
                                    if is_long && trend == Trend::Bearish {
                                        let quantity = strategy.position_size(&account, &symbol_code);
                                        exit_order_id = Some(strategy.exit_long(&symbol_name, Some(symbol_code.clone()), &account, None, quantity, "Exit Long".to_string()).await);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            StrategyEvent::TimeSlice(slice) => {
                for data in slice.iter() {
                    match data {
                        BaseDataEnum::Tick(tick) => {
                            /*let msg = format!("Ticks: Time: {}, Price: {}", strategy.time_local(), tick.price);
                            println!("{}", msg.as_str().bright_cyan());*/
                        }
                        BaseDataEnum::Candle(candle) => {
                            if !candle.is_closed {
                                continue;
                            }

                            let msg = format!("{} {} Candle: Time: {}, High {}, Low {}, Open: {}, Close: {}", candle.symbol.name, candle.resolution, candle.time_closed_local(&strategy.time_zone()), candle.high, candle.low, candle.open, candle.close);
                            if candle.close > candle.open {
                                println!("{}", msg.as_str().bright_green());
                            } else if candle.close < candle.open {
                                println!("{}", msg.as_str().bright_red());
                            } else {
                                println!("{}", msg.as_str().bright_blue());
                            }

                            if candle.close < SAFTEY_LEVEL {
                                if strategy.is_long(&account, &symbol_code) {
                                    let open_quantity = strategy.position_size(&account, &symbol_code);
                                    exit_order_id = Some(strategy.exit_long(&candle.symbol.name, Some(symbol_code.clone()), &account, None, open_quantity, "Exit Long Target Reached".to_string()).await);
                                }
                                println!("Below saftey level");
                                continue
                            }

                            if state == StrategyControls::Pause {
                                continue
                            }

                            // Stop trading if we hit max loss or max profit
                            let balance = strategy.balance(&account);
                            if balance != dec!(0) {
                                println!("{} Balance: {}", account, balance);
                                if balance >= MAX_BALANCE || balance <= MIN_BALANCE {
                                    println!("Balance is too high or too low, flattening all positions: {}", balance);
                                    if strategy.is_long(&account, &symbol_code) {
                                        let open_quantity = strategy.position_size(&account, &symbol_code);
                                        exit_order_id = Some(strategy.exit_long(&candle.symbol.name, Some(symbol_code.clone()), &account, None, open_quantity, "Exit Long Target Reached".to_string()).await);
                                    }
                                    break 'strategy_loop;
                                }
                            }

                            // Exit before the close of the market
                            if let Some(seconds_until_close) = trading_hours.seconds_until_close(strategy.time_utc()) {
                                if seconds_until_close < 500 {
                                    if strategy.is_long(&account, &symbol_code) && exit_order_id.is_none() {
                                        let open_quantity = strategy.position_size(&account, &symbol_code);
                                        exit_order_id = Some(strategy.exit_long(&candle.symbol.name, Some(symbol_code.clone()), &account, None, open_quantity, "Exit Before Close".to_string()).await);
                                        println!("Flattening all positions for {} due to market close", symbol_code);
                                    } else if strategy.is_short(&account, &symbol_code) && exit_order_id.is_none() {
                                        let open_quantity = strategy.position_size(&account, &symbol_code);
                                        exit_order_id = Some(strategy.exit_short(&candle.symbol.name, Some(symbol_code.clone()), &account, None, open_quantity, "Exit Before Close".to_string()).await);
                                        println!("Flattening all positions for {} due to market close", symbol_code);
                                    }
                                    if let Some(order_id) = &hard_stop {
                                        strategy.cancel_order(order_id.clone()).await;
                                    }
                                    println!("Market is closing soon, waiting for next day: Time: {}", strategy.time_local());
                                    continue;
                                }
                            }

                            if HAS_NO_TRADE_HOURS {
                                if strategy.time_local().hour() <= NO_TRADE_HOURS && !strategy.is_long(&account, &symbol_code) {
                                    continue;
                                }
                            }

                            if !warmup_complete || candle.resolution != candle_subscription.resolution {
                                continue;
                            }

                            // Execution logic
                            if let (Some(bar_strength_value)) = (strategy.indicator_index(&strength_indicator, 0)) {
                                if let (strength, average_strength, average_bull_strength, average_bear_strength) = (bar_strength_value.get_plot(&strength).unwrap().value, bar_strength_value.get_plot(&average_strength).unwrap().value, bar_strength_value.get_plot(&bull_strength).unwrap().value, bar_strength_value.get_plot(&bear_strength).unwrap().value) {

                                    let msg = format!("Bar Strength: Time: {}, Strength: {}, Average Strength {}, Average Bull Strength {}, Average Bear Strength {}", strategy.time_local(), strength, average_strength, average_bull_strength, average_bear_strength);
                                    println!("{}", msg.as_str().bright_cyan());


                                    let atr = match strategy.indicator_index(&IndicatorName::from("ATR"), 0) {
                                        Some(atr) => match atr.get_plot(&"atr".to_string()) {
                                            Some(atr) => atr.value,
                                            None => continue
                                        },
                                        None => continue
                                    };

                                    let msg = format!("ATR: Time: {}, ATR: {}", strategy.time_local(), atr);
                                    println!("{}", msg.as_str().bright_cyan());

                                    let is_long = strategy.is_long(&account, &symbol_code);
                                    let open_profit = strategy.pnl(&account, &symbol_code);
                                    let open_quantity = strategy.position_size(&account, &symbol_code);

                                    let bull_close = candle.close > candle.open;
                                    let bear_close = candle.close < candle.open;

                                    let last_candle = match strategy.candle_index(&candle_subscription, 1) {
                                        Some(candle) => candle,
                                        None => continue
                                    };

                                    let bull_signal = bull_close && candle.close >= last_candle.open && candle.close >= last_candle.high;
                                    // Entry logic
                                    if (!is_long || (is_long && open_profit > add_value && open_quantity < MAX_SIZE))
                                        && entries < MAX_ENTRIES
                                        && entry_order_id.is_none()
                                        && bull_signal
                                        && trend == Trend::Bullish
                                        && strength > average_bull_strength
                                        && average_bull_strength > average_bear_strength
                                        && strength >= dec!(60)
                                    {
                                        entry_order_id = Some(strategy.enter_long(&candle.symbol.name, Some(symbol_code.clone()), &account, None, SIZE, "Enter Long".to_string()).await);
                                        entries += 1;

                                    }

                                    // Exit logic
                                    if is_long && (open_profit < sl_value && average_bear_strength > average_bull_strength && bear_close)
                                    {
                                        exit_order_id = Some(strategy.exit_long(&candle.symbol.name, Some(symbol_code.clone()), &account, None, open_quantity, "Exit Long".to_string()).await);
                                    }

                                    // Partial TP logic
                                    if is_long && open_quantity >= MAX_SIZE
                                        && exit_order_id.is_none()
                                        && open_profit >= tp_value
                                        && average_bear_strength > average_bull_strength
                                    {
                                        exit_order_id = Some(strategy.exit_long(&candle.symbol.name, Some(symbol_code.clone()), &account, None, SIZE, "Partial TP".to_string()).await);
                                    }

                                    let booked_pnl = strategy.booked_pnl_account(&account);
                                    let pnl = strategy.pnl(&account, &symbol_code);
                                    let quantity = strategy.position_size(&account, &symbol_code);
                                    let msg = format!("{} Strategy: Open pnl: {}, Quantity: {}, Total Pnl: {}", symbol_code, pnl, quantity, booked_pnl);
                                    println!("{}", msg.as_str().bright_blue());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            },
            StrategyEvent::ShutdownEvent(event) => {
                strategy.flatten_all_for(account.clone()).await;
                strategy.print_ledgers();
                strategy.print_trade_statistics(&account);
                let msg = format!("{}",event);
                println!("{}", msg.as_str().bright_magenta());
                strategy.export_positions_to_csv(&format!("./trades exports/{}/{}", account.brokerage.to_string(), account.account_id));
                strategy.export_trades_to_csv(&account, &format!("./trades exports/{}/{}", account.brokerage.to_string(), account.account_id));
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
                    PositionUpdateEvent::PositionOpened { average_price, .. } => {
                        if hard_stop.is_none() {
                            let total_size = strategy.position_size(&account, &symbol_code);
                            let stop_price = strategy.calculate_stop_price(average_price, PositionSide::Long, absolute_sl_value, symbol_info.value_per_tick, symbol_info.tick_size, total_size);
                            let price = max(stop_price, SAFTEY_LEVEL);
                            //eprintln!("Stop Price: {}", stop_price);
                            hard_stop = Some(strategy.stop_order(&symbol_name, Some(symbol_code.clone()), &account, None, total_size, OrderSide::Sell, price, TimeInForce::Day, "Hard Stop".to_string()).await);
                        }
                    }
                    PositionUpdateEvent::Increased { average_price, .. } => {
                        if let Some(order_id) = &hard_stop {
                            strategy.cancel_order(order_id.clone()).await;
                        }
                        let total_size = strategy.position_size(&account, &symbol_code);
                        let stop_price = strategy.calculate_stop_price(average_price, PositionSide::Long, absolute_sl_value, symbol_info.value_per_tick, symbol_info.tick_size, total_size);
                        let price = max(stop_price, SAFTEY_LEVEL);
                        //eprintln!("Stop Price: {}", stop_price);
                        hard_stop = Some(strategy.stop_order(&symbol_name, Some(symbol_code.clone()), &account, None, total_size, OrderSide::Sell, price, TimeInForce::Day, "Hard Stop".to_string()).await);
                    }
                    PositionUpdateEvent::PositionReduced { .. } => {
                        strategy.print_ledger(event.account());
                    },
                    PositionUpdateEvent::PositionClosed { ref side, ref booked_pnl, .. } => {
                        entries = 0;
                        let is_win = *booked_pnl > dec!(0);
                        metrics.update_streaks(is_win);
                        strategy.print_trade_statistics(&account);
                        strategy.print_ledger(event.account());
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
                            _ => {}
                        }
                        if let Some(order_id) = &hard_stop {
                            println!("Cancelling Hard Stop");
                            strategy.cancel_order(order_id.clone()).await;
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
                        if let Some(order_id) = &hard_stop {
                            if event.order_id() == order_id {
                                hard_stop = None;
                            }
                        }
                    },
                    OrderUpdateEvent::OrderCancelled { .. }  => {
                        //strategy.print_ledger(event.account()).await;
                        println!("{}", msg.as_str().bright_blue());
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
                        if let Some(order_id) = &hard_stop {
                            if event.order_id() == order_id {
                                hard_stop = None;
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
                                strategy.print_trade_statistics(&account);
                            }
                        }
                        if let Some(order_id) = &hard_stop {
                            if event.order_id() == order_id {
                                hard_stop = None;
                            }
                        }
                    },
                    _ =>  println!("{}", msg.as_str().bright_yellow())
                }
            }
            StrategyEvent::TimedEvent(name) => {
                println!("{} has triggered", name);
            }
            StrategyEvent::StrategyControls(control_message) => {
                match control_message {
                    StrategyControls::Continue => {
                        state = StrategyControls::Continue;
                    }
                    StrategyControls::Pause => {
                        state = StrategyControls::Pause;
                    }
                    StrategyControls::Stop => {
                        break 'strategy_loop
                    }
                    StrategyControls::Start => {
                        state = StrategyControls::Continue;
                    }
                    StrategyControls::Delay(_) => {}
                    StrategyControls::Custom(custom_event) => {
                        if custom_event == "Reduce".to_string() {
                            println!("REDUCING POSITION: USER REQUEST");
                            if strategy.is_long(&account, &symbol_code) && exit_order_id.is_none() {
                                let open_quantity = strategy.position_size(&account, &symbol_code);
                                let reduce_size = min(open_quantity, SIZE);
                                exit_order_id = Some(strategy.exit_long(&symbol_name, Some(symbol_code.clone()), &account, None, reduce_size, "Reduce".to_string()).await);
                            }
                        }
                        else if custom_event == "Increase".to_string() {
                            println!("INCREASING POSITION: USER REQUEST");
                            if entry_order_id.is_none() {
                                entry_order_id = Some(strategy.enter_long(&symbol_name, Some(symbol_code.clone()), &account, None, SIZE, "Increase".to_string()).await);
                                entries += 1;
                            }
                        }
                        else if custom_event == "Flatten".to_string() {
                            println!("FLATTENING: USER REQUEST");
                            if strategy.is_long(&account, &symbol_code) && exit_order_id.is_none() {
                                let open_quantity = strategy.position_size(&account, &symbol_code);
                                exit_order_id = Some(strategy.exit_long(&symbol_name, Some(symbol_code.clone()), &account, None, open_quantity, "Flatten".to_string()).await);
                            }
                        }
                        //sleep(std::time::Duration::from_secs(2))
                    }
                    StrategyControls::CustomBytes(_, _) => {}
                }
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

#[derive(Debug, Clone)]
struct TradingMetrics {
    win_streak: i32,
    loss_streak: i32,
    base_size: Decimal,
    max_size: Decimal,
    min_size: Decimal,
    atr_sizing_factor: Decimal,
    max_risk_per_trade: Decimal,
    symbol_info: SymbolInfo
}

impl TradingMetrics {
    fn new(base_size: Decimal, max_size: Decimal, min_size: Decimal, atr_sizing_factor: Decimal, max_risk_per_trade: Decimal, symbol_info: SymbolInfo) -> Self {
        Self {
            win_streak: 0,
            loss_streak: 0,
            base_size,
            max_size,
            min_size,
            atr_sizing_factor, // Adjust this factor based on testing
            max_risk_per_trade,
            symbol_info
        }
    }

    fn update_streaks(&mut self, is_win: bool) {
        if is_win {
            self.win_streak += 1;
            self.loss_streak = 0;
        } else {
            self.loss_streak += 1;
            self.win_streak = 0;
        }
    }

    fn calculate_position_size(&self, atr: Decimal) -> Decimal {
        // Convert ATR to ticks using symbol's tick size
        let atr_ticks = atr / self.symbol_info.tick_size;
        //println!("ATR ticks: {}", atr_ticks);

        // Calculate dollar value of ATR
        let atr_value = atr_ticks * self.symbol_info.value_per_tick;
        //println!("ATR value: ${}", atr_value);

        // Calculate base position size based on risk per trade and ATR
        let atr_based_size = (self.max_risk_per_trade / (atr_value * self.atr_sizing_factor)).round();
        //println!("ATR based size: {}", atr_based_size);

        // Win/loss streak adjustment
        let streak_adjustment = if self.loss_streak > 0 {
            dec!(1) / (Decimal::from(self.loss_streak) + dec!(1.0))
        } else {
            dec!(1) + (Decimal::from(self.win_streak) * dec!(0.2))
        };
        //println!("Streak adjustment: {}", streak_adjustment);

        // Calculate final size and round to nearest whole number
        let size = (atr_based_size * streak_adjustment).round();
       // println!("Final size before clamp: {}", size);

        // Clamp between min and max sizes (both should be whole numbers)
        let final_size = size.clamp(self.min_size, self.max_size);
       // println!("Final size after clamp: {}", final_size);

        final_size
    }
}
