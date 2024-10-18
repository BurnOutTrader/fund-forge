use std::collections::BTreeMap;
use std::str::FromStr;
use chrono::{DateTime, Utc};
use crate::strategies::ledgers::Ledger;
use crate::standardized_types::enums::{OrderSide, PositionSide, StrategyMode};
use crate::strategies::strategy_events::StrategyEvent;
use crate::standardized_types::subscriptions::{Symbol, SymbolName};
use std::sync::Arc;
use dashmap::DashMap;
use lazy_static::lazy_static;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc;
use crate::standardized_types::broker_enum::Brokerage;
use crate::strategies::client_features::server_connections::{add_buffer, is_warmup_complete};
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::time_slices::TimeSlice;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::standardized_types::accounts::{AccountId, AccountSetup, Currency};
use crate::standardized_types::books::BookLevel;
use crate::standardized_types::new_types::Price;
use crate::standardized_types::orders::{Order, OrderId, OrderRequest, OrderUpdateEvent, OrderUpdateType};
use crate::standardized_types::symbol_info::SymbolInfo;
use crate::strategies::handlers::market_handler::backtest_matching_engine;
use crate::strategies::historical_time::get_backtest_time;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum MarketMessageEnum {
    RegisterSymbol(Symbol),
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

pub(crate) async fn initialize_live_account(brokerage: Brokerage, account_id: &AccountId) {
    if !LIVE_LEDGERS.contains_key(&brokerage) {
        LIVE_LEDGERS.insert(brokerage.clone(), DashMap::new());
    }
    if let Some(mut broker_map) = LIVE_LEDGERS.get_mut(&brokerage) {
        if !broker_map.contains_key(account_id) {
            let account_info = match brokerage.account_info(account_id.clone()).await {
                Ok(info) => info,
                Err(e) => {
                    eprintln!("Unable to get account info for: {} {}: error: {}", brokerage, account_id, e);
                    return;
                }
            };

            let ledger = Ledger::new(account_info, StrategyMode::Live);
            broker_map.value_mut().insert(account_id.clone(), ledger);
        }
    }
}

fn time_slice_ledger_updates(mode: StrategyMode, time_slice: TimeSlice, time: DateTime<Utc>) {
    if mode != StrategyMode::Live {
        for broker_map in BACKTEST_LEDGERS.iter() {
            for mut account_map in broker_map.iter_mut() {
                account_map.value_mut().timeslice_update(time_slice.clone(), time);
            }
        }
    } else {
        for broker_map in LIVE_LEDGERS.iter() {
            for mut account_map in broker_map.iter_mut() {
                account_map.value_mut().timeslice_update(time_slice.clone(), time);
            }
        }
    }
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
                    println!("symbol registered: {:?}", symbol);
                    BID_BOOKS.insert(symbol.name.clone(), BTreeMap::new());
                    ASK_BOOKS.insert(symbol.name.clone(), BTreeMap::new());
                }
                MarketMessageEnum::TimeSliceUpdate(time_slice) => {
                    for base_data in time_slice.iter() {
                        update_base_data(base_data.clone());
                    }
                    time_slice_ledger_updates(mode, time_slice.clone(), time);
                    if mode == StrategyMode::LivePaperTrading || mode == StrategyMode::Backtest {
                        backtest_matching_engine::backtest_matching_engine(time).await;
                    }
                }
                MarketMessageEnum::OrderBookSnapShot{symbol , bid_book, ask_book } => {
                    if !BID_BOOKS.contains_key(&symbol.name){
                        BID_BOOKS.insert(symbol.name.clone(), BTreeMap::new());
                    }
                    BID_BOOKS.insert(symbol.name.clone(), bid_book);
                    ASK_BOOKS.insert(symbol.name.clone(), ask_book);
                    if mode == StrategyMode::LivePaperTrading || mode == StrategyMode::Backtest {
                        backtest_matching_engine::backtest_matching_engine(time).await;
                    }
                }
                MarketMessageEnum::OrderRequest(order_request) => {
                    if !is_warmup_complete() {
                        continue;
                    }
                    match mode {
                        StrategyMode::Live => panic!("Live orders do not get sent via market handler"),
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
            backtest_matching_engine::backtest_matching_engine(time.clone()).await;
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
                let update_event = StrategyEvent::OrderEvents(OrderUpdateEvent::OrderUpdated { brokerage, account_id: order.account_id.clone(), order_id: order.id.clone(), update_type: update, tag: order.tag.clone(), time: time.to_string()});
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
    if let Some(broker_map) = LIVE_LEDGERS.get(brokerage) {
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
        if let Some(account_map) = broker_map.value().get(account_id) {
            return account_map.value().booked_pnl(symbol_name)
        }
    }
    dec!(0.0)
}

pub(crate) fn position_size_live(symbol_name: &SymbolName, brokerage: &Brokerage, account_id: &AccountId) -> Decimal {
    if let Some(broker_map) = LIVE_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.value().get(account_id) {
            return account_map.value().position_size(symbol_name)
        }
    }
    dec!(0.0)
}

pub(crate) fn position_size_paper(symbol_name: &SymbolName, brokerage: &Brokerage, account_id: &AccountId) -> Decimal {
    if let Some(broker_map) = BACKTEST_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.value().get(account_id) {
            return account_map.value().position_size(symbol_name)
        }
    }
    dec!(0.0)
}


