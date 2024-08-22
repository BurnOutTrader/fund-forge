use std::{env};
use crate::strategy_state::StrategyStartState;
use tokio::sync::{mpsc, Notify, RwLock};
use std::path::Path;
use std::sync::Arc;
use ahash::AHashMap;
use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc};
use chrono_tz::Tz;
use ff_charting::drawing_tool_enum::DrawingTool;
use ff_standard_lib::apis::brokerage::Brokerage;
use ff_standard_lib::subscription_handler::{SubscriptionHandler};
use ff_standard_lib::helpers::converters::{time_convert_utc_datetime_to_fixed_offset, time_convert_utc_naive_to_fixed_offset};
use ff_standard_lib::history_handler::HistoryHandler;
use ff_standard_lib::server_connections::{initialize_clients, PlatformMode};
use ff_standard_lib::standardized_types::accounts::ledgers::{AccountId};
use ff_standard_lib::standardized_types::enums::{OrderSide, StrategyMode};
use ff_standard_lib::standardized_types::orders::orders::Order;
use ff_standard_lib::standardized_types::OwnerId;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol};
use crate::market_handlers::{MarketHandlerEnum};
use crate::drawing_object_handler::DrawingObjectHandler;
use crate::engine::Engine;
use crate::interaction_handler::InteractionHandler;
use crate::messages::strategy_events::{EventTimeSlice, StrategyInteractionMode};

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

    history: Arc<RwLock<HistoryHandler>>, //todo use this object to save history from timeslices
}

impl FundForgeStrategy {

