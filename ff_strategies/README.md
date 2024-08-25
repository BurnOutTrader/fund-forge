## Launching a strategy
####`FundForgeStrategy::Initialize()`
##### `owner_id: Option<OwnerId>:` 
The unique identifier for the owner of the strategy. If None, a unique identifier will be generated based on the executable's name.

##### `notify: Arc<Notify>:` 
The notification mechanism for the strategy, this is useful to slow the message sender channel until we have processed the last message.

##### `strategy_mode: StrategyMode:` 
The mode of the strategy (Backtest, Live, LivePaperTrading).

##### `interaction_mode: StrategyInteractionMode:` 
The interaction mode for the strategy.

##### `start_date: NaiveDateTime:` 
The start date of the strategy.

##### `end_date: NaiveDateTime:` 
The end date of the strategy.

##### `time_zone: Tz:` 
The time zone of the strategy, you can use Utc for default.

##### `warmup_duration: Duration:` 
The warmup duration for the strategy. used if we need to warmup consolidators, indicators etc.
We might also need a certain amount of history to be available before starting, this will ensure that it is.

##### `subscriptions: Vec<DataSubscription>:` 
The initial data subscriptions for the strategy.
strategy_event_sender: mpsc::Sender<EventTimeSlice>: The sender for strategy events.
If you subscriptions are empty, you will need to add some at the start of your `fn on_data_received()` function.

#### `replay_delay_ms: Option<u64>:` 
The delay in milliseconds between time slices for market replay style backtesting. this will be ignored in live trading.

#### `retain_history: usize:` 
The number of bars to retain in memory for the strategy. This is useful for strategies that need to reference previous bars for calculations, this is only for our initial subscriptions.
any additional subscriptions added later will be able to specify their own history requirements.

##### `buffering_resolution: Option<Duration>:`
If None then it will default to a 1-second buffer.
The buffering resolution of the strategy. If we are backtesting, any data of a lower granularity will be consolidated into a single time slice.
If our base data source is tick data, but we are trading only on 15min bars, then we can just set this to any Duration < 15 minutes and consolidate the tick data to ignore it in on_data_received().
In live trading our strategy will capture the tick stream in a buffer and pass it to the strategy in the correct resolution/durations, this helps to prevent spamming our on_data_received() fn.
In live: If we don't need to make strategy decisions on every tick, we can just consolidate the tick stream into buffered time slice events of a higher than instant resolution.
This also helps us get consistent results between backtesting and live trading and also reduces cpu load from constantly sending messages to our `fn on_data_received()`.


### Running the strategy and receiving events
We first need to run `initialize_clients(&PlatformMode::SingleMachine).await.unwrap();` in main to initialize the clients.
If we are using a multi-machine setup, we will need to use `initialize_clients(&PlatformMode::MultiMachine).await.unwrap();` instead.

Then we simply Initialize the strategy using the parameters above and pass it to our `fn on_data_received()` function.
The engine will automatically be created and started in the background, and we will receive events in our `fn on_data_received()` function.

We can divert strategy events to different functions if we want to separate the logic, some tasks are less critical than others. 
We can use  `notify.notify_one();` to slow the message sender channel until we have processed the last message.

