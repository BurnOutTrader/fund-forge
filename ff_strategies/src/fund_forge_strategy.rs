use crate::engine::BackTestEngine;
use ff_standard_lib::interaction_handler::InteractionHandler;
use crate::strategy_state::StrategyStartState;
use ahash::AHashMap;
use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc, Duration as ChronoDuration};
use chrono_tz::Tz;
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
use ff_standard_lib::standardized_types::orders::orders::{Order, OrderId, OrderRequest, ProtectiveOrder};
use ff_standard_lib::standardized_types::rolling_window::RollingWindow;
use ff_standard_lib::standardized_types::strategy_events::{
    EventTimeSlice, StrategyInteractionMode,
};
use ff_standard_lib::standardized_types::subscription_handler::SubscriptionHandler;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, SymbolName};
use ff_standard_lib::standardized_types::time_slices::TimeSlice;
use ff_standard_lib::standardized_types::{Price, Volume};
use ff_standard_lib::timed_events_handler::{TimedEvent, TimedEventHandler};
use std::collections::BTreeMap;
use std::env;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use dashmap::DashMap;
use dashmap::mapref::one::Ref;
use tokio::sync::{mpsc, Notify};
use tokio::sync::mpsc::Sender;
use ff_standard_lib::apis::brokerage::broker_enum::Brokerage;
use ff_standard_lib::market_handler::market_handlers::{MarketHandler};
use ff_standard_lib::server_connections::{init_connections, init_sub_handler, initialize_static, live_order_handler, live_subscription_handler, set_warmup_complete, subscribe_primary_subscription_updates};
use ff_standard_lib::servers::settings::client_settings::initialise_settings;
use ff_standard_lib::standardized_types::data_server_messaging::DataServerRequest;

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
    start_state: StrategyStartState,
    subscription_handler: Arc<SubscriptionHandler>,

    indicator_handler: Arc<IndicatorHandler>,

    market_handler: Arc<MarketHandler>,

    timed_event_handler: Arc<TimedEventHandler>,

    interaction_handler: Arc<InteractionHandler>,

    drawing_objects_handler: Arc<DrawingObjectHandler>,

    orders_count: DashMap<Brokerage, i64>,

    order_sender: Sender<OrderRequest>
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
        notify: Arc<Notify>,
        strategy_mode: StrategyMode,
        interaction_mode: StrategyInteractionMode,
        start_date: NaiveDateTime,
        end_date: NaiveDateTime,
        time_zone: Tz,
        warmup_duration: ChronoDuration,
        subscriptions: Vec<DataSubscription>,
        retain_history: u64,
        strategy_event_sender: mpsc::Sender<EventTimeSlice>,
        replay_delay_ms: Option<u64>,
        buffering_resolution: ChronoDuration,
        gui_enabled: bool
    ) -> FundForgeStrategy {
        let buffering_resolution = Duration::from_secs(buffering_resolution.num_minutes() as u64 * 60);
        let warmup_duration = Duration::from_secs(warmup_duration.num_minutes() as u64 * 60);

        let subscription_handler = SubscriptionHandler::new(strategy_mode).await;
        let subscription_handler = Arc::new(subscription_handler);
        let indicator_handler = Arc::new(IndicatorHandler::new(strategy_mode.clone()).await);
        init_sub_handler(subscription_handler.clone(), strategy_event_sender, indicator_handler.clone()).await;
        init_connections(gui_enabled, buffering_resolution, strategy_mode.clone()).await;

        let start_state = StrategyStartState::new(
            strategy_mode.clone(),
            start_date,
            end_date,
            time_zone.clone(),
            warmup_duration,
            buffering_resolution,
        );
        let start_time = time_convert_utc_naive_to_fixed_offset(&time_zone, start_date);

        subscription_handler.set_subscriptions(subscriptions, retain_history, start_time.to_utc() - warmup_duration).await;


        let (order_sender, order_receiver) = mpsc::channel(100);
        let market_event_handler = match strategy_mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => MarketHandler::new(start_time.to_utc(), Some(order_receiver)).await,
            StrategyMode::Live => {
                live_order_handler(strategy_mode, order_receiver).await;
                MarketHandler::new(start_time.to_utc(), None).await
            },
        };
        let market_event_handler = Arc::new(market_event_handler);


        let timed_event_handler = Arc::new(TimedEventHandler::new());
        let interaction_handler = Arc::new(InteractionHandler::new(replay_delay_ms, interaction_mode));
        let drawing_objects_handler = Arc::new(DrawingObjectHandler::new(AHashMap::new()));
        let strategy = FundForgeStrategy {
            start_state: start_state.clone(),
            market_handler: market_event_handler.clone(),
            subscription_handler,
            indicator_handler: indicator_handler.clone(),
            timed_event_handler: timed_event_handler.clone(),
            interaction_handler: interaction_handler.clone(),
            drawing_objects_handler: drawing_objects_handler.clone(),
            orders_count: Default::default(),
            order_sender,
        };

        initialize_static(
            market_event_handler,
            timed_event_handler,
            interaction_handler,
            drawing_objects_handler
        ).await;

        let (tx, rx) = mpsc::channel(100);
        subscribe_primary_subscription_updates("Live Subscription Update Loop".to_string(), tx).await;
        match strategy_mode {
            StrategyMode::Backtest => {
                let engine = BackTestEngine::new(notify, start_state, gui_enabled.clone(), rx).await;
                BackTestEngine::launch(engine).await;
            }
            StrategyMode::LivePaperTrading | StrategyMode::Live  => {
                live_subscription_handler(strategy_mode, rx).await;
            },
        }
        strategy
    }

    pub async fn is_shutdown(&self) -> bool {
        let end_time = self.start_state.end_date.to_utc();
        if self.time_utc() >= end_time {
            return true;
        }
        false
    }

    pub async fn is_long(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
        self.market_handler.is_long(brokerage, account_id, symbol_name).await
    }

    pub async fn is_flat(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
        self.market_handler.is_flat(brokerage, account_id, symbol_name).await
    }

    pub async fn is_short(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
        self.market_handler.is_short(brokerage, account_id, symbol_name).await
    }

    pub async fn enter_long(
        &self,
        symbol_name: &SymbolName,
        account_id: &AccountId,
        brokerage: &Brokerage,
        quantity: Volume,
        tag: String,
        brackets: Option<Vec<ProtectiveOrder>>
    ) -> OrderId {
        let order_id = format!(
            "ENL{}-{}-{}-{}-{}",
            brokerage,
            account_id,
            symbol_name,
            self.time_utc().timestamp_millis(),
            OrderSide::Buy
        );
        let order = Order::enter_long(
            symbol_name.clone(),
            brokerage.clone(),
            quantity,
            tag,
            account_id.clone(),
            order_id.clone(),
            self.time_utc(),
            brackets
        );
        self.order_sender.send(OrderRequest::Create{ brokerage: order.brokerage.clone(), order}).await.unwrap();
        order_id
    }

    pub async fn enter_short(
        &self,
        symbol_name: &SymbolName,
        account_id: &AccountId,
        brokerage: &Brokerage,
        quantity: Volume,
        tag: String,
        brackets: Option<Vec<ProtectiveOrder>>
    ) -> OrderId {
        let order_id = format!(
            "ENS{}-{}-{}-{}-{}",
            brokerage,
            account_id,
            symbol_name,
            self.time_utc().timestamp_millis(),
            OrderSide::Sell
        );
        let order = Order::enter_short(
            symbol_name.clone(),
            brokerage.clone(),
            quantity,
            tag,
            account_id.clone(),
            order_id.clone(),
            self.time_utc(),
            brackets
        );
        self.order_sender.send(OrderRequest::Create{ brokerage: order.brokerage.clone(), order}).await.unwrap();
        order_id
    }

    pub async fn order_id(
        &self,
        symbol_name: &SymbolName,
        account_id: &AccountId,
        brokerage: &Brokerage,
        side: OrderSide,
    ) -> OrderId {
        let num = match self.orders_count.get_mut(brokerage) {
            None => {
                self.orders_count.insert(brokerage.clone(), 1);
                1
            }
            Some(mut broker_order_number) => {
                *broker_order_number.value_mut() += 1;
                broker_order_number.value().clone()
            }
        };
        format!(
            "{}-{}-{}-{}-{}-{}",
            brokerage,
            account_id,
            symbol_name,
            self.time_utc().timestamp_millis(),
            side,
            num
        )
    }

    pub async fn exit_long(
        &self,
        symbol_name: &SymbolName,
        account_id: &AccountId,
        brokerage: &Brokerage,
        quantity: Volume,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id(symbol_name, account_id, brokerage, OrderSide::Sell).await;
        let order = Order::exit_long(
            symbol_name.clone(),
            brokerage.clone(),
            quantity,
            tag,
            account_id.clone(),
            order_id.clone(),
            self.time_utc(),
        );
        self.order_sender.send(OrderRequest::Create{ brokerage: order.brokerage.clone(), order}).await.unwrap();
        order_id
    }

    pub async fn exit_short(
        &self,
        symbol_name: &SymbolName,
        account_id: &AccountId,
        brokerage: &Brokerage,
        quantity: Volume,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id(symbol_name, account_id, brokerage, OrderSide::Buy).await;
        let order = Order::exit_short(
            symbol_name.clone(),
            brokerage.clone(),
            quantity,
            tag,
            account_id.clone(),
            order_id.clone(),
            self.time_utc(),
        );
        self.order_sender.send(OrderRequest::Create{ brokerage: order.brokerage.clone(), order}).await.unwrap();
        order_id
    }

    pub async fn buy_market(
        &self,
        account_id: &AccountId,
        symbol_name: &SymbolName,
        brokerage: &Brokerage,
        quantity: Volume,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id(symbol_name, account_id, brokerage, OrderSide::Buy).await;
        let order = Order::market_order(
            symbol_name.clone(),
            brokerage.clone(),
            quantity,
            OrderSide::Buy,
            tag,
            account_id.clone(),
            order_id.clone(),
            self.time_utc(),
        );
        self.order_sender.send(OrderRequest::Create{ brokerage: order.brokerage.clone(), order}).await.unwrap();
        order_id
    }

    pub async fn sell_market(
        &self,
        account_id: &AccountId,
        symbol_name: &SymbolName,
        brokerage: &Brokerage,
        quantity: Volume,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id(symbol_name, account_id, brokerage, OrderSide::Sell).await;
        let order = Order::market_order(
            symbol_name.clone(),
            brokerage.clone(),
            quantity,
            OrderSide::Sell,
            tag,
            account_id.clone(),
            order_id.clone(),
            self.time_utc(),
        );
        self.order_sender.send(OrderRequest::Create{ brokerage: order.brokerage.clone(), order}).await.unwrap();
        order_id
    }

    pub async fn cancel_order(&self, brokerage: Brokerage, order_id: OrderId, account_id: AccountId) {
        let cancel_msg =  OrderRequest::Cancel{order_id, brokerage, account_id};
        self.order_sender.send(cancel_msg).await.unwrap()
    }

    pub async fn last_price(&self, symbol_name: &SymbolName) -> Option<Price> {
        self.market_handler.get_last_price(symbol_name).await
    }

    pub async fn update_order(&self, order_id: OrderId, order: Order) {
        let cancel_msg =  OrderRequest::Update {
            brokerage: order.brokerage.clone(),
            order_id,
            order,
        };
        self.order_sender.send(cancel_msg).await.unwrap()
    }

    pub async fn orders_pending(&self) -> Vec<Order> {
        self.market_handler.get_pending_orders().await
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
        //todo, add is_subscribed() for subscription manager so we can auto subscribe for indicators.
        self.indicator_handler
            .add_indicator(indicator, self.time_utc())
            .await
    }

    /// see the indicator_enum.rs for more details
    pub async fn indicator_unsubscribe(&self, name: &IndicatorName) {
        self.indicator_handler.remove_indicator(name).await
    }

    /// see the indicator_enum.rs for more details
    pub async fn indicator_unsubscribe_subscription(&self, subscription: &DataSubscription) {
        self.indicator_handler
            .indicators_unsubscribe_subscription(subscription)
            .await
    }

    /// see the indicator_enum.rs for more details
    pub async fn indicator_index(
        &self,
        name: &IndicatorName,
        index: usize,
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
        self
            .subscription_handler
            .subscribe(subscription.clone(), retain_history, self.time_utc())
            .await
    }

    /// Unsubscribes from a subscription.
    pub async fn unsubscribe(&self, subscription: DataSubscription) {
        self
            .subscription_handler
            .unsubscribe(subscription.clone())
            .await;

        self.indicator_handler
                    .indicators_unsubscribe_subscription(&subscription)
                    .await;
    }

    /// Sets the subscriptions for the strategy using the subscriptions_closure.
    /// This method is called when the strategy is initialized and can be called at any time to update the subscriptions based on the provided user logic within the closure.
    pub async fn subscriptions_update(
        &self,
        subscriptions: Vec<DataSubscription>,
        retain_history: u64,
    ) {
        self.subscription_handler.set_subscriptions(subscriptions, retain_history, self.time_utc()).await;
    }

    /// returns the nth last bar at the specified index. 1 = 1 bar ago, 0 = current bar.
    pub async fn bar_index(
        &self,
        subscription: &DataSubscription,
        index: usize,
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
                self.time_utc(),
            ),
            _ => time_convert_utc_datetime_to_fixed_offset(&self.start_state.time_zone, Utc::now()),
        }
    }

    /// Current Utc time, depends on the `StrategyMode`. \
    /// Backtest will return the last data point time, live will return the current time.
    pub fn time_utc(&self) -> DateTime<Utc> {
        match self.start_state.mode {
            StrategyMode::Backtest => self.market_handler.get_last_time(),
            _ => Utc::now(),
        }
    }

    pub async fn print_ledgers(&self) -> Vec<String> {
        self.market_handler.process_ledgers().await
    }

    pub async fn print_ledger(&self, brokerage: Brokerage, account_id: AccountId) -> Option<String> {
        self.market_handler.print_ledger(brokerage, account_id).await
    }

    pub fn export_trades(&self, folder: &str) {
        self.market_handler.export_trades(folder);
    }

    pub async fn history_from_local_time(
        &self,
        from_time: NaiveDateTime,
        time_zone: Tz,
        subscription: &DataSubscription,
    ) -> BTreeMap<DateTime<Utc>, TimeSlice> {
        let start_date = convert_to_utc(from_time, time_zone);
        range_data(start_date, self.time_utc(), subscription.clone()).await
    }

    pub async fn history_from_utc_time(
        &self,
        from_time: NaiveDateTime,
        subscription: &DataSubscription,
    ) -> BTreeMap<DateTime<Utc>, TimeSlice> {
        let start_date = DateTime::<Utc>::from_naive_utc_and_offset(from_time, Utc);
        range_data(start_date, self.time_utc(), subscription.clone()).await
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

        let end_date = match end_date > self.time_utc() {
            true => self.time_utc(),
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

        let end_date = match end_date > self.time_utc() {
            true => self.time_utc(),
            false => end_date,
        };

        range_data(start_date, end_date, subscription.clone()).await
    }

    pub async fn get_order_book(&self, symbol_name: &SymbolName) -> Option<Arc<OrderBook>> {
        self.market_handler.get_order_book(symbol_name).await
    }
}
