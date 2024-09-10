pub mod accounts;
pub mod base_data;
pub mod data_server_messaging;
pub mod enums;
pub mod indicators;
pub mod orders;
pub mod rolling_window;
pub mod strategy_events;
pub mod subscription_handler;
pub mod subscriptions;
pub mod time_slices;

pub type OwnerId = String;
pub type TimeString = String;

pub type Price = f64;

use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Deserialize, Serialize)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
}
impl Color {
    pub fn new(red: u8, green: u8, blue: u8) -> Self {
        Color { red, green, blue }
    }

    pub fn into_tuple(&self) -> (u8, u8, u8) {
        (self.red.clone(), self.green.clone(), self.blue.clone())
    }
}

pub type TimeStamp = i64;
