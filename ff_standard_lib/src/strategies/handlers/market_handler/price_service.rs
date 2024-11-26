use std::collections::BTreeMap;
use std::sync::Arc;
use dashmap::DashMap;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::books::BookLevel;
use crate::standardized_types::enums::OrderSide;
use crate::standardized_types::new_types::Price;
use crate::standardized_types::subscriptions::{SymbolCode, SymbolName};
use crate::standardized_types::time_slices::TimeSlice;
use crate::standardized_types::base_data::tick::Aggressor;

pub struct MarketPriceService {
    bid_books: DashMap<SymbolName, BTreeMap<u16, BookLevel>>,
    ask_books: DashMap<SymbolName, BTreeMap<u16, BookLevel>>,
    has_quotes: DashMap<SymbolName, bool>,
    last_price: DashMap<SymbolName, Price>,
}

impl MarketPriceService {
    pub fn new() -> Self {
        MarketPriceService {
            bid_books: DashMap::new(),
            ask_books: DashMap::new(),
            has_quotes: DashMap::new(),
            last_price: DashMap::new(),
        }
    }

    pub fn update_market_data(&self, time_slice: Arc<TimeSlice>) {
        for base_data in time_slice.iter() {
            match base_data {
                BaseDataEnum::Candle(candle) => {
                    self.last_price.insert(candle.symbol.name.clone(), candle.close);
                }
                BaseDataEnum::QuoteBar(quotebar) => {
                    let symbol_name = &quotebar.symbol.name;
                    if self.has_quotes.contains_key(symbol_name) {
                        continue;
                    }

                    let mut bid_book = self.bid_books.entry(symbol_name.clone()).or_insert_with(BTreeMap::new);
                    let mut ask_book = self.ask_books.entry(symbol_name.clone()).or_insert_with(BTreeMap::new);

                    bid_book.insert(0, BookLevel::new(0, quotebar.bid_close, dec!(0.0)));
                    ask_book.insert(0, BookLevel::new(0, quotebar.ask_close, dec!(0.0)));
                }
                BaseDataEnum::Tick(tick) => {
                    let symbol_name = &tick.symbol.name;
                    self.last_price.insert(symbol_name.clone(), tick.price);

                    if tick.aggressor != Aggressor::None && !self.has_quotes.contains_key(symbol_name) {
                        let mut bid_book = self.bid_books.entry(symbol_name.clone()).or_insert_with(BTreeMap::new);
                        let mut ask_book = self.ask_books.entry(symbol_name.clone()).or_insert_with(BTreeMap::new);

                        match tick.aggressor {
                            Aggressor::Buy => ask_book.insert(0, BookLevel::new(0, tick.price, dec!(0.0))),
                            Aggressor::Sell => bid_book.insert(0, BookLevel::new(0, tick.price, dec!(0.0))),
                            _ => None,
                        };
                    }
                }
                BaseDataEnum::Quote(quote) => {
                    let symbol_name = &quote.symbol.name;
                    if !self.has_quotes.contains_key(symbol_name) {
                        self.has_quotes.insert(symbol_name.clone(), true);
                        let mut bid_book = self.bid_books.entry(symbol_name.clone()).or_insert_with(BTreeMap::new);
                        let mut ask_book = self.ask_books.entry(symbol_name.clone()).or_insert_with(BTreeMap::new);

                        bid_book.insert(0, BookLevel::new(0, quote.bid, quote.bid_volume));
                        ask_book.insert(0, BookLevel::new(0, quote.ask, quote.ask_volume));
                    }
                }
                _ => eprintln!("Market Price Service: Incorrect data type in Market Updates: {}", base_data.base_data_type())
            }
        }
    }

