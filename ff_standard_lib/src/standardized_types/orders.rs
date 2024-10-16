use std::fmt;
use crate::strategies::ledgers::AccountId;
use crate::standardized_types::enums::OrderSide;
use crate::standardized_types::subscriptions::SymbolName;
use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use serde_derive::{Deserialize, Serialize};
use std::str::FromStr;
use rust_decimal_macros::dec;
use crate::standardized_types::broker_enum::Brokerage;
use crate::standardized_types::new_types::{Price, TimeString, TzString, Volume};

#[derive(
    Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Serialize, Deserialize,
)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum OrderRequest {
    Create{brokerage: Brokerage, order: Order, order_type: OrderType},
    Cancel{brokerage: Brokerage, order_id: OrderId, account_id: AccountId},
    Update{brokerage: Brokerage, order_id: OrderId, account_id: AccountId, update: OrderUpdateType },
    CancelAll{brokerage: Brokerage, account_id: AccountId, symbol_name: SymbolName},
    FlattenAllFor{brokerage: Brokerage, account_id: AccountId},
}

impl OrderRequest {
    pub fn brokerage(&self) -> Brokerage {
        match self {
            OrderRequest::Create { brokerage, .. } => brokerage.clone(),
            OrderRequest::Cancel { brokerage, .. } => brokerage.clone(),
            OrderRequest::Update { brokerage,.. } => brokerage.clone(),
            OrderRequest::CancelAll { brokerage,.. } => brokerage.clone(),
            OrderRequest::FlattenAllFor { brokerage,.. } => brokerage.clone(),
        }
    }
}

#[derive(
    Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Serialize, Deserialize,
)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum OrderError {
    InvalidPrice,
    InvalidQuantity,
    InvalidSide,
    InsufficientFunds,
    InvalidTag,
    InvalidOrderId,
    OrderNotFound,
    OrderAlreadyFilled,
}

#[derive(
    Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Serialize, Deserialize,
)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum TimeInForce {
    GTC,
    IOC,
    FOK,
    Day(TzString),
    Time(TimeString, TzString)
}

#[derive(Archive, Clone, rkyv::Serialize, rkyv::Deserialize, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
/// Protective Orders always exit the entire position, for custom exits we can just submit regular orders.
pub enum ProtectiveOrder {
    TakeProfit {
        price: Price
    },
    StopLoss {
        price: Price
    },
    TrailingStopLoss {
        price: Price,
        trail_value: Price
    },
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Serialize, Deserialize)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum OrderType {
    Limit,
    Market,
    MarketIfTouched,
    StopMarket,
    StopLimit,
    /// If we are adding to  an existing position and have Some(brackets), the existing brackets will be replaced.
    /// # Arguments
    /// brackets: `Option<Vec<ProtectiveOrder>>`,
    EnterLong,
    /// If we are adding to  an existing position and have Some(brackets), the existing brackets will be replaced.
    /// # Arguments
    /// brackets: `Option<Vec<ProtectiveOrder>>`,
    EnterShort,
    ExitLong,
    ExitShort,
    //UpdateBrackets(Brokerage, AccountId, SymbolName, Vec<ProtectiveOrder>)
}

#[derive(
    Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Serialize, Deserialize,
)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum OrderState {
    Created,
    Accepted,
    Filled,
    PartiallyFilled,
    Cancelled,
    Rejected(String),
}

#[derive(
    Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Serialize, Deserialize,
)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct Order {
    pub symbol_name: SymbolName,
    pub brokerage: Brokerage,
    pub quantity_open: Volume,
    pub quantity_filled: Volume,
    pub average_fill_price: Option<Price>,
    pub limit_price: Option<Price>,
    pub trigger_price: Option<Price>,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub time_in_force: TimeInForce,
    pub tag: String,
    pub id: OrderId,
    pub time_created_utc: String,
    pub time_filled_utc: Option<String>,
    pub state: OrderState,
    pub fees: Price,
    pub value: Price,
    pub account_id: AccountId,
}

impl Order {
    pub fn update_time_created_utc(&mut self, time: DateTime<Utc>) {
        self.time_created_utc = time.to_string();
    }

    pub fn limit_order(
        symbol_name: SymbolName,
        brokerage: Brokerage,
        quantity: Volume,
        side: OrderSide,
        tag: String,
        account_id: AccountId,
        order_id: OrderId,
        time: DateTime<Utc>,
        limit_price: Price,
        tif: TimeInForce
    ) -> Self {
        Self {
            symbol_name,
            brokerage,
            quantity_open: quantity,
            quantity_filled: dec!(0.0),
            average_fill_price: None,
            limit_price: Some(limit_price),
            trigger_price: None,
            side,
            order_type: OrderType::Limit,
            time_in_force: tif,
            tag,
            id: order_id,
            time_created_utc: time.to_string(),
            time_filled_utc: None,
            state: OrderState::Created,
            fees: dec!(0.0),
            value: dec!(0.0),
            account_id,
        }
    }

