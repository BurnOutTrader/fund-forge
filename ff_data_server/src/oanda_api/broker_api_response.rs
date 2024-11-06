use std::str::FromStr;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use rust_decimal::{Decimal};
use rust_decimal_macros::dec;
use uuid::Uuid;
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::product_maps::oanda::maps::{calculate_oanda_margin, OANDA_SYMBOL_INFO};
use crate::server_features::server_side_brokerage::BrokerApiResponse;
use ff_standard_lib::standardized_types::accounts::{Account, AccountId};
use ff_standard_lib::standardized_types::enums::{OrderSide, PositionSide, StrategyMode};
use ff_standard_lib::standardized_types::new_types::Volume;
use ff_standard_lib::standardized_types::orders::{Order, OrderId, OrderState, OrderType, OrderUpdateEvent, OrderUpdateType, TimeInForce};
use ff_standard_lib::standardized_types::subscriptions::{SymbolName};
use ff_standard_lib::StreamName;
use crate::oanda_api::api_client::OandaClient;
use crate::oanda_api::models::order::limit_order::LimitOrderRequest;
use crate::oanda_api::models::order::market_if_touched_order::MarketIfTouchedOrderRequest;
use crate::oanda_api::models::order::market_order::{MarketOrderRequest};
use crate::oanda_api::models::order::order_related;
use crate::oanda_api::models::order::order_related::{OrderPositionFill, OrderTriggerCondition};
use crate::oanda_api::models::order::stop_order::StopOrderRequest;
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
    async fn intraday_margin_required_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, quantity: Volume, callback_id: u64) -> DataServerResponse {
        // Calculate the margin required based on symbol and position size
        if let Some(margin_used) = calculate_oanda_margin(&symbol_name, quantity, quantity) { //todo this will all be replaced, for live only, the backtesting info will come from the coded maps
            return DataServerResponse::IntradayMarginRequired {
                callback_id,
                symbol_name,
                price: Some(margin_used),
            }
        }

        DataServerResponse::Error {
            callback_id,
            error: FundForgeError::ClientSideErrorDebug(format!("Symbol not found in margin map: {}", symbol_name)),
        }
    }

    #[allow(unused)]
    async fn overnight_margin_required_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, quantity: Volume, callback_id: u64) -> DataServerResponse {
        self.intraday_margin_required_response(mode, stream_name, symbol_name, quantity, callback_id).await
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
            TimeInForce::GTC => (order_related::TimeInForce::GTC, None),
            TimeInForce::IOC => (order_related::TimeInForce::IOC, None),
            TimeInForce::FOK => (order_related::TimeInForce::FOK, None),
            TimeInForce::Day => (order_related::TimeInForce::GFD, None),
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

                (order_related::TimeInForce::GTD, Some(crate::oanda_api::models::primitives::DateTime::new(time.naive_utc())))
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

        let response = match order.order_type {
            OrderType::Market | OrderType::EnterLong | OrderType::EnterShort | OrderType::ExitLong | OrderType::ExitShort => {
                let req = MarketOrderRequest {
                    order_type,
                    instrument: oanda_symbol,
                    units,
                    time_in_force,
                    price_bound: None,
                    position_fill,
                    client_extensions,
                    take_profit_on_fill: None,
                    stop_loss_on_fill: None,
                    guaranteed_stop_loss_on_fill: None,
                    trailing_stop_loss_on_fill: None,
                    trade_client_extensions: None,
                };
                self.client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .header("Authorization", format!("Bearer {}", self.api_key))
                    .json(&req)
                    .send()
                    .await
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
                let req =  LimitOrderRequest {
                    order_type,
                    instrument: oanda_symbol,
                    units,
                    price,
                    time_in_force,
                    gtd_time,
                    position_fill,
                    trigger_condition: OrderTriggerCondition::Default,
                    client_extensions,
                    take_profit_on_fill: None,
                    stop_loss_on_fill: None,
                    guaranteed_stop_loss_on_fill: None,
                    trailing_stop_loss_on_fill: None,
                    trade_client_extensions: None,
                };
                self.client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .header("Authorization", format!("Bearer {}", self.api_key))
                    .json(&req)
                    .send()
                    .await
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
                let req = MarketIfTouchedOrderRequest {
                    order_type,
                    instrument: oanda_symbol,
                    units,
                    price,
                    price_bound: order.limit_price,
                    time_in_force,
                    gtd_time,
                    position_fill,
                    trigger_condition: OrderTriggerCondition::Default,
                    client_extensions,
                    take_profit_on_fill: None,
                    stop_loss_on_fill: None,
                    guaranteed_stop_loss_on_fill: None,
                    trailing_stop_loss_on_fill: None,
                    trade_client_extensions: None,
                };
                self.client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .header("Authorization", format!("Bearer {}", self.api_key))
                    .json(&req)
                    .send()
                    .await
            }
            OrderType::StopMarket | OrderType::StopLimit => {
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
                let req = StopOrderRequest {
                    order_type,
                    instrument: oanda_symbol,
                    units,
                    price,
                    price_bound: order.limit_price,
                    time_in_force,
                    gtd_time,
                    position_fill,
                    trigger_condition: OrderTriggerCondition::Default,
                    client_extensions,
                    take_profit_on_fill: None,
                    stop_loss_on_fill: None,
                    guaranteed_stop_loss_on_fill: None,
                    trailing_stop_loss_on_fill: None,
                    trade_client_extensions: None,
                };
                self.client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .header("Authorization", format!("Bearer {}", self.api_key))
                    .json(&req)
                    .send()
                    .await
            }
        };

        match response {
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
                                //send to the stream receiver
                                if let Some(stream_receiver) = RESPONSE_SENDERS.get(&stream_name) {
                                    stream_receiver.send(DataServerResponse::OrderUpdates {
                                        event: accept_event,
                                        time: Utc::now().to_string(),
                                    }).await;
                                }

                                // If order was immediately filled
                                // Check if fill transaction exists and get its fields
                                if let Some(fill) = create_response["orderFillTransaction"].as_object() {
                                    let quantity = match fill["units"]
                                        .as_str()
                                        .and_then(|u| Decimal::from_str(u).ok()) {
                                        Some(q) => q,
                                        None => return Ok(())
                                    };

                                    let price = match fill["price"]
                                        .as_str()
                                        .and_then(|p| Decimal::from_str(p).ok()) {
                                        Some(p) => p,
                                        None => return Ok(())
                                    };

                                    let fill_event = OrderUpdateEvent::OrderFilled {
                                        order_id: order.id,
                                        account: order.account,
                                        symbol_name: order.symbol_name.clone(),
                                        symbol_code: order.symbol_name,
                                        quantity,
                                        price,
                                        side: order.side,
                                        tag: order.tag,
                                        time: Utc::now().to_string(),
                                    };

                                    if let Some(stream_receiver) = RESPONSE_SENDERS.get(&stream_name) {
                                        stream_receiver.send(DataServerResponse::OrderUpdates {
                                            event: fill_event,
                                            time: Utc::now().to_string(),
                                        }).await;
                                    }
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
                    symbol_code: None,
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
