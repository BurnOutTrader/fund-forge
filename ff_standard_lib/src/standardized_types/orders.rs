use std::fmt;
use crate::standardized_types::accounts::{Account, AccountId};
use crate::standardized_types::enums::OrderSide;
use crate::standardized_types::subscriptions::{SymbolCode, SymbolName};
use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use serde_derive::{Deserialize, Serialize};
use std::str::FromStr;
use rust_decimal_macros::dec;
use strum_macros::Display;
use crate::standardized_types::broker_enum::Brokerage;
use crate::standardized_types::new_types::{Price, TzString, Volume};

#[derive(
    Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Serialize, Deserialize,
)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum OrderRequest {
    Create{account: Account, order: Order, order_type: OrderType},
    Cancel{account: Account, order_id: OrderId},
    Update{account: Account, order_id: OrderId, update: OrderUpdateType },
    CancelAll{account: Account, symbol_name: SymbolName},
    FlattenAllFor{account: Account},
}

impl OrderRequest {
    pub fn brokerage(&self) -> Brokerage {
        match self {
            OrderRequest::Create { account, .. } => account.brokerage.clone(),
            OrderRequest::Cancel { account, .. } => account.brokerage.clone(),
            OrderRequest::Update { account,.. } => account.brokerage.clone(),
            OrderRequest::CancelAll { account,.. } => account.brokerage.clone(),
            OrderRequest::FlattenAllFor { account,.. } => account.brokerage.clone(),
        }
    }

    pub fn account_id(&self) -> &AccountId {
        match self {
            OrderRequest::Create { account, .. } => &account.account_id,
            OrderRequest::Cancel { account, .. } => &account.account_id,
            OrderRequest::Update { account,.. } => &account.account_id,
            OrderRequest::CancelAll { account,.. } => &account.account_id,
            OrderRequest::FlattenAllFor { account,.. } => &account.account_id,
        }
    }

