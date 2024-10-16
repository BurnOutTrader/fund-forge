use ahash::AHashMap;
use chrono::{DateTime, Duration as ChronoDuration, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use crate::strategies::handlers::drawing_object_handler::DrawingObjectHandler;
use crate::gui_types::drawing_objects::drawing_tool_enum::DrawingTool;
use crate::strategies::indicators::indicator_enum::IndicatorEnum;
use crate::strategies::handlers::indicator_handler::IndicatorHandler;
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};
use crate::strategies::indicators::indicator_values::IndicatorValues;
use crate::standardized_types::accounts::AccountSetup;
use crate::standardized_types::base_data::history::range_history_data;
use crate::standardized_types::enums::{OrderSide, StrategyMode};
use crate::standardized_types::rolling_window::RollingWindow;
use crate::strategies::strategy_events::StrategyEventBuffer;
use crate::strategies::handlers::subscription_handler::SubscriptionHandler;
use crate::standardized_types::subscriptions::{DataSubscription, SymbolCode, SymbolName};
use crate::strategies::handlers::timed_events_handler::{TimedEvent, TimedEventHandler};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use dashmap::DashMap;
use rust_decimal::Decimal;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use crate::helpers::converters::{naive_date_time_to_tz, naive_date_time_to_utc};
use crate::standardized_types::broker_enum::Brokerage;
use crate::strategies::handlers::market_handler::market_handlers::{add_account, booked_pnl_live, booked_pnl_paper, export_trades, in_drawdown_live, in_drawdown_paper, in_profit_live, in_profit_paper, initialize_live_account, is_flat_live, is_flat_paper, is_long_live, is_long_paper, is_short_live, is_short_paper, market_handler, pnl_live, pnl_paper, position_size_live, position_size_paper, print_ledger, process_ledgers, MarketMessageEnum, BACKTEST_OPEN_ORDER_CACHE, LAST_PRICE, LIVE_ORDER_CACHE};
use crate::strategies::client_features::server_connections::{init_connections, init_sub_handler, initialize_static, live_subscription_handler, send_request, StrategyRequest};
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::base_data::quote::Quote;
use crate::standardized_types::base_data::quotebar::QuoteBar;
use crate::standardized_types::base_data::tick::Tick;
use crate::messages::data_server_messaging::{DataServerRequest, FundForgeError};
use crate::standardized_types::accounts::{AccountId, Currency};
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::orders::{Order, OrderId, OrderRequest, OrderType, OrderUpdateType, TimeInForce};
use crate::strategies::client_features::connection_types::ConnectionType;
use crate::strategies::handlers::market_handler::backtest_matching_engine::{get_market_fill_price_estimate, get_market_price};
use crate::strategies::historical_engine::HistoricalEngine;
use crate::strategies::historical_time::get_backtest_time;
use crate::strategies::indicators::indicator_events::IndicatorEvents;

/// The `FundForgeStrategy` struct is the main_window struct for the FundForge strategy. It contains the state of the strategy and the callback function for data updates.

/// # Properties
#[allow(dead_code)]
pub struct FundForgeStrategy {
    mode: StrategyMode,

    time_zone: chrono_tz::Tz,

    buffer_resolution: Duration,

    subscription_handler: Arc<SubscriptionHandler>,

    indicator_handler: Arc<IndicatorHandler>,

    timed_event_handler: Arc<TimedEventHandler>,

    drawing_objects_handler: Arc<DrawingObjectHandler>,

    orders_count: DashMap<Brokerage, i64>,

    market_event_sender: Sender<MarketMessageEnum>,

    synchronize_accounts: bool,
}

