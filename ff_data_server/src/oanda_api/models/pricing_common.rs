use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// A PriceBucket represents a prices available for an amount of liquidity.
///
/// This is an application/json object with the following Schema.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PriceBucket {
    /// The Price offered by the PriceBucket.
    pub price: Decimal,

    /// The amount of liquidity offered by the PriceBucket.
    ///
    /// This can be an integer or a decimal. The field is represented
    /// using Rust's Decimal type for precision.
    pub liquidity: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct PriceStreamResponse {
    #[serde(default)]
    pub asks: Vec<PriceBucket>,
    #[serde(default)]
    pub bids: Vec<PriceBucket>,
    #[serde(rename = "closeoutAsk", default)]
    pub closeout_ask: String,
    #[serde(rename = "closeoutBid", default)]
    pub closeout_bid: String,
    pub instrument: String,
    pub time: String,
    #[serde(default)]
    pub status: String,
    #[serde(rename = "type", default)]
    pub r#type: Option<String>,  // For heartbeat messages
}