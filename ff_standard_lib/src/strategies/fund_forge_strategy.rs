use ahash::AHashMap;
use chrono::{DateTime, Duration as ChronoDuration, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use crate::strategies::handlers::drawing_object_handler::DrawingObjectHandler;
use crate::gui_types::drawing_objects::drawing_tool_enum::DrawingTool;
use crate::strategies::handlers::indicator_handler::IndicatorHandler;
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};
use crate::strategies::indicators::indicator_values::IndicatorValues;
use crate::standardized_types::base_data::history::range_history_data;
use crate::standardized_types::enums::{OrderSide, StrategyMode, PrimarySubscription, FuturesExchange, PositionSide};
use crate::standardized_types::rolling_window::RollingWindow;
use crate::strategies::strategy_events::StrategyEvent;
use crate::strategies::handlers::subscription_handler::SubscriptionHandler;
use crate::standardized_types::subscriptions::{DataSubscription, SymbolCode, SymbolName};
use crate::strategies::handlers::timed_events_handler::{TimedEvent, TimedEventHandler};
use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use dashmap::DashMap;
use rust_decimal::Decimal;
use tokio::sync::{mpsc, oneshot, Notify};
use tokio::sync::mpsc::Sender;
use uuid::Uuid;
use crate::helpers::converters::{naive_date_time_to_tz, naive_date_time_to_utc, resolve_market_datetime_in_timezone};
use crate::helpers::decimal_calculators::round_to_tick_size;
use crate::strategies::client_features::server_connections::{init_connections, is_warmup_complete};
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::base_data::quote::Quote;
use crate::standardized_types::base_data::quotebar::QuoteBar;
use crate::standardized_types::base_data::tick::Tick;
use crate::messages::data_server_messaging::{DataServerRequest, DataServerResponse};
use crate::product_maps::rithmic::rollover::get_front_month;
use crate::standardized_types::accounts::{Account, Currency};
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::broker_enum::Brokerage;
use crate::standardized_types::market_hours::TradingHours;
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::orders::{Order, OrderId, OrderRequest, OrderType, OrderUpdateType, TimeInForce};
use crate::standardized_types::position::Position;
use crate::strategies::client_features::connection_types::ConnectionType;
use crate::strategies::client_features::live_subscriptions::live_subscription_handler;
use crate::strategies::client_features::request_handler::{send_request, StrategyRequest};
use crate::strategies::handlers::live_warmup::live_warm_up;
use crate::strategies::handlers::market_handler::backtest_matching_engine;
use crate::strategies::handlers::market_handler::backtest_matching_engine::BackTestEngineMessage;
use crate::strategies::handlers::market_handler::live_order_matching::live_order_handler;
use crate::strategies::handlers::market_handler::price_service::MarketPriceService;
use crate::strategies::historical_engine::HistoricalEngine;
use crate::strategies::historical_time::{get_backtest_time, update_backtest_time};
use crate::strategies::indicators::indicator_events::IndicatorEvents;
use crate::strategies::ledgers::ledger_service::LedgerService;

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

    synchronize_accounts: bool,

    open_order_cache: Arc<DashMap<OrderId, Order>>,

    closed_order_cache: Arc<DashMap<OrderId, Order>>,

    backtest_accounts_starting_cash: Decimal,

    backtest_account_currency: Currency,

    historical_message_sender: Option<Sender<BackTestEngineMessage>>,

    accounts: Vec<Account>,

    ledger_service: Arc<LedgerService>,

    market_price_service: Arc<MarketPriceService>

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
    /// This also helps us get_requests consistent results between backtesting and live trading.
    /// If 0 then it will default to a 1-millisecond buffer.
    ///
    /// `gui_enabled: bool`: If true the engine will forward all StrategyEventSlice's sent to the strategy, to the strategy registry so they can be used by GUI implementations.
    ///
    /// `tick_over_no_data: bool`: If true the Backtest engine will tick at buffer resolution speed over weekends or other no data periods.
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
        intraday_subscriptions: Vec<(Option<PrimarySubscription>, DataSubscription, Option<TradingHours>)>,
        fill_forward: bool,
        retain_history: usize,
        strategy_event_sender: mpsc::Sender<StrategyEvent>,
        buffering_duration: Duration,
        gui_enabled: bool,
        tick_over_no_data: bool,
        synchronize_accounts: bool,
        accounts: Vec<Account>
    ) -> FundForgeStrategy {

        let price_service = Arc::new(MarketPriceService::new());
        let ledger_service = Arc::new(LedgerService::new(strategy_event_sender.clone(), price_service.clone()));

        let timed_event_handler = Arc::new(TimedEventHandler::new(strategy_event_sender.clone()));
        let drawing_objects_handler = Arc::new(DrawingObjectHandler::new(AHashMap::new()));


        let start_time = resolve_market_datetime_in_timezone(time_zone, start_date).to_utc();
        let end_time = resolve_market_datetime_in_timezone(time_zone, end_date).to_utc();

        let warm_up_start_time = start_time - warmup_duration;
        update_backtest_time(warm_up_start_time);

        let open_order_cache: Arc<DashMap<OrderId, Order>> = Arc::new(DashMap::new());
        let closed_order_cache: Arc<DashMap<OrderId, Order>> = Arc::new(DashMap::new());

        let notify = Arc::new(Notify::new());
        let subscription_handler = Arc::new(SubscriptionHandler::new(strategy_mode, strategy_event_sender.clone()).await);
        let indicator_handler = Arc::new(IndicatorHandler::new(strategy_mode.clone(), subscription_handler.clone()).await);

        let (live_order_updates_sender, live_order_updates_receiver) = tokio::sync::mpsc::channel(100);
        if strategy_mode == StrategyMode::Live {
            live_order_handler(open_order_cache.clone(), closed_order_cache.clone(), live_order_updates_receiver, strategy_event_sender.clone(), ledger_service.clone(), synchronize_accounts);
        }

        init_connections(gui_enabled, buffering_duration.clone(), strategy_mode.clone(), live_order_updates_sender, synchronize_accounts, strategy_event_sender.clone(), ledger_service.clone(), indicator_handler.clone(), subscription_handler.clone(), price_service.clone()).await;

        for (primary, sub, trading_hours) in intraday_subscriptions {
            subscription_handler.subscribe(primary, sub, warm_up_start_time, fill_forward, retain_history, false, trading_hours).await;
        }

        let paper_order_sender = match strategy_mode {
            StrategyMode::Live => None,
            StrategyMode::LivePaperTrading | StrategyMode::Backtest => {
                let sender = backtest_matching_engine::backtest_matching_engine(open_order_cache.clone(), closed_order_cache.clone(), strategy_event_sender.clone(), ledger_service.clone(), notify.clone(), price_service.clone()).await;
                Some(sender) //todo, live paper wont update orders unless we update time in the backtest engine.
            }
        };

        let strategy = FundForgeStrategy {
            historical_message_sender: paper_order_sender.clone(),
            backtest_accounts_starting_cash,
            backtest_account_currency,
            open_order_cache,
            closed_order_cache,
            mode: strategy_mode.clone(),
            buffer_resolution: buffering_duration.clone(),
            time_zone,
            subscription_handler: subscription_handler.clone(),
            indicator_handler: indicator_handler.clone(),
            timed_event_handler: timed_event_handler.clone(),
            drawing_objects_handler,
            synchronize_accounts,
            accounts: accounts.clone(),
            ledger_service: ledger_service.clone(),
            market_price_service: price_service.clone()
        };


        match strategy_mode {
            StrategyMode::Backtest => {
                let engine = HistoricalEngine::new(
                    strategy_mode.clone(),
                    start_time.to_utc(),
                    end_time.to_utc(),
                    warmup_duration.clone(),
                    buffering_duration.clone(),
                    gui_enabled.clone(),
                    tick_over_no_data,
                    strategy_event_sender.clone(),
                    notify,
                    paper_order_sender,
                    ledger_service.clone(),
                    timed_event_handler.clone(),
                    indicator_handler.clone(),
                    subscription_handler.clone(),
                    price_service.clone()
                ).await;

                HistoricalEngine::launch(engine).await;
            }
            StrategyMode::LivePaperTrading | StrategyMode::Live  => {
                TimedEventHandler::run_time_updates(timed_event_handler.clone()).await;
                live_subscription_handler(strategy_mode.clone(), subscription_handler.clone()).await;
            },
        }

        for account in accounts {
            ledger_service.init_ledger(&account,strategy_mode, synchronize_accounts, backtest_accounts_starting_cash, backtest_account_currency).await;
        }

        if strategy_mode != StrategyMode::Backtest {
            live_warm_up(Utc::now() - warmup_duration, buffering_duration, subscription_handler, strategy_event_sender, timed_event_handler, ledger_service, indicator_handler, price_service.clone()).await;
        }
        strategy
    }

    pub fn calculate_stop_price(&self, entry_price: Decimal, position_side: PositionSide, max_loss: Decimal, value_per_tick: Decimal, tick_size: Decimal, quantity: Decimal) -> Decimal {
        // Calculate loss per contract
        let loss_per_contract = max_loss.abs() / quantity;

        // Calculate ticks per contract to reach max loss
        let ticks_to_loss = (loss_per_contract / value_per_tick).floor();
        let price_distance = ticks_to_loss * tick_size;

        match position_side {
            PositionSide::Long => {
                let stop = entry_price - price_distance;
                round_to_tick_size(stop, tick_size)
            },
            PositionSide::Short => {
                let stop = entry_price + price_distance;
                round_to_tick_size(stop, tick_size)
            },
            _ => panic!("Invalid PositionSide")
        }
    }


    /// In backtesting this will return the front month using:
    /// ```rust
    /// ff_standard_lib::product_maps::rithmic::rollover::get_front_month;
    /// ```
    /// In Live or LivePaper mode, this function will instead call the rithmic api via the ff_data_server to get the front month.
    /// The front month is automatically adjusted each trading day during rollover, there is no automatic position rollover, this must be handled by the strategy.
    ///
    /// This should not be called on every tick, as it will make a request to the data server.
    pub async fn get_front_month(&self, brokerage: Brokerage, symbol_name: SymbolName, exchange: FuturesExchange) -> Option<SymbolCode> {
        match self.mode {
            StrategyMode::Backtest => {
                match get_front_month(&symbol_name, self.time_utc()) {
                    Ok(symbol_code) => Some(symbol_code),
                    Err(_) => panic!("Error getting front month for backtest")
                }
            }
            StrategyMode::LivePaperTrading | StrategyMode::Live => {
                let request = DataServerRequest::FrontMonthInfo {
                    callback_id: 0,
                    symbol_name,
                    exchange,
                    brokerage,
                };
                let (sender, receiver) = oneshot::channel();
                let msg = StrategyRequest::CallBack(ConnectionType::Broker(brokerage), request, sender);
                send_request(msg).await;
                match receiver.await {
                    Ok(response) => {
                        match response {
                            DataServerResponse::FrontMonthInfo { info, .. } => Some(info.symbol_code),
                            DataServerResponse::Error { error, .. } => {
                                eprintln!("Error getting front month: {:?}", error);
                                None
                            },
                            _ => None
                        }
                    },
                    Err(e) => {
                        eprintln!("Error getting front month: {:?}", e);
                        None
                    }
                }
            }
        }
    }

    pub fn accounts(&self) -> &Vec<Account> {
        &self.accounts
    }

    pub async fn get_market_fill_price_estimate (
        &self,
        order_side: OrderSide,
        symbol_name: &SymbolName,
        symbol_code: &SymbolCode,
        volume: Volume,
    ) -> Option<Price> {
        self.market_price_service.estimate_fill_price(order_side, &symbol_name,  &symbol_code, volume)
    }

    ///
    pub async fn get_market_price (
        &self,
        order_side: OrderSide,
        symbol_name: &SymbolName,
        symbol_code: &SymbolCode
    ) -> Option<Price> {
        self.market_price_service.get_market_price(order_side, &symbol_name, &symbol_code)
    }

    /// true if long, false if flat or short.
    pub fn is_long(&self, account: &Account, name: &String) -> bool {
        self.ledger_service.is_long(account, name)
    }

    pub fn is_flat(&self, account: &Account, name: &String) -> bool {
        self.ledger_service.is_flat(account, name)
    }

    pub fn is_short(&self, account: &Account, name: &String) -> bool {
        self.ledger_service.is_short(account, name)
    }

    async fn order_id(
        &self,
    ) -> OrderId {
        let guid = Uuid::new_v4();
        format!(
            "{}",
            guid
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
        account: &Account,
        exchange: Option<String>,
        quantity: Volume,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id().await;
        let order = Order::enter_long(
            symbol_name.clone(),
            symbol_code,
            account,
            quantity,
            tag,
            order_id.clone(),
            self.time_utc(),
            exchange
        );
        let order_request = OrderRequest::Create{ account: account.clone(), order: order.clone(), order_type: OrderType::EnterLong };
        if self.mode == StrategyMode::Live {
            self.open_order_cache.insert(order_id.clone(), order.clone());
            let connection_type = ConnectionType::Broker(order_request.brokerage());
            let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest { request: order_request });
            send_request(request).await;
        } else {
            if let Some(historical_message_sender) = &self.historical_message_sender {
                historical_message_sender.send(BackTestEngineMessage::OrderRequest(order_request)).await.unwrap();
            }
        }
        order_id
    }

    /// Enters a short position and closes any long positions open for the account and symbol
    pub async fn enter_short(
        &self,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        account: &Account,
        exchange: Option<String>,
        quantity: Volume,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id().await;
        let order = Order::enter_short(
            symbol_name.clone(),
            symbol_code,
            account,
            quantity,
            tag,
            order_id.clone(),
            self.time_utc(),
            exchange
        );
        let order_request = OrderRequest::Create{ account: account.clone(), order: order.clone(), order_type: OrderType::EnterShort};
        if self.mode == StrategyMode::Live {
            self.open_order_cache.insert(order_id.clone(), order.clone());
            let connection_type = ConnectionType::Broker(order_request.brokerage());
            let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest { request: order_request });
            send_request(request).await;
        } else {
            if let Some(historical_message_sender) = &self.historical_message_sender {
                historical_message_sender.send(BackTestEngineMessage::OrderRequest(order_request)).await.unwrap();
            }
        }
        order_id
    }

    /// Exits a long position or does nothing if no long position
    pub async fn exit_long(
        &self,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        account: &Account,
        exchange: Option<String>,
        quantity: Volume,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id().await;
        let order = Order::exit_long(
            symbol_name.clone(),
            symbol_code,
            account,
            quantity,
            tag,
            order_id.clone(),
            self.time_utc(),
            exchange
        );
        let order_request = OrderRequest::Create{ account: account.clone(), order: order.clone(), order_type: OrderType::ExitLong};
        if self.mode == StrategyMode::Live {
            self.open_order_cache.insert(order_id.clone(), order.clone());
            let connection_type = ConnectionType::Broker(order_request.brokerage());
            let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest { request: order_request });
            send_request(request).await;
        } else {
            if let Some(historical_message_sender) = &self.historical_message_sender {
                historical_message_sender.send(BackTestEngineMessage::OrderRequest(order_request)).await.unwrap();
            }
        }
        order_id
    }

    /// Exits a short position or does nothing if no short position
    pub async fn exit_short(
        &self,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        account: &Account,
        exchange: Option<String>,
        quantity: Volume,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id().await;
        let order = Order::exit_short(
            symbol_name.clone(),
            symbol_code,
            account,
            quantity,
            tag,
            order_id.clone(),
            self.time_utc(),
            exchange
        );
        let order_request = OrderRequest::Create{ account: account.clone(), order: order.clone(), order_type: OrderType::ExitShort};
        if self.mode == StrategyMode::Live {
            self.open_order_cache.insert(order_id.clone(), order.clone());
            let connection_type = ConnectionType::Broker(order_request.brokerage());
            let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest { request: order_request });
            send_request(request).await;
        } else {
            if let Some(historical_message_sender) = &self.historical_message_sender {
                historical_message_sender.send(BackTestEngineMessage::OrderRequest(order_request)).await.unwrap();
            }
        }
        order_id
    }

    /// Buys the market and effects any open positions, or creates a new one
    pub async fn buy_market(
        &self,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        account: &Account,
        exchange: Option<String>,
        quantity: Volume,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id().await;
        let order = Order::market_order(
            symbol_name.clone(),
            symbol_code,
            account,
            quantity,
            OrderSide::Buy,
            tag,
            order_id.clone(),
            self.time_utc(),
            exchange
        );
        let order_request = OrderRequest::Create{ account: account.clone(), order: order.clone(), order_type: OrderType::Market};

        if self.mode == StrategyMode::Live {
            self.open_order_cache.insert(order_id.clone(), order.clone());
            let connection_type = ConnectionType::Broker(order_request.brokerage());
            let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest { request: order_request });
            send_request(request).await;
        } else {
            if let Some(historical_message_sender) = &self.historical_message_sender {
                historical_message_sender.send(BackTestEngineMessage::OrderRequest(order_request)).await.unwrap();
            }
        }
        order_id
    }

    /// Sells the market and effects any open positions, or creates a new one
    pub async fn sell_market(
        &self,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        account: &Account,
        exchange: Option<String>,
        quantity: Volume,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id().await;
        let order = Order::market_order(
            symbol_name.clone(),
            symbol_code,
            account,
            quantity,
            OrderSide::Sell,
            tag,
            order_id.clone(),
            self.time_utc(),
            exchange
        );
        let order_request = OrderRequest::Create{ account: account.clone(), order: order.clone(), order_type: OrderType::Market};
        if self.mode == StrategyMode::Live {
            self.open_order_cache.insert(order_id.clone(), order.clone());
            let connection_type = ConnectionType::Broker(order_request.brokerage());
            let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest { request: order_request });
            send_request(request).await;
        } else {
            if let Some(historical_message_sender) = &self.historical_message_sender {
                historical_message_sender.send(BackTestEngineMessage::OrderRequest(order_request)).await.unwrap();
            }
        }
        order_id
    }

    /// Will wait for limit price to be hit to fill, if TIF == TimeInForce::Day, it will be cancelled in backtests when the day is over.
    pub async fn limit_order(
        &self,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        account: &Account,
        exchange: Option<String>,
        quantity: Volume,
        side: OrderSide,
        limit_price: Price,
        tif: TimeInForce,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id().await;
        let order = Order::limit_order(symbol_name.clone(), symbol_code, account, quantity, side, tag, order_id.clone(), self.time_utc(), limit_price, tif, exchange);
        let order_request = OrderRequest::Create{ account: account.clone(), order: order.clone(), order_type: OrderType::Limit};
        if self.mode == StrategyMode::Live {
            self.open_order_cache.insert(order_id.clone(), order.clone());
            let connection_type = ConnectionType::Broker(order_request.brokerage());
            let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest { request: order_request });
            send_request(request).await;
        } else {
            if let Some(historical_message_sender) = &self.historical_message_sender {
                historical_message_sender.send(BackTestEngineMessage::OrderRequest(order_request)).await.unwrap();
            }
        }
        order_id
    }

    /// Will trigger if trigger price is hit and buy or sell at market price.
    pub async fn market_if_touched (
        &self,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        account: &Account,
        exchange: Option<String>,
        quantity: Volume,
        side: OrderSide,
        trigger_price: Price,
        tif: TimeInForce,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id().await;
        let order = Order::market_if_touched(symbol_name.clone(), symbol_code, account, quantity, side, tag, order_id.clone(), self.time_utc(),trigger_price, tif, exchange);
        let order_request = OrderRequest::Create{ account: account.clone(), order: order.clone(), order_type: OrderType::MarketIfTouched};
        if self.mode == StrategyMode::Live {
            self.open_order_cache.insert(order_id.clone(), order.clone());
            let connection_type = ConnectionType::Broker(order_request.brokerage());
            let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest { request: order_request });
            send_request(request).await;
        } else {
            if let Some(historical_message_sender) = &self.historical_message_sender {
                historical_message_sender.send(BackTestEngineMessage::OrderRequest(order_request)).await.unwrap();
            }
        }
        order_id
    }

    /// Will buy or sell market price if trigger is hit
    pub async fn stop_order (
        &self,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        account: &Account,
        exchange: Option<String>,
        quantity: Volume,
        side: OrderSide,
        trigger_price: Price,
        tif: TimeInForce,
        tag: String,
    ) -> OrderId {
        let order_id = self.order_id().await;
        let order = Order::stop(symbol_name.clone(), symbol_code, account, quantity, side, tag, order_id.clone(), self.time_utc(),trigger_price, tif, exchange);
        let order_request = OrderRequest::Create{ account: account.clone(), order: order.clone(), order_type: OrderType::StopMarket};
        if self.mode == StrategyMode::Live {
            self.open_order_cache.insert(order_id.clone(), order.clone());
            let connection_type = ConnectionType::Broker(order_request.brokerage());
            let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest { request: order_request });
            send_request(request).await;
        } else {
            if let Some(historical_message_sender) = &self.historical_message_sender {
                historical_message_sender.send(BackTestEngineMessage::OrderRequest(order_request)).await.unwrap();
            }
        }
        order_id
    }

    /// Will trigger on trigger price but fill only when price is on the correct side of limit price, will partially fill in backtest if we have order book data present.
    pub async fn stop_limit (
        &self,
        symbol_name: &SymbolName,
        symbol_code: Option<SymbolCode>,
        account: &Account,
        exchange: Option<String>,
        quantity: Volume,
        side: OrderSide,
        tag: String,
        limit_price: Price,
        trigger_price: Price,
        tif: TimeInForce
    ) -> OrderId {
        let order_id = self.order_id().await;
        let order = Order::stop_limit(symbol_name.clone(), symbol_code, account, quantity, side, tag, order_id.clone(), self.time_utc(),limit_price, trigger_price, tif, exchange);
        let order_request = OrderRequest::Create{ account: account.clone(), order: order.clone(), order_type: OrderType::StopLimit};
        if self.mode == StrategyMode::Live {
            self.open_order_cache.insert(order_id.clone(), order.clone());
            let connection_type = ConnectionType::Broker(order_request.brokerage());
            let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest { request: order_request });
            send_request(request).await;
        } else {
            if let Some(historical_message_sender) = &self.historical_message_sender {
                historical_message_sender.send(BackTestEngineMessage::OrderRequest(order_request)).await.unwrap();
            }
        }
        order_id
    }

    /// Cancels the order if it is not filled, cancelled or rejected.
    pub async fn cancel_order(&self, order_id: OrderId) {
        // Clone the necessary data from the Ref
        // need a market handler callback fn for this
        let account = if let Some(id_order_ref) = self.open_order_cache.get(&order_id) {
            id_order_ref.account.clone()
        } else {
            return; // Order not found, exit the function
        };

        let order_request = OrderRequest::Cancel {
            order_id,
            account
        };

        if self.mode == StrategyMode::Live {
            let connection_type = ConnectionType::Broker(order_request.brokerage());
            let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest { request: order_request });
            send_request(request).await;
        } else {
            if let Some(historical_message_sender) = &self.historical_message_sender {
                historical_message_sender.send(BackTestEngineMessage::OrderRequest(order_request)).await.unwrap();
            }
        }
    }

    /// Updates the order if it is not filled, cancelled or rejected.
    pub async fn update_order(&self, order_id: OrderId, order_update_type: OrderUpdateType) {
        // Clone the necessary data from the Ref
        //todo need a market handler update for this
        let account = if let Some(id_order_ref) = self.open_order_cache.get(&order_id) {
            id_order_ref.account.clone()
        } else {
            return; // Order not found, exit the function
        };

        let order_request = OrderRequest::Update {
            order_id,
            account,
            update: order_update_type,
        };

        if self.mode == StrategyMode::Live {
            let connection_type = ConnectionType::Broker(order_request.brokerage());
            let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest { request: order_request });
            send_request(request).await;
        } else {
            if let Some(historical_message_sender) = &self.historical_message_sender {
                historical_message_sender.send(BackTestEngineMessage::OrderRequest(order_request)).await.unwrap();
            }
        }
    }

    /// Cancel all pending orders on the account for the symbol_name, this cancels only orders on the symbol which were created by the strategy
    pub async fn cancel_orders(&self, account: Account, symbol_name: SymbolName) {
        for order in self.open_order_cache.iter() {
            if account == order.account && order.symbol_name == symbol_name {
                self.cancel_order(order.id.clone()).await;
            }
        }
    }

    /// if not synchronize_accounts mode, this will cancel only orders placed by the strategy,
    /// if synchronize_accounts mode, this will cancel all orders on the account.
    pub async fn cancel_orders_account(&self, account: Account) {
        if !self.synchronize_accounts || self.mode == StrategyMode::Backtest {
            for order in self.open_order_cache.iter() {
                if account == order.account {
                    self.cancel_order(order.id.clone()).await;
                }
            }
        } else {
            let order_request = OrderRequest::CancelAll {account};
            if self.mode == StrategyMode::Live {
                let connection_type = ConnectionType::Broker(order_request.brokerage());
                let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest { request: order_request });
                send_request(request).await;
            }
        }
    }

    /// Flatten all positions on the account. this will try to flatten the whole account regardless of is_synchronize_accounts.
    pub async fn flatten_all_for(&self, account: Account) {
        if self.mode == StrategyMode::Live {
            let order_request = OrderRequest::FlattenAllFor {account};
            let connection_type = ConnectionType::Broker(order_request.brokerage());
            let request = StrategyRequest::OneWay(connection_type, DataServerRequest::OrderRequest { request: order_request });
            send_request(request).await;
        } else {
            let order_request = OrderRequest::FlattenAllFor {account};
            if let Some(historical_message_sender) = &self.historical_message_sender {
                historical_message_sender.send(BackTestEngineMessage::OrderRequest(order_request)).await.unwrap();
            }
        }
    }

    /// get_requests the last price for the symbol name
    pub async fn last_price(&self, _symbol_name: &SymbolName) -> Option<Price> {
        todo!("send callback to price service")
    }

    /// ALl pending orders on the account.
    pub async fn orders_pending(&self) -> Arc<DashMap<OrderId, Order>> {
        self.open_order_cache.clone()
    }

    pub async fn orders_closed(&self) -> Arc<DashMap<OrderId, Order>> {
        self.closed_order_cache.clone()
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
    /// # Arguments
    /// * `indicator: Box<dyn Indicators>` - The indicator to subscribe to.
    /// * `trading_hours: Option<TradingHours>` - The trading hours for the indicator, this is only used for non-intraday resolutions. for example daily or weekly bars will be constructed based on the market hours.
    /// * `week_start: Option<Weekday>` - The day of the week to start the week on, this is only used for weekly subscriptions.
    pub async fn subscribe_indicator(&self, indicator: Box<dyn Indicators>, trading_hours: Option<TradingHours>) {
        match self.mode {
            StrategyMode::Backtest => {
                self.indicator_handler
                    .add_indicator(indicator, self.time_utc(), trading_hours)
                    .await;
                //add_buffer(self.time_utc(), StrategyEvent::IndicatorEvent(event)).await;
            }
            StrategyMode::Live | StrategyMode::LivePaperTrading => {
                let indicator_handler = self.indicator_handler.clone();
               // tokio::task::spawn(async move {
                    indicator_handler
                        .add_indicator(indicator, Utc::now(), trading_hours)
                        .await;
                   // add_buffer(Utc::now(), StrategyEvent::IndicatorEvent(event)).await;
               // });
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

    /// Returns the account balance
    /// In live rithmic doesn't update the balance until after a position is opened, to avoid a balance of 0 before placing orders we can do this in live trading
    /// ```rust
    /// let balance = strategy.balance(&account);
    ///  if balance != dec!(0) {
    ///     println!("Balance: {}", balance);
    ///     if balance >= MAX_BALANCE || balance <= MIN_BALANCE {
    ///         println!("Balance is too high or too low, flattening all positions: {}", balance);
    ///         if strategy.is_long(&account, &symbol_code) {
    ///             let open_quantity = strategy.position_size(&account, &symbol_code);
    ///             exit_order_id = Some(strategy.exit_long(&candle.symbol.name, Some(symbol_code.clone()), &account, None, open_quantity, "Exit Long Target Reached".to_string()).await);
    ///         }
    ///     break 'strategy_loop;
    ///    }
    ///  }
    /// ```
    pub fn balance(&self, account: &Account) -> Decimal {
        self.ledger_service.balance(account)
    }

    /// see the indicator_enum.rs for more details
    pub fn indicator_history(
        &self,
        name: &IndicatorName,
    ) -> Option<RollingWindow<IndicatorValues>> {
        self.indicator_handler.history(name)
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
    pub async fn subscribe(&self, primary_source: Option<PrimarySubscription>, subscription: DataSubscription, history_to_retain: usize, fill_forward: bool, trading_hours: Option<TradingHours>) {
        match self.mode {
            StrategyMode::Backtest => {
                let _ = self.subscription_handler
                    .subscribe(primary_source, subscription.clone(), self.time_utc(), fill_forward, history_to_retain, true, trading_hours)
                    .await;
            }
            StrategyMode::Live | StrategyMode::LivePaperTrading => {
                let handler = self.subscription_handler.clone();
                //tokio::task::spawn(async move{
                    let _ = handler
                        .subscribe(primary_source, subscription.clone(), Utc::now(), fill_forward, history_to_retain, true, trading_hours)
                        .await;
                //});
            }
        }
    }

    /// Unsubscribes from a subscription.
    pub async fn unsubscribe(&self,subscription: DataSubscription) {
        self.subscription_handler
            .unsubscribe(subscription.clone(), true)
            .await;

        self.indicator_handler
                    .indicators_unsubscribe_subscription(&subscription)
                    .await;
    }

    /// Returns currently open `QuoteBar` for the subscription
    pub fn open_bar(&self, subscription: &DataSubscription) -> Option<QuoteBar> {
        self.subscription_handler.open_bar(subscription)
    }

    /// Returns currently open candle for the subscription
    pub fn open_candle(&self, subscription: &DataSubscription) -> Option<Candle> {
        self.subscription_handler.open_candle(subscription)
    }

    /// Returns `Candle` at the specified index, where 0 is current closed `Candle` and 1 is last closed and 10 closed 10 candles ago (11th).
    pub fn candle_index(&self, subscription: &DataSubscription, index: usize) -> Option<Candle> {
        self.subscription_handler.candle_index(subscription, index)
    }

    /// Returns `QuoteBar` at the specified index, where 0 is current closed `QuoteBar` and 1 is last closed and 10 closed 10 `QuoteBar`s ago (11th).
    pub fn bar_index(&self, subscription: &DataSubscription, index: usize) -> Option<QuoteBar> {
        self.subscription_handler.bar_index(subscription, index)
    }

    /// Returns `Tick` at the specified index, where 0 is current `Tick` and 1 is 2nd last `Tick` and 10 is 10 `Ticks`s ago (11th).
    pub fn tick_index(&self, subscription: &DataSubscription, index: usize) -> Option<Tick> {
        self.subscription_handler.tick_index(subscription, index)
    }

    /// Returns `Quote` at the specified index, where 0 is current `Quote` and 1 is 2nd last `Quote` and 10 is 10 `Quote`s ago (11th).
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
        match is_warmup_complete() {
            true => match self.mode {
                StrategyMode::Backtest => get_backtest_time(),
                _ => Utc::now(),
            },
            false => get_backtest_time(),
        }
    }

    /// Returns a BTreeMap of BaseDataEnum where data.time_closed_utc() is key and data is value.
    /// From the time, to the current strategy time
    pub async fn history_from_local_time(
        &self,
        from_time: NaiveDateTime,
        time_zone: Tz,
        subscription: &DataSubscription,
        trading_hours: Option<TradingHours>,
    ) -> BTreeMap<DateTime<Utc>, BaseDataEnum> {
        let start_date = naive_date_time_to_tz(from_time, time_zone);
        range_history_data(start_date.to_utc(), self.time_utc(), subscription.clone(), self.mode, trading_hours).await
    }

    /// Returns a BTreeMap of BaseDataEnum where data.time_closed_utc() is key and data is value.
    /// From the time, to the current strategy time
    pub async fn history_from_utc_time(
        &self,
        from_time: NaiveDateTime,
        subscription: &DataSubscription,
        trading_hours: Option<TradingHours>,
    ) -> BTreeMap<DateTime<Utc>, BaseDataEnum> {
        let start_date = naive_date_time_to_utc(from_time);
        range_history_data(start_date.to_utc(), self.time_utc(), subscription.clone(), self.mode, trading_hours).await
    }

    /// Returns a BTreeMap of BaseDataEnum where data.time_closed_utc() is key and data is value.
    /// If to time > strategy.time then to time will be changed to strategy.time to avoid lookahead bias
    pub async fn historical_range_from_local_time(
        &self,
        from_time: NaiveDateTime,
        to_time: NaiveDateTime,
        time_zone: Tz,
        subscription: &DataSubscription,
        trading_hours: Option<TradingHours>,
    ) -> BTreeMap<DateTime<Utc>, BaseDataEnum> {
        let start_date = naive_date_time_to_tz(from_time, time_zone);
        let end_date =  naive_date_time_to_tz(to_time, time_zone).to_utc();

        let end_date = match end_date > self.time_utc() {
            true => self.time_utc(),
            false => end_date.to_utc(),
        };

        range_history_data(start_date.to_utc(), end_date, subscription.clone(), self.mode, trading_hours).await
    }

    /// Currently returns only primary data that is available, needs to be updated to be able to return all subscriptions via consolidated data
    pub async fn historical_range_from_utc_time(
        &self,
        from_time: NaiveDateTime,
        to_time: NaiveDateTime,
        subscription: &DataSubscription,
        trading_hours: Option<TradingHours>,
    ) -> BTreeMap<DateTime<Utc>, BaseDataEnum> {
        let start_date = DateTime::<Utc>::from_naive_utc_and_offset(from_time, Utc);
        let end_date = DateTime::<Utc>::from_naive_utc_and_offset(to_time, Utc);

        let end_date = match end_date > self.time_utc() {
            true => self.time_utc(),
            false => end_date,
        };

        range_history_data(start_date.to_utc(), end_date, subscription.clone(), self.mode, trading_hours).await
    }

    /// Prints a ledgers statistics
    pub fn print_ledger(&self, account: &Account) {
        self.ledger_service.print_ledger(account);
    }

    /// Prints all ledgers statistics
    pub fn print_ledgers(&self) {
        self.ledger_service.print_ledgers();
    }

    /// Exports trades to a csv file in the directory
    pub fn export_positions_to_csv(&self, directory: &str) {
        for account_entry in self.ledger_service.ledgers.iter() {
            self.ledger_service.export_positions_to_csv(account_entry.key(), directory);
        }
    }

    /// Exports positions (cumulative) to a csv file in the directory
    pub fn export_trades_to_csv(&self, account: &Account, directory: &str) {
        self.ledger_service.export_trades_to_csv(account, directory);
    }

    /// Save positions to a json file in the directory
    /// Useful for machine learning etc.
    pub fn save_positions_to_file(&self, account: &Account, file_path: &str) {
        self.ledger_service.save_positions_to_file(account, file_path);
    }

    /// Used to load positions from disk when saved as json format
    /// Useful for machine learning
    pub fn load_positions_from_file(file: &str) -> Vec<Position> {
        // Read the file content
        let file_content = fs::read_to_string(file).expect("Failed to read the file");

        // Deserialize the JSON into a Vec<Position>
        let positions: Vec<Position> = serde_json::from_str(&file_content).expect("Failed to parse JSON");

        positions
    }

    /// Get a clone of closed positions for the account
    pub fn get_positions(&self, account: &Account) -> DashMap<SymbolCode, Vec<Position>> {
        self.ledger_service.get_positions(account)
    }

    /// Exports trades (individual) to a csv file in the directory
    pub fn print_trade_statistics(&self, account: &Account) {
        self.ledger_service.print_trade_statistics(account);
    }

    // Updated position query functions
    pub fn in_profit(&self, account: &Account, symbol_name: &SymbolName) -> bool {
        self.ledger_service.in_profit(account, symbol_name)
    }

    /// boolean if the account is in a drawdown on the symbol
    pub fn in_drawdown(&self, account: &Account, symbol_name: &SymbolName) -> bool {
        self.ledger_service.in_drawdown(account, symbol_name)
    }

    /// The accounts open pnl on a symbol
    pub fn pnl(&self, account: &Account, symbol_name: &SymbolName) -> Decimal {
        self.ledger_service.open_pnl_symbol(account, symbol_name)
    }

    /// The accounts open pnl total
    pub fn pnl_total(&self, account: &Account) -> Decimal {
        self.ledger_service.open_pnl(account)
    }

    /// The accounts closed pnl total for the symbol
    pub fn booked_pnl(&self, account: &Account, symbol_name: &SymbolName) -> Decimal {
        self.ledger_service.booked_pnl(account, symbol_name)
    }

    /// The accounts current position size for the symbol
    pub fn position_size(&self, account: &Account, symbol_name: &SymbolName) -> Decimal {
        self.ledger_service.position_size(account, symbol_name)
    }

    pub fn booked_pnl_account(&self, account: &Account) -> Decimal {
        self.ledger_service.booked_pnl_account(account)
    }
}
