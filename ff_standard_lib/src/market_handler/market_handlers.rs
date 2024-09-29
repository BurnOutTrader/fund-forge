use std::collections::{BTreeMap};
use std::str::FromStr;
use chrono::{DateTime, Utc};
use crate::standardized_types::accounts::ledgers::{AccountId, Currency};
use crate::standardized_types::enums::{OrderSide, PositionSide, StrategyMode};
use crate::standardized_types::orders::orders::{Order, OrderUpdateType, OrderRequest, OrderUpdateEvent, OrderId, OrderState, ProtectiveOrder, OrderType};
use crate::standardized_types::strategy_events::{StrategyEvent};
use crate::standardized_types::subscriptions::{Symbol, SymbolName};
use crate::standardized_types::{Price, Volume};
use std::sync::Arc;
use ahash::AHashMap;
use dashmap::DashMap;
use lazy_static::lazy_static;
use tokio::sync::mpsc::{Sender};
use tokio::sync::{mpsc};
use crate::apis::brokerage::broker_enum::Brokerage;
use crate::server_connections::{add_buffer, get_backtest_time, is_warmup_complete, send_request, ConnectionType, StrategyRequest};
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::time_slices::TimeSlice;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::helpers::decimal_calculators::round_to_tick_size;
use crate::servers::settings::client_settings::{initialise_settings};
use crate::standardized_types::accounts::position::{Position, PositionId};
use crate::standardized_types::data_server_messaging::{DataServerRequest, FundForgeError};
use crate::standardized_types::symbol_info::SymbolInfo;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct BookLevel {
    level: u8,
    price: Decimal,
    volume: Volume
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum MarketMessageEnum {
    RegisterSymbol(Symbol),
    BaseDataUpdate(BaseDataEnum),
    TimeSliceUpdate(TimeSlice),
    OrderRequest(OrderRequest),
    OrderBookSnapShot{symbol: Symbol, bid_book: Vec<BookLevel>, ask_book: Vec<BookLevel>},
    DeregisterSymbol(Symbol),
    LiveOrderUpdate(OrderUpdateEvent)
}


lazy_static!(
    pub static ref BID_BOOKS: Arc<DashMap<SymbolName, BTreeMap<u8, (Price, Volume)>>>= Arc::new(DashMap::new());
    pub static ref ASK_BOOKS: Arc<DashMap<SymbolName, BTreeMap<u8, (Price, Volume)>>> = Arc::new(DashMap::new());
    pub static ref LAST_PRICE: Arc<DashMap<SymbolName, Price>> = Arc::new(DashMap::new());
    pub static ref SYMBOL_INFO: Arc<DashMap<SymbolName, SymbolInfo>> = Arc::new(DashMap::new());
    //static ref LAST_PRICE_MOMENTUM: Arc<DashMap<Symbol, BTreeMap<u8, > = Arc::new(DashMap::new()); we could use this to record last tick was up or down for x periods
    static ref POSITIONS_COUNTER: Arc<DashMap<Brokerage, DashMap<AccountId, AHashMap<SymbolName, u64>>>> = Arc::new(DashMap::new());
    //ToDO implement a 4th strategy mode variant to trade live and paper in parallel

    //LIVE STATICS
    pub static ref LIVE_CASH_AVAILABLE: Arc<DashMap<Brokerage, DashMap<AccountId, Price>>> = Arc::new(DashMap::new());
    pub static ref LIVE_CASH_USED: Arc<DashMap<Brokerage, DashMap<AccountId, Price>>> = Arc::new(DashMap::new());
    static ref LIVE_CURRENCY: Arc<DashMap<Brokerage, DashMap<AccountId, Currency>>> = Arc::new(DashMap::new());
    pub static ref LIVE_BOOKED_PNL: Arc<DashMap<Brokerage, DashMap<AccountId, Decimal>>> = Arc::new(DashMap::new());
    pub static ref LIVE_OPEN_PNL: Arc<DashMap<Brokerage, DashMap<AccountId, Decimal>>> = Arc::new(DashMap::new());
    pub static ref LIVE_ORDER_CACHE: Arc<DashMap<OrderId, Order>> = Arc::new(DashMap::new());
    pub static ref LIVE_CLOSED_ORDER_CACHE: Arc<DashMap<OrderId, Order>> = Arc::new(DashMap::new());
    pub static ref OPEN_LIVE_POSITIONS: Arc<DashMap<Brokerage, DashMap<AccountId, Position>>> = Arc::new(DashMap::new());
    pub static ref CLOSED_LIVE_POSITIONS: Arc<DashMap<Brokerage, DashMap<AccountId, Vec<Position>>>> = Arc::new(DashMap::new());


    //BACKTEST STATICS
    pub static ref BACKTEST_CASH_AVAILABLE: Arc<DashMap<Brokerage, DashMap<AccountId, Price>>> = Arc::new(DashMap::new());
    pub static ref BACKTEST_CASH_USED: Arc<DashMap<Brokerage, DashMap<AccountId, Price>>> = Arc::new(DashMap::new());
    pub static ref BACKTEST_CURRENCY: Arc<DashMap<Brokerage, DashMap<AccountId, Currency>>> = Arc::new(DashMap::new());
    pub static ref BACKTEST_BOOKED_PNL: Arc<DashMap<Brokerage, DashMap<AccountId, Decimal>>> = Arc::new(DashMap::new());
    static ref BACKTEST_OPEN_PNL: Arc<DashMap<Brokerage, DashMap<AccountId, Decimal>>> = Arc::new(DashMap::new());
    pub static ref BACKTEST_OPEN_ORDER_CACHE: Arc<DashMap<OrderId, Order>> = Arc::new(DashMap::new());
    pub static ref BACKTEST_CLOSED_ORDER_CACHE: Arc<DashMap<OrderId, Order>> = Arc::new(DashMap::new());
    pub static ref OPEN_BACKTEST_POSITIONS: Arc<DashMap<Brokerage, DashMap<AccountId, Position>>> = Arc::new(DashMap::new());
    pub static ref CLOSED_BACKTEST_POSITIONS: Arc<DashMap<Brokerage, DashMap<AccountId, Vec<Position>>>> = Arc::new(DashMap::new());
);

pub async fn market_handler(mode: StrategyMode) -> Sender<MarketMessageEnum> {
    let (sender, receiver) = mpsc::channel(1000);
    let mut receiver = receiver;
    tokio::task::spawn(async move{
        let settings_map = Arc::new(initialise_settings().unwrap());
        while let Some(message) = receiver.recv().await {
            match message {
                MarketMessageEnum::RegisterSymbol(symbol) => {
                    BID_BOOKS.insert(symbol.name.clone(), BTreeMap::new());
                    ASK_BOOKS.insert(symbol.name.clone(), BTreeMap::new());
                }
                MarketMessageEnum::BaseDataUpdate(base_data ) => {
                    update_base_data(mode, base_data);
                }
                MarketMessageEnum::OrderBookSnapShot{symbol , bid_book, ask_book } => {
                    if !BID_BOOKS.contains_key(&symbol.name){
                        BID_BOOKS.insert(symbol.name.clone(), BTreeMap::new());
                    }
                    if let Some(mut order_book) = BID_BOOKS.get_mut(&symbol.name) {
                        for book_level in bid_book {
                            order_book.value_mut().insert(book_level.level, (book_level.price, book_level.volume));
                        }
                    }
                    if !ASK_BOOKS.contains_key(&symbol.name){
                        ASK_BOOKS.insert(symbol.name.clone(), BTreeMap::new());
                    }
                    if let Some(mut order_book) = ASK_BOOKS.get_mut(&symbol.name) {
                        for book_level in ask_book {
                            order_book.value_mut().insert(book_level.level, (book_level.price, book_level.volume));
                        }
                    }
                }
                MarketMessageEnum::TimeSliceUpdate(time_slice) => {
                    for base_data in time_slice.iter() {
                        update_base_data(mode, base_data.clone());
                    }
                }
                MarketMessageEnum::OrderRequest(order_request) => {
                    if !is_warmup_complete() {
                        continue;
                    }
                    match mode {
                        StrategyMode::Live => {
                            let connection_type = ConnectionType::Broker(order_request.brokerage());
                            let connection_type = match settings_map.contains_key(&connection_type) {
                                true => connection_type,
                                false => ConnectionType::Default
                            };
                            let datat_server_request = DataServerRequest::OrderRequest {
                                request: order_request.clone()
                            };
                            send_request(StrategyRequest::OneWay(connection_type, datat_server_request)).await;
                            match order_request {
                                OrderRequest::Create { brokerage, order } => {
                                    LIVE_ORDER_CACHE.insert(order.id.clone(), order);
                                }
                                _ => {}
                            }
                        }
                        StrategyMode::LivePaperTrading | StrategyMode::Backtest => {
                            simulated_order_matching(order_request).await;
                        }
                    }
                }
                MarketMessageEnum::DeregisterSymbol(symbol) => {
                    BID_BOOKS.remove(&symbol.name);
                    ASK_BOOKS.remove(&symbol.name);
                    LAST_PRICE.remove(&symbol.name);
                }
                MarketMessageEnum::LiveOrderUpdate(order_upate_event) => {
                    //todo we update our live positions here
                    match order_upate_event {
                        OrderUpdateEvent::Accepted { .. } => {}
                        OrderUpdateEvent::Filled { .. } => {}
                        OrderUpdateEvent::PartiallyFilled { .. } => {}
                        OrderUpdateEvent::Cancelled { .. } => {}
                        OrderUpdateEvent::Rejected { .. } => {}
                        OrderUpdateEvent::Updated { .. } => {}
                        OrderUpdateEvent::UpdateRejected { .. } => {}
                    }
                }
            }
        };
    });
    sender
}

fn update_base_data(mode: StrategyMode, base_data_enum: BaseDataEnum) {
    match base_data_enum {
        BaseDataEnum::Candle(candle) => {
            LAST_PRICE.insert(candle.symbol.name, candle.close);
        }
        BaseDataEnum::QuoteBar(quotebar) => {
            if mode == StrategyMode::Backtest {
                if let Some(mut bid_book) = BID_BOOKS.get_mut(&quotebar.symbol.name) {
                    bid_book.value_mut().insert(0, (quotebar.bid_close, dec!(0.0)));
                }
                if let Some(mut ask_book) = ASK_BOOKS.get_mut(&quotebar.symbol.name) {
                    ask_book.value_mut().insert(0, (quotebar.ask_close, dec!(0.0)));
                }
            }
        }
        BaseDataEnum::Tick(tick) => {
            LAST_PRICE.insert(tick.symbol.name, tick.price);
        }
        BaseDataEnum::Quote(quote) => {
            if !BID_BOOKS.contains_key(&quote.symbol.name) {
                BID_BOOKS.insert(quote.symbol.name.clone(), BTreeMap::new());
            }
            if let Some(mut bid_book) = BID_BOOKS.get_mut(&quote.symbol.name) {
                bid_book.value_mut().insert(0, (quote.bid, quote.bid_volume));
            }
            if !ASK_BOOKS.contains_key(&quote.symbol.name) {
                ASK_BOOKS.insert(quote.symbol.name.clone(), BTreeMap::new());
            }
            if let Some(mut ask_book) = ASK_BOOKS.get_mut(&quote.symbol.name) {
                ask_book.value_mut().insert(0, (quote.ask, quote.ask_volume));
            }
        }
        _ => panic!("Incorrect data type in Market Updates: {}", base_data_enum.base_data_type())
    }
}

// need to rethink this.. do we have ledgers or just static properties linked to account id's and positions and orders linked to account ids
pub async fn simulated_order_matching(
    order_request: OrderRequest,
) {
    let time = get_backtest_time();
    match order_request {
        OrderRequest::Create { order, .. } => {
            let time = DateTime::from_str(&order.time_created_utc).unwrap();
            BACKTEST_OPEN_ORDER_CACHE.insert(order.id.clone(), order);
            backtest_matching_engine(time.clone()).await;
        }
        OrderRequest::Cancel{brokerage, order_id, account_id } => {
            let existing_order = BACKTEST_OPEN_ORDER_CACHE.remove(&order_id);
            if let Some((existing_order_id, order)) = existing_order {
                let cancel_event = StrategyEvent::OrderEvents(OrderUpdateEvent::Cancelled{ brokerage, account_id: order.account_id.clone(), order_id: existing_order_id});
                add_buffer(time, cancel_event).await;
                BACKTEST_CLOSED_ORDER_CACHE.insert(order_id, order);
            } else {
                let fail_event = StrategyEvent::OrderEvents(OrderUpdateEvent::UpdateRejected { brokerage, account_id, order_id, reason: String::from("No pending order found") });
                add_buffer(time, fail_event).await;
            }
        }
        OrderRequest::Update{ brokerage, order_id, account_id, update } => {
            let mut existing_order = BACKTEST_OPEN_ORDER_CACHE.remove(&order_id);
            if let Some((order_id, mut order)) = existing_order {
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
                        order.quantity_ordered = quantity;
                    }
                    OrderUpdateType::Tag(tag) => {
                        order.tag = tag;
                    }
                }
                let update_event = StrategyEvent::OrderEvents(OrderUpdateEvent::Updated{ brokerage, account_id: order.account_id.clone(), order_id: order.id.clone()});
                BACKTEST_OPEN_ORDER_CACHE.insert(order_id, order);
                add_buffer(time, update_event).await;
            } else {
                let fail_event = StrategyEvent::OrderEvents(OrderUpdateEvent::UpdateRejected { brokerage, account_id, order_id, reason: String::from("No pending order found") });
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
                    let cancel_event = StrategyEvent::OrderEvents(OrderUpdateEvent::Cancelled { brokerage: order.brokerage.clone(), account_id: order.account_id.clone(), order_id: order.id.clone() });
                    add_buffer(time.clone(), cancel_event).await;
                    BACKTEST_CLOSED_ORDER_CACHE.insert(order_id, order);
                }
            }
        }
    }
}

