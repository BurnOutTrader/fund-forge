use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ff_rithmic_api::rithmic_proto_objects::rti::{RequestNewOrder};
use ff_rithmic_api::rithmic_proto_objects::rti::last_trade::TransactionType;
use ff_rithmic_api::rithmic_proto_objects::rti::request_bracket_order::{Duration, OrderPlacement};
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
use ff_rithmic_api::rithmic_proto_objects::rti::request_new_order::PriceType;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::server_features::server_side_brokerage::BrokerApiResponse;
use ff_standard_lib::standardized_types::enums::{FuturesExchange, OrderSide, StrategyMode};
use ff_standard_lib::standardized_types::new_types::Volume;
use ff_standard_lib::standardized_types::orders::{Order, OrderUpdateEvent};
use ff_standard_lib::standardized_types::subscriptions::SymbolName;
use ff_standard_lib::strategies::ledgers::{AccountId, AccountInfo, Currency};
use ff_standard_lib::StreamName;
use crate::get_shutdown_sender;
use crate::rithmic_api::api_client::RithmicClient;
use crate::rithmic_api::products::{get_available_symbol_names, get_exchange_by_code, get_futures_commissions_info, get_intraday_margin, get_overnight_margin, get_symbol_info};

#[async_trait]
impl BrokerApiResponse for RithmicClient {
    async fn symbol_names_response(&self, _mode: StrategyMode, _time: Option<DateTime<Utc>>, _stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        let symbol_names = get_available_symbol_names();

        if symbol_names.is_empty() {
            DataServerResponse::Error {
                callback_id,
                error: FundForgeError::ClientSideErrorDebug("No symbols available".to_string()),
            }
        } else {
            DataServerResponse::SymbolNames {
                callback_id,
                symbol_names: symbol_names.clone(),
            }
        }
    }

