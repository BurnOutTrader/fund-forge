use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use rust_decimal_macros::dec;
use crate::helpers::converters::{time_convert_utc_to_local, time_local_from_utc_str};
use crate::helpers::decimal_calculators::round_to_tick_size;
use crate::messages::data_server_messaging::FundForgeError;
use crate::standardized_types::broker_enum::Brokerage;
use crate::standardized_types::enums::OrderSide;
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::orders::{OrderId, OrderState, OrderType, OrderUpdateEvent, TimeInForce};
use crate::standardized_types::subscriptions::SymbolName;
use crate::strategies::client_features::server_connections::add_buffer;
use crate::strategies::handlers::market_handler::market_handlers;
use crate::strategies::handlers::market_handler::market_handlers::{ASK_BOOKS, BACKTEST_CLOSED_ORDER_CACHE, BACKTEST_LEDGERS, BACKTEST_OPEN_ORDER_CACHE, BID_BOOKS, LAST_PRICE, SYMBOL_INFO};
use crate::strategies::strategy_events::StrategyEvent;

pub async fn backtest_matching_engine(
    time: DateTime<Utc>,
) {
    if BACKTEST_OPEN_ORDER_CACHE.len() == 0 {
        return;
    }
    let mut rejected = Vec::new();
    let mut accepted = Vec::new();
    let mut filled = Vec::new();
    let mut partially_filled = Vec::new();
    for order in BACKTEST_OPEN_ORDER_CACHE.iter() {
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
                    let reason = "Time In Force Expired: TimeInForce::IOC".to_string();
                    rejected.push((order.id.clone(), reason));
                }
            }
            TimeInForce::FOK => {
                if time > order.time_created_utc() {
                    let reason = "Time In Force Expired: TimeInForce::FOK".to_string();
                    rejected.push((order.id.clone(), reason));
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
                let market_price = match get_market_price(order.side, &order.symbol_name).await {
                    Ok(price) => price,
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
                let (market_price, volume_filled) = match get_market_limit_fill_price_estimate(order.side, &order.symbol_name, order.quantity_open, order.brokerage, order.limit_price.unwrap()).await {
                    Ok(price_volume) => price_volume,
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
                let market_price = match get_market_fill_price_estimate(order.side, &order.symbol_name, order.quantity_filled, order.brokerage).await {
                    Ok(price) => price,
                    Err(_) => continue
                };
                filled.push((order.id.clone(), market_price));
            },
            // Handle OrderType::StopMarket separately
            OrderType::StopMarket => {
                let market_price = match get_market_price(order.side, &order.symbol_name).await {
                    Ok(price) => price,
                    Err(_) => continue,
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

                let market_fill_price = match get_market_fill_price_estimate(order.side, &order.symbol_name, order.quantity_filled, order.brokerage).await {
                    Ok(price) => price,
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
                let market_price = match get_market_price(order.side, &order.symbol_name).await {
                    Ok(price) => price,
                    Err(_) => continue,
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

                let market_fill_price = match get_market_fill_price_estimate(order.side, &order.symbol_name, order.quantity_filled, order.brokerage).await {
                    Ok(price) => price,
                    Err(_) => continue,
                };

                if is_fill_triggered {
                    filled.push((order.id.clone(), market_fill_price));
                } else if order.state == OrderState::Created {
                    accepted.push((order.id.clone(), time));
                }
            }
            OrderType::StopLimit => {
                let market_price = match get_market_price(order.side, &order.symbol_name).await {
                    Ok(price) => price,
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
                let (market_price, volume_filled) = match get_market_limit_fill_price_estimate(order.side, &order.symbol_name, order.quantity_open, order.brokerage, order.limit_price.unwrap()).await {
                    Ok(price_volume) => price_volume,
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
                let market_price = match get_market_fill_price_estimate(order.side, &order.symbol_name, order.quantity_filled, order.brokerage).await {
                    Ok(price) => price,
                    Err(_) => continue
                };
                if let Some(broker_map) = BACKTEST_LEDGERS.get(&order.brokerage) {
                    if let Some(mut account_map) = broker_map.get_mut(&order.account_id) {
                        if account_map.value().is_short(&order.symbol_name) {
                            account_map.value_mut().exit_position(&order.symbol_name, time, market_price, String::from("Reverse Position")).await;
                        }
                    }
                }
                filled.push((order.id.clone(), market_price));
            }
            OrderType::EnterShort => {
                let market_price = match get_market_fill_price_estimate(order.side, &order.symbol_name, order.quantity_filled, order.brokerage).await {
                    Ok(price) => price,
                    Err(_) => continue
                };
                if let Some(broker_map) = BACKTEST_LEDGERS.get(&order.brokerage) {
                    if let Some(mut account_map) = broker_map.get_mut(&order.account_id) {
                        if account_map.value().is_long(&order.symbol_name) {
                            account_map.value_mut().exit_position(&order.symbol_name, time, market_price, String::from("Reverse Position")).await;
                        }
                    }
                }
                filled.push((order.id.clone(), market_price));
            }
            OrderType::ExitLong => {
                if market_handlers::is_long_paper(&order.brokerage, &order.account_id, &order.symbol_name) {
                    let market_price = match get_market_fill_price_estimate(order.side, &order.symbol_name, order.quantity_filled, order.brokerage).await {
                        Ok(price) => price,
                        Err(_) => continue
                    };
                    filled.push((order.id.clone(), market_price));
                } else {
                    let reason = "No long position to exit".to_string();
                    rejected.push((order.id.clone(), reason));
                }
            }
            OrderType::ExitShort => {
                if market_handlers::is_short_paper(&order.brokerage, &order.account_id, &order.symbol_name) {
                    let market_price = match get_market_fill_price_estimate(order.side, &order.symbol_name, order.quantity_filled, order.brokerage).await {
                        Ok(price) => price,
                        Err(_) => continue
                    };
                    filled.push((order.id.clone(), market_price));
                } else {
                    let reason = "No short position to exit".to_string();
                    rejected.push((order.id.clone(), reason));
                }
            }
        }
    }

    for (order_id, reason) in rejected {
        reject_order(reason, &order_id, time).await
    }
    for (order_id , time)in accepted {
        accept_order(&order_id, time).await;
    }
    for (order_id, price) in filled {
        fill_order(&order_id, time, price).await;
    }
    for (order_id, price, volume) in partially_filled {
        partially_fill_order(&order_id, time, price, volume).await;
    }
}

async fn fill_order(
    order_id: &OrderId,
    time: DateTime<Utc>,
    market_price: Price,
) {
    if let Some((_, mut order)) = BACKTEST_OPEN_ORDER_CACHE.remove(order_id) {
        BACKTEST_CLOSED_ORDER_CACHE.insert(order.id.clone(), order.clone());
        if let Some(broker_map) = BACKTEST_LEDGERS.get(&order.brokerage) {
            if let Some(mut account_map) = broker_map.get_mut(&order.account_id) {
                match account_map.value_mut().update_or_create_position(&order.symbol_name, order_id.clone(), order.quantity_open, order.side.clone(), time, market_price, order.tag.clone()).await {
                    Ok(events) => {
                        order.state = OrderState::Filled;
                        order.average_fill_price = Some(market_price);
                        order.quantity_filled = order.quantity_open;
                        order.quantity_open = dec!(0.0);
                        order.time_filled_utc = Some(time.to_string());
                        for event in events {
                            add_buffer(time, StrategyEvent::PositionEvents(event)).await;
                        }
                        add_buffer(time, StrategyEvent::OrderEvents(OrderUpdateEvent::OrderFilled { order_id: order.id.clone(), brokerage: order.brokerage, account_id: order.account_id.clone(), tag: order.tag.clone(),time: time.to_string()})).await;
                    }
                    Err(e) => {
                        match &e {
                            OrderUpdateEvent::OrderRejected { reason, .. } => {
                                order.state = OrderState::Rejected(reason.clone());
                                let event = StrategyEvent::OrderEvents(e);
                                add_buffer(time, event).await;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

async fn partially_fill_order(
    order_id: &OrderId,
    time: DateTime<Utc>,
    fill_price: Price,
    fill_volume: Volume,
) {
    if let Some(mut order) = BACKTEST_OPEN_ORDER_CACHE.get_mut(order_id) {
        if let Some(broker_map) = BACKTEST_LEDGERS.get(&order.brokerage) {
            if let Some(mut account_map) = broker_map.get_mut(&order.account_id) {
                match account_map.value_mut().update_or_create_position(&order.symbol_name, order_id.clone(), fill_volume, order.side.clone(), time, fill_price, order.tag.clone()).await {
                    Ok(events) => {
                        order.time_filled_utc = Some(time.to_string());
                        order.state = OrderState::PartiallyFilled;
                        order.average_fill_price = Some(fill_price);
                        order.quantity_filled = fill_volume;
                        order.quantity_open -= fill_volume;
                        order.time_filled_utc = Some(time.to_string());
                        for event in events {
                            add_buffer(time, StrategyEvent::PositionEvents(event)).await;
                        }
                        add_buffer(time, StrategyEvent::OrderEvents(OrderUpdateEvent::OrderPartiallyFilled { order_id: order.id.clone(), brokerage: order.brokerage, account_id: order.account_id.clone(), tag: order.tag.clone(), time: time.to_string()})).await;
                    }
                    Err(e) => {
                        match &e {
                            OrderUpdateEvent::OrderRejected { reason, .. } => {
                                order.state = OrderState::Rejected(reason.clone());
                                let event = StrategyEvent::OrderEvents(e);
                                add_buffer(time, event).await;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

async fn reject_order(
    reason: String,
    order_id: &OrderId,
    time: DateTime<Utc>,
) {
    if let Some((_, mut order)) = BACKTEST_OPEN_ORDER_CACHE.remove(order_id) {
        order.state = OrderState::Rejected(reason.clone());
        order.time_created_utc = time.to_string();

        add_buffer(
            time,
            StrategyEvent::OrderEvents(OrderUpdateEvent::OrderRejected {
                order_id: order.id.clone(),
                brokerage: order.brokerage,
                account_id: order.account_id.clone(),
                reason,
                tag: order.tag.clone(),
                time: time.to_string()
            }),
        ).await;
        BACKTEST_CLOSED_ORDER_CACHE.insert(order.id.clone(), order.clone());
    }
}

async fn accept_order(
    order_id: &OrderId,
    time: DateTime<Utc>,
) {
    if let Some(mut order) = BACKTEST_OPEN_ORDER_CACHE.get_mut(order_id) {
        order.state = OrderState::Accepted;
        order.time_created_utc = time.to_string();

        add_buffer(
            time,
            StrategyEvent::OrderEvents(OrderUpdateEvent::OrderAccepted {
                order_id: order.id.clone(),
                brokerage: order.brokerage.clone(),
                account_id: order.account_id.clone(),
                tag: order.tag.clone(),
                time: time.to_string()
            }),
        ).await;
    }
}

pub(crate) async fn get_market_fill_price_estimate (
    order_side: OrderSide,
    symbol_name: &SymbolName,
    volume: Volume,
    brokerage: Brokerage
) -> Result<Price, FundForgeError> {
    let order_book = match order_side {
        OrderSide::Buy => ASK_BOOKS.get(symbol_name),
        OrderSide::Sell => BID_BOOKS.get(symbol_name)
    };

    let tick_size = match SYMBOL_INFO.get(symbol_name) {
        None => {
            let info = brokerage.symbol_info(symbol_name.clone()).await.unwrap();
            let tick_size = info.tick_size.clone();
            SYMBOL_INFO.insert(symbol_name.clone(), info);
            tick_size
        }
        Some(info) => info.value().tick_size
    };

    if let Some(book_price_volume_map) = order_book {
        let mut total_price_volume = dec!(0.0);
        let mut total_volume_filled = dec!(0.0);
        let mut remaining_volume = volume;

        for level in 0.. {
            if let Some(bool_level) = book_price_volume_map.value().get(&level) {
                if bool_level.volume == dec!(0.0) && total_volume_filled == dec!(0.0) {
                    return Ok(bool_level.price.clone())
                }
                if bool_level.volume == dec!(0.0) {
                    continue;
                }
                let volume_to_use = remaining_volume.min(bool_level.volume);
                total_price_volume += bool_level.price * volume_to_use;
                total_volume_filled += volume_to_use;
                remaining_volume -= volume_to_use;

                if remaining_volume == dec!(0.0) {
                    // We've filled the entire requested volume
                    let fill_price = round_to_tick_size(total_price_volume / total_volume_filled, tick_size);
                    return Ok(fill_price);
                }
            } else {
                // No more levels in the order book
                break;
            }
        }

        if total_volume_filled > dec!(0.0) {
            // We filled some volume, but not all. Return the average price for what we could fill.
            let fill_price = round_to_tick_size(total_price_volume / total_volume_filled, tick_size);
            return Ok(fill_price);
        }
    }

    // If we couldn't get a price from the order book, fall back to the last price
    if let Some(last_price) = LAST_PRICE.get(symbol_name) {
        return Ok(last_price.value().clone());
    }

    Err(FundForgeError::ClientSideErrorDebug(String::from("No market price found for symbol")))
}

pub(crate) async fn get_market_limit_fill_price_estimate(
    order_side: OrderSide,
    symbol_name: &SymbolName,
    volume: Volume,
    brokerage: Brokerage,
    limit: Price,
) -> Result<(Price, Volume), FundForgeError> {
    let order_book = match order_side {
        OrderSide::Buy => ASK_BOOKS.get(symbol_name),
        OrderSide::Sell => BID_BOOKS.get(symbol_name)
    };

    let tick_size = match SYMBOL_INFO.get(symbol_name) {
        None => {
            let info = brokerage.symbol_info(symbol_name.clone()).await.unwrap();
            let tick_size = info.tick_size.clone();
            SYMBOL_INFO.insert(symbol_name.clone(), info);
            tick_size
        }
        Some(info) => info.value().tick_size
    };

    if let Some(book_price_volume_map) = order_book {
        let mut total_price_volume = dec!(0.0);
        let mut total_volume_filled = dec!(0.0);
        let mut remaining_volume = volume;
        for level in 0.. {
            if let Some(bool_level) = book_price_volume_map.value().get(&level) {
                if bool_level.volume == dec!(0.0) && total_volume_filled == dec!(0.0) && level == 0 {
                    return Ok((bool_level.price.clone(), volume))
                }
                if bool_level.volume == dec!(0.0) {
                    continue;
                }
                match order_side {
                    OrderSide::Buy => {
                        if bool_level.price > limit {
                            break;
                        }
                    }
                    OrderSide::Sell => {
                        if bool_level.price < limit {
                            break;
                        }
                    }
                }
                let volume_to_use = remaining_volume.min(bool_level.volume);
                total_price_volume += bool_level.price * volume_to_use;
                total_volume_filled += volume_to_use;
                remaining_volume -= volume_to_use;

                if remaining_volume == dec!(0.0) {
                    // We've filled the entire requested volume
                    let fill_price = round_to_tick_size(total_price_volume / total_volume_filled, tick_size);
                    return Ok((fill_price, total_volume_filled));
                }
            } else {
                break;
            }
        }

        if total_volume_filled > dec!(0.0) {
            return Ok((round_to_tick_size(total_price_volume / total_volume_filled, tick_size), total_volume_filled));
        }
    }

    // If we couldn't get a price from the order book, fall back to the last price
    if let Some(last_price) = LAST_PRICE.get(symbol_name) {
        return Ok((last_price.value().clone(), volume));
    }

    Err(FundForgeError::ClientSideErrorDebug(String::from("No market price found for symbol")))
}

pub(crate) async fn get_market_price (
    order_side: OrderSide,
    symbol_name: &SymbolName,
) -> Result<Price, FundForgeError> {
    let order_book = match order_side {
        OrderSide::Buy => ASK_BOOKS.get(symbol_name),
        OrderSide::Sell => BID_BOOKS.get(symbol_name)
    };

    if let Some(symbol_book) = order_book {
        if let Some(book_level) = symbol_book.value().get(&0) {
            return Ok(book_level.price.clone())
        }
    }

    // If we couldn't get a price from the order book, fall back to the last price
    if let Some(last_price) = LAST_PRICE.get(symbol_name) {
        return Ok(last_price.value().clone());
    }

    Err(FundForgeError::ClientSideErrorDebug(String::from("No market price found for symbol")))
}