use crate::standardized_types::base_data::base_data_type::BaseDataType;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use strum_macros::Display;
use crate::standardized_types::resolution::Resolution;

// Enum for exchanges
#[derive(Serialize, Deserialize, Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialOrd, Eq, Ord, PartialEq, Copy, Debug, Display, Hash)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum Exchange {
    CBOT,
    CME,
    COMEX,
    NYMEX,
    MGEX,
    NYBOT
    // Add other exchanges if necessary
}
/// Used for internal ff calulcations
#[derive(Serialize, Deserialize, Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialOrd, Eq, Ord, PartialEq, Copy, Debug, Display, Hash)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum MarketType {
    Forex,
    CFD,
    Futures(Exchange),
    Equities,
    Crypto,
    ETF,
    Fundamentals,
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

