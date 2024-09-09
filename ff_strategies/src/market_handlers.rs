use chrono::{DateTime, Utc};
use ff_standard_lib::apis::brokerage::Brokerage;
use ff_standard_lib::standardized_types::accounts::ledgers::{AccountId, Ledger};
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::order_book::{OrderBook, OrderBookUpdate};
use ff_standard_lib::standardized_types::enums::{OrderSide, StrategyMode};
use ff_standard_lib::standardized_types::orders::orders::{
    Order, OrderState, OrderType, OrderUpdateEvent,
};
use ff_standard_lib::standardized_types::strategy_events::{EventTimeSlice, StrategyEvent};
use ff_standard_lib::standardized_types::subscriptions::{SymbolName};
use ff_standard_lib::standardized_types::time_slices::TimeSlice;
use ff_standard_lib::standardized_types::{OwnerId, Price};
use std::collections::{BTreeMap};
use std::sync::{Arc, RwLockWriteGuard};
use dashmap::DashMap;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{Mutex, RwLock};

pub(crate) enum MarketHandlerEnum {
    BacktestDefault(HistoricalMarketHandler),
    LiveDefault(Arc<HistoricalMarketHandler>), //ToDo Will use this later so that live strategies share an event handler.. the local platform will also use this event handler
}

impl MarketHandlerEnum {
    pub async fn new(owner_id: OwnerId, start_time: DateTime<Utc>, mode: StrategyMode, primary_data_receiver: Receiver<MarketHandlerUpdate>, event_sender: Sender<Option<EventTimeSlice>>) -> Self {
        match mode {
            StrategyMode::Backtest => MarketHandlerEnum::BacktestDefault(
                HistoricalMarketHandler::new(owner_id, start_time, primary_data_receiver, event_sender).await,
            ),
            _ => panic!("Live mode not supported yet"),
        }
    }

    pub(crate) async fn last_time(&self) -> DateTime<Utc> {
        match self {
            MarketHandlerEnum::BacktestDefault(handler) => handler.last_time.read().await.clone(),
            MarketHandlerEnum::LiveDefault(_) => {
                panic!("Live mode should not use this function for time")
            }
        }
    }

    pub async fn process_ledgers(&self) -> Vec<String> {
        match self {
            MarketHandlerEnum::BacktestDefault(handler) => handler.process_ledgers().await,
            MarketHandlerEnum::LiveDefault(handler) => handler.process_ledgers().await,
        }
    }

    pub async fn get_order_book(&self, symbol_name: &SymbolName) -> Option<Arc<OrderBook>> {
        match self {
            MarketHandlerEnum::BacktestDefault(handler) => handler.get_order_book(symbol_name).await,
            MarketHandlerEnum::LiveDefault(handler) => handler.get_order_book(symbol_name).await,
        }
    }

    pub async fn is_long(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
        match self {
            MarketHandlerEnum::BacktestDefault(handler) => handler.is_long(brokerage, account_id, symbol_name).await,
            MarketHandlerEnum::LiveDefault(handler) => handler.is_long(brokerage, account_id, symbol_name).await
        }
    }

    pub async fn is_short(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
        match self {
            MarketHandlerEnum::BacktestDefault(handler) =>handler.is_short(brokerage, account_id, symbol_name).await,
            MarketHandlerEnum::LiveDefault(handler) => handler.is_short(brokerage, account_id, symbol_name).await,
        }
    }

    pub async fn is_flat(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
        match self {
            MarketHandlerEnum::BacktestDefault(handler) => handler.is_flat(brokerage, account_id, symbol_name).await,
            MarketHandlerEnum::LiveDefault(handler) => handler.is_flat(brokerage, account_id, symbol_name).await
        }
    }
}


pub enum MarketHandlerUpdate {
    Time(DateTime<Utc>),
    TimeSlice(TimeSlice),
    Order(Order)
}

pub(crate) struct HistoricalMarketHandler {
    owner_id: String,
    /// The strategy receives its timeslices using strategy events, this is for other processes that need time slices and do not need synchronisation with the strategy
    /// These time slices are sent before they are sent to the strategy as events
    //
    // 3. Option 1 is the best, In live trading we will be selecting the brokerage, the vendor is irrelevant, and we will ofcourse chose the best price, os maybe here we should use best bid and best ask.
    order_books: Arc<DashMap<SymbolName, Arc<OrderBook>>>,
    last_price: Arc<DashMap<SymbolName, Price>>,
    ledgers: Arc<DashMap<Brokerage, DashMap<AccountId, Ledger>>>,
    last_time: Arc<RwLock<DateTime<Utc>>>,
    order_cache: Arc<RwLock<Vec<Order>>>,
}

