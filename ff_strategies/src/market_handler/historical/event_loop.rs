use ff_standard_lib::standardized_types::{OwnerId, Price};
use std::sync::Arc;
use dashmap::DashMap;
use ff_standard_lib::apis::brokerage::Brokerage;
use ff_standard_lib::standardized_types::accounts::ledgers::{AccountId, Ledger};
use ff_standard_lib::standardized_types::base_data::order_book::{OrderBook, OrderBookUpdate};
use ff_standard_lib::standardized_types::subscriptions::SymbolName;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use ff_standard_lib::standardized_types::orders::orders::{Order, OrderUpdateEvent};
use ff_standard_lib::standardized_types::strategy_events::{EventTimeSlice, StrategyEvent};
use tokio::sync::mpsc::{Receiver, Sender};
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use std::collections::BTreeMap;
use futures::future::join_all;
use crate::market_handler::historical::order_matching;
use crate::market_handler::market_handlers::MarketHandlerUpdate;

pub async fn historical_engine(
    owner_id: OwnerId,
    mut primary_data_receiver: Receiver<MarketHandlerUpdate>,
    event_sender: Sender<Option<EventTimeSlice>>,
    order_books: Arc<DashMap<SymbolName, Arc<OrderBook>>>,
    last_price: Arc<DashMap<SymbolName, Price>>,
    ledgers: Arc<DashMap<Brokerage, Arc<DashMap<AccountId, Ledger>>>>,
    last_time: Arc<RwLock<DateTime<Utc>>>,
    order_cache: Arc<RwLock<Vec<Order>>>,
) {
    tokio::task::spawn(async move {
        let mut event_buffer = EventTimeSlice::new();
        while let Some(update) = primary_data_receiver.recv().await {
            match update {
                MarketHandlerUpdate::Time(time) => {
                    *last_time.write().await = time;
                }
                MarketHandlerUpdate::TimeSlice(time_slice) => {
                    let last_time_utc = last_time.read().await.clone();
                    let mut updates = vec![];

                    // update ledgers concurrently per brokerage, this slows us down when only testing 1 ledger, but when using multi broker it will be much faster than running as a single task.
                    for mut brokerage_map in ledgers.iter() {
                        let brokerage_map_ref = brokerage_map.value().clone();
                        let time_slice_ref = time_slice.clone();
                        let last_time_ref = last_time_utc.clone();
                        let update_future = tokio::spawn(async move {
                            for mut ledger in brokerage_map_ref.iter_mut() {
                                ledger.on_data_update(time_slice_ref.clone(), &last_time_ref).await;
                            }
                        });
                        updates.push(update_future);
                    }

                    let last_price_ref = last_price.clone();
                    let order_book_ref = order_books.clone();
                    let update_future = tokio::spawn(async move {
                        for base_data in time_slice {
                            match base_data {
                                BaseDataEnum::Price(ref price) => {
                                    last_price_ref.insert(price.symbol.name.clone(), price.price);
                                }
                                BaseDataEnum::Candle(ref candle) => {
                                    last_price_ref.insert(candle.symbol.name.clone(), candle.close);
                                }
                                BaseDataEnum::QuoteBar(ref bar) => {
                                    if !order_book_ref.contains_key(&bar.symbol.name) {
                                        order_book_ref.insert(
                                            bar.symbol.name.clone(),
                                            Arc::new(OrderBook::new(bar.symbol.clone(), bar.time_utc())),
                                        );
                                    }
                                    if let Some(book) = order_book_ref.get_mut(&bar.symbol.name) {
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
                                    last_price_ref.insert(tick.symbol.name.clone(), tick.price);
                                }
                                BaseDataEnum::Quote(ref quote) => {
                                    if !order_book_ref.contains_key(&quote.symbol.name) {
                                        order_book_ref.insert(
                                            quote.symbol.name.clone(),
                                            Arc::new(OrderBook::new(quote.symbol.clone(), quote.time_utc())),
                                        );
                                    }
                                    if let Some(book) = order_book_ref.get_mut(&quote.symbol.name) {
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
                    });
                    updates.push(update_future);
                    join_all(updates).await;
                    match order_matching::backtest_matching_engine(&owner_id, order_books.clone(), last_price.clone(), ledgers.clone(), last_time.clone(), order_cache.clone()).await {
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
                    match order_matching::backtest_matching_engine(&owner_id, order_books.clone(), last_price.clone(), ledgers.clone(), last_time.clone(), order_cache.clone()).await {
                        None => {},
                        Some(event) => event_buffer.extend(event)
                    }
                }
                MarketHandlerUpdate::CancelOrder(id) => {
                    let mut existing_order: Option<Order> = None;
                    let mut cache = order_cache.write().await;
                    'order_search: for order in &*cache {
                        if order.id == id {
                            existing_order = Some(order.clone());
                            break 'order_search;
                        }
                    }
                    if let Some(order) = existing_order {
                        cache.retain(|x | x.id != id );
                        let cancel_event = StrategyEvent::OrderEvents(order.owner_id.clone(), OrderUpdateEvent::Cancelled(order.id));
                        event_buffer.push(cancel_event);
                    } else {
                        let fail_event = StrategyEvent::OrderEvents(owner_id.clone(), OrderUpdateEvent::UpdateRejected{id, reason: String::from("No pending order found")});
                        event_buffer.push(fail_event);
                    }
                }
                MarketHandlerUpdate::UpdateOrder(id, order) => {
                    let mut existing_order: Option<Order> = None;
                    let mut cache = order_cache.write().await;
                    'order_search: for order in &*cache {
                        if order.id == id {
                            existing_order = Some(order.clone());
                            break 'order_search;
                        }
                    }
                    if let Some(_) = existing_order {
                        cache.retain(|x | x.id != id );
                        let update_event = StrategyEvent::OrderEvents(order.owner_id.clone(), OrderUpdateEvent::Updated(order.id.clone()));
                        cache.push(order);
                        event_buffer.push(update_event);
                    } else {
                        let fail_event = StrategyEvent::OrderEvents(owner_id.clone(), OrderUpdateEvent::UpdateRejected{id, reason: String::from("No pending order found")});
                        event_buffer.push(fail_event);
                    }
                    match order_matching::backtest_matching_engine(&owner_id, order_books.clone(), last_price.clone(), ledgers.clone(), last_time.clone(), order_cache.clone()).await {
                        None => {},
                        Some(event) => event_buffer.extend(event)
                    }
                }
            }
        }
    });
}