    pub fn account(&self) -> &AccountId {
        match self {
            OrderRequest::Create { account, .. } => &account.account_id,
            OrderRequest::Cancel { account, .. } =>  &account.account_id,
            OrderRequest::Update { account,.. } =>  &account.account_id,
            OrderRequest::CancelAll { account,.. } =>  &account.account_id,
            OrderRequest::FlattenAllFor { account,.. } =>  &account.account_id,
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
    Time(i64, TzString)
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

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Serialize, Deserialize)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct Order {
    pub symbol_name: SymbolName,
    pub symbol_code: Option<SymbolCode>,
    pub account: Account,
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
    pub exchange: Option<String>
}

impl Order {
    pub fn update_time_created_utc(&mut self, time: DateTime<Utc>) {
        self.time_created_utc = time.to_string();
    }

    pub fn limit_order(
        symbol_name: SymbolName,
        symbol_code: Option<SymbolCode>,
        account: &Account,
        quantity: Volume,
        side: OrderSide,
        tag: String,
        order_id: OrderId,
        time: DateTime<Utc>,
        limit_price: Price,
        tif: TimeInForce,
        exchange: Option<String>
    ) -> Self {

        Self {
            symbol_name,
            symbol_code,
            account: account.clone(),
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
            exchange
        }
    }

    pub fn market_if_touched (
        symbol_name: SymbolName,
        symbol_code: Option<SymbolCode>,
        account: &Account,
        quantity: Volume,
        side: OrderSide,
        tag: String,
        order_id: OrderId,
        time: DateTime<Utc>,
        trigger_price: Price,
        tif: TimeInForce,
        exchange: Option<String>
    ) -> Self {
        Self {
            symbol_name,
            symbol_code,
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
            account: account.clone(),
            exchange
        }
    }

    pub fn stop (
        symbol_name: SymbolName,
        symbol_code: Option<SymbolCode>,
        account: &Account,
        quantity: Volume,
        side: OrderSide,
        tag: String,
        order_id: OrderId,
        time: DateTime<Utc>,
        trigger_price: Price,
        tif: TimeInForce,
        exchange: Option<String>
    ) -> Self {
        Self {
            symbol_name,
            symbol_code,
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
            account: account.clone(),
            exchange
        }
    }

    pub fn stop_limit (
        symbol_name: SymbolName,
        symbol_code: Option<SymbolCode>,
        account: &Account,
        quantity: Volume,
        side: OrderSide,
        tag: String,
        order_id: OrderId,
        time: DateTime<Utc>,
        limit_price: Price,
        trigger_price: Price,
        tif: TimeInForce,
        exchange: Option<String>
    ) -> Self {
        Self {
            symbol_name,
            symbol_code,
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
            account: account.clone(),
            exchange
        }
    }

    pub fn market_order(
        symbol_name: SymbolName,
        symbol_code: Option<SymbolCode>,
        account: &Account,
        quantity: Volume,
        side: OrderSide,
        tag: String,
        order_id: OrderId,
        time: DateTime<Utc>,
        exchange: Option<String>
    ) -> Self {
        Order {
            id:order_id,
            symbol_name,
            symbol_code,
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
            account: account.clone(),
            exchange
        }
    }

    pub fn exit_long(
        symbol_name: SymbolName,
        symbol_code: Option<SymbolCode>,
        account: &Account,
        quantity: Volume,
        tag: String,
        order_id: OrderId,
        time: DateTime<Utc>,
        exchange: Option<String>
    ) -> Self {
        Order {
            id: order_id,
            symbol_name,
            symbol_code,
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
            account: account.clone(),
            exchange
        }
    }

    pub fn exit_short(
        symbol_name: SymbolName,
        symbol_code: Option<SymbolCode>,
        account: &Account,
        quantity: Volume,
        tag: String,
        order_id: OrderId,
        time: DateTime<Utc>,
        exchange: Option<String>
    ) -> Self {
        Order {
            id: order_id,
            symbol_name,
            symbol_code,
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
            account: account.clone(),
            exchange
        }
    }

    pub fn enter_long(
        symbol_name: SymbolName,
        symbol_code: Option<SymbolCode>,
        account: &Account,
        quantity: Volume,
        tag: String,
        order_id: OrderId,
        time: DateTime<Utc>,
        exchange: Option<String>
    ) -> Self {
        Order {
            id: order_id,
            symbol_name,
            symbol_code,
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
            account: account.clone(),
            exchange
        }
    }

    pub fn enter_short(
        symbol_name: SymbolName,
        symbol_code: Option<SymbolCode>,
        account: &Account,
        quantity: Volume,
        tag: String,
        order_id: OrderId,
        time: DateTime<Utc>,
        exchange: Option<String>
    ) -> Self {
        Order {
            id: order_id,
            symbol_name,
            symbol_code,
            account: account.clone(),
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
            exchange
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
    Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Serialize, Deserialize, Display
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

    /// Example, product: MNQZ4,
    OrderAccepted {account: Account, symbol_name: SymbolName, symbol_code: SymbolCode, order_id: OrderId, tag: String, time: String},

    ///Quantity should only represent the quantity filled on this event.
    OrderFilled {account: Account, symbol_name: SymbolName, symbol_code: SymbolCode, order_id: OrderId, price: Price, quantity: Volume, tag: String, time: String},

    ///Quantity should only represent the quantity filled on this event.
    OrderPartiallyFilled {account: Account,  symbol_name: SymbolName, symbol_code: SymbolCode, order_id: OrderId, price: Price, quantity: Volume, tag: String, time: String},

    OrderCancelled {account: Account, symbol_name: SymbolName, symbol_code: SymbolCode, order_id: OrderId, tag: String, time: String},

    OrderRejected {account: Account,  symbol_name: SymbolName, symbol_code: SymbolCode, order_id: OrderId, reason: String, tag: String, time: String},

    OrderUpdated {account: Account,  symbol_name: SymbolName, symbol_code: SymbolCode, order_id: OrderId, update_type: OrderUpdateType, tag: String, time: String},

    OrderUpdateRejected {account: Account,  order_id: OrderId, reason: String, time: String},
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

    pub fn symbol_code(&self) -> Option<String> {
        match self {
            OrderUpdateEvent::OrderAccepted { symbol_code, .. } => Some(symbol_code.clone()),
            OrderUpdateEvent::OrderFilled { symbol_code, .. } => Some(symbol_code.clone()),
            OrderUpdateEvent::OrderPartiallyFilled { symbol_code, .. } => Some(symbol_code.clone()),
            OrderUpdateEvent::OrderCancelled { symbol_code, .. } => Some(symbol_code.clone()),
            OrderUpdateEvent::OrderRejected { symbol_code, .. } => Some(symbol_code.clone()),
            OrderUpdateEvent::OrderUpdated { symbol_code, .. } => Some(symbol_code.clone()),
            OrderUpdateEvent::OrderUpdateRejected {  .. } => None,
        }
    }

    pub fn brokerage(&self) -> &Brokerage {
        match self {
            OrderUpdateEvent::OrderAccepted { account, .. } => &account.brokerage,
            OrderUpdateEvent::OrderFilled  { account, .. } => &account.brokerage,
            OrderUpdateEvent::OrderPartiallyFilled  { account, .. } => &account.brokerage,
            OrderUpdateEvent::OrderCancelled  { account, .. } => &account.brokerage,
            OrderUpdateEvent::OrderRejected { account, .. } => &account.brokerage,
            OrderUpdateEvent::OrderUpdated  { account, .. } => &account.brokerage,
            OrderUpdateEvent::OrderUpdateRejected  { account, .. } => &account.brokerage,
        }
    }

    pub fn account(&self) -> &Account {
        match self {
            OrderUpdateEvent::OrderAccepted { account, .. } => &account,
            OrderUpdateEvent::OrderFilled  { account, .. } => &account,
            OrderUpdateEvent::OrderPartiallyFilled  { account, .. } => &account,
            OrderUpdateEvent::OrderCancelled  { account, .. } => &account,
            OrderUpdateEvent::OrderRejected { account, .. } => &account,
            OrderUpdateEvent::OrderUpdated  { account, .. } => &account,
            OrderUpdateEvent::OrderUpdateRejected  { account, .. } => &account,
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
            OrderUpdateEvent::OrderAccepted { account,symbol_name, symbol_code: product,order_id,tag,.. } => {
                write!(f, "Order Accepted: Account: {}, Symbol Name: {}, Symbol Code: {}, Order ID: {}, Tag: {}", account, symbol_name, product, order_id, tag)
            }
            OrderUpdateEvent::OrderFilled { account,symbol_name, symbol_code: product, order_id,tag,.. } => {
                write!(f, "Order Filled: Account: {}, Symbol Name: {}, Symbol Code: {}, Order ID: {}, Tag: {}", account, symbol_name, product, order_id, tag)
            }
            OrderUpdateEvent::OrderPartiallyFilled { account,symbol_name, symbol_code: product, order_id,tag,.. } => {
                write!(f, "Order Partially Filled: Account: {}, Symbol Name: {}, Symbol Code: {}, Order ID: {}, Tag: {}", account, symbol_name, product, order_id, tag)
            }
            OrderUpdateEvent::OrderCancelled { account,symbol_name, symbol_code: product, order_id,tag,.. } => {
                write!(f, "Order Cancelled: Account: {}, Symbol Name: {}, Symbol Code: {},Order ID: {}, Tag: {}", account, symbol_name, product, order_id, tag)
            }
            OrderUpdateEvent::OrderRejected { account,symbol_name, symbol_code: product, order_id, reason,tag,.. } => {
                write!(f, "Order Rejected: Account: {}, Symbol Name: {}, Symbol Code: {}, Order ID: {}. Reason: {}, Tag: {}", account, symbol_name, product, order_id, reason, tag)
            }
            OrderUpdateEvent::OrderUpdated { account,symbol_name, symbol_code: product, order_id, update_type, tag, ..} => {
                write!(f, "Order Updated: Account: {}, UpdateType: {:?}, Symbol Name: {}, Symbol Code: {}, Order ID: {}, Tag: {}", account, symbol_name, product, update_type, order_id, tag)
            }
            OrderUpdateEvent::OrderUpdateRejected { account, order_id, reason,.. } => {
                write!(f, "Order Update Rejected: Account: {}, Order ID: {}. Reason: {}", account,  order_id, reason)
            }
        }
    }
}
