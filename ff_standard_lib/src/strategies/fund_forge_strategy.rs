use ahash::AHashMap;
use chrono::{DateTime, Duration as ChronoDuration, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use crate::strategies::handlers::drawing_object_handler::DrawingObjectHandler;
use crate::gui_types::drawing_objects::drawing_tool_enum::DrawingTool;
use crate::strategies::indicators::indicator_enum::IndicatorEnum;
use crate::strategies::indicators::indicator_handler::IndicatorHandler;
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};
use crate::strategies::indicators::values::IndicatorValues;
use crate::strategies::ledgers::{AccountId, Currency};
use crate::standardized_types::base_data::history::range_data;
use crate::standardized_types::enums::{OrderSide, StrategyMode};
use crate::standardized_types::rolling_window::RollingWindow;
use crate::strategies::strategy_events::StrategyEventBuffer;
use crate::strategies::handlers::subscription_handler::SubscriptionHandler;
use crate::standardized_types::subscriptions::{DataSubscription, DataSubscriptionEvent, SymbolName};
use crate::standardized_types::time_slices::TimeSlice;
use crate::strategies::handlers::timed_events_handler::{TimedEvent, TimedEventHandler};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use dashmap::DashMap;
use rust_decimal::Decimal;
use tokio::sync::{mpsc, RwLock};
use tokio::sync::mpsc::Sender;
use crate::standardized_types::broker_enum::Brokerage;
use crate::strategies::handlers::market_handlers::{export_trades, get_market_fill_price_estimate, get_market_price, is_flat_live, is_flat_paper, is_long_live, is_long_paper, is_short_live, is_short_paper, market_handler, print_ledger, process_ledgers, MarketMessageEnum, BACKTEST_OPEN_ORDER_CACHE, LAST_PRICE, LIVE_ORDER_CACHE};
use crate::client_features::server_connections::{get_backtest_time, init_connections, init_sub_handler, initialize_static, live_subscription_handler, update_historical_timestamp};
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::base_data::quote::Quote;
use crate::standardized_types::base_data::quotebar::QuoteBar;
use crate::standardized_types::base_data::tick::Tick;
use crate::messages::data_server_messaging::FundForgeError;
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::orders::{Order, OrderId, OrderRequest, OrderUpdateType, TimeInForce};
use crate::strategies::historical_engine::HistoricalEngine;

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
#[allow(dead_code)]
pub struct FundForgeStrategy {
    mode: StrategyMode,

    time_zone: Tz,

    buffer_resolution: Option<Duration>,

    subscription_handler: Arc<SubscriptionHandler>,

    indicator_handler: Arc<IndicatorHandler>,

    timed_event_handler: Arc<TimedEventHandler>,

    drawing_objects_handler: Arc<DrawingObjectHandler>,

    orders_count: DashMap<Brokerage, i64>,

    market_event_sender: Sender<MarketMessageEnum>,

    orders: RwLock<AHashMap<OrderId, (Brokerage, AccountId)>>,
}

