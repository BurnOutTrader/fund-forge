use std::str::FromStr;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use rust_decimal::{Decimal};
use rust_decimal_macros::dec;
use uuid::Uuid;
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::product_maps::oanda::maps::{OANDA_SYMBOL_INFO};
use crate::server_features::server_side_brokerage::BrokerApiResponse;
use ff_standard_lib::standardized_types::accounts::{Account, AccountId};
use ff_standard_lib::standardized_types::enums::{OrderSide, PositionSide, StrategyMode};
use ff_standard_lib::standardized_types::orders::{Order, OrderId, OrderState, OrderType, OrderUpdateEvent, OrderUpdateType, TimeInForce};
use ff_standard_lib::standardized_types::subscriptions::{SymbolName};
use ff_standard_lib::StreamName;
use crate::oanda_api::api_client::OandaClient;
use crate::oanda_api::models::order::order_related;
use crate::oanda_api::models::order::order_related::{OrderPositionFill};
use crate::oanda_api::models::transaction_related::ClientExtensions;
use crate::request_handlers::RESPONSE_SENDERS;

#[async_trait]
impl BrokerApiResponse for OandaClient {
    #[allow(unused)]
    async fn symbol_names_response(&self, mode: StrategyMode, time: Option<DateTime<Utc>>, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        let mut symbol_names: Vec<SymbolName> = Vec::new();
        for symbol in self.instruments_map.iter() {
            symbol_names.push(symbol.key().clone());
        }
        DataServerResponse::SymbolNames {
            callback_id,
            symbol_names,
        }
    }

    #[allow(unused)]
    async fn account_info_response(&self, mode: StrategyMode, stream_name: StreamName, account_id: AccountId, callback_id: u64) -> DataServerResponse {
        match self.account_info.get(&account_id) {
            None => {
                DataServerResponse::Error {callback_id, error: FundForgeError::ClientSideErrorDebug(
                    format!("No account found for id: {}", account_id)
                )}
            }
            Some(account_info) => {
                DataServerResponse::AccountInfo {callback_id, account_info: account_info.clone()}
            }
        }
    }

