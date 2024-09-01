use crate::engine::Engine;
use crate::interaction_handler::InteractionHandler;
use crate::market_handlers::MarketHandlerEnum;
use crate::strategy_state::StrategyStartState;
use ahash::AHashMap;
use chrono::{DateTime, Duration, FixedOffset, NaiveDateTime, Utc};
use chrono_tz::Tz;
use ff_standard_lib::apis::brokerage::Brokerage;
use ff_standard_lib::drawing_objects::drawing_object_handler::DrawingObjectHandler;
use ff_standard_lib::drawing_objects::drawing_tool_enum::DrawingTool;
use ff_standard_lib::helpers::converters::{
    convert_to_utc, time_convert_utc_datetime_to_fixed_offset,
    time_convert_utc_naive_to_fixed_offset,
};
use ff_standard_lib::indicators::indicator_enum::IndicatorEnum;
use ff_standard_lib::indicators::indicator_handler::IndicatorHandler;
use ff_standard_lib::indicators::indicators_trait::IndicatorName;
use ff_standard_lib::indicators::values::IndicatorValues;
use ff_standard_lib::standardized_types::accounts::ledgers::AccountId;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::history::range_data;
use ff_standard_lib::standardized_types::base_data::order_book::OrderBook;
use ff_standard_lib::standardized_types::enums::{OrderSide, StrategyMode};
use ff_standard_lib::standardized_types::orders::orders::Order;
use ff_standard_lib::standardized_types::rolling_window::RollingWindow;
use ff_standard_lib::standardized_types::strategy_events::{
    EventTimeSlice, StrategyEvent, StrategyInteractionMode,
};
use ff_standard_lib::standardized_types::subscription_handler::SubscriptionHandler;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol};
use ff_standard_lib::standardized_types::time_slices::TimeSlice;
use ff_standard_lib::standardized_types::OwnerId;
use ff_standard_lib::timed_events_handler::{TimedEvent, TimedEventHandler};
use std::collections::BTreeMap;
use std::env;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{mpsc, Notify};

/// The `FundForgeStrategy` struct is the main_window struct for the FundForge strategy. It contains the state of the strategy and the callback function for data updates.
///
/// In Backtest the data stream is parsed from bytes, sorted into time slices and wrapped in StrategyEvent to be sent by the broadcaster to the passed in Receiver, where it needs to be converted using StrategyEvent::from_bytes() to be used.
///
/// In Live mode the objects are parsed from bytes, wrapped in a StrategyEvent to be sent by the broadcaster to the passed in Receiver, where it needs to be converted using StrategyEvent::from_bytes() to be used.
///
/// All to_bytes() and from_bytes() conversions are 0 cost, as they are just a wrapper around rkyv, there is negligible difference in speed between sending the object or the bytes and so it is decided to just use to and from bytes.
/// By using bytes rather than the object we are able to use the same Subscriber pattern for all data sent between different machines or internal processes or even both at the same time using the same BroadCaster
/// This allows a high level of flexibility and scalability for the FundForge system, allowing infinite internal and external subscribers to subscribe to StrategyEvents.
/// # Properties
pub struct FundForgeStrategy {
    /// the id used by the local fund_forge_instance to identify the strategy, private so that it cannot be changed at run time.
    pub owner_id: OwnerId,

    start_state: StrategyStartState,
    /// Any drawing objects associated with the subscription for this strategy.
    /// Drawing objects aren't just Ui objects, they can be interacted with by the engine backend and used for trading signals.
    drawing_objects_handler: DrawingObjectHandler,

    interaction_handler: Arc<InteractionHandler>,

    market_event_handler: Arc<MarketHandlerEnum>,

    subscription_handler: Arc<SubscriptionHandler>,

    indicator_handler: Arc<IndicatorHandler>,

    timed_event_handler: Arc<TimedEventHandler>,
}

