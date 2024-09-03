use crate::apis::brokerage::Brokerage;
use crate::helpers::converters::time_local_from_str;
use crate::standardized_types::accounts::ledgers::AccountId;
use crate::standardized_types::data_server_messaging::FundForgeError;
use crate::standardized_types::enums::OrderSide;
use crate::standardized_types::subscriptions::{SymbolName};
use crate::standardized_types::OwnerId;
use chrono::{DateTime, FixedOffset, Utc};
use chrono_tz::Tz;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use serde_derive::{Deserialize, Serialize};
use std::str::FromStr;
use strum_macros::Display;

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
    Day,
}

#[derive(
    Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Serialize, Deserialize,
)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum OrderType {
    Limit,
    Market,
    MarketIfTouched,
    StopMarket,
    StopLimit,
    TakeProfit,
    StopLoss,
    GuaranteedStopLoss,
    TrailingStopLoss,
    TrailingGuaranteedStopLoss,
    EnterLong,
    EnterShort,
    ExitLong,
    ExitShort,
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

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct Order {
    pub owner_id: OwnerId,
    pub symbol_name: SymbolName,
    pub brokerage: Brokerage,
    pub quantity_ordered: u64,
    pub quantity_filled: u64,
    pub average_fill_price: Option<f64>,
    pub limit_price: Option<f64>,
    pub trigger_price: Option<f64>,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub time_in_force: TimeInForce,
    pub tag: String,
    pub id: OrderId,
    pub time_created_utc: String,
    pub time_filled_utc: Option<String>,
    pub state: OrderState,
    pub fees: f64,
    pub value: f64,
    pub account_id: AccountId,
}

impl Order {
    pub fn update_time_created_utc(&mut self, time: DateTime<Utc>) {
        self.time_created_utc = time.to_string();
    }

    pub fn market_order(
        owner_id: OwnerId,
        symbol_name: SymbolName,
        brokerage: Brokerage,
        quantity: u64,
        side: OrderSide,
        tag: String,
        account_id: AccountId,
        time: DateTime<Utc>,
    ) -> Self {
        Order {
            id: format!(
                "{}-{}-{}-{}",
                brokerage,
                symbol_name,
                time.timestamp_millis(),
                side
            ),
            owner_id,
            symbol_name,
            brokerage,
            quantity_ordered: quantity,
            quantity_filled: 0,
            average_fill_price: None,
            limit_price: None,
            trigger_price: None,
            side,
            order_type: OrderType::Market,
            time_in_force: TimeInForce::FOK,
            tag,
            time_created_utc: "".to_string(),
            time_filled_utc: None,
            state: OrderState::Created,
            fees: 0.0,
            value: 0.0,
            account_id,
        }
    }

    pub fn exit_long(
        owner_id: OwnerId,
        symbol_name: SymbolName,
        brokerage: Brokerage,
        quantity: u64,
        tag: String,
        account_id: AccountId,
        time: DateTime<Utc>,
    ) -> Self {
        Order {
            id: format!(
                "{}-{}-{}-{}",
                brokerage,
                symbol_name,
                time.timestamp_millis(),
                OrderSide::Sell
            ),
            owner_id,
            symbol_name,
            brokerage,
            quantity_ordered: quantity,
            quantity_filled: 0,
            average_fill_price: None,
            limit_price: None,
            trigger_price: None,
            side: OrderSide::Sell,
            order_type: OrderType::ExitLong,
            time_in_force: TimeInForce::FOK,
            tag,
            time_created_utc: "".to_string(),
            time_filled_utc: None,
            state: OrderState::Created,
            fees: 0.0,
            value: 0.0,
            account_id,
        }
    }

    pub fn exit_short(
        owner_id: OwnerId,
        symbol_name: SymbolName,
        brokerage: Brokerage,
        quantity: u64,
        tag: String,
        account_id: AccountId,
        time: DateTime<Utc>,
    ) -> Self {
        Order {
            id: format!(
                "{}-{}-{}-{}",
                brokerage,
                symbol_name,
                time.timestamp_millis(),
                OrderSide::Buy
            ),
            owner_id,
            symbol_name,
            brokerage,
            quantity_ordered: quantity,
            quantity_filled: 0,
            average_fill_price: None,
            limit_price: None,
            trigger_price: None,
            side: OrderSide::Buy,
            order_type: OrderType::ExitShort,
            time_in_force: TimeInForce::FOK,
            tag,
            time_created_utc: "".to_string(),
            time_filled_utc: None,
            state: OrderState::Created,
            fees: 0.0,
            value: 0.0,
            account_id,
        }
    }

    pub fn enter_long(
        owner_id: OwnerId,
        symbol_name: SymbolName,
        brokerage: Brokerage,
        quantity: u64,
        tag: String,
        account_id: AccountId,
        time: DateTime<Utc>,
    ) -> Self {
        Order {
            id: format!(
                "{}-{}-{}-{}",
                brokerage,
                symbol_name,
                time.timestamp_millis(),
                OrderSide::Buy
            ),
            owner_id,
            symbol_name,
            brokerage,
            quantity_ordered: quantity,
            quantity_filled: 0,
            average_fill_price: None,
            limit_price: None,
            trigger_price: None,
            side: OrderSide::Buy,
            order_type: OrderType::EnterLong,
            time_in_force: TimeInForce::FOK,
            tag,
            time_created_utc: "".to_string(),
            time_filled_utc: None,
            state: OrderState::Created,
            fees: 0.0,
            value: 0.0,
            account_id,
        }
    }

    pub fn enter_short(
        owner_id: OwnerId,
        symbol_name: SymbolName,
        brokerage: Brokerage,
        quantity: u64,
        tag: String,
        account_id: AccountId,
        time: DateTime<Utc>,
    ) -> Self {
        Order {
            id: format!(
                "{}-{}-{}-{}",
                brokerage,
                symbol_name,
                time.timestamp_millis(),
                OrderSide::Sell
            ),
            owner_id,
            symbol_name,
            brokerage,
            quantity_ordered: quantity,
            quantity_filled: 0,
            average_fill_price: None,
            limit_price: None,
            trigger_price: None,
            side: OrderSide::Sell,
            order_type: OrderType::EnterShort,
            time_in_force: TimeInForce::FOK,
            tag,
            time_created_utc: "".to_string(),
            time_filled_utc: None,
            state: OrderState::Created,
            fees: 0.0,
            value: 0.0,
            account_id,
        }
    }

    pub fn can_cancel(&self) -> bool {
        if self.state == OrderState::Created
            || self.state == OrderState::Accepted
            || self.state == OrderState::PartiallyFilled
        {
            match self.order_type {
                OrderType::Limit
                | OrderType::StopLimit
                | OrderType::TrailingStopLoss
                | OrderType::MarketIfTouched => return true,
                _ => return false,
            }
        }
        false
    }

    pub fn time_created_utc(&self) -> DateTime<Utc> {
        DateTime::from_str(&self.time_created_utc).unwrap()
    }
    pub fn time_created_local(&self, time_zone: &Tz) -> DateTime<FixedOffset> {
        time_local_from_str(time_zone, &self.time_created_utc)
    }

    pub fn time_filled_utc(&self) -> Option<DateTime<Utc>> {
        match &self.time_filled_utc {
            Some(time) => Some(DateTime::from_str(time).unwrap()),
            None => None,
        }
    }

    pub fn time_filled_local(&self, time_zone: &Tz) -> Option<DateTime<FixedOffset>> {
        match &self.time_filled_utc {
            Some(time) => Some(time_local_from_str(time_zone, time)),
            None => None,
        }
    }
}

pub type OrderId = String;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Display)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum OrderChangeType {
    LimitPrice(f64),
    TriggerPrice(f64),
    TimeInForce(TimeInForce),
    Quantity(u64),
    Tag(String),
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Display)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
/// Represents the various states and updates an order can undergo in the trading system.
///
/// This enum is used to communicate changes in order status between the trading strategy, the user interface, and the brokerage connection. Each variant represents a specific type of update or state change that an order can experience.
pub enum OrderUpdateEvent {
    Accepted(Order),

