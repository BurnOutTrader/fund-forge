use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use ahash::AHashMap;
use dashmap::DashMap;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tokio::sync::{mpsc, oneshot};
use crate::messages::data_server_messaging::FundForgeError;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::books::BookLevel;
use crate::standardized_types::enums::OrderSide;
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::subscriptions::{SymbolCode, SymbolName};
use crate::standardized_types::time_slices::TimeSlice;
use once_cell::sync::Lazy;
use crate::standardized_types::base_data::tick::Aggressor;

static MARKET_PRICE_SERVICE: Lazy<MarketPriceService> = Lazy::new(|| {
    MarketPriceService::new()
});
pub(crate) fn get_price_service_sender() -> mpsc::Sender<PriceServiceMessage> {
    MARKET_PRICE_SERVICE.sender.clone()
}

pub(crate) async fn price_service_request_market_price(order_side: OrderSide, symbol_name: SymbolName, symbol_code: SymbolCode) -> Result<PriceServiceResponse, FundForgeError> {
    let callback_id = MARKET_PRICE_SERVICE.next_id.fetch_add(1, Ordering::SeqCst);
    let (response_sender, response_receiver) = oneshot::channel();
    let message = PriceServiceMessage::MarketPrice{callback_id, order_side, symbol_name, symbol_code};

    MARKET_PRICE_SERVICE.callbacks.insert(callback_id, response_sender);
    match MARKET_PRICE_SERVICE.sender.send(message).await {
        Ok(_) => {}
        Err(e) => return Err(FundForgeError::ClientSideErrorDebug(format!("Market Price Service: Failed to send request: {}", e)))
    }

    let response = match response_receiver.await {
        Ok(response) => response,
        Err(e) => return Err(FundForgeError::ClientSideErrorDebug(format!("Market Price Service: Failed to receive callback: {}", e)))
    };
    Ok(response)
}

pub(crate) async fn price_service_request_market_fill_price(order_side: OrderSide, symbol_name: SymbolName, symbol_code: SymbolCode, volume: Volume) -> Result<PriceServiceResponse, FundForgeError> {
    let callback_id = MARKET_PRICE_SERVICE.next_id.fetch_add(1, Ordering::SeqCst);
    let (response_sender, response_receiver) = oneshot::channel();
    let message = PriceServiceMessage::FillPriceEstimate{callback_id, order_side, symbol_name, symbol_code, volume };

    MARKET_PRICE_SERVICE.callbacks.insert(callback_id, response_sender);
    match MARKET_PRICE_SERVICE.sender.send(message).await {
        Ok(_) => {}
        Err(e) => return Err(FundForgeError::ClientSideErrorDebug(format!("Market Price Service: Failed to send request: {}", e)))
    }

    let response = match response_receiver.await {
        Ok(response) => response,
        Err(e) => return Err(FundForgeError::ClientSideErrorDebug(format!("Market Price Service: Failed to receive callback: {}", e)))
    };
    Ok(response)
}

pub(crate) async fn price_service_request_limit_fill_price_quantity(order_side: OrderSide, symbol_name: SymbolName, symbol_code: SymbolCode, volume: Volume, limit: Decimal) -> Result<PriceServiceResponse, FundForgeError> {
    let callback_id = MARKET_PRICE_SERVICE.next_id.fetch_add(1, Ordering::SeqCst);
    let (response_sender, response_receiver) = oneshot::channel();
    let message = PriceServiceMessage::LimitFillPriceEstimate{callback_id, order_side, symbol_name, symbol_code, volume, limit };

    MARKET_PRICE_SERVICE.callbacks.insert(callback_id, response_sender);
    match MARKET_PRICE_SERVICE.sender.send(message).await {
        Ok(_) => {}
        Err(e) => return Err(FundForgeError::ClientSideErrorDebug(format!("Market Price Service: Failed to send request: {}", e)))
    }

    let response = match response_receiver.await {
        Ok(response) => response,
        Err(e) => return Err(FundForgeError::ClientSideErrorDebug(format!("Market Price Service: Failed to receive callback: {}", e)))
    };
    Ok(response)
}

#[derive(Debug)]
pub enum PriceServiceMessage {
    TimeSliceUpdate(Arc<TimeSlice>),
    FillPriceEstimate{callback_id: u64, order_side: OrderSide, symbol_name: SymbolName, symbol_code: SymbolCode, volume: Volume},
    LimitFillPriceEstimate{callback_id: u64, order_side: OrderSide, symbol_name: SymbolName, symbol_code: SymbolCode, volume: Volume, limit: Price},
    MarketPrice{callback_id: u64, order_side: OrderSide, symbol_name: SymbolName, symbol_code: SymbolCode},
}

