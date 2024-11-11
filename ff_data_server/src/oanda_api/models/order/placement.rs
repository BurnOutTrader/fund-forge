use serde::{Deserialize, Serialize};
use crate::oanda_api::models::order::order_related;
use crate::oanda_api::models::order::order_related::OandaOrderState;
use crate::oanda_api::models::primitives::DateTime;
use crate::oanda_api::models::transaction_related::ClientExtensions;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OandaOrderRequest {
    pub(crate) order: OandaOrder,
}

#[derive(Debug, Deserialize)]
pub struct OandaOrderUpdate {
    pub id: String,
    #[serde(rename = "createTime")]
    pub create_time: DateTime,
    pub state: OandaOrderState,
    pub instrument: String,
    #[serde(rename = "timeInForce")]
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub units: String,
    pub price: Option<String>,
    #[serde(rename = "positionFill")]
    pub position_fill: String,
    #[serde(rename = "triggerCondition")]
    pub trigger_condition: String,
    #[serde(rename = "partialFill")]
    pub partial_fill: String,
    #[serde(rename = "clientExtensions")]
    pub client_extensions: Option<ClientExtensions>,
    #[serde(rename = "replacesOrderID")]
    pub replaces_order_id: Option<String>,
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
    pub(crate) price: String,
    pub(crate) units: String,
}
