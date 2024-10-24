use std::cmp::PartialEq;
use chrono::{Duration, NaiveDate, Timelike, Utc};
use chrono_tz::{Australia, UTC};
use colored::Colorize;
use ff_rithmic_api::systems::RithmicSystem;
use rust_decimal::Decimal;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::{FuturesExchange, MarketType, OrderSide, StrategyMode};
use ff_standard_lib::strategies::strategy_events::{StrategyEvent};
use ff_standard_lib::standardized_types::subscriptions::{CandleType, DataSubscription, SymbolName};
use ff_standard_lib::strategies::fund_forge_strategy::FundForgeStrategy;
use rust_decimal_macros::dec;
use tokio::sync::mpsc;
#[allow(unused_imports)]
use ff_standard_lib::gui_types::settings::Color;
use ff_standard_lib::standardized_types::accounts::{Account, Currency};
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
use ff_standard_lib::strategies::indicators::indicators_trait::Indicators;

// TODO WARNING THIS IS LIVE TRADING
const IS_LONG_STRATEGY: bool = true;
const IS_SHORT_STRATEGY: bool = true;
const MAX_PROFIT: Decimal = dec!(9000);
const MAX_LOSS: Decimal = dec!(1500);
const MIN_ATR_VALUE: Decimal = dec!(2.5);
const PROFIT_TARGET: Decimal = dec!(600);
const RISK: Decimal = dec!(200);
const DATAVENDOR: DataVendor = DataVendor::Rithmic(RithmicSystem::Apex);

#[tokio::main]
async fn main() {
    //todo You will need to put in your paper account ID here or the strategy will crash on initialization, you can trade multiple accounts and brokers and mix and match data feeds.
    let account = Account::new(Brokerage::Rithmic(RithmicSystem::Apex), "YOUR_ACCOUNT_HERE".to_string()); //todo change your brokerage to the correct broker, prop firm or rithmic system.
    let symbol_name = SymbolName::from("MGC");
    let mut symbol_code = symbol_name.clone();
    symbol_code.push_str("Z24");

    let subscription = DataSubscription::new_custom(
        symbol_name.clone(),
        DATAVENDOR,
        Resolution::Minutes(3),
        MarketType::Futures(FuturesExchange::COMEX), //todo, dont forget to change the exchange for the symbol you are trading
        CandleType::HeikinAshi
        );

    let (strategy_event_sender, strategy_event_receiver) = mpsc::channel(100);
    let strategy = FundForgeStrategy::initialize(
        StrategyMode::Live,
        dec!(100000),
        Currency::USD,
        NaiveDate::from_ymd_opt(2024, 6, 5).unwrap().and_hms_opt(0, 0, 0).unwrap(),
        NaiveDate::from_ymd_opt(2024, 08, 28).unwrap().and_hms_opt(0, 0, 0).unwrap(),
        Australia::Sydney,
        Duration::hours(5),
        vec![
            DataSubscription::new(
                subscription.symbol.name.clone(),
                subscription.symbol.data_vendor.clone(),
                Resolution::Ticks(1),
                BaseDataType::Ticks,
                MarketType::Futures(FuturesExchange::COMEX) //todo, dont forget to change the exchange for the symbol you are trading
            ),

            //subscribe to our subscription
            subscription.clone() //todo, add any more data feeds you want into here.
        ],
        true,
        100,
        strategy_event_sender,
        core::time::Duration::from_millis(10),
        false,
        true,
        true,
        vec![account.clone()] //todo, add any more accounts you want into here.
    ).await;

    on_data_received(strategy, strategy_event_receiver, account, subscription, symbol_code).await;
}

#[allow(dead_code, unused)]
#[derive(Clone, PartialEq, Debug)]
enum LastSide {
    Long,
    Flat,
    Short
}
#[derive(Clone, PartialEq, Debug)]
enum TradeResult {
    Win,
    Loss,
    BreakEven,
}

