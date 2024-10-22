use std::cmp::PartialEq;
use chrono::{Duration, NaiveDate, Utc};
use chrono_tz::{Australia, UTC};
use colored::Colorize;
use ff_rithmic_api::systems::RithmicSystem;
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
// to launch on separate machine
#[tokio::main]
async fn main() {

    let account = Account::new(Brokerage::Rithmic(RithmicSystem::RithmicPaperTrading), "TPT1053217".to_string());
    let data_vendor = DataVendor::Rithmic(RithmicSystem::RithmicPaperTrading);
    let subscription = DataSubscription::new(
        SymbolName::from("MNQ"),
        data_vendor.clone(),
        Resolution::Seconds(1),
        BaseDataType::QuoteBars,
        MarketType::Futures(FuturesExchange::CME));

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
            //subscribe to a quote feed to ensure we use quotes
            DataSubscription::new(
                SymbolName::from("MNQ"),
                data_vendor.clone(),
                Resolution::Instant,
                BaseDataType::Quotes,
                MarketType::Futures(FuturesExchange::CME)
            ),
            //subscribe to our subscription
            subscription.clone()
        ],
        true,
        100,
        strategy_event_sender,
        core::time::Duration::from_millis(10),
        false,
        true,
        false,
        vec![account.clone()]
    ).await;

    on_data_received(strategy, strategy_event_receiver, account, subscription).await;
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
    subscription: DataSubscription
) {
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
            Color::new (128, 0, 128)
        ).await,
    );

    //if you set auto subscribe to false and change the resolution, the strategy will intentionally panic to let you know you won't have data for the indicator
    strategy.subscribe_indicator(atr_10.clone(), false).await;

    let mut warmup_complete = false;
    let mut last_side = LastSide::Flat;
    let account: Account = Account::new(Brokerage::Rithmic(RithmicSystem::RithmicPaperTrading), "TPT1053217".to_string());
    let mut symbol_code = "MNQZ4".to_string();
    println!("Staring Strategy Loop");
    let symbol = "MNQ".to_string();
    let mut count = 0;
    let mut bars_since_entry = 0;
    let mut entry_order_id = None;
    let mut add_order_id = None;
    let mut exit_order_id = None;
    let mut position_size = 0;
    let atr_plot = "atr".to_string();
    let mut last_result = TradeResult::BreakEven;
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
                            // Place trades based on the AUD-CAD Heikin Ashi Candles

                        }
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


                                if last_candle.is_none() || quotebar.resolution != Resolution::Seconds(1) || last_atr.is_none() || current_atr.is_none() {
                                    println!("Last Candle or Indicator Values Is None");
                                    continue;
                                }

                                let is_flat = strategy.is_flat(&account, &symbol_code);
                                let last_candle = last_candle.unwrap();
                                let last_atr = last_atr.unwrap().get_plot(&atr_plot).unwrap().value;
                                let current_atr = current_atr.unwrap().get_plot(&atr_plot).unwrap().value;
                                let min_atr = current_atr >= dec!(0.75);
                                let atr_increasing = current_atr > last_atr;

                                    // entry orders
                                if (last_side != LastSide::Long || (last_side == LastSide::Long && last_result == TradeResult::Win)) && is_flat && quotebar.bid_close > last_candle.bid_high && entry_order_id.is_none() && atr_increasing && min_atr {
                                    println!("Submitting long entry");
                                    let cancel_order_time = Utc::now() + Duration::seconds(15);
                                    let order_id = strategy.limit_order(&symbol, None, &account, None,dec!(2), OrderSide::Buy, last_candle.bid_high, TimeInForce::Time(cancel_order_time.timestamp(), UTC.to_string()), String::from("Enter Long Limit")).await;
                                    entry_order_id = Some(order_id);
                                    exit_order_id = None;

                                }
                                else if (last_side != LastSide::Short || (last_side == LastSide::Short && last_result == TradeResult::Win)) && is_flat && quotebar.bid_close < last_candle.bid_low && entry_order_id.is_none() && atr_increasing && min_atr {
                                    println!("Submitting short limit");
                                    let cancel_order_time = Utc::now() + Duration::seconds(30);
                                    let order_id = strategy.limit_order(&symbol, None, &account, None,dec!(2), OrderSide::Sell, last_candle.bid_high, TimeInForce::Time(cancel_order_time.timestamp(), UTC.to_string()), String::from("Enter Short Limit")).await;
                                    entry_order_id = Some(order_id);
                                    exit_order_id = None;
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
                                    "Bars: {}, Open Profit: {}, Position Size: {}",
                                    bars_since_entry, open_profit, position_size
                                );

                                if (is_long || is_short) && bars_since_entry > 1 && open_profit >= dec!(10) && position_size == dec!(2) {
                                    let cancel_order_time = Utc::now() + Duration::seconds(5);
                                    if is_long && quotebar.ask_close < last_candle.ask_high {
                                        let new_add_order_id = strategy.stop_limit(&symbol, None, &account, None,dec!(3), OrderSide::Buy,  String::from("Add Long Stop Limit"), last_candle.ask_high + dec!(0.5), last_candle.ask_high + dec!(0.25), TimeInForce::Time(cancel_order_time.timestamp(), UTC.to_string())).await;
                                        bars_since_entry = 0;
                                        exit_order_id = None;
                                        add_order_id = Some(new_add_order_id)
                                    }
                                    else if is_short && quotebar.bid_close > last_candle.bid_low {
                                        let new_add_order_id =strategy.stop_limit(&symbol, None, &account, None,dec!(3), OrderSide::Sell,  String::from("Add Short Stop Limit"), last_candle.bid_low - dec!(0.5), last_candle.bid_low - dec!(0.25), TimeInForce::Time(cancel_order_time.timestamp(), UTC.to_string())).await;
                                        bars_since_entry = 0;
                                        exit_order_id = None;
                                        add_order_id = Some(new_add_order_id)
                                    }
                                }

                                if open_profit > dec!(100) || (open_profit < dec!(-30) && bars_since_entry > 10) && add_order_id.is_none() && entry_order_id.is_none() {
                                    let open_profit = strategy.pnl(&account, &symbol_code);
                                    let is_long = strategy.is_long(&account, &symbol_code);
                                    let is_short = strategy.is_short(&account, &symbol_code);
                                    if is_long && exit_order_id == None {
                                        let position_size = strategy.position_size(&account, &symbol_code);
                                        let exit_id = strategy.exit_long(&symbol, None, &account, None, position_size, String::from("Exit Long")).await;
                                        exit_order_id = Some(exit_id);
                                        bars_since_entry = 0;
                                        last_side = LastSide::Long;
                                    }
                                    else if is_short && exit_order_id == None {
                                        let position_size = strategy.position_size(&account, &symbol_code);
                                        let exit_id = strategy.exit_short(&symbol, None, &account, None, position_size, String::from("Exit Short")).await;
                                        exit_order_id = Some(exit_id);
                                        bars_since_entry = 0;
                                        last_side = LastSide::Short;
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
                    PositionUpdateEvent::PositionOpened { .. } => {},
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
                    Some(code) => {
                        if code.starts_with("MNQ") {
                            symbol_code = code;
                        }
                    }
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