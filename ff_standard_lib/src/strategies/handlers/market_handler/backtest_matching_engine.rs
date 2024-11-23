use chrono::{DateTime, Datelike, NaiveDateTime, NaiveTime, TimeZone, Utc, Weekday};
use chrono_tz::Tz;
use dashmap::DashMap;
use std::sync::Arc;
use rust_decimal_macros::dec;
use tokio::sync::mpsc::{Sender};
use tokio::sync::oneshot;
use crate::helpers::converters::{time_convert_utc_to_local};
use crate::standardized_types::broker_enum::Brokerage;
use crate::standardized_types::enums::{OrderSide};
use crate::product_maps::rithmic::maps::get_futures_trading_hours;
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::orders::{Order, OrderId, OrderRequest, OrderState, OrderType, OrderUpdateEvent, OrderUpdateType, TimeInForce};
use crate::strategies::handlers::market_handler::price_service::{price_service_request_limit_fill_price_quantity, price_service_request_market_fill_price, price_service_request_market_price, PriceServiceResponse};
use crate::strategies::historical_time::get_backtest_time;
use crate::strategies::ledgers::ledger_service::{LedgerService};
use crate::strategies::strategy_events::StrategyEvent;

pub enum BackTestEngineMessage {
    TickBufferTime,
    OrderRequest(OrderRequest)
}

