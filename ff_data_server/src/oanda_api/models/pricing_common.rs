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