use chrono::{DateTime, Utc};
use ff_standard_lib::apis::brokerage::Brokerage;
use ff_standard_lib::standardized_types::accounts::ledgers::{AccountId, Ledger};
use ff_standard_lib::standardized_types::base_data::order_book::OrderBook;
use ff_standard_lib::standardized_types::enums::{OrderSide, StrategyMode};
use ff_standard_lib::standardized_types::orders::orders::{Order, OrderId};
use ff_standard_lib::standardized_types::strategy_events::EventTimeSlice;
use ff_standard_lib::standardized_types::subscriptions::SymbolName;
use ff_standard_lib::standardized_types::time_slices::TimeSlice;
use ff_standard_lib::standardized_types::{OwnerId, Price};
use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::RwLock;
use crate::market_handler::historical::event_loop;

pub enum MarketHandlerUpdate {
    Time(DateTime<Utc>),
    TimeSlice(TimeSlice),
    Order(Order),
    CancelOrder(OrderId),
    UpdateOrder(OrderId, Order)
}

pub(crate) struct MarketHandler {
    owner_id: String,
    order_books: Arc<DashMap<SymbolName, Arc<OrderBook>>>,
    last_price: Arc<DashMap<SymbolName, Price>>,
    ledgers: Arc<DashMap<Brokerage, Arc<DashMap<AccountId, Ledger>>>>,
    last_time: Arc<RwLock<DateTime<Utc>>>,
    order_cache: Arc<RwLock<Vec<Order>>>,
    /// Links the local order ID to a broker order ID
    id_map: Arc<DashMap<OrderId, String>>
}

impl MarketHandler {
    pub(crate) async fn new(
        owner_id: OwnerId,
        start_time: DateTime<Utc>,
        primary_data_receiver: Receiver<MarketHandlerUpdate>,
        event_sender: Sender<Option<EventTimeSlice>>,
        mode: StrategyMode
    ) -> Self {
        let handler = Self {
            owner_id,
            order_books: Arc::new(DashMap::new()),
            last_price: Arc::new(DashMap::new()),
            ledgers: Arc::new(DashMap::new()),
            last_time: Arc::new(RwLock::new(start_time)),
            order_cache: Arc::new(RwLock::new(Vec::new())),
            id_map: Arc::new(DashMap::new())
        };
        handler.on_data_update(mode, primary_data_receiver, event_sender).await;
        handler
    }

    pub async fn get_order_book(&self, symbol_name: &SymbolName) -> Option<Arc<OrderBook>> {
        if let Some(book) = self.order_books.get(symbol_name) {
            return Some(book.clone());
        }
        None
    }

    pub async fn get_last_price(&self, symbol_name: &SymbolName) -> Option<Price> {
        if let Some(price) = self.last_price.get(symbol_name) {
            return Some(price.value().clone());
        }
        None
    }

    pub async fn get_pending_orders(&self) -> Vec<Order> {
        self.order_cache.read().await.clone()
    }

    pub async fn get_last_time(&self) -> DateTime<Utc> {
        self.last_time.read().await.clone()
    }

    /// only primary data gets to here, so we can update our order books etc
    pub(crate) async fn on_data_update(&self, mode: StrategyMode, primary_data_receiver: Receiver<MarketHandlerUpdate>, event_sender: Sender<Option<EventTimeSlice>>) {
        match mode {
            StrategyMode::Backtest => event_loop::historical_engine(self.owner_id.clone(), primary_data_receiver, event_sender, self.order_books.clone(), self.last_price.clone(), self.ledgers.clone(), self.last_time.clone(), self.order_cache.clone()).await,
            StrategyMode::Live => panic!("Not implemented"),
            StrategyMode::LivePaperTrading => panic!("Not implemented"),
        }
    }

    pub(crate) async fn process_ledgers(&self) -> Vec<String> {
        // Acquire a read lock on the RwLock
        // Iterate over the HashMap while holding the read lock
        let mut return_strings = vec![];
        for brokerage_map in self.ledgers.iter() {
            for ledger in brokerage_map.iter() {
                return_strings.push(format!("{} \n", ledger.value().print()));
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

pub(crate) async fn get_market_price(
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