/// Returned values to be rounded to symbol decimal accuracy
#[derive(Debug)]
pub enum PriceServiceResponse {
    FillPriceEstimate(Option<Decimal>),
    LimitFillPriceEstimate{fill_price: Option<Decimal>, fill_volume: Option<Decimal>},
    MarketPrice(Option<Decimal>),
}

impl PriceServiceResponse {
    pub fn price(&self) -> Option<Decimal> {
        match self {
            PriceServiceResponse::FillPriceEstimate(price) => price.clone(),
            PriceServiceResponse::LimitFillPriceEstimate { fill_price, .. } => fill_price.clone(),
            PriceServiceResponse::MarketPrice(fill_price) => fill_price.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn volume_filled(&self) -> Option<Volume> {
        match self {
            PriceServiceResponse::FillPriceEstimate(_) => None,
            PriceServiceResponse::LimitFillPriceEstimate { fill_volume, .. } => fill_volume.clone(),
            PriceServiceResponse::MarketPrice(_) => None
        }
    }
}


struct MarketPriceService {
    sender: mpsc::Sender<PriceServiceMessage>,
    callbacks: Arc<DashMap<u64, oneshot::Sender<PriceServiceResponse>>>,
    next_id: AtomicU64,
}

impl MarketPriceService {
    fn new() -> Self {
        let (sender, receiver) = mpsc::channel(100);
        let service = MarketPriceService {
            sender,
            callbacks: Arc::new(DashMap::new()),
            next_id: AtomicU64::new(0),
        };

        tokio::spawn(MarketPriceService::run(service.callbacks.clone(), receiver));

        service
    }

    async fn run(callbacks: Arc<DashMap<u64, oneshot::Sender<PriceServiceResponse>>>, receiver: mpsc::Receiver<PriceServiceMessage>) {
        let mut receiver = receiver;
        let mut bid_books: AHashMap<SymbolName, BTreeMap<u16, BookLevel>> = AHashMap::new();
        let mut ask_books: AHashMap<SymbolName, BTreeMap<u16, BookLevel>> = AHashMap::new();
        let mut has_quotes: AHashMap<SymbolName, bool> = AHashMap::new();
        let mut last_price: AHashMap<SymbolName, Price> = AHashMap::new();

        'main_loop: while let Some(message) = receiver.recv().await {
            match message {
                PriceServiceMessage::TimeSliceUpdate(time_slice) => {
                    'data_loop: for base_data in time_slice.iter() {
                        match base_data {
                            BaseDataEnum::Candle(candle) => {
                                last_price.insert(candle.symbol.name.clone(), candle.close);
                            }
                            BaseDataEnum::QuoteBar(quotebar) => {
                                let symbol_name = &quotebar.symbol.name;
                                // Early continue if we already have quotes
                                if has_quotes.contains_key(symbol_name) {
                                    continue 'data_loop;
                                }

                                // Initialize books if needed - do this once for both bid and ask
                                let (bid_book, ask_book) = match (bid_books.get_mut(symbol_name), ask_books.get_mut(symbol_name)) {
                                    (Some(bid), Some(ask)) => (bid, ask),
                                    _ => {
                                        bid_books.entry(symbol_name.clone()).or_insert_with(BTreeMap::new);
                                        ask_books.entry(symbol_name.clone()).or_insert_with(BTreeMap::new);
                                        (
                                            bid_books.get_mut(symbol_name).unwrap(),
                                            ask_books.get_mut(symbol_name).unwrap()
                                        )
                                    }
                                };

                                // Update both books
                                bid_book.insert(0, BookLevel::new(0, quotebar.bid_close, dec!(0.0)));
                                ask_book.insert(0, BookLevel::new(0, quotebar.ask_close, dec!(0.0)));
                            }
                            BaseDataEnum::Tick(tick) => {
                                let symbol_name = &tick.symbol.name;
                                last_price.insert(symbol_name.clone(), tick.price);

                                // Process tick data only if we don't have quotes and there's an aggressor
                                if tick.aggressor != Aggressor::None && !has_quotes.contains_key(symbol_name) {
                                    // Initialize books if needed - do this once
                                    let (bid_book, ask_book) = match (bid_books.get_mut(symbol_name), ask_books.get_mut(symbol_name)) {
                                        (Some(bid), Some(ask)) => (bid, ask),
                                        _ => {
                                            bid_books.entry(symbol_name.clone()).or_insert_with(BTreeMap::new);
                                            ask_books.entry(symbol_name.clone()).or_insert_with(BTreeMap::new);
                                            (
                                                bid_books.get_mut(symbol_name).unwrap(),
                                                ask_books.get_mut(symbol_name).unwrap()
                                            )
                                        }
                                    };

                                    // Update appropriate book based on aggressor
                                    match tick.aggressor {
                                        Aggressor::Buy => ask_book.insert(0, BookLevel::new(0, tick.price, dec!(0.0))),
                                        Aggressor::Sell => bid_book.insert(0, BookLevel::new(0, tick.price, dec!(0.0))),
                                        _ => None,
                                    };
                                }
                            }
                            BaseDataEnum::Quote(quote) => {
                                let symbol_name = &quote.symbol.name;
                                // Initialize books and set has_quotes flag if needed
                                if !has_quotes.contains_key(symbol_name) {
                                    has_quotes.insert(symbol_name.clone(), true);
                                    let (bid_book, ask_book) = match (bid_books.get_mut(symbol_name), ask_books.get_mut(symbol_name)) {
                                        (Some(bid), Some(ask)) => (bid, ask),
                                        _ => {
                                            bid_books.entry(symbol_name.clone()).or_insert_with(BTreeMap::new);
                                            ask_books.entry(symbol_name.clone()).or_insert_with(BTreeMap::new);
                                            (
                                                bid_books.get_mut(symbol_name).unwrap(),
                                                ask_books.get_mut(symbol_name).unwrap()
                                            )
                                        }
                                    };

                                    // Update both books
                                    bid_book.insert(0, BookLevel::new(0, quote.bid, quote.bid_volume));
                                    ask_book.insert(0, BookLevel::new(0, quote.ask, quote.ask_volume));
                                }
                            }
                            _ => eprintln!("Market Price Service: Incorrect data type in Market Updates: {}", base_data.base_data_type())
                        }
                    }
                }
                PriceServiceMessage::FillPriceEstimate { callback_id, order_side, symbol_name, symbol_code, volume } => {
                    if let Some((_, callback_sender)) = callbacks.remove(&callback_id) {
                        let order_book = match order_side {
                            OrderSide::Buy => {
                                match ask_books.get(&symbol_code) {
                                    Some(book) => Some(book),
                                    None => ask_books.get(&symbol_name)
                                }

                            },
                            OrderSide::Sell => {
                                match bid_books.get(&symbol_code) {
                                    Some(book) => Some(book),
                                    None => bid_books.get(&symbol_name)
                                }
                            }
                        };

                        if let Some(book_price_volume_map) = order_book {
                            let mut total_price_volume = dec!(0.0);
                            let mut total_volume_filled = dec!(0.0);
                            let mut remaining_volume = volume;

                            'book_loop: for level in 0.. {
                                if let Some(bool_level) = book_price_volume_map.get(&level) {
                                    if bool_level.volume == dec!(0.0) && total_volume_filled == dec!(0.0) {
                                        let message = PriceServiceResponse::FillPriceEstimate(Some(bool_level.price.clone()));
                                        if let Err(_e) = callback_sender.send(message) {
                                            eprintln!("Market Price Service: Failed to send response");
                                        }
                                        continue 'main_loop
                                    }
                                    if bool_level.volume == dec!(0.0) {
                                        continue 'book_loop;
                                    }
                                    let volume_to_use = remaining_volume.min(bool_level.volume);
                                    total_price_volume += bool_level.price * volume_to_use;
                                    total_volume_filled += volume_to_use;
                                    remaining_volume -= volume_to_use;

                                    if remaining_volume == dec!(0.0) {
                                        // We've filled the entire requested volume
                                        let fill_price = total_price_volume / total_volume_filled;
                                        let message = PriceServiceResponse::FillPriceEstimate(Some(fill_price));
                                        if let Err(_e) = callback_sender.send(message) {
                                            eprintln!("Market Price Service: Failed to send response");
                                        }
                                        continue 'main_loop;
                                    }
                                } else {
                                    // No more levels in the order book
                                    break 'book_loop;
                                }
                            }

                            if total_volume_filled > dec!(0.0) {
                                // We filled some volume, but not all. Return the average price for what we could fill.
                                let fill_price = total_price_volume / total_volume_filled;
                                let message = PriceServiceResponse::FillPriceEstimate(Some(fill_price));
                                if let Err(_e) = callback_sender.send(message) {
                                    eprintln!("Market Price Service: Failed to send response");
                                }
                            }
                        }
                        else if let Some(last_price) = last_price.get(&symbol_name) {
                            let message = PriceServiceResponse::FillPriceEstimate(Some(last_price.clone()));
                            if let Err(_e) = callback_sender.send(message) {
                                eprintln!("Market Price Service: Failed to send response");
                            }
                        }
                        else {
                            let message = PriceServiceResponse::FillPriceEstimate(None);
                            if let Err(_e) = callback_sender.send(message) {
                                eprintln!("Market Price Service: Failed to send response");
                            }
                        }
                    }
                }
                PriceServiceMessage::LimitFillPriceEstimate { callback_id, order_side, symbol_name, symbol_code, volume, limit } => {
                    if let Some((_, callback_sender)) = callbacks.remove(&callback_id) {
                        let order_book = match order_side {
                            OrderSide::Buy => {
                                match ask_books.get(&symbol_code) {
                                    Some(book) => Some(book),
                                    None => ask_books.get(&symbol_name)
                                }

                            },
                            OrderSide::Sell => {
                                match bid_books.get(&symbol_code) {
                                    Some(book) => Some(book),
                                    None => bid_books.get(&symbol_name)
                                }
                            }
                        };

                        if let Some(book_price_volume_map) = order_book {
                            let mut total_price_volume = dec!(0.0);
                            let mut total_volume_filled = dec!(0.0);
                            let mut remaining_volume = volume;
                            'book_loop: for level in 0.. {
                                if let Some(bool_level) = book_price_volume_map.get(&level) {
                                    if bool_level.volume == dec!(0.0) && total_volume_filled == dec!(0.0) && level == 0 {
                                        let message = PriceServiceResponse::LimitFillPriceEstimate{fill_price: Some(bool_level.price.clone()), fill_volume: Some(volume)};
                                        if let Err(_e) = callback_sender.send(message) {
                                            eprintln!("Market Price Service: Failed to send response");
                                        }
                                        continue 'main_loop
                                    }
                                    if bool_level.volume == dec!(0.0) {
                                        continue 'book_loop;
                                    }
                                    match order_side {
                                        OrderSide::Buy => {
                                            if bool_level.price > limit {
                                                break 'book_loop;
                                            }
                                        }
                                        OrderSide::Sell => {
                                            if bool_level.price < limit {
                                                break 'book_loop;
                                            }
                                        }
                                    }
                                    let volume_to_use = remaining_volume.min(bool_level.volume);
                                    total_price_volume += bool_level.price * volume_to_use;
                                    total_volume_filled += volume_to_use;
                                    remaining_volume -= volume_to_use;

                                    if remaining_volume == dec!(0.0) {
                                        // We've filled the entire requested volume
                                        let fill_price = total_price_volume / total_volume_filled;
                                        let message = PriceServiceResponse::LimitFillPriceEstimate {fill_price: Some(fill_price), fill_volume: Some(total_volume_filled)};
                                        if let Err(_e) = callback_sender.send(message) {
                                            eprintln!("Market Price Service: Failed to send response");
                                        }
                                        continue 'main_loop
                                    }
                                } else {
                                    break 'book_loop;
                                }
                            }
                            if total_volume_filled > dec!(0.0) {
                                let fill_price = total_price_volume / total_volume_filled;
                                let message = PriceServiceResponse::LimitFillPriceEstimate{fill_price: Some(fill_price), fill_volume: Some(total_volume_filled)};
                                if let Err(_e) = callback_sender.send(message) {
                                    eprintln!("Market Price Service: Failed to send response");
                                }
                            }
                        }
                        else if let Some(last_price) = last_price.get(&symbol_name) {
                            let message = PriceServiceResponse::LimitFillPriceEstimate{fill_price: Some(last_price.clone()), fill_volume: Some(volume)};
                            if let Err(_e) = callback_sender.send(message) {
                                eprintln!("Market Price Service: Failed to send response");
                            }
                        }
                        else {
                            let message = PriceServiceResponse::LimitFillPriceEstimate{fill_price: None, fill_volume: None };
                            if let Err(_e) = callback_sender.send(message) {
                                eprintln!("Market Price Service: Failed to send response");
                            }
                        }
                    }
                }
                PriceServiceMessage::MarketPrice { callback_id, order_side, symbol_name, symbol_code } => {
                    if let Some((_, callback_sender)) = callbacks.remove(&callback_id) {
                        let order_book = match order_side {
                            OrderSide::Buy => {
                                match ask_books.get(&symbol_code) {
                                    Some(book) => Some(book),
                                    None => ask_books.get(&symbol_name)
                                }

                            },
                            OrderSide::Sell => {
                                match bid_books.get(&symbol_code) {
                                    Some(book) => Some(book),
                                    None => bid_books.get(&symbol_name)
                                }
                            }
                        };

                        if let Some(symbol_book) = order_book {
                            if let Some(book_level) = symbol_book.get(&0) {
                                let message = PriceServiceResponse::MarketPrice(Some(book_level.price));
                                if let Err(_e) = callback_sender.send(message) {
                                    eprintln!("Market Price Service: Failed to send response");
                                }
                            }
                        } else if let Some(last_price) = last_price.get(&symbol_name) {
                            let message = PriceServiceResponse::MarketPrice(Some(last_price.clone()));
                            if let Err(_e) = callback_sender.send(message) {
                                eprintln!("Market Price Service: Failed to send response");
                            }
                        }
                        else {
                            let message = PriceServiceResponse::MarketPrice(None);
                            if let Err(_e) = callback_sender.send(message) {
                                eprintln!("Market Price Service: Failed to send response");
                            }
                        }
                    }
                }
            }
        }
    }
}


