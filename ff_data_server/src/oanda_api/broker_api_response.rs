use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::product_maps::oanda::maps::{calculate_oanda_margin, OANDA_SYMBOL_INFO};
use crate::server_features::server_side_brokerage::BrokerApiResponse;
use ff_standard_lib::standardized_types::accounts::{Account, AccountId};
use ff_standard_lib::standardized_types::enums::{OrderSide, StrategyMode};
use ff_standard_lib::standardized_types::new_types::Volume;
use ff_standard_lib::standardized_types::orders::{Order, OrderId, OrderType, OrderUpdateEvent, OrderUpdateType, TimeInForce};
use ff_standard_lib::standardized_types::subscriptions::{SymbolName};
use ff_standard_lib::StreamName;
use crate::oanda_api::api_client::OandaClient;
use crate::oanda_api::models::order::order_related;
use crate::oanda_api::models::order::order_related::OrderPositionFill;
use crate::oanda_api::models::order::placement::{OandaOrder, OandaOrderRequest};

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
        todo!()
    }

    #[allow(unused)]
    async fn live_enter_short(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        todo!()
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

        let time_in_force = match order.time_in_force {
            TimeInForce::GTC => order_related::TimeInForce::GTC,
            TimeInForce::IOC => order_related::TimeInForce::IOC,
            TimeInForce::FOK => order_related::TimeInForce::FOK,
            TimeInForce::Day => order_related::TimeInForce::GFD,
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
                self.custom_tif_cancel_time_map.insert(order.id.clone(), time);
                order_related::TimeInForce::GTC
            }
        };

        let (price, order_type, position_fill) = match order.order_type {
            OrderType::Limit => (Some(order.limit_price.unwrap().to_string()), order_related::OrderType::Limit, OrderPositionFill::ReduceFirst),
            OrderType::Market => (None, order_related::OrderType::Market, OrderPositionFill::ReduceFirst),
            OrderType::MarketIfTouched => (Some(order.trigger_price.unwrap().to_string()), order_related::OrderType::MarketIfTouched, OrderPositionFill::ReduceFirst),
            OrderType::StopMarket =>  (Some(order.trigger_price.unwrap().to_string()), order_related::OrderType::Stop, OrderPositionFill::ReduceFirst),
            OrderType::EnterLong | OrderType::EnterShort => (Some(order.trigger_price.unwrap().to_string()), order_related::OrderType::Market, OrderPositionFill::Default),
            OrderType::ExitLong |  OrderType::ExitShort => (Some(order.trigger_price.unwrap().to_string()), order_related::OrderType::Market, OrderPositionFill::ReduceOnly),
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

        let oanda_order = OandaOrderRequest {
            order: OandaOrder {
                units: units.to_string(),
                instrument: oanda_symbol,
                time_in_force,
                order_type,
                position_fill,
                price,
            },
        };

        self.open_orders.insert(order.id.clone(), order.clone());
        self.id_stream_name_map.insert(order.id.clone(), stream_name.clone());

        // Construct the endpoint
        let endpoint = format!("/accounts/{}/orders", order.account.account_id);
        let url = format!("{}{}", self.base_endpoint, endpoint);

        // Acquire a permit from the rate limiter
        let permit = self.rate_limiter.acquire().await;

        match self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&order)
            .send()
            .await
        {
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
