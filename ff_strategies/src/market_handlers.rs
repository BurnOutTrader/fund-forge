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
use ff_standard_lib::standardized_types::data_server_messaging::FundForgeError;

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
            let mut event_buffer = EventTimeSlice::new();
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
                        match backtest_matching_engine(&owner_id, order_books.clone(), last_price.clone(), ledgers.clone(), last_time.clone(), order_cache.clone()).await {
                            None => {},
                            Some(event) => event_buffer.extend(event)
                        }
                        if event_buffer.len() > 0 {
                            event_sender.send(Some(event_buffer.clone())).await.unwrap();
                            event_buffer = EventTimeSlice::new();
                        } else {
                            event_sender.send(None).await.unwrap();
                        }

                    }
                    MarketHandlerUpdate::Order(order) => {
                        order_cache.write().await.push(order);
                        match backtest_matching_engine(&owner_id, order_books.clone(), last_price.clone(), ledgers.clone(), last_time.clone(), order_cache.clone()).await {
                            None => {},
                            Some(event) => event_buffer.extend(event)
                        }
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
                return_strings.push(map.value().print());
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
    let mut remaining_orders: Vec<Order> = Vec::new();
    for order in &mut *orders {
        //1. If we don't have a brokerage + account create one
        if !ledgers.contains_key(&order.brokerage) {
            ledgers
                .insert(order.brokerage.clone(), DashMap::new());
        }
        if !ledgers
            .get(&order.brokerage)?
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
                .get_mut(&order.brokerage)?
                .insert(order.account_id.clone(), ledger);
            println!("Ledger Created")
        }

        //2. send the order to the ledger to be handled
        let mut brokerage_ledger = ledgers
            .get_mut(&order.brokerage)?;
        let mut ledger = brokerage_ledger
            .get_mut(&order.account_id)?;

        let owner_id = owner_id.clone();
        let market_price = get_market_price(&order.side, &order.symbol_name, order_books.clone(), last_price.clone()).await.unwrap();
        let last_time_utc = last_time.read().await.clone();

        let fill_order = | mut order: Order, events: &mut Vec<StrategyEvent> | {
            order.time_filled_utc = Some(last_time_utc.to_string());
            order.state = OrderState::Filled;
            order.average_fill_price = Some(market_price);
            events.push(StrategyEvent::OrderEvents(
                owner_id.clone(),
                OrderUpdateEvent::Filled(order.clone()),
            ));
        };

        let reject_order = |reason: String, mut order: Order, events: &mut Vec<StrategyEvent>| {
            order.state = OrderState::Rejected(reason);
            order.time_created_utc = last_time_utc.to_string();
            events.push(StrategyEvent::OrderEvents(
                owner_id.to_string(),
                OrderUpdateEvent::Rejected(order.clone()),
            ));
        };

        //3. respond with an order event

        match order.order_type {
            OrderType::Limit | OrderType::TakeProfit => {
                let is_filled = match order.side {
                    OrderSide::Buy => market_price <= order.limit_price?,
                    OrderSide::Sell => market_price >= order.limit_price?
                };
                if is_filled {
                    order.quantity_filled = order.quantity_ordered;
                    order.time_filled_utc = Some(last_time_utc.to_string());
                    match ledger.update_or_create_paper_position(&order.symbol_name.clone(), order.quantity_filled, market_price, order.side, last_time_utc).await {
                        Ok(_) => fill_order(order.clone(), &mut events),
                        Err(e) => reject_order("Insufficient Margin".to_string(), order.clone(), &mut events)
                    }
                } else {
                    remaining_orders.push(order.clone());
                }
            }
            OrderType::Market => {
                match ledger.update_or_create_paper_position(&order.symbol_name.clone(), order.quantity_filled, market_price, order.side, last_time_utc).await {
                    Ok(_) => fill_order(order.clone(), &mut events),
                    Err(e) => reject_order("Insufficient Margin".to_string(), order.clone(), &mut events)
                }
            },
            OrderType::MarketIfTouched | OrderType::StopMarket | OrderType::StopLoss | OrderType::GuaranteedStopLoss | OrderType::TrailingStopLoss | OrderType::TrailingGuaranteedStopLoss => {
                let is_filled = match order.side {
                    OrderSide::Buy => market_price <= order.trigger_price?,
                    OrderSide::Sell => market_price >= order.trigger_price?
                };
                if is_filled {
                    order.quantity_filled = order.quantity_ordered;
                    order.time_filled_utc = Some(last_time_utc.to_string());
                    match ledger.update_or_create_paper_position(&order.symbol_name.clone(), order.quantity_filled, market_price, order.side, last_time_utc).await {
                        Ok(_) => fill_order(order.clone(), &mut events),
                        Err(e) => reject_order("Insufficient Margin".to_string(), order.clone(), &mut events)
                    }
                } else {
                    remaining_orders.push(order.clone());
                }
            }
            OrderType::StopLimit => {
                let is_filled = match order.side {
                    OrderSide::Buy => {
                        market_price <= order.trigger_price? && market_price > order.limit_price?
                    },
                    OrderSide::Sell => market_price >= order.trigger_price? && market_price < order.limit_price?
                };
                if is_filled {
                    order.quantity_filled = order.quantity_ordered;
                    order.time_filled_utc = Some(last_time_utc.to_string());
                    match ledger.update_or_create_paper_position(&order.symbol_name.clone(), order.quantity_filled, market_price, order.side, last_time_utc).await {
                        Ok(_) => fill_order(order.clone(), &mut events),
                        Err(e) => reject_order("Insufficient Margin".to_string(), order.clone(), &mut events)
                    }
                } else {
                    remaining_orders.push(order.clone());
                }
            },
            OrderType::EnterLong => {
                if ledger.is_short(&order.symbol_name).await {
                    ledger.exit_position_paper(&order.symbol_name).await;
                }
                match ledger.update_or_create_paper_position(&order.symbol_name.clone(), order.quantity_filled, market_price, order.side, last_time_utc).await {
                    Ok(_) => {
                        fill_order(order.clone(), &mut events);
                    }
                    Err(_) =>  {
                        let reason = "Insufficient Margin".to_string();
                        reject_order(reason, order.clone(), &mut events)
                    }
                }
            }
            OrderType::EnterShort => {
                if ledger.is_long(&order.symbol_name).await {
                    ledger.exit_position_paper(&order.symbol_name).await;
                }

                match ledger.update_or_create_paper_position(&order.symbol_name.clone(), order.quantity_filled, market_price, order.side, last_time_utc).await {
                    Ok(_) => fill_order(order.clone(), &mut events),
                    Err(_) =>  {
                        let reason = "Insufficient Margin".to_string();
                        reject_order(reason, order.clone(), &mut events)
                    }
                }
            }
            OrderType::ExitLong => {
                if ledger.is_long(&order.symbol_name).await {
                    match ledger.update_or_create_paper_position(&order.symbol_name.clone(), order.quantity_filled, market_price, order.side, last_time_utc).await {
                        Ok(_) => fill_order(order.clone(), &mut events),
                        Err(_) =>  {
                            let reason = "Insufficient Margin".to_string();
                            reject_order(reason, order.clone(), &mut events)
                        }
                    }
                } else {
                    let reason = "No long position to exit".to_string();
                    reject_order(reason, order.clone(), &mut events);
                }
            }
            OrderType::ExitShort => {
                if ledger.is_short(&order.symbol_name).await {
                    match ledger.update_or_create_paper_position(&order.symbol_name.clone(), order.quantity_filled, market_price, order.side, last_time_utc).await {
                        Ok(_) => fill_order(order.clone(), &mut events),
                        Err(_) =>  {
                            let reason = "Insufficient Margin".to_string();
                            reject_order(reason, order.clone(), &mut events)
                        }
                    }
                } else {
                    let reason = "No short position to exit".to_string();
                    reject_order(reason, order.clone(), &mut events);
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