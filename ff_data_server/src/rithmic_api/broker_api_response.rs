use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::product_maps::rithmic::maps::{find_base_symbol, get_available_symbol_names, get_exchange_by_symbol_name, get_futures_commissions_info, get_intraday_margin, get_overnight_margin, get_symbol_info};
use crate::server_features::server_side_brokerage::BrokerApiResponse;
use ff_standard_lib::standardized_types::accounts::{Account, AccountId};
use ff_standard_lib::standardized_types::enums::StrategyMode;
use ff_standard_lib::standardized_types::new_types::Volume;
use ff_standard_lib::standardized_types::orders::{Order, OrderId, OrderUpdateEvent, OrderUpdateType};
use ff_standard_lib::standardized_types::subscriptions::SymbolName;
use ff_standard_lib::StreamName;
use crate::request_handlers::RESPONSE_SENDERS;
use crate::rithmic_api::api_client::RithmicBrokerageClient;
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_login::SysInfraType;
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::{RequestCancelAllOrders, RequestCancelOrder, RequestExitPosition, RequestModifyOrder};
#[async_trait]
impl BrokerApiResponse for RithmicBrokerageClient {
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
        match mode {
            StrategyMode::Backtest | StrategyMode::LivePaperTrading => {
                return DataServerResponse::Error {callback_id, error: FundForgeError::ClientSideErrorDebug("No account info for paper accounts".to_string())}
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

    async fn symbol_info_response(
        &self,
        _mode: StrategyMode,
        _stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse {
        match get_symbol_info(&symbol_name) {
            Ok(symbol_info) => DataServerResponse::SymbolInfo {callback_id, symbol_info},
            Err(e) => {
                match find_base_symbol(symbol_name) {
                    None => {}
                    Some(symbol) => {
                        return match get_symbol_info(&symbol) {
                            Ok(info) => DataServerResponse::SymbolInfo { callback_id, symbol_info: info },
                            Err(e) => DataServerResponse::Error { callback_id, error: FundForgeError::ServerErrorDebug(format!("{}", e)) }
                        }
                    }
                };
                DataServerResponse::Error {callback_id, error: FundForgeError::ClientSideErrorDebug(format!("{}", e))}
            }
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
        let details = match self.rithmic_order_details(mode, stream_name, &order).await {
            Ok(details) => details,
            Err(e) => return Err(e)
        };
        self.submit_market_order(stream_name, order, details).await;
        Ok(())
    }

    async fn live_enter_long(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        let mut details = match self.rithmic_order_details(mode, stream_name, &order).await {
            Ok(details) => details,
            Err(e) => return Err(e)
        };

        //check if we are short and add to quantity
        if let Some(account_short_map) = self.short_quantity.get(&order.account.account_id) {
            if let Some(symbol_volume) = account_short_map.get(&details.symbol_code) {
                let additional_volume = match symbol_volume.to_i32() {
                    None => {
                        return Err(OrderUpdateEvent::OrderRejected {
                            account: order.account,
                            symbol_name: order.symbol_name,
                            symbol_code:  details.symbol_code,
                            order_id: order.id.clone(),
                            reason: "Server Error: Unable to Parse Existing Position Size".to_string(),
                            tag: order.tag,
                            time: Utc::now().to_string(),
                        })
                    }
                    Some(volume) => volume
                };
                details.quantity += additional_volume;
            }
        }
        self.submit_market_order(stream_name, order, details).await;
        Ok(())
    }

    async fn live_enter_short(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        let mut details = match self.rithmic_order_details(mode, stream_name, &order).await {
            Ok(details) => details,
            Err(e) => return Err(e)
        };

        //todo, we need to send the exit order first, so that the strategy engine does not use the order size as the new positions size.
        //check if we are short and add to quantity
        if let Some(account_long_map) = self.long_quantity.get(&order.account.account_id) {
            if let Some(symbol_volume) = account_long_map.get(&details.symbol_code) {
                let additional_volume = match symbol_volume.to_i32() {
                    None => {
                        return Err(OrderUpdateEvent::OrderRejected {
                            account: order.account,
                            symbol_name: order.symbol_name,
                            symbol_code:  details.symbol_code.clone(),
                            order_id: order.id.clone(),
                            reason: "Server Error: Unable to Parse Existing Position Size".to_string(),
                            tag: order.tag,
                            time: Utc::now().to_string(),
                        })
                    }
                    Some(volume) => volume
                };
                details.quantity += additional_volume;
            }
        }
        self.submit_market_order(stream_name, order, details).await;
        Ok(())
    }

    async fn live_exit_short(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        let mut details = match self.rithmic_order_details(mode, stream_name, &order).await {
            Ok(details) => details,
            Err(e) => return Err(e)
        };

        let reject_order = |reason: String| -> Result<(), OrderUpdateEvent> {
            Err(OrderUpdateEvent::OrderRejected {
                account: order.account.clone(),
                symbol_name: order.symbol_name.clone(),
                symbol_code: details.symbol_code.clone(),
                order_id: order.id.clone(),
                reason,
                tag: order.tag.clone(),
                time: Utc::now().to_string(),
            })
        };

        // Check if we have a short position
        if let Some(account_short_map) = self.short_quantity.get(&order.account.account_id) {
            if let Some(symbol_volume) = account_short_map.value().get(&details.symbol_code) {
                let volume = match symbol_volume.value().to_i32() {
                    None => {
                        return reject_order("Server Error: Unable to Parse Existing Position Size".to_string())
                    }
                    Some(volume) => volume
                };

                if volume <= 0 {
                    return reject_order(format!("No Short Position To Exit: {}", details.symbol_code))
                }

                // For shorts, just ensure we don't exit more than we have
                if details.quantity > volume {
                    let time = Utc::now().to_string();
                    details.quantity = volume;
                    let order_update_event = OrderUpdateEvent::OrderUpdated {
                        account: order.account.clone(),
                        symbol_name: order.symbol_name.clone(),
                        symbol_code: details.symbol_code.clone(),
                        order_id: order.id.clone(),
                        update_type: OrderUpdateType::Quantity(Decimal::from_i32(volume).unwrap()),
                        tag: order.tag.clone(),
                        text: String::from("ff_data_server Api adjusted exit quantity to prevent over fill"),
                        time: time.clone(),
                    };
                    let order_event = DataServerResponse::OrderUpdates{event: order_update_event, time};
                    if let Some(sender) = RESPONSE_SENDERS.get(&stream_name) {
                        match sender.send(order_event).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("failed to forward ResponseNewOrder 313 to strategy stream {}", e)
                        }
                    }
                }

                // For short exits, quantity should be positive
                details.quantity = details.quantity.abs();

                self.submit_market_order(stream_name, order, details).await;
                Ok(())
            } else {
                reject_order(format!("No Short Position To Exit: {}", details.symbol_code))
            }
        } else {
            reject_order(format!("No Short Position To Exit: {}", details.symbol_code))
        }
    }

    async fn live_exit_long(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        let mut details = match self.rithmic_order_details(mode, stream_name, &order).await {
            Ok(details) => details,
            Err(e) => return Err(e)
        };

        let reject_order = |reason: String| -> Result<(), OrderUpdateEvent> {
            Err(OrderUpdateEvent::OrderRejected {
                symbol_name: order.symbol_name.clone(),
                symbol_code: details.symbol_code.clone(),
                account: order.account.clone(),
                order_id: order.id.clone(),
                reason,
                tag: order.tag.clone(),
                time: Utc::now().to_string(),
            })
        };

        //check if we are long and adjust quantity
        if let Some(account_long_map) = self.long_quantity.get(&order.account.account_id) {
            if let Some(symbol_volume) = account_long_map.value().get(&details.symbol_code) {
                let volume = match symbol_volume.value().to_i32() {
                    None => {
                        return reject_order("Server Error: Unable to Parse Existing Position Size".to_string())
                    }
                    Some(volume) => volume
                };

                if volume <= 0 {
                    return reject_order(format!("No Short Position To Exit: {}", details.symbol_code))
                }

                // Use absolute values for comparison and adjustment
                if details.quantity.abs() > volume.abs() {
                    details.quantity = volume.abs();  // Keep positive for the order
                    let time = Utc::now().to_string();
                    let order_update_event = OrderUpdateEvent::OrderUpdated {
                        account: order.account.clone(),
                        symbol_name: order.symbol_name.clone(),
                        symbol_code: details.symbol_code.clone(),
                        order_id: order.id.clone(),
                        update_type: OrderUpdateType::Quantity(Decimal::from_i32(volume).unwrap()),
                        tag: order.tag.clone(),
                        text: String::from("ff_data_server Api adjusted exit quantity to prevent over fill"),
                        time: time.clone(),
                    };
                    let order_event = DataServerResponse::OrderUpdates{event: order_update_event, time};
                    if let Some(sender) = RESPONSE_SENDERS.get(&stream_name) {
                        match sender.send(order_event).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("failed to forward ResponseNewOrder 313 to strategy stream {}", e)
                        }
                    }
                }
                self.submit_market_order(stream_name, order, details).await;
                Ok(())
            } else {
                reject_order(format!("No Long Position To Exit: {}", details.symbol_code))
            }
        } else {
            reject_order(format!("No Long Position To Exit: {}", details.symbol_code))
        }
    }

    async fn other_orders(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        let details = match self.rithmic_order_details(mode, stream_name, &order).await {
            Ok(details) => details,
            Err(e) => return Err(e)
        };
        self.submit_order(stream_name, order, details).await
    }

    async fn cancel_orders_on_account(&self, account: Account) {
        const PLANT: SysInfraType = SysInfraType::OrderPlant;
        //Cancel All Orders Request 346
        let req = RequestCancelAllOrders {
            template_id: 346,
            user_msg: vec!["Cancel All Orders".to_string()],
            fcm_id: self.fcm_id.clone(),
            ib_id: self.ib_id.clone(),
            account_id: Some(account.account_id),
            user_type: self.credentials.user_type,
            manual_or_auto: Some(2),
        };
        self.send_message(&PLANT, req).await;
    }

    async fn cancel_order(&self, account: Account, order_id: OrderId) {
        const PLANT: SysInfraType = SysInfraType::OrderPlant;
        //Cancel Order Request 316
        if let Some(account_map) = self.id_to_basket_id_map.get(&account.account_id) {
            if let Some(order) = account_map.get(&order_id) {
                let req = RequestCancelOrder {
                    template_id: 316,
                    user_msg: vec!["Cancel Order".to_string()],
                    window_name: None,
                    fcm_id: self.fcm_id.clone(),
                    ib_id: self.ib_id.clone(),
                    account_id: Some(account.account_id),
                    basket_id: Some(order.value().to_string()),
                    manual_or_auto: Some(2),
                };
                //Cancel Order Request 316
                self.send_message(&PLANT, req).await;
            }
        }
    }

    async fn flatten_all_for(&self, account: Account) {
        const PLANT: SysInfraType = SysInfraType::OrderPlant;
        self.cancel_orders_on_account(account.clone()).await;
        if let Some(tag_map) = self.last_tag.get(&account.account_id) {
            for tag in tag_map.value() {
                let exchange = match get_exchange_by_symbol_name(tag.key()) {
                    None => {
                        eprintln!("Rithmic `flatten_all_for()` error, No exchange found for symbol: {}", tag.key());
                        continue
                    },
                    Some(exchange) => exchange
                };
                let req = RequestExitPosition {
                    template_id: 3504,
                    user_msg: vec!["Flatten All".to_string()],
                    window_name: None,
                    fcm_id: self.fcm_id.clone(),
                    ib_id: self.ib_id.clone(),
                    account_id: Some(account.account_id.clone()),
                    symbol: Some(tag.key().to_string()),
                    exchange: Some(exchange.to_string()),
                    trading_algorithm: None,
                    manual_or_auto: Some(2),
                };
                self.send_message(&PLANT, req).await;
            }
        }
        //Exit Position Request 3504 for all positions
    }

    async fn update_order(&self, account: Account, order_id: OrderId, update: OrderUpdateType) -> Result<(), OrderUpdateEvent> {
        const PLANT: SysInfraType = SysInfraType::OrderPlant;
        let map = self.pending_order_updates.entry(account.brokerage.clone()).or_insert(DashMap::new());
        map.insert(order_id.clone(), update.clone());
        let basket_id = match self.id_to_basket_id_map.get(&account.account_id) {
            None => {
                return Err(OrderUpdateEvent::OrderUpdateRejected {
                    account,
                    order_id,
                    reason: "No basket id found for order id".to_string(),
                    time: Utc::now().to_string(),
                })
            }
            Some(basket_id_map) => match basket_id_map.get(&order_id) {
                None => {
                    return Err(OrderUpdateEvent::OrderUpdateRejected {
                        account,
                        order_id,
                        reason: "No basket id found for order id".to_string(),
                        time: Utc::now().to_string(),
                    })
                }
                Some(basket_id) => basket_id.value().clone()
            },
        };

        match self.orders_open.get(&order_id) {
            None => {
                Err(OrderUpdateEvent::OrderUpdateRejected {
                    account,
                    order_id,
                    reason: "No order found for id".to_string(),
                    time: Utc::now().to_string(),
                })
            }
            Some(order) => {
                let (quantity, limit_price, stop_price) = match update {
                    OrderUpdateType::Quantity(q) => match q.to_i32() {
                        None => {
                            return Err(OrderUpdateEvent::OrderUpdateRejected {
                                account,
                                order_id,
                                reason: "Unable to parse quantity".to_string(),
                                time: Utc::now().to_string(),
                            })
                        }
                        Some(q) => (Some(q), None, None)
                    },
                    OrderUpdateType::LimitPrice(price) => {
                        match price.to_f64() {
                            None => {
                                return Err(OrderUpdateEvent::OrderUpdateRejected {
                                    account,
                                    order_id,
                                    reason: "Unable to parse limit price".to_string(),
                                    time: Utc::now().to_string(),
                                })
                            }
                            Some(price) => (None, Some(price), None)
                        }
                    }
                    OrderUpdateType::TriggerPrice(price) => {
                        match price.to_f64() {
                            None => {
                                return Err(OrderUpdateEvent::OrderUpdateRejected {
                                    account,
                                    order_id,
                                    reason: "Unable to parse trigger price".to_string(),
                                    time: Utc::now().to_string(),
                                })
                            }
                            Some(price) => (None, None, Some(price))
                        }
                    }
                };
                let req = RequestModifyOrder {
                    template_id: 314,
                    user_msg: vec![order.tag.clone()],
                    window_name: None,
                    fcm_id: self.fcm_id.clone(),
                    ib_id: self.ib_id.clone(),
                    account_id: Some(account.account_id),
                    basket_id: Some(basket_id),
                    symbol: order.symbol_code.clone(),
                    exchange: order.exchange.clone(),
                    quantity: quantity,
                    price: limit_price,
                    trigger_price: stop_price,
                    price_type: None,
                    manual_or_auto: Some(2),
                    trailing_stop: None,
                    trail_by_ticks: None,
                    if_touched_symbol: None,
                    if_touched_exchange: None,
                    if_touched_condition: None,
                    if_touched_price_field: None,
                    if_touched_price: None,
                };
                self.send_message(&PLANT, req).await;
                Ok(())
            }
        }
    }
}