    async fn account_info_response(&self, mode: StrategyMode, _stream_name: StreamName, account_id: AccountId, callback_id: u64) -> DataServerResponse {
        //todo use match mode to create sim account
        match mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                todo!("Not implemented for backtest")
            }
            StrategyMode::Live => {
                match self.account_info.get(&account_id) {
                    None => DataServerResponse::Error {callback_id, error:FundForgeError::ClientSideErrorDebug(format!("{} Has No Account for {}",self.brokerage, account_id))},
                    Some(account_info) => DataServerResponse::AccountInfo {
                        callback_id,
                        account_info: account_info.value().clone(),
                    }
                }
            }
        }
    }

    async fn paper_account_init(&self, account_id: AccountId, callback_id: u64) -> DataServerResponse {
        let account_info = AccountInfo {
            account_id,
            brokerage: self.brokerage,
            cash_value: dec!(0.0),
            cash_available: dec!(0.0),
            currency: Currency::USD,
            open_pnl: dec!(0.0),
            booked_pnl: dec!(0.0),
            day_open_pnl: dec!(0.0),
            day_booked_pnl: dec!(0.0),
            cash_used: dec!(0.0),
            positions: vec![],
            is_hedging: false,
            leverage: 1,
            buy_limit: None,
            sell_limit: None,
            max_orders: None,
            daily_max_loss: None,
            daily_max_loss_reset_time: None,
        };
        DataServerResponse::PaperAccountInit {
            callback_id,
            account_info,
        }
    }

    async fn symbol_info_response(
        &self,
        _mode: StrategyMode,
        _stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse {
        match get_symbol_info(&symbol_name) {
            Ok(symbol_info) => DataServerResponse::SymbolInfo {callback_id, symbol_info},
            Err(e) => DataServerResponse::Error {callback_id, error: FundForgeError::ClientSideErrorDebug(format!("{}", e))}
        }
    }

    async fn intraday_margin_required_response(
        &self,
        _mode: StrategyMode, //todo we should check with broker when live
        _stream_name: StreamName,
        symbol_name: SymbolName,
        quantity: Volume,
        callback_id: u64
    ) -> DataServerResponse {
        match get_intraday_margin(&symbol_name) {
            None => {
                DataServerResponse::Error {
                    callback_id,
                    error: FundForgeError::ClientSideErrorDebug(format!("{} not found with: {}", symbol_name, self.brokerage)),
                }
            }
            Some(margin) => {
                let required_margin = margin * Decimal::from(quantity.abs());
                DataServerResponse::IntradayMarginRequired {
                    callback_id,
                    symbol_name,
                    price: Some(required_margin),
                }
            }
        }
    }

    async fn overnight_margin_required_response(
        &self,
        _mode: StrategyMode, //todo we should check with broker when live
        _stream_name: StreamName,
        symbol_name: SymbolName,
        quantity: Volume,
        callback_id: u64
    ) -> DataServerResponse {
        match get_overnight_margin(&symbol_name) {
            None => {
                DataServerResponse::Error {
                    callback_id,
                    error: FundForgeError::ClientSideErrorDebug(format!("{} not found with: {}", symbol_name, self.brokerage)),
                }
            }
            Some(margin) => {
                let required_margin = margin * Decimal::from(quantity.abs());

                DataServerResponse::IntradayMarginRequired {
                    callback_id,
                    symbol_name,
                    price: Some(required_margin),
                }
            }
        }
    }

    async fn accounts_response(&self, _mode: StrategyMode, _stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        // The accounts are collected on initializing the client
        let accounts = self.account_info.iter().map(|entry| entry.key().clone()).collect();
        DataServerResponse::Accounts {
            callback_id,
            accounts,
        }
    }

    async fn logout_command(&self, stream_name: StreamName) {
        //todo handle dynamically from server using stream name to remove subscriptions and callbacks
        self.callbacks.remove(&stream_name);
    }

    async fn commission_info_response(&self, _mode: StrategyMode, _stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        //todo add a mode to get live commsions from specific brokerage.
        match get_futures_commissions_info(&symbol_name) {
            Ok(commission_info) => DataServerResponse::CommissionInfo {
                callback_id,
                commission_info,
            },
            Err(e) => DataServerResponse::Error {
                callback_id,
                error: FundForgeError::ClientSideErrorDebug(e)
            }
        }
    }

    async fn live_market_order(
        &self,
        stream_name: StreamName,
        mode: StrategyMode,
        order: Order,
    ) -> Result<(), OrderUpdateEvent>
    {
        if mode != StrategyMode::Live {
            get_shutdown_sender().send(()).unwrap();
            panic!("This should never happen, Live order sent by Backtest")
        }

        match self.is_valid_order(&order) {
            Err(e) =>
                return Err(OrderUpdateEvent::OrderRejected {
                    brokerage: order.brokerage,
                    account_id: order.account_id,
                    order_id: order.id,
                    reason: e,
                    tag: order.tag,
                    time: Utc::now().to_string() }),

            Ok(_) => {}
        }

       let quantity = match order.quantity_open.to_i32() {
           None => {
               return Err(OrderUpdateEvent::OrderRejected {
                   brokerage: order.brokerage,
                   account_id: order.account_id,
                   order_id: order.id,
                   reason: "Invalid Quantity".to_string(),
                   tag: order.tag,
                   time: Utc::now().to_string() })
           }
           Some(q) => q
       };

        let (symbol, exchange): (SymbolName, FuturesExchange) = match &order.exchange {
            None => {
                match get_exchange_by_code(&order.symbol_name) {
                    None => {
                        return Err(OrderUpdateEvent::OrderRejected {
                            brokerage: order.brokerage,
                            account_id: order.account_id,
                            order_id: order.id,
                            reason: format!("Exchange Not found with {} for {}",order.brokerage, order.symbol_name),
                            tag: order.tag,
                            time: Utc::now().to_string() })
                    }
                    Some(exchange) => {
                        let front_month = match self.front_month(stream_name, order.symbol_name.clone(), exchange.clone()).await {
                            Ok(info) => info,
                            Err(e) => {
                                return Err(OrderUpdateEvent::OrderRejected {
                                    brokerage: order.brokerage,
                                    account_id: order.account_id,
                                    order_id: order.id,
                                    reason: e,
                                    tag: order.tag,
                                    time: Utc::now().to_string() })
                            }
                        };
                        (front_month.trade_symbol, exchange)
                    }
                }
            }
            Some(exchange_string) => {
                match FuturesExchange::from_string(&exchange_string) {
                    Ok(exchange) => {
                        (order.symbol_name.clone(), exchange)
                    },
                    Err(e) => {
                        return Err(OrderUpdateEvent::OrderRejected {
                            brokerage: order.brokerage,
                            account_id: order.account_id,
                            order_id: order.id,
                            reason: e,
                            tag: order.tag,
                            time: Utc::now().to_string() })
                    }
                }
            }
        };

        let route = match self.default_trade_route.get(&exchange) {
            None => {
                return Err(OrderUpdateEvent::OrderRejected {
                    brokerage: order.brokerage,
                    account_id: order.account_id,
                    order_id: order.id,
                    reason: format!("Order Route Not found with {} for {}",order.brokerage, order.symbol_name),
                    tag: order.tag,
                    time: Utc::now().to_string() })
            }
            Some(route) => route.value().clone(),
        };

        let transaction_type = match order.side {
            OrderSide::Buy => TransactionType::Buy,
            OrderSide::Sell => TransactionType::Sell,
        };

       let req = RequestNewOrder {
           template_id: 312,
           user_msg: vec![order.id.clone()],
           user_tag: Some(order.tag.clone()),
           window_name: None,
           fcm_id: self.fcm_id.clone(),
           ib_id: self.ib_id.clone(),
           account_id: Some(order.account_id.clone()),
           symbol: Some(symbol),
           exchange: Some(exchange.to_string()),
           quantity: Some(quantity),
           price: None,
           trigger_price: None,
           transaction_type: Some(transaction_type.into()),
           duration: Some(Duration::Fok.into()),
           price_type: Some(PriceType::Market.into()),
           trade_route: Some(route),
           manual_or_auto: Some(OrderPlacement::Auto.into()),
           trailing_stop: None,
           trail_by_ticks: None,
           trail_by_price_id: None,
           release_at_ssboe: None,
           release_at_usecs: None,
           cancel_at_ssboe: None,
           cancel_at_usecs: None,
           cancel_after_secs: None,
           if_touched_symbol: None,
           if_touched_exchange: None,
           if_touched_condition: None,
           if_touched_price_field: None,
           if_touched_price: None,
       };

        self.send_message(&SysInfraType::OrderPlant, req).await;
        Ok(())
    }
}