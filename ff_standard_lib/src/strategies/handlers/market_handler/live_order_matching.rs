use chrono::Utc;
use crate::standardized_types::orders::{OrderState, OrderUpdateEvent, OrderUpdateType};
use crate::strategies::client_features::server_connections::add_buffer;
use crate::strategies::handlers::market_handler::market_handlers::{LIVE_CLOSED_ORDER_CACHE, LIVE_LEDGERS, LIVE_ORDER_CACHE};
use crate::strategies::strategy_events::StrategyEvent;

pub fn live_order_update(order_update_event: OrderUpdateEvent) {
    tokio::task::spawn(async move {
        match order_update_event {
            OrderUpdateEvent::OrderAccepted { brokerage, account_id, symbol_name, symbol_code, order_id, tag, time } => {
                if let Some(mut order) = LIVE_ORDER_CACHE.get_mut(&order_id) {
                    order.value_mut().state = OrderState::Accepted;
                    order.symbol_code = Some(symbol_code.clone());
                    let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderAccepted { symbol_name, symbol_code, order_id, account_id: account_id.clone(), brokerage, tag, time });
                    add_buffer(Utc::now(), event).await;
                }
            }
            OrderUpdateEvent::OrderFilled { brokerage, account_id, symbol_name, symbol_code, order_id, price, quantity, tag, time } => {
                if let Some((order_id, mut order)) = LIVE_ORDER_CACHE.remove(&order_id) {
                    order.symbol_code = Some(symbol_code.clone());
                    order.state = OrderState::Filled;
                    order.quantity_filled += quantity;
                    order.time_filled_utc = Some(time.clone());
                    let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderFilled { order_id: order_id.clone(), price, account_id: account_id.clone(), symbol_name, brokerage, tag, time, quantity, symbol_code: symbol_code.clone() });
                    add_buffer(Utc::now(), event).await;
                    if let Some(broker_map) = LIVE_LEDGERS.get(&brokerage) {
                        if let Some(mut account_ledger) = broker_map.get_mut(&account_id) {
                            match account_ledger.update_or_create_position(&order.symbol_name, &symbol_code, order_id.clone(), quantity, order.side, Utc::now(), price, order.tag.clone()).await {
                                Ok(events) => {
                                    for event in events {
                                        let strategy_event = StrategyEvent::PositionEvents(event);
                                        add_buffer(Utc::now(), strategy_event).await;
                                    }
                                }
                                Err(_) => {}  //this error is only related to backtesting
                            }
                        }
                    }
                    LIVE_CLOSED_ORDER_CACHE.insert(order_id.clone(), order);
                }
            }
            OrderUpdateEvent::OrderPartiallyFilled { brokerage, account_id, symbol_name, symbol_code, order_id, price, quantity, tag, time } => {
                if let Some(mut order) = LIVE_ORDER_CACHE.get_mut(&order_id) {
                    order.state = OrderState::PartiallyFilled;
                    order.symbol_code = Some(symbol_code.clone());
                    order.quantity_filled += quantity;
                    order.time_filled_utc = Some(time.clone());
                    let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderFilled { order_id: order_id.clone(), price, account_id: account_id.clone(), symbol_name: symbol_name.clone(), brokerage, tag, time, quantity, symbol_code: symbol_code.clone() });
                    add_buffer(Utc::now(), event).await;
                    if let Some(broker_map) = LIVE_LEDGERS.get(&brokerage) {
                        if let Some(mut account_ledger) = broker_map.get_mut(&account_id) {
                            match account_ledger.update_or_create_position(&symbol_name, &symbol_code, order_id.clone(), quantity, order.side, Utc::now(), price, order.tag.clone()).await {
                                Ok(events) => {
                                    for event in events {
                                        let strategy_event = StrategyEvent::PositionEvents(event);
                                        add_buffer(Utc::now(), strategy_event).await;
                                    }
                                }
                                _ => {} //this error is only related to backtesting
                            }
                        }
                    }
                }
            }
            OrderUpdateEvent::OrderCancelled { brokerage, account_id, symbol_name, symbol_code, order_id, tag, time } => {
                if let Some((order_id, mut order)) = LIVE_ORDER_CACHE.remove(&order_id) {
                    order.state = OrderState::Cancelled;
                    order.symbol_code = Some(symbol_code.clone());
                    let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderCancelled { order_id: order_id.clone(), account_id: account_id.clone(), symbol_name, brokerage, tag, time, symbol_code });
                    add_buffer(Utc::now(), event).await;
                    LIVE_CLOSED_ORDER_CACHE.insert(order_id.clone(), order);
                }
            }
            OrderUpdateEvent::OrderRejected { brokerage, account_id, symbol_name, symbol_code, order_id, reason, tag, time } => {
                if let Some((order_id, mut order)) = LIVE_ORDER_CACHE.remove(&order_id) {
                    order.state = OrderState::Rejected(reason.clone());
                    order.symbol_code = Some(symbol_code.clone());
                    let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderRejected { order_id: order_id.clone(), account_id, symbol_name, brokerage, reason, tag, time, symbol_code: symbol_code });
                    add_buffer(Utc::now(), event).await;
                    LIVE_CLOSED_ORDER_CACHE.insert(order_id.clone(), order);
                }
            }
            OrderUpdateEvent::OrderUpdated { brokerage, account_id, symbol_name, symbol_code, order_id, update_type, tag, time } => {
                if let Some((_id, mut order)) = LIVE_ORDER_CACHE.remove(&order_id) {
                    order.symbol_code = Some(symbol_code.clone());
                    match &update_type {
                        OrderUpdateType::LimitPrice(price) => order.limit_price = Some(price.clone()),
                        OrderUpdateType::TriggerPrice(price) => order.trigger_price = Some(price.clone()),
                        OrderUpdateType::TimeInForce(tif) => order.time_in_force = tif.clone(),
                        OrderUpdateType::Quantity(quantity) => order.quantity_open = quantity.clone(),
                        OrderUpdateType::Tag(tag) => order.tag = tag.clone()
                    }
                }
                let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderUpdated { order_id, account_id, symbol_name, brokerage, tag, time, update_type, symbol_code });
                add_buffer(Utc::now(), event).await;
            }
            OrderUpdateEvent::OrderUpdateRejected { brokerage, account_id, order_id, reason, time } => {
                if let Some((order_id, order)) = LIVE_ORDER_CACHE.remove(&order_id) {
                    LIVE_CLOSED_ORDER_CACHE.insert(order_id.clone(), order);
                }
                let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderUpdateRejected {order_id, account_id, brokerage, reason, time});
                add_buffer(Utc::now(), event).await;
            }
        }
    });
}