impl FundForgeStrategy {
    /// Initializes a new `FundForgeStrategy` instance with the provided parameters.
    ///
    /// # Arguments
    /// `notify: Arc<Notify>`: The notification mechanism for the strategy, this is useful to slow the message sender channel until we have processed the last message. \
    /// `strategy_mode: StrategyMode`: The mode of the strategy (Backtest, Live, LivePaperTrading). \
    /// `interaction_mode: StrategyInteractionMode`: The interaction mode for the strategy. SemiAutomated allows the Gui to send and modify drawing tools \
    /// `start_date: NaiveDateTime`: The start date of the strategy. In the local time_zone that you pass in. \
    /// `end_date: NaiveDateTime`: The end date of the strategy. In the local time_zone that you pass in\
    /// `time_zone: Tz`: The time zone of the strategy, you can use Utc for default. \
    /// `warmup_duration: chrono::Duration`: The warmup duration for the strategy. \
    /// `subscriptions: Vec<DataSubscription>`: The initial data subscriptions for the strategy. \
    /// `fill_forward: bool`: If true we will fill forward with flat bars based on the last close when there is no data, this is only for consolidated data and applies to the initial subscriptions. \
    /// `retain_history: usize`: The number of bars to retain in memory for the strategy. This is useful for strategies that need to reference previous bars for calculations, this is only for our initial subscriptions. \
    /// `strategy_event_sender: mpsc::Sender<EventTimeSlice>`: The sender for strategy events. \
    /// `replay_delay_ms: Option<u64>`: The delay in milliseconds between time slices for market replay style backtesting. \
    ///  any additional subscriptions added later will be able to specify their own history requirements.
    /// `buffering_resolution: u64`: The buffering resolution of the strategy in milliseconds. If we are backtesting, any data of a lower granularity will be consolidated into a single time slice.
    /// If out base data source is tick data, but we are trading only on 15min bars, then we can just consolidate the tick data and ignore it in on_data_received().
    /// In live trading our strategy will capture the tick stream in a buffer and pass it to the strategy in the correct resolution/durations, this helps to prevent spamming our on_data_received() fn.
    /// If we don't need to make strategy decisions on every tick, we can just consolidate the tick stream into buffered time slice events.
    /// This also helps us get consistent results between backtesting and live trading.
    /// If 0 then it will default to a 1-millisecond buffer.
    /// `gui_enabled: bool`: If true the engine will forward all StrategyEventSlice's sent to the strategy, to the strategy registry so they can be used by GUI implementations.
    pub async fn initialize(
        strategy_mode: StrategyMode,
        backtest_accounts_starting_cash: Decimal,
        backtest_account_currency: Currency,
        start_date: NaiveDateTime,
        end_date: NaiveDateTime,
        time_zone: Tz,
        warmup_duration: ChronoDuration,
        subscriptions: Vec<DataSubscription>,
        fill_forward: bool,
        retain_history: usize,
        strategy_event_sender: mpsc::Sender<StrategyEventBuffer>,
        buffering_duration: Option<Duration>,
        gui_enabled: bool
    ) -> FundForgeStrategy {
        let start_time = time_zone.from_local_datetime(&start_date).unwrap().to_utc();
        let end_time = time_zone.from_local_datetime(&end_date).unwrap().to_utc();
        let warm_up_start_time = start_time - warmup_duration;
        update_historical_timestamp(warm_up_start_time.clone());
        let is_buffered = match buffering_duration {
            None => false,
            Some(_) => true
        };
        let market_event_sender = market_handler(strategy_mode, backtest_accounts_starting_cash, backtest_account_currency, is_buffered).await;
        let subscription_handler = Arc::new(SubscriptionHandler::new(strategy_mode, market_event_sender.clone()).await);
        let indicator_handler = Arc::new(IndicatorHandler::new(strategy_mode.clone()).await);

        init_sub_handler(subscription_handler.clone(), strategy_event_sender, indicator_handler.clone()).await;
        init_connections(gui_enabled, buffering_duration.clone(), strategy_mode.clone(), market_event_sender.clone()).await;


        subscription_handler.set_subscriptions(subscriptions, retain_history, warm_up_start_time.clone(), fill_forward, false).await;

        //todo There is a problem with quote bars not being produced consistently since refactoring



        let timed_event_handler = Arc::new(TimedEventHandler::new());
        let drawing_objects_handler = Arc::new(DrawingObjectHandler::new(AHashMap::new()));

        let strategy = FundForgeStrategy {
            mode: strategy_mode.clone(),
            buffer_resolution: buffering_duration.clone(),
            time_zone,
            subscription_handler,
            indicator_handler: indicator_handler.clone(),
            timed_event_handler: timed_event_handler.clone(),
            drawing_objects_handler: drawing_objects_handler.clone(),
            orders_count: Default::default(),
            market_event_sender: market_event_sender.clone(),
            orders: Default::default()
        };

        initialize_static(
            timed_event_handler,
            drawing_objects_handler
        ).await;

        match strategy_mode {
            StrategyMode::Backtest => {
                let engine = HistoricalEngine::new(strategy_mode.clone(), start_time.to_utc(),  end_time.to_utc(), warmup_duration.clone(), buffering_duration.clone(), gui_enabled.clone(), market_event_sender).await;
                HistoricalEngine::launch(engine).await;
            }
            StrategyMode::LivePaperTrading | StrategyMode::Live  => {
                live_subscription_handler(strategy_mode.clone()).await;
            },
        }
        strategy
    }

