use std::sync::Arc;
use async_trait::async_trait;
use lazy_static::lazy_static;
use rust_decimal_macros::dec;
use crate::apis::brokerage::broker_enum::Brokerage;
use crate::apis::brokerage::server_side_brokerage::BrokerApiResponse;
use crate::apis::data_vendor::datavendor_enum::DataVendor;
use crate::apis::data_vendor::server_side_datavendor::VendorApiResponse;
use crate::helpers::decimal_calculators::round_to_decimals;
use crate::standardized_types::accounts::ledgers::{AccountId, AccountInfo, Currency};
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::data_server_messaging::DataServerResponse;
use crate::standardized_types::enums::{MarketType, Resolution, SubscriptionResolutionType};
use crate::standardized_types::subscriptions::{Symbol, SymbolName};
use crate::standardized_types::symbol_info::SymbolInfo;
use crate::standardized_types::Volume;

lazy_static! {
    pub static ref TEST_CLIENT: Arc<TestApiClient> = Arc::new(TestApiClient{});
}

pub struct TestApiClient {}
#[async_trait]
impl BrokerApiResponse for TestApiClient {
    async fn symbols_response(&self, stream_name: String, market_type: MarketType, callback_id: u64) -> DataServerResponse {
        DataServerResponse::Symbols {
            callback_id,
            symbols: vec![
                Symbol::new("EUR-USD".to_string(), DataVendor::Test, MarketType::Forex),
                Symbol::new("AUD-USD".to_string(), DataVendor::Test, MarketType::Forex),
                Symbol::new("AUD-CAD".to_string(), DataVendor::Test, MarketType::Forex),
            ],
            market_type,
        }
    }

    async fn account_info_response(&self, stream_name: String, account_id: AccountId, callback_id: u64) -> DataServerResponse {
        let account_info = AccountInfo {
            brokerage: Brokerage::Test,
            cash_value: dec!(100000),
            cash_available:dec!(100000),
            currency: Currency::USD,
            cash_used: dec!(0),
            positions: vec![],
            account_id,
            is_hedging: false,
        };
        DataServerResponse::AccountInfo {
            callback_id,
            account_info,
        }
    }

    async fn symbol_info_response(&self, stream_name: String, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        let symbol_info = SymbolInfo {
            symbol_name,
            pnl_currency: Currency::USD,
            value_per_tick: dec!(0.1),
            tick_size: dec!(0.00001),
        };
        DataServerResponse::SymbolInfo {
            callback_id,
            symbol_info,
        }
    }

    async fn margin_required_historical_response(&self, stream_name: String, symbol_name: SymbolName, quantity: Volume, callback_id: u64) -> DataServerResponse {
        let value = round_to_decimals(quantity  * dec!(100.0), 2);
        DataServerResponse::MarginRequired {
            callback_id,
            symbol_name,
            price: value,
        }
    }

    async fn margin_required_live_response(&self, stream_name: String, symbol_name: SymbolName, quantity: Volume, callback_id: u64) -> DataServerResponse {
        todo!()
    }
}

#[async_trait]
impl VendorApiResponse for TestApiClient {
    async fn symbols_response(&self, stream_name: String, market_type: MarketType, callback_id: u64) -> DataServerResponse{
        DataServerResponse::Symbols {
            callback_id,
            symbols: vec![
                Symbol::new("EUR-USD".to_string(), DataVendor::Test, MarketType::Forex),
                Symbol::new("AUD-USD".to_string(), DataVendor::Test, MarketType::Forex),
                Symbol::new("AUD-CAD".to_string(), DataVendor::Test, MarketType::Forex),
            ],
            market_type,
        }
    }

    async fn resolutions_response(&self, stream_name: String, market_type: MarketType, callback_id: u64) -> DataServerResponse {
        let res = SubscriptionResolutionType {
            base_data_type: BaseDataType::Quotes,
            resolution: Resolution::Instant,
        };
        DataServerResponse::Resolutions {
            callback_id,
            subscription_resolutions_types: vec![res],
            market_type,
        }
    }

    async fn markets_response(&self, stream_name: String, callback_id: u64) -> DataServerResponse {
        DataServerResponse::Markets {
            callback_id,
            markets: vec![MarketType::Forex],
        }
    }

    async fn decimal_accuracy_response(&self, stream_name: String, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        DataServerResponse::DecimalAccuracy {
            callback_id,
            accuracy: 5,
        }
    }

    async fn tick_size_response(&self, stream_name: String, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        DataServerResponse::TickSize {
            callback_id,
            tick_size: dec!(0.00001),
        }
    }
}