pub async fn backtest_matching_engine(
    time: DateTime<Utc>
) {
    if BACKTEST_OPEN_ORDER_CACHE.len() == 0 {
        return;
    }

    //todo need a better way to simulate stop limits, use custom market price fn
    // need to handle partial fills for data sets will volume. at-least partial fill stop limits and limits.

    let mut rejected = Vec::new();
    let mut accepted = Vec::new();
    let mut filled = Vec::new();
    for order in BACKTEST_OPEN_ORDER_CACHE.iter() {
        let market_price = match get_market_fill_price_estimate(order.side, &order.symbol_name, order.quantity_ordered, order.brokerage).await {
            Ok(price) => price,
            Err(e) => {
                eprintln!("{}: for {}", e, order.symbol_name);
                continue
            }
        };

        //3. respond with an order event
        let mut brackets: Option<Vec<ProtectiveOrder>> = None;
        match &order.order_type {
            OrderType::Limit => {
                let is_fill_triggered = match order.side {
                    OrderSide::Buy => market_price <= order.limit_price.unwrap(),
                    OrderSide::Sell => market_price >= order.limit_price.unwrap()
                };
                if is_fill_triggered {
                    filled.push((order.id.clone(), order.limit_price.unwrap().clone()));
                }
            }
            OrderType::Market => {
                filled.push((order.id.clone(), market_price));
            },
            OrderType::MarketIfTouched | OrderType::StopMarket => {
                let is_fill_triggered = match order.side {
                    OrderSide::Buy => market_price <= order.trigger_price.unwrap(),
                    OrderSide::Sell => market_price >= order.trigger_price.unwrap()
                };
                if is_fill_triggered {
                    filled.push((order.id.clone(), market_price));
                }
            }
            OrderType::StopLimit => {
                let is_fill_triggered = match order.side {
                    OrderSide::Buy => market_price <= order.trigger_price.unwrap() && market_price > order.limit_price.unwrap(),
                    OrderSide::Sell => market_price >= order.trigger_price.unwrap() && market_price < order.limit_price.unwrap()
                };
                if is_fill_triggered {
                    //todo need a better way to simulate stop limits, use custom market price fn to
                    filled.push((order.id.clone(),  order.limit_price.unwrap().clone()));
                }
            },
            OrderType::EnterLong(new_brackets) => {
                if is_short_paper(&order.brokerage, &order.account_id, &order.symbol_name) {
                    exit_position_paper(order.brokerage, &order.account_id, &order.symbol_name, time.clone()).await;
                }
                filled.push((order.id.clone(), market_price));
                brackets = new_brackets.clone();
            }
            OrderType::EnterShort(new_brackets) => {
                if is_long_paper(&order.brokerage, &order.account_id, &order.symbol_name) {
                    exit_position_paper(order.brokerage, &order.account_id, &order.symbol_name, time.clone()).await;
                }
                filled.push((order.id.clone(), market_price));
                brackets = new_brackets.clone();
            }
            OrderType::ExitLong => {
                if is_long_paper(&order.brokerage, &order.account_id, &order.symbol_name) {
                    filled.push((order.id.clone(), market_price));
                } else {
                    let reason = "No long position to exit".to_string();
                    rejected.push((order.id.clone(), reason));
                }
            }
            OrderType::ExitShort => {
                if is_short_paper(&order.brokerage, &order.account_id, &order.symbol_name) {
                    filled.push((order.id.clone(), market_price));
                } else {
                    let reason = "No short position to exit".to_string();
                    rejected.push((order.id.clone(), reason));
                }
            }
            OrderType::UpdateBrackets(broker, account_id, symbol_name, brackets) => {
                if let Some(brokerage_map) = OPEN_BACKTEST_POSITIONS.get_mut(&broker) {
                    if let Some(mut position) = brokerage_map.get_mut(account_id) {
                        position.brackets = Some(brackets.clone());
                        let event = StrategyEvent::OrderEvents(OrderUpdateEvent::Updated {
                            brokerage: order.brokerage,
                            account_id: order.id.clone(),
                            order_id: order.id.clone(),
                        });
                        add_buffer(time, event).await;
                    }
                } else {
                    let reason = "No position for update brackets".to_string();
                    rejected.push((order.id.clone(), reason));
                }
            }
        }
    }

    for (order_id, reason) in rejected {
        reject_order(reason, &order_id, time).await
    }
    for order_id in accepted {
        accept_order(order_id, time).await;
    }
    for (order_id, price) in filled {
        fill_order(&order_id, time, price).await
    }
}