    pub fn get_market_price(&self, order_side: OrderSide, symbol_name: &SymbolName, symbol_code: &SymbolCode) -> Option<Decimal> {
        let order_book = match order_side {
            OrderSide::Buy => self.ask_books.get(symbol_code).or_else(|| self.ask_books.get(symbol_name)),
            OrderSide::Sell => self.bid_books.get(symbol_code).or_else(|| self.bid_books.get(symbol_name)),
        };

        if let Some(symbol_book) = order_book {
            symbol_book.get(&0).map(|level| level.price.clone())
        } else {
            if let Some(value) = self.last_price.get(symbol_name) {
                Some(value.clone())
            } else {
                None
            }
        }
    }

    pub fn estimate_fill_price(&self, order_side: OrderSide, symbol_name: &SymbolName, symbol_code: &SymbolCode, volume: Decimal) -> Option<Decimal> {
        let order_book = match order_side {
            OrderSide::Buy => self.ask_books.get(symbol_code).or_else(|| self.ask_books.get(symbol_name)),
            OrderSide::Sell => self.bid_books.get(symbol_code).or_else(|| self.bid_books.get(symbol_name)),
        };

        if let Some(book) = order_book {
            let mut total_price_volume = dec!(0.0);
            let mut total_volume_filled = dec!(0.0);
            let mut remaining_volume = volume;

            for level in 0.. {
                if let Some(book_level) = book.get(&level) {
                    if book_level.volume == dec!(0.0) && total_volume_filled == dec!(0.0) {
                        return Some(book_level.price.clone());
                    }
                    if book_level.volume == dec!(0.0) {
                        continue;
                    }

                    let volume_to_use = remaining_volume.min(book_level.volume);
                    total_price_volume += book_level.price * volume_to_use;
                    total_volume_filled += volume_to_use;
                    remaining_volume -= volume_to_use;

                    if remaining_volume == dec!(0.0) {
                        return Some(total_price_volume / total_volume_filled);
                    }
                } else {
                    break;
                }
            }

            if total_volume_filled > dec!(0.0) {
                return Some(total_price_volume / total_volume_filled);
            }
        }

        match self.last_price.get(symbol_name) {
            Some(price) => Some(price.clone()),
            None => None,
        }
    }

    pub fn estimate_limit_fill(&self, order_side: OrderSide, symbol_name: &SymbolName, symbol_code: &SymbolCode, volume: Decimal, limit: Decimal) -> Option<(Decimal, Decimal)> {
        let order_book = match order_side {
            OrderSide::Buy => self.ask_books.get(symbol_code).or_else(|| self.ask_books.get(symbol_name)),
            OrderSide::Sell => self.bid_books.get(symbol_code).or_else(|| self.bid_books.get(symbol_name)),
        };

        if let Some(book) = order_book {
            let mut total_price_volume = dec!(0.0);
            let mut total_volume_filled = dec!(0.0);
            let mut remaining_volume = volume;

            for level in 0.. {
                if let Some(book_level) = book.get(&level) {
                    if book_level.volume == dec!(0.0) && total_volume_filled == dec!(0.0) && level == 0 {
                        return Some((book_level.price.clone(), volume));
                    }
                    if book_level.volume == dec!(0.0) {
                        continue;
                    }

                    match order_side {
                        OrderSide::Buy if book_level.price > limit => break,
                        OrderSide::Sell if book_level.price < limit => break,
                        _ => {}
                    }

                    let volume_to_use = remaining_volume.min(book_level.volume);
                    total_price_volume += book_level.price * volume_to_use;
                    total_volume_filled += volume_to_use;
                    remaining_volume -= volume_to_use;

                    if remaining_volume == dec!(0.0) {
                        return Some((total_price_volume / total_volume_filled, total_volume_filled));
                    }
                } else {
                    break;
                }
            }

            if total_volume_filled > dec!(0.0) {
                return Some((total_price_volume / total_volume_filled, total_volume_filled));
            }
        }

        if let Some(price) = self.last_price.get(symbol_name) {
            Some((price.clone(), volume))
        } else {
            None
        }
    }
}

