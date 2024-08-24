# `FundForgeStrategy::Initialize()`
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

