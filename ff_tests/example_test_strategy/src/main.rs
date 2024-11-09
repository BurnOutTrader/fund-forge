use chrono::{Duration, NaiveDate};
use chrono_tz::Australia;
use colored::Colorize;
use rust_decimal::Decimal;
use ff_standard_lib::strategies::indicators::indicator_events::IndicatorEvents;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::{MarketType, OrderSide, StrategyMode};
use ff_standard_lib::strategies::strategy_events::{StrategyControls, StrategyEvent};
use ff_standard_lib::standardized_types::subscriptions::{CandleType, DataSubscription, SymbolName};
use ff_standard_lib::strategies::fund_forge_strategy::FundForgeStrategy;
use rust_decimal_macros::dec;
use tokio::sync::mpsc;
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::strategies::indicators::built_in::average_true_range::AverageTrueRange;
use ff_standard_lib::strategies::indicators::indicator_enum::IndicatorEnum;
use ff_standard_lib::strategies::indicators::indicators_trait::IndicatorName;
use ff_standard_lib::standardized_types::base_data::quotebar::QuoteBar;
use ff_standard_lib::gui_types::settings::Color;
use ff_standard_lib::standardized_types::accounts::{Account, Currency};
use ff_standard_lib::standardized_types::orders::{OrderId, OrderState, OrderUpdateEvent, TimeInForce};
use ff_standard_lib::standardized_types::position::PositionUpdateEvent;
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::strategies::indicators::indicator_values::IndicatorValues;

