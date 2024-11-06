use async_trait::async_trait;
use chrono::{DateTime, Utc};
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

        let (limit_price, order_type, position_fill, trigger_price) = match order.order_type {
            OrderType::Limit => (Some(order.limit_price.unwrap()), order_related::OrderType::Limit, OrderPositionFill::ReduceFirst, None),
            OrderType::Market => (None, order_related::OrderType::Market, OrderPositionFill::ReduceFirst, None),
            OrderType::MarketIfTouched => (None, order_related::OrderType::MarketIfTouched, OrderPositionFill::ReduceFirst, Some(order.trigger_price.unwrap())),
            OrderType::StopMarket =>  (None, order_related::OrderType::Stop, OrderPositionFill::ReduceFirst, Some(order.trigger_price.unwrap())),
            OrderType::StopLimit =>  (Some(order.limit_price.unwrap()), order_related::OrderType::Stop, OrderPositionFill::ReduceFirst, Some(order.trigger_price.unwrap())),
            OrderType::EnterLong | OrderType::EnterShort => (Some(order.trigger_price.unwrap()), order_related::OrderType::Market, OrderPositionFill::Default, None),
            OrderType::ExitLong |  OrderType::ExitShort => (Some(order.trigger_price.unwrap()), order_related::OrderType::Market, OrderPositionFill::ReduceOnly, None),
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

        self.open_orders.insert(order.id.clone(), order.clone());
        self.id_stream_name_map.insert(order.id.clone(), stream_name.clone());

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
                let req =  LimitOrderRequest {
                    order_type,
                    instrument: oanda_symbol,
                    units,
                    price: limit_price.unwrap(),
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
                let req = MarketIfTouchedOrderRequest {
                    order_type,
                    instrument: oanda_symbol,
                    units,
                    price: trigger_price.unwrap(),
                    price_bound: limit_price,
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
                let req = StopOrderRequest {
                    order_type,
                    instrument: oanda_symbol,
                    units,
                    price: trigger_price.unwrap(),
                    price_bound: limit_price,
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
                Ok(())
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
        todo!()
    }

    #[allow(unused)]
    async fn cancel_order(&self, account: Account, order_id: OrderId) {
        todo!()
    }

    #[allow(unused)]
    async fn flatten_all_for(&self, account: Account) {
        todo!()
    }

    #[allow(unused)]
    async fn update_order(&self, account: Account, order_id: OrderId, update: OrderUpdateType) -> Result<(), OrderUpdateEvent> {
        todo!()
    }
}
