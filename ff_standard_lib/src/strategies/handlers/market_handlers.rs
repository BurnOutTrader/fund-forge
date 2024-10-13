use std::collections::BTreeMap;
use std::str::FromStr;
use chrono::{DateTime, Utc};
use crate::strategies::ledgers::{AccountId, AccountInfo, Currency, Ledger};
use crate::standardized_types::enums::{OrderSide, PositionSide, StrategyMode};
use crate::strategies::strategy_events::StrategyEvent;
use crate::standardized_types::subscriptions::{Symbol, SymbolName};
use std::sync::Arc;
use chrono_tz::Tz;
use dashmap::DashMap;
use lazy_static::lazy_static;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc;
use crate::standardized_types::broker_enum::Brokerage;
use crate::strategies::client_features::server_connections::{add_buffer, forward_buffer, is_warmup_complete};
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::time_slices::TimeSlice;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::helpers::converters::{time_convert_utc_to_local, time_local_from_utc_str};
use crate::helpers::decimal_calculators::round_to_tick_size;
use crate::standardized_types::base_data::traits::BaseData;
use crate::messages::data_server_messaging::{FundForgeError};
use crate::standardized_types::base_data::quote::Quote;
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::orders::{Order, OrderId, OrderRequest, OrderState, OrderType, OrderUpdateEvent, OrderUpdateType, TimeInForce};
use crate::standardized_types::symbol_info::SymbolInfo;
use crate::strategies::historical_time::get_backtest_time;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct BookLevel {
    level: u16,
    pub price: Price,
    pub volume: Volume
}

impl BookLevel {
    pub fn new(level: u16, price: Price, volume: Volume) -> Self {
        BookLevel {
            level,
            price,
            volume
        }
    }