    /// Initializes a new `FundForgeStrategy` instance with the provided parameters.
    ///
    /// This asynchronous function sets up the initial state of the strategy, including its mode, cash balance, start and end dates, time zone, and subscription settings. It creates a message channel for strategy events and returns a handle to the newly created strategy along with the receiver end of the channel.
    pub async fn initialize(
        owner_id: Option<OwnerId>,
        notify: Arc<Notify>,
        platform_mode: PlatformMode,
        strategy_mode: StrategyMode,
        interaction_mode: StrategyInteractionMode,
        start_date: NaiveDateTime,
        end_date: NaiveDateTime,
        time_zone: Tz,
        warmup_duration: chrono::Duration,
        subscriptions: Vec<DataSubscription>,
        strategy_event_sender: mpsc::Sender<EventTimeSlice>,
        replay_delay_ms: Option<u64>,
        retain_history: usize,
    ) -> FundForgeStrategy {
        initialize_clients(&platform_mode).await.unwrap();
        let start_state = StrategyStartState::new(strategy_mode.clone(), start_date, end_date, time_zone.clone(), warmup_duration);
        let start_time = time_convert_utc_naive_to_fixed_offset(&time_zone, start_date);
        let owner_id = match owner_id {
            Some(owner_id) => owner_id,
            None => FundForgeStrategy::assign_owner_id(),
        };

        let market_event_handler = match strategy_mode {
            StrategyMode::Backtest => MarketHandlerEnum::new(owner_id.clone(), start_time.to_utc(), strategy_mode.clone()),
            StrategyMode::Live => panic!("Live mode not yet implemented"),
            StrategyMode::LivePaperTrading => panic!("Live paper mode not yet implemented")
        };

        let subscription_handler = SubscriptionHandler::new().await;
        for subscription in subscriptions {
            subscription_handler.subscribe(subscription.clone()).await.unwrap();
        }

        let strategy = FundForgeStrategy {
            start_state: start_state.clone(),
            owner_id,
            drawing_objects_handler: DrawingObjectHandler::new(Default::default()),
            market_event_handler: Arc::new(market_event_handler),
            interaction_handler: Arc::new(InteractionHandler::new(replay_delay_ms, interaction_mode)),
            subscription_handler: Arc::new(subscription_handler),
            history: Arc::new(RwLock::new(HistoryHandler::new(retain_history))),
        };

        let engine = Engine::new(strategy.owner_id.clone(), notify, start_state, strategy_event_sender.clone(), strategy.subscription_handler.clone(), strategy.market_event_handler.clone(), strategy.interaction_handler.clone());
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

    pub async fn enter_long(&self, account_id: AccountId, symbol_name: Symbol, brokerage: Brokerage, quantity: u64, tag: String) {
        let order = Order::enter_long(self.owner_id.clone(), symbol_name, brokerage, quantity, tag, account_id, self.time_utc().await);
        self.market_event_handler.send_order(order).await;
    }

    pub async fn enter_short(&self, account_id: AccountId, symbol_name: Symbol, brokerage: Brokerage, quantity: u64, tag: String) {
        let order = Order::enter_short(self.owner_id.clone(), symbol_name, brokerage, quantity,  tag, account_id, self.time_utc().await);
        self.market_event_handler.send_order(order).await;
    }

    pub async fn exit_long(&self, account_id: AccountId, symbol_name: Symbol, brokerage: Brokerage, quantity: u64, tag: String) {
        let order = Order::exit_long(self.owner_id.clone(), symbol_name, brokerage, quantity, tag, account_id, self.time_utc().await);
        self.market_event_handler.send_order(order).await;
    }

    pub async fn exit_short(&self, account_id: AccountId, symbol_name: Symbol, brokerage: Brokerage, quantity: u64, tag: String) {
        let order = Order::exit_short(self.owner_id.clone(), symbol_name, brokerage, quantity, tag, account_id, self.time_utc().await);
        self.market_event_handler.send_order(order).await;
    }

    pub async fn buy_market(&self, account_id: AccountId, symbol_name: Symbol, brokerage: Brokerage, quantity: u64, tag: String) {
        let order = Order::market_order(self.owner_id.clone(), symbol_name, brokerage, quantity, OrderSide::Buy, tag, account_id, self.time_utc().await);
        self.market_event_handler.send_order(order).await;
    }

    pub async fn sell_market(&self, account_id: AccountId, symbol_name: Symbol, brokerage: Brokerage, quantity: u64, tag: String) {
        let order = Order::market_order(self.owner_id.clone(), symbol_name, brokerage, quantity, OrderSide::Sell, tag, account_id, self.time_utc().await);
        self.market_event_handler.send_order(order).await;
    }

    pub fn time_zone(&self) -> &Tz {
        &self.start_state.time_zone
    }

    pub fn state(&self) -> &StrategyStartState {
        &self.start_state
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
        self.drawing_objects_handler.drawing_tool_add(drawing_tool).await;
    }
    
    /// Removes a drawing tool from the strategy.
    /// Drawing objects aren't just Ui objects, they can be interacted with by the engine backend and used for trading signals.
    /// # Arguments
    /// * `drawing_tool` - The drawing tool to remove from the strategy.
    pub async fn drawing_tool_remove(&self, drawing_tool: DrawingTool) {
        self.drawing_objects_handler.drawing_tool_remove(drawing_tool).await;
    }
    
    /// Updates a drawing tool in the strategy.
    pub async fn drawing_tool_update(&self, drawing_tool: DrawingTool) {
        self.drawing_objects_handler.drawing_tool_update(drawing_tool).await;
    }
    
    /// Removes all drawing tools from the strategy.
    pub async fn drawing_tools_remove_all(&self) {
        self.drawing_objects_handler.drawing_tools_remove_all().await;
    }
    
    pub async fn subscriptions(&self) -> Vec<DataSubscription> {
        self.subscription_handler.subscriptions().await
    }

    /// Subscribes to a new subscription, we can only subscribe to a subscription once.
    pub async fn subscribe(&self, subscription: DataSubscription, _retain_history: usize) {
        match self.subscription_handler.subscribe(subscription.clone()).await {
            Ok(_) => {},
            Err(e) => {
                println!("Error subscribing: {:?}", e);
            }
        }
        //self.fwd_event(vec![StrategyEvent::DataSubscriptionEvents(self.owner_id.clone(), DataSubscriptionEvent::Subscribed(subscription), self.time_utc().await.timestamp())]).await;
    }

    /// Unsubscribes from a subscription.
    pub async fn unsubscribe(&self, subscription: DataSubscription) {
        match self.subscription_handler.unsubscribe(subscription.clone()).await {
            Ok(_) => {},
            Err(e) => {
                println!("Error subscribing: {:?}", e);
            }
        }
        // should be sent from the subscription handler self.fwd_event(vec![StrategyEvent::DataSubscriptionEvents(self.owner_id.clone(), DataSubscriptionEvent::Unsubscribed(subscription), self.time_utc().await.timestamp())]).await;
    }

    /// Sets the subscriptions for the strategy using the subscriptions_closure.
    /// This method is called when the strategy is initialized and can be called at any time to update the subscriptions based on the provided user logic within the closure.
    pub async fn subscriptions_update(&self, subscriptions: Vec<DataSubscription>, _retain_history: usize) {
        let current_subscriptions = self.subscription_handler.subscriptions().await;
        //toDo sort subscriptions so lowest resolution comes first on iter for performance boost later

        // We subscribe to the new subscriptions and unsubscribe from the old ones
        for subscription in &subscriptions {
            if !current_subscriptions.contains(&subscription) {
                match self.subscription_handler.subscribe(subscription.clone()).await {
                    Ok(_) => {},
                    Err(e) => {
                        println!("Error subscribing: {:?}", e);
                    }
                }
            }
        }

        // Unsubscribe from the old subscriptions
        for subscription in current_subscriptions {
            if !subscriptions.contains(&subscription) {
                match self.subscription_handler.unsubscribe(subscription.clone()).await {
                    Ok(_) => {},
                    Err(e) => {
                        println!("Error unsubscribing: {:?}", e);
                    }
                }
            }
        }
    }
/*
    /// returns the nth last bar at the specified index. 1 = 1 bar ago, 0 = current bar.
    pub async fn data_index(&self, subscription: &DataSubscription, index: usize) -> Option<BaseDataEnum> {
        self.subscription_handler.data_index(subscription, index).await
        //Todo Data is being recieved in the above function... but not received here
    }

    pub async fn data_current(&self, subscription: &DataSubscription) -> Option<BaseDataEnum> {
        self.subscription_handler.data_current(subscription).await
        //Todo Data is being recieved in the above function... but not received here
    }*/

    /// Current Tz time, depends on the `StrategyMode`. \
    /// Backtest will return the last data point time, live will return the current time.
    pub async fn time_local(&self) -> DateTime<FixedOffset> {
        match self.start_state.mode {
            StrategyMode::Backtest => time_convert_utc_datetime_to_fixed_offset(&self.start_state.time_zone, self.time_utc().await),
            _ => time_convert_utc_datetime_to_fixed_offset(&self.start_state.time_zone, Utc::now())
        }
    }

    /// Current Utc time, depends on the `StrategyMode`. \
    /// Backtest will return the last data point time, live will return the current time.
    pub async fn time_utc(&self) -> DateTime<Utc> {
        match self.start_state.mode {
            StrategyMode::Backtest => self.market_event_handler.last_time().await,
            _ => Utc::now()
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
            .unwrap_or("unknown strategy").to_string()
    }
 }





