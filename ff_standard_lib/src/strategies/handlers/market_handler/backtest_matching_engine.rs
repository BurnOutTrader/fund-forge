use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{Sender};
use tokio::sync::oneshot;
use crate::helpers::converters::{time_convert_utc_to_local, time_local_from_utc_str};
use crate::standardized_types::enums::{OrderSide, PositionSide};
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::orders::{Order, OrderId, OrderRequest, OrderState, OrderType, OrderUpdateEvent, OrderUpdateType, TimeInForce};
use crate::strategies::client_features::server_connections::add_buffer;
use crate::strategies::handlers::market_handler::price_service::{price_service_request_limit_fill_price_quantity, price_service_request_market_fill_price, price_service_request_market_price, PriceServiceResponse};
use crate::strategies::ledgers::{LedgerMessage, LedgerRequest, LedgerResponse, LEDGER_SERVICE};
use crate::strategies::strategy_events::StrategyEvent;

pub enum BackTestEngineMessage {
    Time(DateTime<Utc>),
    OrderRequest(DateTime<Utc>, OrderRequest)
}
pub async fn backtest_matching_engine(
    open_order_cache: Arc<DashMap<OrderId, Order>>, //todo, make these static or lifetimes if possible.. might not be optimal though, look it up!
    closed_order_cache: Arc<DashMap<OrderId, Order>>,
) -> Sender<BackTestEngineMessage> {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(100);
    tokio::task::spawn(async move {
        while let Some(backtest_message) = receiver.recv().await {
            match backtest_message {
                BackTestEngineMessage::OrderRequest(time, order_request) => {
                    match order_request {
                        OrderRequest::Create { order, .. } => {
                            println!("Order received: {:?}", order);
                            open_order_cache.insert(order.id.clone(), order);
                            simulated_order_matching(time.clone(), &open_order_cache, &closed_order_cache).await;
                        }
                        OrderRequest::Cancel { brokerage, order_id, account_id } => {
                            let existing_order = open_order_cache.remove(&order_id);
                            if let Some((existing_order_id, order)) = existing_order {
                                let cancel_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderCancelled {
                                    brokerage, account_id: order.account_id.clone(), symbol_name: order.symbol_name.clone(), symbol_code: order.symbol_name.clone(),
                                    order_id: existing_order_id, tag: order.tag.clone(), time: time.to_string()
                                });
                                add_buffer(time, cancel_event).await;
                                closed_order_cache.insert(order_id, order);
                            } else {
                                let fail_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderUpdateRejected {
                                    brokerage, account_id, order_id, reason: String::from("No pending order found"), time: time.to_string()
                                });
                                add_buffer(time, fail_event).await;
                            }
                        }
                        OrderRequest::Update { brokerage, order_id, account_id, update } => {
                            if let Some((order_id, mut order)) = open_order_cache.remove(&order_id) {
                                match &update {
                                    OrderUpdateType::LimitPrice(price) => {
                                        if let Some(ref mut limit_price) = order.limit_price {
                                            *limit_price = price.clone();
                                        }
                                    }
                                    OrderUpdateType::TriggerPrice(price) => {
                                        if let Some(ref mut trigger_price) = order.trigger_price {
                                            *trigger_price = price.clone();
                                        }
                                    }
                                    OrderUpdateType::TimeInForce(tif) => {
                                        order.time_in_force = tif.clone();
                                    }
                                    OrderUpdateType::Quantity(quantity) => {
                                        order.quantity_open = quantity.clone();
                                    }
                                    OrderUpdateType::Tag(tag) => {
                                        order.tag = tag.clone();
                                    }
                                }
                                let update_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderUpdated {
                                    brokerage, account_id: order.account_id.clone(), symbol_name: order.symbol_name.clone(), symbol_code: order.symbol_name.clone(),
                                    order_id: order.id.clone(), update_type: update, tag: order.tag.clone(), time: time.to_string()
                                });
                                open_order_cache.insert(order_id, order);
                                add_buffer(time, update_event).await;
                            } else {
                                let fail_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderUpdateRejected {
                                    brokerage, account_id, order_id, reason: String::from("No pending order found"), time: time.to_string()
                                });
                                add_buffer(time, fail_event).await;
                            }
                        }
                        OrderRequest::CancelAll { brokerage, account_id, symbol_name } => {
                            let mut remove = vec![];
                            for order in open_order_cache.iter() {
                                if order.brokerage == brokerage && order.account_id == account_id && order.symbol_name == symbol_name {
                                    remove.push(order.id.clone());
                                }
                            }
                            for order_id in remove {
                                let order = open_order_cache.remove(&order_id);
                                if let Some((order_id, order)) = order {
                                    let cancel_event = StrategyEvent::OrderEvents(
                                        OrderUpdateEvent::OrderCancelled {
                                            brokerage: order.brokerage.clone(), account_id: order.account_id.clone(), symbol_name: order.symbol_name.clone(),
                                            symbol_code: order.symbol_name.clone(), order_id: order.id.clone(), tag: order.tag.clone(), time: time.to_string()
                                        });
                                    add_buffer(time.clone(), cancel_event).await;
                                    closed_order_cache.insert(order_id, order);
                                }
                            }
                        }
                        OrderRequest::FlattenAllFor { brokerage: _, account_id: _ } => {
                            //Todo
                        /*    if let Some(broker_map) = ledger_senders.get(&brokerage) {
                                if let Some(account_map) = broker_map.get(&account_id) {
                                    match account_map.send(LedgerMessage::FlattenAccount(time)).await {
                                        Ok(_) => {}
                                        Err(e) => panic!("Failed to send ledger flatten account message: {}", e)
                                    }
                                }
                            }*/
                        }
                    }
                }
                BackTestEngineMessage::Time(time) => {
                    simulated_order_matching(time.clone(), &open_order_cache, &closed_order_cache).await;
                }
            }
        }
    });
    sender
}

