use serde::Deserialize;
use serde::Serialize;
use crate::oanda_api::models::account::enums::GuaranteedStopLossOrderMutability;

/// Represents the current mutability and hedging settings related to guaranteed Stop Loss orders.
#[derive(Debug, Serialize, Deserialize)]
pub struct GuaranteedStopLossOrderParameters {
    /// The current guaranteed Stop Loss Order mutability setting of the Account when the market is open.
    #[serde(rename = "mutabilityMarketOpen")]
    mutability_market_open: GuaranteedStopLossOrderMutability,

    /// The current guaranteed Stop Loss Order mutability setting of the Account when the market is halted.
    #[serde(rename = "mutabilityMarketHalted")]
    mutability_market_halted: GuaranteedStopLossOrderMutability,
}