impl FundForgeStrategy {
    /// Initializes a new `FundForgeStrategy` instance with the provided parameters.
    ///
    /// # Arguments
    /// `owner_id: Option<OwnerId>`: The unique identifier for the owner of the strategy. If None, a unique identifier will be generated based on the executable's name. \
    /// `notify: Arc<Notify>`: The notification mechanism for the strategy, this is useful to slow the message sender channel until we have processed the last message. \
    /// `strategy_mode: StrategyMode`: The mode of the strategy (Backtest, Live, LivePaperTrading). \
    /// `interaction_mode: StrategyInteractionMode`: The interaction mode for the strategy. \
    /// `start_date: NaiveDateTime`: The start date of the strategy. \
    /// `end_date: NaiveDateTime`: The end date of the strategy. \
    /// `time_zone: Tz`: The time zone of the strategy, you can use Utc for default. \
    /// `warmup_duration: Duration`: The warmup duration for the strategy. \
    /// `subscriptions: Vec<DataSubscription>`: The initial data subscriptions for the strategy. \
    /// `strategy_event_sender: mpsc::Sender<EventTimeSlice>`: The sender for strategy events. \
    /// `replay_delay_ms: Option<u64>`: The delay in milliseconds between time slices for market replay style backtesting. \
    /// `retain_history: usize`: The number of bars to retain in memory for the strategy. This is useful for strategies that need to reference previous bars for calculations, this is only for our initial subscriptions. \
    ///  any additional subscriptions added later will be able to specify their own history requirements.
    /// `buffering_resolution: Option<Duration>`: The buffering resolution of the strategy. If we are backtesting, any data of a lower granularity will be consolidated into a single time slice.
    /// If out base data source is tick data, but we are trading only on 15min bars, then we can just consolidate the tick data and ignore it in on_data_received().
    /// In live trading our strategy will capture the tick stream in a buffer and pass it to the strategy in the correct resolution/durations, this helps to prevent spamming our on_data_received() fn.
    /// If we don't need to make strategy decisions on every tick, we can just consolidate the tick stream into buffered time slice events.
    /// This also helps us get consistent results between backtesting and live trading.
    /// If None then it will default to a 1-second buffer.
    pub async fn initialize(
        owner_id: Option<OwnerId>,
        notify: Arc<Notify>,
        strategy_mode: StrategyMode,
        interaction_mode: StrategyInteractionMode,
        start_date: NaiveDateTime,
        end_date: NaiveDateTime,
        time_zone: Tz,
        warmup_duration: Duration,
        subscriptions: Vec<DataSubscription>,
        retain_history: u64,
        strategy_event_sender: mpsc::Sender<EventTimeSlice>,
        replay_delay_ms: Option<u64>,
        buffering_resolution: Option<Duration>,
    ) -> FundForgeStrategy {
        let start_state = StrategyStartState::new(
            strategy_mode.clone(),
            start_date,
            end_date,
            time_zone.clone(),
            warmup_duration,
            buffering_resolution,
        );
        let start_time = time_convert_utc_naive_to_fixed_offset(&time_zone, start_date);
        let owner_id = match owner_id {
            Some(owner_id) => owner_id,
            None => FundForgeStrategy::assign_owner_id(),
        };

        let market_event_handler = match strategy_mode {
            StrategyMode::Backtest => {
                MarketHandlerEnum::new(owner_id.clone(), start_time.to_utc(), strategy_mode.clone())
            }
            StrategyMode::Live => panic!("Live mode not yet implemented"),
            StrategyMode::LivePaperTrading => panic!("Live paper mode not yet implemented"),
        };

        let subscription_handler = SubscriptionHandler::new(strategy_mode).await;
        if !subscriptions.is_empty() {
            for subscription in subscriptions {
                subscription_handler
                    .subscribe(subscription.clone(), retain_history, start_time.to_utc())
                    .await
                    .unwrap();
            }
            let subscription_events = subscription_handler.subscription_events().await;
            let strategy_event = vec![StrategyEvent::DataSubscriptionEvents(
                owner_id.clone(),
                subscription_events,
                start_time.timestamp(),
            )];
            match strategy_event_sender.send(strategy_event).await {
                Ok(_) => {}
                Err(e) => {
                    println!("Error forwarding event: {:?}", e);
                }
            }
            subscription_handler.set_subscriptions_updated(false).await;
        }

        let strategy = FundForgeStrategy {
            start_state: start_state.clone(),
            owner_id: owner_id.clone(),
            drawing_objects_handler: DrawingObjectHandler::new(Default::default()),
            market_event_handler: Arc::new(market_event_handler),
            interaction_handler: Arc::new(InteractionHandler::new(
                replay_delay_ms,
                interaction_mode,
            )),
            subscription_handler: Arc::new(subscription_handler),
            indicator_handler: Arc::new(IndicatorHandler::new(
                owner_id.clone(),
                strategy_mode.clone(),
            )),
            timed_event_handler: Arc::new(TimedEventHandler::new()),
        };

        let engine = Engine::new(
            strategy.owner_id.clone(),
            notify,
            start_state,
            strategy_event_sender.clone(),
            strategy.subscription_handler.clone(),
            strategy.market_event_handler.clone(),
            strategy.interaction_handler.clone(),
            strategy.indicator_handler.clone(),
            strategy.timed_event_handler.clone(),
        )
        .await;

        Engine::launch(engine).await;

        strategy
    }