pub async fn simulated_order_matching (
    time: DateTime<Utc>,
    open_order_cache: &Arc<DashMap<OrderId, Order>>,
    closed_order_cache: &Arc<DashMap<OrderId, Order>>,
) {
    if open_order_cache.len() == 0 {
        return;
    }
    let mut rejected = Vec::new();
    let mut accepted = Vec::new();
    let mut filled = Vec::new();
    let mut partially_filled = Vec::new();
    for order in open_order_cache.iter() {
        //println!("Order matching: {:?}", order.value());
        match &order.time_in_force {
            TimeInForce::GTC => {},
            TimeInForce::Day(time_zone_string) => {
                let tz: Tz = time_zone_string.parse().unwrap();
                let order_time = time_convert_utc_to_local(&tz, order.time_created_utc());
                let local_time = time_convert_utc_to_local(&tz, time);
                if local_time.date_naive() != order_time.date_naive() {
                    let reason = "Time In Force Expired: TimeInForce::Day".to_string();
                    rejected.push((order.id.clone(), reason));
                }
            }
            TimeInForce::IOC=> {
                if time > order.time_created_utc() {
                /*    let reason = "Time In Force Expired: TimeInForce::IOC".to_string();
                    rejected.push((order.id.clone(), reason));*/
                }
            }
            TimeInForce::FOK => {
                if time > order.time_created_utc() {
                /*    let reason = "Time In Force Expired: TimeInForce::FOK".to_string();
                    rejected.push((order.id.clone(), reason));*/
                }
            }
            TimeInForce::Time(cancel_time, time_zone_string) => {
                let tz: Tz = time_zone_string.parse().unwrap();
                let local_time = time_convert_utc_to_local(&tz, time);
                let cancel_time = time_local_from_utc_str(&tz, cancel_time);
                if local_time >= cancel_time {
                    let reason = "Time In Force Expired: TimeInForce::Time".to_string();
                    rejected.push((order.id.clone(), reason));
                }
            }
        }
        //3. respond with an order event
        match &order.order_type {
            OrderType::Limit => {
                let market_price = match price_service_request_market_price(order.side, order.symbol_name.clone()).await {
                    Ok(price) => match price.price() {
                        None => continue,
                        Some(price) => price
                    }
                    Err(_) => continue
                };

                let limit_price = order.limit_price.unwrap();
                match order.side {
                    // Buy Stop Limit logic
                    OrderSide::Buy => {
                        if limit_price > market_price {// todo double check this logic
                            rejected.push((
                                String::from("Invalid Price: Buy Limit Price Must Be At or Below Market Price"),
                                order.id.clone(),
                            ));
                            continue;
                        }
                        // No need to compare market price vs limit price before triggering, only after the stop is triggered
                    }

                    // Sell Stop Limit logic
                    OrderSide::Sell => {
                        if limit_price < market_price {
                            rejected.push((
                                String::from("Invalid Price: Sell Limit Price Must Be At or Above Market Price"),
                                order.id.clone(),
                            ));
                            continue;
                        }
                        // No need to compare market price vs limit price before triggering, only after the stop is triggered
                    }
                }
                let is_fill_triggered = match order.side {
                    OrderSide::Buy => market_price <= order.limit_price.unwrap(),
                    OrderSide::Sell => market_price >= order.limit_price.unwrap()
                };
                let (market_price, volume_filled) = match price_service_request_limit_fill_price_quantity(order.side, order.symbol_name.clone(), order.quantity_open, order.quantity_open).await {
                    Ok(price_volume) => {
                        match price_volume {
                            PriceServiceResponse::LimitFillPriceEstimate { fill_price, fill_volume } => {
                                if let (Some(fill_price), Some(fill_volume)) = (fill_price, fill_volume) {
                                    (fill_price, fill_volume)
                                } else {
                                    continue
                                }
                            }
                            _ => panic!("Incorrect response received from price service")
                        }
                    },
                    Err(_) => continue
                };
                if is_fill_triggered {
                    match volume_filled == order.quantity_open {
                        true => filled.push((order.id.clone(),  market_price)),
                        false => partially_filled.push((order.id.clone(),  market_price, volume_filled))
                    }
                } else if order.state == OrderState::Created {
                    accepted.push((order.id.clone(), time))
                }
            }
            OrderType::Market => {
                let market_price = match price_service_request_market_fill_price(order.side, order.symbol_name.clone(), order.quantity_filled).await {
                    Ok(price) => {
                        match price.price() {
                            None =>  continue,
                            Some(price) => price
                        }
                    },
                    Err(_) => continue
                };
                filled.push((order.id.clone(), market_price));
            },
            // Handle OrderType::StopMarket separately
            OrderType::StopMarket => {
                let market_price = match price_service_request_market_price(order.side, order.symbol_name.clone()).await {
                    Ok(price) => match price.price() {
                        None => continue,
                        Some(price) => price
                    }
                    Err(_) => continue
                };
                let trigger_price = order.trigger_price.unwrap();

                match order.side {
                    OrderSide::Buy => {// todo double check this logic
                        // Buy Stop Market: trigger price must be ABOVE market price to avoid instant fill
                        if trigger_price <= market_price {
                            rejected.push((
                                String::from("Invalid Price: Buy Stop Price Must Be Above Market Price"),
                                order.id.clone(),
                            ));
                            continue;
                        }
                    }
                    OrderSide::Sell => {
                        // Sell Stop Market: trigger price must be BELOW market price to avoid instant fill
                        if trigger_price >= market_price {
                            rejected.push((
                                String::from("Invalid Price: Sell Stop Price Must Be Below Market Price"),
                                order.id.clone(),
                            ));
                            continue;
                        }
                    }
                }

                let is_fill_triggered = match order.side {
                    OrderSide::Buy => market_price >= trigger_price,
                    OrderSide::Sell => market_price <= trigger_price,
                };

                let market_fill_price = match price_service_request_market_fill_price(order.side, order.symbol_name.clone(), order.quantity_filled).await {
                    Ok(price) => match price.price() {
                        None => continue,
                        Some(price) => price
                    }
                    Err(_) => continue,
                };

                if is_fill_triggered {
                    filled.push((order.id.clone(), market_fill_price));
                } else if order.state == OrderState::Created {
                    accepted.push((order.id.clone(), time));
                }
            }

            // Handle OrderType::MarketIfTouched separately
            OrderType::MarketIfTouched => {
                let market_price = match price_service_request_market_price(order.side, order.symbol_name.clone()).await {
                    Ok(price) => match price.price() {
                        None => continue,
                        Some(price) => price
                    }
                    Err(_) => continue
                };
                let trigger_price = order.trigger_price.unwrap();

                match order.side {
                    OrderSide::Buy => {// todo double check this logic
                        // Buy MIT: trigger price must be BELOW market price to wait for favorable dip
                        if trigger_price >= market_price {
                            rejected.push((
                                String::from("Invalid Price: Buy MIT Price Must Be Below Market Price"),
                                order.id.clone(),
                            ));
                            continue;
                        }
                    }
                    OrderSide::Sell => {
                        // Sell MIT: trigger price must be ABOVE market price to wait for favorable rise
                        if trigger_price <= market_price {
                            rejected.push((
                                String::from("Invalid Price: Sell MIT Price Must Be Above Market Price"),
                                order.id.clone(),
                            ));
                            continue;
                        }
                    }
                }

                let is_fill_triggered = match order.side {
                    OrderSide::Buy => market_price <= trigger_price,
                    OrderSide::Sell => market_price >= trigger_price,
                };

                let market_fill_price = match price_service_request_market_fill_price(order.side, order.symbol_name.clone(), order.quantity_filled).await {
                    Ok(price) => match price.price() {
                        None => continue,
                        Some(price) => price
                    }
                    Err(_) => continue,
                };

                if is_fill_triggered {
                    filled.push((order.id.clone(), market_fill_price));
                } else if order.state == OrderState::Created {
                    accepted.push((order.id.clone(), time));
                }
            }
            OrderType::StopLimit => {
                let market_price = match price_service_request_market_price(order.side, order.symbol_name.clone()).await {
                    Ok(price) => match price.price() {
                        None => continue,
                        Some(price) => price
                    }
                    Err(_) => continue
                };
                let trigger_price = order.trigger_price.unwrap();
                let limit_price = order.limit_price.unwrap();

                match order.side { // todo double check this logic
                    // Buy Stop Limit logic
                    OrderSide::Buy => {
                        if market_price >= trigger_price {
                            rejected.push((
                                String::from("Invalid Price: Buy Stop Price Must Be Above Market Price"),
                                order.id.clone(),
                            ));
                            continue;
                        } else if limit_price < trigger_price {
                            rejected.push((
                                String::from("Invalid Price: Buy Limit Price Must Be At or Above Trigger Price"),
                                order.id.clone(),
                            ));
                            continue;
                        } else if limit_price > market_price {
                            rejected.push((
                                String::from("Invalid Price: Buy Limit Price Must Be At or Below Market Price"),
                                order.id.clone(),
                            ));
                            continue;
                        }
                    }

                    // Sell Stop Limit logic todo double check this logic
                    OrderSide::Sell => {
                        // Would result in immediate trigger
                        if market_price <= trigger_price {
                            rejected.push((
                                String::from("Invalid Price: Sell Stop Price Must Be Below Market Price"),
                                order.id.clone(),
                            ));
                            continue;
                            // Would not make sense as limiting positive slippage
                        } else if limit_price > trigger_price {
                            rejected.push((
                                String::from("Invalid Price: Sell Limit Price Must Be At or Below Trigger Price"),
                                order.id.clone(),
                            ));
                            continue;
                            // would immediatly fill as a market order
                        } else if limit_price < market_price {
                            rejected.push((
                                String::from("Invalid Price: Sell Limit Price Must Be At or Above Market Price"),
                                order.id.clone(),
                            ));
                            continue;
                        }
                    }
                }
                let is_fill_triggered = match order.side {
                    OrderSide::Buy => market_price <= order.trigger_price.unwrap() && market_price > order.limit_price.unwrap(),
                    OrderSide::Sell => market_price >= order.trigger_price.unwrap() && market_price < order.limit_price.unwrap()
                };
                let (market_price, volume_filled) = match price_service_request_limit_fill_price_quantity(order.side, order.symbol_name.clone(), order.quantity_open, order.quantity_open).await {
                    Ok(price_volume) => {
                        match price_volume {
                            PriceServiceResponse::LimitFillPriceEstimate { fill_price, fill_volume } => {
                                if let (Some(fill_price), Some(fill_volume)) = (fill_price, fill_volume) {
                                    (fill_price, fill_volume)
                                } else {
                                    continue
                                }
                            }
                            _ => panic!("Incorrect response received from price service")
                        }
                    },
                    Err(_) => continue
                };
                if is_fill_triggered {
                    match volume_filled == order.quantity_open {
                        true => filled.push((order.id.clone(),  market_price)),
                        false => partially_filled.push((order.id.clone(),  market_price, volume_filled))
                    }
                } else if order.state == OrderState::Created {
                    accepted.push((order.id.clone(), time))
                }
            },
            OrderType::EnterLong => {
                let market_fill_price = match price_service_request_market_fill_price(order.side, order.symbol_name.clone(), order.quantity_filled).await {
                    Ok(price) => match price.price() {
                        None => continue,
                        Some(price) => price
                    }
                    Err(_) => continue
                };
                let request = LedgerRequest::PaperExitPosition {
                    symbol_name: order.symbol_name.clone(),
                    side: PositionSide::Short,
                    symbol_code: order.symbol_code.clone(),
                    order_id: order.id.clone(),
                    time,
                    tag: "Force Exit: By Enter Long".to_string(),
                };
                let receiver: oneshot::Receiver<LedgerResponse> = LEDGER_SERVICE.request_callback(order.brokerage, &order.account_id, request).await;
                match receiver.await {
                    Ok(response) => {
                        match response {
                            LedgerResponse::PaperExitPosition { had_position } => {
                                match had_position {
                                    true => {}
                                    false => {}
                                }
                            }
                            _ => eprintln!("Incorrect response at callback: {:?}", response)
                        }
                    },
                    Err(_) => {}
                }
                filled.push((order.id.clone(), market_fill_price))
            }
            OrderType::EnterShort => {
                let market_fill_price = match price_service_request_market_fill_price(order.side, order.symbol_name.clone(), order.quantity_filled).await {
                    Ok(price) => match price.price() {
                        None => continue,
                        Some(price) => price
                    }
                    Err(_) => continue,
                };
                let request = LedgerRequest::PaperExitPosition {
                    symbol_name: order.symbol_name.clone(),
                    side: PositionSide::Long,
                    symbol_code: order.symbol_code.clone(),
                    order_id: order.id.clone(),
                    time,
                    tag: "Force Exit: By Enter Short".to_string(),
                };
                let receiver: oneshot::Receiver<LedgerResponse> = LEDGER_SERVICE.request_callback(order.brokerage, &order.account_id, request).await;
                // the result makes no difference here
                match receiver.await {
                    Ok(_response) => {},
                    Err(_) => {}
                }

                filled.push((order.id.clone(), market_fill_price));
            }
            OrderType::ExitLong => {
                let request = LedgerRequest::PaperExitPosition {
                    symbol_name: order.symbol_name.clone(),
                    side: PositionSide::Long,
                    symbol_code: order.symbol_code.clone(),
                    order_id: order.id.clone(),
                    time,
                    tag: order.tag.clone(),
                };
                let receiver: oneshot::Receiver<LedgerResponse> = LEDGER_SERVICE.request_callback(order.brokerage, &order.account_id, request).await;
                match receiver.await {
                    Ok(response) => {
                        match response {
                            LedgerResponse::PaperExitPosition { had_position } => {
                                match had_position {
                                    true => {
                                        let market_fill_price = match price_service_request_market_fill_price(order.side, order.symbol_name.clone(), order.quantity_filled).await {
                                            Ok(price) => match price.price() {
                                                None => continue,
                                                Some(price) => price
                                            }
                                            Err(_) => continue
                                        };
                                        filled.push((order.id.clone(), market_fill_price));
                                    }
                                    false => {
                                        let reason = "No Long Position To Exit".to_string();
                                        rejected.push((order.id.clone(), reason));
                                    }
                                }
                            }
                            _ => eprintln!("Incorrect response at callback: {:?}", response)
                        }
                    },
                    Err(e) => eprintln!("Error on callback: {:?}", e)
                }
            }
            OrderType::ExitShort => {
                let request = LedgerRequest::PaperExitPosition {
                    symbol_name: order.symbol_name.clone(),
                    side: PositionSide::Short,
                    symbol_code: order.symbol_code.clone(),
                    order_id: order.id.clone(),
                    time,
                    tag: order.tag.clone(),
                };
                let receiver: oneshot::Receiver<LedgerResponse> = LEDGER_SERVICE.request_callback(order.brokerage, &order.account_id, request).await;
                match receiver.await {
                    Ok(response) => {
                        match response {
                            LedgerResponse::PaperExitPosition { had_position } => {
                                match had_position {
                                    true => {
                                        let market_fill_price = match price_service_request_market_fill_price(order.side, order.symbol_name.clone(), order.quantity_filled).await {
                                            Ok(price) => match price.price() {
                                                None => continue,
                                                Some(price) => price
                                            }
                                            Err(_) => continue
                                        };
                                        filled.push((order.id.clone(), market_fill_price));
                                    }
                                    false => {
                                        let reason = "No Short Position To Exit".to_string();
                                        rejected.push((order.id.clone(), reason));
                                    }
                                }
                            }
                            _ => eprintln!("Incorrect response at callback: {:?}", response)
                        }
                    },
                    Err(e) => eprintln!("Error on callback: {:?}", e)
                }
            }
        }
    }

    for (order_id, reason) in rejected {
        reject_order(reason, &order_id, time, &open_order_cache, closed_order_cache).await
    }
    for (order_id , time)in accepted {
        accept_order(&order_id, time, &open_order_cache).await;
    }
    for (order_id, price) in filled {
        fill_order(&order_id, time, price, &open_order_cache, &closed_order_cache).await;
    }
    for (order_id, price, volume) in partially_filled {
        partially_fill_order(&order_id, time, price, volume, &open_order_cache).await;
    }
}

