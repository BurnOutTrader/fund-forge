use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal_macros::dec;
use ff_standard_lib::helpers::converters::fund_forge_formatted_symbol_name;
use ff_standard_lib::helpers::decimal_calculators::round_to_decimals;
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::server_features::server_side_brokerage::BrokerApiResponse;
use ff_standard_lib::server_features::server_side_datavendor::VendorApiResponse;
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::enums::StrategyMode;
use ff_standard_lib::standardized_types::new_types::Volume;
use ff_standard_lib::standardized_types::orders::{Order, OrderUpdateEvent};
use ff_standard_lib::standardized_types::subscriptions::SymbolName;
use ff_standard_lib::standardized_types::symbol_info::{CommissionInfo, SymbolInfo};
use ff_standard_lib::strategies::ledgers::{AccountId, AccountInfo, Currency};
use ff_standard_lib::StreamName;
use crate::test_api::api_client::TestApiClient;

#[async_trait]
impl BrokerApiResponse for TestApiClient {
    async fn symbol_names_response(&self, _mode: StrategyMode, _time: Option<DateTime<Utc>>, _stream_name: StreamName,  callback_id: u64) -> DataServerResponse {
        DataServerResponse::SymbolNames {
            callback_id,
            symbol_names: vec![
                "EUR-USD".to_string(),
                "AUD-USD".to_string(),
                "AUD-CAD".to_string(),
            ],
        }
    }


    async fn account_info_response(&self, _mode: StrategyMode, _stream_name: StreamName, account_id: AccountId, callback_id: u64) -> DataServerResponse {
        let account_info = AccountInfo {
            brokerage: Brokerage::Test,
            cash_value: dec!(100000),
            cash_available:dec!(100000),
            currency: Currency::USD,
            open_pnl: dec!(0.0),
            booked_pnl: dec!(0.0),
            day_open_pnl: dec!(0.0),
            day_booked_pnl: dec!(0.0),
            cash_used: dec!(0),
            positions: vec![],
            account_id,
            is_hedging: false,
            leverage: 0,
            buy_limit: None,
            sell_limit: None,
            max_orders: None,
            daily_max_loss: None,
            daily_max_loss_reset_time: None,
        };
        DataServerResponse::AccountInfo {
            callback_id,
            account_info,
        }
    }

    async fn paper_account_init(&self, account_id: AccountId, callback_id: u64) -> DataServerResponse {
       DataServerResponse::PaperAccountInit {
           account_info: AccountInfo {
               brokerage: Brokerage::Test,
               cash_value: dec!(100000),
               cash_available: dec!(100000),
               currency: Currency::USD,
               open_pnl: dec!(0),
               booked_pnl: dec!(0),
               day_open_pnl: dec!(0),
               day_booked_pnl: dec!(0),
               cash_used: dec!(0),
               positions: vec![],
               account_id,
               is_hedging: false,
               leverage: 1,
               buy_limit: None,
               sell_limit: None,
               max_orders: None,
               daily_max_loss: None,
               daily_max_loss_reset_time: None,
           },
           callback_id
       }
    }

    async fn symbol_info_response(
        &self,
        _mode: StrategyMode,
        _stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse {
        let symbol_name = fund_forge_formatted_symbol_name(&symbol_name);
        let (pnl_currency, value_per_tick, tick_size) = match symbol_name.as_str() {
            "EUR-USD" => (Currency::USD, dec!(1.0), dec!(0.0001)), // EUR/USD with $1 per tick
            "AUD-CAD" => (Currency::USD, dec!(1.0), dec!(0.0001)), // AUD/CAD with $1 per tick (approximate)
            _ => (Currency::USD, dec!(0.1), dec!(0.00001))         // Default values
        };

        let symbol_info = SymbolInfo {
            symbol_name,
            pnl_currency,
            value_per_tick,
            tick_size,
            decimal_accuracy: 5,
        };

        DataServerResponse::SymbolInfo {
            callback_id,
            symbol_info,
        }
    }

    async fn intraday_margin_required_response(
        &self,
        _mode: StrategyMode,
        _stream_name: StreamName,
        symbol_name: SymbolName,
        quantity: Volume,
        callback_id: u64
    ) -> DataServerResponse {
        // Ensure quantity is not zero
        let symbol_name = fund_forge_formatted_symbol_name(&symbol_name);
        if quantity == dec!(0) {
            return DataServerResponse::Error {
                callback_id,
                error: FundForgeError::ClientSideErrorDebug("Quantity cannot be 0".to_string())
            };
        }

        // Assuming 100:1 leverage, calculate margin required
        // You may want to factor in symbol-specific prices if available
        let margin_required = round_to_decimals(quantity * dec!(100.0), 2);

        DataServerResponse::IntradayMarginRequired {
            callback_id,
            symbol_name,
            price: Some(margin_required),  // Here price represents the margin required
        }
    }

    async fn overnight_margin_required_response(
        &self,
        mode: StrategyMode,
        stream_name: StreamName,
        symbol_name: SymbolName,
        quantity: Volume,
        callback_id: u64
    ) -> DataServerResponse {
        self.intraday_margin_required_response( mode, stream_name, symbol_name, quantity, callback_id).await
    }

    async fn accounts_response(&self, _mode: StrategyMode,_stream_name: StreamName, callback_id: u64) -> DataServerResponse {
       DataServerResponse::Accounts {callback_id, accounts: vec!["TestAccount1".to_string(), "TestAccount2".to_string()]}
    }

    async fn logout_command(&self, stream_name: StreamName) {
        self.logout_command_vendors(stream_name).await;
    }

    async fn commission_info_response(&self, _mode: StrategyMode, _stream_name: StreamName, _symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        DataServerResponse::CommissionInfo {
            callback_id,
            commission_info: CommissionInfo {
                per_side: dec!(0.0),
                currency: Currency::USD,
            },
        }
    }
    #[allow(unused)]
    async fn live_market_order(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        todo!()
    }

    async fn buy_market_order(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        todo!()
    }

    async fn sell_market_order(&self, stream_name: StreamName, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
        todo!()
    }
}