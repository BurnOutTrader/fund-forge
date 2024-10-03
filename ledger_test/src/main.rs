use chrono::{Duration, NaiveDate};
use chrono_tz::Australia;
use colored::Colorize;
use rust_decimal::Decimal;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::{MarketType, StrategyMode};
use ff_standard_lib::strategies::strategy_events::{StrategyEvent, StrategyEventBuffer};
use ff_standard_lib::standardized_types::subscriptions::{CandleType, DataSubscription, SymbolName};
use ff_standard_lib::strategies::fund_forge_strategy::FundForgeStrategy;
use rust_decimal_macros::dec;
use tokio::sync::mpsc;
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::strategies::ledgers::{AccountId, Currency};
use ff_standard_lib::strategies::client_features::connection_types::GUI_DISABLED;
use ff_standard_lib::standardized_types::position::PositionUpdateEvent;
use ff_standard_lib::standardized_types::resolution::Resolution;

// to launch on separate machine
#[tokio::main]
async fn main() {
    let (strategy_event_sender, strategy_event_receiver) = mpsc::channel(1000);
    let strategy = FundForgeStrategy::initialize(
        StrategyMode::Backtest,
        dec!(100000),
        Currency::USD,
        NaiveDate::from_ymd_opt(2024, 6, 19).unwrap().and_hms_opt(0, 0, 0).unwrap(),
        NaiveDate::from_ymd_opt(2024, 06, 21).unwrap().and_hms_opt(0, 0, 0).unwrap(),
        Australia::Sydney,
        Duration::hours(5),
        vec![
            DataSubscription::new_custom(
                SymbolName::from("EUR-USD"),
                DataVendor::Test,
                Resolution::Minutes(3),
                MarketType::Forex,
                CandleType::HeikinAshi
            ),
        ],
        false,
        100,
        strategy_event_sender,
        Some(core::time::Duration::from_millis(100)),
        //None,

        GUI_DISABLED
    ).await;

    on_data_received(strategy, strategy_event_receiver).await;
}

pub async fn on_data_received(
    strategy: FundForgeStrategy,
    mut event_receiver: mpsc::Receiver<StrategyEventBuffer>,
) {
    let brokerage = Brokerage::Test;
    let mut warmup_complete = false;
    let account_1 = AccountId::from("Test_Account_1");
    let account_2 = AccountId::from("Test_Account_2");
    let mut trade_placed_1 = false;
    let mut trade_placed_2 = false;
    // The engine will send a buffer of strategy events at the specified buffer interval, it will send an empty buffer if no events were buffered in the period.
    'strategy_loop: while let Some(event_slice) = event_receiver.recv().await {
        for (_time, strategy_event) in event_slice.iter() {
            match strategy_event {
                // when a drawing tool is added from some external source the event will also show up here (the tool itself will be added to the strategy.drawing_objects HashMap behind the scenes)
                StrategyEvent::DrawingToolEvents(_event) => {}

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

                                    if !warmup_complete {
                                        continue;
                                    }

                                    //long test
                                    {
                                        let is_long = strategy.is_long(&brokerage, &account_1, &candle.symbol.name);
                                        // buy AUD-CAD if consecutive green HA candles if our other account is long on EUR
                                        if !is_long && candle.close > candle.open {
                                            let _entry_order_id = strategy.enter_long(&candle.symbol.name, &account_1, &brokerage, dec!(30), String::from("Enter Long")).await;
                                            trade_placed_1 = true;
                                        }
                                        // take profit conditions

                                        let pnl = strategy.pnl(&brokerage, &account_1, &candle.symbol.name);
                                        let position_size: Decimal = strategy.position_size(&brokerage, &account_1, &candle.symbol.name);
                                        if is_long && pnl > dec!(100.0) {
                                            let _exit_order_id = strategy.exit_long(&candle.symbol.name, &account_1, &brokerage, position_size, String::from("Exit Long Take Profit")).await;
                                        } else if is_long && pnl < dec!(100.0) {
                                            let _exit_order_id = strategy.exit_long(&candle.symbol.name, &account_1, &brokerage, position_size, String::from("Exit Long Take Loss")).await;
                                        }
                                    }

                                    //short test
                                    {
                                        let is_short = strategy.is_short(&brokerage, &account_2, &candle.symbol.name);
                                        // test short ledger
                                        if !is_short && candle.close < candle.open {
                                            let _entry_order_id = strategy.enter_short(&candle.symbol.name, &account_2, &brokerage, dec!(30), String::from("Enter Short")).await;
                                            trade_placed_2 = true;
                                        }

                                        let pnl_2 = strategy.pnl(&brokerage, &account_2, &candle.symbol.name);
                                        let position_size_2: Decimal = strategy.position_size(&brokerage, &account_2, &candle.symbol.name);
                                        // take profit conditions
                                        if is_short && pnl_2 > dec!(100.0) {
                                            let _exit_order_id = strategy.exit_short(&candle.symbol.name, &account_2, &brokerage, position_size_2, String::from("Exit Short Take Profit")).await;
                                        } else  if is_short && pnl_2 < dec!(100.0) {
                                            let _exit_order_id = strategy.exit_short(&candle.symbol.name, &account_2, &brokerage, position_size_2, String::from("Exit Short Take Loss")).await;
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }

                StrategyEvent::ShutdownEvent(event) => {
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
                        PositionUpdateEvent::PositionOpened { .. } => {}
                        PositionUpdateEvent::Increased { .. } => {}
                        PositionUpdateEvent::PositionReduced { .. } => strategy.print_ledger(event.brokerage(), event.account_id()),
                        PositionUpdateEvent::PositionClosed { .. } => strategy.print_ledger(event.brokerage(), event.account_id()),
                    }
                    let msg = format!("{}", event);
                    println!("{}", msg.as_str().purple())
                }
                _ => {}
            }
        }
    }
    event_receiver.close();
    println!("Strategy: Event Loop Ended");
}