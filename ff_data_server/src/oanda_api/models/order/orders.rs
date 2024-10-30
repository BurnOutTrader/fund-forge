use serde::{Deserialize, Serialize};
use ff_standard_lib::standardized_types::orders::OrderId;
use crate::oanda_api::models::order::order_related::{OrderState};
use crate::oanda_api::models::primitives::DateTime;
use crate::oanda_api::models::transaction_related::ClientExtensions;

/// The base Order definition specifies the properties that are common to all Orders.
/// Implemented by: MarketOrder, FixedPriceOrder, LimitOrder, StopOrder, MarketIfTouchedOrder, TakeProfitOrder, StopLossOrder, GuaranteedStopLossOrder, TrailingStopLossOrder
#[derive(Serialize, Deserialize, Debug)]
pub struct Order {
    /// The Order’s identifier, unique within the Order’s Account.
    #[serde(rename = "id")]
    pub id: OrderId,

    /// The time when the Order was created.
    #[serde(rename = "createTime")]
    pub create_time: DateTime,

    /// The current state of the Order.
    #[serde(rename = "state")]
    pub state: OrderState,

    /// The client extensions of the Order. Do not set, modify, or delete
    /// clientExtensions if your account is associated with MT4.
    #[serde(rename = "clientExtensions")]
    pub client_extensions: ClientExtensions,
}