    pub fn into_quote(symbol: Symbol, best_offer: Self, best_bid: Self, time: DateTime<Utc>) -> Quote {
        Quote {
            symbol,
            ask: best_offer.price,
            bid: best_bid.price,
            ask_volume: best_offer.volume,
            bid_volume: best_bid.volume,
            time: time.to_string(),
        }
    }
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum MarketMessageEnum {
    RegisterSymbol(Symbol),
    BaseDataUpdate(BaseDataEnum),
    TimeSliceUpdate(TimeSlice),
    OrderRequest(OrderRequest),
    OrderBookSnapShot{symbol: Symbol, bid_book: BTreeMap<u16, BookLevel>, ask_book: BTreeMap<u16, BookLevel>},
    DeregisterSymbol(Symbol),
}

lazy_static!(
    pub(crate) static ref BID_BOOKS: Arc<DashMap<SymbolName, BTreeMap<u16, BookLevel>>>= Arc::new(DashMap::new());
    pub(crate) static ref ASK_BOOKS: Arc<DashMap<SymbolName, BTreeMap<u16, BookLevel>>> = Arc::new(DashMap::new());
    static ref HAS_QUOTES: DashMap<SymbolName, bool> = DashMap::new();
    pub(crate) static ref LAST_PRICE: Arc<DashMap<SymbolName, Price>> = Arc::new(DashMap::new());
    pub(crate) static ref SYMBOL_INFO: Arc<DashMap<SymbolName, SymbolInfo>> = Arc::new(DashMap::new());
    //static ref LAST_PRICE_MOMENTUM: Arc<DashMap<Symbol, BTreeMap<u8, > = Arc::new(DashMap::new()); we could use this to record last tick was up or down for x periods
    //ToDO implement a 4th strategy mode variant to trade live and paper in parallel

    //LIVE STATICS
    pub(crate) static ref LIVE_ORDER_CACHE: Arc<DashMap<OrderId, Order>> = Arc::new(DashMap::new());
    pub(crate) static ref LIVE_CLOSED_ORDER_CACHE: Arc<DashMap<OrderId, Order>> = Arc::new(DashMap::new());
    pub(crate) static ref LIVE_LEDGERS: Arc<DashMap<Brokerage, DashMap<AccountId, Ledger>>> = Arc::new(DashMap::new());

    //BACKTEST STATICS
    pub(crate) static ref BACKTEST_OPEN_ORDER_CACHE: Arc<DashMap<OrderId, Order>> = Arc::new(DashMap::new());
    pub(crate) static ref BACKTEST_CLOSED_ORDER_CACHE: Arc<DashMap<OrderId, Order>> = Arc::new(DashMap::new());
    pub(crate) static ref BACKTEST_LEDGERS: Arc<DashMap<Brokerage, DashMap<AccountId, Ledger>>> = Arc::new(DashMap::new());
);

fn historical_time_slice_ledger_updates(time_slice: TimeSlice, time: DateTime<Utc>) {
    for broker_map in BACKTEST_LEDGERS.iter() {
        for mut account_map in broker_map.iter_mut() {
            account_map.value_mut().on_historical_timeslice_update(time_slice.clone(), time);
        }
    }
}

fn historical_base_data_updates(base_data_enum: BaseDataEnum, time: DateTime<Utc>) {
    for broker_map in BACKTEST_LEDGERS.iter() {
        if let Some(mut account_map) = broker_map.get_mut(&base_data_enum.symbol().name) {
            account_map.value_mut().on_base_data_update(base_data_enum.clone(), time);
        }
    }
}

pub(crate) fn add_account(mode: StrategyMode, account_info: AccountInfo) {
    let map = BACKTEST_LEDGERS.entry(account_info.brokerage).or_insert(DashMap::new());
    map.insert(account_info.account_id.clone() ,Ledger::new(account_info, mode));
}

pub(crate) async fn market_handler(mode: StrategyMode, starting_balances: Decimal, account_currency: Currency) -> Sender<MarketMessageEnum> {
    let (sender, receiver) = mpsc::channel(1000);
    let mut receiver = receiver;
    tokio::task::spawn(async move{
        while let Some(message) = receiver.recv().await {
            let time = match mode {
                StrategyMode::Backtest => get_backtest_time(),
                StrategyMode::Live | StrategyMode::LivePaperTrading => Utc::now()
            };
            match message {
                MarketMessageEnum::RegisterSymbol(symbol) => {
                    BID_BOOKS.insert(symbol.name.clone(), BTreeMap::new());
                    ASK_BOOKS.insert(symbol.name.clone(), BTreeMap::new());
                }
                MarketMessageEnum::BaseDataUpdate(base_data ) => {
                    update_base_data(base_data.clone());
                    if mode == StrategyMode::LivePaperTrading || mode == StrategyMode::Backtest {
                        backtest_matching_engine(time).await;
                    }
                    historical_base_data_updates(base_data, time);
                }
                MarketMessageEnum::TimeSliceUpdate(time_slice) => {
                    for base_data in time_slice.iter() {
                        update_base_data(base_data.clone());
                    }
                    historical_time_slice_ledger_updates(time_slice.clone(), time);
                    if mode == StrategyMode::LivePaperTrading || mode == StrategyMode::Backtest {
                        backtest_matching_engine(time).await;
                    }
                }
                MarketMessageEnum::OrderBookSnapShot{symbol , bid_book, ask_book } => {
                    if !BID_BOOKS.contains_key(&symbol.name){
                        BID_BOOKS.insert(symbol.name.clone(), BTreeMap::new());
                    }
                    BID_BOOKS.insert(symbol.name.clone(), bid_book);
                    ASK_BOOKS.insert(symbol.name.clone(), ask_book);
                    if mode == StrategyMode::LivePaperTrading || mode == StrategyMode::Backtest {
                        backtest_matching_engine(time).await;
                    }
                }
                MarketMessageEnum::OrderRequest(order_request) => {
                    if !is_warmup_complete() {
                        continue;
                    }
                    match mode {
                        StrategyMode::Live => {
                            panic!("Live orders do not get sent via market handler");
                        }
                        StrategyMode::LivePaperTrading | StrategyMode::Backtest => {
                            simulated_order_matching(mode, starting_balances, account_currency, order_request, time).await;
                        }
                    }
                }
                MarketMessageEnum::DeregisterSymbol(symbol) => {
                    BID_BOOKS.remove(&symbol.name);
                    ASK_BOOKS.remove(&symbol.name);
                    LAST_PRICE.remove(&symbol.name);
                }
            }
        };
    });
    sender
}

fn update_base_data(base_data_enum: BaseDataEnum) {
    match base_data_enum {
        BaseDataEnum::Candle(candle) => {
            LAST_PRICE.insert(candle.symbol.name, candle.close);
        }
        BaseDataEnum::QuoteBar(quotebar) => {
            if HAS_QUOTES.contains_key(&quotebar.symbol.name) {
                return;
            }
            if !BID_BOOKS.contains_key(&quotebar.symbol.name) {
                BID_BOOKS.insert(quotebar.symbol.name.clone(), BTreeMap::new());
            }
            if let Some(mut bid_book) = BID_BOOKS.get_mut(&quotebar.symbol.name) {
                bid_book.value_mut().insert(0, BookLevel::new(0, quotebar.bid_close, dec!(0.0)));
            }
            if !ASK_BOOKS.contains_key(&quotebar.symbol.name) {
                ASK_BOOKS.insert(quotebar.symbol.name.clone(), BTreeMap::new());
            }
            if let Some(mut ask_book) = ASK_BOOKS.get_mut(&quotebar.symbol.name) {
                ask_book.value_mut().insert(0,  BookLevel::new(0, quotebar.ask_close, dec!(0.0)));
            }
        }
        BaseDataEnum::Tick(tick) => {
            LAST_PRICE.insert(tick.symbol.name, tick.price);
        }
        BaseDataEnum::Quote(quote) => {
            if !HAS_QUOTES.contains_key(&quote.symbol.name) {
                HAS_QUOTES.insert(quote.symbol.name.clone(), true);
                BID_BOOKS.insert(quote.symbol.name.clone(), BTreeMap::new());
                ASK_BOOKS.insert(quote.symbol.name.clone(), BTreeMap::new());
            }
            if let Some(mut bid_book) = BID_BOOKS.get_mut(&quote.symbol.name) {
                bid_book.value_mut().insert(0, BookLevel::new(0, quote.bid, quote.bid_volume));
            }
            if let Some(mut ask_book) = ASK_BOOKS.get_mut(&quote.symbol.name) {
                ask_book.value_mut().insert(0,  BookLevel::new(0, quote.ask, quote.ask_volume));
            }
        }
        _ => panic!("Incorrect data type in Market Updates: {}", base_data_enum.base_data_type())
    }
}

// need to rethink this.. do we have ledgers or just static properties linked to account id's and positions and orders linked to account ids
pub async fn simulated_order_matching(
    mode: StrategyMode,
    starting_balance: Decimal,
    currency: Currency,
    order_request: OrderRequest,
    time: DateTime<Utc>
) {
    match order_request {
        OrderRequest::Create { order, .. } => {
            let time = DateTime::from_str(&order.time_created_utc).unwrap();
            if !BACKTEST_LEDGERS.contains_key(&order.brokerage) {
                let broker_map = DashMap::new();
                BACKTEST_LEDGERS.insert(order.brokerage, broker_map);
            }
            if !BACKTEST_LEDGERS.get(&order.brokerage).unwrap().contains_key(&order.account_id) {
                let ledger = match order.brokerage.paper_account_init(mode, starting_balance, currency, order.account_id.clone()).await {
                    Ok(ledger) => ledger,
                    Err(e) => {
                        panic!("Order Matching Engine: Error Initializing Account: {}", e);
                    }
                };
                BACKTEST_LEDGERS.get(&order.brokerage).unwrap().insert(order.account_id.clone(), ledger);
            }
            BACKTEST_OPEN_ORDER_CACHE.insert(order.id.clone(), order);
            backtest_matching_engine(time.clone()).await;
        }
        OrderRequest::Cancel{brokerage, order_id, account_id } => {
            let existing_order = BACKTEST_OPEN_ORDER_CACHE.remove(&order_id);
            if let Some((existing_order_id, order)) = existing_order {
                let cancel_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderCancelled { brokerage, account_id: order.account_id.clone(), order_id: existing_order_id, tag: order.tag.clone(), time: time.to_string()});
                add_buffer(time, cancel_event).await;
                BACKTEST_CLOSED_ORDER_CACHE.insert(order_id, order);
            } else {
                let fail_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderUpdateRejected { brokerage, account_id, order_id, reason: String::from("No pending order found"), time: time.to_string()});
                add_buffer(time, fail_event).await;
            }
        }
        OrderRequest::Update{ brokerage, order_id, account_id, update } => {
            if let Some((order_id, mut order)) = BACKTEST_OPEN_ORDER_CACHE.remove(&order_id) {
                match update {
                    OrderUpdateType::LimitPrice(price) => {
                        if let Some(ref mut limit_price) = order.limit_price {
                            *limit_price = price;
                        }
                    }
                    OrderUpdateType::TriggerPrice(price) => {
                        if let Some(ref mut trigger_price) = order.trigger_price {
                            *trigger_price = price;
                        }
                    }
                    OrderUpdateType::TimeInForce(tif) => {
                        order.time_in_force = tif;
                    }
                    OrderUpdateType::Quantity(quantity) => {
                        order.quantity_open = quantity;
                    }
                    OrderUpdateType::Tag(tag) => {
                        order.tag = tag;
                    }
                }
                let update_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderUpdated { brokerage, account_id: order.account_id.clone(), order_id: order.id.clone(), order: order.clone(), tag: order.tag.clone(), time: time.to_string()});
                BACKTEST_OPEN_ORDER_CACHE.insert(order_id, order);
                add_buffer(time, update_event).await;
            } else {
                let fail_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderUpdateRejected { brokerage, account_id, order_id, reason: String::from("No pending order found"), time: time.to_string() });
                add_buffer(time, fail_event).await;
            }
        }
        OrderRequest::CancelAll { brokerage, account_id, symbol_name } => {
            let mut remove = vec![];
            for order in BACKTEST_OPEN_ORDER_CACHE.iter() {
                if order.brokerage == brokerage && order.account_id == account_id && order.symbol_name == symbol_name {
                    remove.push(order.id.clone());
                }
            }
            for order_id in remove {
                let order = BACKTEST_OPEN_ORDER_CACHE.remove(&order_id);
                if let Some((order_id, order)) = order {
                    let cancel_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderCancelled { brokerage: order.brokerage.clone(), account_id: order.account_id.clone(), order_id: order.id.clone(), tag: order.tag.clone(), time: time.to_string()});
                    add_buffer(time.clone(), cancel_event).await;
                    BACKTEST_CLOSED_ORDER_CACHE.insert(order_id, order);
                }
            }
        }
        OrderRequest::FlattenAllFor { brokerage, account_id } => {
             flatten_all_paper_for(&brokerage, &account_id, time).await;
        }
    }
}