```rust
fn set_subscriptions_initial() -> Vec<DataSubscription> {
    let subscriptions: Vec<DataSubscription> = vec![
        DataSubscription::new("AUD-CAD".to_string(), DataVendor::Test, Resolution::Ticks(1), BaseDataType::Ticks, MarketType::Forex),
        DataSubscription::new("AUD-USD".to_string(), DataVendor::Test, Resolution::Ticks(1), BaseDataType::Ticks, MarketType::Forex),
        DataSubscription::new("AUD-CAD".to_string(), DataVendor::Test, Resolution::Minutes(15), BaseDataType::Candles, MarketType::Forex)
    ];
    subscriptions
}

#[tokio::main]
async fn main() {
    initialize_clients(&PlatformMode::SingleMachine).await.unwrap();
    let (strategy_event_sender, strategy_event_receiver) = mpsc::channel(1000);
    let notify = Arc::new(Notify::new());
    // we initialize our strategy as a new strategy, meaning we are not loading drawing tools or existing data from previous runs.
    let strategy = FundForgeStrategy::initialize(
        Some(String::from("test")), //if none is passed in an id will be generated based on the executing program name, todo! this needs to be upgraded in the future to be more reliable in Single and Multi machine modes.
        notify.clone(),
        StrategyMode::Backtest, // Backtest, Live, LivePaper
        StrategyInteractionMode::SemiAutomated,  // In semi-automated the strategy can interact with the user drawing tools and the user can change data subscriptions, in automated they cannot. // the base currency of the strategy
        NaiveDate::from_ymd_opt(2023, 03, 20).unwrap().and_hms_opt(0, 0, 0).unwrap(), // Starting date of the backtest is a NaiveDateTime not NaiveDate
        NaiveDate::from_ymd_opt(2023, 03, 30).unwrap().and_hms_opt(0, 0, 0).unwrap(), // Ending date of the backtest is a NaiveDateTime not NaiveDate
        Australia::Sydney, // the strategy time zone
        Duration::days(3), // the warmup duration, the duration of historical data we will pump through the strategy to warm up indicators etc before the strategy starts executing.
        set_subscriptions_initial(), //the closure or function used to set the subscriptions for the strategy. this allows us to have multiple subscription methods for more complex strategies
        strategy_event_sender, // the sender for the strategy events
        None,
        100,

        //strategy resolution, all data at a lower resolution will be consolidated to this resolution, if using tick data, you will want to set this at 1 second or less depending on the data granularity
        //this allows us full control over how the strategy buffers data and how it processes data, in live trading .
        Some(Duration::seconds(1))
    ).await;

    on_data_received(strategy, notify, strategy_event_receiver).await;
}

pub async fn on_data_received(strategy: FundForgeStrategy, notify: Arc<Notify>, mut event_receiver: mpsc::Receiver<EventTimeSlice>)  {
    let mut warmup_complete = false;
    'strategy_loop: while let Some(event_slice) = event_receiver.recv().await {
        for strategy_event in event_slice {
            match strategy_event {
                // when a drawing tool is added from some external source the event will also show up here (the tool itself will be added to the strategy.drawing_objects HashMap behind the scenes)
                StrategyEvent::DrawingToolEvents(_, drawing_tool_event, _) => {
                    // The engine is being designed to allow for extremely high levels of user interaction with strategies, 
                    // where strategies can be written to interact with the users analysis through drawing tools.
                }
                StrategyEvent::TimeSlice(_time, time_slice) => {
                    'base_data_loop: for base_data in &time_slice {
                        if !warmup_complete {
                            continue 'strategy_loop;
                        }
                        match base_data {
                            BaseDataEnum::Price(_) => {}
                            BaseDataEnum::Candle(ref candle) => {}
                            BaseDataEnum::QuoteBar(_) => {}
                            BaseDataEnum::Tick(tick) => {}
                            BaseDataEnum::Quote(_) => {}
                            BaseDataEnum::Fundamental(_) => {}
                        }
                    }
                }
                StrategyEvent::OrderEvents(_, event) => {
                    // order updates are received here, excluding order creation events, the event loop here starts with an OrderEvent::Accepted event and ends with the last fill, rejection or cancellation events.
                }
                StrategyEvent::DataSubscriptionEvents(_, events, _) => {
                    // if an external source adds or removes a data subscription it will show up here, this is useful for SemiAutomated mode
                }
                StrategyEvent::StrategyControls(_, _, _) => {
                    // strategy controls are received here, this is useful for SemiAutomated mode. we could close all positions on a pause of the strategy, or custom handle other user inputs.
                }
                StrategyEvent::ShutdownEvent(_, _) => break 'strategy_loop, //we should handle shutdown gracefully by first ending the strategy loop.
                StrategyEvent::WarmUpComplete(_) => {
                    warmup_complete = true;
                }
                //todo add more event types 
            }
            notify.notify_one();
        }
    }
    event_receiver.close();
    println!("Strategy Event Loop Ended");
}
```


