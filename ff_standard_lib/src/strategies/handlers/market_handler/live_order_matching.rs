use std::sync::Arc;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use rust_decimal_macros::dec;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver};
use crate::standardized_types::orders::{Order, OrderId, OrderState, OrderUpdateEvent, OrderUpdateType};
use crate::strategies::ledgers::ledger_service::{LedgerService};
use crate::strategies::strategy_events::StrategyEvent;

//todo, this probably isnt needed

pub(crate) fn live_order_handler(
    open_order_cache: Arc<DashMap<OrderId, Order>>, //todo, make these static or lifetimes if possible.. might not be optimal though, look it up!
    closed_order_cache: Arc<DashMap<OrderId, Order>>,
    mut order_event_receiver: Receiver<(OrderUpdateEvent, DateTime<Utc>)>,
    strategy_event_sender: mpsc::Sender<StrategyEvent>,
    ledger_service: Arc<LedgerService>, //it is better to do this, because using a direct fn call we can concurrently update individual ledgers and have a que per ledger. sending a msg here would cause a bottleneck with more ledgers.
    synchronize_positions: bool
) {
    //todo, we need a message que for ledger, where orders and positions are update the ledger 1 at a time per symbol_code, this should fix the possible race conditions of positions updates
    tokio::task::spawn(async move {
        while let Some((ref order_update_event, time_utc)) = order_event_receiver.recv().await {
            match order_update_event {
                #[allow(unused)]
                OrderUpdateEvent::OrderAccepted { account, symbol_name, symbol_code, order_id, tag, time } => {
                    if let Some(mut order) = open_order_cache.get_mut(order_id) {
                        if order.state != OrderState::Created {
                            continue;
                        }
                        order.value_mut().state = OrderState::Accepted;
                        order.symbol_code = symbol_code.clone();
                        match strategy_event_sender.send(StrategyEvent::OrderEvents(order_update_event.clone())).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("{}", e)
                        }
                    }
                }
                OrderUpdateEvent::OrderFilled { account, symbol_name, symbol_code, order_id, price, quantity, tag, time, side } => {
                    #[allow(unused)]
                     if let Some((order_id, mut order)) = open_order_cache.remove(order_id) {
                         if order.state == OrderState::Filled {
                            continue;
                         }
                         order.symbol_code = symbol_code.clone();
                         let quantity = order.quantity_open;
                         order.quantity_filled += order.quantity_open;
                         order.quantity_open = dec!(0.0);
                         order.time_filled_utc = Some(time.clone());
                         order.state = OrderState::Filled;

                         //println!("{}", order_update_event);
                         match synchronize_positions {
                             false => ledger_service.update_or_create_position(&account, symbol_name.clone(), symbol_code.clone(), quantity, side.clone(), time_utc, *price, tag.to_string(), None, None).await,
                             true => {}//ledger_service.process_synchronized_orders(order.clone(), quantity.clone(), time_utc).await
                         };

                         match strategy_event_sender.send(StrategyEvent::OrderEvents(order_update_event.clone())).await {
                             Ok(_) => {}
                             Err(e) => eprintln!("{}", e)
                         }
                    }
                }
                OrderUpdateEvent::OrderPartiallyFilled { account, symbol_name, symbol_code, order_id, price, quantity, tag, time,  side} => {
                   if let Some(mut order) = open_order_cache.get_mut(order_id) {
                       if order.state == OrderState::Filled {
                           continue;
                       }
                       //println!("{}", order_update_event);
                       order.state = OrderState::PartiallyFilled;
                       order.symbol_code = symbol_code.clone();
                       order.quantity_filled += quantity;
                       order.quantity_open -= quantity;
                       order.time_filled_utc = Some(time.clone());
                       match synchronize_positions {
                           false => ledger_service.update_or_create_position(&account, symbol_name.clone(), symbol_code.clone(), quantity.clone(), side.clone(), time_utc, *price, tag.to_string(), None, None).await,
                           true => {}//ledger_service.process_synchronized_orders(order.clone(), quantity.clone(), time_utc).await,
                       };
                       match strategy_event_sender.send(StrategyEvent::OrderEvents(order_update_event.clone())).await {
                           Ok(_) => {}
                           Err(e) => eprintln!("{}", e)
                       }
                   }
                }
                OrderUpdateEvent::OrderCancelled { order_id,symbol_code,.. } => {
                    if let Some((order_id, mut order)) = open_order_cache.remove(order_id) {
                        order.state = OrderState::Cancelled;
                        order.quantity_open = dec!(0);
                        order.symbol_code = symbol_code.clone();
                        closed_order_cache.insert(order_id.clone(), order);
                        match strategy_event_sender.send(StrategyEvent::OrderEvents(order_update_event.clone())).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("{}", e)
                        }
                    }
                }
                OrderUpdateEvent::OrderRejected {symbol_code, order_id,reason, .. } => {
                    if let Some((order_id, mut order)) = open_order_cache.remove(order_id) {
                        order.state = OrderState::Rejected(reason.clone());
                        order.symbol_code = symbol_code.clone();
                        order.quantity_open = dec!(0);
                        closed_order_cache.insert(order_id.clone(), order);
                        match strategy_event_sender.send(StrategyEvent::OrderEvents(order_update_event.clone())).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("{}", e)
                        }
                    }
                }
                OrderUpdateEvent::OrderUpdated { order_id, symbol_code, update_type,.. } => {
                    if let Some(mut order) = open_order_cache.get_mut(order_id) {
                        order.symbol_code = symbol_code.clone();
                        match &update_type {
                            OrderUpdateType::LimitPrice(price) => order.limit_price = Some(price.clone()),
                            OrderUpdateType::TriggerPrice(price) => order.trigger_price = Some(price.clone()),
                            OrderUpdateType::Quantity(quantity) => order.quantity_open = quantity.clone(),
                        }
                        match strategy_event_sender.send(StrategyEvent::OrderEvents(order_update_event.clone())).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("{}", e)
                        }
                    }
                }
                OrderUpdateEvent::OrderUpdateRejected { order_id, .. } => {
                    //todo not sure if we remove here, depends if update id is its own order
                    if let Some((order_id, order)) = open_order_cache.remove(order_id) {
                        closed_order_cache.insert(order_id.clone(), order);
                        match strategy_event_sender.send(StrategyEvent::OrderEvents(order_update_event.clone())).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("{}", e)
                        }
                    }
                }
            }
        }
    });
}