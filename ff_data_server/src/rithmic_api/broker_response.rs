use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::server_features::server_side_brokerage::BrokerApiResponse;
use ff_standard_lib::standardized_types::accounts::{AccountId, AccountInfo, Currency};
use ff_standard_lib::standardized_types::enums::StrategyMode;
use ff_standard_lib::standardized_types::new_types::Volume;
use ff_standard_lib::standardized_types::orders::{Order, OrderUpdateEvent};
use ff_standard_lib::standardized_types::subscriptions::SymbolName;
use ff_standard_lib::StreamName;
use crate::rithmic_api::api_client::RithmicClient;
use crate::rithmic_api::products::{get_available_symbol_names, get_futures_commissions_info, get_intraday_margin, get_overnight_margin, get_symbol_info};

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
        if let Some(account_short_map) = self.short_quantity.get(&order.account_id) {
            if let Some(symbol_volume) = account_short_map.get(&details.symbol) {
                let additional_volume = match symbol_volume.to_i32() {
                    None => {
                        return Err(OrderUpdateEvent::OrderRejected {
                            brokerage: order.brokerage,
                            account_id: order.account_id,
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

        //check if we are short and add to quantity
        if let Some(account_long_map) = self.long_quantity.get(&order.account_id) {
            if let Some(symbol_volume) = account_long_map.get(&details.symbol) {
                let additional_volume = match symbol_volume.to_i32() {
                    None => {
                        return Err(OrderUpdateEvent::OrderRejected {
                            brokerage: order.brokerage,
                            account_id: order.account_id,
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
                brokerage: order.brokerage.clone(),
                account_id: order.account_id.clone(),
                order_id: order.id.clone(),
                reason,
                tag: order.tag.clone(),
                time: Utc::now().to_string(),
            })
        };

        //check if we are short and add to quantity
        if let Some(account_short_map) = self.short_quantity.get(&order.account_id) {
            println!("{:?}", account_short_map);
            println!("{:?}", details.symbol);
            if let Some(symbol_volume) = account_short_map.value().get(&details.symbol) {
                let volume = match symbol_volume.value().to_i32() {
                    None => {
                        return reject_order("Server Error: Unable to Parse Existing Position Size".to_string())
                    }
                    Some(volume) => volume
                };
                if details.quantity > volume {
                    details.quantity = volume;
                }
                self.submit_market_order(stream_name, order, details).await;
            } else {
                return reject_order(format!("No Short Position To Exit: {}", details.symbol))
            }
        } else {
            return reject_order(format!("No Short Position To Exit: {}", details.symbol))
        }
        Ok(())
    }

    async fn live_exit_long(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        let mut details = match self.rithmic_order_details(mode, stream_name, &order).await {
            Ok(details) => details,
            Err(e) => return Err(e)
        };

        let reject_order = |reason: String| -> Result<(), OrderUpdateEvent> {
            Err(OrderUpdateEvent::OrderRejected {
                brokerage: order.brokerage.clone(),
                account_id: order.account_id.clone(),
                order_id: order.id.clone(),
                reason,
                tag: order.tag.clone(),
                time: Utc::now().to_string(),
            })
        };

        //check if we are short and add to quantity
        if let Some(account_long_map) = self.long_quantity.get(&order.account_id) {
            if let Some(symbol_volume) = account_long_map.value().get(&details.symbol) {
                let volume = match symbol_volume.value().to_i32() {
                    None => {
                        return reject_order("Server Error: Unable to Parse Existing Position Size".to_string())
                    }
                    Some(volume) => volume
                };
                if details.quantity > volume {
                    details.quantity = volume;
                }
                self.submit_market_order(stream_name, order, details).await;
            } else {
                return reject_order(format!("No Long Position To Exit: {}", details.symbol))
            }
        } else {
            return reject_order(format!("No Long Position To Exit: {}", details.symbol))
        }
        Ok(())
    }
}