## Time
chrono_tz will automatically handle live and historical time zone conversions for us.
All serialized data should be saved in UTC time as a `DateTime<Utc>.to_string()`, and then converted to the strategy's time zone when needed.
there are converters for both local and utc time in ff_standard_lib/src/helpers.
If you know the time zone of your data, you must parse it as UTC for serialization!
The engine is designed to handle all serialized data as UTC, and then convert it to the strategy's time zone when needed.
```rust
use chrono_tz::Australia;

pub async fn on_data_received(strategy: FundForgeStrategy, notify: Arc<Notify>, mut event_receiver: mpsc::Receiver<EventTimeSlice>) {
    'strategy_loop: while let Some(event_slice) = event_receiver.recv().await {
        // time_local() will return the current time in the strategy's time zone as DateTime<FixedOffset>
        println!("{}... time local {}", count, strategy.time_local().await);
        
        // time_utc() will return the current time in UTC as DateTime<Utc>
        println!("{}... time utc {}", count, strategy.time_utc().await);
        
        let data = Candle::default();
        // The data time property is a string which has to do with 0 copy serde.
        // to access data time we use a fn.
        let time_zone = Australia::Sydney;
        let candle_time_string = data.time.clone();
        let candle_time_utc = candle.time_utc();
        let candle_time_local = candle.time_local(strategy.time_zone());
        let candle_time_sydney = candle.time_local(time_zone);
        let strategy_time_local = strategy.time_local();
        let strategy_time_utc = strategy.time_utc();

        notify.notify_one();
    }
}
```

## Subscriptions
```rust
pub async fn on_data_received(strategy: FundForgeStrategy, notify: Arc<Notify>, mut event_receiver: mpsc::Receiver<EventTimeSlice>) {

    // subscribing to multiple items while unsubscribing from existing items
    // if our strategy has already warmed up, the subscription will automatically have warm up to the maximum number of bars and have history available.
    let aud_cad_60m = DataSubscription::new_custom("AUD-CAD".to_string(), DataVendor::Test, Resolution::Minutes(60), BaseDataType::Candles, MarketType::Forex, CandleType::HeikinAshi);
    let aud_usd_15m = DataSubscription::new("AUD-USD".to_string(), DataVendor::Test, Resolution::Minutes(15), BaseDataType::Candles, MarketType::Forex);

    // Note that this function completely overrides our current subcsriptions, If we have any current subscriptions they will be unsubscribed if not also passed in.
    // Any existing subscriptions which are not primary subscriptions (tick stream etc) will not be unsubscribed from.
    // Any primary subscription, which is being used to consolidate data which is not being unsubscribed will not be unsubscribed.
    // The second parameter is the number of bars to retain in memory for the strategy.
    // The engine will automatically consolidate the data to the required resolution and will try to maintain only a single primary subscription per symbol to minimise data vendor api usage.
    strategy.subscriptions_update(vec![aud_usd_15m.clone(), aud_cad_60m.clone()], 100).await;

    //or we can subscribe to a single item and not effect any existing subscriptions
    // The second parameter is the number of bars to retain in memory for the strategy.
    strategy.subscribe(aud_usd_15m.clone(), 100).await;

    //or we can unsubscribe from a single item
    strategy.unsubscribe(&aud_usd_15m.symbol).await;

    //we can see our subscriptions
    let subscriptions = strategy.subscriptions().await;
    println!("subscriptions: {:?}", subscriptions);

    // we can also access the subscription for BaseDataEnums 
    // base_data.subscription() which returns a DataSubscription object
    // all objects wrapped in a BaseDataEnum will have a subscription() fn.
    
    'strategy_loop: while let Some(event_slice) = event_receiver.recv().await {
        // we can subscribe in the event loop with no problems, the engine can handle this in live and backtest without skipping data.
        notify.notify_one();
    }
}
```

