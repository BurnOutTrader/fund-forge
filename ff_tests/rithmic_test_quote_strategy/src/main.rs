use std::cmp::PartialEq;
use std::collections::HashMap;
use chrono::{Duration, NaiveDate, Timelike, Utc};
use chrono_tz::{Australia, UTC};
use colored::Colorize;
use ff_rithmic_api::systems::RithmicSystem;
use rust_decimal::Decimal;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::{FuturesExchange, MarketType, OrderSide, StrategyMode};
use ff_standard_lib::strategies::strategy_events::{StrategyEvent};
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, SymbolName};
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
/*
  - This strategy is designed to trade 1 symbol at a time, the logic will not work for multiple symbols.
    - It can trade multiple accounts / brokers.
    - It is designed to get into a position and add to winners, up to 4x the original position size. (2 contracts starting size, up to 8)
    - It will start by limiting in on a clean bull or bear bar closing on its high or low, where atr is increasing and above a minimum value.
    - If it is in a winner it will look to increase size on the next signal by taking a momentum stop limit entry up to 2x after initial entry.
    - Both the initial limit order and the add stop limit orders use a 15 to 30 second time in force to expire if not filled, this is done on the exchange side.
    - It will only enter trades when the atr looks to be increasing.
    - It will not enter trades below the minimum atr value.
    - It can only trade the same direction consecutively if the last trade was a win.
    - If the last trade was a loss it will only trade the opposite direction on the next trade.
    - It has not been back-tested, it is just tuned each day and depends heavily on manual symbol selection and common sense.
    - If synchronize_accounts is enabled the statistics will not be correct, but you will be able to open or close positions in rithmic and the strategy can interact with them.
    - If synchronize_accounts is disabled the statistics will be correct, but you will not be able to open or close positions in rithmic and the strategy will not interact with them,
        If you do not sync accounts, you should not interfere with the strategy and should also watch the strategy closely to ensure it is working as expected.
     - This strategy is for testing purposes, It is not 100% going to make money and is not a recommendation to trade live, although it is profitable for me.
*/

const IS_LONG_STRATEGY: bool = true;
const IS_SHORT_STRATEGY: bool = true;
const MAX_PROFIT: Decimal = dec!(9000);
const MAX_LOSS: Decimal = dec!(1500);
const MIN_ATR_VALUE: Decimal = dec!(01.25);
const PROFIT_TARGET: Decimal = dec!(150);
const RISK: Decimal = dec!(100);
const DATAVENDOR: DataVendor = DataVendor::Rithmic(RithmicSystem::Apex);