async fn fill_order(
    order_id: &OrderId,
    time: DateTime<Utc>,
    market_price: Price,
    open_order_cache: &Arc<DashMap<OrderId, Order>>,
    closed_order_cache: &Arc<DashMap<OrderId, Order>>,
) {
    if let Some((_, order)) = open_order_cache.remove(order_id) {
        open_order_cache.insert(order.id.clone(), order.clone());
        let symbol_code = match &order.symbol_code {
            None => order.symbol_name.clone(),
            Some(code) => code.clone()
        };
        let ledger_message = LedgerMessage::UpdateOrCreatePosition {symbol_name: order.symbol_name.clone(), symbol_code, order_id: order_id.clone(), quantity: order.quantity_open.clone(), side: order.side.clone(), time, market_fill_price: market_price,tag: order.tag.clone()};
        LEDGER_SERVICE.send_message(order.brokerage, &order.account_id, ledger_message).await;
        closed_order_cache.insert(order.id.clone(), order);
    }
}

async fn partially_fill_order(
    order_id: &OrderId,
    time: DateTime<Utc>,
    fill_price: Price,
    fill_volume: Volume,
    open_order_cache: &Arc<DashMap<OrderId, Order>>,
) {
    if let Some(order) = open_order_cache.get_mut(order_id) {
        let symbol_code = match &order.symbol_code {
            None => order.symbol_name.clone(),
            Some(code) => code.clone()
        };
        let ledger_message = LedgerMessage::UpdateOrCreatePosition {symbol_name: order.symbol_name.clone(), symbol_code, order_id: order_id.clone(), quantity: fill_volume, side: order.side.clone(), time, market_fill_price: fill_price,tag: order.tag.clone()};
        LEDGER_SERVICE.send_message(order.brokerage, &order.account_id, ledger_message).await;
    }
}

