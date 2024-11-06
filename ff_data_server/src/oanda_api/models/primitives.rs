use rust_decimal::Decimal;
use chrono::{NaiveDateTime, DateTime as ChronoDateTime, Utc};
use serde::{self, Deserialize, Deserializer, Serializer, Serialize, de};

/// A tag associated with an entity.
#[derive(Serialize, Deserialize, Debug)]
pub struct Tag {
    #[serde(rename = "type")]
    pub tag_type: String,
    pub name: String,
}

/// Instrument name identifier.
pub type InstrumentName = String;

/// The type of an Instrument.
#[derive(Serialize, Deserialize, Debug)]
pub enum InstrumentType {
    #[serde(rename = "CURRENCY")]
    Currency,
    #[serde(rename = "CFD")]
    Cfd,
    #[serde(rename = "METAL")]
    Metal,
}

/// Representation of the day of the week.
#[derive(Serialize, Deserialize, Debug)]
pub enum DayOfWeek {
    #[serde(rename = "SUNDAY")]
    Sunday,
    #[serde(rename = "MONDAY")]
    Monday,
    #[serde(rename = "TUESDAY")]
    Tuesday,
    #[serde(rename = "WEDNESDAY")]
    Wednesday,
    #[serde(rename = "THURSDAY")]
    Thursday,
    #[serde(rename = "FRIDAY")]
    Friday,
    #[serde(rename = "SATURDAY")]
    Saturday,
}

/// Financing day of the week with the number of days charged.
#[derive(Serialize, Deserialize, Debug)]
pub struct FinancingDayOfWeek {
    #[serde(rename = "dayOfWeek")]
    pub day_of_week: DayOfWeek,
    #[serde(rename = "daysCharged")]
    pub days_charged: i32,
}
/// Financing data for an instrument.
#[derive(Serialize, Deserialize, Debug)]
pub struct InstrumentFinancing {
    #[serde(rename = "longRate")]
    pub long_rate: Decimal,

    #[serde(rename = "shortRate")]
    pub short_rate: Decimal,

    #[serde(rename = "financingDaysOfWeek")]
    pub financing_days_of_week: Vec<FinancingDayOfWeek>,
}

/// Full specification of an Instrument.
#[derive(Serialize, Deserialize, Debug)]
pub struct Instrument {
    /// The name of the Instrument
    #[serde(rename = "name")]
    pub name: InstrumentName,

    /// The type of the Instrument
    #[serde(rename = "type")]
    pub type_of: InstrumentType,

    /// The display name of the Instrument
    #[serde(rename = "displayName")]
    pub display_name: String,

    /// The location of the “pip” for this instrument. The decimal position of
    /// the pip in this Instrument’s prices can be found at 10 ^ pipLocation
    /// (e.g. -4 pipLocation results in a decimal pip position of 10 ^ -4 = 0.0001).
    #[serde(rename = "pipLocation")]
    pub pip_location: i32,

    /// The number of decimal places that should be used to display prices for
    /// this instrument.
    #[serde(rename = "displayPrecision")]
    pub display_precision: i32,

    /// The amount of decimal places that may be provided when specifying the
    /// number of units traded for this instrument.
    #[serde(rename = "tradeUnitsPrecision")]
    pub trade_units_precision: i32,

    /// The smallest number of units allowed to be traded for this instrument.
    #[serde(rename = "minimumTradeSize")]
    pub minimum_trade_size: Decimal,

    /// The maximum trailing stop distance allowed for a trailing stop loss
    /// created for this instrument. Specified in prices units.
    #[serde(rename = "maximumTrailingStopDistance")]
    pub maximum_trailing_stop_distance: Decimal,

    /// The minimum distance allowed between the Trade’s fill prices and the
    /// configured prices for guaranteed Stop Loss Orders created for this
    /// instrument. Specified in prices units.
    #[serde(rename = "minimumGuaranteedStopLossDistance")]
    pub minimum_guaranteed_stop_loss_distance: Decimal,

    /// The minimum trailing stop distance allowed for a trailing stop loss
    /// created for this instrument. Specified in prices units.
    #[serde(rename = "minimumTrailingStopDistance")]
    pub minimum_trailing_stop_distance: Decimal,

    /// The maximum position size allowed for this instrument. Specified in
    /// units.
    #[serde(rename = "maximumPositionSize")]
    pub maximum_position_size: Decimal,

    /// The maximum units allowed for an Order placed for this instrument.
    /// Specified in units.
    #[serde(rename = "maximumOrderUnits")]
    pub maximum_order_units: Decimal,

    /// The margin rate for this instrument.
    #[serde(rename = "marginRate")]
    pub margin_rate: Decimal,

    /// The commission structure for this instrument.
    #[serde(rename = "commission")]
    pub commission: InstrumentCommission,

    /// The current Guaranteed Stop Loss Order mode of the Account for this
    /// Instrument.
    #[serde(rename = "guaranteedStopLossOrderMode")]
    pub guaranteed_stop_loss_order_mode: GuaranteedStopLossOrderModeForInstrument,