    pub async fn get_market_fill_price_estimate (
        &self,
        order_side: OrderSide,
        symbol_name: &SymbolName,
        volume: Volume,
        brokerage: Brokerage
    ) -> Result<Price, FundForgeError> {
        get_market_fill_price_estimate(order_side, symbol_name, volume, brokerage).await
    }

    pub async fn get_market_price (
        order_side: OrderSide,
        symbol_name: &SymbolName,
    ) -> Result<Price, FundForgeError> {
        get_market_price(order_side, symbol_name).await
    }

    pub fn is_long(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
        match self.mode {
            StrategyMode::Backtest| StrategyMode::LivePaperTrading => is_long_paper(brokerage, account_id, symbol_name),
            StrategyMode::Live  => is_long_live(brokerage, account_id, symbol_name)
        }
    }

    pub fn is_flat(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
        match self.mode {
            StrategyMode::Backtest| StrategyMode::LivePaperTrading => is_flat_paper(brokerage, account_id, symbol_name),
            StrategyMode::Live  => is_flat_live(brokerage, account_id, symbol_name)
        }
    }

    pub fn is_short(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
        match self.mode {
            StrategyMode::Backtest| StrategyMode::LivePaperTrading => is_short_paper(brokerage, account_id, symbol_name),
            StrategyMode::Live => is_short_live(brokerage, account_id, symbol_name)
        }
    }