    pub async fn is_shutdown(&self) -> bool {
        let end_time = self.start_state.end_date.to_utc();
        if self.time_utc().await >= end_time {
            return true;
        }
        false
    }

    pub async fn enter_long(
        &self,
        account_id: AccountId,
        symbol_name: Symbol,
        brokerage: Brokerage,
        quantity: u64,
        tag: String,
    ) {
        let order = Order::enter_long(
            self.owner_id.clone(),
            symbol_name,
            brokerage,
            quantity,
            tag,
            account_id,
            self.time_utc().await,
        );
        self.market_event_handler.send_order(order).await;
    }

    pub async fn enter_short(
        &self,
        account_id: AccountId,
        symbol_name: Symbol,
        brokerage: Brokerage,
        quantity: u64,
        tag: String,
    ) {
        let order = Order::enter_short(
            self.owner_id.clone(),
            symbol_name,
            brokerage,
            quantity,
            tag,
            account_id,
            self.time_utc().await,
        );
        self.market_event_handler.send_order(order).await;
    }

    pub async fn exit_long(
        &self,
        account_id: AccountId,
        symbol_name: Symbol,
        brokerage: Brokerage,
        quantity: u64,
        tag: String,
    ) {
        let order = Order::exit_long(
            self.owner_id.clone(),
            symbol_name,
            brokerage,
            quantity,
            tag,
            account_id,
            self.time_utc().await,
        );
        self.market_event_handler.send_order(order).await;
    }

    pub async fn exit_short(
        &self,
        account_id: AccountId,
        symbol_name: Symbol,
        brokerage: Brokerage,
        quantity: u64,
        tag: String,
    ) {
        let order = Order::exit_short(
            self.owner_id.clone(),
            symbol_name,
            brokerage,
            quantity,
            tag,
            account_id,
            self.time_utc().await,
        );
        self.market_event_handler.send_order(order).await;
    }

    pub async fn buy_market(
        &self,
        account_id: AccountId,
        symbol_name: Symbol,
        brokerage: Brokerage,
        quantity: u64,
        tag: String,
    ) {
        let order = Order::market_order(
            self.owner_id.clone(),
            symbol_name,
            brokerage,
            quantity,
            OrderSide::Buy,
            tag,
            account_id,
            self.time_utc().await,
        );
        self.market_event_handler.send_order(order).await;
    }

    pub async fn sell_market(
        &self,
        account_id: AccountId,
        symbol_name: Symbol,
        brokerage: Brokerage,
        quantity: u64,
        tag: String,
    ) {
        let order = Order::market_order(
            self.owner_id.clone(),
            symbol_name,
            brokerage,
            quantity,
            OrderSide::Sell,
            tag,
            account_id,
            self.time_utc().await,
        );
        self.market_event_handler.send_order(order).await;
    }

    /// see the timed_event_handler.rs for more details
    pub async fn add_timed_event(&self, timed_event: TimedEvent) {
        self.timed_event_handler.add_event(timed_event).await;
    }

    /// see the timed_event_handler.rs for more details
    pub async fn remove_timed_event(&self, name: String) {
        self.timed_event_handler.remove_event(name).await;
    }

    /// see the indicator_enum.rs for more details
    pub async fn indicator_subscribe(&self, indicator: IndicatorEnum) {
        self.indicator_handler
            .add_indicator(indicator, self.time_utc().await)
            .await
    }

    /// see the indicator_enum.rs for more details
    pub async fn indicator_unsubscribe(&self, name: &IndicatorName) {
        self.indicator_handler.remove_indicator(name).await
    }

    /// see the indicator_enum.rs for more details
    pub async fn indicator_unsubscribe_subscription(&self, subscription: &DataSubscription) {
        self.indicator_handler
            .indicators_unsubscribe(subscription)
            .await
    }