pub(crate) fn is_long_live(brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
    if let Some(broker_map) = LIVE_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.value().get(account_id) {
            return account_map.value().is_long(symbol_name)
        }
    }
    false
}

pub(crate) fn is_short_live(brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
    if let Some(broker_map) = LIVE_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.value().get(account_id) {
            return account_map.value().is_short(symbol_name)
        }
    }
    false
}

pub(crate) fn is_flat_live(brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
    if let Some(broker_map) = LIVE_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.value().get(account_id) {
            return account_map.value().is_flat(symbol_name)
        }
    }
    true
}

pub(crate) fn is_long_paper(brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
    if let Some(broker_map) = BACKTEST_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.value().get(account_id) {
            return account_map.value().is_long(symbol_name)
        }
    }
    false
}

pub(crate) fn is_short_paper(brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
    if let Some(broker_map) = BACKTEST_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.value().get(account_id) {
            return account_map.value().is_short(symbol_name)
        }
    }
    false
}

pub(crate) fn is_flat_paper(brokerage: &Brokerage, account_id: &AccountId, symbol_name: &SymbolName) -> bool {
    if let Some(broker_map) = BACKTEST_LEDGERS.get(&brokerage) {
        if let Some(account_map) = broker_map.value().get(account_id) {
            return account_map.value().is_flat(symbol_name)
        }
    }
    true
}

async fn flatten_all_paper_for(brokerage: &Brokerage, account_id: &AccountId, time: DateTime<Utc>) {
    if let Some(broker_map) = BACKTEST_LEDGERS.get(&brokerage) {
        if let Some(mut account_map) = broker_map.value().get_mut(account_id) {
            for (symbol_name, position) in account_map.positions.clone() {
                let side = match position.side {
                    PositionSide::Long => OrderSide::Sell,
                    PositionSide::Short => OrderSide::Buy
                };
                let market_price = backtest_matching_engine::get_market_fill_price_estimate(side, &symbol_name, position.quantity_open, position.brokerage).await.unwrap();
                if let Some(event) = account_map.exit_position(&symbol_name, time, market_price, String::from("Flatten Account")).await {
                    add_buffer(time, StrategyEvent::PositionEvents(event)).await;
                }
            }
        }
    }
}

pub(crate) fn add_account(mode: StrategyMode, account_setup: AccountSetup) {
    let map = BACKTEST_LEDGERS.entry(account_setup.brokerage).or_insert(DashMap::new());
    if map.contains_key(&account_setup.account_id) {
        return;
    }
    map.insert(account_setup.account_id.clone() ,Ledger::user_initiated(account_setup, mode));
}




