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
    pub base_currency: Option<Currency>,
    pub pnl_currency: Currency,
    pub value_per_tick: Price,
    pub tick_size: Price,
    pub decimal_accuracy: u32
}

impl SymbolInfo {
    pub fn new(
        symbol_name: SymbolName,
        base_currency: Option<Currency>,
       pnl_currency: Currency,
       value_per_tick: Price,
       tick_size: Price,
        decimal_accuracy: u32
    ) -> Self {
        Self {
            symbol_name,
            base_currency,
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
pub struct FrontMonthInfo {
    pub exchange: FuturesExchange,
    pub symbol_name: SymbolName,
    pub symbol_code: SymbolCode
}


impl FrontMonthInfo {
    pub fn new(
        exchange: FuturesExchange,
        symbol_name: SymbolName,
        symbol_code: SymbolCode
    ) -> Self {
        Self {
            exchange,
            symbol_name,
            symbol_code
        }
    }
}