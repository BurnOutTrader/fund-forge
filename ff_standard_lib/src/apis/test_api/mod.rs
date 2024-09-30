use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use chrono::{NaiveDate, TimeZone, Utc};
use dashmap::DashMap;
use lazy_static::lazy_static;
use rust_decimal_macros::dec;
use tokio::sync::mpsc::{Sender};
use tokio::time::sleep;
use crate::apis::brokerage::broker_enum::Brokerage;
use crate::apis::brokerage::server_side_brokerage::BrokerApiResponse;
use crate::apis::data_vendor::datavendor_enum::DataVendor;
use crate::apis::data_vendor::server_side_datavendor::VendorApiResponse;
use crate::apis::StreamName;
use crate::helpers::converters::{fund_forge_formatted_symbol_name, load_as_bytes};
use crate::helpers::decimal_calculators::round_to_decimals;
use crate::helpers::get_data_folder;
use crate::servers::internal_broadcaster::StaticInternalBroadcaster;
use crate::standardized_types::accounts::ledgers::{AccountId, AccountInfo, Currency};
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::data_server_messaging::{DataServerResponse, FundForgeError};
use crate::standardized_types::enums::{MarketType, Resolution, StrategyMode, SubscriptionResolutionType};
use crate::standardized_types::subscriptions::{DataSubscription, Symbol, SymbolName};
use crate::standardized_types::symbol_info::SymbolInfo;
use crate::standardized_types::Volume;

lazy_static! {
    pub static ref TEST_CLIENT: Arc<TestApiClient> = Arc::new(TestApiClient::new());
}

pub struct TestApiClient {
    data_feed_broadcasters: Arc<DashMap<DataSubscription, Arc<StaticInternalBroadcaster<DataServerResponse>>>>
}

impl TestApiClient {
    fn new() -> Self {
        Self {
            data_feed_broadcasters: Default::default(),
        }
    }
}

