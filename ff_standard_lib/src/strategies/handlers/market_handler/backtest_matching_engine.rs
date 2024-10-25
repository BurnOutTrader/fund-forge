use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use dashmap::DashMap;
use std::sync::Arc;
use rust_decimal_macros::dec;
use tokio::sync::mpsc::{Sender};
use tokio::sync::Notify;
use crate::helpers::converters::{time_convert_utc_to_local};
use crate::standardized_types::enums::{OrderSide};
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::orders::{Order, OrderId, OrderRequest, OrderState, OrderType, OrderUpdateEvent, OrderUpdateType, TimeInForce};
use crate::strategies::handlers::market_handler::price_service::{price_service_request_limit_fill_price_quantity, price_service_request_market_fill_price, price_service_request_market_price, PriceServiceResponse};
use crate::strategies::ledgers::{LEDGER_SERVICE};
use crate::strategies::strategy_events::StrategyEvent;

pub enum BackTestEngineMessage {
    Time(DateTime<Utc>),
    OrderRequest(DateTime<Utc>, OrderRequest)
}

pub async fn backtest_matching_engine(
    open_order_cache: Arc<DashMap<OrderId, Order>>, //todo, make these static or lifetimes if possible.. might not be optimal though, look it up!
    closed_order_cache: Arc<DashMap<OrderId, Order>>,
    strategy_event_sender: Sender<StrategyEvent>,
    notify: Arc<Notify>,
) -> Sender<BackTestEngineMessage> {
    notify.notify_waiters();
    let (sender, mut receiver) = tokio::sync::mpsc::channel(100);
    tokio::task::spawn(async move {
       // notify.notify_one();
        while let Some(backtest_message) = receiver.recv().await {
            match backtest_message {
                BackTestEngineMessage::OrderRequest(time, order_request) => {
                    //println!("{:?}", order_request);
                    match order_request {
                        OrderRequest::Create {  .. } => {
                            simulated_order_matching(time.clone(), &open_order_cache, &closed_order_cache, strategy_event_sender.clone()).await;
                        }
                        OrderRequest::Cancel { account,order_id } => {
                            let existing_order = open_order_cache.remove(&order_id);
                            if let Some((existing_order_id, order)) = existing_order {
                                let cancel_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderCancelled {
                                    account,
                                    symbol_name: order.symbol_name.clone(),
                                    symbol_code: order.symbol_name.clone(),
                                    order_id: existing_order_id,
                                    tag: order.tag.clone(), 
                                    time: time.to_string(),
                                    reason: "User Request".to_string(),
                                });
                                match strategy_event_sender.send(cancel_event).await {
                                    Ok(_) => {}
                                    Err(e) => eprintln!("Timed Event Handler: Failed to send event: {}", e)
                                }
                                closed_order_cache.insert(order_id, order);
                            } else {
                                let fail_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderUpdateRejected {
                                    account, order_id, reason: String::from("No pending order found"), time: time.to_string()
                                });
                                match strategy_event_sender.send(fail_event).await {
                                    Ok(_) => {}
                                    Err(e) => eprintln!("Timed Event Handler: Failed to send event: {}", e)
                                }
                            }
                        }
                        OrderRequest::Update { account, order_id, update } => {
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
                                    account, symbol_name: order.symbol_name.clone(), symbol_code: order.symbol_name.clone(),
                                    order_id: order.id.clone(), update_type: update, text: "User Request".to_string(), tag: order.tag.clone(), time: time.to_string()
                                });
                                open_order_cache.insert(order_id, order);
                                match strategy_event_sender.send(update_event).await {
                                    Ok(_) => {}
                                    Err(e) => eprintln!("Timed Event Handler: Failed to send event: {}", e)
                                }
                            } else {
                                let fail_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderUpdateRejected {
                                    account, order_id, reason: String::from("No pending order found"), time: time.to_string()
                                });
                                match strategy_event_sender.send(fail_event).await {
                                    Ok(_) => {}
                                    Err(e) => eprintln!("Timed Event Handler: Failed to send event: {}", e)
                                }
                            }
                        }
                        OrderRequest::CancelAll { account, symbol_name } => {
                            let mut remove = vec![];
                            for order in open_order_cache.iter() {
                                if order.account == account && order.symbol_name == symbol_name {
                                    remove.push(order.id.clone());
                                }
                            }
                            for order_id in remove {
                                if let Some((order_id, mut order)) = open_order_cache.remove(&order_id) {
                                    order.state = OrderState::Cancelled;
                                    let cancel_event = StrategyEvent::OrderEvents(
                                        OrderUpdateEvent::OrderCancelled {
                                            account: account.clone(),
                                            symbol_name: order.symbol_name.clone(),
                                            symbol_code: order.symbol_name.clone(),
                                            order_id: order.id.clone(),
                                            reason: "OrderRequest::CancelAll".to_string(),
                                            tag: order.tag.clone(),
                                            time: time.to_string(),
                                        });
                                    match strategy_event_sender.send(cancel_event).await {
                                        Ok(_) => {}
                                        Err(e) => eprintln!("Timed Event Handler: Failed to send event: {}", e)
                                    }
                                    closed_order_cache.insert(order_id, order);
                                }
                            }
                        }
                        OrderRequest::FlattenAllFor { account: _} => {
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
                    if !open_order_cache.is_empty() {
                        simulated_order_matching(time.clone(), &open_order_cache, &closed_order_cache, strategy_event_sender.clone()).await;
                    }

                }
            }
            //notify.notify_one();
        }
    });
    sender
}

