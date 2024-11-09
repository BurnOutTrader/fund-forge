use rust_decimal::prelude::{FromPrimitive, ToPrimitive, Zero};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::standardized_types::accounts::Currency;
use crate::standardized_types::enums::{PositionSide};
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::symbol_info::SymbolInfo;

pub fn calculate_theoretical_pnl(
    side: PositionSide,
    entry_price: Price,
    market_price: Price,
    quantity: Volume,
    symbol_info: &SymbolInfo,
    exchange_rate_multiplier: Decimal,
    account_currency: Currency,
) -> Price {
    let raw_ticks = match side {
        PositionSide::Long => {
            let ticks = ((market_price - entry_price) / symbol_info.tick_size).round_dp(symbol_info.decimal_accuracy);
            if ticks == dec!(0.0) {
                return dec!(0.0)
            }
            ticks
        },
        PositionSide::Short => {
            let ticks = ((entry_price - market_price) / symbol_info.tick_size).round_dp(symbol_info.decimal_accuracy);
            if ticks == dec!(0.0) {
                return dec!(0.0)
            }
            ticks
        },
    };

    let pnl = raw_ticks * symbol_info.value_per_tick * quantity;

    // Convert PnL to account currency
    if let Some(base_curr) = symbol_info.base_currency {
        if account_currency == base_curr {
            // Case 1: Account currency is base currency
            // Example: AUD account trading AUD/JPY
            pnl / entry_price
        } else if account_currency == symbol_info.pnl_currency {
            // Case 2: Account currency is quote/pnl currency
            // Example: JPY account trading AUD/JPY
            pnl
        } else {
            // Case 3: Account currency is neither
            // Example: EUR account trading AUD/JPY
            pnl / exchange_rate_multiplier
        }
    } else {
        // Not a currency pair
        pnl * exchange_rate_multiplier
    }
}

/// Safely divides two f64 values using Decimal for precision.
/// Panics if the divisor is zero or if conversion fails.
pub fn divide_f64(dividend: f64, divisor: f64) -> f64 {
    if divisor == 0.0 {
        panic!("attempt to divide by zero");
    }
    let decimal_dividend = Decimal::from_f64(dividend).expect("Invalid dividend");
    let decimal_divisor = Decimal::from_f64(divisor).expect("Invalid divisor");

    let result = decimal_dividend / decimal_divisor;
    result.to_f64().expect("Error converting result to f64")
}

pub fn round_to_decimals(value: Decimal, decimals: u32) -> Price {
    // Create a factor of 10^decimals using Decimal
    let factor = Decimal::from_i64(10_i64.pow(decimals)).unwrap();

    // Perform the rounding operation
    (value * factor).round() / factor
}

pub fn round_to_tick_size(value: Decimal, tick_size: Decimal) -> Decimal {
    // Divide the value by the tick size, then round to the nearest integer
    let ticks = (value / tick_size).round();

    // Multiply the rounded number of ticks by the tick size to get the final rounded value
    ticks * tick_size
}

/// Calculates the average of a vector of floating-point numbers using Decimal for high precision.
/// Skips NaN values and entries associated with zero quantity if applicable.
pub fn average_of_f64(values: &Vec<f64>) -> f64 {
    let mut total = Decimal::zero();
    let mut count = 0;

    for &price in values.iter() {
        // Here's an assumption that price should be skipped if it's NaN or somehow erroneous
        if price.is_nan() {
            continue; // Skip NaN values
        }
        // Convert float to Decimal, using zero as a fallback for conversion failure
        let decimal_price = Decimal::from_f64(price).unwrap();
        total += decimal_price;
        count += 1;
    }

    // Use the safe_divide function to perform division
    divide_decimal_by_usize(total, count)
}

/// Safely divides a Decimal by a usize, returning the result as a f64.
/// Panics if the denominator is zero.
pub fn divide_decimal_by_usize(numerator: Decimal, denominator: usize) -> f64 {
    if denominator == 0 {
        panic!("attempt to divide by zero");
    }
    let denominator_decimal = Decimal::from(denominator);
    let result = numerator / denominator_decimal;
    result.to_f64().unwrap_or_default() // Using default for f64 which is 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use float_cmp::approx_eq;
    use rust_decimal::Decimal;

    #[test]
    fn test_calculate_average_high_precision() {
        let prices = vec![0.00000001, 0.00000002, 0.00000003, 0.00000004];
        let average = average_of_f64(&prices);
        assert!(approx_eq!(
            f64,
            average,
            0.000000025,
            epsilon = 0.0000000001
        ));

        let prices_high_range = vec![100000.0, 200000.0, 300000.0];
        let average = average_of_f64(&prices_high_range);
        assert!(approx_eq!(f64, average, 200000.0, epsilon = 0.0001));
    }

    #[test]
    fn test_calculate_average_with_negative_values() {
        let prices = vec![-10.0, 20.0, -30.0, 40.0];
        let average = average_of_f64(&prices);
        assert!(approx_eq!(f64, average, 5.0, epsilon = 0.0001));
    }

    #[test]
    #[should_panic(expected = "attempt to divide by zero")]
    fn test_divide_decimal_by_usize_zero_denominator() {
        let numerator = Decimal::from(10);
        divide_decimal_by_usize(numerator, 0);
    }

    #[test]
    fn test_divide_decimal_by_usize_normal() {
        let numerator = Decimal::from_f64(50.0).unwrap();
        let result = divide_decimal_by_usize(numerator, 2);
        assert!(approx_eq!(f64, result, 25.0, epsilon = 0.00000001));
    }

    #[test]
    #[should_panic(expected = "attempt to divide by zero")]
    fn test_divide_f64_zero_divisor() {
        divide_f64(50.0, 0.0);
    }

    #[test]
    fn test_divide_f64_normal() {
        let result = divide_f64(50.0, 2.0);
        assert!(approx_eq!(f64, result, 25.0, epsilon = 0.00000001));
    }

    #[test]
    fn test_divide_f64_negative() {
        let result = divide_f64(-100.0, 4.0);
        assert!(approx_eq!(f64, result, -25.0, epsilon = 0.00000001));
    }
}
