use rust_decimal::Decimal;
use serde::{Serialize, Deserialize};
use ff_standard_lib::standardized_types::orders::OrderId;
use crate::oanda_api::models::primitives::GuaranteedStopLossOrderLevelRestriction;

/// The type of the Order.
#[derive(Serialize, Deserialize, Debug)]
pub enum OrderType {
    #[serde(rename = "MARKET")]
    Market,
    #[serde(rename = "LIMIT")]
    Limit,
    #[serde(rename = "STOP")]
    Stop,
    #[serde(rename = "MARKET_IF_TOUCHED")]
    MarketIfTouched,
    #[serde(rename = "TAKE_PROFIT")]
    TakeProfit,
    #[serde(rename = "STOP_LOSS")]
    StopLoss,
    #[serde(rename = "GUARANTEED_STOP_LOSS")]
    GuaranteedStopLoss,
    #[serde(rename = "TRAILING_STOP_LOSS")]
    TrailingStopLoss,
    #[serde(rename = "FIXED_PRICE")]
    FixedPrice,
}

/// The type of the Order.
#[derive(Serialize, Deserialize, Debug)]
pub enum CancellableOrderType {
    #[serde(rename = "LIMIT")]
    Limit,
    #[serde(rename = "STOP")]
    Stop,
    #[serde(rename = "MARKET_IF_TOUCHED")]
    MarketIfTouched,
    #[serde(rename = "TAKE_PROFIT")]
    TakeProfit,
    #[serde(rename = "STOP_LOSS")]
    StopLoss,
    #[serde(rename = "GUARANTEED_STOP_LOSS")]
    GuaranteedStopLoss,
    #[serde(rename = "TRAILING_STOP_LOSS")]
    TrailingStopLoss,
}

/// The current state of the Order.
#[derive(Serialize, Deserialize, Debug)]
pub enum OrderState {
    #[serde(rename = "PENDING")]
    Pending,
    #[serde(rename = "FILLED")]
    Filled,
    #[serde(rename = "TRIGGERED")]
    Triggered,
    #[serde(rename = "CANCELLED")]
    Cancelled,
}

/// The state to filter the requested Orders by.
#[derive(Serialize, Deserialize, Debug)]
pub enum OrderStateFilter {
    #[serde(rename = "PENDING")]
    Pending,
    #[serde(rename = "FILLED")]
    Filled,
    #[serde(rename = "TRIGGERED")]
    Triggered,
    #[serde(rename = "CANCELLED")]
    Cancelled,
    #[serde(rename = "ALL")]
    All,
}

/// An OrderIdentifier is used to refer to an Order, and contains both the OrderID and the ClientOrderID.
#[derive(Serialize, Deserialize, Debug)]
pub struct OrderIdentifier {
    /// The OANDA-assigned Order ID
    #[serde(rename = "orderID")]
    pub order_id: OrderId,

    /// The client-provided client Order ID
    #[serde(rename = "clientOrderID")]
    pub client_order_id: String,
}

/// The time-in-force of an Order.
#[derive(Serialize, Deserialize, Debug)]
pub enum TimeInForce {
    #[serde(rename = "GTC")]
    GTC,
    #[serde(rename = "GTD")]
    GTD,
    #[serde(rename = "GFD")]
    GFD,
    #[serde(rename = "FOK")]
    FOK,
    #[serde(rename = "IOC")]
    IOC,
}

/// Specification of how Positions in the Account are modified when the Order is filled.
#[derive(Serialize, Deserialize, Debug)]
pub enum OrderPositionFill {
    #[serde(rename = "OPEN_ONLY")]
    OpenOnly,
    #[serde(rename = "REDUCE_FIRST")]
    ReduceFirst,
    #[serde(rename = "REDUCE_ONLY")]
    ReduceOnly,
    #[serde(rename = "DEFAULT")]
    Default,
}

/// Specification of which prices component should be used when determining if an Order should be triggered and filled.
#[derive(Serialize, Deserialize, Debug)]
pub enum OrderTriggerCondition {
    #[serde(rename = "DEFAULT")]
    Default,
    #[serde(rename = "INVERSE")]
    Inverse,
    #[serde(rename = "BID")]
    Bid,
    #[serde(rename = "ASK")]
    Ask,
    #[serde(rename = "MID")]
    Mid,
}

/// Details required by clients creating a Guaranteed Stop Loss Order
#[derive(Serialize, Deserialize, Debug)]
pub struct GuaranteedStopLossOrderEntryData {
    /// The minimum distance allowed between the Trade’s fill prices and the configured prices for guaranteed Stop Loss Orders created for this instrument. Specified in prices units.
    #[serde(rename = "minimumDistance")]
    pub minimum_distance: Decimal,

    /// The amount that is charged to the account if a guaranteed Stop Loss Order is triggered and filled. The value is in prices units and is charged for each unit of the Trade.
    pub premium: Decimal,

    /// The guaranteed Stop Loss Order level restriction for this instrument.
    #[serde(rename = "levelRestriction")]
    pub level_restriction: GuaranteedStopLossOrderLevelRestriction,
}

/// Representation of how many units of an Instrument are available to be traded by an Order depending on its positionFill option.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnitsAvailable {
    /// The number of units that are available to be traded using an Order with a positionFill option of “DEFAULT”.
    pub default: UnitsAvailableDetails,

    /// The number of units that may are available to be traded with an Order with a positionFill option of “REDUCE_FIRST”.
    #[serde(rename = "reduceFirst")]
    pub reduce_first: UnitsAvailableDetails,

    /// The number of units that may are available to be traded with an Order with a positionFill option of “REDUCE_ONLY”.
    #[serde(rename = "reduceOnly")]
    pub reduce_only: UnitsAvailableDetails,

    /// The number of units that may are available to be traded with an Order with a positionFill option of “OPEN_ONLY”.
    #[serde(rename = "openOnly")]
    pub open_only: UnitsAvailableDetails,
}

/// The dynamic state of an Order. This is only relevant to TrailingStopLoss Orders, as no other Order type has dynamic state.
#[derive(Serialize, Deserialize, Debug)]
pub struct DynamicOrderState {
    /// The Order’s ID.
    pub id: OrderId,

    /// The Order’s calculated trailing stop value.
    #[serde(rename = "trailingStopValue")]
    pub trailing_stop_value: Decimal,

    /// The distance between the Trailing Stop Loss Order’s trailingStopValue and the current Market Price. This represents the distance (in prices units) of the Order from a triggering prices. If the distance could not be determined, this value will not be set.
    #[serde(rename = "triggerDistance")]
    pub trigger_distance: Decimal,

    /// True if an exact trigger distance could be calculated. If false, it means the provided trigger distance is a best estimate. If the distance could not be determined, this value will not be set.
    #[serde(rename = "isTriggerDistanceExact")]
    pub is_trigger_distance_exact: bool,
}

/// Representation of how many units of an Instrument are available to be traded for both long and short Orders.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnitsAvailableDetails {
    /// The units available for long Orders.
    pub long: Decimal,

    /// The units available for short Orders.
    pub short: Decimal,
}