pub async fn simulated_order_matching (
    time: DateTime<Utc>,
    open_order_cache: &Arc<DashMap<OrderId, Order>>,
    closed_order_cache: &Arc<DashMap<OrderId, Order>>,
    strategy_event_sender: Sender<StrategyEvent>
) {
    let mut rejected = Vec::new();
    let mut accepted = Vec::new();
    let mut cancelled = Vec::new();
    let mut filled = Vec::new();
    let mut events= vec![];
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
                    cancelled.push((order.id.clone(), reason));
                }
            }
            TimeInForce::IOC=> {
               /* if time > order.time_created_utc() + Duration::seconds(1) {
                    let reason = "Time In Force Expired: TimeInForce::IOC".to_string();
                    cancelled.push((order.id.clone(), reason));
                }*/
            }
            TimeInForce::FOK => {
                /*if time > order.time_created_utc() + buffer_resolution  {
                   let reason = "Time In Force Expired: TimeInForce::FOK".to_string();
                    cancelled.push((order.id.clone(), reason));
                }*/
            }
            TimeInForce::Time(cancel_time, time_zone_string) => {
                let tz: Tz = time_zone_string.parse().unwrap();
                let local_time = time_convert_utc_to_local(&tz, time);
                let cancel_time = tz.timestamp_opt(*cancel_time, 0).unwrap();
                if local_time >= cancel_time {
                    let reason = "Time In Force Expired: TimeInForce::Time".to_string();
                    cancelled.push((order.id.clone(), reason));
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
                if LEDGER_SERVICE.is_short(&order.account, &order.symbol_name) {
                    match LEDGER_SERVICE.paper_exit_position(&order.account,  &order.symbol_name, time, market_fill_price, String::from("Force Exit By Enter Long")).await {
                        Some(event) => {
                            events.push(StrategyEvent::PositionEvents(event));
                        }
                        None => {}
                    }
                }
                filled.push((order.id.clone(), market_fill_price));
            }
            OrderType::EnterShort => {
                let market_fill_price = match price_service_request_market_fill_price(order.side, order.symbol_name.clone(), order.quantity_filled).await {
                    Ok(price) => match price.price() {
                        None => continue,
                        Some(price) => price
                    }
                    Err(_) => continue,
                };
                if LEDGER_SERVICE.is_long(&order.account, &order.symbol_name) {
                    match LEDGER_SERVICE.paper_exit_position(&order.account,  &order.symbol_name, time, market_fill_price, String::from("Force Exit By Enter Short")).await {
                        Some(event) => {
                            events.push(StrategyEvent::PositionEvents(event));
                        }
                        None => {}
                    }
                }
                filled.push((order.id.clone(), market_fill_price));
            }
            OrderType::ExitLong => {
                let long_quantity = LEDGER_SERVICE.position_size(&order.account, &order.symbol_name);
                let is_long = LEDGER_SERVICE.is_long(&order.account, &order.symbol_name);
                if long_quantity == dec!(0.0) && is_long {
                    let reason = "No Long Position To Exit".to_string();
                    rejected.push((order.id.clone(), reason));
                    continue;
                };
                let adjusted_size = match order.quantity_open > long_quantity && is_long {
                    true => long_quantity,
                    false => order.quantity_open
                };
                let market_fill_price = match price_service_request_market_fill_price(order.side, order.symbol_name.clone(), adjusted_size).await {
                    Ok(price) => match price.price() {
                        None => continue,
                        Some(price) => price
                    }
                    Err(_) => continue,
                };
                if is_long {
                    filled.push((order.id.clone(), market_fill_price));
                } else {
                    let reason = "No Long Position To Exit".to_string();
                    rejected.push((order.id.clone(), reason));
                }
            }
            OrderType::ExitShort => {
                let short_quantity = LEDGER_SERVICE.position_size(&order.account, &order.symbol_name);
                let is_short = LEDGER_SERVICE.is_short(&order.account, &order.symbol_name);
                if short_quantity == dec!(0.0) && is_short {
                    let reason = "No Short Position To Exit".to_string();
                    rejected.push((order.id.clone(), reason));
                    continue;
                };
                let adjusted_size = match order.quantity_open > short_quantity && is_short {
                    true => short_quantity,
                    false => order.quantity_open
                };
                let market_fill_price = match price_service_request_market_fill_price(order.side, order.symbol_name.clone(), adjusted_size).await {
                    Ok(price) => match price.price() {
                        None => continue,
                        Some(price) => price
                    }
                    Err(_) => continue,
                };
                if is_short {
                   filled.push((order.id.clone(), market_fill_price));
                } else {
                    let reason = "No Short Position To Exit".to_string();
                    rejected.push((order.id.clone(), reason));
                }
            }
        }
    }

    for (order_id, reason) in rejected {
        reject_order(reason, &order_id, time, &open_order_cache, closed_order_cache, &strategy_event_sender).await;
    }
    for (order_id , time)in accepted {
        accept_order(&order_id, time, &open_order_cache, &strategy_event_sender).await;
    }
    for (order_id, price) in filled {
        fill_order(&order_id, time, price, &open_order_cache, &closed_order_cache, &strategy_event_sender).await;
    }
    for (order_id, price, volume) in partially_filled {
        partially_fill_order(&order_id, time, price, volume, &open_order_cache, &closed_order_cache, &strategy_event_sender).await;
    }

    for (order_id, reason) in cancelled {
        cancel_order(reason, &order_id, time, &open_order_cache, closed_order_cache, &strategy_event_sender).await;
    }

    for event in events {
        strategy_event_sender.send(event).await.unwrap();
    }
}

