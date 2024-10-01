use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::standardized_types::accounts::ledgers::Currency;
use crate::standardized_types::new_types::Price;
use crate::standardized_types::subscriptions::SymbolName;
use serde_derive::{Deserialize, Serialize};
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug, PartialEq, Serialize, Deserialize, PartialOrd,)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct SymbolInfo {
    pub symbol_name: SymbolName,
    pub pnl_currency: Currency,
    pub value_per_tick: Price,
    pub tick_size: Price
}

impl SymbolInfo {
    pub fn new(symbol_name: SymbolName,
               pnl_currency: Currency,
               value_per_tick: Price,
               tick_size: Price) -> Self {
        Self {
            symbol_name,
            pnl_currency,
            value_per_tick,
            tick_size
        }
    }
}