pub(crate) async fn backtest_matching_engine(
    open_order_cache: Arc<DashMap<OrderId, Order>>, //todo, make these static or lifetimes if possible.. might not be optimal though, look it up!
    closed_order_cache: Arc<DashMap<OrderId, Order>>,
    strategy_event_sender: Sender<StrategyEvent>,
    ledger_service: Arc<LedgerService>,
    notify: Arc<tokio::sync::Notify>
) -> Sender<BackTestEngineMessage> {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(100);
    tokio::task::spawn(async move {
       notify.notify_one();
        while let Some(backtest_message) = receiver.recv().await {
            match backtest_message {
                BackTestEngineMessage::OrderRequest(order_request) => {
                    //println!("{:?}", order_request);
                    let time = get_backtest_time();
                    match order_request {
                        OrderRequest::Create { account, mut order, .. } => {
                            let market_price = match price_service_request_market_price(order.side, order.symbol_name.clone(), order.symbol_code.clone()).await {
                                Ok(price) => match price.price() {
                                    None => panic!("No market price found"),
                                    Some(price) => price
                                }
                                Err(_) => panic!("No market price found")
                            };
                            //eprintln!("Market Price: {}", market_price);
                            if order.quantity_open <= dec!(0) {
                                open_order_cache.remove(&order.id);
                                let fail_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderRejected {
                                    account,
                                    symbol_name: order.symbol_name,
                                    symbol_code: order.symbol_code,
                                    order_id: order.id.clone(), reason: String::from("Order Quantity Must Be Greater Than Zero"),
                                    tag: order.tag,
                                    time: time.to_string()
                                });
                                match strategy_event_sender.send(fail_event).await {
                                    Ok(_) => {}
                                    Err(e) => eprintln!("Timed Event Handler: Failed to send event: {}", e)
                                }
                                continue
                            }
                            // check limit price
                            if (order.order_type == OrderType::StopLimit || order.order_type == OrderType::Limit) &&
                                ((order.side == OrderSide::Buy && order.limit_price.unwrap() > market_price)
                                || (order.side == OrderSide::Sell && order.limit_price.unwrap() < market_price))
                            {
                                let side_string = match order.side {
                                    OrderSide::Buy => "Below",
                                    OrderSide::Sell => "Above"
                                };
                                let fail_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderRejected {
                                    account,
                                    symbol_name: order.symbol_name,
                                    symbol_code: order.symbol_code,
                                    order_id: order.id.clone(), reason: String::from(format!("{} Limit Price Must Be {} or Equal to Market Price", order.side, side_string)),
                                    tag: order.tag,
                                    time: time.to_string()
                                });
                                match strategy_event_sender.send(fail_event).await {
                                    Ok(_) => {}
                                    Err(e) => eprintln!("Timed Event Handler: Failed to send event: {}", e)
                                }
                                continue
                            }
                            ///check trigger price
                            if (order.order_type == OrderType::StopMarket || order.order_type == OrderType::StopLimit || order.order_type == OrderType::MarketIfTouched) &&
                                ((order.side == OrderSide::Sell && order.trigger_price.unwrap() >= market_price) ||
                                    (order.side == OrderSide::Buy && order.trigger_price.unwrap() <= market_price))
                            {

                                let side_string = match order.side {
                                    OrderSide::Buy => "Above",  // Changed from "Below"
                                    OrderSide::Sell => "Below"  // Changed from "Above"
                                };
                                let fail_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderRejected {
                                    account,
                                    symbol_name: order.symbol_name,
                                    symbol_code: order.symbol_code,
                                    order_id: order.id.clone(), reason: String::from(format!("{} Trigger Price Must Be {} Market Price", order.side, side_string)),
                                    tag: order.tag,
                                    time: time.to_string()
                                });
                                match strategy_event_sender.send(fail_event).await {
                                    Ok(_) => {}
                                    Err(e) => eprintln!("Timed Event Handler: Failed to send event: {}", e)
                                }
                                continue
                            }

                            if order.order_type == OrderType::ExitLong && !ledger_service.is_long(&account, &order.symbol_code) {
                                let fail_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderRejected {
                                    account,
                                    symbol_name: order.symbol_name,
                                    symbol_code: order.symbol_code,
                                    order_id: order.id.clone(), reason: String::from("No Long Position To Exit"),
                                    tag: order.tag,
                                    time: time.to_string()
                                });
                                match strategy_event_sender.send(fail_event).await {
                                    Ok(_) => {}
                                    Err(e) => eprintln!("Timed Event Handler: Failed to send event: {}", e)
                                }
                                continue
                            }

                            if order.order_type == OrderType::ExitShort && !ledger_service.is_short(&account, &order.symbol_code) {
                                let fail_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderRejected {
                                    account,
                                    symbol_name: order.symbol_name,
                                    symbol_code: order.symbol_code,
                                    order_id: order.id.clone(), reason: String::from("No Short Position To Exit"),
                                    tag: order.tag,
                                    time: time.to_string()
                                });
                                match strategy_event_sender.send(fail_event).await {
                                    Ok(_) => {}
                                    Err(e) => eprintln!("Timed Event Handler: Failed to send event: {}", e)
                                }
                                continue
                            }

                            order.state = OrderState::Accepted;
                            {
                                open_order_cache.insert(order.id.clone(), order.clone());
                            }
                            let accept_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderAccepted {
                                account: account.clone(),
                                symbol_name: order.symbol_name.clone(),
                                symbol_code: order.symbol_code.clone(),
                                order_id: order.id.clone(),
                                tag: order.tag.clone(),
                                time: time.to_string()
                            });
                            match strategy_event_sender.send(accept_event).await {
                                Ok(_) => {}
                                Err(e) => eprintln!("Timed Event Handler: Failed to send event: {}", e)
                            }
                            simulated_order_matching(&open_order_cache, &closed_order_cache, strategy_event_sender.clone(), &ledger_service).await;
                        }
                        OrderRequest::Cancel { account,order_id } => {
                            let existing_order = open_order_cache.remove(&order_id);
                            if let Some((existing_order_id, order)) = existing_order {
                                let cancel_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderCancelled {
                                    account,
                                    symbol_name: order.symbol_name.clone(),
                                    symbol_code: order.symbol_code.clone(),
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
                            simulated_order_matching(&open_order_cache, &closed_order_cache, strategy_event_sender.clone(), &ledger_service).await;
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
                                    OrderUpdateType::Quantity(quantity) => {
                                        order.quantity_open = quantity.clone();
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
                            simulated_order_matching(&open_order_cache, &closed_order_cache, strategy_event_sender.clone(), &ledger_service).await;
                        }
                        OrderRequest::CancelAll { account } => {
                            let mut remove = vec![];
                            for order in open_order_cache.iter() {
                                if order.account == account {
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
                                            symbol_code: order.symbol_code.clone(),
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
                            simulated_order_matching(&open_order_cache, &closed_order_cache, strategy_event_sender.clone(), &ledger_service).await;
                        }
                        OrderRequest::FlattenAllFor { account} => {
                            let orders_to_remove: Vec<_> = open_order_cache.iter()
                                .filter(|order| order.account == account)
                                .map(|order| order.id.clone())
                                .collect();

                            for order_id in orders_to_remove {
                                let (_, mut order) = open_order_cache.remove(&order_id).unwrap(); // Changed here
                                order.state = OrderState::Cancelled;
                                let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderCancelled {
                                    account: account.clone(),
                                    symbol_name: order.symbol_name.clone(),
                                    symbol_code: order.symbol_code.clone(),
                                    order_id,  // Using the order_id from the for loop
                                    reason: "Flatten All".to_string(),
                                    tag: order.tag.clone(),
                                    time: time.to_string(),
                                });
                                match strategy_event_sender.send(event).await {
                                    Ok(_) => {}
                                    Err(e) => eprintln!("Timed Event Handler: Failed to send event: {}", e)
                                }
                                closed_order_cache.insert(order.id.clone(), order);
                            }
                            ledger_service.flatten_all_for_paper_account(account, time).await;
                        }
                    }
                }
                BackTestEngineMessage::TickBufferTime => {
                    if !open_order_cache.is_empty() {
                        simulated_order_matching(&open_order_cache, &closed_order_cache, strategy_event_sender.clone(), &ledger_service).await;
                    }
                    notify.notify_one();
                }
            }
        }
    });
    sender
}