pub async fn backtest_matching_engine(
    time: DateTime<Utc>,
) {
    if BACKTEST_OPEN_ORDER_CACHE.len() == 0 {
        return;
    }
    //todo need a better way to simulate stop limits, use custom market price fn
    // need to handle partial fills for data sets will volume. at-least partial fill stop limits and limits.

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
                            account_map.value_mut().exit_position_paper(&order.symbol_name, time, market_price, String::from("Reverse Position")).await;
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
                            account_map.value_mut().exit_position_paper(&order.symbol_name, time, market_price, String::from("Reverse Position")).await;
                        }
                    }
                }
                filled.push((order.id.clone(), market_price));
            }
            OrderType::ExitLong => {
                if is_long_paper(&order.brokerage, &order.account_id, &order.symbol_name) {
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
                if is_short_paper(&order.brokerage, &order.account_id, &order.symbol_name) {
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
                match account_map.value_mut().update_or_create_paper_position(&order.symbol_name, order_id.clone(), order.quantity_open, order.side.clone(), time, market_price, order.tag.clone()).await {
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
                match account_map.value_mut().update_or_create_paper_position(&order.symbol_name, order_id.clone(), fill_volume, order.side.clone(), time, fill_price, order.tag.clone()).await {
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

pub async fn live_order_update(order_upate_event: OrderUpdateEvent, is_buffered: bool) {
    match order_upate_event {
        OrderUpdateEvent::OrderAccepted { brokerage, account_id, order_id ,tag, time} => {
            if let Some(mut order) = LIVE_ORDER_CACHE.get_mut(&order_id) {
                order.value_mut().state = OrderState::Accepted;
                let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderAccepted {order_id, account_id: account_id.clone(), brokerage, tag, time});
                add_buffer(Utc::now(), event).await;
                if !is_buffered {
                    forward_buffer(Utc::now()).await;
                }
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
                if !is_buffered {
                    forward_buffer(Utc::now()).await;
                }
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
                if !is_buffered {
                    forward_buffer(Utc::now()).await;
                }
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
                if !is_buffered {
                    forward_buffer(Utc::now()).await;
                }
                LIVE_CLOSED_ORDER_CACHE.insert(order_id.clone(), order);
            }
        }
        OrderUpdateEvent::OrderRejected { brokerage, account_id, order_id, reason ,tag, time} => {
            if let Some((order_id, mut order)) = LIVE_ORDER_CACHE.remove(&order_id) {
                order.state = OrderState::Rejected(reason.clone());
                let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderRejected {order_id: order_id.clone(), account_id, brokerage, reason , tag, time});
                add_buffer(Utc::now(), event).await;
                if !is_buffered {
                    forward_buffer(Utc::now()).await;
                }
                LIVE_CLOSED_ORDER_CACHE.insert(order_id.clone(), order);
            }
        }
        OrderUpdateEvent::OrderUpdated { brokerage, account_id, order_id, order,tag,time} => {
            LIVE_ORDER_CACHE.insert(order_id.clone(), order.clone());
            let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderUpdated {order_id, account_id, brokerage, order, tag, time});
            add_buffer(Utc::now(), event).await;
            if !is_buffered {
                forward_buffer(Utc::now()).await;
            }
        }
        OrderUpdateEvent::OrderUpdateRejected { brokerage, account_id, order_id, reason,time } => {
            if let Some((order_id, order)) = LIVE_ORDER_CACHE.remove(&order_id) {
                LIVE_CLOSED_ORDER_CACHE.insert(order_id.clone(), order);
            }
            let event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderUpdateRejected {order_id, account_id, brokerage, reason, time});
            add_buffer(Utc::now(), event).await;
            if !is_buffered {
                forward_buffer(Utc::now()).await;
            }
        }
    }
}


pub(crate) fn process_ledgers() -> Vec<String> {
    let mut return_strings = vec![];
    for broker_map in BACKTEST_LEDGERS.iter() {
        for account_map in broker_map.iter_mut() {
            return_strings.push(account_map.value().print());
        }
    }
    return_strings
}

pub(crate) fn export_trades(directory: &str) {
    for broker_map in BACKTEST_LEDGERS.iter() {
        for account_map in broker_map.iter_mut() {
            account_map.export_positions_to_csv(directory);
        }
    }
}

pub(crate) fn print_ledger(brokerage: &Brokerage, account_id: &AccountId) -> Option<String>  {
    if let Some(broker_map) = BACKTEST_LEDGERS.get(brokerage) {
        if let Some(account_map) = broker_map.get(account_id) {
            return Some(account_map.value().print())
        }
    }
    None
}

pub(crate) fn in_profit_paper(symbol_name: &SymbolName, brokerage: &Brokerage, account_id: &AccountId) -> bool {
    if let Some(broker_map) = BACKTEST_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.get(account_id) {
            return account_map.value().in_profit(symbol_name)
        }
    }
    false
}