#[tokio::main]
async fn main() {

    // We create the sender and receiver for receiving the strategy event buffers
    let (strategy_event_sender, strategy_event_receiver) = mpsc::channel(1000);

    // we initialize our strategy as a new strategy, meaning we are not loading drawing tools or existing data from previous runs.
    let strategy = FundForgeStrategy::initialize(
        //ToDo: You can Test Live paper using the simulated data feed which simulates quote stream from the server side at 10 ms per quote.
        StrategyMode::Backtest, // Backtest, Live, LivePaper
        dec!(100000),
        Currency::USD,
        NaiveDate::from_ymd_opt(2011, 1, 20).unwrap().and_hms_opt(0, 0, 0).unwrap(), // Starting date of the backtest is a NaiveDateTime not NaiveDate
        NaiveDate::from_ymd_opt(2011, 01, 25).unwrap().and_hms_opt(0, 0, 0).unwrap(), // Ending date of the backtest is a NaiveDateTime not NaiveDate
        Australia::Sydney,                      // the strategy time zone
        Duration::hours(1), // the warmup duration, the duration of historical data we will pump through the strategy to warm up indicators etc before the strategy starts executing.
        vec![
            // Since we only have quote level test data, the 2 subscriptions will be created by consolidating the quote feed. Quote data will automatically be subscribed as primary data source.
            DataSubscription::new(
                SymbolName::from("AUD-JPY"),
                DataVendor::Oanda,
                Resolution::Seconds(5),
                BaseDataType::QuoteBars,
                MarketType::Forex
            ),
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
        vec![Account::new(Brokerage::Oanda, "Test_Account_1".to_string())]
    ).await;

    // we can subscribe to indicators here or in our event loop at run time.
    let quotebar_5s_atr_5 = IndicatorEnum::AverageTrueRange(
        AverageTrueRange::new(
            IndicatorName::from("quotebar_5s_atr_5"),
              // The subscription for the indicator
              DataSubscription::new(
                  SymbolName::from("AUD-JPY"),
                  DataVendor::Oanda,
                  Resolution::Seconds(5),
                  BaseDataType::QuoteBars,
                  MarketType::Forex,
              ),

              // history to retain
              100,

              // atr period
              5,

              // Plot color for GUI or println!()
              Color::new (128, 0, 128),

            true
        ).await,
    );

    //if you set auto subscribe to false and change the resolution, the strategy will intentionally panic to let you know you won't have data for the indicator
    strategy.subscribe_indicator(quotebar_5s_atr_5, true).await;

    // Start receiving the buffers
    on_data_received(strategy, strategy_event_receiver).await;
}

/// Here we listen for incoming data and build our custom strategy logic. this is where the magic happens.
pub async fn on_data_received(
    strategy: FundForgeStrategy,
    mut event_receiver: mpsc::Receiver<StrategyEvent>,
) {
    let account = Account::new(Brokerage::Oanda, "Test_Account_1".to_string());
    let mut warmup_complete = false;
    let mut bars_since_entry = 0;
    let mut entry_order_id: Option<OrderId> = None;
    let mut entry_order_state = OrderState::Created;

    'strategy_loop: while let Some(strategy_event) = event_receiver.recv().await {
        match strategy_event {
            StrategyEvent::DrawingToolEvents(_event) => {}
            StrategyEvent::TimeSlice(time_slice) => {
                for base_data in time_slice.iter() {

                    match base_data {
                        // Limit Order Strategy
                        BaseDataEnum::QuoteBar(quotebar) => {
                            // Place trades based on the EUR-USD QuoteBars
                            if quotebar.is_closed == true {
                                let msg = format!("{} {} QuoteBar Close: {}, {}", quotebar.symbol.name, quotebar.resolution, quotebar.bid_close, quotebar.time_closed_local(strategy.time_zone()));
                                if quotebar.bid_close == quotebar.bid_open {
                                    println!("{}", msg.as_str().blue())
                                } else {
                                    match quotebar.bid_close > quotebar.bid_open {
                                        true => println!("{}", msg.as_str().bright_green()),
                                        false => println!("{}", msg.as_str().bright_red()),
                                    }
                                }

                                if !warmup_complete {
                                    continue;
                                }

                                if quotebar.resolution == Resolution::Seconds(5) && quotebar.symbol.name == "AUD-JPY"  {
                                    // We are using a limit order to enter here, so we will manage our order differently. there are a number of ways to do this, this is probably not the best way.
                                    // Using Option<OrderId> for entry order as an alternative to is_long()
                                    if entry_order_id.is_some() {
                                        bars_since_entry += 1;
                                    }

                                    //if we start the warm up on a weekend, this unwrap will crash, because we didn't have warm up data available for the warm up period.
                                    // To avoid this in live strategies we should set a warm up period >= 3 days for strategies that we need to frequently stop and start.
                                    let last_bar: QuoteBar = strategy.bar_index(&base_data.subscription(), 1).unwrap();

                                    // Since our "heikin_3m_atr_5" indicator was consumed when we used the strategies auto mange strategy.subscribe_indicator() function,
                                    // we can use the name we assigned to get the indicator. We unwrap() since we should have this value, if we don't our strategy logic has a flaw.
                                    let quotebar_3m_atr_5_current_values: IndicatorValues = strategy.indicator_index(&"quotebar_5s_atr_5".to_string(), 0).unwrap();
                                    let quotebar_3m_atr_5_last_values: IndicatorValues = strategy.indicator_index(&"quotebar_5s_atr_5".to_string(), 1).unwrap();

                                    // We want to check the current value for the "atr" plot of the atr indicator. We unwrap() since we should have this value, if we don't our strategy logic has a flaw.
                                    let current_heikin_3m_atr_5: Decimal = quotebar_3m_atr_5_current_values.get_plot(&"atr".to_string()).unwrap().value;
                                    let last_heikin_3m_atr_5: Decimal = quotebar_3m_atr_5_last_values.get_plot(&"atr".to_string()).unwrap().value;

                                    // buy below the low of prior bar when atr is high and atr is increasing and the bars are closing higher, we are using a limit order which will cancel out at the end of the day
                                    if entry_order_id.is_none()
                                        && quotebar.bid_close > last_bar.bid_close
                                        && current_heikin_3m_atr_5 >= dec!(0.003)
                                        && current_heikin_3m_atr_5 > last_heikin_3m_atr_5
                                        && entry_order_id.is_none()
                                    {
                                        let limit_price = last_bar.ask_low;
                                        // we will set the time in force to Day, based on the strategy Tz of Australia::Sydney, I am not sure how this will work in live trading, TIF might be handled by manually sending cancel order on data server.
                                        let time_in_force = TimeInForce::Day;
                                        entry_order_id = Some(strategy.limit_order(&quotebar.symbol.name, None, &account, None, dec!(1000), OrderSide::Buy, limit_price, time_in_force, String::from("Enter Long Limit")).await);
                                        bars_since_entry = 0;
                                    }

                                    if entry_order_state != OrderState::Filled && entry_order_state != OrderState::PartiallyFilled {
                                        continue;
                                    }

                                    let position_size: Decimal = strategy.position_size(&account, &quotebar.symbol.name);

                                    // take profit conditions
                                    if let Some(_entry_order) = &entry_order_id {
                                        let in_profit = strategy.in_profit(&account, &quotebar.symbol.name);
                                        if bars_since_entry > 5
                                            && in_profit
                                        {
                                            let _exit_order_id: OrderId = strategy.exit_long(&quotebar.symbol.name, None, &account, None, position_size, String::from("Exit Take Profit")).await;
                                            bars_since_entry = 0;
                                            entry_order_id = None;
                                            entry_order_state = OrderState::Cancelled;
                                        }

                                    //stop loss conditions
                                        let in_drawdown = strategy.in_drawdown(&account, &quotebar.symbol.name);
                                        if bars_since_entry >= 10
                                            && in_drawdown
                                        {
                                            let _exit_order_id: OrderId = strategy.exit_long(&quotebar.symbol.name, None, &account, None, position_size, String::from("Exit Long Stop Loss")).await;
                                            bars_since_entry = 0;
                                            entry_order_id = None;
                                            entry_order_state = OrderState::Cancelled;
                                        }

                                        // Add to our winners when atr is increasing and we get a new signal
                                        let in_profit = strategy.in_profit(&account, &quotebar.symbol.name);
                                        let position_size: Decimal = strategy.position_size(&account, &quotebar.symbol.name);
                                        if  in_profit
                                            && position_size < dec!(3000)
                                            && bars_since_entry == 3
                                            && current_heikin_3m_atr_5 >= last_heikin_3m_atr_5
                                        {
                                            entry_order_id = Some(strategy.enter_long(&quotebar.symbol.name, None, &account, None, dec!(1000), String::from("Add Long")).await);
                                        }
                                    }
                                }
                            }
                            //do something with the current open bar
                            if !quotebar.is_closed {
                                //println!("Open bar closing time: {}", quotebar.time_closed())
                            }
                        }
                        BaseDataEnum::Tick(_tick) => {}
                        BaseDataEnum::Quote(_quote) => {
                            //primary data feed won't show up in event loop unless specifically subscribed by the strategy
                          /*  let msg = format!("{} Quote: bid: {} ,ask {}, Local Time {}", quote.symbol.name, quote.bid, quote.ask, quote.time_local(strategy.time_zone()));
                            println!("{}", msg.as_str().purple());*/
                        }
                        BaseDataEnum::Candle(_candle) => {}
                        BaseDataEnum::Fundamental(_fundamental) => {}
                    }
                }
            }

            // order updates are received here, excluding order creation events, the event loop here starts with an OrderEvent::Accepted event and ends with the last fill, rejection or cancellation events.
            StrategyEvent::OrderEvents(event) => {
                let msg = format!("{}, Strategy: Order Event: {}", strategy.time_utc(), event);

                match event {
                    OrderUpdateEvent::OrderRejected { .. } | OrderUpdateEvent::OrderUpdateRejected { .. } => println!("{}", msg.as_str().on_bright_magenta().on_bright_red()),
                    _ =>  println!("{}", msg.as_str().bright_yellow())
                }

                // See if our limit order has changed state, we could match each individual event here and handle manually. or we can just use the assumed change based on the event enum.
                if let Some(entry_order_id_2) = &entry_order_id {
                    if event.order_id() == entry_order_id_2 {
                        if let Some(state_change) = event.state_change() {
                            entry_order_state = state_change
                        }
                    }
                }
            }

            // if an external source adds or removes a data subscription it will show up here, this is useful for SemiAutomated mode
            StrategyEvent::DataSubscriptionEvent(event) => {
                    let msg = format!("Strategy: Data Subscription Event: {}", event);
                    println!("{}", msg.as_str().bright_magenta());
            }

            // strategy controls are received here, this is useful for SemiAutomated mode. we could close all positions on a pause of the strategy, or custom handle other user inputs.
            StrategyEvent::StrategyControls(control) => {
                match control {
                    StrategyControls::Continue => {}
                    StrategyControls::Pause => {}
                    StrategyControls::Stop => {}
                    StrategyControls::Start => {}
                    StrategyControls::Delay(_) => {}
                    StrategyControls::Custom(_) => {}
                    StrategyControls::CustomBytes(_, _) => {}
                }
            }

            StrategyEvent::ShutdownEvent(event) => {
                strategy.flatten_all_for(account).await;
                let msg = format!("{}",event);
                println!("{}", msg.as_str().bright_magenta());
                //we should handle shutdown gracefully by first ending the strategy loop.
                break 'strategy_loop
            },

            StrategyEvent::WarmUpComplete => {
                let msg = String::from("Strategy: Warmup Complete");
                println!("{}", msg.as_str().bright_magenta());
                warmup_complete = true;
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

            StrategyEvent::PositionEvents(event) => {
                let msg = format!("{}", event);
                println!("{}", msg.as_str().yellow());
                match event {
                    PositionUpdateEvent::PositionOpened { .. } => {}
                    PositionUpdateEvent::Increased { .. } => {}
                    PositionUpdateEvent::PositionReduced { .. } => strategy.print_ledger(&event.account()).await,
                    PositionUpdateEvent::PositionClosed { .. } => strategy.print_ledger(event.account()).await,
                }
            }
            StrategyEvent::TimedEvent(name) => {
                println!("{} has triggered", name);
            }
        }
    }
    strategy.export_trades(&String::from("./trades exports"));
    strategy.print_ledgers().await;
    event_receiver.close();
    println!("Strategy: Event Loop Ended");
}

// We can subscribe to new indicators at run time
// We can use a strategy reference for strategy functions.
pub async fn subscribe_to_my_atr_example(strategy: &FundForgeStrategy) {
    let msg = format!(
        "{time} Warming Up New heikin_atr10_15min
    • Process: Fetching historical data for warm-up
    • Duration: May take longer if insufficient history available in memory
    • Caution: Potential freeze in current dev state
    • Action: Restart if initialization exceeds 1 minute",
        time = strategy.time_local()
    );
    println!("{}",msg.as_str().purple());
    // this will test both our auto warm up for indicators and data subscriptions
    let quote_bar_atr10_15min = IndicatorEnum::AverageTrueRange(
        AverageTrueRange::new(
            IndicatorName::from("quote_bar_atr10_15min"),
            DataSubscription::new(
                SymbolName::from("EUR-USD"),
                DataVendor::Test,
                Resolution::Minutes(15),
                BaseDataType::QuoteBars,
                MarketType::Forex,
            ),
            5,
            10,
            Color::new(255, 165, 0),
            true
        ).await,
    );
    // we auto subscribe to the subscription, this will warm up the data subscription, which the indicator will then use to warm up.
    // the indicator would still warm up if this was false, but if we  don't have the data subscription already subscribed the strategy will deliberately panic
    strategy.subscribe_indicator(quote_bar_atr10_15min, true).await;
}


// We can subscribe to new data feeds at run time
// We can use a strategy reference for strategy functions.
pub async fn subscribe_to_new_candles_example(strategy: &FundForgeStrategy) {
    let msg = format!(
        "[{}] Warming Up New Subscription Update: AUD-CAD HeikinAshi Candle
    Resolution: 15 Minutes
    Memory: 48 bars
    Caution: Potential freeze in current dev state
    Note: If loading takes >1 min, consider restarting the engine.",
        strategy.time_local(),
    );
    println!("{}",msg.as_str().to_uppercase().purple());

    let minute_15_ha_candles = DataSubscription::new_custom(
        SymbolName::from("AUD-CAD"),
        DataVendor::Test,
        Resolution::Minutes(15),
        MarketType::Forex,
        CandleType::HeikinAshi
    );

    // In live we start a background task for this (untested)
     strategy.subscribe(minute_15_ha_candles, 48, false).await;
}