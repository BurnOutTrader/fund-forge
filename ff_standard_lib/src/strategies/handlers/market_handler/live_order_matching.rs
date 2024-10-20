use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver};
use crate::standardized_types::orders::{Order, OrderId, OrderState, OrderUpdateEvent, OrderUpdateType};
use crate::strategies::strategy_events::StrategyEvent;

//todo, this probably isnt needed

pub fn live_order_update(
    open_order_cache: Arc<DashMap<OrderId, Order>>, //todo, make these static or lifetimes if possible.. might not be optimal though, look it up!
    closed_order_cache: Arc<DashMap<OrderId, Order>>,
    mut order_event_receiver: Receiver<OrderUpdateEvent>,
    strategy_event_sender: mpsc::Sender<StrategyEvent>
) {
    tokio::task::spawn(async move {
        while let Some(order_update_event) = order_event_receiver.recv().await {
            match order_update_event {
                OrderUpdateEvent::OrderAccepted { account, symbol_name, symbol_code, order_id, tag, time } => {
                    if let Some(mut order) = open_order_cache.get_mut(&order_id) {
                        order.value_mut().state = OrderState::Accepted;
                        order.symbol_code = Some(symbol_code.clone());
                        let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderAccepted { symbol_name, symbol_code, order_id, account, tag, time });
                        match strategy_event_sender.send(event).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("Live Order Handler: Failed to send event: {}", e)
                        }
                    }
                }
                OrderUpdateEvent::OrderFilled { account: _, symbol_name: _, symbol_code: _, order_id: _, price: _, quantity: _, tag: _, time: _ } => {
                   //todo send direct via LEDGER_SERVICE
                    /* if let Some((order_id, mut order)) = open_order_cache.remove(&order_id) {
                        order.symbol_code = Some(symbol_code.clone());
                        order.state = OrderState::Filled;
                        order.quantity_filled += quantity;
                        order.time_filled_utc = Some(time.clone());
                        let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderFilled { order_id: order_id.clone(), price, account_id: account_id.clone(), symbol_name, brokerage, tag, time: time.clone(), quantity, symbol_code: symbol_code.clone() });
                        add_buffer(Utc::now(), event).await;
                        if let Some(broker_map) = ledger_senders.get(&order.brokerage) {
                            if let Some(account_map) = broker_map.get_mut(&order.account_id) {
                                let symbol_code = match &order.symbol_code {
                                    None => order.symbol_name.clone(),
                                    Some(code) => code.clone()
                                };
                                let ledger_message = LedgerMessage::UpdateOrCreatePosition {symbol_name: order.symbol_name.clone(), symbol_code, order_id: order_id.clone(), quantity: order.quantity_filled, side: order.side.clone(), time: Utc::now(), market_fill_price: order.average_fill_price.unwrap(), tag: order.tag.clone()};
                                match account_map.value().send(ledger_message).await {
                                    Ok(_) => {}
                                    Err(e) => eprintln!("Error Sender Ledger Message in backtest_matching_engine::fill_order(), {}", e)
                                }
                            }
                        }
                        closed_order_cache.insert(order_id.clone(), order);
                    }*/
                }
                OrderUpdateEvent::OrderPartiallyFilled { account: _, symbol_name: _, symbol_code: _, order_id: _, price: _, quantity: _, tag: _, time: _ } => {
               /*     if let Some(mut order) = open_order_cache.get_mut(&order_id) {
                        order.state = OrderState::PartiallyFilled;
                        order.symbol_code = Some(symbol_code.clone());
                        order.quantity_filled += quantity;
                        order.time_filled_utc = Some(time.clone());
                        let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderFilled { order_id: order_id.clone(), price, account_id: account_id.clone(), symbol_name: symbol_name.clone(), brokerage, tag, time: time.clone(), quantity, symbol_code: symbol_code.clone() });
                        add_buffer(Utc::now(), event).await;
                        if let Some(broker_map) = ledger_senders.get(&order.brokerage) {
                            if let Some(account_map) = broker_map.get_mut(&order.account_id) {
                                let symbol_code = match &order.symbol_code {
                                    None => order.symbol_name.clone(),
                                    Some(code) => code.clone()
                                };
                                let ledger_message = LedgerMessage::UpdateOrCreatePosition {symbol_name: order.symbol_name.clone(), symbol_code, order_id: order_id.clone(), quantity: order.quantity_filled, side: order.side.clone(), time: Utc::now(), market_fill_price: order.average_fill_price.unwrap(), tag: order.tag.clone()};
                                match account_map.value().send(ledger_message).await {
                                    Ok(_) => {}
                                    Err(e) => eprintln!("Error Sender Ledger Message in backtest_matching_engine::fill_order(), {}", e)
                                }
                            }
                        }
                    }*/
                }
                OrderUpdateEvent::OrderCancelled { account, symbol_name, symbol_code, order_id, tag, time } => {
                    if let Some((order_id, mut order)) = open_order_cache.remove(&order_id) {
                        order.state = OrderState::Cancelled;
                        order.symbol_code = Some(symbol_code.clone());
                        let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderCancelled { order_id: order_id.clone(), account, symbol_name, tag, time, symbol_code });
                        closed_order_cache.insert(order_id.clone(), order);
                        match strategy_event_sender.send(event).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("Live Order Handler: Failed to send event: {}", e)
                        }
                    }
                }
                OrderUpdateEvent::OrderRejected { account, symbol_name, symbol_code, order_id, reason, tag, time } => {
                    if let Some((order_id, mut order)) = open_order_cache.remove(&order_id) {
                        order.state = OrderState::Rejected(reason.clone());
                        order.symbol_code = Some(symbol_code.clone());
                        let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderRejected { order_id: order_id.clone(), account, symbol_name, reason, tag, time, symbol_code: symbol_code });
                        closed_order_cache.insert(order_id.clone(), order);
                        match strategy_event_sender.send(event).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("Live Order Handler: Failed to send event: {}", e)
                        }
                    }
                }
                OrderUpdateEvent::OrderUpdated { account, symbol_name, symbol_code, order_id, update_type, tag, time } => {
                    if let Some((id, mut order)) = open_order_cache.remove(&order_id) {
                        order.symbol_code = Some(symbol_code.clone());
                        match &update_type {
                            OrderUpdateType::LimitPrice(price) => order.limit_price = Some(price.clone()),
                            OrderUpdateType::TriggerPrice(price) => order.trigger_price = Some(price.clone()),
                            OrderUpdateType::TimeInForce(tif) => order.time_in_force = tif.clone(),
                            OrderUpdateType::Quantity(quantity) => order.quantity_open = quantity.clone(),
                            OrderUpdateType::Tag(tag) => order.tag = tag.clone()
                        }
                        open_order_cache.insert(id, order);
                    }
                    let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderUpdated { order_id, account, tag, time, update_type, symbol_code, symbol_name });
                    match strategy_event_sender.send(event).await {
                        Ok(_) => {}
                        Err(e) => eprintln!("Live Order Handler: Failed to send event: {}", e)
                    }
                }
                OrderUpdateEvent::OrderUpdateRejected { account, order_id, reason, time } => {
                    if let Some((order_id, order)) = open_order_cache.remove(&order_id) {
                        closed_order_cache.insert(order_id.clone(), order);
                    }
                    let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderUpdateRejected {order_id, account, reason, time});
                    match strategy_event_sender.send(event).await {
                        Ok(_) => {}
                        Err(e) => eprintln!("Live Order Handler: Failed to send event: {}", e)
                    }
                }
            }
        }
    });
}