#[tokio::main]
async fn main() {
    //todo You will need to put in your paper account ID here or the strategy will crash on initialization, you can trade multiple accounts and brokers and mix and match data feeds.
    let account = Account::new(Brokerage::Rithmic(RithmicSystem::Apex), "YOUR_ACCOUNT_ID".to_string()); //todo change your brokerage to the correct broker, prop firm or rithmic system.
    let symbol_name = SymbolName::from("MNQ");
    let mut symbol_code = symbol_name.clone();
    symbol_code.push_str("Z24");

    let subscription = DataSubscription::new(
        symbol_name.clone(),
        DATAVENDOR,
        Resolution::Seconds(3),
        BaseDataType::QuoteBars,
        MarketType::Futures(FuturesExchange::CME));  //todo, dont forget to change the exchange for the symbol you are trading

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
            //subscribe to a quote feed to ensure we use quotes
            DataSubscription::new(
                symbol_name,
                DATAVENDOR,
                Resolution::Instant,
                BaseDataType::Quotes,
                MarketType::Futures(FuturesExchange::CME)  //todo, dont forget to change the exchange for the symbol you are trading
            ),
           /* DataSubscription::new(
                SymbolName::from("MNQ"),
                data_vendor.clone(),
                Resolution::Ticks(1),
                BaseDataType::Ticks,
                MarketType::Futures(FuturesExchange::CME)
            ),*/

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
    let mut bars_since_entry_map: HashMap<Account, u64> = strategy
        .accounts()
        .into_iter().map(|account| (account.clone(), 0))
        .collect();

    fn increment_bars_since_entry(bars_since_entry: &mut HashMap<Account, u64>, account: &Account) {
        if let Some(entry_count) = bars_since_entry.get_mut(account) {
            *entry_count += 1;
        }
    }

    // Function to reset bars_since_entry for a given account
    fn reset_bars_since_entry(bars_since_entry: &mut HashMap<Account, u64>, account: &Account) {
        if let Some(entry_count) = bars_since_entry.get_mut(account) {
            *entry_count = 0;
        }
    }
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
                        BaseDataEnum::Quote(quote) => {
                            //let msg = format!("{} Quote: {} @ {}", quote.symbol.name, quote.bid, quote.time_local(strategy.time_zone()));
                            //println!("{}", msg.as_str().yellow());
                        }
                        BaseDataEnum::QuoteBar(quotebar) => {
                            if quotebar.is_closed == true  {
                                let msg = format!("{} {} {} Close: {}, {}", quotebar.symbol.name, quotebar.resolution, quotebar.candle_type, quotebar.bid_close, quotebar.time_closed_local(strategy.time_zone()));
                                if quotebar.bid_close == quotebar.bid_open {
                                    println!("{}", msg.as_str().blue())
                                } else {
                                    match quotebar.bid_close > quotebar.bid_open {
                                        true => println!("{}", msg.as_str().bright_green()),
                                        false => println!("{}", msg.as_str().bright_red()),
                                    }
                                }

                                count += 1;

                                let last_candle = strategy.bar_index(&subscription, 1);
                                let last_atr = strategy.indicator_index(&atr_10.name(), 1);
                                let current_atr = strategy.indicator_index(&atr_10.name(), 0);

                                if last_candle.is_none() || quotebar.resolution != subscription.resolution || last_atr.is_none() || current_atr.is_none() {
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
                                let bar_time = quotebar.time_utc();

                                let high_close = match quotebar.bid_close.cmp(&quotebar.bid_open) {
                                    std::cmp::Ordering::Greater => { // Bullish candle
                                        // Close should be near high for bullish
                                        match quotebar.bid_high.cmp(&quotebar.bid_close) {
                                            std::cmp::Ordering::Greater => (quotebar.bid_high - quotebar.bid_close) / quotebar.bid_high <= dec!(0.05),
                                            _ => false
                                        }
                                    },
                                    _ => false // Must be bullish to check high close
                                };

                                let low_close = match quotebar.bid_close.cmp(&quotebar.bid_open) {
                                    std::cmp::Ordering::Less => { // Bearish candle
                                        // Close should be near low for bearish
                                        match quotebar.bid_close.cmp(&quotebar.bid_low) {
                                            std::cmp::Ordering::Greater => (quotebar.bid_close - quotebar.bid_low) / quotebar.bid_low <= dec!(0.05),
                                            _ => false
                                        }
                                    },
                                    _ => false // Must be bearish to check low close
                                };

                                // See if we have a clean bullish entry bar
                                let high_1 = quotebar.bid_low >= last_candle.bid_open && // Changed to >= for true support
                                    quotebar.bid_close > last_candle.ask_high &&
                                    high_close &&
                                    quotebar.bid_close > quotebar.bid_open; // Add check for bullish close

                                // See if we have a clean bearish entry bar
                                let low_1 = quotebar.ask_high <= last_candle.ask_open && // Changed to <= for true resistance
                                    quotebar.ask_close < last_candle.bid_low &&
                                    low_close &&
                                    quotebar.ask_close < quotebar.ask_open; // Add check for bearish close

                                for account in strategy.accounts() {

                                    // Check if time is between Chicago close (19:50) utc and NZ open (21:00) Utc or if we hit daily profit target or loss limit.
                                    if (bar_time.hour() == 19 && bar_time.minute() >= 50) || bar_time.hour() == 20 || booked_pnl >= MAX_PROFIT || booked_pnl <= -MAX_LOSS {
                                        // Close any existing positions
                                        if strategy.is_long(&account, &symbol_code) {
                                            let position_size = strategy.position_size(&account, &symbol_code);
                                            let exit_id = strategy.exit_long(&quotebar.symbol.name, None, &account, None, position_size, String::from("Exit Long")).await;
                                            exit_order_id = Some(exit_id);
                                            reset_bars_since_entry(&mut bars_since_entry_map, &account);
                                            last_side = LastSide::Long;
                                        } else if strategy.is_short(&account, &symbol_code) {
                                            let position_size = strategy.position_size(&account, &symbol_code);
                                            let exit_id = strategy.exit_short(&quotebar.symbol.name, None, &account, None, position_size, String::from("Exit Short")).await;
                                            exit_order_id = Some(exit_id);
                                            reset_bars_since_entry(&mut bars_since_entry_map, &account);
                                            last_side = LastSide::Short;
                                        }
                                        continue;
                                    }

                                    // entry orders
                                    if IS_LONG_STRATEGY && (last_side != LastSide::Long || (last_side == LastSide::Long && last_result == TradeResult::Win)) && is_flat && high_1 && entry_order_id.is_none() && atr_increasing && min_atr {
                                        //println!("Submitting long entry");
                                        let cancel_order_time = Utc::now() + Duration::seconds(30);
                                        let order_id = strategy.limit_order(&quotebar.symbol.name, None, &account, None, dec!(2), OrderSide::Buy, last_candle.bid_high, TimeInForce::Time(cancel_order_time.timestamp(), UTC.to_string()), String::from("Enter Long Limit")).await;
                                        entry_order_id = Some(order_id);
                                        exit_order_id = None;
                                        attempting_entry = "Long".to_string();
                                    } else if IS_SHORT_STRATEGY && (last_side != LastSide::Short || (last_side == LastSide::Short && last_result == TradeResult::Win)) && is_flat && low_1 && entry_order_id.is_none() && atr_increasing && min_atr {
                                        //println!("Submitting short limit");
                                        let cancel_order_time = Utc::now() + Duration::seconds(30);
                                        let order_id = strategy.limit_order(&quotebar.symbol.name, None, &account, None, dec!(2), OrderSide::Sell, last_candle.bid_high, TimeInForce::Time(cancel_order_time.timestamp(), UTC.to_string()), String::from("Enter Short Limit")).await;
                                        entry_order_id = Some(order_id);
                                        exit_order_id = None;
                                        attempting_entry = "Short".to_string();
                                    }

                                    // exit orders
                                    let is_long = strategy.is_long(&account, &symbol_code);
                                    let is_short = strategy.is_short(&account, &symbol_code);
                                    if is_long || is_short {
                                       increment_bars_since_entry(&mut bars_since_entry_map, &account);
                                    }

                                    let open_profit = strategy.pnl(&account, &symbol_code);
                                    let position_size = strategy.position_size(&account, &symbol_code);
                                    println!(
                                        "Account: {}, {}, Open Profit: {}, Position Size: {}, Last Entry Attempt: {}",
                                        account, symbol_code, open_profit, position_size, attempting_entry
                                    );

                                    let bars_since_entry = bars_since_entry_map.get(&account).unwrap().clone();
                                    //Add to winners up to 2x if we have momentum
                                    if (is_long || is_short) && bars_since_entry > 2 && open_profit >= dec!(40) && position_size <= dec!(5) && add_order_id.is_none()  {
                                        let cancel_order_time = Utc::now() + Duration::seconds(15);
                                        if IS_LONG_STRATEGY && is_long && high_1 {
                                            let new_add_order_id = strategy.stop_limit(&quotebar.symbol.name, None, &account, None, dec!(3), OrderSide::Buy, String::from("Add Long Stop Limit"), last_candle.ask_high + dec!(0.5), last_candle.ask_high + dec!(0.25), TimeInForce::Time(cancel_order_time.timestamp(), UTC.to_string())).await;
                                            reset_bars_since_entry(&mut bars_since_entry_map, &account);
                                            add_order_id = Some(new_add_order_id);
                                        } else if IS_SHORT_STRATEGY && is_short && low_1 {
                                            let new_add_order_id = strategy.stop_limit(&quotebar.symbol.name, None, &account, None, dec!(3), OrderSide::Sell, String::from("Add Short Stop Limit"), last_candle.bid_low - dec!(0.5), last_candle.bid_low - dec!(0.25), TimeInForce::Time(cancel_order_time.timestamp(), UTC.to_string())).await;
                                            reset_bars_since_entry(&mut bars_since_entry_map, &account);
                                            add_order_id = Some(new_add_order_id);
                                        }
                                    }

                                    let profit_goal = match position_size > dec!(5) {
                                        true => PROFIT_TARGET,
                                        false => PROFIT_TARGET * dec!(2)
                                    };

                                    // Cut losses and take profits, we check entry order is none to prevent exiting while an entry order is unfilled, entry order will expire and go to none on the TIF expiry, or on fill.
                                    if open_profit > profit_goal || (open_profit < RISK && bars_since_entry > 10) && exit_order_id.is_none() {
                                        let is_long = strategy.is_long(&account, &symbol_code);
                                        let is_short = strategy.is_short(&account, &symbol_code);

                                        if is_long {
                                            let position_size = strategy.position_size(&account, &symbol_code);
                                            let exit_id = strategy.exit_long(&quotebar.symbol.name, None, &account, None, position_size, String::from("Exit Long")).await;
                                            exit_order_id = Some(exit_id);
                                            reset_bars_since_entry(&mut bars_since_entry_map, &account);
                                        } else if is_short {
                                            let position_size = strategy.position_size(&account, &symbol_code);
                                            let exit_id = strategy.exit_short(&quotebar.symbol.name, None, &account, None, position_size, String::from("Exit Short")).await;
                                            exit_order_id = Some(exit_id);
                                            reset_bars_since_entry(&mut bars_since_entry_map, &account);
                                        }
                                    }

                                    //Take smaller profit if we add and don't get momentum
                                    let bars_since_entry = bars_since_entry_map.get(&account).unwrap().clone();
                                    if bars_since_entry > 5 && open_profit < dec!(60) && open_profit >= dec!(30) && position_size > dec!(2) && exit_order_id.is_none() {
                                        let is_long = strategy.is_long(&account, &symbol_code);
                                        let is_short = strategy.is_short(&account, &symbol_code);
                                        let position_size = strategy.position_size(&account, &symbol_code);

                                        if is_long {
                                            let exit_id = strategy.exit_long(&quotebar.symbol.name, None, &account, None, position_size, String::from("No Momo Exit Long")).await;
                                            exit_order_id = Some(exit_id);
                                            reset_bars_since_entry(&mut bars_since_entry_map, &account);
                                        }
                                        else if is_short {
                                            let exit_id = strategy.exit_short(&quotebar.symbol.name, None, &account, None, position_size, String::from("No Momo Exit Short")).await;
                                            exit_order_id = Some(exit_id);
                                            reset_bars_since_entry(&mut bars_since_entry_map, &account);
                                        }
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