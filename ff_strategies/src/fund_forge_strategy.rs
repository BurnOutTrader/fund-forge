use std::collections::HashMap;
use std::{env};
use crate::strategy_state::StrategyState;
use tokio::sync::{mpsc};
use std::path::Path;
use std::sync::Arc;
use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc};
use chrono_tz::Tz;
use futures::SinkExt;
use tokio::task;
use ff_charting::drawing_tool_enum::DrawingTool;
use ff_standard_lib::apis::brokerage::Brokerage;
use ff_standard_lib::subscription_handler::SubscriptionHandler;
use ff_standard_lib::helpers::converters::{time_convert_utc_datetime_to_fixed_offset, time_convert_utc_naive_to_fixed_offset};
use ff_standard_lib::server_connections::{initialize_clients, PlatformMode};
use ff_standard_lib::standardized_types::accounts::ledgers::{AccountId};
use ff_standard_lib::standardized_types::enums::{OrderSide, StrategyMode};
use ff_standard_lib::standardized_types::orders::orders::Order;
use ff_standard_lib::standardized_types::OwnerId;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol};
use crate::market_handlers::{HistoricalMarketHandler, MarketHandlerEnum};
use crate::drawing_object_handler::DrawingObjectHandler;
use crate::interaction_handler::InteractionHandler;
use crate::messages::strategy_events::{DataSubscriptionEvent, StrategyEvent, StrategyInteractionMode};


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
    pub(crate) owner_id: OwnerId,

    /// The state of the strategy, used to keep track of the strategy's state by the backend system.
    pub(crate) state: StrategyState,

    /// Any drawing objects associated with the subscription for this strategy.
    /// Drawing objects aren't just Ui objects, they can be interacted with by the engine backend and used for trading signals.
    drawing_objects_handler: DrawingObjectHandler,

    pub(crate) interaction_handler: InteractionHandler,

    pub(crate) market_event_handler: MarketHandlerEnum,

    strategy_event_sender: mpsc::Sender<StrategyEvent>,

    pub(crate) subscription_handler: SubscriptionHandler,
}

impl FundForgeStrategy {

    /// Initializes a new `FundForgeStrategy` instance with the provided parameters.
    ///
    /// This asynchronous function sets up the initial state of the strategy, including its mode, cash balance, start and end dates, time zone, and subscription settings. It creates a message channel for strategy events and returns a handle to the newly created strategy along with the receiver end of the channel.
    pub async fn initialize(
        owner_id: Option<OwnerId>,
        platform_mode: PlatformMode,
        strategy_mode: StrategyMode,
        interaction_mode: StrategyInteractionMode,
        start_date: NaiveDateTime,
        end_date: NaiveDateTime,
        time_zone: Tz,
        warmup_duration: chrono::Duration,
        subscriptions: Vec<DataSubscription>,
        strategy_event_sender: mpsc::Sender<StrategyEvent>,
        replay_delay_ms: Option<u64>,
        retain_history: usize,
    ) -> Arc<FundForgeStrategy> {
        initialize_clients(&platform_mode).await.unwrap();
        let state = StrategyState::new(strategy_mode.clone(), start_date, end_date, time_zone, warmup_duration);
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
            subscription_handler.subscribe(subscription.clone(), retain_history).await.unwrap();
        }
        let strategy = FundForgeStrategy {
            state,
            owner_id,
            drawing_objects_handler: DrawingObjectHandler::new(Default::default()),
            market_event_handler,
            strategy_event_sender,
            interaction_handler: InteractionHandler::new(replay_delay_ms, interaction_mode),
            subscription_handler
        };

        let strategy = Arc::new(strategy);
        let strategy_clone = Arc::clone(&strategy);

        task::spawn(async move {
            FundForgeStrategy::launch(strategy_clone).await;
        });