    pub fn market_if_touched (
        symbol_name: SymbolName,
        brokerage: Brokerage,
        quantity: Volume,
        side: OrderSide,
        tag: String,
        account_id: AccountId,
        order_id: OrderId,
        time: DateTime<Utc>,
        trigger_price: Price,
        tif: TimeInForce
    ) -> Self {
        Self {
            symbol_name,
            brokerage,
            quantity_open: quantity,
            quantity_filled: dec!(0.0),
            average_fill_price: None,
            limit_price: None,
            trigger_price: Some(trigger_price),
            side,
            order_type: OrderType::MarketIfTouched,
            time_in_force: tif,
            tag,
            id: order_id,
            time_created_utc: time.to_string(),
            time_filled_utc: None,
            state: OrderState::Created,
            fees: dec!(0.0),
            value: dec!(0.0),
            account_id,
        }
    }

    pub fn stop (
        symbol_name: SymbolName,
        brokerage: Brokerage,
        quantity: Volume,
        side: OrderSide,
        tag: String,
        account_id: AccountId,
        order_id: OrderId,
        time: DateTime<Utc>,
        trigger_price: Price,
        tif: TimeInForce
    ) -> Self {
        Self {
            symbol_name,
            brokerage,
            quantity_open: quantity,
            quantity_filled: dec!(0.0),
            average_fill_price: None,
            limit_price: None,
            trigger_price: Some(trigger_price),
            side,
            order_type: OrderType::StopMarket,
            time_in_force: tif,
            tag,
            id: order_id,
            time_created_utc: time.to_string(),
            time_filled_utc: None,
            state: OrderState::Created,
            fees: dec!(0.0),
            value: dec!(0.0),
            account_id,
        }
    }

    pub fn stop_limit (
        symbol_name: SymbolName,
        brokerage: Brokerage,
        quantity: Volume,
        side: OrderSide,
        tag: String,
        account_id: AccountId,
        order_id: OrderId,
        time: DateTime<Utc>,
        limit_price: Price,
        trigger_price: Price,
        tif: TimeInForce
    ) -> Self {
        Self {
            symbol_name,
            brokerage,
            quantity_open: quantity,
            quantity_filled: dec!(0.0),
            average_fill_price: None,
            limit_price: Some(limit_price),
            trigger_price: Some(trigger_price),
            side,
            order_type: OrderType::StopLimit,
            time_in_force: tif,
            tag,
            id: order_id,
            time_created_utc: time.to_string(),
            time_filled_utc: None,
            state: OrderState::Created,
            fees: dec!(0.0),
            value: dec!(0.0),
            account_id,
        }
    }

    pub fn market_order(
        symbol_name: SymbolName,
        brokerage: Brokerage,
        quantity: Volume,
        side: OrderSide,
        tag: String,
        account_id: AccountId,
        order_id: OrderId,
        time: DateTime<Utc>,
    ) -> Self {
        Order {
            id:order_id,
            symbol_name,
            brokerage,
            quantity_open: quantity,
            quantity_filled: dec!(0.0),
            average_fill_price: None,
            limit_price: None,
            trigger_price: None,
            side,
            order_type: OrderType::Market,
            time_in_force: TimeInForce::FOK,
            tag,
            time_created_utc: time.to_string(),
            time_filled_utc: None,
            state: OrderState::Created,
            fees: dec!(0.0),
            value: dec!(0.0),
            account_id,
        }
    }

    pub fn exit_long(
        symbol_name: SymbolName,
        brokerage: Brokerage,
        quantity: Volume,
        tag: String,
        account_id: AccountId,
        order_id: OrderId,
        time: DateTime<Utc>,
    ) -> Self {
        Order {
            id: order_id,
            symbol_name,
            brokerage,
            quantity_open: quantity,
            quantity_filled: dec!(0.0),
            average_fill_price: None,
            limit_price: None,
            trigger_price: None,
            side: OrderSide::Sell,
            order_type: OrderType::ExitLong,
            time_in_force: TimeInForce::FOK,
            tag,
            time_created_utc: time.to_string(),
            time_filled_utc: None,
            state: OrderState::Created,
            fees: dec!(0.0),
            value: dec!(0.0),
            account_id,
        }
    }

    pub fn exit_short(
        symbol_name: SymbolName,
        brokerage: Brokerage,
        quantity: Volume,
        tag: String,
        account_id: AccountId,
        order_id: OrderId,
        time: DateTime<Utc>,
    ) -> Self {
        Order {
            id: order_id,
            symbol_name,
            brokerage,
            quantity_open: quantity,
            quantity_filled: dec!(0.0),
            average_fill_price: None,
            limit_price: None,
            trigger_price: None,
            side: OrderSide::Buy,
            order_type: OrderType::ExitShort,
            time_in_force: TimeInForce::FOK,
            tag,
            time_created_utc: time.to_string(),
            time_filled_utc: None,
            state: OrderState::Created,
            fees: dec!(0.0),
            value: dec!(0.0),
            account_id,
        }
    }