    /// see the indicator_enum.rs for more details
    pub async fn indicator_index(
        &self,
        name: &IndicatorName,
        index: u64,
    ) -> Option<IndicatorValues> {
        self.indicator_handler.index(name, index).await
    }

    /// see the indicator_enum.rs for more details
    pub async fn indicator_current(&self, name: &IndicatorName) -> Option<IndicatorValues> {
        self.indicator_handler.current(name).await
    }

    /// see the indicator_enum.rs for more details
    pub async fn indicator_history(
        &self,
        name: IndicatorName,
    ) -> Option<RollingWindow<IndicatorValues>> {
        self.indicator_handler.history(name).await
    }

    /// returns the strategy time zone.
    pub fn time_zone(&self) -> &Tz {
        &self.start_state.time_zone
    }

    pub fn owner_id(&self) -> &OwnerId {
        &self.owner_id
    }

    pub async fn drawing_tools(&self) -> AHashMap<DataSubscription, Vec<DrawingTool>> {
        self.drawing_objects_handler.drawing_tools().await.clone()
    }

    /// Adds a drawing tool to the strategy.
    /// Drawing objects aren't just Ui objects, they can be interacted with by the engine backend and used for trading signals.
    /// # Arguments
    /// * `drawing_tool` - The drawing tool to add to the strategy.
    pub async fn drawing_tool_add(&self, drawing_tool: DrawingTool) {
        self.drawing_objects_handler
            .drawing_tool_add(drawing_tool)
            .await;
    }

    /// Removes a drawing tool from the strategy.
    /// Drawing objects aren't just Ui objects, they can be interacted with by the engine backend and used for trading signals.
    /// # Arguments
    /// * `drawing_tool` - The drawing tool to remove from the strategy.
    pub async fn drawing_tool_remove(&self, drawing_tool: DrawingTool) {
        self.drawing_objects_handler
            .drawing_tool_remove(drawing_tool)
            .await;
    }

    /// Updates a drawing tool in the strategy.
    pub async fn drawing_tool_update(&self, drawing_tool: DrawingTool) {
        self.drawing_objects_handler
            .drawing_tool_update(drawing_tool)
            .await;
    }

    /// Removes all drawing tools from the strategy.
    pub async fn drawing_tools_remove_all(&self) {
        self.drawing_objects_handler
            .drawing_tools_remove_all()
            .await;
    }

    pub async fn subscriptions(&self) -> Vec<DataSubscription> {
        self.subscription_handler.subscriptions().await
    }