## Retained History
The consolidators will retain history when specified during subscription.
If we want to have the engine keep a history automatically, we will need a reference to the subscription to access it.
```rust
pub async fn on_data_received(strategy: FundForgeStrategy, notify: Arc<Notify>, mut event_receiver: mpsc::Receiver<EventTimeSlice>) {
    
    // if our strategy has already warmed up, the subscription will automatically have warm up to the maximum number of bars and have history available.
    let aud_cad_60m = DataSubscription::new_custom("AUD-CAD".to_string(), DataVendor::Test, Resolution::Minutes(60), BaseDataType::Candles, MarketType::Forex, CandleType::HeikinAshi);

    // this will return a RollingWindow<BaseData> for the subscription by cloning the history.
    // at the current point this clones the whole rolling window, and so is not suitable for frequent use of large history.
    // An alternative would be to get the history once, after initializing the indicator, so we have a warmed up history, then keep the history in a separate variable and add the new data to it.
    history: &RollingWindow<BaseDataEnum>  = strategy.history(&aud_usd_15m).await;

    // if we are keeping a large history and need to access it often, it could be better to manually keep the history we need to avoid clone()ing the whole history on every iter.
    // we could set the history_to_retain variable to some small number and keep the larger history in a separate variable.
    let rolling_window: RollingWindow<BaseDataEnum> = RollingWindow::new(100);
    for data in history {
        rolling_window.add(data);
    }
    
    'strategy_loop: while let Some(event_slice) = event_receiver.recv().await {
        // this will give us the closed bar, 2 bars ago
        let two_bars_ago = &strategy.bar_index(&aud_usd_15m, 2).await;
        println!("{}...{} Three bars ago: {:?}", count, aud_cad_60m.symbol.name, three_bars_ago);
        
        // this will give us the current open bar
        let data_current = &strategy.data_current(&aud_cad_60m).await;
        println!("{}...{} Current data: {:?}, {}", count, aud_cad_60m.symbol.name, data_current.is_closed);
        
        //The data points can be accessed by index. where 0 is the latest data point.
        let last_data_point = rolling_window.get(0);
        
        notify.notify_one();
    }
}
```


## Handling BaseDataEnum
```rust
pub async fn on_data_received(strategy: FundForgeStrategy, notify: Arc<Notify>, mut event_receiver: mpsc::Receiver<EventTimeSlice>) {
    'strategy_loop: while let Some(event_slice) = event_receiver.recv().await {
        for strategy_event in event_slice {
            match strategy_event {
                StrategyEvent::TimeSlice(_time, time_slice) => {
                    'base_data_loop: for base_data in &time_slice {
                        match base_data {
                            BaseDataEnum::Price(price) => {
                                println!("{}...{} Price: {:?}", count, price.symbol.name, price.close);
                            }
                            BaseDataEnum::Candle(candle) => {
                                println!("{}...{} Candle: {:?}", count, candle.symbol.name, candle.close);
                            }
                            BaseDataEnum::QuoteBar(quote_bar) => {
                                // quote bars contain bid and ask data
                                println!("{}...{} QuoteBar: {:?}, {:?}", count, quote_bar.symbol.name, quote_bar.bid_close, bar.ask_close);
                            }
                            BaseDataEnum::Tick(tick) => {
                                println!("{}...{} Tick: {:?}", count, tick.symbol.name, tick.price);
                            }
                            BaseDataEnum::Quote(quote) => {
                                println!("{}...{} Quote: {:?}", count, quote.symbol.name, quote.bid);
                            }
                            BaseDataEnum::Fundamental(fundamental) => {
                                println!("{}...{} Fundamental: {:?}", count, fundamental.symbol.name, fundamental.price);
                                // fundamental data can vary wildly, i have built in the ability to add custom data to the fundamental struct.
                                // we can use rkyv to parse from bytes if we know the type, we can determine the type using fundamental.name
                                // or we can use fundamental variant to hold strings, like json or csv data.
                            }
                        }
                    }
                }
            }
        }
        notify.notify_one();
    }
}
```