#[async_trait]
impl BrokerApiResponse for TestApiClient {
    async fn symbols_response(&self, mode: StrategyMode, _stream_name: String, market_type: MarketType, callback_id: u64) -> DataServerResponse {
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

    async fn account_info_response(&self, mode: StrategyMode, _stream_name: String, account_id: AccountId, callback_id: u64) -> DataServerResponse {
        let account_info = AccountInfo {
            brokerage: Brokerage::Test,
            cash_value: dec!(100000),
            cash_available:dec!(100000),
            currency: Currency::USD,
            cash_used: dec!(0),
            positions: vec![],
            account_id,
            is_hedging: false,
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

    async fn symbol_info_response(
        &self,
        mode: StrategyMode,
        _stream_name: String,
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
        };

        DataServerResponse::SymbolInfo {
            callback_id,
            symbol_info,
        }
    }

    async fn margin_required_response(
        &self,
        mode: StrategyMode,
        _stream_name: String,
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
        let margin_required = round_to_decimals(quantity / dec!(100.0), 2);

        DataServerResponse::MarginRequired {
            callback_id,
            symbol_name,
            price: margin_required,  // Here price represents the margin required
        }
    }

    async fn accounts_response(&self, mode: StrategyMode,stream_name: String, callback_id: u64) -> DataServerResponse {
       DataServerResponse::Accounts {callback_id, accounts: vec!["TestAccount1".to_string(), "TestAccount2".to_string()]}
    }
}

#[async_trait]
impl VendorApiResponse for TestApiClient {
    async fn symbols_response(&self,  mode: StrategyMode, _stream_name: String, market_type: MarketType, callback_id: u64) -> DataServerResponse{
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

    async fn resolutions_response(&self, mode: StrategyMode, _stream_name: String, market_type: MarketType, callback_id: u64) -> DataServerResponse {
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

    async fn markets_response(&self, mode: StrategyMode, _stream_name: String, callback_id: u64) -> DataServerResponse {
        DataServerResponse::Markets {
            callback_id,
            markets: vec![MarketType::Forex],
        }
    }

    async fn decimal_accuracy_response(&self, mode: StrategyMode, _stream_name: String, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        let symbol_name = fund_forge_formatted_symbol_name(&symbol_name);
        DataServerResponse::DecimalAccuracy {
            callback_id,
            accuracy: 5,
        }
    }

    async fn tick_size_response(&self,  mode: StrategyMode, _stream_name: String, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        let symbol_name = fund_forge_formatted_symbol_name(&symbol_name);
        DataServerResponse::TickSize {
            callback_id,
            tick_size: dec!(0.00001),
        }
    }

    async fn data_feed_subscribe(&self,  mode: StrategyMode, stream_name: String, subscription: DataSubscription, sender: Sender<DataServerResponse>) -> DataServerResponse {
        let available_subscription_1 = DataSubscription::new(SymbolName::from("AUD-CAD"), DataVendor::Test, Resolution::Instant, BaseDataType::Quotes, MarketType::Forex);
        let available_subscription_2 = DataSubscription::new(SymbolName::from("EUR-USD"), DataVendor::Test, Resolution::Instant, BaseDataType::Quotes, MarketType::Forex);
        if subscription != available_subscription_1 && subscription != available_subscription_2 {
            return DataServerResponse::SubscribeResponse{ success: false, subscription: subscription.clone(), reason: Some(format!("This subscription is not available with DataVendor::Test: {}", subscription))}
        }
        if !self.data_feed_broadcasters.contains_key(&subscription) {
            self.data_feed_broadcasters.insert(subscription.clone(), Arc::new(StaticInternalBroadcaster::new()));
            self.data_feed_broadcasters.get(&subscription).unwrap().value().subscribe(stream_name, sender).await;
            println!("Subscribing: {}", subscription);
        } else {
            // If we already have a running task, we dont need a new one, we just subscribe to the broadcaster
            self.data_feed_broadcasters.get(&subscription).unwrap().value().subscribe(stream_name, sender).await;
            return DataServerResponse::SubscribeResponse{ success: true, subscription: subscription.clone(), reason: None}
        }
        println!("data_feed_subscribe Starting loop");
        let subscription_clone = subscription.clone();
        let subscription_clone_2 = subscription.clone();
        let broadcasters = self.data_feed_broadcasters.clone();
        let broadcaster = self.data_feed_broadcasters.get(&subscription).unwrap().value().clone();
        tokio::task::spawn(async move {
            let naive_dt_1 = NaiveDate::from_ymd_opt(2024, 6, 01).unwrap().and_hms_opt(0, 0, 0).unwrap();
            let utc_dt_1 = Utc.from_utc_datetime(&naive_dt_1);

            let naive_dt_2 = NaiveDate::from_ymd_opt(2024, 8, 31).unwrap().and_hms_opt(0, 0, 0).unwrap();
            let utc_dt_2 = Utc.from_utc_datetime(&naive_dt_2);

            let mut last_time = utc_dt_1;
            'main_loop: while last_time < utc_dt_2 {
                let data_folder = PathBuf::from(get_data_folder());
                let file = BaseDataEnum::file_path(&data_folder, &subscription, &last_time).unwrap();
                let data = load_as_bytes(file.clone()).unwrap();
                let month_time_slices = BaseDataEnum::from_array_bytes(&data).unwrap();

                for mut base_data in month_time_slices {
                    last_time = base_data.time_created_utc();
                    match base_data {
                        BaseDataEnum::Quote(ref mut quote) => {
                            if broadcaster.has_subscribers() {
                                quote.time = Utc::now().to_string();
                                broadcaster.broadcast(DataServerResponse::BaseDataUpdates(base_data)).await;
                                sleep(Duration::from_millis(20)).await;
                            } else {
                                println!("No subscribers");
                                break 'main_loop;
                            }
                        }
                        _ => {}
                    }
                }
            }
            broadcasters.remove(&subscription_clone);
        });
        DataServerResponse::SubscribeResponse{ success: true, subscription: subscription_clone_2.clone(), reason: None}
    }

    async fn data_feed_unsubscribe(&self,  mode: StrategyMode, stream_name: String, subscription: DataSubscription) -> DataServerResponse {
        if let Some(broadcaster) = self.data_feed_broadcasters.get(&subscription) {
            broadcaster.unsubscribe(stream_name).await;
            return DataServerResponse::UnSubscribeResponse{ success: true, subscription, reason: None}
        }
        DataServerResponse::UnSubscribeResponse{ success: false, subscription: subscription.clone(), reason: Some(format!("There is no active subscription for: {}", subscription))}
    }

    async fn base_data_types_response(&self,  mode: StrategyMode, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        DataServerResponse::BaseDataTypes {
            callback_id,
            base_data_types: vec![BaseDataType::Quotes],
        }
    }
}