    pub fn enter_long(
        symbol_name: SymbolName,
        brokerage: Brokerage,
        quantity: Volume,
        tag: String,
        account_id: AccountId,
        order_id: OrderId,
        time: DateTime<Utc>,
    ) -> Self {
        Order {
            id: order_id,
            symbol_name,
            brokerage,
            quantity_open: quantity,
            quantity_filled: dec!(0.0),
            average_fill_price: None,
            limit_price: None,
            trigger_price: None,
            side: OrderSide::Buy,
            order_type: OrderType::EnterLong,
            time_in_force: TimeInForce::FOK,
            tag,
            time_created_utc: time.to_string(),
            time_filled_utc: None,
            state: OrderState::Created,
            fees: dec!(0.0),
            value: dec!(0.0),
            account_id,
        }
    }

    pub fn enter_short(
        symbol_name: SymbolName,
        brokerage: Brokerage,
        quantity: Volume,
        tag: String,
        account_id: AccountId,
        order_id: OrderId,
        time: DateTime<Utc>,
    ) -> Self {
        Order {
            id: order_id,
            symbol_name,
            brokerage,
            quantity_open: quantity,
            quantity_filled: dec!(0.0),
            average_fill_price: None,
            limit_price: None,
            trigger_price: None,
            side: OrderSide::Sell,
            order_type: OrderType::EnterShort,
            time_in_force: TimeInForce::FOK,
            tag,
            time_created_utc: time.to_string(),
            time_filled_utc: None,
            state: OrderState::Created,
            fees: dec!(0.0),
            value: dec!(0.0),
            account_id,
        }
    }

    pub fn can_cancel(&self) -> bool {
        if self.state == OrderState::Created
            || self.state == OrderState::Accepted
            || self.state == OrderState::PartiallyFilled
        {
            return match self.order_type {
                OrderType::Limit
                | OrderType::StopLimit
                | OrderType::StopMarket
                | OrderType::MarketIfTouched => true,
                _ => false,
            }
        }
        false
    }

    pub fn time_created_utc(&self) -> DateTime<Utc> {
        DateTime::from_str(&self.time_created_utc).unwrap()
    }
    pub fn time_created_local(&self, time_zone: &Tz) -> DateTime<Tz> {
        let utc_time: DateTime<Utc> = DateTime::from_str(&self.time_created_utc).unwrap();
        time_zone.from_utc_datetime(&utc_time.naive_utc())
    }

    pub fn time_filled_utc(&self) -> Option<DateTime<Utc>> {
        match &self.time_filled_utc {
            Some(time) => Some(DateTime::from_str(time).unwrap()),
            None => None,
        }
    }

    pub fn time_filled_local(&self, time_zone: &Tz) -> Option<DateTime<Tz>> {
        match &self.time_filled_utc {
            Some(time) => {
                let utc_time: DateTime<Utc> = DateTime::from_str(&time).unwrap();
                Some(time_zone.from_utc_datetime(&utc_time.naive_utc()))
            },
            None => None,
        }
    }
}

pub type OrderId = String;