impl HistoricalMarketHandler {
    pub(crate) async fn new(
        owner_id: OwnerId,
        start_time: DateTime<Utc>,
        primary_data_receiver: Receiver<MarketHandlerUpdate>,
        event_sender: Sender<Option<EventTimeSlice>>
    ) -> Self {
        let handler = Self {
            owner_id,
            order_books: Default::default(),
            last_price: Default::default(),
            ledgers: Default::default(),
            last_time: Arc::new(RwLock::new(start_time)),
            order_cache: Default::default(),
        };
        handler.on_data_update(primary_data_receiver, event_sender).await;
        handler
    }

    pub async fn get_order_book(&self, symbol_name: &SymbolName) -> Option<Arc<OrderBook>> {
        if let Some(book) = self.order_books.get(symbol_name) {
            return Some(book.clone());
        }
        None
    }

    /// only primary data gets to here, so we can update our order books etc
    pub(crate) async fn on_data_update(&self, primary_data_receiver: Receiver<MarketHandlerUpdate>, event_sender: Sender<Option<EventTimeSlice>>) {
        let mut primary_data_receiver = primary_data_receiver;
        let mut last_time = self.last_time.clone();
        let ledgers = self.ledgers.clone();
        let order_books = self.order_books.clone();
        let last_price = self.last_price.clone();
        let event_sender = event_sender;
        let owner_id: OwnerId = self.owner_id.clone();
        let order_cache = self.order_cache.clone();

        tokio::task::spawn(async move {
            while let Some(update) = primary_data_receiver.recv().await {
                match update {
                    MarketHandlerUpdate::Time(time) => {
                        *last_time.write().await = time;
                    }
                    MarketHandlerUpdate::TimeSlice(time_slice) => {
                        for brokerage_map in ledgers.iter() {
                            for ledger in brokerage_map.value() {
                                ledger.value().on_data_update(time_slice.clone()).await;
                            }
                        }
                        for base_data in time_slice {
                            //println!("Base data: {:?}", base_data.time_created_utc());
                            match base_data {
                                BaseDataEnum::Price(ref price) => {
                                    last_price.insert(price.symbol.name.clone(), price.price);
                                }
                                BaseDataEnum::Candle(ref candle) => {
                                    last_price.insert(candle.symbol.name.clone(), candle.close);
                                }
                                BaseDataEnum::QuoteBar(ref bar) => {
                                    if !order_books.contains_key(&bar.symbol.name) {
                                        order_books.insert(
                                            bar.symbol.name.clone(),
                                            Arc::new(OrderBook::new(bar.symbol.clone(), bar.time_utc())),
                                        );
                                    }
                                    if let Some(book) = order_books.get_mut(&bar.symbol.name) {
                                        let mut bid = BTreeMap::new();
                                        bid.insert(0, bar.bid_close.clone());
                                        let mut ask = BTreeMap::new();
                                        ask.insert(0, bar.ask_close.clone());
                                        let order_book_update =
                                            OrderBookUpdate::new(bar.symbol.clone(), bid, ask, bar.time_utc());
                                        book.update(order_book_update).await;
                                    }
                                }
                                BaseDataEnum::Tick(ref tick) => {
                                    last_price.insert(tick.symbol.name.clone(), tick.price);
                                }
                                BaseDataEnum::Quote(ref quote) => {
                                    if !order_books.contains_key(&quote.symbol.name) {
                                        order_books.insert(
                                            quote.symbol.name.clone(),
                                            Arc::new(OrderBook::new(quote.symbol.clone(), quote.time_utc())),
                                        );
                                    }
                                    if let Some(book) = order_books.get_mut(&quote.symbol.name) {
                                        let mut bid = BTreeMap::new();
                                        bid.insert(quote.book_level.clone(), quote.bid.clone());
                                        let mut ask = BTreeMap::new();
                                        ask.insert(quote.book_level.clone(), quote.ask.clone());
                                        let order_book_update =
                                            OrderBookUpdate::new(quote.symbol.clone(), bid, ask, quote.time_utc());
                                        book.update(order_book_update).await;
                                    }
                                }
                                BaseDataEnum::Fundamental(_) => (),
                            }
                        }
                        event_sender.send(backtest_matching_engine(&owner_id, order_books.clone(), last_price.clone(), ledgers.clone(), last_time.clone(), order_cache.clone()).await).await.unwrap();
                    }
                    MarketHandlerUpdate::Order(order) => {
                        order_cache.write().await.push(order);
                    }
                }
            }
        });
    }