    /// The amount that is charged to the account if a guaranteed Stop Loss Order
    /// is triggered and filled. Specified in prices units and charged per unit of the Trade.
    /// Present only if the Account’s guaranteedStopLossOrderMode for this Instrument is not ‘DISABLED’.
    #[serde(rename = "guaranteedStopLossOrderExecutionPremium")]
    pub guaranteed_stop_loss_order_execution_premium: Option<Decimal>,

    /// The guaranteed Stop Loss Order level restriction for this instrument.
    /// Present only if the Account’s guaranteedStopLossOrderMode for this Instrument is not ‘DISABLED’.
    #[serde(rename = "guaranteedStopLossOrderLevelRestriction")]
    pub guaranteed_stop_loss_order_level_restriction: Option<GuaranteedStopLossOrderLevelRestriction>,

    /// Financing data for this instrument.
    #[serde(rename = "financing")]
    pub financing: InstrumentFinancing,

    /// The tags associated with this instrument.
    #[serde(rename = "tags")]
    pub tags: Vec<Tag>,
}

/// A date and time value.
#[derive(Debug, Clone)]
pub struct DateTime(NaiveDateTime);

impl DateTime {
    pub fn to_naive_datetime(&self) -> NaiveDateTime {
        self.0
    }

    pub fn new(naive_datetime: NaiveDateTime) -> Self {
        DateTime(naive_datetime)
    }
}

impl Serialize for DateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Just convert to UTC and format as RFC3339
        let utc_dt = self.0.and_utc();
        let formatted = utc_dt.to_rfc3339();
        serializer.serialize_str(&formatted)
    }
}

impl<'de> Deserialize<'de> for DateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        // Try RFC3339 format first
        if let Ok(dt) = ChronoDateTime::parse_from_rfc3339(&s) {
            return Ok(DateTime(dt.naive_utc()));
        }

        // Try Unix timestamp with fractional seconds
        if let Ok(unix_time) = s.parse::<f64>() {
            let secs = unix_time as i64;
            let millis = ((unix_time - secs as f64) * 1000.0) as u32 * 1_000_000; // Convert to nanos for chrono
            if let Some(dt) = ChronoDateTime::<Utc>::from_timestamp(secs, millis) {
                return Ok(DateTime(dt.naive_utc()));
            }
        }

        Err(de::Error::custom(format!("Unable to parse datetime: {}", s)))
    }
}
/// Represents the acceptable datetime formats for the API.
///
/// The datetime format can be either:
/// - `UNIX`: DateTime fields will be specified or returned in the "12345678.000000123" format.
/// - `RFC3339`: DateTime will be specified or returned in the "YYYY-MM-DDTHH:MM:SS.nnnnnnnnnZ" format.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum AcceptDatetimeFormat {
    Unix,
    Rfc3339,
}

/// Instrument-specific commission details.
#[derive(Serialize, Deserialize, Debug)]
pub struct InstrumentCommission {
    /// Commission amount (in the Account’s home currency) charged per units traded of the instrument.
    #[serde(rename = "commission")]
    pub commission: Decimal,

    /// Number of units traded that the commission amount is based on.
    #[serde(rename = "unitsTraded")]
    pub units_traded: Decimal,

    /// Minimum commission amount (in the Account’s home currency) charged when an Order is filled.
    #[serde(rename = "minimumCommission")]
    pub minimum_commission: Decimal,
}

/// Behavior of the Account regarding Guaranteed Stop Loss Orders for a specific Instrument.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum GuaranteedStopLossOrderModeForInstrument {
    Disabled,
    Allowed,
    Required,
}

/// Restrictions for Guaranteed Stop Loss Order levels for a specific Instrument.
#[derive(Serialize, Deserialize, Debug)]
pub struct GuaranteedStopLossOrderLevelRestriction {
    /// Total allowed Trade volume within the prices range for Trades with guaranteed Stop Loss Orders.
    pub volume: Decimal,

    /// Price range the volume applies to, in prices units.
    #[serde(rename = "priceRange")]
    pub price_range: Decimal,
}

/// Direction of an Order or a Trade.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum Direction {
    Long,
    Short,
}

/// Price component(s) for candlestick data.
#[allow(dead_code)]
pub(crate) type PricingComponent = String;

/// Conversion factor for currency conversion to the Account’s home currency.
#[derive(Serialize, Deserialize, Debug)]
pub struct ConversionFactor {
    /// Factor to multiply the amount in a given currency to obtain the amount in the home currency.
    pub factor: Decimal,
}

/// Conversion factors for converting amounts to the Account’s home currency.
#[derive(Serialize, Deserialize, Debug)]
pub struct HomeConversionFactors {
    /// Conversion factor for gains in Instrument quote units to the home currency.
    #[serde(rename = "gainQuoteHome")]
    pub gain_quote_home: ConversionFactor,

    /// Conversion factor for losses in Instrument quote units to the home currency.
    #[serde(rename = "lossQuoteHome")]
    pub loss_quote_home: ConversionFactor,

    /// Conversion factor for gains in Instrument base units to the home currency.
    #[serde(rename = "gainBaseHome")]
    pub gain_base_home: ConversionFactor,

    /// Conversion factor for losses in Instrument base units to the home currency.
    #[serde(rename = "lossBaseHome")]
    pub loss_base_home: ConversionFactor,
}
