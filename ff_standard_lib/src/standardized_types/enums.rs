use crate::standardized_types::base_data::base_data_type::BaseDataType;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use rust_decimal::Decimal;
use strum_macros::Display;
use crate::helpers::decimal_calculators::round_to_tick_size;
use crate::standardized_types::resolution::Resolution;

// Enum for exchanges
#[derive(Serialize, Deserialize, Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialOrd, Eq, Ord, PartialEq, Copy, Debug, Display, Hash)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum FuturesExchange {
    CBOT,
    CME,
    COMEX,
    NYMEX,
    MGEX,
    NYBOT
    // Add other exchanges if necessary
}

impl FuturesExchange {
    pub fn from_string(s: &str) -> Result<Self, String> {
        match s.to_uppercase().as_str() {
            "CBOT" => Ok(FuturesExchange::CBOT),
            "CME" => Ok(FuturesExchange::CME),
            "COMEX" => Ok(FuturesExchange::COMEX),
            "NYMEX" => Ok(FuturesExchange::NYMEX),
            "MGEX" => Ok(FuturesExchange::MGEX),
            "NYBOT" => Ok(FuturesExchange::NYBOT),
            _ => Err(format!("Unknown exchange: {}", s)),
        }
    }
}

/// Used for internal ff calulcations
#[derive(Serialize, Deserialize, Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialOrd, Eq, Ord, PartialEq, Copy, Debug, Display, Hash)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum MarketType {
    Forex,
    CFD,
    Futures(FuturesExchange),
    Equities,
    Crypto,
    ETF,
    Fundamentals,
}

impl MarketType {
    pub fn round_price(&self, value: Decimal, tick_size: Decimal, decimal_accuracy: u32) -> Decimal {
        match self {
            MarketType::Forex => value.round_dp(decimal_accuracy),
            MarketType::CFD => value.round_dp(decimal_accuracy),
            MarketType::Futures(_) => round_to_tick_size(value, tick_size),
            MarketType::Equities => value.round_dp(decimal_accuracy),
            MarketType::Crypto => value.round_dp(decimal_accuracy),
            MarketType::ETF => value.round_dp(decimal_accuracy),
            MarketType::Fundamentals => value.round_dp(decimal_accuracy),
        }
    }
}

// Bias
#[derive(Serialize, Deserialize, Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialOrd, Eq, Ord, PartialEq, Copy, Debug, Display, Hash)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
/// A bias is a general direction of the market. It can be bullish, bearish, or neutral.
pub enum Bias {
    Bullish,
    Bearish,
    Neutral,
}

#[derive(Serialize, Deserialize, Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialOrd, Eq, Ord, PartialEq, Copy, Debug, Display, Hash)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
/// The `Side` enum is used to specify the side of a trade.
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Serialize, Deserialize, Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialOrd, Eq, Ord, PartialEq, Copy, Debug, Display, Hash)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum PositionSide {
    Long,
    Short,
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Copy)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum StrategyMode {
    Backtest,
    Live,
    LivePaperTrading,
}

#[derive(Serialize, Deserialize, Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialOrd, Eq, Ord, PartialEq, Copy, Debug, Hash)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct SubscriptionResolutionType {
    pub base_data_type: BaseDataType,
    pub resolution: Resolution,
}

impl SubscriptionResolutionType {
    pub fn new(resolution: Resolution, base_data_type: BaseDataType) -> Self {
        SubscriptionResolutionType {
            resolution,
            base_data_type,
        }
    }
}