async fn fill_order(
    order_id: &OrderId,
    last_time_utc: DateTime<Utc>,
    market_price: Price
) {
    if let Some((order_id, mut order)) = BACKTEST_OPEN_ORDER_CACHE.remove(order_id) {
        order.time_filled_utc = Some(last_time_utc.to_string());
        order.state = OrderState::Filled;
        order.average_fill_price = Some(market_price);
        order.quantity_filled = order.quantity_ordered;
        order.time_filled_utc = Some(last_time_utc.to_string());
        add_buffer(last_time_utc, StrategyEvent::OrderEvents(OrderUpdateEvent::Filled { order_id: order.id.clone(), brokerage: order.brokerage, account_id: order.account_id.clone() })).await;
        BACKTEST_CLOSED_ORDER_CACHE.insert(order.id.clone(), order.clone());
    }
}

async fn reject_order(
    reason: String,
    order_id: &OrderId,
    time: DateTime<Utc>,
) {
    if let Some((order_id, mut order)) = BACKTEST_OPEN_ORDER_CACHE.remove(order_id) {
        order.state = OrderState::Rejected(reason.clone());
        order.time_created_utc = time.to_string();

        add_buffer(
            time,
            StrategyEvent::OrderEvents(OrderUpdateEvent::Rejected {
                order_id: order.id.clone(),
                brokerage: order.brokerage,
                account_id: order.account_id.clone(),
                reason,
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
            StrategyEvent::OrderEvents(OrderUpdateEvent::Accepted {
                order_id: order.id.clone(),
                brokerage: order.brokerage.clone(),
                account_id: order.account_id.clone(),
            }),
        ).await;
    }
}

pub async fn process_ledgers() -> Vec<String> {
    // Acquire a read lock on the RwLock
    // Iterate over the HashMap while holding the read lock
    /*let mut return_strings = vec![];
    for brokerage_map in self.ledgers.iter() {
        for ledger in brokerage_map.iter() {
            return_strings.push(format!("{} \n", ledger.value().print()));
        }
    }
    return_strings*/
    todo!()
}

pub async fn print_ledger(brokerage: Brokerage, account_id: AccountId) -> Option<String> {
    todo!() //Generate strategy statistics from closed positions
}

pub fn is_long_live(brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
    if let Some(ledger_map) = OPEN_LIVE_POSITIONS.get(brokerage) {
        if let Some(broker_ledgers) = ledger_map.get(account_id) {
            return match broker_ledgers.side {
                PositionSide::Long => true,
                PositionSide::Short => false,
            }
        }
        return false
    }
    false
}

pub fn is_short_live(brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
    if let Some(ledger_map) = OPEN_LIVE_POSITIONS.get(brokerage) {
        if let Some(broker_ledgers) = ledger_map.get(account_id) {
            return match broker_ledgers.side {
                PositionSide::Long => false,
                PositionSide::Short => true,
            }
        }
        return false
    }
    false
}

pub fn is_flat_live(brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
    if let Some(ledger_map) = OPEN_LIVE_POSITIONS.get(brokerage) {
        if let Some(broker_ledgers) = ledger_map.get(account_id) {
            return match broker_ledgers.side {
                PositionSide::Long => false,
                PositionSide::Short => false,
            }
        }
        return true
    }
    true
}

pub fn is_long_paper(brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
    if let Some(ledger_map) = OPEN_BACKTEST_POSITIONS.get(brokerage) {
        if let Some(broker_ledgers) = ledger_map.get(account_id) {
            return match broker_ledgers.side {
                PositionSide::Long => true,
                PositionSide::Short => false,
            }
        }
        return false
    }
    false
}

pub fn is_short_paper(brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
    if let Some(ledger_map) = OPEN_BACKTEST_POSITIONS.get(brokerage) {
        if let Some(broker_ledgers) = ledger_map.get(account_id) {
            return match broker_ledgers.side {
                PositionSide::Long => false,
                PositionSide::Short => true,
            }
        }
        return false
    }
    false
}

pub fn is_flat_paper(brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
    if let Some(ledger_map) = OPEN_BACKTEST_POSITIONS.get(brokerage) {
        if let Some(broker_ledgers) = ledger_map.get(account_id) {
            return match broker_ledgers.side {
                PositionSide::Long => false,
                PositionSide::Short => false,
            }
        }
        return true
    }
    true
}

pub async fn get_market_fill_price_estimate (
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
            if let Some((price, available_volume)) = book_price_volume_map.value().get(&level) {
                if *available_volume == dec!(0.0) && total_volume_filled == dec!(0.0) {
                    return Ok(price.clone())
                }
                if *available_volume == dec!(0.0) {
                    continue;
                }
                let volume_to_use = remaining_volume.min(*available_volume);
                total_price_volume += *price * volume_to_use;
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

pub async fn get_market_price (
    order_side: OrderSide,
    symbol_name: &SymbolName,
) -> Result<Price, FundForgeError> {
    let order_book = match order_side {
        OrderSide::Buy => ASK_BOOKS.get(symbol_name),
        OrderSide::Sell => BID_BOOKS.get(symbol_name)
    };

    if let Some(symbol_book) = order_book {
        if let Some((price, _)) = symbol_book.value().get(&0) {
            return Ok(price.clone())
        }
    }

    // If we couldn't get a price from the order book, fall back to the last price
    if let Some(last_price) = LAST_PRICE.get(symbol_name) {
        return Ok(last_price.value().clone());
    }

    Err(FundForgeError::ClientSideErrorDebug(String::from("No market price found for symbol")))
}

pub async fn update_or_create_paper_position(
    symbol_name: &SymbolName,
    brokerage: Brokerage,
    account_id: &AccountId,
    quantity: Volume,
    side: OrderSide,
    time: &DateTime<Utc>,
    brackets: Option<Vec<ProtectiveOrder>>,
    market_price: Price
) -> Result<(), FundForgeError>
{
    // Check if there's an existing position for the given symbol
    if let Some(broker_positions_map) = OPEN_BACKTEST_POSITIONS.get_mut(&brokerage) {
        if let Some((account_id, mut existing_position)) = broker_positions_map.remove(account_id) {
            let is_reducing = (existing_position.side == PositionSide::Long && side == OrderSide::Sell)
                || (existing_position.side == PositionSide::Short && side == OrderSide::Buy);

            if is_reducing {
                if let Some(new_brackets) = brackets {
                    existing_position.brackets = Some(new_brackets);
                }
                // TODO: Adjust paper account balance, get market price, determine average closing price
                Ok(())
            } else {
                existing_position.add_to_position(market_price, quantity, time);

                if let Some(new_brackets) = brackets {
                    existing_position.brackets = Some(new_brackets);
                }

                broker_positions_map.insert(symbol_name.clone(), existing_position);
                // TODO: Determine paper account balances
                Ok(())
            }
        } else {
            Err(FundForgeError::ClientSideErrorDebug(format!("No existing position for account: {}", account_id)))
        }
    } else {
        // Fetch symbol info if it doesn't exist
        if !SYMBOL_INFO.contains_key(symbol_name) {
            match brokerage.symbol_info(symbol_name.clone()).await {
                Ok(info) => {
                    SYMBOL_INFO.insert(symbol_name.clone(), info.clone());
                },
                Err(_) => return Err(FundForgeError::ClientSideErrorDebug(format!("No Symbol info for: {}", symbol_name)))
            };
        }

        // Determine the side of the position based on the order side
        let position_side = match side {
            OrderSide::Buy => PositionSide::Long,
            OrderSide::Sell => PositionSide::Short,
        };

        let info = SYMBOL_INFO.get(symbol_name).unwrap().value().clone();

        // Create a new position
        let position = Position::enter(
            symbol_name.clone(),
            brokerage.clone(),
            account_id.clone(),
            position_side,
            quantity,
            market_price,
            generate_id(symbol_name, brokerage, account_id, time.clone(), position_side).await,
            info.clone(),
            info.pnl_currency,
            brackets
        );

        // Insert the new position into the positions map
        let mut account_map = DashMap::new();
        account_map.insert(account_id.clone(), position);
        OPEN_BACKTEST_POSITIONS.insert(brokerage, account_map);

        // TODO: Adjust paper account balance
        Ok(())
    }
}

pub async fn generate_id(
    symbol_name: &SymbolName,
    brokerage: Brokerage,
    account_id: &AccountId,
    time: DateTime<Utc>,
    side: PositionSide
) -> PositionId {
    // Get or insert the brokerage map
    let broker_count_map = POSITIONS_COUNTER.entry(brokerage)
        .or_insert_with(DashMap::new);

    // Get or insert the account map
    let mut account_map = broker_count_map.entry(account_id.clone())
        .or_insert_with(AHashMap::new);

    // Increment the counter for the symbol, or insert it if it doesn't exist
    let counter = account_map.entry(symbol_name.clone())
        .and_modify(|count| *count += 1)
        .or_insert(1);

    // Return the generated position ID
    format!("{}-{}-{}-{}", symbol_name, counter, time.timestamp_nanos_opt().unwrap(), side)
}

pub async fn exit_position_paper(brokerage: Brokerage, account_id: &AccountId, symbol_name: &SymbolName, time: DateTime<Utc>) {
    if let Some(broker_map) = OPEN_BACKTEST_POSITIONS.get(&brokerage) {
        if let Some((account, position)) = broker_map.remove(account_id) {
            let side = match position.side {
                PositionSide::Long => OrderSide::Sell,
                PositionSide::Short => OrderSide::Buy
            };
            let market_price = get_market_fill_price_estimate(side, symbol_name, position.quantity_open, brokerage).await.unwrap();
            //todo update position exit price using average price

            if !CLOSED_BACKTEST_POSITIONS.contains_key(&brokerage) {
                let mut account_map = DashMap::new();
                account_map.insert(account_id.clone(), vec![position]);
                CLOSED_BACKTEST_POSITIONS.insert(brokerage, account_map);
            } else if let Some(broker_map) = CLOSED_BACKTEST_POSITIONS.get_mut(&brokerage) {
                if !broker_map.contains_key(account_id) {
                    broker_map.insert(account_id.clone(), vec![position.clone()]);
                }
                if let Some(mut account_closed_map) = broker_map.get_mut(account_id) {
                    account_closed_map.value_mut().push(position);
                }
            }
        }
    }
}

pub async fn on_historical_data_update(time_slice: TimeSlice, time: &DateTime<Utc>) {
    let mut open_pnl = dec!(0.0);
    let mut brackets_booked_pnl = dec!(0.0);
    for base_data_enum in time_slice.iter() {
        //if let Some(position)
    }
    //todo adjust pnl for booked profits
}

