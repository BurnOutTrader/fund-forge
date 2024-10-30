use rust_decimal::Decimal;
use serde::{Serialize, Deserialize, Serializer};
use ff_standard_lib::standardized_types::accounts::Currency;
use crate::oanda_api::models::instruments::CandlestickGranularity;
use crate::oanda_api::models::order::order_related::UnitsAvailable;
use crate::oanda_api::models::pricing_common::PriceBucket;
use crate::oanda_api::models::primitives::{DateTime, InstrumentName, PricingComponent};


/// A struct representing a Client-specific Price.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientPrice {
    /// The string “PRICE”. Used to identify the a Price object when found in a stream.
    #[serde(rename = "type")]
    pub type_of: String,

    /// The Price’s Instrument.
    pub instrument: InstrumentName,

    /// The date/time when the Price was created
    pub time: DateTime,

    /// The status of the Price.
    /// Deprecated: Will be removed in a future API update.
    pub status: Option<PriceStatus>,

    /// Flag indicating if the Price is tradeable or not
    pub tradeable: bool,

    /// The list of prices and liquidity available on the Instrument’s bid side.
    pub bids: Vec<PriceBucket>,

    /// The list of prices and liquidity available on the Instrument’s ask side.
    pub asks: Vec<PriceBucket>,

    /// The closeout bid Price.
    #[serde(rename = "closeoutBid")]
    pub closeout_bid: Decimal,

    /// The closeout ask Price.
    #[serde(rename = "closeoutAsk")]
    pub closeout_ask: Decimal,

    /// The factors used to convert quantities of this prices’s Instrument’s quote
    /// currency into a quantity of the Account’s home currency.
    /// Deprecated: Will be removed in a future API update.
    #[serde(rename = "quoteHomeConversionFactors")]
    pub quote_home_conversion_factors: Option<QuoteHomeConversionFactors>,

    /// Representation of how many units of an Instrument are available to be
    /// traded by an Order depending on its positionFill option.
    /// Deprecated: Will be removed in a future API update.
    #[serde(rename = "unitsAvailable")]
    pub units_available: Option<UnitsAvailable>,
}



/// Enumeration representing the status of the Price.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PriceStatus {
    /// The Instrument’s prices is tradeable.
    #[serde(rename = "tradeable")]
    Tradeable,

    /// The Instrument’s prices is not tradeable.
    #[serde(rename = "non-tradable")]
    NonTradeable,

    /// The Instrument of the prices is invalid or there is no valid Price for the Instrument.
    #[serde(rename = "invalid")]
    Invalid,
}


/// Struct for QuoteHomeConversionFactors
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QuoteHomeConversionFactors {
    /// The factor used to convert a positive amount of the Price’s Instrument’s
    /// quote currency into a positive amount of the Account’s home currency.
    /// Conversion is performed by multiplying the quote units by the conversion
    /// factor.
    #[serde(rename = "positiveUnits")]
    positive_units: Decimal,

    /// The factor used to convert a negative amount of the Price’s Instrument’s
    /// quote currency into a negative amount of the Account’s home currency.
    /// Conversion is performed by multiplying the quote units by the conversion
    /// factor.
    #[serde(rename = "negativeUnits")]
    negative_units: Decimal,
}

/// Represents the factors to use to convert quantities of a given currency into the Account’s home currency.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HomeConversions {
    /// The currency to be converted into the home currency.
    currency: Currency,

    /// The factor used to convert any gains for an Account in the specified
    /// currency into the Account’s home currency. This would include positive
    /// realized P/L and positive financing amounts. Conversion is performed by
    /// multiplying the positive P/L by the conversion factor.
    #[serde(rename = "accountGain")]
    account_gain: Decimal,

    /// The factor used to convert any losses for an Account in the specified
    /// currency into the Account’s home currency. This would include negative
    /// realized P/L and negative financing amounts. Conversion is performed by
    /// multiplying the positive P/L by the conversion factor.
    #[serde(rename = "accountLoss")]
    account_loss: Decimal,

    /// The factor used to convert a Position or Trade Value in the specified
    /// currency into the Account’s home currency. Conversion is performed by
    /// multiplying the Position or Trade Value by the conversion factor.
    #[serde(rename = "positionValue")]
    position_value: Decimal,
}

/// Represents a PricingHeartbeat object in the Pricing stream.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PricingHeartbeat {
    /// The type of the object, defaulting to "HEARTBEAT"
    #[serde(rename = "type")]
    pub type_of: String, //

    /// The date/time when the Heartbeat was created.
    pub time: DateTime,
}

/// Represents a candlestick data specification.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct CandleSpecification {
    instrument_name: InstrumentName,
    candlestick_granularity: CandlestickGranularity,
    pricing_component: PricingComponent,
}

impl CandleSpecification {
    #[allow(dead_code)]
    pub fn new(instrument_name: InstrumentName, candlestick_granularity: CandlestickGranularity, pricing_component: PricingComponent) -> Self {
        Self {
            instrument_name,
            candlestick_granularity,
            pricing_component,
        }
    }
}
impl ToString for CandleSpecification {
    fn to_string(&self) -> String {
        format!("{:?}:{:?}:{:?}", self.instrument_name, self.candlestick_granularity, self.pricing_component)
    }
}

/// Custom serialization for CandleSpecification.
impl Serialize for CandleSpecification {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let spec = format!(
            "{:?}:{:?}:{:?}",
            self.instrument_name,
            self.candlestick_granularity,
            self.pricing_component
        );
        serializer.serialize_str(&spec)
    }
}
