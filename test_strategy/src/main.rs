use chrono::{Datelike, Duration, NaiveDate};
use chrono_tz::Australia;
use ff_standard_lib::indicators::indicator_handler::IndicatorEvents;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::{MarketType, Resolution, StrategyMode};
use ff_standard_lib::standardized_types::strategy_events::{
    EventTimeSlice, StrategyEvent, StrategyInteractionMode,
};
use ff_standard_lib::standardized_types::subscriptions::{CandleType, DataSubscription, SymbolName};
use ff_strategies::fund_forge_strategy::FundForgeStrategy;
use std::sync::Arc;
use rust_decimal_macros::dec;
use tokio::sync::{mpsc, Notify};
use ff_standard_lib::apis::brokerage::broker_enum::Brokerage;
use ff_standard_lib::apis::data_vendor::datavendor_enum::DataVendor;
use ff_standard_lib::indicators::built_in::average_true_range::AverageTrueRange;
use ff_standard_lib::indicators::indicator_enum::IndicatorEnum;
use ff_standard_lib::indicators::indicators_trait::IndicatorName;
use ff_standard_lib::server_connections::{GUI_DISABLED};
use ff_standard_lib::standardized_types::accounts::ledgers::AccountId;
use ff_standard_lib::standardized_types::base_data::candle::Candle;
use ff_standard_lib::standardized_types::base_data::quotebar::QuoteBar;
use ff_standard_lib::standardized_types::Color;
use ff_standard_lib::standardized_types::orders::orders::OrderUpdateEvent;
use ff_standard_lib::standardized_types::rolling_window::RollingWindow;


// to launch on separate machine
#[tokio::main]
async fn main() {
    let (strategy_event_sender, strategy_event_receiver) = mpsc::channel(1000);
    let notify = Arc::new(Notify::new());
    // we initialize our strategy as a new strategy, meaning we are not loading drawing tools or existing data from previous runs.
    let strategy = FundForgeStrategy::initialize(
        notify.clone(),
        StrategyMode::LivePaperTrading, // Backtest, Live, LivePaper
        StrategyInteractionMode::SemiAutomated, // In semi-automated the strategy can interact with the user drawing tools and the user can change data subscriptions, in automated they cannot. // the base currency of the strategy
        NaiveDate::from_ymd_opt(2024, 6, 10)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(), // Starting date of the backtest is a NaiveDateTime not NaiveDate
        NaiveDate::from_ymd_opt(2024, 06, 15)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(), // Ending date of the backtest is a NaiveDateTime not NaiveDate
        Australia::Sydney,                      // the strategy time zone
        Duration::days(1), // the warmup duration, the duration of historical data we will pump through the strategy to warm up indicators etc before the strategy starts executing.
        vec![
            DataSubscription::new_custom(
                SymbolName::from("EUR-USD"),
                DataVendor::Test,
                Resolution::Seconds(5),
                BaseDataType::QuoteBars,
                MarketType::Forex,
                CandleType::CandleStick,
            ),
            DataSubscription::new_custom(
                 SymbolName::from("AUD-CAD"),
                 DataVendor::Test,
                 Resolution::Seconds(5),
                 BaseDataType::QuoteBars,
                 MarketType::Forex,
                 CandleType::HeikinAshi,
             ),],
        5,
        strategy_event_sender, // the sender for the strategy events
        None,
        //strategy resolution, all data at a lower resolution will be consolidated to this resolution, if using tick data, you will want to set this at 1 second or less depending on the data granularity
        //this allows us full control over how the strategy buffers data and how it processes data, in live trading .
        Duration::milliseconds(100),
        GUI_DISABLED
    ).await;

    on_data_received(strategy, notify, strategy_event_receiver).await;
}