impl FundForgeStrategy {
    /// Initializes a new `FundForgeStrategy` instance with the provided parameters.
    ///
    /// # Arguments
    /// `strategy_mode: StrategyMode`: The mode of the strategy (Backtest, Live, LivePaperTrading).
    ///
    /// `backtest_accounts_starting_cash: Decimal` use dec!(number) to easily initialize decimals, this will set the default starting balance of backtest accounts.
    ///
    /// `start_date: NaiveDateTime`: The start date of the strategy. In the local time_zone that you pass in.
    ///
    /// `end_date: NaiveDateTime`: The end date of the strategy. In the local time_zone that you pass in
    ///
    /// `time_zone: Tz`: The time zone of the strategy, you can use Utc for default.
    ///
    /// `warmup_duration: chrono::Duration`: The warmup duration for the strategy.
    ///
    /// `subscriptions: Vec<DataSubscription>`: The initial data subscriptions for the strategy.
    ///
    /// `fill_forward: bool`: If true we will fill forward with flat bars based on the last close when there is no data, this is only for consolidated data and applies to the initial subscriptions.
    ///
    /// `retain_history: usize`: The number of bars to retain in memory for the strategy. This is useful for strategies that need to reference previous bars for calculations, this is only for our initial subscriptions.
    ///
    /// `strategy_event_sender: mpsc::Sender<EventTimeSlice>`: The sender for strategy events.
    ///
    /// `replay_delay_ms: Option<u64>`: The delay in milliseconds between time slices for market replay style backtesting. \
    ///  any additional subscriptions added later will be able to specify their own history requirements.
    ///
    /// `buffering_resolution: u64`: The buffering resolution of the strategy in milliseconds. If we are backtesting, any data of a lower granularity will be consolidated into a single time slice.
    /// If out base data source is tick data, but we are trading only on 15min bars, then we can just consolidate the tick data and ignore it in on_data_received().
    /// In live trading our strategy will capture the tick stream in a buffer and pass it to the strategy in the correct resolution/durations, this helps to prevent spamming our on_data_received() fn.
    /// If we don't need to make strategy decisions on every tick, we can just consolidate the tick stream into buffered time slice events.
    /// This also helps us get consistent results between backtesting and live trading.
    /// If 0 then it will default to a 1-millisecond buffer.
    ///
    /// `gui_enabled: bool`: If true the engine will forward all StrategyEventSlice's sent to the strategy, to the strategy registry so they can be used by GUI implementations.
    ///
    /// ` tick_over_no_data: bool`: If true the Backtest engine will tick at buffer resolution speed over weekends or other no data periods.
    ///
    /// `synchronize_accounts: bool` If true strategy positions will update in sync with the brokerage, if false the engine will simulate positions using the same logic as backtesting. //todo[ReadMe], explain in more detail
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
        buffering_duration: Duration,
        gui_enabled: bool,
        tick_over_no_data: bool,
        synchronize_accounts: bool
    ) -> FundForgeStrategy {

        let timed_event_handler = Arc::new(TimedEventHandler::new());
        let drawing_objects_handler = Arc::new(DrawingObjectHandler::new(AHashMap::new()));
        initialize_static(
            timed_event_handler.clone(),
            drawing_objects_handler.clone()
        ).await;

        let start_time = time_zone.from_local_datetime(&start_date).unwrap().to_utc();
        let end_time = time_zone.from_local_datetime(&end_date).unwrap().to_utc();
        let warm_up_start_time = start_time - warmup_duration;
        let market_event_sender = market_handler(strategy_mode, backtest_accounts_starting_cash, backtest_account_currency).await;
        let subscription_handler = Arc::new(SubscriptionHandler::new(strategy_mode, market_event_sender.clone()).await);
        let indicator_handler = Arc::new(IndicatorHandler::new(strategy_mode.clone()).await);

        init_sub_handler(subscription_handler.clone(), strategy_event_sender, indicator_handler.clone()).await;
        init_connections(gui_enabled, buffering_duration.clone(), strategy_mode.clone(), market_event_sender.clone(), synchronize_accounts).await;

        subscription_handler.set_subscriptions(subscriptions, retain_history, warm_up_start_time.clone(), fill_forward, false).await;


        let strategy = FundForgeStrategy {
            mode: strategy_mode.clone(),
            buffer_resolution: buffering_duration.clone(),
            time_zone,
            subscription_handler,
            indicator_handler: indicator_handler.clone(),
            timed_event_handler,
            drawing_objects_handler,
            orders_count: Default::default(),
            market_event_sender: market_event_sender.clone(),
            synchronize_accounts
        };

        match strategy_mode {
            StrategyMode::Backtest => {
                let engine = HistoricalEngine::new(strategy_mode.clone(), start_time.to_utc(),  end_time.to_utc(), warmup_duration.clone(), buffering_duration.clone(), gui_enabled.clone(), market_event_sender, tick_over_no_data).await;
                HistoricalEngine::launch(engine).await;
            }
            StrategyMode::LivePaperTrading | StrategyMode::Live  => {
                live_subscription_handler(strategy_mode.clone()).await;
            },
        }
        strategy
    }

    pub async fn add_account(&self, account_setup: AccountSetup) {
        add_account(self.mode.clone(), account_setup);
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

    ///
    pub async fn get_market_price (
        order_side: OrderSide,
        symbol_name: &SymbolName,
    ) -> Result<Price, FundForgeError> {
        get_market_price(order_side, symbol_name).await
    }

    /// true if long, false if flat or short.
    pub fn is_long(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
        match self.mode {
            StrategyMode::Backtest| StrategyMode::LivePaperTrading => is_long_paper(brokerage, account_id, symbol_name),
            StrategyMode::Live  => is_long_live(brokerage, account_id, symbol_name)
        }
    }

    /// true if no position opened for account and symbol
    pub fn is_flat(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
        match self.mode {
            StrategyMode::Backtest| StrategyMode::LivePaperTrading => is_flat_paper(brokerage, account_id, symbol_name),
            StrategyMode::Live  => is_flat_live(brokerage, account_id, symbol_name)
        }
    }

    /// true if short, false if flat or long
    pub fn is_short(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
        match self.mode {
            StrategyMode::Backtest| StrategyMode::LivePaperTrading => is_short_paper(brokerage, account_id, symbol_name),
            StrategyMode::Live => is_short_live(brokerage, account_id, symbol_name)
        }
    }

    async fn order_id(
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

    //todo[Strategy]
    pub async fn custom_order(&self, _order: Order, _order_type: OrderType) -> OrderId {
        todo!("Make a fn that takes an order and figures out what to do with it")
    }


    /// Enters a long position and closes any short positions open for the account and symbol
    pub async fn enter_long(
        &self,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        account_id: &AccountId,
        brokerage: &Brokerage,
        exchange: Option<String>,
        quantity: Volume,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id(symbol_name, account_id, brokerage, &"Enter Long").await;
        let order = Order::enter_long(
            symbol_name.clone(),
            symbol_code,
            brokerage.clone(),
            quantity,
            tag,
            account_id.clone(),
            order_id.clone(),
            self.time_utc(),
            exchange
        );
        let market_request = OrderRequest::Create{ brokerage: order.brokerage.clone(), order: order.clone(), order_type: OrderType::EnterLong };
        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                let request =MarketMessageEnum::OrderRequest(market_request);
                self.market_event_sender.send(request).await.unwrap();
            }
            StrategyMode::Live => {
                LIVE_ORDER_CACHE.insert(order.id.clone(), order);
                initialize_live_account(brokerage.clone(), &account_id, self.synchronize_accounts).await;
                let connection_type = ConnectionType::Broker(brokerage.clone());
                let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest {request: market_request});
                send_request(request).await;
            }
        }
        order_id
    }

    /// Enters a short position and closes any long positions open for the account and symbol
    pub async fn enter_short(
        &self,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        account_id: &AccountId,
        brokerage: &Brokerage,
        exchange: Option<String>,
        quantity: Volume,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id(symbol_name, account_id, brokerage, &"Enter Short").await;
        let order = Order::enter_short(
            symbol_name.clone(),
            symbol_code,
            brokerage.clone(),
            quantity,
            tag,
            account_id.clone(),
            order_id.clone(),
            self.time_utc(),
            exchange
        );
        let request = OrderRequest::Create{ brokerage: order.brokerage.clone(), order: order.clone(), order_type: OrderType::EnterShort};
        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                let request =MarketMessageEnum::OrderRequest(request);
                self.market_event_sender.send(request).await.unwrap();
            }
            StrategyMode::Live => {
                LIVE_ORDER_CACHE.insert(order.id.clone(), order);
                initialize_live_account(brokerage.clone(), &account_id, self.synchronize_accounts).await;
                let connection_type = ConnectionType::Broker(brokerage.clone());
                let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest {request});
                send_request(request).await;
            }
        }
        order_id
    }

    /// Exits a long position or does nothing if no long position
    pub async fn exit_long(
        &self,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        account_id: &AccountId,
        brokerage: &Brokerage,
        exchange: Option<String>,
        quantity: Volume,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id(symbol_name, account_id, brokerage, &"Exit Long").await;
        let order = Order::exit_long(
            symbol_name.clone(),
            symbol_code,
            brokerage.clone(),
            quantity,
            tag,
            account_id.clone(),
            order_id.clone(),
            self.time_utc(),
            exchange
        );
        let request = OrderRequest::Create{ brokerage: order.brokerage.clone(), order: order.clone(), order_type: OrderType::ExitLong};
        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                let request =MarketMessageEnum::OrderRequest(request);
                self.market_event_sender.send(request).await.unwrap();
            }
            StrategyMode::Live => {
                LIVE_ORDER_CACHE.insert(order.id.clone(), order);
                let connection_type = ConnectionType::Broker(brokerage.clone());
                initialize_live_account(brokerage.clone(), &account_id, self.synchronize_accounts).await;
                let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest {request});
                send_request(request).await;
            }
        }
        order_id
    }

    /// Exits a short position or does nothing if no short position
    pub async fn exit_short(
        &self,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        account_id: &AccountId,
        brokerage: &Brokerage,
        exchange: Option<String>,
        quantity: Volume,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id(symbol_name, account_id, brokerage, &"Exit Short").await;
        let order = Order::exit_short(
            symbol_name.clone(),
            symbol_code,
            brokerage.clone(),
            quantity,
            tag,
            account_id.clone(),
            order_id.clone(),
            self.time_utc(),
            exchange
        );
        let request = OrderRequest::Create{ brokerage: order.brokerage.clone(), order: order.clone(), order_type: OrderType::ExitShort};
        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                let request =MarketMessageEnum::OrderRequest(request);
                self.market_event_sender.send(request).await.unwrap();
            }
            StrategyMode::Live => {
                LIVE_ORDER_CACHE.insert(order.id.clone(), order);
                let connection_type = ConnectionType::Broker(brokerage.clone());
                initialize_live_account(brokerage.clone(), &account_id, self.synchronize_accounts).await;
                let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest {request: request});
                send_request(request).await;
            }
        }
        order_id
    }

    /// Buys the market and effects any open positions, or creates a new one
    pub async fn buy_market(
        &self,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        account_id: &AccountId,
        brokerage: &Brokerage,
        exchange: Option<String>,
        quantity: Volume,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id(symbol_name, account_id, brokerage, &"Buy Market").await;
        let order = Order::market_order(
            symbol_name.clone(),
            symbol_code,
            brokerage.clone(),
            quantity,
            OrderSide::Buy,
            tag,
            account_id.clone(),
            order_id.clone(),
            self.time_utc(),
            exchange
        );
        let request = OrderRequest::Create{ brokerage: order.brokerage.clone(), order: order.clone(), order_type: OrderType::Market};
        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                let request = MarketMessageEnum::OrderRequest(request);
                self.market_event_sender.send(request).await.unwrap();
            }
            StrategyMode::Live => {
                LIVE_ORDER_CACHE.insert(order.id.clone(), order);
                let connection_type = ConnectionType::Broker(brokerage.clone());
                initialize_live_account(brokerage.clone(), &account_id, self.synchronize_accounts).await;
                let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest {request});
                send_request(request).await;
            }
        }
        order_id
    }

    /// Sells the market and effects any open positions, or creates a new one
    pub async fn sell_market(
        &self,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        account_id: &AccountId,
        brokerage: &Brokerage,
        exchange: Option<String>,
        quantity: Volume,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id(symbol_name, account_id, brokerage, &"Sell Market").await;
        let order = Order::market_order(
            symbol_name.clone(),
            symbol_code,
            brokerage.clone(),
            quantity,
            OrderSide::Sell,
            tag,
            account_id.clone(),
            order_id.clone(),
            self.time_utc(),
            exchange
        );
        let request = OrderRequest::Create{ brokerage: order.brokerage.clone(), order: order.clone(), order_type: OrderType::Market};
        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                let request =MarketMessageEnum::OrderRequest(request);
                self.market_event_sender.send(request).await.unwrap();
            }
            StrategyMode::Live => {
                LIVE_ORDER_CACHE.insert(order.id.clone(), order);
                let connection_type = ConnectionType::Broker(brokerage.clone());
                initialize_live_account(brokerage.clone(), &account_id, self.synchronize_accounts).await;
                let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest {request});
                send_request(request).await;
            }
        }
        order_id
    }

    /// Will wait for limit price to be hit to fill, if TIF == TimeInForce::Day, it will be cancelled in backtests when the day is over.
    pub async fn limit_order(
        &self,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        account_id: &AccountId,
        brokerage: &Brokerage,
        exchange: Option<String>,
        quantity: Volume,
        side: OrderSide,
        limit_price: Price,
        tif: TimeInForce,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id(symbol_name, account_id, brokerage, &format!("{} Limit", side)).await;
        let order = Order::limit_order(symbol_name.clone(), symbol_code, brokerage.clone(), quantity, side, tag, account_id.clone(), order_id.clone(), self.time_utc(), limit_price, tif, exchange);
        let order_request = OrderRequest::Create{ brokerage: order.brokerage.clone(), order: order.clone(), order_type: OrderType::Limit};
        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                let request =MarketMessageEnum::OrderRequest(order_request);
                self.market_event_sender.send(request).await.unwrap();
            }
            StrategyMode::Live => {
                LIVE_ORDER_CACHE.insert(order.id.clone(), order);
                let connection_type = ConnectionType::Broker(brokerage.clone());
                initialize_live_account(brokerage.clone(), &account_id, self.synchronize_accounts).await;
                let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest {request: order_request});
                send_request(request).await;
            }
        }
        order_id
    }

    /// Will trigger if trigger price is hit and buy or sell at market price.
    pub async fn market_if_touched (
        &self,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        account_id: &AccountId,
        brokerage: &Brokerage,
        exchange: Option<String>,
        quantity: Volume,
        side: OrderSide,
        trigger_price: Price,
        tif: TimeInForce,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id(&symbol_name, account_id, brokerage, &format!("{} MIT", side)).await;
        let order = Order::market_if_touched(symbol_name.clone(), symbol_code, brokerage.clone(), quantity, side, tag, account_id.clone(), order_id.clone(), self.time_utc(),trigger_price, tif, exchange);
        let order_request = OrderRequest::Create{ brokerage: order.brokerage.clone(), order: order.clone(), order_type: OrderType::MarketIfTouched};
        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                let request =MarketMessageEnum::OrderRequest(order_request);
                self.market_event_sender.send(request).await.unwrap();
            }
            StrategyMode::Live => {
                LIVE_ORDER_CACHE.insert(order.id.clone(), order);
                let connection_type = ConnectionType::Broker(brokerage.clone());
                initialize_live_account(brokerage.clone(), &account_id, self.synchronize_accounts).await;
                let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest {request: order_request});
                send_request(request).await;
            }
        }
        order_id
    }

    /// Will buy or sell market price if trigger is hit
    pub async fn stop_order (
        &self,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        account_id: &AccountId,
        brokerage: &Brokerage,
        exchange: Option<String>,
        quantity: Volume,
        side: OrderSide,
        trigger_price: Price,
        tif: TimeInForce,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id(symbol_name, account_id, brokerage, &format!("{} Stop", side)).await;
        let order = Order::stop(symbol_name.clone(), symbol_code, brokerage.clone(), quantity, side, tag, account_id.clone(), order_id.clone(), self.time_utc(),trigger_price, tif, exchange);
        let order_request = OrderRequest::Create{ brokerage: order.brokerage.clone(), order: order.clone(), order_type: OrderType::StopMarket};
        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                let request =MarketMessageEnum::OrderRequest(order_request);
                self.market_event_sender.send(request).await.unwrap();
            }
            StrategyMode::Live => {
                LIVE_ORDER_CACHE.insert(order.id.clone(), order);
                let connection_type = ConnectionType::Broker(brokerage.clone());
                initialize_live_account(brokerage.clone(), &account_id, self.synchronize_accounts).await;
                let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest {request: order_request});
                send_request(request).await;
            }
        }
        order_id
    }

    /// Will trigger on trigger price but fill only when price is on the correct side of limit price, will partially fill in backtest if we have order book data present.
    pub async fn stop_limit (
        &self,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        account_id: &AccountId,
        brokerage: &Brokerage,
        exchange: Option<String>,
        quantity: Volume,
        side: OrderSide,
        tag: String,
        limit_price: Price,
        trigger_price: Price,
        tif: TimeInForce
    ) -> OrderId {
        let order_id = self.order_id(symbol_name, account_id, brokerage, &format!("{} Stop Limit", side)).await;
        let order = Order::stop_limit(symbol_name.clone(), symbol_code, brokerage.clone(), quantity, side, tag, account_id.clone(), order_id.clone(), self.time_utc(),limit_price, trigger_price, tif, exchange);
        let request = OrderRequest::Create{ brokerage: order.brokerage.clone(), order: order.clone(), order_type: OrderType::StopLimit};
        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                self.market_event_sender.send(MarketMessageEnum::OrderRequest(request)).await.unwrap();
            }
            StrategyMode::Live => {
                LIVE_ORDER_CACHE.insert(order.id.clone(), order);
                let connection_type = ConnectionType::Broker(brokerage.clone());
                initialize_live_account(brokerage.clone(), &account_id, self.synchronize_accounts).await;
                let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest {request});
                send_request(request).await;
            }
        }
        order_id
    }

    /// Cancels the order if it is not filled, cancelled or rejected.
    pub async fn cancel_order(&self, order_id: OrderId) {
        let map: Arc<DashMap<OrderId, Order>> = match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => BACKTEST_OPEN_ORDER_CACHE.clone(),
            StrategyMode::Live => LIVE_ORDER_CACHE.clone(),
        };

        // Clone the necessary data from the Ref
        let (brokerage, account_id) = if let Some(id_order_ref) = map.get(&order_id) {
            (id_order_ref.brokerage.clone(), id_order_ref.account_id.clone())
        } else {
            return; // Order not found, exit the function
        };

        let order_request = OrderRequest::Cancel {
            order_id,
            brokerage: brokerage.clone(),
            account_id: account_id.clone()
        };

        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                let request = MarketMessageEnum::OrderRequest(order_request);
                self.market_event_sender.send(request).await.unwrap();
            }
            StrategyMode::Live => {
                let connection_type = ConnectionType::Broker(brokerage);
                let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest {request: order_request});
                send_request(request).await;
            }
        }
    }

    /// Updates the order if it is not filled, cancelled or rejected.
    pub async fn update_order(&self, order_id: OrderId, order_update_type: OrderUpdateType) {
        let map = match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => BACKTEST_OPEN_ORDER_CACHE.clone(),
            StrategyMode::Live => LIVE_ORDER_CACHE.clone(),
        };

        // Clone the necessary data from the Ref
        let (brokerage, account_id) = if let Some(id_order_ref) = map.get(&order_id) {
            (id_order_ref.brokerage.clone(), id_order_ref.account_id.clone())
        } else {
            return; // Order not found, exit the function
        };

        let order_request = OrderRequest::Update {
            brokerage: brokerage.clone(),
            order_id,
            account_id,
            update: order_update_type,
        };

        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                let update_msg = MarketMessageEnum::OrderRequest(order_request);
                self.market_event_sender.send(update_msg).await.unwrap();
            }
            StrategyMode::Live => {
                let connection_type = ConnectionType::Broker(brokerage);
                let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest {request: order_request});
                send_request(request).await;
            }
        }
    }

    /// Cancel all pending orders on the account for the symbol_name
    pub async fn cancel_orders(&self, brokerage: Brokerage, account_id: AccountId, symbol_name: SymbolName) {
        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                let cancel_msg =  MarketMessageEnum::OrderRequest(OrderRequest::CancelAll{brokerage, account_id, symbol_name});
                self.market_event_sender.send(cancel_msg).await.unwrap();
            }
            StrategyMode::Live => {
                let connection_type = ConnectionType::Broker(brokerage);
                let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest {request: OrderRequest::CancelAll{brokerage, account_id, symbol_name}});
                send_request(request).await;
            }
        }
    }

    /// Flatten all positions on the account.
    pub async fn flatten_all_for(&self, brokerage: Brokerage, account_id: &AccountId) {
        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                let order = MarketMessageEnum::OrderRequest(OrderRequest::FlattenAllFor {brokerage, account_id: account_id.clone()});
                self.market_event_sender.send(order).await.unwrap();
            }
            StrategyMode::Live => {
                let connection_type = ConnectionType::Broker(brokerage);
                let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest {request: OrderRequest::FlattenAllFor {brokerage, account_id: account_id.clone()}});
                send_request(request).await;
            }
        }
    }

    /// get the last price for the symbol name
    pub async fn last_price(&self, symbol_name: &SymbolName) -> Option<Price> {
        match LAST_PRICE.get(symbol_name) {
            None => None,
            Some(price) => Some(price.clone())
        }
    }

    /// ALl pending orders on the account.
    pub async fn orders_pending(&self) -> Arc<DashMap<OrderId, Order>> {
        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => BACKTEST_OPEN_ORDER_CACHE.clone(),
            StrategyMode::Live => LIVE_ORDER_CACHE.clone()
        }
    }

    /// Adds a timed event which will trigger a time message to the receiver at the time (or after time updates again if time has passed in backtest)
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
    /// Using unwrap on historical index() data in live mode should still be safe when using the current data as reference for the new subscription,
    /// because we won't forward bars until the consolidator is warmed up.
    pub async fn subscribe_indicator(&self, indicator: IndicatorEnum, auto_subscribe: bool) {
        match self.mode {
            StrategyMode::Backtest => {
                let subscriptions = self.subscriptions().await;
                if !subscriptions.contains(indicator.subscription()) {
                    match auto_subscribe {
                        true => {
                            self.subscribe(indicator.subscription().clone(), (indicator.data_required_warmup() + 1) as usize, false).await;
                        }
                        false => panic!("You have no subscription: {}, for the indicator subscription {} and AutoSubscribe is not enabled", indicator.subscription(), indicator.name())
                    }
                }
                self.indicator_handler
                    .add_indicator(indicator, self.time_utc())
                    .await;
                //add_buffer(self.time_utc(), StrategyEvent::IndicatorEvent(event)).await;
            }
            StrategyMode::Live | StrategyMode::LivePaperTrading => {
                let handler = self.subscription_handler.clone();
                let indicator_handler = self.indicator_handler.clone();
                let subscriptions = self.subscriptions().await;
                tokio::task::spawn(async move {
                    if !subscriptions.contains(&indicator.subscription()) {
                        match auto_subscribe {
                            true => {
                                let result = handler.subscribe(indicator.subscription().clone(), Utc::now(),false, (indicator.data_required_warmup() + 1) as usize, true).await;
                                match result {
                                    Ok(_) => {
                                       // add_buffer(Utc::now(), StrategyEvent::DataSubscriptionEvent(sub_result)).await;
                                    },
                                    Err(_) =>  {
                                       // add_buffer(Utc::now(), StrategyEvent::DataSubscriptionEvent(sub_result)).await;
                                    },
                                }
                            }
                            false => eprintln!("You have no subscription: {}, for the indicator subscription {} and AutoSubscribe is not enabled", indicator.subscription(), indicator.name())
                        }
                    }
                    indicator_handler
                        .add_indicator(indicator, Utc::now())
                        .await;
                   // add_buffer(Utc::now(), StrategyEvent::IndicatorEvent(event)).await;
                });
            }
        }
    }

    /// see the indicator_enum.rs for more details
    pub async fn indicator_unsubscribe(&self, name: &IndicatorName) -> Option<IndicatorEvents> {
        self.indicator_handler.remove_indicator(name).await
    }

    /// see the indicator_enum.rs for more details
    pub async fn indicator_unsubscribe_subscription(&self, subscription: &DataSubscription) {
        self.indicator_handler
            .indicators_unsubscribe_subscription(subscription)
            .await
    }

    /// see the indicator_enum.rs for more details
    pub fn indicator_index(
        &self,
        name: &IndicatorName,
        index: usize,
    ) -> Option<IndicatorValues> {
        self.indicator_handler.index(name, index)
    }

    /// see the indicator_enum.rs for more details
    pub fn indicator_current(&self, name: &IndicatorName) -> Option<IndicatorValues> {
        self.indicator_handler.current(name)
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

    /// Returns all the subscriptions including primary and consolidators
    pub async fn subscriptions_all(&self) -> Vec<DataSubscription> {
        self.subscription_handler.subscriptions().await
    }

    /// Returns subscriptions that the strategy subscribed to, ignoring primary subscriptions used by consolidators if they were not implicitly subscribed by the strategy.
    pub async fn subscriptions(&self) -> Vec<DataSubscription> {
        self.subscription_handler.strategy_subscriptions().await
    }

    /// Subscribes to a new subscription, we can only subscribe to a subscription once.
    /// In live mode we will warm up the subscription as a background task, in backtest we will block the main thread.
    /// Using unwrap on historical index() data in live mode should still be safe when using the current data as reference for the new subscription,
    /// because we won't forward bars until the consolidator is warmed up.
    pub async fn subscribe(&self, subscription: DataSubscription, history_to_retain: usize, fill_forward: bool) {
        match self.mode {
            StrategyMode::Backtest => {
                let result = self.subscription_handler
                    .subscribe(subscription.clone(), self.time_utc(), fill_forward, history_to_retain, true)
                    .await;
                match &result {
                    Ok(_sub_result) => {
                        //add_buffer(self.time_utc(), StrategyEvent::DataSubscriptionEvent(sub_result.to_owned())).await;
                    },
                    Err(_sub_result) =>  {
                        //add_buffer(self.time_utc(), StrategyEvent::DataSubscriptionEvent(sub_result.to_owned())).await;
                    }
                }
            }
            StrategyMode::Live | StrategyMode::LivePaperTrading => {
                let handler = self.subscription_handler.clone();
                //tokio::task::spawn(async move{
                    let result = handler
                        .subscribe(subscription.clone(), Utc::now(), fill_forward, history_to_retain, true)
                        .await;
                    match &result {
                        Ok(_sub_result) => {
                            //add_buffer(Utc::now(), StrategyEvent::DataSubscriptionEvent(sub_result.to_owned())).await;
                        },
                        Err(_sub_result) =>  {
                            //add_buffer(Utc::now(), StrategyEvent::DataSubscriptionEvent(sub_result.to_owned())).await;
                        }
                    }
                //});
            }
        }
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
    /// This method will unsubscribe any subscriptions not included and set the new subscriptions to those that are passed in.
    pub async fn subscriptions_update(
        &self,
        subscriptions: Vec<DataSubscription>,
        retain_to_history: usize,
        fill_forward: bool
    ) {
        match self.mode {
            StrategyMode::Backtest => {
                self.subscription_handler.set_subscriptions(subscriptions, retain_to_history, self.time_utc(), fill_forward, true).await;
            }
            StrategyMode::Live | StrategyMode::LivePaperTrading => {
                let handler = self.subscription_handler.clone();
                tokio::task::spawn(async move{
                    handler.set_subscriptions(subscriptions, retain_to_history, Utc::now(), fill_forward, true).await;
                });
            }
        }

    }

    /// Returns currently open `QuoteBar` for the subscription
    pub fn open_bar(&self, subscription: &DataSubscription) -> Option<QuoteBar> {
        self.subscription_handler.open_bar(subscription)
    }

    /// Returns currently open candle for the subscription
    pub fn open_candle(&self, subscription: &DataSubscription) -> Option<Candle> {
        self.subscription_handler.open_candle(subscription)
    }

    /// Returns `Candle` at the specified index, where 0 is current closed `Candle` and 1 is last closed and 10 closed 10 candles ago.
    pub fn candle_index(&self, subscription: &DataSubscription, index: usize) -> Option<Candle> {
        self.subscription_handler.candle_index(subscription, index)
    }

    /// Returns `QuoteBar` at the specified index, where 0 is current closed `QuoteBar` and 1 is last closed and 10 closed 10 `QuoteBar`s ago.
    pub fn bar_index(&self, subscription: &DataSubscription, index: usize) -> Option<QuoteBar> {
        self.subscription_handler.bar_index(subscription, index)
    }

    /// Returns `Tick` at the specified index, where 0 is last `Tick` and 1 is 2nd last `Tick` and 10 is 10 `Ticks`s ago.
    pub fn tick_index(&self, subscription: &DataSubscription, index: usize) -> Option<Tick> {
        self.subscription_handler.tick_index(subscription, index)
    }

    /// Returns `Quote` at the specified index, where 0 is last `Quote` and 1 is 2nd last `Quote` and 10 is 10 `Quote`s ago.
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

    /// Prints the ledger statistics
    pub fn print_ledger(&self, brokerage: &Brokerage, account_id: &AccountId) {
        if let Some(ledger_string) = print_ledger(brokerage, account_id) {
            println!("{}", ledger_string);
        }
    }

    /// Prints all ledgers statistics
    pub fn print_ledgers(&self) {
        let strings = process_ledgers();
        for string in strings {
            println!("{}", string);
        }
    }

    pub fn export_trades(&self, directory: &str) {
        export_trades(directory);
    }

    /// Returns a BTreeMap of BaseDataEnum where data.time_closed_utc() is key and data is value.
    /// From the time, to the current strategy time
    pub async fn history_from_local_time(
        &self,
        from_time: NaiveDateTime,
        time_zone: Tz,
        subscription: &DataSubscription,
    ) -> BTreeMap<DateTime<Utc>, BaseDataEnum> {
        let start_date = naive_date_time_to_tz(from_time, time_zone);
        range_history_data(start_date.to_utc(), self.time_utc(), subscription.clone(), self.mode).await
    }

    /// Returns a BTreeMap of BaseDataEnum where data.time_closed_utc() is key and data is value.
    /// From the time, to the current strategy time
    pub async fn history_from_utc_time(
        &self,
        from_time: NaiveDateTime,
        subscription: &DataSubscription,
    ) -> BTreeMap<DateTime<Utc>, BaseDataEnum> {
        let start_date = naive_date_time_to_utc(from_time);
        range_history_data(start_date.to_utc(), self.time_utc(), subscription.clone(), self.mode).await
    }

    /// Returns a BTreeMap of BaseDataEnum where data.time_closed_utc() is key and data is value.
    /// If to time > strategy.time then to time will be changed to strategy.time to avoid lookahead bias
    pub async fn historical_range_from_local_time(
        &self,
        from_time: NaiveDateTime,
        to_time: NaiveDateTime,
        time_zone: Tz,
        subscription: &DataSubscription,
    ) -> BTreeMap<DateTime<Utc>, BaseDataEnum> {
        let start_date = naive_date_time_to_tz(from_time, time_zone);
        let end_date =  naive_date_time_to_tz(to_time, time_zone).to_utc();

        let end_date = match end_date > self.time_utc() {
            true => self.time_utc(),
            false => end_date.to_utc(),
        };

        range_history_data(start_date.to_utc(), end_date, subscription.clone(), self.mode).await
    }

    /// Currently returns only primary data that is available, needs to be updated to be able to return all subscriptions via consolidated data
    pub async fn historical_range_from_utc_time(
        &self,
        from_time: NaiveDateTime,
        to_time: NaiveDateTime,
        subscription: &DataSubscription,
    ) -> BTreeMap<DateTime<Utc>, BaseDataEnum> {
        let start_date = DateTime::<Utc>::from_naive_utc_and_offset(from_time, Utc);
        let end_date = DateTime::<Utc>::from_naive_utc_and_offset(to_time, Utc);

        let end_date = match end_date > self.time_utc() {
            true => self.time_utc(),
            false => end_date,
        };

        range_history_data(start_date.to_utc(), end_date, subscription.clone(), self.mode).await
    }

    /// Returns true of the account is in profit on this symbol.
    ///
    /// Returns false if no position.
    pub fn in_profit(&self,brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => in_profit_paper(symbol_name, brokerage, account_id),
            StrategyMode::Live => in_profit_live(symbol_name, brokerage, account_id)
        }
    }

    /// Returns true of the account is in drawdown on this symbol.
    ///
    /// Returns false if no position.
    pub fn in_drawdown(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => in_drawdown_paper(symbol_name, brokerage, account_id),
            StrategyMode::Live => in_drawdown_live(symbol_name, brokerage, account_id)
        }
    }

    /// Returns the open pnl for the current position.
    ///
    /// Returns 0.0 if no position open
    pub fn pnl(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> Decimal {
        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => pnl_paper(symbol_name, brokerage, account_id),
            StrategyMode::Live => pnl_live(symbol_name, brokerage, account_id)
        }
    }

    /// Returns the booked pnl for the current position (not all positions we have booked profit on).
    ///
    /// Returns 0.0 if no position open.
    ///
    /// does not return the total pnl for all closed positions on the symbol, just the current open one.
    pub fn booked_pnl(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> Decimal {
        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => booked_pnl_paper(symbol_name, brokerage, account_id),
            StrategyMode::Live => booked_pnl_live(symbol_name, brokerage, account_id)
        }
    }

    pub fn position_size(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> Decimal {
        match self.mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => position_size_paper(symbol_name, brokerage, account_id),
            StrategyMode::Live => position_size_live(symbol_name, brokerage, account_id)
        }
    }
}