        strategy
    }

    pub async fn fwd_event(&self, event: StrategyEvent) {
        match self.strategy_event_sender.send(event).await {
            Ok(_) => {},
            Err(e) => {
                println!("Error forwarding event: {:?}", e);
            }
        }
    }

    pub async fn is_warmup_complete(&self) -> bool {
        self.state.is_warmup_complete().await
    }

    pub async fn is_shutdown(&self) -> bool {
        let end_time = self.state.end_date.to_utc();
        if self.time_utc().await >= end_time {
            return true;
        }
        false
    }

    pub async fn enter_long(&self, symbol_name: Symbol, brokerage: Brokerage, quantity: u64, tag: String, account_id: AccountId) {
        let order = Order::enter_long(self.owner_id.clone(), symbol_name, brokerage, quantity, account_id, tag, self.time_utc().await);
        self.market_event_handler.send_order(order).await;
    }

    pub async fn enter_short(&self, symbol_name: Symbol, brokerage: Brokerage, quantity: u64, tag: String, account_id: AccountId) {
        let order = Order::enter_short(self.owner_id.clone(), symbol_name, brokerage, quantity,  account_id, tag, self.time_utc().await);
        self.market_event_handler.send_order(order).await;
    }

    pub async fn exit_long(&self, symbol_name: Symbol, brokerage: Brokerage, quantity: u64, tag: String, account_id: AccountId) {
        let order = Order::exit_long(self.owner_id.clone(), symbol_name, brokerage, quantity, account_id, tag, self.time_utc().await);
        self.market_event_handler.send_order(order).await;
    }

    pub async fn exit_short(&self, symbol_name: Symbol, brokerage: Brokerage, quantity: u64, tag: String, account_id: AccountId) {
        let order = Order::exit_short(self.owner_id.clone(), symbol_name, brokerage, quantity, account_id, tag, self.time_utc().await);
        self.market_event_handler.send_order(order).await;
    }
    //Todo for position pricing support only USD, AUD and convert using historical data.. more can be added later

    pub async fn buy_market(&self, symbol_name: Symbol, brokerage: Brokerage, quantity: u64, tag: String, account_id: AccountId) {
        let order = Order::market_order(self.owner_id.clone(), symbol_name, brokerage, quantity, OrderSide::Buy, account_id, tag, self.time_utc().await);
        self.market_event_handler.send_order(order).await;
    }

    pub async fn sell_market(&self, symbol_name: Symbol, brokerage: Brokerage, quantity: u64, tag: String, account_id: AccountId) {
        let order = Order::market_order(self.owner_id.clone(), symbol_name, brokerage, quantity, OrderSide::Sell, account_id, tag, self.time_utc().await);
        self.market_event_handler.send_order(order).await;
    }

    pub fn time_zone(&self) -> &Tz {
        &self.state.time_zone
    }

    pub fn state(&self) -> &StrategyState {
        &self.state
    }

    pub fn owner_id(&self) -> &OwnerId {
        &self.owner_id
    }

    pub async fn drawing_tools(&self) -> HashMap<DataSubscription, Vec<DrawingTool>> {
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
    /// todo need a way to prevent strategy subscribing to higher res data and always consolidate by default, a as new lower res subscriptions are made the consolidator is replaced with a new one.
    pub async fn subscribe(&self, subscription: DataSubscription, retain_history: usize) {
        match self.subscription_handler.subscribe(subscription.clone(), retain_history).await {
            Ok(_) => {},
            Err(e) => {
                println!("Error subscribing: {:?}", e);
            }
        }
        self.fwd_event(StrategyEvent::DataSubscriptionEvents(self.owner_id.clone(), DataSubscriptionEvent::Subscribed(subscription), self.time_utc().await.timestamp())).await;
        self.subscription_handler.set_subscriptions_updated(true).await;
    }

    /// Unsubscribes from a subscription.
    pub async fn unsubscribe(&self, subscription: DataSubscription) {
        match self.subscription_handler.unsubscribe(subscription.clone()).await {
            Ok(_) => {},
            Err(e) => {
                println!("Error subscribing: {:?}", e);
            }
        }
        self.fwd_event(StrategyEvent::DataSubscriptionEvents(self.owner_id.clone(), DataSubscriptionEvent::Unsubscribed(subscription), self.time_utc().await.timestamp())).await;
        self.subscription_handler.set_subscriptions_updated(true).await;
    }

    /// Sets the subscriptions for the strategy using the subscriptions_closure.
    /// This method is called when the strategy is initialized and can be called at any time to update the subscriptions based on the provided user logic within the closure.
    pub async fn subscriptions_update(&self, subscriptions: Vec<DataSubscription>, retain_history: usize) {
        let current_subscriptions = self.subscription_handler.subscriptions().await;
        // We subscribe to the new subscriptions and unsubscribe from the old ones
        for subscription in &subscriptions {
            if !current_subscriptions.contains(&subscription) {
                match self.subscription_handler.subscribe(subscription.clone(), retain_history).await {
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
                        println!("Error subscribing: {:?}", e);
                    }
                }
            }
        }
    }

    /// Current Tz time, depends on the `StrategyMode`. \
    /// Backtest will return the last data point time, live will return the current time.
    pub async fn time_local(&self) -> DateTime<FixedOffset> {
        match self.state.mode {
            StrategyMode::Backtest => time_convert_utc_datetime_to_fixed_offset(&self.state.time_zone, self.time_utc().await),
            _ => time_convert_utc_datetime_to_fixed_offset(&self.state.time_zone, Utc::now())
        }
    }

    /// Current Utc time, depends on the `StrategyMode`. \
    /// Backtest will return the last data point time, live will return the current time.
    pub async fn time_utc(&self) -> DateTime<Utc> {
        match self.state.mode {
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





