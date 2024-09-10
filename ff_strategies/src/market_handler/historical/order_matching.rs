use ff_standard_lib::standardized_types::{OwnerId, Price};
use std::sync::Arc;
use dashmap::DashMap;
use ff_standard_lib::apis::brokerage::Brokerage;
use ff_standard_lib::standardized_types::accounts::ledgers::{AccountId, Ledger};
use ff_standard_lib::standardized_types::base_data::order_book::OrderBook;
use ff_standard_lib::standardized_types::subscriptions::SymbolName;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use ff_standard_lib::standardized_types::enums::OrderSide;
use ff_standard_lib::standardized_types::orders::orders::{Order, OrderState, OrderType, OrderUpdateEvent, ProtectiveOrder};
use ff_standard_lib::standardized_types::strategy_events::{EventTimeSlice, StrategyEvent};
use crate::market_handler::market_handlers::get_market_price;

pub async fn backtest_matching_engine(
    owner_id: &OwnerId,
    order_books: Arc<DashMap<SymbolName, Arc<OrderBook>>>,
    last_price: Arc<DashMap<SymbolName, Price>>,
    ledgers: Arc<DashMap<Brokerage, Arc<DashMap<AccountId, Ledger>>>>,
    last_time: Arc<RwLock<DateTime<Utc>>>,
    order_cache: Arc<RwLock<Vec<Order>>>,
) -> Option<EventTimeSlice> {
    let mut order_cache= order_cache.write().await;
    if order_cache.len() == 0 {
        return None;
    }

    let fill_order = | mut order: Order, last_time_utc: DateTime<Utc>, market_price: Price, events: &mut Vec<StrategyEvent> | {
        order.time_filled_utc = Some(last_time_utc.to_string());
        order.state = OrderState::Filled;
        order.average_fill_price = Some(market_price);
        order.quantity_filled = order.quantity_ordered;
        order.time_filled_utc = Some(last_time_utc.to_string());
        events.push(StrategyEvent::OrderEvents(
            owner_id.clone(),
            OrderUpdateEvent::Filled(order.id),
        ));
    };

    let reject_order = |reason: String, mut order: Order, last_time_utc: DateTime<Utc>, events: &mut Vec<StrategyEvent>| {
        order.state = OrderState::Rejected(reason.clone());
        order.time_created_utc = last_time_utc.to_string();
        events.push(StrategyEvent::OrderEvents(
            owner_id.to_string(),
            OrderUpdateEvent::Rejected{id: order.id, reason},
        ));
    };

    let mut remaining_orders: Vec<Order> = Vec::new();
    let accept_order = |mut order: Order, last_time_utc: DateTime<Utc>, events: &mut Vec<StrategyEvent>, remaining_orders: &mut Vec<Order>| {
        order.state = OrderState::Accepted;
        order.time_created_utc = last_time_utc.to_string();
        events.push(StrategyEvent::OrderEvents(
            owner_id.to_string(),
            OrderUpdateEvent::Accepted(order.id.clone()),
        ));
        remaining_orders.push(order);
    };

    let mut events = Vec::new();
    let orders = &mut *order_cache;
    'order_loop: for order in &mut *orders {
        //1. If we don't have a brokerage + account create one
        if !ledgers.contains_key(&order.brokerage) {
            ledgers
                .insert(order.brokerage.clone(), Arc::new(DashMap::new()));
        }
        if !ledgers
            .get(&order.brokerage)?.value()
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
                .get_mut(&order.brokerage)?.value_mut()
                .insert(order.account_id.clone(), ledger);
        }

        //2. send the order to the ledger to be handled
        let mut brokerage_map = ledgers.get_mut(&order.brokerage)?;

        let mut account_ledger = brokerage_map.value_mut().get_mut(&order.account_id)?;

        let market_price = get_market_price(&order.side, &order.symbol_name, order_books.clone(), last_price.clone()).await.unwrap();
        let last_time_utc = last_time.read().await.clone();

        //3. respond with an order event
        let mut is_fill_triggered = false;
        let mut brackets: Option<Vec<ProtectiveOrder>> = None;
        match &order.order_type {
            OrderType::Limit => {
                is_fill_triggered = match order.side {
                    OrderSide::Buy => market_price <= order.limit_price?,
                    OrderSide::Sell => market_price >= order.limit_price?
                };
            }
            OrderType::Market => {
                is_fill_triggered = true;
            },
            OrderType::MarketIfTouched | OrderType::StopMarket => {
                is_fill_triggered = match order.side {
                    OrderSide::Buy => market_price <= order.trigger_price?,
                    OrderSide::Sell => market_price >= order.trigger_price?
                };
            }
            OrderType::StopLimit => {
                is_fill_triggered = match order.side {
                    OrderSide::Buy => market_price <= order.trigger_price? && market_price > order.limit_price?,
                    OrderSide::Sell => market_price >= order.trigger_price? && market_price < order.limit_price?
                };
            },
            OrderType::EnterLong(new_brackets) => {
                if account_ledger.is_short(&order.symbol_name).await {
                    account_ledger.exit_position_paper(&order.symbol_name, market_price, last_time_utc).await;
                }
                is_fill_triggered = true;
                brackets = new_brackets.clone();
            }
            OrderType::EnterShort(new_brackets) => {
                if account_ledger.is_long(&order.symbol_name).await {
                    account_ledger.exit_position_paper(&order.symbol_name, market_price, last_time_utc).await;
                }
                is_fill_triggered = true;
                brackets = new_brackets.clone();
            }
            OrderType::ExitLong => {
                if account_ledger.is_long(&order.symbol_name).await {
                    is_fill_triggered = true;
                } else {
                    let reason = "No long position to exit".to_string();
                    reject_order(reason, order.clone(), last_time_utc, &mut events);
                    continue 'order_loop;
                }
            }
            OrderType::ExitShort => {
                if account_ledger.is_short(&order.symbol_name).await {
                    is_fill_triggered = true;
                } else {
                    let reason = "No short position to exit".to_string();
                    reject_order(reason, order.clone(), last_time_utc, &mut events);
                    continue 'order_loop;
                }
            }
            OrderType::UpdateBrackets(broker, account_id, symbol_name, brackets) => {
                is_fill_triggered = false;
                let mut updated = false;
                if let Some(ledger) = ledgers.get(broker) {
                    if let Some(ledger) = ledger.value().get(account_id) {
                        ledger.update_brackets(symbol_name, brackets.clone());
                        updated = true;
                    }
                }
                if !updated {
                    let reason = "No position for update brackets".to_string();
                    reject_order(reason, order.clone(), last_time_utc, &mut events);
                    continue 'order_loop;
                }
            }
        }
        if is_fill_triggered {
                match account_ledger.update_or_create_paper_position(&order.symbol_name.clone(), order.quantity_ordered, market_price, order.side, &last_time_utc, brackets).await {
                    Ok(_) => fill_order(order.clone(), last_time_utc, market_price, &mut events),
                    Err(e) => reject_order(e.to_string(), order.clone(), last_time_utc, &mut events)
                }
        } else {
            accept_order(order.clone(), last_time_utc, &mut events, &mut remaining_orders);
        }
    }

    *order_cache = remaining_orders;
    if events.len() == 0 {
        None
    } else {
        Some(events)
    }
}