use chrono::Utc;
use crate::standardized_types::orders::{OrderState, OrderUpdateEvent};
use crate::strategies::client_features::server_connections::add_buffer;
use crate::strategies::handlers::market_handler::market_handlers::{LIVE_CLOSED_ORDER_CACHE, LIVE_LEDGERS, LIVE_ORDER_CACHE};
use crate::strategies::strategy_events::StrategyEvent;

pub async fn live_order_update(order_upate_event: OrderUpdateEvent) {
    match order_upate_event {
        OrderUpdateEvent::OrderAccepted { brokerage, account_id, order_id ,tag, time} => {
            if let Some(mut order) = LIVE_ORDER_CACHE.get_mut(&order_id) {
                order.value_mut().state = OrderState::Accepted;
                let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderAccepted {order_id, account_id: account_id.clone(), brokerage, tag, time});
                add_buffer(Utc::now(), event).await;
            }
            if let Some(broker_map) = LIVE_LEDGERS.get(&brokerage) {
                if let Some(_account_ledger) = broker_map.get_mut(&account_id) {
                    todo!()
                }
            }
        }
        OrderUpdateEvent::OrderFilled { brokerage, account_id, order_id ,tag,time} => {
            if let Some((order_id,mut order)) = LIVE_ORDER_CACHE.remove(&order_id) {
                order.state = OrderState::Filled;
                let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderFilled {order_id: order_id.clone(), account_id: account_id.clone(), brokerage, tag, time});
                add_buffer(Utc::now(), event).await;
                if let Some(broker_map) = LIVE_LEDGERS.get(&brokerage) {
                    if let Some(_account_ledger) = broker_map.get_mut(&account_id) {
                        todo!()
                    }
                }
                LIVE_CLOSED_ORDER_CACHE.insert(order_id.clone(), order);
            }
        }
        OrderUpdateEvent::OrderPartiallyFilled { brokerage, account_id, order_id,tag, time } => {
            if let Some(mut order) = LIVE_ORDER_CACHE.get_mut(&order_id) {
                order.value_mut().state = OrderState::PartiallyFilled;
                let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderFilled {order_id, account_id: account_id.clone(), brokerage, tag, time});
                add_buffer(Utc::now(), event).await;
                if let Some(broker_map) = LIVE_LEDGERS.get(&brokerage) {
                    if let Some(_account_ledger) = broker_map.get_mut(&account_id) {
                        todo!()
                    }
                }
            }
        }
        OrderUpdateEvent::OrderCancelled { brokerage, account_id, order_id,tag, time} => {
            if let Some((order_id, mut order)) = LIVE_ORDER_CACHE.remove(&order_id) {
                order.state = OrderState::Cancelled;
                let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderCancelled {order_id: order_id.clone(), account_id: account_id.clone(), brokerage, tag, time});
                add_buffer(Utc::now(), event).await;
                LIVE_CLOSED_ORDER_CACHE.insert(order_id.clone(), order);
            }
        }
        OrderUpdateEvent::OrderRejected { brokerage, account_id, order_id, reason ,tag, time} => {
            if let Some((order_id, mut order)) = LIVE_ORDER_CACHE.remove(&order_id) {
                order.state = OrderState::Rejected(reason.clone());
                let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderRejected {order_id: order_id.clone(), account_id, brokerage, reason , tag, time});
                add_buffer(Utc::now(), event).await;
                LIVE_CLOSED_ORDER_CACHE.insert(order_id.clone(), order);
            }
        }
        OrderUpdateEvent::OrderUpdated { brokerage, account_id, order_id, order,tag,time} => {
            LIVE_ORDER_CACHE.insert(order_id.clone(), order.clone());
            let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderUpdated {order_id, account_id, brokerage, order, tag, time});
            add_buffer(Utc::now(), event).await;
        }
        OrderUpdateEvent::OrderUpdateRejected { brokerage, account_id, order_id, reason,time } => {
            if let Some((order_id, order)) = LIVE_ORDER_CACHE.remove(&order_id) {
                LIVE_CLOSED_ORDER_CACHE.insert(order_id.clone(), order);
            }
            let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderUpdateRejected {order_id, account_id, brokerage, reason, time});
            add_buffer(Utc::now(), event).await;
        }
    }
}