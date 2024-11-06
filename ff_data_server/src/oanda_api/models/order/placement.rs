use serde::{Deserialize, Serialize};
use crate::oanda_api::models::order::order_related;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OandaOrderRequest {
    pub(crate) order: OandaOrder,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct OandaOrder {
    pub(crate) units: String,
    pub(crate) instrument: String,
    pub(crate) time_in_force: order_related::TimeInForce,
    #[serde(rename = "type")]
    pub(crate) order_type: order_related::OrderType,
    pub(crate) position_fill: order_related::OrderPositionFill,
    pub(crate) price: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderResponse {
    last_transaction_id: String,
    order_create_transaction: OrderCreateTransaction,
    order_fill_transaction: Option<OrderFillTransaction>,
    related_transaction_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderCreateTransaction {
    id: String,
    time: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderFillTransaction {
    id: String,
    order_id: String,
    time: String,
    price: String,
    units: String,
}