    /// Subscribes to a new subscription, we can only subscribe to a subscription once.
    pub async fn subscribe(&self, subscription: DataSubscription, retain_history: u64) {
        match self
            .subscription_handler
            .subscribe(subscription.clone(), retain_history, self.time_utc().await)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                println!("Error subscribing: {:?}", e);
            }
        }
    }

    /// Unsubscribes from a subscription.
    pub async fn unsubscribe(&self, subscription: DataSubscription) {
        match self
            .subscription_handler
            .unsubscribe(subscription.clone())
            .await
        {
            Ok(_) => {
                self.indicator_handler
                    .indicators_unsubscribe(&subscription)
                    .await
            }
            Err(e) => {
                println!("Error subscribing: {:?}", e);
            }
        }
    }

    /// Sets the subscriptions for the strategy using the subscriptions_closure.
    /// This method is called when the strategy is initialized and can be called at any time to update the subscriptions based on the provided user logic within the closure.
    pub async fn subscriptions_update(
        &self,
        subscriptions: Vec<DataSubscription>,
        retain_history: u64,
    ) {
        let current_subscriptions = self.subscription_handler.subscriptions().await;
        //toDo sort subscriptions so lowest resolution comes first on iter for performance boost later

        // We subscribe to the new subscriptions and unsubscribe from the old ones
        for subscription in &subscriptions {
            if !current_subscriptions.contains(&subscription) {
                match self
                    .subscription_handler
                    .subscribe(subscription.clone(), retain_history, self.time_utc().await)
                    .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error subscribing: {:?}", e);
                    }
                }
            }
        }

        // Unsubscribe from the old subscriptions
        for subscription in current_subscriptions {
            if !subscriptions.contains(&subscription) {
                match self
                    .subscription_handler
                    .unsubscribe(subscription.clone())
                    .await
                {
                    Ok(_) => {
                        self.indicator_handler
                            .indicators_unsubscribe(&subscription)
                            .await
                    }
                    Err(e) => {
                        println!("Error unsubscribing: {:?}", e);
                    }
                }
            }
        }
    }

    /// returns the nth last bar at the specified index. 1 = 1 bar ago, 0 = current bar.
    pub async fn bar_index(
        &self,
        subscription: &DataSubscription,
        index: u64,
    ) -> Option<BaseDataEnum> {
        self.subscription_handler
            .bar_index(subscription, index)
            .await
    }

    pub async fn bar_current(&self, subscription: &DataSubscription) -> Option<BaseDataEnum> {
        self.subscription_handler.bar_current(subscription).await
    }

    /// Current Tz time, depends on the `StrategyMode`. \
    /// Backtest will return the last data point time, live will return the current time.
    pub async fn time_local(&self) -> DateTime<FixedOffset> {
        match self.start_state.mode {
            StrategyMode::Backtest => time_convert_utc_datetime_to_fixed_offset(
                &self.start_state.time_zone,
                self.time_utc().await,
            ),
            _ => time_convert_utc_datetime_to_fixed_offset(&self.start_state.time_zone, Utc::now()),
        }
    }

    /// Current Utc time, depends on the `StrategyMode`. \
    /// Backtest will return the last data point time, live will return the current time.
    pub async fn time_utc(&self) -> DateTime<Utc> {
        match self.start_state.mode {
            StrategyMode::Backtest => self.market_event_handler.last_time().await,
            _ => Utc::now(),
        }
    }

    /// Generates a unique identifier for the owner of the strategy based on the executable's name.
    ///
    /// This function retrieves the name of the currently running executable, which is assumed to be the strategy's name, and uses it as the owner ID. If the executable's name cannot be determined, a default value of "unknown strategy" is used.
    ///
    /// # Returns
    /// A `String` representing the unique identifier for the strategy owner. This is either the name of the executable or "unknown strategy" if the name cannot be determined.
    fn assign_owner_id() -> String {
        let args: Vec<String> = env::args().collect();
        let program_path = &args[0];
        Path::new(program_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown strategy")
            .to_string()
    }

    pub async fn history_from_local_time(
        &self,
        from_time: NaiveDateTime,
        time_zone: Tz,
        subscription: &DataSubscription,
    ) -> BTreeMap<DateTime<Utc>, TimeSlice> {
        let start_date = convert_to_utc(from_time, time_zone);
        range_data(start_date, self.time_utc().await, subscription.clone()).await
    }

    pub async fn history_from_utc_time(
        &self,
        from_time: NaiveDateTime,
        subscription: &DataSubscription,
    ) -> BTreeMap<DateTime<Utc>, TimeSlice> {
        let start_date = DateTime::<Utc>::from_naive_utc_and_offset(from_time, Utc);
        range_data(start_date, self.time_utc().await, subscription.clone()).await
    }

    pub async fn historical_range_from_local_time(
        &self,
        from_time: NaiveDateTime,
        to_time: NaiveDateTime,
        time_zone: Tz,
        subscription: &DataSubscription,
    ) -> BTreeMap<DateTime<Utc>, TimeSlice> {
        let start_date = convert_to_utc(from_time, time_zone.clone());
        let end_date = convert_to_utc(to_time, time_zone);

        let end_date = match end_date > self.time_utc().await {
            true => self.time_utc().await,
            false => end_date,
        };

        range_data(start_date, end_date, subscription.clone()).await
    }

    pub async fn historical_range_from_utc_time(
        &self,
        from_time: NaiveDateTime,
        to_time: NaiveDateTime,
        subscription: &DataSubscription,
    ) -> BTreeMap<DateTime<Utc>, TimeSlice> {
        let start_date = DateTime::<Utc>::from_naive_utc_and_offset(from_time, Utc);
        let end_date = DateTime::<Utc>::from_naive_utc_and_offset(to_time, Utc);

        let end_date = match end_date > self.time_utc().await {
            true => self.time_utc().await,
            false => end_date,
        };

        range_data(start_date, end_date, subscription.clone()).await
    }

    pub async fn get_order_book(&self, symbol: &Symbol) -> Option<Arc<OrderBook>> {
        self.market_event_handler.get_order_book(symbol).await
    }
}