    #[allow(unused)]
    async fn symbol_info_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        if let Some(info) = OANDA_SYMBOL_INFO.get(&symbol_name) { //todo this will all be replaced, for live only, the backtesting info will come from the coded maps
            return DataServerResponse::SymbolInfo {
                callback_id,
                symbol_info: info.clone(),
            }
        }
        DataServerResponse::Error {
            callback_id,
            error: FundForgeError::ClientSideErrorDebug(format!("Symbol not found: {}", symbol_name)),
        }
    }

    #[allow(unused)]
    async fn accounts_response(&self, mode: StrategyMode, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        let accounts: Vec<AccountId> = self.accounts.iter().map(|a| a.account_id.clone()).collect();
        DataServerResponse::Accounts {
            callback_id,
            accounts,
        }
    }

    #[allow(unused)]
    async fn logout_command(&self, stream_name: StreamName) {
        todo!()
    }

    #[allow(unused)]
    async fn commission_info_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    #[allow(unused)]
    async fn live_market_order(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        self.other_orders(stream_name, mode, order).await
    }

    #[allow(unused)]
    async fn live_enter_long(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        if let Some(position_map) = self.positions.get(&order.account.account_id) {
            if let Some(position) = position_map.get(&order.symbol_name) {
                if position.side == PositionSide::Short {
                    let exit_long_order = Order {
                        id: order.id.clone(),
                        time_created_utc: Utc::now().to_string(),
                        time_filled_utc: None,
                        state: OrderState::Created,
                        fees: Default::default(),
                        value: Default::default(),
                        account: order.account.clone(),
                        symbol_name: order.symbol_name.clone(),
                        side: OrderSide::Buy,
                        order_type: OrderType::ExitShort,
                        quantity_open: position.quantity_open,
                        quantity_filled: Default::default(),
                        average_fill_price: None,
                        limit_price: None,
                        trigger_price: None,
                        time_in_force: TimeInForce::FOK,
                        tag: "Exit Short, Before Enter Long".to_string(),
                        symbol_code: order.symbol_code.clone(),
                        exchange: order.exchange.clone(),
                    };
                    match self.other_orders(stream_name.clone(), mode, exit_long_order).await {
                        Ok(_) => {}
                        Err(e) => {
                            return Err(e)
                        }
                    }
                }
            }
        }
        self.other_orders(stream_name, mode, order).await
    }

    #[allow(unused)]
    async fn live_enter_short(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        if let Some(position_map) = self.positions.get(&order.account.account_id) {
            if let Some(position) = position_map.get(&order.symbol_name) {
                if position.side == PositionSide::Long {
                    let exit_long_order = Order {
                        id: order.id.clone(),
                        time_created_utc: Utc::now().to_string(),
                        time_filled_utc: None,
                        state: OrderState::Created,
                        fees: Default::default(),
                        value: Default::default(),
                        account: order.account.clone(),
                        symbol_name: order.symbol_name.clone(),
                        side: OrderSide::Sell,
                        order_type: OrderType::ExitLong,
                        quantity_open: position.quantity_open,
                        quantity_filled: Default::default(),
                        average_fill_price: None,
                        limit_price: None,
                        trigger_price: None,
                        time_in_force: TimeInForce::FOK,
                        tag: "Exit Long, Before Enter Short".to_string(),
                        symbol_code: order.symbol_code.clone(),
                        exchange: order.exchange.clone(),
                    };
                    match self.other_orders(stream_name.clone(), mode, exit_long_order).await {
                        Ok(_) => {}
                        Err(e) => {
                            return Err(e)
                        }
                    }
                }
            }
        }
        self.other_orders(stream_name, mode, order).await
    }

    #[allow(unused)]
    async fn live_exit_short(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        self.other_orders(stream_name, mode, order).await
    }

    #[allow(unused)]
    async fn live_exit_long(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        self.other_orders(stream_name, mode, order).await
    }

    #[allow(unused)]
    async fn other_orders(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        let mut order = order;
        // Convert the symbol format from "EUR/USD" to "EUR_USD"
        let oanda_symbol =  if let Some(instrument) = self.instruments_map.get(&order.symbol_name) {
            // Add to cleaned subs if there's an active broadcaster or it's a new subscription
            instrument.name.clone()
        } else {
            return Err(OrderUpdateEvent::OrderRejected {
                account: order.account,
                symbol_name: order.symbol_name.to_string(),
                symbol_code: order.symbol_name,
                order_id: order.id,
                reason: "No Oanda instrument found when converting name".to_string(),
                tag: order.tag,
                time: Utc::now().to_string(),
            });
        };

        // Format quantity as string with sign
        let units = match order.side {
            OrderSide::Buy => order.quantity_open,
            OrderSide::Sell => -order.quantity_open,
        };

        let (time_in_force, gtd_time) = match order.time_in_force {
            TimeInForce::GTC => ("GTC".to_string(), None),
            TimeInForce::IOC => ("IOC".to_string(), None),
            TimeInForce::FOK => ("FOK".to_string(), None),
            TimeInForce::Day => ("GFD".to_string(), None),
            TimeInForce::Time(time_stamp) => {
                let time = match DateTime::<Utc>::from_timestamp(time_stamp, 0) {
                    Some(t) => t,
                    None => {
                        return Err(OrderUpdateEvent::OrderRejected {
                            account: order.account,
                            symbol_name: order.symbol_name.to_string(),
                            symbol_code: order.symbol_name,
                            order_id: order.id,
                            reason: "Invalid time stamp".to_string(),
                            tag: order.tag,
                            time: Utc::now().to_string(),
                        });
                    }
                };

                ("GTD".to_string(), Some(crate::oanda_api::models::primitives::DateTime::new(time.naive_utc())))
            }
        };

        let (order_type, position_fill) = match order.order_type {
            OrderType::Limit => (order_related::OrderType::Limit, OrderPositionFill::ReduceFirst),
            OrderType::Market => (order_related::OrderType::Market, OrderPositionFill::ReduceFirst),
            OrderType::MarketIfTouched => (order_related::OrderType::MarketIfTouched, OrderPositionFill::ReduceFirst),
            OrderType::StopMarket =>  (order_related::OrderType::Stop, OrderPositionFill::ReduceFirst),
            OrderType::StopLimit =>  (order_related::OrderType::Stop, OrderPositionFill::ReduceFirst),
            OrderType::EnterLong | OrderType::EnterShort => (order_related::OrderType::Market, OrderPositionFill::Default),
            OrderType::ExitLong |  OrderType::ExitShort => (order_related::OrderType::Market, OrderPositionFill::ReduceOnly),
            _ => {
                return Err(OrderUpdateEvent::OrderRejected {
                    account: order.account,
                    symbol_name: order.symbol_name.to_string(),
                    symbol_code: order.symbol_name,
                    order_id: order.id,
                    reason: "Order type not supported".to_string(),
                    tag: order.tag,
                    time: Utc::now().to_string(),
                });
            },
        };

        let client_extensions = Some(ClientExtensions {
            id: order.id.clone(),
            tag: order.tag.clone(),
            comment: "".to_string(),
        });

        if stream_name != 0 {
            self.open_orders.insert(order.id.clone(), order.clone());
            self.id_stream_name_map.insert(order.id.clone(), stream_name.clone());
        }

        // Construct the endpoint
        let endpoint = format!("/accounts/{}/orders", order.account.account_id);
        let url = format!("{}{}", self.base_endpoint, endpoint);

        // Acquire a permit from the rate limiter
        let permit = self.rate_limiter.acquire().await;

       let json_order =  match order.order_type {
            OrderType::Market | OrderType::EnterLong | OrderType::EnterShort | OrderType::ExitLong | OrderType::ExitShort => {
                serde_json::json!({
                    "order": {
                        "type": order_type,
                        "instrument": oanda_symbol,
                        "units": units,
                        "timeInForce": time_in_force,
                        "positionFill": "REDUCE_FIRST".to_string(),
                        "clientExtensions": client_extensions,
                    }
                })
            }
            OrderType::Limit => {
                let price = match order.limit_price {
                    Some(p) => p,
                    None => {
                        return Err(OrderUpdateEvent::OrderRejected {
                            account: order.account,
                            symbol_name: order.symbol_name.to_string(),
                            symbol_code: order.symbol_name,
                            order_id: order.id,
                            reason: "No limit price provided".to_string(),
                            tag: order.tag,
                            time: Utc::now().to_string(),
                        });
                    }
                };
                serde_json::json!({
                    "order": {
                        "type": order_type,
                        "instrument": oanda_symbol,
                        "units": units,
                        "timeInForce": time_in_force,
                        "gtd_time": gtd_time,
                        "positionFill": "REDUCE_FIRST".to_string(),
                        "clientExtensions": client_extensions,
                        "trigger_condition": "DEFAULT".to_string(),
                        "price": price,

                    }
                })
            }
            
            OrderType::MarketIfTouched => {
                let price = match order.trigger_price {
                    Some(p) => p,
                    None => {
                        return Err(OrderUpdateEvent::OrderRejected {
                            account: order.account,
                            symbol_name: order.symbol_name.to_string(),
                            symbol_code: order.symbol_name,
                            order_id: order.id,
                            reason: "No trigger price provided".to_string(),
                            tag: order.tag,
                            time: Utc::now().to_string(),
                        });
                    }
                };
                serde_json::json!({
                    "order": {
                        "type": order_type,
                        "instrument": oanda_symbol,
                        "units": units,
                        "timeInForce": time_in_force,
                        "gtd_time": gtd_time,
                        "positionFill": "REDUCE_FIRST".to_string(),
                        "clientExtensions": client_extensions,
                        "trigger_condition": "DEFAULT".to_string(),
                        "price": price,
                        "price_bound": order.limit_price,

                    }
                })
            }
            OrderType::StopMarket => {
                let price = match order.trigger_price {
                    Some(p) => p,
                    None => {
                        return Err(OrderUpdateEvent::OrderRejected {
                            account: order.account,
                            symbol_name: order.symbol_name.to_string(),
                            symbol_code: order.symbol_name,
                            order_id: order.id,
                            reason: "No trigger price provided".to_string(),
                            tag: order.tag,
                            time: Utc::now().to_string(),
                        });
                    }
                };
                serde_json::json!({
                    "order": {
                        "type": order_type,
                        "instrument": oanda_symbol,
                        "units": units,
                        "timeInForce": time_in_force,
                        "gtd_time": gtd_time,
                        "positionFill": "REDUCE_FIRST".to_string(),
                        "clientExtensions": client_extensions,
                        "trigger_condition": "DEFAULT".to_string(),
                        "price": price,
                    }
                })
            }
           OrderType::StopLimit => {
               let price = match order.trigger_price {
                   Some(p) => p,
                   None => {
                       return Err(OrderUpdateEvent::OrderRejected {
                           account: order.account,
                           symbol_name: order.symbol_name.to_string(),
                           symbol_code: order.symbol_name,
                           order_id: order.id,
                           reason: "No trigger price provided".to_string(),
                           tag: order.tag,
                           time: Utc::now().to_string(),
                       });
                   }
               };
               serde_json::json!({
                    "order": {
                        "type": order_type,
                        "instrument": oanda_symbol,
                        "units": units,
                        "timeInForce": time_in_force,
                        "gtd_time": gtd_time,
                        "positionFill": "REDUCE_FIRST".to_string(),
                        "clientExtensions": client_extensions,
                        "trigger_condition": "DEFAULT".to_string(),
                        "price": price,
                        "price_bound": order.limit_price,
                    }
                })
           }
       };
        match self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json_order)
            .send()
            .await {
            Ok(response) => {
                println!("Response: {:?}", response);
                match response.status() {
                    StatusCode::CREATED => {
                        match response.json::<serde_json::Value>().await {
                            Ok(create_response) => {
                                // Send order accepted event
                                let accept_event = OrderUpdateEvent::OrderAccepted {
                                    order_id: order.id.clone(),
                                    account: order.account.clone(),
                                    symbol_name: order.symbol_name.clone(),
                                    symbol_code: order.symbol_name.clone(),
                                    tag: order.tag.clone(),
                                    time: Utc::now().to_string(),
                                };
                                order.state = OrderState::Accepted;
                                //send to the stream receiver
                                if let Some(stream_receiver) = RESPONSE_SENDERS.get(&stream_name) {
                                    stream_receiver.send(DataServerResponse::OrderUpdates {
                                        event: accept_event,
                                        time: Utc::now().to_string(),
                                    }).await;
                                }

                                // If order was immediately filled
                                // Check if fill transaction exists and get_requests its fields
                                if let Some(fill) = create_response["orderFillTransaction"].as_object() {
                                    let quantity = match fill["units"]
                                        .as_str()
                                        .and_then(|u| Decimal::from_str(u).ok()) {
                                        Some(q) => q,
                                        None => return Ok(())
                                    }.abs();

                                    let price = match fill["price"]
                                        .as_str()
                                        .and_then(|p| Decimal::from_str(p).ok()) {
                                        Some(p) => p,
                                        None => return Ok(())
                                    };

                                    let fill_event = match quantity == order.quantity_open {
                                        true => {
                                            order.state = OrderState::Filled;
                                            OrderUpdateEvent::OrderFilled {
                                                order_id: order.id.clone(),
                                                account: order.account.clone(),
                                                symbol_name: order.symbol_name.clone(),
                                                symbol_code: order.symbol_name.clone(),
                                                quantity,
                                                price,
                                                side: order.side.clone(),
                                                tag: order.tag.clone(),
                                                time: Utc::now().to_string(),
                                            }
                                        },
                                        false => {
                                            order.state = OrderState::PartiallyFilled;
                                            OrderUpdateEvent::OrderPartiallyFilled {
                                                order_id: order.id.clone(),
                                                account: order.account.clone(),
                                                symbol_name: order.symbol_name.clone(),
                                                symbol_code: order.symbol_name.clone(),
                                                quantity,
                                                price,
                                                side: order.side.clone(),
                                                tag: order.tag.clone(),
                                                time: Utc::now().to_string(),
                                            }
                                        },
                                    };
                                    order.quantity_open -= quantity;
                                    order.quantity_filled += quantity;

                                    if let Some(stream_receiver) = RESPONSE_SENDERS.get(&stream_name) {
                                        stream_receiver.send(DataServerResponse::OrderUpdates {
                                            event: fill_event,
                                            time: Utc::now().to_string(),
                                        }).await;
                                    }
                                }

                                if order.state != OrderState::Filled {
                                    self.open_orders.insert(order.id.clone(), order);
                                }
                                Ok(())
                            }
                            Err(e) => Err(OrderUpdateEvent::OrderRejected {
                                account: order.account,
                                symbol_name: order.symbol_name.to_string(),
                                symbol_code: order.symbol_name,
                                order_id: order.id,
                                reason: format!("Failed to parse order response: {}", e),
                                tag: order.tag,
                                time: Utc::now().to_string(),
                            })
                        }
                    }
                    StatusCode::BAD_REQUEST | StatusCode::NOT_FOUND => {
                        match response.json::<serde_json::Value>().await {
                            Ok(json) => Err(OrderUpdateEvent::OrderRejected {
                                account: order.account,
                                symbol_name: order.symbol_name.to_string(),
                                symbol_code: order.symbol_name,
                                order_id: order.id,
                                reason: json["errorMessage"].as_str().unwrap_or("Unknown error").to_string(),
                                tag: order.tag,
                                time: Utc::now().to_string(),
                            }),
                            Err(e) => Err(OrderUpdateEvent::OrderRejected {
                                account: order.account,
                                symbol_name: order.symbol_name.to_string(),
                                symbol_code: order.symbol_name,
                                order_id: order.id,
                                reason: format!("Failed to parse rejection: {}", e),
                                tag: order.tag,
                                time: Utc::now().to_string(),
                            })
                        }
                    }
                    status => Err(OrderUpdateEvent::OrderRejected {
                        account: order.account,
                        symbol_name: order.symbol_name.to_string(),
                        symbol_code: order.symbol_name,
                        order_id: order.id,
                        reason: format!("Unexpected response status: {}", status),
                        tag: order.tag,
                        time: Utc::now().to_string(),
                    })
                }
            }
            Err(e) => {
                Err(OrderUpdateEvent::OrderRejected {
                    account: order.account,
                    symbol_name: order.symbol_name.to_string(),
                    symbol_code: order.symbol_name,
                    order_id: order.id,
                    reason: format!("Server Error sending order: {}", e),
                    tag: order.tag,
                    time: Utc::now().to_string(),
                })
            }
        }
    }

    #[allow(unused)]
    async fn cancel_orders_on_account(&self, account: Account) {
        for order in self.open_orders.iter() {
            if order.account == account {
                self.cancel_order(account.clone(), order.id.clone()).await;
            }
        }
    }

    #[allow(unused)]
    async fn cancel_order(&self, account: Account, order_id: OrderId) {
        let endpoint = format!("/accounts/{}/orders/{}/cancel", account.account_id, order_id);
        let url = format!("{}{}", self.base_endpoint, endpoint);

        // Acquire a permit from the rate limiter
        let _permit = self.rate_limiter.acquire().await;

        if let Err(e) = self.client
            .put(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
        {
            return;
        }
    }

    #[allow(unused)]
    async fn flatten_all_for(&self, account: Account) {
        self.cancel_orders_on_account(account.clone()).await;
        if let Some(position_map) = self.positions.get(&account.account_id) {
            for position in position_map.iter() {
                let guid = Uuid::new_v4();
                let (tag, side,order_type) = match position.value().side {
                    PositionSide::Long => {
                        ("Flatten Long".to_string(), OrderSide::Sell, OrderType::ExitLong)
                    }
                    PositionSide::Short => {
                        ("Flatten Short".to_string(), OrderSide::Buy, OrderType::ExitShort)
                    }
                };
                let exit_order = Order {
                    id: guid.to_string(),
                    time_created_utc: Utc::now().to_string(),
                    time_filled_utc: None,
                    state: OrderState::Created,
                    fees: Default::default(),
                    value: Default::default(),
                    account: account.clone(),
                    symbol_name: position.symbol_name.clone(),
                    side,
                    order_type,
                    quantity_open: position.quantity_open,
                    quantity_filled: dec!(0),
                    average_fill_price: None,
                    limit_price: None,
                    trigger_price: None,
                    time_in_force: TimeInForce::FOK,
                    tag,
                    symbol_code: position.symbol_name.clone(),
                    exchange: None,
                };
                let _ = self.other_orders(0, StrategyMode::Live, exit_order).await;
            }
        }
    }

    #[allow(unused)]
    async fn update_order(&self, account: Account, order_id: OrderId, update: OrderUpdateType) -> Result<(), OrderUpdateEvent> {
        Err(OrderUpdateEvent::OrderUpdateRejected {
            account,
            order_id,
            reason: "Order updates not supported with Oanda, please cancel order and replace".to_string(),
            time: Utc::now().to_string(),
        })
    }
}
