use std::time::Duration;
use iso_currency::Currency;
use crate::standardized_types::subscriptions::Symbol;

/// Represents a futures product, providing essential properties specific to futures contracts.
///
/// This trait defines methods for accessing key futures contract properties, such as the time remaining until expiration.
/// Understanding the time to expiration is crucial for managing futures positions, as it affects both the pricing and the strategy for holding or closing a position.
pub trait FuturesProduct {
    /// Returns the time remaining until the futures contract expires.
    ///
    /// This duration is critical for traders and algorithms that need to manage the rollover of positions,
    /// assess the time value remaining in a contract, or avoid the potential delivery or settlement at expiration.
    ///
    /// # Returns
    /// A `Duration` representing the time remaining until the contract's expiration.
    fn time_to_expiration(&self) -> Duration;

    /// Returns the time remaining until the futures contract expires.
    ///
    /// The Tick value is used to determine the cash value of each tick. 
    /// eg: NQ tick value is 20 USD
    ///
    /// # Returns
    /// A `f64` value of 1 tick in underlying currency.
    fn tick_value(&self) -> f64;
    
    /// The size of 1 tick.
    /// The tick size is used to calculate the number of ticks etc.
    /// Eg: NQ TickSize = 0.25
    fn tick_size(&self) -> f64;

    /// Returns the base currency of the Forex instrument.
    ///
    /// The base currency is the first currency in a currency pair quotation, against which the second currency (quote currency) is measured.
    /// For example, in the EUR/USD currency pair, EUR is the base currency.
    ///
    /// # Returns
    /// A `Currency` representing the base currency of the instrument.
    fn currency_underlying(&self) -> Currency;
}

/// Represents a Forex (foreign exchange) product, providing essential Forex-specific properties.
///
/// This trait defines methods for accessing properties unique to Forex instruments, such as pip location,
/// which is crucial for calculating price movements and trading strategies in the Forex market.
pub trait ForexProduct {
    /// Returns the pip location for the Forex instrument.
    ///
    /// The pip location indicates the decimal place of the pip value in the instrument's price quote.
    /// This is essential for calculating price movements, as a pip is the smallest price move that a given
    /// exchange rate can make based on market convention.
    ///
    /// # Returns
    /// A `u32` representing the pip location in the instrument's price quote.
    fn pip_location(&self) -> u32;
    
    /// Returns the base currency of the Forex instrument.
    ///
    /// The base currency is the first currency in a currency pair quotation, against which the second currency (quote currency) is measured.
    /// For example, in the EUR/USD currency pair, EUR is the base currency.
    ///
    /// # Returns
    /// A `Currency` representing the base currency of the instrument.
    fn currency_base(&self) -> Currency; 
    
    /// Returns the quote currency of the Forex instrument.
    ///
    /// The quote currency is the second currency in a currency pair quotation, which is used to determine the value of the base currency.
    /// For example, in the EUR/USD currency pair, USD is the quote currency.
    ///
    /// # Returns
    /// A `Currency` representing the quote currency of the instrument.
    fn currency_quote(&self) -> Currency;
}


/// Represents a Cfd product, providing essential Cfd-specific properties.
/// 
/// These methods are used to access properties unique to Cfd instruments, such as the underlying currency
 pub trait CfdProduct {
    /// Returns the quote currency of the CFD instrument.
    ///
    /// This is the currency in which the price of the CFD is denominated. Understanding the quote currency is essential for evaluating the monetary value of trades and positions within the CFD market.
    ///
    /// # Returns
    /// A `Currency` representing the quote currency of the instrument.
    fn currency_quoted(&self) -> Currency;
}

/// Represents a product that can be displayed or viewed, providing essential formatting details.
///
/// This trait defines methods for accessing vendor-related properties of a financial instrument,
/// such as its decimal precision and symbol representation. These properties are crucial for the fund forge infrastructure.
pub trait ViewableProduct {
    /// Returns the number of decimal places used for the instrument's value.
    ///
    /// This method is important for formatting the instrument's value correctly, ensuring
    /// that it is displayed with the appropriate precision.
    ///
    /// # Returns
    /// A `usize` indicating the number of decimal places.
    fn decimal_precision(&self) -> usize;

    /// Returns the symbol of the instrument in a standardized format.
    ///
    /// This method provides a uniform representation of the instrument's symbol, facilitating
    /// its identification and use across different parts of the application or system.
    ///
    /// # Returns
    /// A `Symbol` representing the instrument's standardized symbol.
    fn symbol(&self) -> Symbol;

    /// Returns the symbol of the instrument in the vendor-specific format.
    ///
    /// This method provides the instrument's symbol as recognized by the data vendor or brokerage,
    /// which may differ from its standardized or commonly used symbol. This is particularly useful
    /// for operations involving external systems or APIs that require the vendor-specific notation.
    ///
    /// # Returns
    /// A `String` representing the instrument's vendor-specific symbol.
    fn symbol_vendor_format(&self) -> String;
}

/// Specifies the required parameters for a tradeable instrument.
/// These get are used by the engine to create order requests for the instrument.
pub trait TradableProduct {
    /// Returns the min tradeable amount for the instrument.
    /// If the min tradeable amount is not know consider just returning 1 or 0.xxx minimum decimal helpers unit
    fn min_tradeable_amount(&self) -> u64;
    
    /// Returns the maximum amount that can be traded for the instrument.
    /// If no maximum amount is specified with the brokerage, specify f64::MAX.
    fn max_tradeable_amount(&self) -> u64;
    
    /// Rounds the trade size to the nearest multiple of the brokerage's minimum tradeable amount.
    fn round_trade_size(&self, original_size: u64) -> u64;

    /// `margin_rate()` fn is crucial for determining how much a trader needs to deposit 
    /// to open a position with that particular instrument. If the instrument has a `margin_rate()` 
    /// of 0.02 (or 2%), and a trader wants to open a position that's nominally valued at $50,000, 
    /// the calculation for the required margin would be:
    ///
    /// Required Margin = $50,000 × 0.02 = $1,000
    ///
    /// This means the trader needs to have at least $1,000 in their trading account to open this position.
    /// If the brokerage doesn't have a `margin_rate()` function, this function should return 1.0 to represent required cash is 100% of the value of the instrument.
    fn margin_rate(&self) -> f64;
    
    /// Returns the required margin for the instrument holding the specified quantity overnight.
    /// if the brokerage doesn't have a `overnight_margin_requirement()` function, this function should return the quantity * price.
    /// 
    /// # Returns
    /// `f64` The margin requirement in the underlying market currency.
    fn overnight_margin_requirement(&self, quantity: u64, entry_price: f64) -> f64;
    
    /// Returns the required margin for the instrument holding the specified quantity during trading hours.
    /// if the brokerage doesn't have a `overnight_margin_requirement()` function, this function should return the quantity * price.
    /// # Returns
    /// `f64` The margin requirement in the underlying market currency.
    fn day_margin_requirement(&self, quantity: u64, entry_price: f64) -> f64;

    /// Returns the required margin for the instrument holding the specified quantity.
    /// If the brokerage doesn't have a `overnight_margin_requirement()` function, this function should return the quantity * price.
    /// # Returns
    /// `f64` The margin requirement in the underlying market currency.
    fn margin_requirement(&self, quantity: u64, entry_price: f64) -> f64;
}