pub(crate) fn in_profit_live(symbol_name: &SymbolName, brokerage: &Brokerage, account_id: &AccountId) -> bool {
    if let Some(broker_map) = LIVE_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.get(account_id) {
            return account_map.value().in_profit(symbol_name)
        }
    }
    false
}

pub(crate) fn in_drawdown_paper(symbol_name: &SymbolName, brokerage: &Brokerage, account_id: &AccountId) -> bool {
    if let Some(broker_map) = BACKTEST_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.get(account_id) {
            return account_map.value().in_drawdown(symbol_name)
        }
    }
    false
}

pub(crate) fn in_drawdown_live(symbol_name: &SymbolName, brokerage: &Brokerage, account_id: &AccountId) -> bool {
    if let Some(broker_map) = LIVE_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.get(account_id) {
            return account_map.value().in_drawdown(symbol_name)
        }
    }
    false
}

pub(crate) fn pnl_paper(symbol_name: &SymbolName, brokerage: &Brokerage, account_id: &AccountId) -> Decimal {
    if let Some(broker_map) = BACKTEST_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.get(account_id) {
            return account_map.value().pnl(symbol_name)
        }
    }
    dec!(0.0)
}

pub(crate) fn booked_pnl_paper(symbol_name: &SymbolName, brokerage: &Brokerage, account_id: &AccountId) -> Decimal {
    if let Some(broker_map) = BACKTEST_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.get(account_id) {
            return account_map.value().booked_pnl(symbol_name)
        }
    }
    dec!(0.0)
}

