use std::fmt;
use std::fmt::{Display, Formatter};
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use rust_decimal::Decimal;
use serde_derive::{Deserialize, Serialize};
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
    pub buy_limit: Option<Volume>,
    pub sell_limit: Option<Volume>,
    pub max_orders: Option<Volume>,
    pub daily_max_loss: Option<Price>,
    pub daily_max_loss_reset_time: Option<String>,
    pub leverage: u32
}

pub type AccountId = String;
pub type AccountName = String;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Serialize, Deserialize, PartialOrd, Eq, Ord, Hash)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct Account {
    pub brokerage: Brokerage,
    pub account_id: AccountId,
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

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Serialize, Deserialize, PartialOrd, Copy, Eq, Ord, Hash)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum Currency {
    AUD,
    USD,
    CAD,
    EUR,
    JPY,
    CHF,
    GBP,
    SEK,
    NOK,
    TRY,
    PLN,
    HUF,
    CZK,
    MXN,
    ZAR,
    HKD,
    SGD,
    NZD,
    CNH,
    BCH,
    BTC,
    ETH,
    LTC,
}

impl Currency {
    pub fn from_str(string: &str) -> Self {
        match string {
            "AUD" => Currency::AUD,
            "USD" => Currency::USD,
            "CAD" => Currency::CAD,
            "EUR" => Currency::EUR,
            "JPY" => Currency::JPY,
            "CHF" => Currency::CHF,
            "GBP" => Currency::GBP,
            "SEK" => Currency::SEK,
            "NOK" => Currency::NOK,
            "TRY" => Currency::TRY,
            "PLN" => Currency::PLN,
            "HUF" => Currency::HUF,
            "CZK" => Currency::CZK,
            "MXN" => Currency::MXN,
            "ZAR" => Currency::ZAR,
            "HKD" => Currency::HKD,
            "SGD" => Currency::SGD,
            "NZD" => Currency::NZD,
            "CNH" => Currency::CNH,
            _ => panic!("No currency matching string: {}", string),
        }
    }
}

impl Display for Currency {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Currency::AUD => "AUD",
            Currency::USD => "USD",
            Currency::CAD => "CAD",
            Currency::EUR => "EUR",
            Currency::JPY => "JPY",
            Currency::CHF => "CHF",
            Currency::GBP => "GBP",
            Currency::SEK => "SEK",
            Currency::NOK => "NOK",
            Currency::TRY => "TRY",
            Currency::PLN => "PLN",
            Currency::HUF => "HUF",
            Currency::CZK => "CZK",
            Currency::MXN => "MXN",
            Currency::ZAR => "ZAR",
            Currency::HKD => "HKD",
            Currency::SGD => "SGD",
            Currency::NZD => "NZD",
            Currency::CNH => "CNH",
            Currency::BCH => "BCH",
            Currency::BTC => "BTC",
            Currency::ETH => "ETH",
            Currency::LTC => "LTC",
        })
    }
}