    pub(crate) async fn process_ledgers(&self) -> Vec<String> {
        // Acquire a read lock on the RwLock
        // Iterate over the HashMap while holding the read lock
        let mut return_strings = vec![];
        for brokerage_map in self.ledgers.iter() {
            for map in brokerage_map.iter() {
                return_strings.push(format!(
                    "Brokerage: {:?} Account: {:?} Ledger: {:?}",
                    brokerage_map.key(), map.key().clone(), map.value().clone()
                ));
            }
        }
        return_strings
    }

    pub async fn is_long(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
        if let Some(ledger_map) = self.ledgers.get(brokerage) {
            if let Some(broker_ledgers) = ledger_map.get(account_id) {
                return broker_ledgers.value().is_long(symbol_name).await
            }
            return false
        }
        false
    }

    pub async fn is_short(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
        if let Some(ledger_map) = self.ledgers.get(brokerage) {
            if let Some(broker_ledgers) = ledger_map.get(account_id) {
                return broker_ledgers.value().is_short(symbol_name).await
            }
            return false
        }
        false
    }

    pub async fn is_flat(&self, brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
        if let Some(ledger_map) = self.ledgers.get(brokerage) {
            if let Some(broker_ledgers) = ledger_map.get(account_id) {
                return broker_ledgers.value().is_flat(symbol_name).await
            }
            return true
        }
        true
    }
}

