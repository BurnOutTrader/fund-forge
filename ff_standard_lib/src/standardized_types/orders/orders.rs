use crate::standardized_types::accounts::ledgers::AccountId;
use crate::standardized_types::data_server_messaging::FundForgeError;
use crate::standardized_types::enums::OrderSide;
use crate::standardized_types::subscriptions::{SymbolName};
use crate::standardized_types::{Price, Volume};
use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use serde_derive::{Deserialize, Serialize};
use std::str::FromStr;
use rust_decimal_macros::dec;
use strum_macros::Display;
use crate::apis::brokerage::broker_enum::Brokerage;

#[derive(
    Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Serialize, Deserialize,
)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum OrderRequest {
    Create{brokerage: Brokerage, order: Order},
    Cancel{brokerage: Brokerage, order_id: OrderId, account_id: AccountId},
    Update{brokerage: Brokerage, order_id: OrderId, account_id: AccountId, update: OrderUpdateType },
    CancelAll{brokerage: Brokerage, account_id: AccountId, symbol_name: SymbolName}
}

impl OrderRequest {
    pub fn brokerage(&self) -> Brokerage {
        match self {
            OrderRequest::Create { brokerage, .. } => brokerage.clone(),
            OrderRequest::Cancel { brokerage, .. } => brokerage.clone(),
            OrderRequest::Update { brokerage,.. } => brokerage.clone(),
            OrderRequest::CancelAll { brokerage,.. } => brokerage.clone(),
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
    Day,
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
    pub quantity_ordered: Volume,
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
            quantity_ordered: quantity,
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
            quantity_ordered: quantity,
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
            quantity_ordered: quantity,
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
            quantity_ordered: quantity,
            quantity_filled: dec!(0.0),
            average_fill_price: None,
            limit_price: Some(trigger_price),
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
            quantity_ordered: quantity,
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
            quantity_ordered: quantity,
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
            quantity_ordered: quantity,
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
            quantity_ordered: quantity,
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
            quantity_ordered: quantity,
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

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Display)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
/// Represents the various states and updates an order can undergo in the trading system.
///
/// This enum is used to communicate changes in order status between the trading strategy, the user interface, and the brokerage connection. Each variant represents a specific type of update or state change that an order can experience.
pub enum OrderUpdateEvent {
    OrderAccepted {brokerage:Brokerage, account_id: AccountId, order_id: OrderId},

    OrderFilled {brokerage:Brokerage, account_id: AccountId, order_id: OrderId},

    OrderPartiallyFilled {brokerage:Brokerage, account_id: AccountId, order_id: OrderId},

    OrderCancelled {brokerage:Brokerage, account_id: AccountId, order_id: OrderId},

    OrderRejected {brokerage:Brokerage, account_id: AccountId, order_id: OrderId, reason: String},

    OrderUpdated {brokerage:Brokerage, account_id: AccountId, order_id: OrderId, order: Order},

    OrderUpdateRejected {brokerage:Brokerage, account_id: AccountId, order_id: OrderId, reason: String},
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