    Filled(Order),

    PartiallyFilled(Order),

    Cancelled(Order),

    Rejected(Order),

    Updated(Order),

    UpdateRejected(Order),
}

impl OrderUpdateEvent {
    pub fn order_id(&self) -> &OrderId {
        match self {
            OrderUpdateEvent::Accepted(order) => &order.id,
            OrderUpdateEvent::Filled(order) => &order.id,
            OrderUpdateEvent::PartiallyFilled(order) => &order.id,
            OrderUpdateEvent::Cancelled(order) => &order.id,
            OrderUpdateEvent::Rejected(order) => &order.id,
            OrderUpdateEvent::Updated(order) => &order.id,
            OrderUpdateEvent::UpdateRejected(order) => &order.id,
        }
    }

    pub fn symbol_name(&self) -> &SymbolName {
        match self {
            OrderUpdateEvent::Accepted(order) => &order.symbol_name,
            OrderUpdateEvent::Filled(order) => &order.symbol_name,
            OrderUpdateEvent::PartiallyFilled(order) => &order.symbol_name,
            OrderUpdateEvent::Cancelled(order) => &order.symbol_name,
            OrderUpdateEvent::Rejected(order) => &order.symbol_name,
            OrderUpdateEvent::Updated(order) => &order.symbol_name,
            OrderUpdateEvent::UpdateRejected(order) => &order.symbol_name,
        }
    }

    pub fn brokerage(&self) -> &Brokerage {
        match self {
            OrderUpdateEvent::Accepted(order) => &order.brokerage,
            OrderUpdateEvent::Filled(order) => &order.brokerage,
            OrderUpdateEvent::PartiallyFilled(order) => &order.brokerage,
            OrderUpdateEvent::Cancelled(order) => &order.brokerage,
            OrderUpdateEvent::Rejected(order) => &order.brokerage,
            OrderUpdateEvent::Updated(order) => &order.brokerage,
            OrderUpdateEvent::UpdateRejected(order) => &order.brokerage,
        }
    }

    pub fn account_id(&self) -> &AccountId {
        match self {
            OrderUpdateEvent::Accepted(order) => &order.account_id,
            OrderUpdateEvent::Filled(order) => &order.account_id,
            OrderUpdateEvent::PartiallyFilled(order) => &order.account_id,
            OrderUpdateEvent::Cancelled(order) => &order.account_id,
            OrderUpdateEvent::Rejected(order) => &order.account_id,
            OrderUpdateEvent::Updated(order) => &order.account_id,
            OrderUpdateEvent::UpdateRejected(order) => &order.account_id,
        }
    }
}

impl OrderUpdateEvent {
    fn to_bytes(&self) -> Vec<u8> {
        let vec = rkyv::to_bytes::<_, 256>(self).unwrap();
        vec.into()
    }

    fn from_bytes(archived: &[u8]) -> Result<OrderUpdateEvent, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<OrderUpdateEvent>(archived) {
            //Ignore this warning: Trait `Deserialize<ResponseType, SharedDeserializeMap>` is not implemented for `ArchivedRequestType` [E0277]
            Ok(response) => Ok(response),
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(e.to_string())),
        }
    }
}
