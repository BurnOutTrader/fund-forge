use std::fmt;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use rust_decimal::Decimal;
use serde_derive::{Deserialize, Serialize};
use chrono_tz::Tz;
use crate::standardized_types::broker_enum::Brokerage;
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::position::Position;
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct AccountInfo {
    pub account_id: AccountId,
    pub brokerage: Brokerage,
    pub cash_value: Price,
    pub cash_available: Price,
    pub currency: Currency,
    pub open_pnl: Decimal,
    pub booked_pnl: Decimal,
    pub day_open_pnl: Decimal,
    pub day_booked_pnl: Decimal,
    pub cash_used: Price,
    pub positions: Vec<Position>,
    pub is_hedging: bool,
    pub leverage: u32,
    pub buy_limit: Option<Volume>,
    pub sell_limit: Option<Volume>,
    pub max_orders: Option<Volume>,
    pub daily_max_loss: Option<Price>,
    pub daily_max_loss_reset_time: Option<String>
}

pub type AccountId = String;
pub type AccountName = String;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Serialize, Deserialize, PartialOrd, Eq, Ord, Hash)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct Account {
    pub brokerage: Brokerage,
    pub account_id: AccountId
}
impl Account {
    pub fn new(brokerage: Brokerage, account_id: AccountId) -> Self {
        Account {
            brokerage,
            account_id
        }
    }
}
// Implement Display for Account
impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - {}", self.brokerage, self.account_id)
    }
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Serialize, Deserialize, PartialOrd, Copy)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum Currency {
    AUD,
    USD,
    CAD,
    EUR,
    JPY,
    USDT
}

impl Currency {
    pub fn from_str(string: &str) -> Self {
        match string {
            "AUD" => Currency::AUD,
            "USD" => Currency::USD,
            "CAD" => Currency::CAD,
            "EUR" => Currency::EUR,
            "JPY" => Currency::JPY,
            "USDT" => Currency::USDT,
            _ => panic!("No currency matching string, please implement")
        }
    }
}

pub struct AccountSetup {
    pub account_id: AccountId,
    pub brokerage: Brokerage,
    pub cash_value: Price,
    pub currency: Currency,
    pub size_limit: Option<Volume>,
    pub max_orders: Option<Volume>,
    pub daily_max_loss: Option<Price>,
    pub daily_max_loss_reset_hour: Option<u32>, //the hour of the day that the dail max loss resets
    pub daily_max_loss_reset_time_zone: Tz,
    pub leverage: Option<u32>
}