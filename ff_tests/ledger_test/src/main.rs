use std::cmp::PartialEq;
use chrono::{Duration, NaiveDate};
use chrono_tz::Australia;
use colored::Colorize;
use rust_decimal::Decimal;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::{MarketType, StrategyMode};
use ff_standard_lib::strategies::strategy_events::{StrategyEvent};
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, SymbolName};
use ff_standard_lib::strategies::fund_forge_strategy::FundForgeStrategy;
use rust_decimal_macros::dec;
use tokio::sync::mpsc;
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
        Currency::USD,
        NaiveDate::from_ymd_opt(2024, 10, 8).unwrap().and_hms_opt(0, 0, 0).unwrap(), // Starting date of the backtest is a NaiveDateTime not NaiveDate
        NaiveDate::from_ymd_opt(2024, 10, 10).unwrap().and_hms_opt(0, 0, 0).unwrap(), // Ending date of the backtest is a NaiveDateTime not NaiveDate
        Australia::Sydney,                      // the strategy time zone
        Duration::hours(1), // the warmup duration, the duration of historical data we will pump through the strategy to warm up indicators etc before the strategy starts executing.
        vec![
            // Since we only have quote level test data, the 2 subscriptions will be created by consolidating the quote feed. Quote data will automatically be subscribed as primary data source.
            (None, DataSubscription::new(
                SymbolName::from("NAS100-USD"),
                DataVendor::Oanda,
                Resolution::Seconds(5),
                BaseDataType::QuoteBars,
                MarketType::CFD
            ), None),
        ],

        //fill forward
        false,

        // history to retain for our initial subscriptions
        100,

        // the sender for the strategy events
        strategy_event_sender,

        // Buffer Duration
        //strategy resolution in milliseconds, all data at a lower resolution will be consolidated to this resolution, if using tick data, you will want to set this at 100 or less depending on the data granularity
        core::time::Duration::from_millis(100),


        // Enabled will launch the strategy registry handler to connect to a GUI, currently will crash if enabled
        false,

        //tick over no data, strategy will run at buffer resolution speed to simulate weekends and holidays, if false we will just skip over them to the next data point.
        false,
        false,
        vec![Account::new(Brokerage::Oanda, "Test_Account_1".to_string()), Account::new(Brokerage::Oanda, "Test_Account_2".to_string())]
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
    let account_1 = Account::new(Brokerage::Oanda, "Test_Account_1".to_string());
    let mut last_side = LastSide::Flat;

    'strategy_loop: while let Some(strategy_event) = event_receiver.recv().await {
        match strategy_event {
            StrategyEvent::TimeSlice(time_slice) => {
                for base_data in time_slice.iter() {
                    match base_data {
                        BaseDataEnum::Candle(_candle) => {}

                        BaseDataEnum::QuoteBar(qb) => {
                            if qb.is_closed == true {
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
                                    let is_flat = strategy.is_flat(&account_1, &qb.symbol.name);
                                    // buy AUD-CAD if consecutive green HA candles if our other account is long on EUR
                                    if is_flat
                                        && qb.bid_close > qb.bid_open
                                    {
                                        let _entry_order_id = strategy.enter_long(&qb.symbol.name, None, &account_1, None, dec!(1), String::from("Enter Long")).await;
                                        println!("Strategy: Enter Long, Time {}", strategy.time_local());
                                        last_side = LastSide::Long;
                                    }

                                    // ADD LONG
                                    let is_short = strategy.is_short(&account_1, &qb.symbol.name);
                                    let is_long = strategy.is_long(&account_1, &qb.symbol.name);
                                    let long_pnl = strategy.pnl(&account_1, &qb.symbol.name);
                                    println!("Open pnl: {}, Is_short: {}, is_long:{} ", long_pnl, is_short, is_long);

                                    // LONG SL+TP
                                    if is_long && long_pnl > dec!(10.0) {
                                        let position_size = strategy.position_size(&account_1, &qb.symbol.name);
                                        let _exit_order_id = strategy.exit_long(&qb.symbol.name, None, &account_1, None, position_size, String::from("Exit Long Take Profit")).await;
                                        println!("Strategy: Exit Long Take Profit, Time {}", strategy.time_local());  // Fixed message
                                    } else if is_long
                                        && long_pnl <= dec!(-10.0)
                                    {
                                        let position_size: Decimal = strategy.position_size(&account_1, &qb.symbol.name);
                                        let _exit_order_id = strategy.exit_long(&qb.symbol.name, None, &account_1, None, position_size, String::from("Exit Long Take Loss")).await;
                                        println!("Strategy: Exit Long Take Loss, Time {}", strategy.time_local());
                                    }
                                }
                            }
                        }

                        BaseDataEnum::Quote(q) => {
                            println!("{}", q);
                        }
                        _ => {}
                    }
                }
            }
            StrategyEvent::ShutdownEvent(event) => {
                strategy.flatten_all_for(account_1.clone()).await;
                let msg = format!("{}",event);
                println!("{}", msg.as_str().bright_magenta());
                strategy.export_trades_to_csv(&account_1, &String::from("./trades exports"));
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
                    PositionUpdateEvent::PositionReduced { .. } => {
                        strategy.print_ledger(event.account())
                    },
                    PositionUpdateEvent::PositionClosed { .. } => {
                        strategy.print_ledger(event.account())
                    },
                }
                let quantity = strategy.position_size(&account_1, &"EUR-USD".to_string());
                let msg = format!("{}, Time Local: {}", event, event.time_local(strategy.time_zone()));
                println!("{}", msg.as_str().purple());
                println!("Strategy: Open Quantity: {}", quantity);
            }
            StrategyEvent::OrderEvents(event) => {
                let msg = format!("Strategy: Order Event: {}, Time: {}", event, event.time_local(strategy.time_zone()));
                match event {
                    OrderUpdateEvent::OrderRejected { .. } | OrderUpdateEvent::OrderUpdateRejected { .. } => {
                        strategy.print_ledger(event.account());
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