async fn reject_order(
    reason: String,
    order_id: &OrderId,
    time: DateTime<Utc>,
    open_order_cache: &Arc<DashMap<OrderId, Order>>,
    closed_order_cache: &Arc<DashMap<OrderId, Order>>,
) {
    if let Some((_, mut order)) = open_order_cache.remove(order_id) {
        order.state = OrderState::Rejected(reason.clone());
        order.time_created_utc = time.to_string();

        add_buffer(
            time,
            StrategyEvent::OrderEvents(OrderUpdateEvent::OrderRejected {
                order_id: order.id.clone(),
                brokerage: order.brokerage,
                account_id: order.account_id.clone(),
                symbol_name: order.symbol_name.clone(),
                reason,
                tag: order.tag.clone(),
                time: time.to_string(),
                symbol_code: order.symbol_name.clone(),
            }),
        ).await;
        closed_order_cache.insert(order.id.clone(), order.clone());
    }
}

async fn accept_order(
    order_id: &OrderId,
    time: DateTime<Utc>,
    open_order_cache: &Arc<DashMap<OrderId, Order>>,
) {
    if let Some(mut order) = open_order_cache.get_mut(order_id) {
        order.state = OrderState::Accepted;
        order.time_created_utc = time.to_string();

        add_buffer(
            time,
            StrategyEvent::OrderEvents(OrderUpdateEvent::OrderAccepted {
                order_id: order.id.clone(),
                brokerage: order.brokerage.clone(),
                account_id: order.account_id.clone(),
                symbol_name: order.symbol_name.clone(),
                tag: order.tag.clone(),
                time: time.to_string(),
                symbol_code: order.symbol_name.clone(),
            }),
        ).await;
    }
}