pub(crate) fn pnl_live(symbol_name: &SymbolName, brokerage: &Brokerage, account_id: &AccountId) -> Decimal {
    if let Some(broker_map) = LIVE_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.get(account_id) {
            return account_map.value().pnl(symbol_name)
        }
    }
    dec!(0.0)
}

pub(crate) fn booked_pnl_live(symbol_name: &SymbolName, brokerage: &Brokerage, account_id: &AccountId) -> Decimal {
    if let Some(broker_map) = LIVE_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.get(account_id) {
            return account_map.value().booked_pnl(symbol_name)
        }
    }
    dec!(0.0)
}

pub(crate) fn position_size_live(symbol_name: &SymbolName, brokerage: &Brokerage, account_id: &AccountId) -> Decimal {
    if let Some(broker_map) = LIVE_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.get(account_id) {
            return account_map.value().position_size(symbol_name)
        }
    }
    dec!(0.0)
}

pub(crate) fn position_size_paper(symbol_name: &SymbolName, brokerage: &Brokerage, account_id: &AccountId) -> Decimal {
    if let Some(broker_map) = BACKTEST_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.get(account_id) {
            return account_map.value().position_size(symbol_name)
        }
    }
    dec!(0.0)
}


pub(crate) fn is_long_live(brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
    if let Some(broker_map) = LIVE_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.get(account_id) {
            return account_map.value().is_long(symbol_name)
        }
    }
    false
}