async fn fill_order(
    order_id: &OrderId,
    time: DateTime<Utc>,
    market_price: Price,
    open_order_cache: &Arc<DashMap<OrderId, Order>>,
    closed_order_cache: &Arc<DashMap<OrderId, Order>>,
    strategy_event_sender: &Sender<StrategyEvent>
) {
    if let Some((_, mut order)) = open_order_cache.remove(order_id) {  // Remove the order here
        let symbol_code = match &order.symbol_code {
            None => order.symbol_name.clone(),
            Some(code) => code.clone()
        };

        order.quantity_filled = order.quantity_open;
        order.quantity_open = dec!(0);

        match LEDGER_SERVICE.update_or_create_position(&order.account, order.symbol_name.clone(), symbol_code.clone(), order_id.clone(), order.quantity_filled.clone(), order.side.clone(), time.clone(), market_price, order.tag.clone()).await {
            Ok(events) => {
                //todo, need to send an accepted event first if the order state != accepted
                let order_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderFilled {
                    account: order.account.clone(),
                    symbol_name: order.symbol_name.clone(),
                    symbol_code: symbol_code,
                    order_id: order.id.clone(),
                    price: market_price,
                    quantity: order.quantity_filled.clone(),
                    tag: order.tag.clone(),
                    time: time.to_string(),
                    side: order.side.clone(),
                });
                match strategy_event_sender.send(order_event).await {
                    Ok(_) => {}
                    Err(e) => eprintln!("Backtest Matching Engine: Failed to send event: {}", e)
                }
                for event in events {
                    match strategy_event_sender.send(StrategyEvent::PositionEvents(event)).await {
                        Ok(_) => {}
                        Err(e) => eprintln!("Backtest Matching Engine: Failed to send event: {}", e)
                    }
                }
            }
            Err(rejection_event) => {
                match strategy_event_sender.send(StrategyEvent::OrderEvents(rejection_event)).await {
                    Ok(_) => {}
                    Err(e) => eprintln!("Backtest Matching Engine: Failed to send event: {}", e)
                }
            }
        }
        closed_order_cache.insert(order.id.clone(), order);
    }
}
async fn partially_fill_order(
    order_id: &OrderId,
    time: DateTime<Utc>,
    fill_price: Price,
    fill_volume: Volume,
    open_order_cache: &Arc<DashMap<OrderId, Order>>,
    closed_order_cache: &Arc<DashMap<OrderId, Order>>,
    strategy_event_sender: &Sender<StrategyEvent>
) {
    if let Some(mut order) = open_order_cache.get_mut(order_id) {
        let symbol_code = match &order.symbol_code {
            None => order.symbol_name.clone(),
            Some(code) => code.clone()
        };


        // If the order is now fully filled, remove it from open_order_cache
        let order_event = if order.quantity_open <= dec!(0) {
            open_order_cache.remove(order_id);
            Some(OrderUpdateEvent::OrderFilled {
                order_id: order.id.clone(),
                account: order.account.clone(),
                symbol_name: order.symbol_name.clone(),
                tag: order.tag.clone(),
                time: time.to_string(),
                symbol_code: order.symbol_name.clone(),
                quantity: fill_volume,
                price: fill_price,
                side: order.side.clone(),
            })

        } else if fill_volume > dec!(0) {
            Some(OrderUpdateEvent::OrderPartiallyFilled {
                order_id: order.id.clone(),
                account: order.account.clone(),
                symbol_name: order.symbol_name.clone(),
                tag: order.tag.clone(),
                time: time.to_string(),
                symbol_code: order.symbol_name.clone(),
                quantity: fill_volume,
                price: fill_price,
                side: order.side.clone(),
            })
        } else {
            None
        };

        match LEDGER_SERVICE.update_or_create_position(&order.account, order.symbol_name.clone(), symbol_code, order_id.clone(), fill_volume, order.side.clone(), time, fill_price, order.tag.clone()).await {
            Ok(events) => {
                //todo, need to send an accepted event first if the order state != accepted
                if let Some(order_event) = order_event {
                    match strategy_event_sender.send(StrategyEvent::OrderEvents(order_event)).await {
                        Ok(_) => {}
                        Err(e) => eprintln!("Backtest Matching Engine: Failed to send event: {}", e)
                    }
                }
                for event in events {
                    match strategy_event_sender.send(StrategyEvent::PositionEvents(event)).await {
                        Ok(_) => {}
                        Err(e) => eprintln!("Backtest Matching Engine: Failed to send event: {}", e)
                    }
                }
            }
            Err(rejection_event) => {
                match strategy_event_sender.send(StrategyEvent::OrderEvents(rejection_event)).await {
                    Ok(_) => {}
                    Err(e) => eprintln!("Backtest Matching Engine: Failed to send event: {}", e)
                }
            }
        }

        // Update the remaining quantity
        order.quantity_open -= fill_volume;

        if order.quantity_open == dec!(0) {
            closed_order_cache.insert(order.id.clone(), order.value().clone());
        }
    }
}

