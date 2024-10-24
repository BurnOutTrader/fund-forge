use std::str::FromStr;
use chrono::{DateTime, Utc};
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use rust_decimal::Decimal;
use crate::standardized_types::accounts::Currency;
use crate::standardized_types::new_types::Price;
use crate::standardized_types::subscriptions::{SymbolCode, SymbolName};
use serde_derive::{Deserialize, Serialize};
use crate::standardized_types::enums::FuturesExchange;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug, PartialEq, Serialize, Deserialize, PartialOrd,)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct SymbolInfo {
    pub symbol_name: SymbolName,
    pub pnl_currency: Currency,
    pub value_per_tick: Price,
    pub tick_size: Price,
    pub decimal_accuracy: u32
}

impl SymbolInfo {
    pub fn new(
        symbol_name: SymbolName,
       pnl_currency: Currency,
       value_per_tick: Price,
       tick_size: Price,
        decimal_accuracy: u32
    ) -> Self {
        Self {
            symbol_name,
            pnl_currency,
            value_per_tick,
            tick_size,
            decimal_accuracy,
        }
    }
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug, PartialEq, Serialize, Deserialize, PartialOrd,)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct CommissionInfo {
    pub per_side: Decimal,
    pub currency: Currency,
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug, PartialEq, Serialize, Deserialize, PartialOrd)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct SessionMarketHours {
    pub has_close: bool,
    pub is_24_hour: bool,
    pub is_closed: bool,
    pub open_time_utc_string: Option<String>,
    pub close_time_utc_string: Option<String>,
}

impl SessionMarketHours {
    pub fn close_time_utc(&self) -> Option<DateTime<Utc>> {
        if let Some(time_string) = &self.close_time_utc_string {
            return Some(DateTime::from_str(time_string).unwrap())
        }
        None
    }

    pub fn open_time_utc(&self) -> Option<DateTime<Utc>> {
        if let Some(time_string) = &self.open_time_utc_string {
            return Some(DateTime::from_str(time_string).unwrap())
        }
        None
    }
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug, PartialEq, Serialize, Deserialize, PartialOrd)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct FrontMonthInfo {
    pub exchange: FuturesExchange,
    pub symbol_name: SymbolName,
    pub symbol_code: SymbolCode
}
