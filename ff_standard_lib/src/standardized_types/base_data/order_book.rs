use crate::standardized_types::base_data::quote::BookLevel;
use crate::standardized_types::subscriptions::Symbol;
use crate::standardized_types::{Price, TimeString};
use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use chrono_tz::Tz;
use futures::future::join_all;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct LevelTwoSubscription {
    pub symbol: Symbol,
    pub time: TimeString,
}

impl LevelTwoSubscription {
    pub fn new(symbol: Symbol, time: TimeString) -> Self {
        LevelTwoSubscription { symbol, time }
    }
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct OrderBookUpdate {
    pub symbol: Symbol,
    pub bid: BTreeMap<BookLevel, Price>,
    pub ask: BTreeMap<BookLevel, Price>,
    time: TimeString,
}

impl OrderBookUpdate {
    pub fn new(
        symbol: Symbol,
        bid: BTreeMap<BookLevel, Price>,
        ask: BTreeMap<BookLevel, Price>,
        time: DateTime<Utc>,
    ) -> Self {
        OrderBookUpdate {
            symbol,
            bid,
            ask,
            time: time.to_string(),
        }
    }

    pub fn time_utc(&self) -> DateTime<chrono::Utc> {
        DateTime::from_str(&self.time).unwrap()
    }

    pub fn time_local(&self, time_zone: &Tz) -> DateTime<Tz> {
        time_zone.from_utc_datetime(&self.time_utc().naive_utc())
    }
}

pub struct OrderBook {
    symbol: Symbol,
    bid: Arc<RwLock<BTreeMap<BookLevel, Price>>>, //make it a retain-able history
    ask: Arc<RwLock<BTreeMap<BookLevel, Price>>>,
    time: Arc<RwLock<DateTime<Utc>>>,
}

impl OrderBook {
    /// Create a new `Price` instance.
    /// # Parameters
    pub fn new(symbol: Symbol, time: DateTime<Utc>) -> Self {
        OrderBook {
            symbol,
            bid: Default::default(),
            ask: Default::default(),
            time: Arc::new(RwLock::new(time)),
        }
    }

    pub async fn update(&self, updates: OrderBookUpdate) {
        if updates.symbol != self.symbol {
            return;
        }
        // Clone necessary fields before moving into the async block
        let updates_bid = updates.bid.clone();
        let updates_ask = updates.ask.clone();
        let updates_time_utc = updates.time_utc();

        let t1 = task::spawn({
            let bid_book = Arc::clone(&self.bid);
            async move {
                let mut bid_book = bid_book.write().await;
                for (level, bid) in updates_bid {
                    bid_book.insert(level, bid);
                }
            }
        });

        let t2 = task::spawn({
            let ask_book = Arc::clone(&self.ask);
            async move {
                let mut ask_book = ask_book.write().await;
                for (level, ask) in updates_ask {
                    ask_book.insert(level, ask);
                }
            }
        });

        let t3 = task::spawn({
            let time = Arc::clone(&self.time);
            async move {
                *time.write().await = updates_time_utc;
            }
        });

        // Wait for all tasks to complete
        join_all(vec![t1, t2, t3]).await;
    }

    pub async fn time_utc(&self) -> DateTime<chrono::Utc> {
        self.time.read().await.clone()
    }

    pub async fn time_local(&self, time_zone: &Tz) -> DateTime<Tz> {
        time_zone.from_utc_datetime(&self.time_utc().await.naive_utc())
    }

    pub async fn ask_level(&self, level: BookLevel) -> Option<Price> {
        self.ask.read().await.get(&level).cloned()
    }

    pub async fn bid_level(&self, level: BookLevel) -> Option<Price> {
        self.bid.read().await.get(&level).cloned()
    }

    pub fn ask_book(&self) -> Arc<RwLock<BTreeMap<BookLevel, Price>>> {
        self.ask.clone()
    }

    pub fn bid_book(&self) -> Arc<RwLock<BTreeMap<BookLevel, Price>>> {
        self.bid.clone()
    }

    pub async fn ask_snapshot(&self) -> BTreeMap<BookLevel, Price> {
        self.ask.read().await.clone()
    }

    pub async fn bid_snapshot(&self) -> BTreeMap<BookLevel, Price> {
        self.bid.read().await.clone()
    }
}