#[derive(
    Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Serialize, Deserialize,
)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum OrderUpdateType {
    LimitPrice(Price),
    TriggerPrice(Price),
    TimeInForce(TimeInForce),
    Quantity(Volume),
    Tag(String),
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
/// Represents the various states and updates an order can undergo in the trading system.
///
/// This enum is used to communicate changes in order status between the trading strategy, the user interface, and the brokerage connection. Each variant represents a specific type of update or state change that an order can experience.
pub enum OrderUpdateEvent {
    OrderAccepted {brokerage:Brokerage, account_id: AccountId, order_id: OrderId, tag: String, time: String},

    OrderFilled {brokerage:Brokerage, account_id: AccountId, order_id: OrderId, tag: String, time: String},

    OrderPartiallyFilled {brokerage:Brokerage, account_id: AccountId, order_id: OrderId, tag: String, time: String},

    OrderCancelled {brokerage:Brokerage, account_id: AccountId, order_id: OrderId, tag: String, time: String},

    OrderRejected {brokerage:Brokerage, account_id: AccountId, order_id: OrderId, reason: String, tag: String, time: String},

    OrderUpdated {brokerage:Brokerage, account_id: AccountId, order_id: OrderId, order: Order, tag: String, time: String},

    OrderUpdateRejected {brokerage:Brokerage, account_id: AccountId, order_id: OrderId, reason: String, time: String},
}

impl OrderUpdateEvent {
    pub fn time_local(&self, time_zone: &Tz) -> DateTime<Tz> {
        let utc_time: DateTime<Utc> = self.time_utc();
        time_zone.from_utc_datetime(&utc_time.naive_utc())
    }

    pub fn time_utc(&self) -> DateTime<Utc> {
        match self {
            OrderUpdateEvent::OrderAccepted { time, .. } => DateTime::from_str(time).unwrap(),
            OrderUpdateEvent::OrderFilled { time, .. } => DateTime::from_str(time).unwrap(),
            OrderUpdateEvent::OrderPartiallyFilled { time, .. } => DateTime::from_str(time).unwrap(),
            OrderUpdateEvent::OrderCancelled { time, .. } => DateTime::from_str(time).unwrap(),
            OrderUpdateEvent::OrderRejected { time, .. } => DateTime::from_str(time).unwrap(),
            OrderUpdateEvent::OrderUpdated { time, .. } => DateTime::from_str(time).unwrap(),
            OrderUpdateEvent::OrderUpdateRejected { time, .. } => DateTime::from_str(time).unwrap(),
        }
    }

    pub fn order_id(&self) -> &OrderId {
        match self {
            OrderUpdateEvent::OrderAccepted { order_id, .. } => order_id,
            OrderUpdateEvent::OrderFilled { order_id, .. } => order_id,
            OrderUpdateEvent::OrderPartiallyFilled { order_id, .. } => order_id,
            OrderUpdateEvent::OrderCancelled { order_id, .. } => order_id,
            OrderUpdateEvent::OrderRejected { order_id, .. } => order_id,
            OrderUpdateEvent::OrderUpdated { order_id, .. } => order_id,
            OrderUpdateEvent::OrderUpdateRejected { order_id, .. } => order_id,
        }
    }

    /// If the event had changed the order state this will return Some(OrderState)
    /// Order Updates and Update Rejections do not change the orders state, they simply change the order, so they return None here
    pub fn state_change(&self) -> Option<OrderState> {
        match self {
            OrderUpdateEvent::OrderAccepted {  .. } => Some(OrderState::Accepted),
            OrderUpdateEvent::OrderFilled {  .. } =>  Some(OrderState::Filled),
            OrderUpdateEvent::OrderPartiallyFilled {  .. } => Some(OrderState::PartiallyFilled),
            OrderUpdateEvent::OrderCancelled {  .. } => Some(OrderState::Cancelled),
            OrderUpdateEvent::OrderRejected {  reason, .. } => Some(OrderState::Rejected(reason.clone())),
            OrderUpdateEvent::OrderUpdated {  .. } => None,
            OrderUpdateEvent::OrderUpdateRejected {  .. } => None,
        }
    }
}
impl fmt::Display for OrderUpdateEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderUpdateEvent::OrderAccepted { brokerage, account_id, order_id,tag,.. } => {
                write!(f, "Order Accepted: Brokerage: {}, Account ID: {}, Order ID: {}, Tag: {}", brokerage, account_id, order_id, tag)
            }
            OrderUpdateEvent::OrderFilled { brokerage, account_id, order_id,tag,.. } => {
                write!(f, "Order Filled: Brokerage: {}, Account ID: {}, Order ID: {}, Tag: {}", brokerage, account_id, order_id, tag)
            }
            OrderUpdateEvent::OrderPartiallyFilled { brokerage, account_id, order_id,tag,.. } => {
                write!(f, "Order Partially Filled: Brokerage: {}, Account ID: {}, Order ID: {}, Tag: {}", brokerage, account_id, order_id, tag)
            }
            OrderUpdateEvent::OrderCancelled { brokerage, account_id, order_id,tag,.. } => {
                write!(f, "Order Cancelled: Brokerage: {}, Account ID: {}, Order ID: {}, Tag: {}", brokerage, account_id, order_id, tag)
            }
            OrderUpdateEvent::OrderRejected { brokerage, account_id, order_id, reason,tag,.. } => {
                write!(f, "Order Rejected: Brokerage: {}, Account ID: {}, Order ID: {}. Reason: {}, Tag: {}", brokerage, account_id, order_id, reason, tag)
            }
            OrderUpdateEvent::OrderUpdated { brokerage, account_id, order_id,tag, .. } => {
                write!(f, "Order Updated: Brokerage: {}, Account ID: {}, Order ID: {}, Tag: {}", brokerage, account_id, order_id, tag)
            }
            OrderUpdateEvent::OrderUpdateRejected { brokerage, account_id, order_id, reason,.. } => {
                write!(f, "Order Update Rejected: Brokerage: {}, Account ID: {}, Order ID: {}. Reason: {}", brokerage, account_id, order_id, reason)
            }
        }
    }
}
