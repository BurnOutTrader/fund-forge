use async_trait::async_trait;
use ff_standard_lib::helpers::converters::{fund_forge_formatted_symbol_name};
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use crate::server_features::server_side_datavendor::VendorApiResponse;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::enums::{MarketType, StrategyMode, PrimarySubscription};
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol, SymbolName};
use ff_standard_lib::StreamName;
use chrono::{DateTime,Utc};
use indicatif::{ProgressBar};
use rust_decimal_macros::dec;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use crate::stream_tasks::{unsubscribe_stream};
use crate::test_api::api_client::TestApiClient;

#[async_trait]
impl VendorApiResponse for TestApiClient {
    async fn symbols_response(&self,  _mode: StrategyMode, _stream_name: StreamName, market_type: MarketType, _time: Option<DateTime<Utc>>, callback_id: u64) -> DataServerResponse{
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

    async fn resolutions_response(&self, _mode: StrategyMode, _stream_name: StreamName, market_type: MarketType, callback_id: u64) -> DataServerResponse {
        let res = PrimarySubscription {
            base_data_type: BaseDataType::Quotes,
            resolution: Resolution::Instant,
        };
        DataServerResponse::Resolutions {
            callback_id,
            subscription_resolutions_types: vec![res],
            market_type,
        }
    }

    async fn markets_response(&self, _mode: StrategyMode, _stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        DataServerResponse::Markets {
            callback_id,
            markets: vec![MarketType::Forex],
        }
    }

    async fn decimal_accuracy_response(&self, _mode: StrategyMode, _stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        let _symbol_name = fund_forge_formatted_symbol_name(&symbol_name);
        DataServerResponse::DecimalAccuracy {
            callback_id,
            accuracy: 5,
        }
    }

    async fn tick_size_response(&self,  _mode: StrategyMode, _stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        let _symbol_name = fund_forge_formatted_symbol_name(&symbol_name);
        DataServerResponse::TickSize {
            callback_id,
            tick_size: dec!(0.00001),
        }
    }

    async fn data_feed_subscribe(&self, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
       todo!()
    }

    async fn data_feed_unsubscribe(&self, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
        let mut empty_broadcaster = false;
        let mut success = false;
        unsubscribe_stream(&stream_name, &subscription).await;
        if let Some(broadcaster) = self.data_feed_broadcasters.get(&subscription) {
            if broadcaster.receiver_count() == 0 {
                empty_broadcaster = true;
            }
            success = true
        }
        if empty_broadcaster {
            self.data_feed_broadcasters.remove(&subscription);
        }
        match success {
            true =>  DataServerResponse::UnSubscribeResponse{ success: true, subscription, reason: None},
            false => DataServerResponse::UnSubscribeResponse{ success: false, subscription: subscription.clone(), reason: Some(format!("There is no active subscription for: {}", subscription))}
        }
    }

    async fn base_data_types_response(&self,  _mode: StrategyMode, _stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        DataServerResponse::BaseDataTypes {
            callback_id,
            base_data_types: vec![BaseDataType::Quotes],
        }
    }

    async fn logout_command_vendors(&self, _stream_name: StreamName) {
        let mut to_remove = vec![];
        for broadcaster in self.data_feed_broadcasters.iter() {
            if broadcaster.receiver_count() == 0 {
                to_remove.push(broadcaster.key().clone());
            }
        }
        for sub in to_remove {
            self.data_feed_broadcasters.remove(&sub);
            if let Some((_subscription, task)) = self.data_feed_tasks.remove(&sub) {
                task.abort();
            }
        }
    }

    #[allow(unused)]
    async fn session_market_hours_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, date_time: DateTime<Utc>, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    #[allow(unused)]
    async fn update_historical_data(&self, symbol: Symbol, base_data_type: BaseDataType, resolution: Resolution, from: DateTime<Utc>, to: DateTime<Utc>, from_back: bool ,progress_bar: ProgressBar, is_bulk_download: bool) -> Result<(), FundForgeError> {
        todo!()
    }
}