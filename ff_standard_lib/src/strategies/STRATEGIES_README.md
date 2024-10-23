# Strategies
## Glossary 
- [Initializing Strategies](#initializing-strategies)
- [Running Strategies](#running-strategies)
- [Time](#time)
- [Accessing Retained History](#retained-history)
- [Subscriptions](#subscriptions)
- [Alternative On Data Function](#alternative-to-iterating-the-buffer)
- [Alternative TimeSlice Handling](#alternative-to-iterating-a-timeslice)
- [BaseDataEnum](#basedataenum) 
- [Indicators](#indicators) or [Full indicators readme](indicators/INDICATORS_README.md)
- [Accounts](#accounts)
- [Timed Events](#timed-events)
- [Requesting History](#history-requests)
- [Drawing Tools](#drawing-tools)
- [Order Books](#order-books-)
- [Estimate Fills Before Placing an Order](#estimate-fill-price)
- [Placing Orders](#placing-orders)
- [Debugging Strategies](#debugging-strategies)

## Important Info
The test strategies might appear to be frozen before warm up,
this is because we are sorting a large amount of quote data into accurate time slices for 2 symbols.
Sometimes pre warm up, it will freeze for longer than 1 min, I am not sure why this is yet, It happens rarely.
It is likely to do with running server locally and stopping and starting strategies during development.
If the strategy doesn't start the data feed and is stuck for more than 1 min just after retrieving data, restart it.
The feed should start 1 or 2 seconds after the engine print line `2024-06-17 Data Points Recovered from Server: 3029551 for 2024-06-17";`.

It is normal for a long pause after this line `Engine: Preparing TimeSlices For: June 2024` when the engine sorts the data from multiple symbols into a single feed.

If the engine is frozen after this line `2024-06-17 Data Points Recovered from Server: 3029551 for 2024-06-17` It is likely frozen indefinitely.

## Setup Info
Get the test data from the instructions provided in the main readme and complete the setup.
To run a strategy.
1. cargo build in the fund-forge directory
2. complete setup from main directory by downloading the test data.
3. In the ff_data_server folder open a terminal and `cargo run`
4. In the test_strategy folder open a terminal and `cargo run`, or run directly in IDE
5. The initial strategy start up will take time, as we recover historical data from our local server instance and (more demandingly) sort the individual quote resolution symbol data into timeslices for perfect accuracy.
   The downloading and sorting of data into time slices is concurrent, but since the test data consists of 3318839 data points per month (2 symbols) it can take some time initially.
   I have tested running the data server remotely, it only adds a few seconds to backtests even at low data resolutions, this means we will be able to have our data server running on a server and keep a permanent copy of historical data in the cloud, while still back testing locally.

Everything found here could be changed during development, you will have to consult your IDE for minor errors like changes to function inputs.

See the [Test strategy](https://github.com/BurnOutTrader/fund-forge/blob/main/test_strategy/src/main.rs) for the most up-to-date working strategy example.

## Initializing Strategies
- Strategies are launched by creating a new instance of the `FundForgeStrategy` struct using the `initialize()` function,
this will automatically create the engine and start the strategy in the background.
- Then we can receive `StrategyEventBuffer`s in our `fn on_data_received()` function.
- The strategy object returned from `initialize()` is a fully owned object, and we can pass it to other function, wrap it in an arc etc. 

It is best practice to use the strategies methods to interact with the strategy rather than calling on any private fields directly.
strategy methods only need a reference to the strategy object, and will handle all the necessary locking and thread safety for you.
It is possible to wrap the strategy in Arc if you need to pass it to multiple threads, all functionality will remain. 

The strategy object is an owned object, however it does not need to be owned or mutable to access strategy methods, all methods can be called with only a reference to the strategy object, this
allows us to pass our strategy in an Arc to any other threads or functions and still utilise its full functionality, the strategy is protected from misuse by using interior mutability.

***Use the test strategy for actual testing, these examples will be partially outdated! But they are a good reference for helpful strategy functions***

### Initializing and Creating a Strategy Instance
#### Parameters for FundForgeStrategy::initialize()
#### `strategy_mode: StrategyMode:`
The mode of the strategy (Backtest, Live, LivePaperTrading).

#### `backtest_accounts_starting_cash: Decimal`:
Only used for backtest and live paper trading to initialize paper accounts
This is per account.

#### `backtest_account_currency: Currency`:
Only used for backtest and live paper trading to initialize paper accounts
For all accounts (currently no way to have unique currency per paper account)

#### `start_date: NaiveDateTime:`
The start date of the strategy.

#### `end_date: NaiveDateTime:`
The end date of the strategy.

#### `time_zone: Tz:`
The time zone of the strategy, you can use Utc for default.

#### `warmup_duration: Duration:`
The warmup duration for the strategy. used if we need to warmup consolidators, indicators etc.
We might also need a certain amount of history to be available before starting, this will ensure that it is.

#### `subscriptions: Vec<DataSubscription>:`
The initial data subscriptions for the strategy.
If your subscriptions are empty, you will need to add some at the start of your `fn on_data_received()`.

##### In Backtest mode 
The engine and server will use consolidators to consolidate historical data from a low resolution. \
This will depend on what historical data we are serializing, currently the resolutions available are hard coded, \
in the future there will be a toml for configuring which resolutions the server should save and make available for backtesting. 

##### In Live or Live paper 
The engine will use Quote data as priority feed for quote bars. 

The engine will try to determine the most suitable resolution.

If you subscribed to 15 seconds Candles it will prioritise using candles, unless you have already subscribed to ticks. \
If you choose fill forward it will always choose to subscribe to ticks and to consolidate the bars itself. 

If the data vendor has live data for the resolution, and you have not already subscribed to a preferred resolution like ticks or lower resolution candles, 
then the engine will subscribe directly from the data vendor, the implications of this will be that you will never have access to the open bar prices. 

If you need Open bar prices, then you should use fill forward, or first subscribe to either Ticks, Quotes or The lowest resolution candles the vendor has, this choice will depend on the vendor and data type. \
The logic can be seen here:
```rust
//This is the logic the engine uses to determine the best resolution for candles, and if we need to consolidate or subscribe directly from the DataVendor
let has_candles = self.vendor_primary_resolutions.contains(&SubscriptionResolutionType::new(Resolution::Seconds(1), BaseDataType::Candles));
let has_ticks = self.vendor_primary_resolutions.contains(&SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks));
//determine the prefered resolution for the subscription
if has_ticks && has_candles {
    if fill_forward {
        SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks)
    } else {
        SubscriptionResolutionType::new(Resolution::Seconds(1), BaseDataType::Candles)
    }
} else {
    if has_candles {
        match self.primary_subscriptions.contains_key(&SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks)) {
            true => SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks),
            false => SubscriptionResolutionType::new(Resolution::Seconds(1), BaseDataType::Candles)
        }
    } else if has_ticks {
        SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks)
    } else {
        SubscriptionResolutionType::new(Resolution::Instant, BaseDataType::Quotes)
    }
}

if self.vendor_primary_resolutions.contains(&sub_res_type) && !self.primary_subscriptions.contains_key(&ideal_subscription) {
    //if these conditions are true we will subscribe directly from the data vendor
}
```

#### `strategy_event_sender: mpsc::Sender<EventTimeSlice>:` 
The sender for strategy events, the send half of the mpsc::channel we will use to receive the `StrategyEventBuffers`

#### `fill_forward`: bool
This is only regarding initial subscriptions, additional subscriptions will have to specify the option.
If true we will create new bars based on the time when there is no new primary data available, this can result in bars where ohlc price are all == to the last bars close price.
Bars filling forward without data normally look like this: "_" where there was not price action. They could also open and then receive a price update sometime during the resolution period.
With fill forward enabled, during market close you will receive a series of bars resembling _ _ _ _ _ instead of no bars at all.
You should consider that some indicators like ATR might see these bars and drop the ATR to 0 during these periods.
If this is false, you will see periods of no data in backtests when the market is closed, as the engine ticks at buffering_millis through the close hours, until new  data is received.

fill forward is best used on very low resolutions, like seconds. 

If fill forward is enabled on a candle feed, the engine will prioritise a tick feed and consolidate the candles.

If fall forward == false, the engine will prioritise a 1-second candle feed if it is available.

If using very low resolutions <= 15 seconds, it is better to use QuoteBars, Quotes have many more updates than Ticks and you will get cleaner bars.
QuoteBars will always update from Quote Feeds, this is a very expensive feed, it is better to use candles if you do not need the low resolutions of a Quote feed.

#### `retain_history: u64:`
The number of bars to retain in memory for the strategy. This is useful for strategies that need to reference previous bars for calculations, this is only for our initial subscriptions.
any additional subscriptions added later will be able to specify their own history requirements.

#### `buffering_duration: Option<core::time::Duration>` 
core::time::Duration::from_millis(100),
The historical engine or server will buffer data streams at this resolution.
This helps us get consistent results between back testing and live trading and also reduces cpu load from constantly sending messages to our `fn on_data_received()`.

#### `gui_enabled: bool` (Do not set to true, in development)
This enables the ff_strategy_registry connection to connect to our gui, if false we will not broadcast events to the registry and will be invisible to the gui.

#### `tick_over_no_data: bool`
If true the historical engine will tick at buffer duration speed when there is no historical data available.
This allows us to use timed events of fill forward over weekends, if no, the engine will skip days where no data was available and jump to the next time instantly.
This does nothing in live.

#### `synchronize_accounts: bool` 
If true strategy positions and open + booked pnl will update in sync with the brokerage, if false the engine will simulate positions using the same logic as backtesting.
With this option enabled you strategy will see positions opened by other strategies or external sources, and will be able to close them, or modify them.
For example: You could place an order in rithmic and a strategy will be able to manage the position.

With this disabled the strategy will assume that it is the only source of orders and positions, and will not be able to close or modify positions opened by other strategies or external sources.
For example, you could close a strategy position, and the strategy will still think the position is open.

#### `accounts: Vec<Account`
Example of initializing accounts.
```rust
let account_1 = Account::new(Brokerage::Test, "Test_Account_1".to_string());
let account_2 = Account::new(Brokerage::Test, "Test_Account_1".to_string());
let accounts = vec![account_1, account_2];
```

```rust
use std::time::Duration;

#[tokio::main]
async fn main() {
    // we create a channel for the receiving strategy events
    let (strategy_event_sender, strategy_event_receiver) = mpsc::channel(1000);

    let strategy = FundForgeStrategy::initialize(
        // Backtest, Live, LivePaper
        StrategyMode::Backtest,

        //starting cash per account
        dec!(100000.0),
        
        //backtest account currency
        Currency::USD,

        // Starting date of the backtest is a NaiveDateTime not NaiveDate
        NaiveDate::from_ymd_opt(2023, 03, 20).unwrap().and_hms_opt(0, 0, 0).unwrap(),

        // Ending date of the backtest is a NaiveDateTime not NaiveDate
        NaiveDate::from_ymd_opt(2023, 03, 30).unwrap().and_hms_opt(0, 0, 0).unwrap(),

        // the strategy time zone (Tz)
        Australia::Sydney,

        // the warmup duration, the duration of historical data we will pump through the strategy to warm up indicators etc before the strategy starts executing.
        Duration::days(3),

        // the initial data subscriptions for the strategy. we can also subscribe or unsubscribe at run time.
        vec![
            DataSubscription::new("AUD-CAD".to_string(), DataVendor::Test, Resolution::Ticks(10), BaseDataType::Candles, MarketType::Forex),
            DataSubscription::new("AUD-USD".to_string(), DataVendor::Test, Resolution::Instant, BaseDataType::Quotes, MarketType::Forex),
            // we can subscribe to fundamental data and alternative data sources (no fundamental test data available yet)
            DataSubscription::new_fundamental("GDP-USA".to_string(), DataVendor::Test)
            //if using new() default candle type is CandleStick
            DataSubscription::new("AUD-CAD".to_string(), DataVendor::Test, Resolution::Minutes(15), BaseDataType::Candles, MarketType::Forex)
            // we can also specify candle types like HeikinAshi, Renko, CandleStick (more to come). 
            DataSubscription::new_custom("AUD-USD".to_string(), DataVendor::Test, Resolution::Minutes(15), MarketType::Forex, CandleType::HeikinAshi)
        ],
        // Fill forward, when the market is closed or no primary data is available, consolidators will create bars based on the last close price. See parameters above
        true,
        //bars to retain in memory for the initial subscriptions
        100,

        // the sender for the strategy events
        strategy_event_sender,

        //if Some(buffer) we will use the buffered backtesting or buffered live trading engines / handlers.
        //If None we will use the unbuffered versions of backtest engine or handlers. The backtesting versions will try to simulate the event flow of their respective live handlers.
        //this allows us full control over how the strategy buffers data and how it processes data, in live trading.
        // In live trading we can set this to None to skip buffering and send the data directly to the strategy or we can use a buffer to keep live consistency with backtesting.
        Some(Duration::from_millis(100)),
        
        //Gui enabled, this needs to be false until strategy registry is overhauled.
        false,
        
        // tick_over_no_data, if true the historical engine will tick at buffer duration speed when there is no historical data available.
        // This allows us to use timed events of fill forward over weekends, if no, the engine will skip days where no data was available and jump to the next time instantly.
        // This does nothing in live.
        true,

        // The accounts we will be trading, there will also be a fn to initialize at run time.
        vec![Account::new(Brokerage::Test, "Test_Account_1".to_string()), Account::new(Brokerage::Test, "Test_Account_2".to_string())]
    ).await;

   // You don't have to pass in accounts, but if you want to customise accounts you can do it like this.
   // Any account you use in a backtest will be automatically created if you do not pass it in here.
   let mut account_info = AccountSetup {
      account_id: "123".to_string(),
      brokerage: Brokerage::Test,
      cash_value: dec!(100000),
      currency: Currency::USD,
      size_limit: None, //will be implemented but not currently relevant
      max_orders: None, //will be implemented but not currently relevant
      daily_max_loss: None, //will be implemented but not currently relevant
      daily_max_reset_time: None, //will be implemented but not currently relevant, will be the hour of the day 
      daily_max_loss_reset_time_zone: Tz, //will be implemented but not currently relevant, the time zone of the max loss reset hour.
      leverage: Some(1), //un-leveraged 1 to 1 == None == 1, rithmic and test brokerage does not use this regardless of input.
   };
   // pass in the account info to create an account for this product
   strategy.add_account(account_info);

    // We start receiving data in our on data fn
    on_data_received(strategy, strategy_event_receiver).await;
}
```

## Running Strategies
Simply Initialize the strategy using the parameters above and pass it to our `fn on_data_received()` function.
The engine will automatically be created and started in the background, and we will receive events in our `fn on_data_received()` function.

We can divert strategy events to different functions if we want to separate the logic, some tasks are less critical than others. 
We can use  `notify.notify_one();` to slow the message sender channel until we have processed the last message.

When we run the strategy we receive a `StrategyEventBuffer` in our receiver, the size of the buffer is determined by the Buffer Option<Duration>
When we `iter()` the buffer we receive the events in the exact order they were captured.
Similarly, when we `iter()` a `TimeSlice` we receive the `BaseDataEnum`'s in the exact order they were created.

```rust
pub async fn on_data_received(strategy: FundForgeStrategy, mut event_receiver: mpsc::Receiver<StrategyEventBuffer>)  {
    let mut warmup_complete = false;

    let account_1 = Account::new(Brokerage::Test, "Test_Account_1".to_string());
    // we can handle our events directly in the `strategy_loop` or we can divert them to other functions or threads.
    'strategy_loop: while let Some(event_slice) = event_receiver.recv().await {
        // when we iterate the buffer the events are returned in the exact order they occured, the time property is the time the event was captured in the buffer, not the current strategy time.
        for (time, strategy_event) in event_slice.iter() {
            match strategy_event {
                // when a drawing tool is added from some external source the event will also show up here (the tool itself will be added to the strategy.drawing_objects HashMap behind the scenes)
                StrategyEvent::DrawingToolEvents(_, drawing_tool_event, _) => {
                    // The engine is being designed to allow for extremely high levels of user interaction with strategies, 
                    // where strategies can be written to interact with the users analysis through drawing tools.
                }
                // only data we specifically subscribe to show up here, if the data is building from ticks but we didn't subscribe to ticks specifically, ticks won't show up but the subscribed resolution will.
                StrategyEvent::TimeSlice(_time, time_slice) => {
                    'base_data_loop: for base_data in time_slice.iter() {
                        if !warmup_complete {
                            continue 'strategy_loop;
                        }
                        match base_data {
                            BaseDataEnum::Candle(ref candle) => {}
                            BaseDataEnum::QuoteBar(ref quote_bar) => {}
                            BaseDataEnum::Tick(ref tick) => {}
                            BaseDataEnum::Quote(ref tick) => {}
                            BaseDataEnum::Fundamental(ref fundamental) => {}
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
                StrategyEvent::IndicatorEvent(_, _) => {

                }
                StrategyEvent::PositionEvents(event) => {
                    println!("{:?}", event);
                }
            }
            
        }
    }
    event_receiver.close();
    println!("Strategy Event Loop Ended");
}
```
## Alternative To Iterating The Buffer
We don't have to iterate the event objects in the order they were buffered, we can separate the objects based on their event type and receive back and iterator of those objects.
```rust
fn example() {
    pub enum StrategyEventType {
        OrderEvents,
        DataSubscriptionEvents,
        StrategyControls,
        DrawingToolEvents,
        TimeSlice,
        ShutdownEvent,
        WarmUpComplete,
        IndicatorEvent,
        PositionEvents,
    }
    let event_buffer: StrategyEventBuffer = StrategyEventBuffer::new();
    // Returns a Vec of events of a given type, (ref version)
    let order_events_borrowed: Iterator<Item=&(DateTime<Utc>, StrategyEvent)> = event_buffer.get_events_by_type(StrategyEventType::OrderEvents);
    // we could pass to a fn that only handles order events
    handle_order_events(order_events_borrowed);

    // Returns a Vec of events of a given type, sorted by time (owned version)
    // This fn does not remove the events but instead clones them.
    // If you are using this fn to move certain events to another function, be careful that you do not double handle events, ie react to the same event twice
    let order_events_owned: Vec<(DateTime<Utc>, StrategyEvent)> = event_buffer.get_owned_events_by_type(StrategyEventType::OrderEvents);

    for order_event in order_events {
        println!("{}", event);
    }
}
```

## Time
### When downloading and parsing data from a DataVendor for the engine
All data should be saved using the static `HybridStorage` object, the data server hosts a public static `DATA_STORAGE` object, this object acts as a data base tool for serializing and loading data.
Historical data loading will be handled automatically by the server, when you need to serialize data in a new API implementation, you should use the `DATA_STORAGE.save_data_bulk(data).await.unwrap()` function.
chrono_tz will automatically handle live and historical time zone conversions for us.
All serialized data should be saved in UTC time as a `DateTime<Utc>`, and then converted to the strategy's time zone when needed.
there are converters for both local and utc time in ff_standard_lib/src/helpers/converters.
1. You can convert to a `NaiveDateTime` from a specific `Tz` like `Australia::Sydney` and return a `DateTime<Tz>` object using the helper
2. You can call `time.to_utc()` to convert to a `DateTime<Utc>`, this will make adjustments to the actual date and hour of the original `DateTime<Tz>` to properly convert the time, not just change the Tz by name.

When working with `BaseDataEnum` types you must know the time zone of your data and you must parse it as `DateTime<Utc>.to_string()` for serialization!
The `time` property of all `BaseDataEnum Variants` is a String, this is for easier serialization and deserialization using rkyv.
This makes it very easy to work between foreign markets starting from a standardized `Tz` (Utc) for all time functions.
see https://docs.rs/chrono-tz/latest/chrono_tz/
### The engine is designed to handle all serialized data as UTC, and then convert it to the strategy's time zone when needed.
```rust
use chrono_tz::Tz;
use chrono_tz::Australia;
use chrono_tz::America;
pub async fn on_data_received(strategy: FundForgeStrategy, notify: Arc<Notify>, mut event_receiver: mpsc::Receiver<EventTimeSlice>) {
    'strategy_loop: while let Some(event_slice) = event_receiver.recv().await {
        // time_local() will return the current time in the strategy's time zone as DateTime<Tz>
        println!("{}... time local {}", count, strategy.time_local().await);
        
        // time_utc() will return the current time in UTC as DateTime<Utc>
        println!("{}... time utc {}", count, strategy.time_utc().await);
        
        let data = Candle::default();
        let time_string: String = data.time; // The data time property is a string which has to do with rkyv ser/de.
        
        // to access data time we use a fn.
        let candle_time_string: String = data.time.clone(); //the open time of the candle
        let candle_time_utc: DateTime<Utc> = candle.time_utc(); //the open time of the candle
        let candle_time_local: DateTime<Tz> = candle.time_local(strategy.time_zone()); //the open time of the candle

        //the close time of the candle (we can do the same with quote bars and the raw BaseDataEnum) 
        // if we call time closed on a enum variant other than Candle or QuoteBar it will just return the data time the same as if we called time_local()
        let candle_close_utc: DateTime<Utc> = candle.time_closed_utc(); //candle close time in utc time
        let candle_close_local: DateTime<Utc> = candle.time_closed_local(strategy.time_zone()); //close time of the candle in local time
        
        // using specific time zone other than strategy time zone or utc
        let time_zone = Australia::Sydney;
        let candle_time_sydney: DateTime<Tz> = candle.time_local(time_zone); //the open time of the candle
        let strategy_time_local: DateTime<Tz> = strategy.time_local(); 
        let strategy_time_utc: DateTime<Utc> = strategy.time_utc(); 

        /// Get back the strategy time with any passed in timezone
        let nyc_time_zone: Tz = America::New_York;
        let strategy_time_nyc: DateTime<Tz> = strategy.time_from_tz(nyc_time_zone);
    }
}
```

## Subscriptions
Subscriptions can be updated at any time, and the engine will handle the consolidation of data to the required resolution.

The engine will also warm up indicators and consolidators after the initial warm up cycle, this may result in a momentary pause in the strategy runtime during back tests, while the data is fetched, consolidated etc.
In live trading this will happen in the background as an async task, and the strategy will continue to execute as normal.

The SubscriptionHandler will automatically build data from the lowest suitable resolution.

The engine will automatically use any primary data available with the data vendor in historical mode, to speed up backtests and prevent using consolidators.

So we can have historical data in all resolutions to make backtests faster.

In live mode the engine will subscribe to the lowest possible resolution data for data feeds: tick and quote is priority or lastly the lowest resolution candles or quotebars.

this is done so that when live streaming with mutlple strategies we only need to maintain 1 live data feed per symbol, no matter the number of strategies and resolutions subscribed.
```rust
// we can access resolution as a Duration with resolution.as_duration() or resolution.as_seconds();
pub enum Resolution {
    Instant,
    Ticks(u64),
    Seconds(u64),
    Minutes(u64),
    Hours(u64),
}

// useful resolution functions
fn example(resolution: Resolution) {
    // Resolution::Instant will return 0 value not null.
    let duration: Duration = resolution.as_duration();
    let seconds: i64 = resolution.as_seconds();
    let nanos: i64 = resolution.as_nanos();
    let millis: i128 = resolution.as_millis();
    let number_of: u64 = resolution.number_of();
}
```
```rust
pub async fn on_data_received(strategy: FundForgeStrategy, notify: Arc<Notify>, mut event_receiver: mpsc::Receiver<EventTimeSlice>) {

    // subscribing to multiple items while unsubscribing from existing items
    // if our strategy has already warmed up, the subscription will automatically have warm up to the maximum number of bars and have history available.
    let aud_cad_60m = DataSubscription::new_custom("AUD-CAD".to_string(), DataVendor::Test, Resolution::Minutes(60), MarketType::Forex, CandleType::HeikinAshi);
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
    // all objects wrapped in a BaseDataEnum also have a subscription() fn. for example candle.subscription() will return the DataSubscription object.

    //only data we specifically subscribe to show up here, if the data is building from ticks but we didn't subscribe to ticks specifically, ticks won't show up but the subscribed resolution will.
    'strategy_loop: while let Some(event_slice) = event_receiver.recv().await {
        // we can subscribe in the event loop with no problems, the engine can handle this in live and backtest without skipping data. 
        // If the strategy was already warmed up, the consolidator will warm itself up to the maximum number of bars (50 in this case) and have history available. 
        // This is assuming we have the historical data serialized on the data server or available in the data vendor.
        let aud_usd_12m = DataSubscription::new("AUD-USD".to_string(), DataVendor::Test, Resolution::Minutes(12), BaseDataType::HeikinAshi, MarketType::Forex);
        strategy.subscribe(aud_usd_12m.clone(), 50).await;
    }
}
```

### Futures Subscriptions
You can subscribe using the `SymbolName` eg "MNQ" or the `SymbolCode` eg "MNQZ4".
If you are placing orders with SymbolCode instead of SymbolName you will also need to pass in the `FuturesExchange` as a string. Example `FuturesExchange::CME.to_string()`
By passing in the exchange string we are telling the data server to specifically place a trade on the "Z4" Contract.

### Subscription Performance Impacts
In back-testing using multiple symbols will slow down the engine only relative to the size of the primary data set, since the Subscription manager updates consolidators concurrently,
adding additional subscriptions per symbol has a minimal impact on performance on multithreaded systems, if you are subscribed to 1 minute bars, you can subscribe to 10min, 15min, 60min simultaneously
and expect no noticeable impact from the additional consolidators.

In back-testing subscribing to multiple symbols, will have a linear performance impact, with each symbol subscribed we are increasing the size of the data which must be sorted into our primary data feed by n(1).
This is one downside of the microservice API instances, we need to check each symbol data vendor api address and request the data per symbol.
If you are backtesting a large number of symbols, you will see a delay in the backtest at the start of each historical month as we pull new primary resolution data from the data server 1 symbol at a time.

I have made some functions to make this concurrent but using them would involve hard coding the platform to only allow 1 data server instance for all DataVendor apis and eliminate the possibility of using api microservices.
I felt the trade-off of longer back-tests was worth it. 

Even if we were receiving the data concurrently it would still have to be validated into TimeSlices 1 data point at a time, so we could not reduce the impact much regardless.

In live trading the above problem would only be an issue if we were constantly requesting for history of very low resolution data sets for many symbols, this can always be overcome with code and so it is not an issue.

## Retained History
The consolidators will retain history when specified during subscription.
If we want to have the engine keep a history automatically, we will need a reference to the subscription to access it.
```rust
pub async fn on_data_received(strategy: FundForgeStrategy, notify: Arc<Notify>, mut event_receiver: mpsc::Receiver<EventTimeSlice>) {
    
    // if our strategy has already warmed up, the subscription will automatically have warm up to the maximum number of bars and have history available.
    let aud_cad_60m = DataSubscription::new_custom("AUD-CAD".to_string(), DataVendor::Test, Resolution::Minutes(60), MarketType::Forex, CandleType::HeikinAshi);

    // this will return a RollingWindow<BaseData> for the subscription by cloning the history.
    // at the current point this clones the whole rolling window, and so is not suitable for frequent use of large history.
    // An alternative would be to get the history once, after initializing the indicator, so we have a warmed up history, then keep the history in a separate variable and add the new data to it.
    let history: Option<RollingWindow<Candle>>  = strategy.candle_history(&aud_usd_15m).await;

    // if we are keeping a large history and need to access it often, it could be better to manually keep the history we need to avoid clone()ing the whole history on every iter.
    // we could set the history_to_retain variable to some small number and keep the larger history in a separate variable.
    let rolling_window: RollingWindow<BaseDataEnum> = RollingWindow::new(100);
    for data in history {
        rolling_window.add(data);
    }
    

    // we can get the open candle for a candles subscription, note we return an optional `Candle` object, not a `BaseDataEnum`
    let aud_cad_60m_candles = DataSubscription::new("AUD-CAD".to_string(), DataVendor::Test, Resolution::Minutes(60), BaseDataType::Candles, MarketType::Forex);
    let current_open_candle: Option<Candle> = strategy.open_candle(&aud_cad_60m_candles);
    // we can get a historical candle from the history we retained according to the 'history_to_retain' parameter when subscribing. (this only retains closed Candles)
    let last_historical_candle: Option<Candle>  = candle_index(&aud_cad_60m_candles, 0);
    //expensive currently clones whole object, not an updating reference, but will give you the whole history should you need it (better to manually keep history in strategy loop)
    let candle_history: Option<RollingWindow<Candle>>  = strategy.candle_history(&aud_cad_60m_candles).await;

    // we can get the open quotebar for a quotebars subscription, note we return an optional `Candle` QuoteBar, not a `BaseDataEnum`
    let aud_cad_60m_quotebars = DataSubscription::new("AUD-CAD".to_string(), DataVendor::Test, Resolution::Minutes(60), BaseDataType::QuoteBars, MarketType::Forex);
    let current_open_candle: Option<QuoteBar> = strategy.open_bar(&aud_cad_60m_quotebars);
    // we can get a historical quotebar from the history we retained according to the 'history_to_retain' parameter when subscribing. (this only retains closed QuoteBars)
    let last_historical_quotebar: Option<QuoteBar>  = bar_index(&aud_cad_60m, 0);
    //expensive currently clones whole object, not an updating reference, but will give you the whole history should you need it (better to manually keep history in strategy loop)
    let bar_history: Option<RollingWindow<QuoteBar>>  = strategy.bar_history(&aud_cad_60m_quotebars).await; 

    let aud_cad_ticks = DataSubscription::new("AUD-CAD".to_string(), DataVendor::Test, Resolution::Ticks(1), BaseDataType::Ticks, MarketType::Forex);
    // we can get a historical tick from the history we retained according to the 'history_to_retain' parameter when subscribing.
    // since ticks are never open or closed the current tick is always in history as index 0, so the last tick is index 1
    let current_tick: Option<Tick>  = tick_index(&aud_cad_ticks, 0);
    let last_historical_tick: Option<Tick>  = tick_index(&aud_cad_ticks, 1);
    //expensive currently clones whole object, not an updating reference, but will give you the whole history should you need it (better to manually keep history in strategy loop)
    let tick_history: Option<RollingWindow<Tick>>  = strategy.tick_history(&aud_cad_ticks).await;

    let aud_cad_quotes = DataSubscription::new("AUD-CAD".to_string(), DataVendor::Test, Resolution::Instant, BaseDataType::Quotes, MarketType::Forex);
    // we can get a historical quote from the history we retained according to the 'history_to_retain' parameter when subscribing.
    // since quotes are never open or closed the current quote is always in history as index 0, so the last quote is index 1
    let current_quote: Option<Quote>  = quote_index(&aud_cad_quotes, 0);
    let last_historical_quote: Option<Quote>  = quote_index(&aud_cad_quotes, 1);
    //expensive currently clones whole object, not an updating reference, but will give you the whole history should you need it (better to manually keep history in strategy loop)
    let quote_history: Option<RollingWindow<Quote>>  = strategy.quote_history(&aud_cad_quotes).await;

    // if our strategy has already warmed up, the subscription will automatically have warmup to the maximum number of bars and have history available.
    let aud_cad_60m = DataSubscription::new_custom("AUD-CAD".to_string(), DataVendor::Test, Resolution::Minutes(60), MarketType::Forex, CandleType::HeikinAshi);
    'strategy_loop: while let Some(event_slice) = event_receiver.recv().await {
        // this will give us the closed bar, 2 bars ago
        let two_closed_bars_ago = &strategy.candle_index(&aud_cad_60m, 1).await;
        println!("{}...{} Three bars ago: {:?}", count, aud_cad_60m.symbol.name, three_bars_ago);
        
        // this will give us the current open bar
        let current_open_candle = &strategy.open_candle(&aud_cad_60m).await;
        println!("{}...{} Current data: {:?}, {}", count, aud_cad_60m.symbol.name, data_current.is_closed);
        
        //The data points can be accessed by index. where 0 is the latest data point.
        let last_data_point = rolling_window.get(0);
        
        notify.notify_one();
    }
}
```

## BaseDataEnum
```rust
pub async fn on_data_received(strategy: FundForgeStrategy, notify: Arc<Notify>, mut event_receiver: mpsc::Receiver<EventTimeSlice>) {
    'strategy_loop: while let Some(event_slice) = event_receiver.recv().await {
        for strategy_event in event_slice {
            match strategy_event {
                StrategyEvent::TimeSlice(_time, time_slice) => {
                    'base_data_loop: for base_data in &time_slice {
                        match base_data {
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
    }
}
```

### Alternative To Iterating a TimeSlice
Similarly, for TimeSlices we can retrieve an iterator for a specific type, allowing us to handle without iterating the entire time slice.
```rust
fn example() {
    pub enum BaseDataType {
        Ticks = 0,
        Quotes = 1,
        QuoteBars = 2,
        Candles = 3,
        Fundamentals = 4,
    }
    
    let time_slice: TimeSlice = TimeSlice::new();
    
    // this will give us an owned iterator of all base data enums of the required type in the slice, so we can pass it to another function etc.
    let owned_iter: Iterator<Item = BaseDataEnum> = time_slice.get_by_type(BaseDataType::Candles);
    // we could pass to a fn that only handles candles.
    handle_candles(owned_iter);

    // this will give us a reference to an iterator of all objects of the required type
    let borrowed_iter: Iterator<Item = &BaseDataEnum> = time_slice.get_by_type_borrowed(data_type: BaseDataType);
}
```

## Indicators
Indicators can be handled automatically by the strategy Indicator handler, or we can create and manage them manually in the `on_data_received()` function.
We can implement the `Indicators trait` for our custom indicators.
If building a custom indicator be sure to add it to the IndicatorEnum and complete the matching statements, so that the Indicator handler can handle it if needed.

If you want to help development, creating common indicators for fund-forge is easy, see [Indicators readme](indicators/INDICATORS_README.md)

### Indicator Values
```rust
fn example() {
    pub struct IndicatorPlot {
        pub name: PlotName,
        pub value: Price,
        pub color: Option<Color>,
    }

    // Some indicators need multiple values per time instance, so each time instance they create an IndicatorValues object, to hold values for all plots
    pub struct IndicatorValues {
        pub name: IndicatorName,
        pub time: String,
        pub subscription: DataSubscription,
        pub plots: BTreeMap<PlotName, IndicatorPlot>, // we can look up a plot value by name
    }
    
    let mut values = IndicatorValues::default();
    
    let name: &IndicatorName = values.name();

    // get the time in the UTC time zone
    let time: DateTime<Utc> = values.time_utc();

    // get the time in the local time zone
    let local_time: dateTime<Tz> = values.time_local(time_zone: &Tz);

    /// get the value of a plot by name
    let plot: IndicatorPlot = values.get_plot(plot_name: &PlotName);

    /// get all the plots`
    let plots : BTreeMap<PlotName, IndicatorPlot> = values.plots();
    
    ///or we can just access the plots directly
    let plots:  &BTreeMap<PlotName, IndicatorPlot> = &values.plots;

    /// insert a value into the values
     values.insert_plot(&mut self, plot_name: PlotName, value: IndicatorPlot);
}
```

### Using Indicators
We can use the inbuilt indicators or create our own custom indicators.

#### Creating Indicators
For creating custom indicators, we just need to implement the `Indicators trait` and either:
1. Create and hardcode `IndicatorEnum` variant including all matching statements, or
2. Wrap our indicator in the `IndicatorEnum::Custom(Box<dyn AsyncIndicators + Send + Sync>)` variant, where it will be used via `Box<dyn Indicators>` dynamic dispatch.
The fist option is the most performant, but if you want to create and test a number of indicators, you can save hardcoding the enum variants by using the second option.
Once you have tested and are happy with the performance of your custom indicator, you can then hardcode it into the IndicatorEnum.
You don't actually need to do any of this if you want to manually handle your Indicators in the `fn on_data_received()` function, but if you wrap in the `IndicatorEnum::Custom(Box<dyn Indicators>)` variant, 
you will be able to handle it in the Indicator handler, which will automatically update the indicators for you and return enums `IndicatorEvents` to the `fn on_data_received()`.
3. Creating indicators for fund-forge is easy, see [Indicators readme](indicators/INDICATORS_README.md)

#### Using Indicators
If we pass the indicator to `strategy.indicator_subscribe(indicator: IndicatorEnum).await;` the handler will automatically handle, history, warmup and deletion of the indicator when we unsubscribe a symbol.
There aren't many reasons not to use this fn.

we can access the indicators values the same way we do for base_data 
```rust
fn example(strategy: FundForgeStrategy) {
    let mut heikin_atr = AverageTrueRange::new(String::from("heikin_atr"), aud_cad_60m.clone(), 100, 14).await;
    let heikin_atr_20 = IndicatorEnum::AverageTrueRange(AverageTrueRange::new(String::from("heikin_atr_20"), aud_cad_60m.clone(), 100, 20).await);

    // auto subscribe will subscribe the strategy to the indicators required data feed if it is not already, 
    // if this is false and you don't have the subscription, the strategy will panic instead.
    // if true then the new data subscription will also show up in the strategy event loop
    let auto_subscribe: bool = true;

    //subscribe the strategy to auto manage the indicator
    strategy.subscribe_indicator(heikin_atr_20, auto_subscribe).await;
}
```
```rust
pub async fn on_data_received(strategy: FundForgeStrategy, notify: Arc<Notify>, mut event_receiver: mpsc::Receiver<EventTimeSlice>) {
    
    // Subscribe to a 60-minute candle for the AUD-CAD pair
    let aud_cad_60m = DataSubscription::new_custom("AUD-CAD".to_string(), DataVendor::Test, Resolution::Minutes(60), MarketType::Forex, CandleType::HeikinAshi);
    strategy.subscriptions_update(vec![aud_cad_60m.clone()],100).await;
    
    // Create a manually managed indicator directly in the on_data_received function (14 period ATR, which retains 100 historical IndicatorValues)
    let mut heikin_atr = AverageTrueRange::new(String::from("heikin_atr"), aud_cad_60m.clone(), 100, 14).await;
    let mut heikin_atr_history: RollingWindow<IndicatorValues> = RollingWindow::new(100);

    // let's make another indicator to be handled by the IndicatorHandler, we need to wrap this as an indicator enum variant of the same name.
    let heikin_atr_20 = IndicatorEnum::AverageTrueRange(AverageTrueRange::new(String::from("heikin_atr_20"), aud_cad_60m.clone(), 100, 20).await);
    strategy.indicator_subscribe(heikin_atr_20).await;
    
    'strategy_loop: while let Some(event_slice) = event_receiver.recv().await {
        for (time, strategy_event) in event_slice.iter() {
            match strategy_event {
                StrategyEvent::TimeSlice(time_slice) => {
                    'base_data_loop: for base_data in time_slice.iter() {
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
                                    let atr = heikin_atr.index(3);
                                    println!("{}...{} ATR 3 bars ago: {}", strategy.time_utc().await, aud_cad_60m.symbol.name, atr.unwrap());
                                    
                                    //or we can use our own history to get the value at a specific index
                                    let atr = heikin_atr_history.get(10);
                                    println!("{}...{} ATR 10 bars ago: {}", strategy.time_utc().await, aud_cad_60m.symbol.name, atr.unwrap());
                                }
                            },
                            _ => {}
                        }
                    }
                }
                StrategyEvent::IndicatorEvent(_, event) => {
                    //we can handle indicator events here, this is useful for working with the IndicatorHandler.
                    // which will handle warming up, updating, subscribing etc for many indicators.
                    match event {
                        IndicatorEvents::IndicatorAdded(name) => {}
                        IndicatorEvents::IndicatorRemoved(name) => {}
                        IndicatorEvents::IndicatorTimeSlice(slice) => {
                            // we can see our auto manged indicator values for here.
                            for indicator_values in slice {
                                println!("Indicator Time Slice: {:?}", indicator_values);
                            }

                            // we could also get the auto-managed indicator values from the strategy at any time. we should have history immediately since the indicator will warm itself up.
                            // this will not be the case if we did not have historical data available for the indicator.
                            let history: Option<RollingWindow<IndicatorValues>> = strategy.indicator_history(IndicatorName::from("heikin_atr_20")).await;
                            if let Some(history) = history {
                                println!("History: {:?}", history.history());
                            }

                            let current: Option<IndicatorValues> = strategy.indicator_current(&IndicatorName::from("heikin_atr_20")).await;
                            if let Some(current) = current {
                                println!("Current: {:?}", current.values());
                            }

                            let index: Option<IndicatorValues> = strategy.indicator_index(&IndicatorName::from("heikin_atr_20"), 3).await;
                            if let Some(index) = index {
                                println!("Index: {:?}", index.values());
                            }
                        }
                        IndicatorEvents::Replaced(name) => {}
                    }
                }
                _ => {}
            }
        }
    }
}
```

## Accounts
Live strategies will not differentiate between positions they opened or other account positions.
They will treat any position on the account as if they opened it, unless you have your own logic for identifying positions.
You could use the Order "tag" property.

Positions are created managed and closed automatically when you place orders, they will update as the account/ledger position updates.
each position has a String 'tag' property: `position.tag` this tag will be the same as the 'order.tag' which resulted in the position being created.

This can provide hints to bugs in your strategy, for example if you have a position with the tag "Exit Long", you know you have over filled you exit order, 
because an exit order should not create a position, it should close one.
```rust
fn example(strategy: &FundForgeStrategy, brokerage: Brokerage, account_name: AccountName, candle: Candle) {

   let account_1 = Account::new(Brokerage::Test, "Test_Account_1".to_string());
   
   let symbol_code = "M6AZ4".to_string();
    // to find out if the broker and account is in profit on the symbol, returns false as default if no position
    let in_profit: bool = strategy.in_profit(&account_1, &symbol_code);

    // to find out if the broker and account is in draw down on the symbol, returns false as default if no position
    let in_drawdown: bool = strategy.in_drawdown(&account_1, &symbol_code);

    // to find out if the broker and account is long on the symbol, returns false as default if no position
    let is_long: bool = strategy.is_long(&account_1, &symbol_code);

    // to find out if the broker and account is short on the symbol, returns false as default if no position
    let is_short: bool = strategy.is_short(&account_1, &symbol_code);

    // to find out if the broker and account is flat on the symbol, returns true as default if no position
    let is_flat: bool = strategy.is_flat(&account_1, &symbol_code);

    // returns the open pnl for the current position, returns 0.0 if there is no position
    let open_profit: Decimal = strategy.pnl(&account_1, &symbol_code);

    // returns the booked pnl for the current position, returns 0.0 if there is no position
    // does not return the total pnl for all closed positions on the symbol, just the current open one.
    let booked_profit: Decimal = strategy.booked_pnl(&account_1, &symbol_code);

    // returns the open quantity / size of our open position
    // if no position it returns dec!(0)
    let position_size: Decimal = strategy.position_size(&account_1, &symbol_code);

    // to flatten an account, in live this will flatten all psotions, not just strategy positions.
    strategy.flatten_all_for(&self, account_1).await;
}
```

### Note for Symbol Name with Futures and StrategyMode:: Live 
When using the functions above with futures in live mode you might need to get the symbol code, if you are only placing orders using the symbol name. \
The symbol code will be returned in order events, an example of a symbol code or futures 'symbol' == "M6AZ4". \
Alternatively just use the symbol code as symbol name. \
There will be functions built to make this effortless at a later date. \
The reason it works this way is to enable the trading of calander spreads, where a trader might place trades on contracts with the same SymbolName. \
Currently in Backtesting, you will need to use the SymbolName of your data, this will all be fixed in the future once the Live api's are stable. \
```rust
  // A Note for Live Mode
   StrategyEvent::OrderEvents(event) => {
      match event.symbol_code() {
         None => {}
         Some(code) => {
            if code.starts_with("M6") {
               let symbol_code: String = code;
            }
         }
      }
   }
```

## Timed Events 
TimedEvents are a way to schedule events to occur at a specific time, they are useful for scheduling events like closing orders at a specific time, or sending notifications.
We can also specify whether the event should fire during warm up.
When an event is triggered the event name will be sent to the StrategyBuffer as a `StrategyEvent::TimedEvents(String)`
```rust
fn example() {
    pub enum EventTimeEnum {
        /// Events to occur at on a specific day of the week
        Weekday {
            day: Weekday,
            fire_in_warmup: bool
        },
        /// Events to occur at a specific hour of the day
        HourOfDay {
            hour: u32,
            fire_in_warmup: bool
        },
        /// Events to occur at a specific time on a specific day of the week
        TimeOnWeekDay {
            day: Weekday,
            hour: u32,
            minute: u32,
            second: u32,
            fire_in_warmup: bool
        },
        /// Events to occur at a specific date and time only once
        DateTime{
            date_time: DateTime<Utc>,
            fire_in_warmup: bool
        },
        /// Events to occur at a specific time of the day
        TimeOfDay {
            hour: u32,
            minute: u32,
            second: u32,
            fire_in_warmup: bool
        },
        /// Events to occur at a specific interval
        Every {
            duration: Duration,
            next_time: DateTime<Utc>,
            fire_in_warmup: bool
        }
    }

    //first create the timed event variant
    let event_time = EventTimeEnum::HourOfDay { hour: 12 };

    // next we need to create a TimedEvent
    
    // we need
    let event = TimedEvent::new("test_event".to_string, event_time);
    
    // then we pass the event to the strategy timed event handler
    strategy.timed_event_subscribe(event).await;
    
    // We can remove the event by name
    strategy.timed_event_unsubscribe("test_event".to_string()).await;
    
    // when the time is reached the event will be sent to the receiver
    while let Some(event) = receiver.recv().await {
        println!("Event: {:?}", event);
    }
}
```

## Drawing Tools
Fund forge strategies are designed to be able to interact with the user through drawing tools.

```rust
fn example() {
    //todo add tool example
    strategy.drawing_tool_add(tool).await;
    strategy.drawing_tool_update(tool).await;
    strategy.drawing_tool_remove("test_tool".to_string()).await;
    strategy.drawing_tools_remove_all().await;
    
    // A strategy event is fires when an outside source alters the drawing tools
    pub enum DrawingToolEvent {
        Add(DrawingTool),
        Remove(DrawingTool),
        Update(DrawingTool),
        RemoveAll
    }
}
```

## History Requests
We can request history for a subscription in the event loop, this is costly if we are requesting a history not provided by the DataVendor as it will need to be consolidated.
This function will avoid look ahead bias, it will never return data.time_utc() > strategy.time_utc()
```rust
async fn example() {
    let strategy = FundForgeStrategy::default();
    
    let aud_cad_60m = DataSubscription::new_custom("AUD-CAD".to_string(), DataVendor::Test, Resolution::Minutes(60), MarketType::Forex, CandleType::HeikinAshi);
    let from_time = NaiveDate::from_ymd_opt(2023, 03, 20).unwrap().and_hms_opt(0, 0, 0).unwrap();
    let time_zone = Australia/Sydney;
    
    // Get the history based on the strategy utc time
    let history_from_local: BTreeMap<DateTime<Utc>, TimeSlice> = strategy.history_from_local_time(from_time, aud_cad_60m.clone()).await;
    for (time, slice) in history_from_local {
        for base_data in slice {
            println!("{}... {}", time, base_data)
        }
    }

    // Get history based on the strategy local time
    // This history will start from a different date, because the from_time will be parsed using the time_zone, however the end date will be the strategy time for both.
    let history_from_utc: BTreeMap<DateTime<Utc>, TimeSlice> = strategy.history_from_utc_time(from_time.clone(), time_zone.clone(), aud_cad_60m.clone()).await;
    for (time, slice) in history_from_utc {
        for base_data in slice {
            println!("{}... {}", time, base_data)
        }
    }
    
    // We can also get a specific date range up to the current strategy time, the strategy methods will protect against look ahead bias.
    let to_time = NaiveDate::from_ymd_opt(2023, 03, 30).unwrap().and_hms_opt(0, 0, 0).unwrap();
    let history_range_from_local = strategy.historical_range_from_local_time(from_time.clone(), to_time.clone(), time_zone.clone(), aud_cad_60m.clone());
    for (time, slice) in history_range_from_local {
        for base_data in slice {
            println!("{}... {}", time, base_data)
        }
    }

    // same as the first examples, the start time will be different due to time zone conversion, the end time will be autocorrected if it is > than strategy.time_utc()
    let history_range_from_utc = strategy.historical_range_from_utc(from_time.clone(), to_time.clone(), aud_cad_60m.clone());
    for (time, slice) in history_range_from_utc {
        for base_data in slice {
            println!("{}... {}", time, base_data)
        }
    }
}
```

## Order Books 
THIS IS NOT FINALIZED
***Things to consider***
- The engine updates best bid, best offer, order book levels and last prices using `SymbolName` if we have more than 1 data feed per SymbolName, those streams will be combined into the same maps.
- The best bid and best offer will always replace and == order book level 0
- The order books are split into BID_BOOK and ASK_BOOK
- There is no point in having 2 feeds for the same SymbolName from multiple `DataVendors`, just use the most accurate or fastest updating vendor.
- If we have Quote order order book data, backtest fills will be simulated as realistically as possible, this will depend on volume and the number of book levels.
- If we have quotes with no volume we will fill at the bid or ask. 
- If we have a full order book, we will consume volume ascending or descending book levels until we fill our order and we will fill at the average price, this assumes we get to absorb all volume.
- A historical currency converter api will be made for backtesting, this will allow currency conversions depending on the symbols pnl currency into the account currency for higher accuracy backtesting.
```rust
async fn example() {
   //this is being updated currently
}
```
[see Market Handler Code](https://github.com/BurnOutTrader/fund-forge/blob/main/ff_standard_lib/src/market_handler/market_handlers.rs)

## Estimate Fill Price
There is a function used by the engine market handler to simulate live fills, if we have multiple order book levels the fill price will be averaged based on volume.
This makes the assumption we get to consume all volume at each level as needed, without comptetion from other participants.
When there is only best bid and best ask prices we will assume a full fill at that price.
When there is no best bid or best ask, we will assume a fill at the last price.
The strategy instance can also use this fn to estimate its fill price ahead of placing an order by calling the associated function:
```rust
fn example() {
    let order_side: OrderSide = OrderSide::Buy;
    let symbol_name: SymbolName =SymbolName::from("AUD-CAD");
    let volume: Volume = dec!(0.0);
    let brokerage: Brokerage = Brokerage::Test;
    
    // we can get the best estimate based on our intended trade volume //todo this needs to be changed, it currently wont work.
    let estimated_fill_price: Result<Price, FundForgeError> = get_market_fill_price_estimate(order_side, symbol_name, volume, brokerage).await;
    let price: Price = estimated_fill_price.unwrap();
    
    // we can get the closest market estimate without volume, just check best price if we have best bid offer we will get the correct return based on order side, else we just get the last price.
    let order_side: OrderSide = OrderSide::Buy;
    let symbol_name: SymbolName =SymbolName::from("AUD-CAD");
    let estimated_price: Result<Price, FundForgeError> = get_market_price (
        order_side: &OrderSide,
        symbol_name: &SymbolName,
    );
    let price: Price = estimated_price.unwrap();
}
```
***Things to consider***
- If we have an order book feed this will be more accurate.
- The engine updates best bid, best offer, order book levels and last prices using `SymbolName` if we have more than 1 data feed per SymbolName, those streams will be combined into the same maps.
- The best bid and best offer will always replace  == order book level 0
- The order books are split into BID_BOOK and ASK_BOOK
- There is no point in having 2 feeds for the same SymbolName from multiple `DataVendors`, just use the most accurate or fastest updating vendor.

## Placing Orders
In backtesting a new ledger will be instantiated for each AccountId and Brokerage combination to simulate any number of accounts.
This is in its infancy, market handlers are very raw and untested and the way they are instantiated and interact with the engine will change in future updates.
The backtesting engine pnl is not accurate at this moment.

The exchange field is an `Option<String>`, this can be used for advanced order routing. \
```rust
let exchange: FuturesExchange = FuturesExchange::CME;
let exchange: String = Some(CME.to_string());
```

When trading futures using Rithmic, the Rithmic Api will try to find the best contract using information from Rithmic. \
If no information is found it will use the front month contract automatically.
To override this behaviour we can pass in `symbol_code: Some("specific_symbol_code)`

If you want the server to make the decision, just `use exchange: None`
This field is currently irrelevant in backtesting, you will need to use "MNQ" as the symbol.
This logic will be improved in the future.

```rust
async fn example() {
    let strategy = FundForgeStrategy::default();

    // Example inputs for account_id, symbol_name, brokerage, etc.
    let account_1 = Account::new(Brokerage::Test, "Test_Account_1".to_string());
    let symbol_name: SymbolName = SymbolName::from("AAPL");
    let quantity: Volume = dec!(100.0); //Decimal to allow crypto
    let tag = String::from("Example Trade");
    
    /* The first 2 order types Enter Long and Enter short have the option of attaching brackets.
        If you are already long and you place another enter long position, it will add to the existing position.
        If you are already long and the new enter long position has brackets, those brackets will replace the existing brackets.
        
        More sophisticated brackets will be added in future versions.
        
        ### Futures Subscriptions
      You can subscribe using the `SymbolName` eg "MNQ" or the `SymbolCode` eg "MNQZ4".
      If you are placing orders with SymbolCode instead of SymbolName you will also need to pass in the `FuturesExchange` as a string. Example `FuturesExchange::CME.to_string()`
      By passing in the exchange string we are telling the data server to specifically place a trade on the "Z4" Contract.
    */

    // Enter a long position and close any existing short position on the same account / symbol
    let order_id: OrderId = strategy.enter_long(
        account: &account_1,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        exchange: Option<String>,
        quantity: Volume,
        brackets: Option<Vec<ProtectiveOrder>>,
        tag: String
    ).await;

    // Enter a short position and close any existing long position on the same account / symbol
    let order_id: OrderId = strategy.enter_short(
        account: &account_1,
        symbol_code: Option<SymbolCode>,
        exchange: Option<String>,
        quantity: Volume,
        brackets: Option<Vec<ProtectiveOrder>>,
        tag: String
    ).await;
    
    // Protective orders for Enter Long and Enter Short
    pub enum ProtectiveOrder {
        TakeProfit {
            price: Price
        },
        StopLoss {
            price: Price
        },
        TrailingStopLoss {
            price: Price,
            trail_value: Price
        },
    }

    // Exit a long position and get back the order_id
    let order_id: OrderId = strategy.exit_long(
        account: &account_1,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        exchange: Option<String>,
        quantity: Volume,
        tag: String
    ).await;

    // Exit a short position and get back the order_id
    let order_id: OrderId = strategy.exit_short(
        account: &account_1,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        exchange: Option<String>,
        quantity: Volume,
        tag: String
    ).await;

    // Place a market buy order and get back the order_id
    let order_id: OrderId = strategy.buy_market(
        account: &account_1,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        exchange: Option<String>,
        quantity: Volume, 
        tag: String
    ).await;

    // Place a market sell order and get back the order_id
    let order_id: OrderId = strategy.sell_market(
        account: &account_1,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        exchange: Option<String>,
        quantity: Volume,
        tag: String
    ).await;

    // Place a limit order and get back the order_id
    let order_id: OrderId = strategy.limit_order(
        account: &account_1,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        exchange: Option<String>,
        quantity: Volume, 
        side: OrderSide, 
        limit_price: Price, 
        tif: TimeInForce, 
        tag: String
    ).await;

    // Enter a market if touched order
    let order_id: OrderId = strategy.market_if_touched (
        account: &account_1,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        exchange: Option<String>,
        quantity: Volume, 
        side: OrderSide, 
        trigger_price: Price, 
        tif: TimeInForce, 
        tag: String
    ).await;

    // Enter a stop order (this is not a protective order)
    let order_id: OrderId = strategy.stop_order (
        account: &account_1,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        exchange: Option<String>,
        quantity: Volume,
        side: OrderSide,
        trigger_price: Price,
        tif: TimeInForce,
        tag: String,
    ).await;

    // Enter a stop limit order
    let order_id: OrderId = strategy.stop_limit (
        account: &account_1,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        exchange: Option<String>,
        quantity: Volume,
        side: OrderSide,
        tag: String,
        limit_price: Price,
        trigger_price: Price,
        tif: TimeInForce
    ).await;

    // Cancel the order using the returned ID. the cancel result will show up in strategy events loop.
    strategy.cancel_order(
        order_id: OrderId
    ).await;
    
    // Cancel all orders for the symbol, with the brokerage and account
    cancel_orders(
        brokerage: Brokerage, 
        account_id: AccountId, 
        symbol_name: SymbolName
    ).await;

    // Update an order using its order_id
    strategy.update_order(
        order_id: OrderId, 
        order_update_type: OrderUpdateType
    ).await;
    
    //update types
    pub enum OrderUpdateType {
        LimitPrice(Price),
        TriggerPrice(Price),
        TimeInForce(TimeInForce),
        Quantity(Volume),
        Tag(String),
    }
}
```

## Debugging Strategies
Exported positions include their tag property, which always == the tag of the order that created the position.

You can export positions and print ledgers at run time using:
```rust
fn example(strategy: &FundForgeStrategy) {
    strategy.export_trades(&String::from("./trades exports"));
    strategy.print_ledgers();
}
```
When a strategy places an order, the order 'tag' property is returned with the order event.

When a new position is created the position 'tag' property will be the tag of the order that resulted in the position.

This has multiple debug benefits:

Scenarios:
1. If you over-fill an order: You are long 100, and you sell at market 200, with you order tag as "Take Profit Long". \
A short position will be opened with the tag: "Take Profit Long", when reviewing positions you will see this tag as entering a short position. \
You will see this in your exported positions `.csv`.


2. You accidentally enter long instead of short. \
You have a method to add to a short position, you have the order tag: "Add Short", but you accidentally use `strategy.enter_long()` instead of `strategy.enter_short()`. \
A long position will be opened with the tag "Add Short". \
You will see this in your exported positions `.csv`.

The `tag` property of `PositionUpdateEvents` that are fed to the strategy, will use the order 'tag' that triggered the event.
In this way we can see in real time the effect of orders on a position.

Uploading your exported trades to an Ai model like claude or GPT will quickly spot the mistake.