/// Here we listen for incoming data and build our custom strategy logic. this is where the magic happens.
pub async fn on_data_received(
    strategy: FundForgeStrategy,
    notify: Arc<Notify>,
    mut event_receiver: mpsc::Receiver<EventTimeSlice>,
) {
    let heikin_atr_5 = IndicatorEnum::AverageTrueRange(
        AverageTrueRange::new(IndicatorName::from("heikin_atr_5"),
              DataSubscription::new_custom(
                  SymbolName::from("EUR-USD"),
                  DataVendor::Test,
                  Resolution::Seconds(5),
                  BaseDataType::QuoteBars,
                  MarketType::Forex,
                  CandleType::CandleStick,
              ),
            100,
            5,
            Some(Color::new(50,50,50))
        ).await,
    );
    strategy.indicator_subscribe(heikin_atr_5).await;

    let brokerage = Brokerage::Test;
    let mut warmup_complete = true;
    let mut bars_since_entry_1 = 0;
    let mut bars_since_entry_2 = 0;
    let mut history_1 : RollingWindow<QuoteBar> = RollingWindow::new(10);
    let mut history_2 : RollingWindow<Candle> = RollingWindow::new(10);
    let account_name = AccountId::from("TestAccount");
    // The engine will send a buffer of strategy events at the specified buffer interval, it will send an empty buffer if no events were buffered in the period.
    'strategy_loop: while let Some(event_slice) = event_receiver.recv().await {
        for strategy_event in event_slice {
            match strategy_event {
                // when a drawing tool is added from some external source the event will also show up here (the tool itself will be added to the strategy.drawing_objects HashMap behind the scenes)
                StrategyEvent::DrawingToolEvents(event, _) => {
                    println!("Strategy: Drawing Tool Event: {:?}", event);
                }
                StrategyEvent::TimeSlice(time, time_slice) => {
                    // here we would process the time slice events and update the strategy state accordingly.
                    for base_data in &time_slice {
                        // only data we specifically subscribe to show up here, if the data is building from ticks but we didn't subscribe to ticks specifically, ticks won't show up but the subscribed resolution will.
                        match base_data {
                            BaseDataEnum::TradePrice(_trade_price) => {}
                            BaseDataEnum::Candle(candle) => {
                                // Place trades based on the AUD-CAD Heikin Ashi Candles
                                if candle.is_closed == true {
                                    println!("Candle {}: {}", candle.symbol.name, candle.time);
                                    history_2.add(candle.clone());
                                    let last_bar =
                                        match history_2.get(1) {
                                            None => {
                                                continue;
                                            },
                                            Some(bar) => bar
                                        };

                                    if !warmup_complete {
                                        continue;
                                    }

                                    if candle.close > last_bar.high
                                        && !strategy.is_long(&brokerage, &account_name, &candle.symbol.name).await
                                    {
                                        let _entry_order_id = strategy.enter_long(&candle.symbol.name, &account_name, &brokerage, dec!(1), String::from("Enter Long"), None).await;
                                        bars_since_entry_2 = 0;
                                    }
                                    else if bars_since_entry_2 > 10
                                        //&& last_bar.bid_close < two_bars_ago.bid_low
                                        && strategy.is_long(&brokerage, &account_name, &candle.symbol.name).await
                                    {
                                        let _exit_order_id = strategy.exit_long(&candle.symbol.name, &account_name, &brokerage,dec!(1), String::from("Exit Long")).await;
                                        bars_since_entry_2 = 0;
                                    }

                                    if strategy.is_long(&brokerage, &account_name, &candle.symbol.name).await {
                                        bars_since_entry_2 += 1;
                                    }
                                }
                            }
                            BaseDataEnum::QuoteBar(quotebar) => {
                                // Place trades based on the EUR-USD QuoteBars
                                if quotebar.is_closed == true {
                                    println!("QuoteBar {}: {}", quotebar.symbol.name, quotebar.time);
                                    history_1.add(quotebar.clone());
                                    let last_bar =
                                    match history_1.get(1) {
                                        None => {
                                            continue;
                                        },
                                        Some(bar) => bar
                                    };

                                    if !warmup_complete {
                                        continue;
                                    }
                                    //todo, make a candle_index and quote_bar_index to get specific data types and save pattern matching


                                    if quotebar.bid_close > last_bar.bid_high
                                        && !strategy.is_long(&brokerage, &account_name, &quotebar.symbol.name).await
                                    {
                                        let _entry_order_id = strategy.enter_long(&quotebar.symbol.name, &account_name, &brokerage, dec!(1), String::from("Enter Long"), None).await;
                                        bars_since_entry_1 = 0;
                                    }
                                    else if bars_since_entry_1 > 10
                                        //&& last_bar.bid_close < two_bars_ago.bid_low
                                        && strategy.is_long(&brokerage, &account_name, &quotebar.symbol.name).await
                                    {
                                        let _exit_order_id = strategy.exit_long(&quotebar.symbol.name, &account_name, &brokerage,dec!(1), String::from("Exit Long")).await;
                                        bars_since_entry_1 = 0;
                                    }

                                    if strategy.is_long(&brokerage, &account_name, &quotebar.symbol.name).await {
                                        bars_since_entry_1 += 1;
                                    }
                                }
                                //do something with the current open bar
                                if !quotebar.is_closed {
                                    //println!("Open bar time: {}", time)
                                }
                            }
                            BaseDataEnum::Tick(_tick) => {}
                            BaseDataEnum::Quote(quote) => {
                                // primary data feed won't show up in event loop unless specifically subscribed by the strategy
                                println!(
                                    "{} Quote: {}",
                                    quote.symbol.name,
                                    base_data.time_created_utc()
                                );
                            }
                            BaseDataEnum::Fundamental(_fundamental) => {}
                        }
                    }
                }
                // order updates are received here, excluding order creation events, the event loop here starts with an OrderEvent::Accepted event and ends with the last fill, rejection or cancellation events.
                StrategyEvent::OrderEvents(event) => {
                    match &event {
                        OrderUpdateEvent::Accepted { brokerage, account_id, order_id } => {
                            println!("{:?}", strategy.print_ledger(brokerage.clone(), account_id.clone()).await.unwrap());
                        }
                        OrderUpdateEvent::Filled { brokerage, account_id, order_id } => {
                            println!("{:?}", strategy.print_ledger(brokerage.clone(), account_id.clone()).await.unwrap());
                        },
                        OrderUpdateEvent::PartiallyFilled { brokerage, account_id, order_id } => {
                            println!("{:?}", strategy.print_ledger(brokerage.clone(), account_id.clone()).await.unwrap());
                        }
                        OrderUpdateEvent::Cancelled { brokerage, account_id, order_id } => {
                            println!("{:?}", strategy.print_ledger(brokerage.clone(), account_id.clone()).await.unwrap());
                        }
                        OrderUpdateEvent::Rejected { brokerage, account_id, order_id, reason } => {
                            println!("{:?}", strategy.print_ledger(brokerage.clone(), account_id.clone()).await.unwrap());
                        }
                        OrderUpdateEvent::Updated { brokerage, account_id, order_id } => {
                            println!("{:?}", strategy.print_ledger(brokerage.clone(), account_id.clone()).await.unwrap());
                        }
                        OrderUpdateEvent::UpdateRejected { brokerage, account_id, order_id, reason } => {
                            println!("{:?}", strategy.print_ledger(brokerage.clone(), account_id.clone()).await.unwrap());
                        }
                    };
                    println!("{}, Strategy: Order Event: {:?}", strategy.time_utc(), event);
                }
                // if an external source adds or removes a data subscription it will show up here, this is useful for SemiAutomated mode
                StrategyEvent::DataSubscriptionEvents(events,_) => {
                    for event in events {
                        println!("Strategy: Data Subscription Event: {:?}", event);
                    }
                }
                // strategy controls are received here, this is useful for SemiAutomated mode. we could close all positions on a pause of the strategy, or custom handle other user inputs.
                StrategyEvent::StrategyControls(control, _) => {}
                StrategyEvent::ShutdownEvent(event) => {
                    println!("{}",event);
                    //strategy.export_trades(&String::from("/Users/kevmonaghan/RustroverProjects/Test Trade Exports"));
                    let ledgers = strategy.print_ledgers().await;
                    for ledger in ledgers {
                        println!("{:?}", ledger);
                    }
                    break 'strategy_loop
                }, //we should handle shutdown gracefully by first ending the strategy loop.
                StrategyEvent::WarmUpComplete{} => {
                    println!("Strategy: Warmup Complete");
                    warmup_complete = true;
                }
                StrategyEvent::IndicatorEvent(indicator_event) => {
                    //we can handle indicator events here, this is useful for debugging and monitoring the state of the indicators.
                    match indicator_event {
                        IndicatorEvents::IndicatorAdded(added_event) => {
                            println!("Strategy:Indicator Added: {:?}", added_event);
                        }
                        IndicatorEvents::IndicatorRemoved(removed_event) => {
                            println!("Strategy:Indicator Removed: {:?}", removed_event);
                        }
                        IndicatorEvents::IndicatorTimeSlice(slice_event) => {
                            // we can see our auto manged indicator values for here.
                            for indicator_values in slice_event {
                                for (_name, plot) in indicator_values.values(){
                                    println!("{}: {:?}", plot.name, plot.value);
                                }
                            }
                        }
                        IndicatorEvents::Replaced(replace_event) => {
                            println!("Strategy:Indicator Replaced: {:?}", replace_event);
                        }
                    }

                    // we could also get the automanaged indicator values from teh strategy at any time.
                }
                StrategyEvent::PositionEvents => {

                }
            }
        }
        notify.notify_one();
    }
    event_receiver.close();
    println!("Strategy: Event Loop Ended");
}