## Indicators
Indicators can be handled automatically by the strategy Indicator handler, or we can create and manage them manually in the `on_data_received()` function.
We can implement the `Indicators trait` for our custom indicators.
If building a custom indicator be sure to add it to the IndicatorEnum and complete the matching statements, so that the Indicator handler can handle it if needed.

### Indicator Values
```rust
///indicators return `IndicatorValues`, the values have the normal fund forge functions for time_utc() and time_local(Tz)
pub struct IndicatorValues {
    time: String,
    pub indicator_name: IndicatorName,
    pub subscription: DataSubscription,
    values: AHashMap<PlotName, f64>
}
impl IndicatorValues {
    pub fn time_utc(&self) -> DateTime<Utc> {
        DateTime::from_str(&self.time).unwrap()
    }

    pub fn time_local(&self, time_zone: &Tz) -> DateTime<FixedOffset> {
        time_convert_utc_datetime_to_fixed_offset(time_zone, self.time_utc())
    }

    /// get the value of a plot by name
    pub fn get_plot(&self, plot_name: &str) -> Option<f64> {
        self.values.get(plot_name).cloned()
    }
}
```

### Using Indicators
```rust
pub async fn on_data_received(strategy: FundForgeStrategy, notify: Arc<Notify>, mut event_receiver: mpsc::Receiver<EventTimeSlice>) {
    
    // Subscribe to a 60-minute candle for the AUD-CAD pair
    let aud_cad_60m = DataSubscription::new_custom("AUD-CAD".to_string(), DataVendor::Test, Resolution::Minutes(60), BaseDataType::Candles, MarketType::Forex, CandleType::HeikinAshi);
    strategy.subscriptions_update(vec![aud_cad_60m.clone()],100).await;
    
    // Create a manually managed indicator directly in the on_data_received function (14 period ATR, which retains 100 historical IndicatorValues)
    let mut heikin_atr = AverageTrueRange::new(aud_cad_60m.clone(), 100, 14).await;
    let mut heikin_atr_history: RollingWindow<IndicatorValues> = RollingWindow::new(100);
    
    'strategy_loop: while let Some(event_slice) = event_receiver.recv().await {
        for strategy_event in event_slice {
            match strategy_event {
                StrategyEvent::TimeSlice(_time, time_slice) => {
                    'base_data_loop: for base_data in &time_slice {
                        match base_data {
                            BaseDataEnum::Candle(candle) => {
                                // lets update the indicator with the new candles
                                if candle.is_closed {
                                    heikin_atr.update_base_data(candle).await;
                                }
                                
                                // lets get the indicator value for the current candle, note for atr we can use current, as it only updates on closed candles.
                                if heikin_atr.is_ready() {
                                    let atr = heikin_atr.current();
                                    println!("{}...{} ATR: {}", strategy.time_utc().await, aud_cad_60m.symbol.name, atr.unwrap());
                                    heikin_atr_history.add(heikin_atr.current());
                               
                                    // we can also get the value at a specific index, current bar (closed) is index 0, 1 bar ago is index 1 etc.
                                    let atr = heikin_atr.index(2);
                                    println!("{}...{} ATR 2 bars ago: {}", strategy.time_utc().await, aud_cad_60m.symbol.name, atr.unwrap());
                                    
                                    //or we can use our own history to get the value at a specific index
                                    let atr = heikin_atr_history.get(10);
                                    println!("{}...{} ATR 10 bars ago: {}", strategy.time_utc().await, aud_cad_60m.symbol.name, atr.unwrap());
                                }
                            },
                            _ => {}
                        }
                    }
                }
            }
        }
        notify.notify_one();
    }
}
```