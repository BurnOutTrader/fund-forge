use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::standardized_types::new_types::{Price, Volume};

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
}