#[allow(dead_code, unused)]
pub async fn on_data_received(
    strategy: FundForgeStrategy,
    mut event_receiver: mpsc::Receiver<StrategyEvent>,
    account: Account,
    subscription: DataSubscription,
    mut symbol_code: String
) {
    println!("Staring Strategy Loop");

    let atr_10 = IndicatorEnum::AverageTrueRange(
        AverageTrueRange::new(
            IndicatorName::from("atr_10"),
            // The subscription for the indicator
            subscription.clone(),

            // history to retain
            10,

            // atr period
            10,

            // Plot color for GUI or println!()
            Color::new (128, 0, 128),

            false
        ).await,
    );

    //if you set auto subscribe to false and change the resolution, the strategy will intentionally panic to let you know you won't have data for the indicator
    strategy.subscribe_indicator(atr_10.clone(), false).await;
    let mut warmup_complete = false;
    let mut last_side = LastSide::Flat;
    let mut count = 0;
    let mut bars_since_entry = 0;
    let mut entry_order_id = None;
    let mut add_order_id = None;
    let mut exit_order_id = None;
    let mut position_size = 0;
    let atr_plot = "atr".to_string();
    let mut last_result = TradeResult::BreakEven;
    let mut attempting_entry = "None".to_string();
    // The engine will send a buffer of strategy events at the specified buffer interval, it will send an empty buffer if no events were buffered in the period.
    'strategy_loop: while let Some(strategy_event) = event_receiver.recv().await {
        //println!("Strategy: Buffer Received Time: {}", strategy.time_local());
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

                                let last_candle = strategy.bar_index(&subscription, 1);
                                let last_atr = strategy.indicator_index(&atr_10.name(), 1);
                                let current_atr = strategy.indicator_index(&atr_10.name(), 0);

                                if last_candle.is_none() || candle.resolution != subscription.resolution || last_atr.is_none() || current_atr.is_none() {
                                    println!("Last Candle or Indicator Values Is None");
                                    continue;
                                }

                                let is_flat = strategy.is_flat(&account, &symbol_code);
                                let last_candle = last_candle.unwrap();
                                let last_atr = last_atr.unwrap().get_plot(&atr_plot).unwrap().value;
                                let current_atr = current_atr.unwrap().get_plot(&atr_plot).unwrap().value;
                                let min_atr = current_atr >= MIN_ATR_VALUE;
                                let atr_increasing = current_atr > last_atr;
                                let booked_pnl = strategy.booked_pnl(&account, &symbol_code);
                                let bar_time = candle.time_utc();

                                // Check if time is between Chicago close (19:50) utc and NZ open (21:00) Utc or if we hit daily profit target or loss limit.
                                if (bar_time.hour() == 19 && bar_time.minute() >= 50) || bar_time.hour() == 20 ||  booked_pnl >= MAX_PROFIT ||  booked_pnl <= -MAX_LOSS {
                                    // Close any existing positions
                                    if strategy.is_long(&account, &symbol_code) {
                                        let position_size = strategy.position_size(&account, &symbol_code);
                                        let exit_id = strategy.exit_long(&candle.symbol.name, None, &account, None, position_size, String::from("Exit Long")).await;
                                        exit_order_id = Some(exit_id);
                                        bars_since_entry = 0;
                                        last_side = LastSide::Long;
                                    }
                                    else if strategy.is_short(&account, &symbol_code) {
                                        let position_size = strategy.position_size(&account, &symbol_code);
                                        let exit_id = strategy.exit_short(&candle.symbol.name, None, &account, None, position_size, String::from("Exit Short")).await;
                                        exit_order_id = Some(exit_id);
                                        bars_since_entry = 0;
                                        last_side = LastSide::Short;
                                    }
                                    continue;
                                }

                                let high_close = match candle.close.cmp(&candle.open) {
                                    std::cmp::Ordering::Greater => { // Bullish candle
                                        // Close should be near high for bullish
                                        match candle.high.cmp(&candle.close) {
                                            std::cmp::Ordering::Greater => (candle.high - candle.close) / candle.high <= dec!(0.05),
                                            _ => false
                                        }
                                    },
                                    _ => false // Must be bullish to check high close
                                };

                                let low_close = match candle.close.cmp(&candle.open) {
                                    std::cmp::Ordering::Less => { // Bearish candle
                                        // Close should be near low for bearish
                                        match candle.close.cmp(&candle.low) {
                                            std::cmp::Ordering::Greater => (candle.close - candle.low) / candle.low <= dec!(0.05),
                                            _ => false
                                        }
                                    },
                                    _ => false // Must be bearish to check low close
                                };

                                // See if we have a clean bullish entry bar
                                let high_1 = candle.low >= last_candle.bid_open && // Changed to >= for true support
                                    candle.close > last_candle.ask_high &&
                                    high_close &&
                                    candle.close > candle.open; // Add check for bullish close

                                // See if we have a clean bearish entry bar
                                let low_1 = candle.high <= last_candle.ask_open && // Changed to <= for true resistance
                                    candle.close < last_candle.bid_low &&
                                    low_close &&
                                    candle.close < candle.open; // Add check for bearish close

                                // entry orders
                                if IS_LONG_STRATEGY && (last_side != LastSide::Long || (last_side == LastSide::Long && last_result == TradeResult::Win)) && is_flat && high_1 && entry_order_id.is_none() && atr_increasing && min_atr {
                                    //println!("Submitting long entry");
                                    let cancel_order_time = Utc::now() + Duration::seconds(30);
                                    let order_id = strategy.limit_order(&candle.symbol.name, None, &account, None,dec!(2), OrderSide::Buy, last_candle.bid_high, TimeInForce::Time(cancel_order_time.timestamp(), UTC.to_string()), String::from("Enter Long Limit")).await;
                                    entry_order_id = Some(order_id);
                                    exit_order_id = None;
                                    attempting_entry = "Long".to_string();

                                }
                                else if IS_SHORT_STRATEGY && (last_side != LastSide::Short || (last_side == LastSide::Short && last_result == TradeResult::Win)) && is_flat && low_1 && entry_order_id.is_none() && atr_increasing && min_atr {
                                    //println!("Submitting short limit");
                                    let cancel_order_time = Utc::now() + Duration::seconds(30);
                                    let order_id = strategy.limit_order(&candle.symbol.name, None, &account, None,dec!(2), OrderSide::Sell, last_candle.bid_high, TimeInForce::Time(cancel_order_time.timestamp(), UTC.to_string()), String::from("Enter Short Limit")).await;
                                    entry_order_id = Some(order_id);
                                    exit_order_id = None;
                                    attempting_entry = "Short".to_string();
                                }

                                // exit orders
                                let is_long = strategy.is_long(&account, &symbol_code);
                                let is_short = strategy.is_short(&account, &symbol_code);
                                if is_long || is_short {
                                    bars_since_entry += 1;
                                }

                                let open_profit = strategy.pnl(&account,  &symbol_code);
                                let position_size = strategy.position_size(&account, &symbol_code);
                                println!(
                                    "{}, Open Profit: {}, Position Size: {}, Last Entry Attempt: {}",
                                    symbol_code, open_profit, position_size, attempting_entry
                                );

                                //Add to winners up to 2x if we have momentum
                                if (is_long || is_short) && bars_since_entry > 2 && open_profit >= dec!(40) && position_size <= dec!(5) && add_order_id.is_none() && exit_order_id.is_none() && entry_order_id.is_none() {

                                    let cancel_order_time = Utc::now() + Duration::seconds(15);

                                    if IS_LONG_STRATEGY && is_long && high_1 {
                                        let new_add_order_id = strategy.stop_limit(&candle.symbol.name, None, &account, None,dec!(3), OrderSide::Buy,  String::from("Add Long Stop Limit"), last_candle.ask_high + dec!(0.5), last_candle.ask_high + dec!(0.25), TimeInForce::Time(cancel_order_time.timestamp(), UTC.to_string())).await;
                                        bars_since_entry = 0;
                                        exit_order_id = None;
                                        add_order_id = Some(new_add_order_id);
                                    }
                                    else if IS_SHORT_STRATEGY && is_short && low_1 {
                                        let new_add_order_id =strategy.stop_limit(&candle.symbol.name, None, &account, None,dec!(3), OrderSide::Sell,  String::from("Add Short Stop Limit"), last_candle.bid_low - dec!(0.5), last_candle.bid_low - dec!(0.25), TimeInForce::Time(cancel_order_time.timestamp(), UTC.to_string())).await;
                                        bars_since_entry = 0;
                                        exit_order_id = None;
                                        add_order_id = Some(new_add_order_id);
                                    }
                                }

                                let profit_goal = match position_size > dec!(5) {
                                    true => PROFIT_TARGET,
                                    false => PROFIT_TARGET * dec!(2)
                                };

                                // Cut losses and take profits, we check entry order is none to prevent exiting while an entry order is unfilled, entry order will expire and go to none on the TIF expiry, or on fill.
                                if open_profit > profit_goal || (open_profit < RISK && bars_since_entry > 10) && add_order_id.is_none() && entry_order_id.is_none() && exit_order_id.is_none() {

                                    let is_long = strategy.is_long(&account, &symbol_code);
                                    let is_short = strategy.is_short(&account, &symbol_code);

                                    if is_long {
                                        let position_size = strategy.position_size(&account, &symbol_code);
                                        let exit_id = strategy.exit_long(&candle.symbol.name, None, &account, None, position_size, String::from("Exit Long")).await;
                                        exit_order_id = Some(exit_id);
                                        bars_since_entry = 0;
                                    }
                                    else if is_short {
                                        let position_size = strategy.position_size(&account, &symbol_code);
                                        let exit_id = strategy.exit_short(&candle.symbol.name, None, &account, None, position_size, String::from("Exit Short")).await;
                                        exit_order_id = Some(exit_id);
                                        bars_since_entry = 0;
                                    }
                                }

                                //Take smaller profit if we add and don't get momentum
                                if bars_since_entry > 5 && open_profit < dec!(60) && open_profit >= dec!(30) && position_size > dec!(2) && add_order_id.is_none() && entry_order_id.is_none() && exit_order_id.is_none() {

                                    let is_long = strategy.is_long(&account, &symbol_code);
                                    let is_short = strategy.is_short(&account, &symbol_code);
                                    let position_size = strategy.position_size(&account, &symbol_code);

                                    if is_long {
                                        let exit_id = strategy.exit_long(&candle.symbol.name, None, &account, None, position_size, String::from("No Momo Exit Long")).await;
                                        exit_order_id = Some(exit_id);
                                        bars_since_entry = 0;
                                    }
                                    else if is_short {
                                        let exit_id = strategy.exit_short(&candle.symbol.name, None, &account, None, position_size, String::from("No Momo Exit Short")).await;
                                        exit_order_id = Some(exit_id);
                                        bars_since_entry = 0;
                                    }
                                }
                            }

                        }
                        _ => {}
                    }
                }
            }
            StrategyEvent::ShutdownEvent(event) => {
                strategy.flatten_all_for(account).await;
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
                let msg = format!("{}, Time Local: {}", event, event.time_local(strategy.time_zone()));
                println!("{}", msg.as_str().purple());

                let quantity = strategy.position_size(&event.account(), &symbol_code);
                println!("Strategy: Open Quantity: {}", quantity);

                match event {
                    PositionUpdateEvent::PositionOpened { .. } => {

                    },
                    PositionUpdateEvent::Increased { .. } => {},
                    PositionUpdateEvent::PositionReduced { .. } => {},
                    PositionUpdateEvent::PositionClosed { booked_pnl, .. } => {
                        exit_order_id = None;
                        entry_order_id = None;
                        strategy.print_ledger(event.account());
                        if booked_pnl > dec!(0) {
                            last_result = TradeResult::Win
                        } else if booked_pnl < dec!(0) {
                            last_result = TradeResult::Loss
                        } else if booked_pnl == dec!(0) {
                            last_result = TradeResult::BreakEven
                        }
                        strategy.print_ledger(event.account()).await
                    }
                }
            }
            StrategyEvent::OrderEvents(event) => {
                match event.symbol_code() {
                    None => {}
                    Some(code) => symbol_code = code
                }
                strategy.print_ledger(event.account());
                let msg = format!("Strategy: Order Event: {}, Time: {}", event, event.time_local(strategy.time_zone()));
                match event {
                    OrderUpdateEvent::OrderRejected { order_id, .. } => {
                        println!("{}", msg.as_str().on_bright_magenta().on_bright_red());
                        let mut closed = false;
                        if let Some(entry_order_id) = &entry_order_id {
                            if *order_id == *entry_order_id {
                                closed = true;
                            }
                        }
                        if closed {
                            entry_order_id = None;
                        }
                        closed = false;
                        if let Some( exit_order_id) =  &exit_order_id {
                            if order_id == *exit_order_id {
                                closed = true;
                            }
                        }
                        if closed {
                            exit_order_id = None;
                        }

                        closed = false;
                        if let Some( add_order_id) =  &add_order_id {
                            if order_id == *add_order_id {
                                closed = true;
                            }
                        }
                        if closed {
                            add_order_id = None;
                        }
                    },
                    OrderUpdateEvent::OrderCancelled {order_id, ..} | OrderUpdateEvent::OrderFilled {order_id, ..} => {
                        println!("{}", msg.as_str().bright_cyan());

                        let mut closed = false;
                        if let Some(entry_order_id) = &entry_order_id {
                            if *order_id == *entry_order_id {
                                closed = true;
                            }
                        }
                        if closed {
                            entry_order_id = None;
                        }

                        closed = false;
                        if let Some( exit_order_id) =  &exit_order_id {
                            if order_id == *exit_order_id {
                                closed = true;
                            }
                        }
                        if closed {
                            exit_order_id = None;
                        }

                        closed = false;
                        if let Some( add_order_id) =  &add_order_id {
                            if order_id == *add_order_id {
                                closed = true;
                            }
                        }
                        if closed {
                            add_order_id = None;
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
    event_receiver.close();
    println!("Strategy: Event Loop Ended");
}