    pub async fn order_id(
        &self,
        symbol_name: &SymbolName,
        account_id: &AccountId,
        brokerage: &Brokerage,
        order_string: &str
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
            "{}: {}:{}, {}, {}",
            order_string,
            brokerage,
            account_id,
            symbol_name,
            num
        )
    }

    pub async fn enter_long(
        &self,
        symbol_name: &SymbolName,
        account_id: &AccountId,
        brokerage: &Brokerage,
        quantity: Volume,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id(symbol_name, account_id, brokerage, &"Enter Long").await;
        let order = Order::enter_long(
            symbol_name.clone(),
            brokerage.clone(),
            quantity,
            tag,
            account_id.clone(),
            order_id.clone(),
            self.time_utc(),
        );
        let market_request = MarketMessageEnum::OrderRequest(OrderRequest::Create{ brokerage: order.brokerage.clone(), order});
        self.market_event_sender.send(market_request).await.unwrap();
        self.orders.write().await.insert(order_id.clone(), (brokerage.clone(), account_id.clone()));
        order_id
    }

    pub async fn enter_short(
        &self,
        symbol_name: &SymbolName,
        account_id: &AccountId,
        brokerage: &Brokerage,
        quantity: Volume,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id(symbol_name, account_id, brokerage, &"Enter Short").await;
        let order = Order::enter_short(
            symbol_name.clone(),
            brokerage.clone(),
            quantity,
            tag,
            account_id.clone(),
            order_id.clone(),
            self.time_utc(),
        );
        let request = MarketMessageEnum::OrderRequest(OrderRequest::Create{ brokerage: order.brokerage.clone(), order});
        self.market_event_sender.send(request).await.unwrap();
        self.orders.write().await.insert(order_id.clone(), (brokerage.clone(), account_id.clone()));
        order_id
    }

    pub async fn exit_long(
        &self,
        symbol_name: &SymbolName,
        account_id: &AccountId,
        brokerage: &Brokerage,
        quantity: Volume,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id(symbol_name, account_id, brokerage, &"Exit Long").await;
        let order = Order::exit_long(
            symbol_name.clone(),
            brokerage.clone(),
            quantity,
            tag,
            account_id.clone(),
            order_id.clone(),
            self.time_utc(),
        );
        let request = MarketMessageEnum::OrderRequest(OrderRequest::Create{ brokerage: order.brokerage.clone(), order});
        self.market_event_sender.send(request).await.unwrap();
        self.orders.write().await.insert(order_id.clone(), (brokerage.clone(), account_id.clone()));
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
        let order_id = self.order_id(symbol_name, account_id, brokerage, &"Exit Short").await;
        let order = Order::exit_short(
            symbol_name.clone(),
            brokerage.clone(),
            quantity,
            tag,
            account_id.clone(),
            order_id.clone(),
            self.time_utc(),
        );
        let request =MarketMessageEnum::OrderRequest(OrderRequest::Create{ brokerage: order.brokerage.clone(), order});
        self.market_event_sender.send(request).await.unwrap();
        self.orders.write().await.insert(order_id.clone(), (brokerage.clone(), account_id.clone()));
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
        let order_id = self.order_id(symbol_name, account_id, brokerage, &"Buy Market").await;
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
        let request =MarketMessageEnum::OrderRequest(OrderRequest::Create{ brokerage: order.brokerage.clone(), order});
        self.market_event_sender.send(request).await.unwrap();
        self.orders.write().await.insert(order_id.clone(), (brokerage.clone(), account_id.clone()));
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
        let order_id = self.order_id(symbol_name, account_id, brokerage, &"Sell Market").await;
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
        let request =MarketMessageEnum::OrderRequest(OrderRequest::Create{ brokerage: order.brokerage.clone(), order});
        self.market_event_sender.send(request).await.unwrap();
        self.orders.write().await.insert(order_id.clone(), (brokerage.clone(), account_id.clone()));
        order_id
    }

    pub async fn limit_order(
        &self,
        account_id: &AccountId,
        symbol_name: &SymbolName,
        brokerage: &Brokerage,
        quantity: Volume,
        side: OrderSide,
        limit_price: Price,
        tif: TimeInForce,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id(symbol_name, account_id, brokerage, &format!("{} Limit", side)).await;
        let order = Order::limit_order(symbol_name.clone(), brokerage.clone(), quantity, side, tag, account_id.clone(), order_id.clone(), self.time_utc(), limit_price, tif);
        let request =MarketMessageEnum::OrderRequest(OrderRequest::Create{ brokerage: order.brokerage.clone(), order});
        self.market_event_sender.send(request).await.unwrap();
        self.orders.write().await.insert(order_id.clone(), (brokerage.clone(), account_id.clone()));
        order_id
    }

    pub async fn market_if_touched (
        &self,
        account_id: &AccountId,
        symbol_name: &SymbolName,
        brokerage: &Brokerage,
        quantity: Volume,
        side: OrderSide,
        trigger_price: Price,
        tif: TimeInForce,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id(&symbol_name, account_id, brokerage, &format!("{} MIT", side)).await;
        let order = Order::market_if_touched(symbol_name.clone(), brokerage.clone(), quantity, side, tag, account_id.clone(), order_id.clone(), self.time_utc(),trigger_price, tif);
        let request =MarketMessageEnum::OrderRequest(OrderRequest::Create{ brokerage: order.brokerage.clone(), order});
        self.market_event_sender.send(request).await.unwrap();
        self.orders.write().await.insert(order_id.clone(), (brokerage.clone(), account_id.clone()));
        order_id
    }

    pub async fn stop_order (
        &self,
        account_id: &AccountId,
        symbol_name: &SymbolName,
        brokerage: &Brokerage,
        quantity: Volume,
        side: OrderSide,
        trigger_price: Price,
        tif: TimeInForce,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id(symbol_name, account_id, brokerage, &format!("{} Stop", side)).await;
        let order = Order::stop(symbol_name.clone(), brokerage.clone(), quantity, side, tag, account_id.clone(), order_id.clone(), self.time_utc(),trigger_price, tif);
        let request =MarketMessageEnum::OrderRequest(OrderRequest::Create{ brokerage: order.brokerage.clone(), order});
        self.market_event_sender.send(request).await.unwrap();
        self.orders.write().await.insert(order_id.clone(), (brokerage.clone(), account_id.clone()));
        order_id
    }

    pub async fn stop_limit (
        &self,
        account_id: &AccountId,
        symbol_name: &SymbolName,
        brokerage: &Brokerage,
        quantity: Volume,
        side: OrderSide,
        tag: String,
        limit_price: Price,
        trigger_price: Price,
        tif: TimeInForce
    ) -> OrderId {
        let order_id = self.order_id(symbol_name, account_id, brokerage, &format!("{} Stop Limit", side)).await;
        let order = Order::stop_limit(symbol_name.clone(), brokerage.clone(), quantity, side, tag, account_id.clone(), order_id.clone(), self.time_utc(),limit_price, trigger_price, tif);
        let request = MarketMessageEnum::OrderRequest(OrderRequest::Create{ brokerage: order.brokerage.clone(), order});
        self.market_event_sender.send(request).await.unwrap();
        self.orders.write().await.insert(order_id.clone(), (brokerage.clone(), account_id.clone()));
        order_id
    }

    pub async fn cancel_order(&self, order_id: OrderId) {
        if let Some((brokerage, account_id)) = self.orders.read().await.get(&order_id) {
            let request = MarketMessageEnum::OrderRequest(OrderRequest::Cancel { order_id, brokerage: brokerage.clone(), account_id: account_id.clone() });
            self.market_event_sender.send(request).await.unwrap();
        }
    }

    pub async fn update_order(&self, order_id: OrderId, order_update_type: OrderUpdateType) {
        if let Some((brokerage, account_id)) = self.orders.read().await.get(&order_id) {
            let update_msg =  MarketMessageEnum::OrderRequest(OrderRequest::Update {
                brokerage: brokerage.clone(),
                order_id,
                account_id: account_id.clone(),
                update: order_update_type,
            });
            self.market_event_sender.send(update_msg).await.unwrap();
        }
    }

    pub async fn cancel_orders(&self, brokerage: Brokerage, account_id: AccountId, symbol_name: SymbolName) {
        let cancel_msg =  MarketMessageEnum::OrderRequest(OrderRequest::CancelAll{brokerage, account_id, symbol_name});
        self.market_event_sender.send(cancel_msg).await.unwrap();
    }

    pub async fn last_price(&self, symbol_name: &SymbolName) -> Option<Price> {
        match LAST_PRICE.get(symbol_name) {
            None => None,
            Some(price) => Some(price.clone())
        }
    }

    pub async fn orders_pending(&self) -> Arc<DashMap<OrderId, Order>> {
        //todo make read only ref
        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => BACKTEST_OPEN_ORDER_CACHE.clone(),
            StrategyMode::Live => LIVE_ORDER_CACHE.clone()
        }
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
    /// If we subscribe to an indicator and we do not have the appropriate data subscription, we will also subscribe to the data subscription.
    pub async fn subscribe_indicator(&self, indicator: IndicatorEnum, auto_subscribe: bool) {
        let subscriptions = self.subscriptions().await;
        if !subscriptions.contains(&indicator.subscription()) {
            match auto_subscribe {
                true => {
                    let result = self.subscribe(indicator.subscription(), (indicator.data_required_warmup() + 1) as usize, false).await;
                    match result {
                        Ok(sub_result) => println!("{}", sub_result),
                        Err(sub_result) =>  eprintln!("{}", sub_result),
                    }
                }
                false => panic!("You have no subscription: {}, for the indicator subscription {} and AutoSubscribe is not enabled", indicator.subscription(), indicator.name())
            }
        }
        self.indicator_handler
            .add_indicator(indicator, self.time_utc())
            .await
    }

    /// see the indicator_enum.rs for more details
    pub async fn indicator_unsubscribe(&self, name: &IndicatorName) {
        self.indicator_handler.remove_indicator(self.time_utc(),name).await
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
        &self.time_zone
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
    pub async fn subscribe(&self, subscription: DataSubscription, history_to_retain: usize, fill_forward: bool) -> Result<DataSubscriptionEvent, DataSubscriptionEvent> {
        let result = self.subscription_handler
            .subscribe(subscription.clone(), self.time_utc(), fill_forward, history_to_retain, true)
            .await;

        match &result {
            Ok(sub_result) => println!("{}", sub_result),
            Err(sub_result) =>  eprintln!("{}", sub_result),
        }

        result
    }

    /// Unsubscribes from a subscription.
    pub async fn unsubscribe(&self,subscription: DataSubscription) {
        self.subscription_handler
            .unsubscribe(self.time_utc(), subscription.clone(), true)
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
        retain_to_history: usize,
        fill_forward: bool
    ) {
        for subscription in &subscriptions {
            if let Some(buffer) = self.buffer_resolution {
                if subscription.resolution.as_nanos() < buffer.as_nanos() as i64 {
                    panic!("Subscription Resolution: {}, Lower than strategy buffer resolution: {:?}", subscription.resolution, self.buffer_resolution)
                }
            }
        }
        self.subscription_handler.set_subscriptions(subscriptions, retain_to_history, self.time_utc(), fill_forward, true).await;
    }

    pub fn open_bar(&self, subscription: &DataSubscription) -> Option<QuoteBar> {
        self.subscription_handler.open_bar(subscription)
    }

    pub fn open_candle(&self, subscription: &DataSubscription) -> Option<Candle> {
        self.subscription_handler.open_candle(subscription)
    }

    pub fn candle_index(&self, subscription: &DataSubscription, index: usize) -> Option<Candle> {
        self.subscription_handler.candle_index(subscription, index)
    }

    pub fn bar_index(&self, subscription: &DataSubscription, index: usize) -> Option<QuoteBar> {
        self.subscription_handler.bar_index(subscription, index)
    }

    pub fn tick_index(&self, subscription: &DataSubscription, index: usize) -> Option<Tick> {
        self.subscription_handler.tick_index(subscription, index)
    }

    pub fn quote_index(&self, subscription: &DataSubscription, index: usize) -> Option<Quote> {
        self.subscription_handler.quote_index(subscription, index)
    }

    /// Current Tz time, depends on the `StrategyMode`. \
    /// Backtest will return the last data point time, live will return the current time.
    pub fn time_local(&self) -> DateTime<Tz> {
        self.time_zone.from_utc_datetime(&self.time_utc().naive_utc())
    }

    /// Get back the strategy time as the passed in timezone
    pub fn time_from_tz(&self, time_zone: Tz) -> DateTime<Tz> {
        time_zone.from_utc_datetime(&self.time_utc().naive_utc())
    }

    /// Current Utc time, depends on the `StrategyMode`. \
    /// Backtest will return the last data point time, live will return the current time.
    pub fn time_utc(&self) -> DateTime<Utc> {
        match self.mode {
            StrategyMode::Backtest => get_backtest_time(),
            _ => Utc::now(),
        }
    }

    pub fn print_ledger(&self, brokerage: Brokerage, account_id: &AccountId) {
        if let Some(ledger_string) = print_ledger(brokerage, account_id) {
            println!("{}", ledger_string);
        }
    }

    pub fn print_ledgers(&self) {
        let strings = process_ledgers();
        for string in strings {
            println!("{}", string);
        }
    }

    pub fn export_trades(&self, directory: &str) {
        export_trades(directory);
    }

    pub async fn history_from_local_time(
        &self,
        from_time: NaiveDateTime,
        time_zone: Tz,
        subscription: &DataSubscription,
    ) -> BTreeMap<DateTime<Utc>, TimeSlice> {
        let start_date = time_zone.from_utc_datetime(&from_time);
        range_data(start_date.to_utc(), self.time_utc(), subscription.clone()).await
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
        let start_date = time_zone.from_utc_datetime(&from_time);
        let end_date =time_zone.from_utc_datetime(&to_time);

        let end_date = match end_date > self.time_utc() {
            true => self.time_utc(),
            false => end_date.to_utc(),
        };

        range_data(start_date.to_utc(), end_date, subscription.clone()).await
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
}