async fn reject_order(
    reason: String,
    order_id: &OrderId,
    time: DateTime<Utc>,
    open_order_cache: &Arc<DashMap<OrderId, Order>>,
    closed_order_cache: &Arc<DashMap<OrderId, Order>>,
    strategy_event_sender: &Sender<StrategyEvent>
) {
    if let Some((_, mut order)) = open_order_cache.remove(order_id) {
        order.state = OrderState::Rejected(reason.clone());
        order.time_created_utc = time.to_string();

        let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderRejected {
                order_id: order.id.clone(),
                account: order.account.clone(),
                symbol_name: order.symbol_name.clone(),
                reason,
                tag: order.tag.clone(),
                time: time.to_string(),
                symbol_code: order.symbol_name.clone(),
            });
        closed_order_cache.insert(order.id.clone(), order.clone());
        match strategy_event_sender.send(event).await {
            Ok(_) => {}
            Err(e) => eprintln!("Backtest Matching Engine: Failed to send event: {}", e)
        }
    }
}

async fn cancel_order(
    reason: String,
    order_id: &OrderId,
    time: DateTime<Utc>,
    open_order_cache: &Arc<DashMap<OrderId, Order>>,
    closed_order_cache: &Arc<DashMap<OrderId, Order>>,
    strategy_event_sender: &Sender<StrategyEvent>
) {
    if let Some((_, mut order)) = open_order_cache.remove(order_id) {
        order.state = OrderState::Rejected(reason.clone());
        order.time_created_utc = time.to_string();

        let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderCancelled {
            order_id: order.id.clone(),
            account: order.account.clone(),
            symbol_name: order.symbol_name.clone(),
            reason,
            tag: order.tag.clone(),
            time: time.to_string(),
            symbol_code: order.symbol_name.clone(),
        });
        closed_order_cache.insert(order.id.clone(), order.clone());
        match strategy_event_sender.send(event).await {
            Ok(_) => {}
            Err(e) => eprintln!("Backtest Matching Engine: Failed to send event: {}", e)
        }
    }
}

async fn accept_order(
    order_id: &OrderId,
    time: DateTime<Utc>,
    open_order_cache: &Arc<DashMap<OrderId, Order>>,
    strategy_event_sender: &Sender<StrategyEvent>
) {
    if let Some(mut order) = open_order_cache.get_mut(order_id) {
        order.state = OrderState::Accepted;
        order.time_created_utc = time.to_string();

        let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderAccepted {
                order_id: order.id.clone(),
                account: order.account.clone(),
                symbol_name: order.symbol_name.clone(),
                tag: order.tag.clone(),
                time: time.to_string(),
                symbol_code: order.symbol_name.clone(),
        });
        match strategy_event_sender.send(event).await {
            Ok(_) => {}
            Err(e) => eprintln!("Backtest Matching Engine: Failed to send event: {}", e)
        }
    }
}