pub(crate) async fn simulated_order_matching (
    open_order_cache: &Arc<DashMap<OrderId, Order>>,
    closed_order_cache: &Arc<DashMap<OrderId, Order>>,
    strategy_event_sender: Sender<StrategyEvent>,
    ledger_service: &Arc<LedgerService>
) {
    let time = get_backtest_time();
    let mut rejected = Vec::new();
    let mut cancelled = Vec::new();
    let mut filled = Vec::new();
    let mut partially_filled = Vec::new();
    for order in open_order_cache.iter() {
        //println!("Order matching: {:?}", order.value());
        match &order.time_in_force {
            TimeInForce::GTC => {},
            TimeInForce::Day => {
                let tz: Tz = order.account.brokerage.timezone();
                let close_time: DateTime<Utc> = match order.account.brokerage {
                    Brokerage::Rithmic(_) => {
                        match get_futures_trading_hours(&order.symbol_name) {
                            None => {
                                // CME closes at 17:00 Eastern Time
                                let local_date = NaiveDateTime::new(
                                    time_convert_utc_to_local(&tz, order.time_created_utc()).date_naive(),
                                    NaiveTime::from_hms_opt(17, 0, 0).unwrap()
                                );
                                tz.from_local_datetime(&local_date)
                                    .unwrap()
                                    .with_timezone(&Utc)
                            }
                            Some(hours) => {
                                match order.time_created_utc().date_naive().weekday() {
                                    Weekday::Sun => {
                                        let time_naive = hours.monday.close.unwrap();
                                        let date_naive = order.time_created_utc()
                                            .date_naive()
                                            .succ_opt()
                                            .unwrap();
                                        let local_dt = NaiveDateTime::new(date_naive, time_naive);
                                        tz.from_local_datetime(&local_dt)
                                            .unwrap()
                                            .to_utc()
                                    }
                                    Weekday::Sat => {
                                        let reason = "Market Closed".to_string();
                                        cancelled.push((order.id.clone(), reason));
                                        continue
                                    }
                                    _ => {
                                        let time_naive = hours.monday.close.unwrap();
                                        let date_naive = order.time_created_utc().date_naive();
                                        let local_dt = NaiveDateTime::new(date_naive, time_naive);
                                        tz.from_local_datetime(&local_dt)
                                            .unwrap()
                                            .to_utc()
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        // For test broker, use end of day in local timezone
                        let local_date = NaiveDateTime::new(
                            time_convert_utc_to_local(&tz, order.time_created_utc()).date_naive(),
                            NaiveTime::from_hms_opt(23, 59, 59).unwrap()
                        );
                        tz.from_local_datetime(&local_date)
                            .unwrap()
                            .to_utc()
                    }
                };

                if time >= close_time {
                    let reason = "Time In Force Expired: TimeInForce::Day".to_string();
                    cancelled.push((order.id.clone(), reason));
                    continue
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
            TimeInForce::Time(cancel_time) => {
                let cancel_time = match DateTime::<Utc>::from_timestamp(*cancel_time, 0) {
                    Some(time) => time,
                    None => {
                        eprintln!("Backtest Matching Engine: Invalid TimeInForce::Time value");
                        let reason = "Time In Force Expired: TimeInForce::Time".to_string();
                        rejected.push((order.id.clone(), reason));
                        continue;
                    }
                };
                if time >= cancel_time {
                    let reason = "Time In Force Expired: TimeInForce::Time".to_string();
                    cancelled.push((order.id.clone(), reason));
                    continue
                }
            }
        }
        //3. respond with an order event
        match &order.order_type {
            OrderType::Limit => {
                let market_price = match price_service_request_market_price(order.side, order.symbol_name.clone(), order.symbol_code.clone()).await {
                    Ok(price) => match price.price() {
                        None => continue,
                        Some(price) => price
                    }
                    Err(_) => continue
                };

                let is_fill_triggered = match order.side {
                    OrderSide::Buy => market_price <= order.limit_price.unwrap(),
                    OrderSide::Sell => market_price >= order.limit_price.unwrap()
                };
                let (market_price, volume_filled) = match price_service_request_limit_fill_price_quantity(order.side, order.symbol_name.clone(), order.symbol_code.clone(), order.quantity_open, order.limit_price.unwrap()).await {
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
                }
            }
            OrderType::Market => {
                let market_price = match price_service_request_market_fill_price(order.side, order.symbol_name.clone(), order.symbol_code.clone(), order.quantity_open).await {
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
                let market_price = match price_service_request_market_price(order.side, order.symbol_name.clone(), order.symbol_code.clone(),).await {
                    Ok(price) => match price.price() {
                        None => continue,
                        Some(price) => price
                    }
                    Err(_) => continue
                };

                let trigger_price = order.trigger_price.unwrap();
                let is_fill_triggered = match order.side {
                    OrderSide::Buy => market_price >= trigger_price,
                    OrderSide::Sell => market_price <= trigger_price,
                };

                let market_fill_price = match price_service_request_market_fill_price(order.side, order.symbol_name.clone(), order.symbol_code.clone(), order.quantity_open).await {
                    Ok(price) => match price.price() {
                        None => continue,
                        Some(price) => price
                    }
                    Err(_) => continue,
                };

                if is_fill_triggered {
                    filled.push((order.id.clone(), market_fill_price));
                }
            }

            // Handle OrderType::MarketIfTouched separately
            OrderType::MarketIfTouched => {
                let market_price = match price_service_request_market_price(order.side, order.symbol_name.clone(), order.symbol_code.clone(),).await {
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

                let market_fill_price = match price_service_request_market_fill_price(order.side, order.symbol_name.clone(), order.symbol_code.clone(), order.quantity_open).await {
                    Ok(price) => match price.price() {
                        None => continue,
                        Some(price) => price
                    }
                    Err(_) => continue,
                };

                if is_fill_triggered {
                    filled.push((order.id.clone(), market_fill_price));
                }
            }
            OrderType::StopLimit => {
                let market_price = match price_service_request_market_price(order.side, order.symbol_name.clone(), order.symbol_code.clone(),).await {
                    Ok(price) => match price.price() {
                        None => continue,
                        Some(price) => price
                    }
                    Err(_) => continue
                };

                let is_fill_triggered = match order.side {
                    OrderSide::Buy => market_price <= order.trigger_price.unwrap() && market_price > order.limit_price.unwrap(),
                    OrderSide::Sell => market_price >= order.trigger_price.unwrap() && market_price < order.limit_price.unwrap()
                };
                let (market_price, volume_filled) = match price_service_request_limit_fill_price_quantity(order.side, order.symbol_name.clone(), order.symbol_code.clone(), order.quantity_open, order.limit_price.unwrap()).await {
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
                }
            },
            OrderType::EnterLong => {
                let market_fill_price = match price_service_request_market_fill_price(order.side, order.symbol_name.clone(), order.symbol_code.clone(), order.quantity_open).await {
                    Ok(price) => match price.price() {
                        None => continue,
                        Some(price) => price
                    }
                    Err(_) => continue
                };
                if ledger_service.is_short(&order.account, &order.symbol_code) {
                    ledger_service.paper_exit_position(&order.account,  order.symbol_code.clone(), time, market_fill_price, String::from("Force Exit By Enter Long")).await;
                }
                filled.push((order.id.clone(), market_fill_price));
            }
            OrderType::EnterShort => {
                let market_fill_price = match price_service_request_market_fill_price(order.side, order.symbol_name.clone(), order.symbol_code.clone(), order.quantity_open).await {
                    Ok(price) => match price.price() {
                        None => continue,
                        Some(price) => price
                    }
                    Err(_) => continue,
                };
                if ledger_service.is_long(&order.account, &order.symbol_code) {
                    ledger_service.paper_exit_position(&order.account,  order.symbol_code.clone(), time, market_fill_price, String::from("Force Exit By Enter Short")).await;
                }
                filled.push((order.id.clone(), market_fill_price));
            }
            OrderType::ExitLong => {
                let long_quantity = ledger_service.position_size(&order.account, &order.symbol_code);
                let is_long = ledger_service.is_long(&order.account, &order.symbol_code);
                if long_quantity <= dec!(0.0) || !is_long {
                    let reason = "No Long Position To Exit".to_string();
                    rejected.push((order.id.clone(), reason));
                    continue;
                };
                let adjusted_size = match order.quantity_open > long_quantity {
                    true => long_quantity,
                    false => order.quantity_open
                };
                let market_fill_price = match price_service_request_market_fill_price(order.side, order.symbol_name.clone(), order.symbol_code.clone(), adjusted_size).await {
                    Ok(price) => match price.price() {
                        None => continue,
                        Some(price) => price
                    }
                    Err(_) => continue,
                };

                filled.push((order.id.clone(), market_fill_price));
            }
            OrderType::ExitShort => {
                let short_quantity = ledger_service.position_size(&order.account, &order.symbol_code);
                let is_short = ledger_service.is_short(&order.account, &order.symbol_code);
                if short_quantity <= dec!(0.0) || !is_short {
                    let reason = "No Short Position To Exit".to_string();
                    rejected.push((order.id.clone(), reason));
                    continue;
                };
                let adjusted_size = match order.quantity_open > short_quantity {
                    true => short_quantity,
                    false => order.quantity_open
                };
                let market_fill_price = match price_service_request_market_fill_price(order.side, order.symbol_name.clone(), order.symbol_code.clone(), adjusted_size).await {
                    Ok(price) => match price.price() {
                        None => continue,
                        Some(price) => price
                    }
                    Err(_) => continue,
                };
                filled.push((order.id.clone(), market_fill_price));
            }
        }
    }

    for (order_id, reason) in rejected {
        reject_order(reason, &order_id, time, &open_order_cache, closed_order_cache, &strategy_event_sender).await;
    }
    for (order_id, price) in filled {
        fill_order(&order_id, time, price, &open_order_cache, &closed_order_cache, &strategy_event_sender, &ledger_service).await;
    }
    for (order_id, price, volume) in partially_filled {
        partially_fill_order(&order_id, time, price, volume, &open_order_cache, &closed_order_cache, &strategy_event_sender, &ledger_service).await;
    }

    for (order_id, reason) in cancelled {
        cancel_order(reason, &order_id, time, &open_order_cache, closed_order_cache, &strategy_event_sender).await;
    }
}

async fn fill_order(
    order_id: &OrderId,
    time: DateTime<Utc>,
    market_price: Price,
    open_order_cache: &Arc<DashMap<OrderId, Order>>,
    closed_order_cache: &Arc<DashMap<OrderId, Order>>,
    strategy_event_sender: &Sender<StrategyEvent>,
    ledger_service: &Arc<LedgerService>
) {
    if let Some((_, mut order)) = open_order_cache.remove(order_id) {  // Remove the order here
       let (sender, receiver) = oneshot::channel();
        ledger_service.update_or_create_position(&order.account, order.symbol_name.clone(), order.symbol_code.clone(), order.quantity_open.clone(), order.side.clone(), time.clone(), market_price, order.tag.clone(), Some(sender), Some(order_id.clone())).await;
        match receiver.await {
            Ok(event) => {
                match event {
                    Some(event) => {
                        order.state = OrderState::Rejected("Insufficient Funds".to_string());
                        match strategy_event_sender.send(StrategyEvent::OrderEvents(event)).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("Backtest Matching Engine: Failed to send event: {}", e)
                        }
                        closed_order_cache.insert(order.id.clone(), order);
                    }
                    None => {

                        //todo, need to send an accepted event first if the order state != accepted
                        let order_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderFilled {
                            account: order.account.clone(),
                            symbol_name: order.symbol_name.clone(),
                            symbol_code: order.symbol_code.clone(),
                            order_id: order.id.clone(),
                            price: market_price,
                            quantity: order.quantity_open.clone(),
                            tag: order.tag.clone(),
                            time: time.to_string(),
                            side: order.side.clone(),
                        });
                        order.quantity_filled += order.quantity_open.clone();
                        order.quantity_open = dec!(0.0);
                        match strategy_event_sender.send(order_event).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("Backtest Matching Engine: Failed to send event: {}", e)
                        }
                        closed_order_cache.insert(order.id.clone(), order);
                    }
                }
            }
            Err(e) => eprintln!("Backtest Matching Engine: Failed to receive event: {}", e)
        }
    }
}

async fn partially_fill_order(
    order_id: &OrderId,
    time: DateTime<Utc>,
    fill_price: Price,
    fill_volume: Volume,
    open_order_cache: &Arc<DashMap<OrderId, Order>>,
    closed_order_cache: &Arc<DashMap<OrderId, Order>>,
    strategy_event_sender: &Sender<StrategyEvent>,
    ledger_service: &Arc<LedgerService>
) {
    if let Some((_, mut order)) = open_order_cache.remove(order_id) {
        let (sender, receiver) = oneshot::channel();
        ledger_service.update_or_create_position(&order.account, order.symbol_name.clone(),  order.symbol_code.clone(), fill_volume, order.side.clone(), time, fill_price, order.tag.clone(), Some(sender), Some(order_id.clone())).await;

        match receiver.await {
            Ok(event) => {
                match event {
                    Some(event) => {
                        order.state = OrderState::Rejected("Insufficient Funds".to_string());
                        match strategy_event_sender.send(StrategyEvent::OrderEvents(event)).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("Backtest Matching Engine: Failed to send event: {}", e)
                        }
                        closed_order_cache.insert(order.id.clone(), order);
                    }
                    None => {
                        order.quantity_open -= fill_volume;
                        order.quantity_filled += fill_volume;
                        let is_fully_filled = order.quantity_open <= dec!(0);

                        let order_event = if is_fully_filled {
                            OrderUpdateEvent::OrderFilled {
                                order_id: order.id.clone(),
                                account: order.account.clone(),
                                symbol_name: order.symbol_name.clone(),
                                tag: order.tag.clone(),
                                time: time.to_string(),
                                symbol_code: order.symbol_code.clone(),
                                quantity: fill_volume,
                                price: fill_price,
                                side: order.side.clone(),
                            }
                        } else {
                            OrderUpdateEvent::OrderPartiallyFilled {
                                order_id: order.id.clone(),
                                account: order.account.clone(),
                                symbol_name: order.symbol_name.clone(),
                                tag: order.tag.clone(),
                                time: time.to_string(),
                                symbol_code: order.symbol_code.clone(),
                                quantity: fill_volume,
                                price: fill_price,
                                side: order.side.clone(),
                            }
                        };
                        match strategy_event_sender.send(StrategyEvent::OrderEvents(order_event)).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("Backtest Matching Engine: Failed to send event: {}", e)
                        }
                        if is_fully_filled {
                            closed_order_cache.insert(order.id.clone(), order);
                        } else {
                            open_order_cache.insert(order_id.clone(), order);
                        }
                    }
                }
            }
            Err(e) => eprintln!("Backtest Matching Engine: Failed to receive event: {}", e)
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
                symbol_code: order.symbol_code.clone(),
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
            symbol_code: order.symbol_code.clone(),
        });
        closed_order_cache.insert(order.id.clone(), order.clone());
        match strategy_event_sender.send(event).await {
            Ok(_) => {}
            Err(e) => eprintln!("Backtest Matching Engine: Failed to send event: {}", e)
        }
    }
}