pub(crate) fn is_short_live(brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
    if let Some(broker_map) = LIVE_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.get(account_id) {
            return account_map.value().is_short(symbol_name)
        }
    }
    false
}

pub(crate) fn is_flat_live(brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
    if let Some(broker_map) = LIVE_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.get(account_id) {
            return account_map.value().is_flat(symbol_name)
        }
    }
    true
}

pub(crate) fn is_long_paper(brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
    if let Some(broker_map) = BACKTEST_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.get(account_id) {
            return account_map.value().is_long(symbol_name)
        }
    }
    false
}

pub(crate) fn is_short_paper(brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
    if let Some(broker_map) = BACKTEST_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.get(account_id) {
            return account_map.value().is_short(symbol_name)
        }
    }
    false
}

pub(crate) fn is_flat_paper(brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
    if let Some(broker_map) = BACKTEST_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.get(account_id) {
            return account_map.value().is_flat(symbol_name)
        }
    }
    true
}

/*async fn flatten_all_paper(time: DateTime<Utc>) {
    for broker_map in BACKTEST_LEDGERS.iter() {
        for account in broker_map.iter() {
            flatten_all_paper_for(broker_map.key(), account.key(), time).await;
        }
    }
}*/

async fn flatten_all_paper_for(brokerage: &Brokerage, account_id: &AccountId, time: DateTime<Utc>) {
    if let Some(broker_map) = BACKTEST_LEDGERS.get(&brokerage) {
        if let Some(mut account_map) = broker_map.get_mut(account_id) {
            for (symbol_name, position) in account_map.positions.clone() {
                let side = match position.side {
                    PositionSide::Long => OrderSide::Sell,
                    PositionSide::Short => OrderSide::Buy
                };
                let market_price = get_market_fill_price_estimate(side, &symbol_name, position.quantity_open, position.brokerage).await.unwrap();
                if let Some(event) = account_map.exit_position_paper(&symbol_name, time, market_price, String::from("Flatten Account")).await {
                    add_buffer(time, StrategyEvent::PositionEvents(event)).await;
                }
            }
        }
    }
}