async fn backtest_matching_engine(
    owner_id: &OwnerId,
    order_books: Arc<DashMap<SymbolName, Arc<OrderBook>>>,
    last_price: Arc<DashMap<SymbolName, Price>>,
    ledgers: Arc<DashMap<Brokerage, DashMap<AccountId, Ledger>>>,
    last_time: Arc<RwLock<DateTime<Utc>>>,
    order_cache: Arc<RwLock<Vec<Order>>>,
) -> Option<EventTimeSlice> {
    let mut order_cache = order_cache.write().await;
    if order_cache.len() == 0 {
        return None;
    }

    let mut events = Vec::new();
    let orders = &mut *order_cache;
    let remaining_orders: Vec<Order> = Vec::new();
    for order in &mut *orders {
        //1. If we don't have a brokerage + account create one
        if !ledgers.contains_key(&order.brokerage) {
            ledgers
                .insert(order.brokerage.clone(), DashMap::new());
        }
        if !ledgers
            .get(&order.brokerage)
            .unwrap()
            .contains_key(&order.account_id)
        {
            let ledger = order
                .brokerage
                .paper_ledger(
                    order.account_id.clone(),
                    order.brokerage.user_currency(),
                    order.brokerage.starting_cash(),
                )
                .await;
            ledgers
                .get_mut(&order.brokerage)
                .unwrap()
                .insert(order.account_id.clone(), ledger);
        }

        //2. send the order to the ledger to be handled
        let mut brokerage_ledger = ledgers
            .get_mut(&order.brokerage)
            .unwrap();
        let mut ledger = brokerage_ledger
            .get_mut(&order.account_id)
            .unwrap();

        let owner_id = owner_id.clone();

        //3. respond with an order event
        match order.order_type {
            OrderType::Limit => {
                // todo! for limit orders that are not hit we need to push the order into remaining orders, so that it can be retained
                panic!("Limit orders not supported in backtest")
            }
            OrderType::Market => panic!("Market orders not supported in backtest"),
            OrderType::MarketIfTouched => {
                panic!("MarketIfTouched orders not supported in backtest")
            }
            OrderType::StopMarket => panic!("StopMarket orders not supported in backtest"),
            OrderType::StopLimit => panic!("StopLimit orders not supported in backtest"),
            OrderType::TakeProfit => panic!("TakeProfit orders not supported in backtest"),
            OrderType::StopLoss => panic!("StopLoss orders not supported in backtest"),
            OrderType::GuaranteedStopLoss => {
                panic!("GuaranteedStopLoss orders not supported in backtest")
            }
            OrderType::TrailingStopLoss => {
                panic!("TrailingStopLoss orders not supported in backtest")
            }
            OrderType::TrailingGuaranteedStopLoss => {
                panic!("TrailingGuaranteedStopLoss orders not supported in backtest")
            }
            OrderType::EnterLong => {
                if ledger.is_short(&order.symbol_name).await {
                    ledger.exit_position_paper(&order.symbol_name).await;
                }
                let market_price = get_market_price(&OrderSide::Buy, &order.symbol_name, order_books.clone(), last_price.clone()).await.unwrap();
                match ledger
                    .enter_long_paper(
                        &order.symbol_name,
                        order.quantity_ordered.clone(),
                        market_price,
                    )
                    .await
                {
                    Ok(_) => {
                        order.state = OrderState::Filled;
                        order.average_fill_price = Some(market_price);
                        events.push(StrategyEvent::OrderEvents(
                            owner_id.clone(),
                            OrderUpdateEvent::Filled(order.clone()),
                        ));
                        println!("Entered Long: {}, balance: {}", order.symbol_name, ledger.cash_available)
                    }
                    Err(e) => {
                        let reason = format!("Failed to enter long position: {:?}", e);
                        order.state = OrderState::Rejected(reason);
                        order.time_created_utc = last_time.read().await.to_string();
                        events.push(StrategyEvent::OrderEvents(
                            owner_id.clone(),
                            OrderUpdateEvent::Rejected(order.clone()),
                        ));
                    }
                }
            }
            OrderType::EnterShort => {
                if ledger.is_long(&order.symbol_name).await {
                    ledger.exit_position_paper(&order.symbol_name).await;
                }
                let market_price = get_market_price(&OrderSide::Sell, &order.symbol_name, order_books.clone(), last_price.clone())
                    .await
                    .unwrap();
                match ledger
                    .enter_short_paper(
                        &order.symbol_name,
                        order.quantity_ordered.clone(),
                        market_price,
                    )
                    .await
                {
                    Ok(_) => {
                        order.state = OrderState::Filled;
                        order.average_fill_price = Some(market_price);
                        events.push(StrategyEvent::OrderEvents(
                            owner_id.clone(),
                            OrderUpdateEvent::Filled(order.clone()),
                        ));
                        println!("Entered Short: {}, balance: {}", order.symbol_name, ledger.cash_available)
                    }
                    Err(e) => {
                        let reason = format!("Failed to enter short position: {:?}", e);
                        order.state = OrderState::Rejected(reason);
                        order.time_created_utc = last_time.read().await.to_string();
                        events.push(StrategyEvent::OrderEvents(
                            owner_id.clone(),
                            OrderUpdateEvent::Rejected(order.clone()),
                        ));
                    }
                }
            }
            OrderType::ExitLong => {
                if ledger.is_long(&order.symbol_name).await {
                    order.state = OrderState::Filled;
                    let market_price = get_market_price(&OrderSide::Sell, &order.symbol_name, order_books.clone(), last_price.clone())
                        .await
                        .unwrap();
                    order.average_fill_price = Some(market_price);
                    events.push(StrategyEvent::OrderEvents(
                        owner_id.clone(),
                        OrderUpdateEvent::Filled(order.clone()),
                    ));
                } else {
                    let reason = "No long position to exit".to_string();
                    order.state = OrderState::Rejected(reason);
                    order.time_created_utc = last_time.read().await.to_string();
                    events.push(StrategyEvent::OrderEvents(
                        owner_id.clone(),
                        OrderUpdateEvent::Rejected(order.clone()),
                    ));
                }
            }
            OrderType::ExitShort => {
                if ledger.is_short(&order.symbol_name).await {
                    order.state = OrderState::Filled;
                    let market_price = get_market_price(&OrderSide::Buy, &order.symbol_name, order_books.clone(), last_price.clone())
                        .await
                        .unwrap();
                    order.average_fill_price = Some(market_price);
                    events.push(StrategyEvent::OrderEvents(
                        owner_id.clone(),
                        OrderUpdateEvent::Filled(order.clone()),
                    ));
                } else {
                    let reason = "No short position to exit".to_string();
                    order.state = OrderState::Rejected(reason);
                    order.time_created_utc = last_time.read().await.to_string();
                    events.push(StrategyEvent::OrderEvents(
                        owner_id.clone(),
                        OrderUpdateEvent::Rejected(order.clone()),
                    ));
                }
            }
        };
    }
    *orders = remaining_orders;
    if events.len() == 0 {
        None
    } else {
        Some(events)
    }
}

async fn get_market_price(
    order_side: &OrderSide,
    symbol_name: &SymbolName,
    order_books: Arc<DashMap<SymbolName, Arc<OrderBook>>>,
    last_price: Arc<DashMap<SymbolName, Price>>,
) -> Result<Price, String> {

    if let Some(book) = order_books.get(symbol_name) {
        match order_side {
            OrderSide::Buy => {
                if let Some(ask_price) = book.ask_level(0).await {
                    return Ok(ask_price);
                }
            }
            OrderSide::Sell => {
                if let Some(bid_price) = book.bid_level(0).await {
                    return Ok(bid_price);
                }
            }
        }
    } else if let Some(last_price) = last_price.get(symbol_name) {
        return Ok(last_price.value().clone());
    }
    Err(String::from("No market price found for symbol"))
}