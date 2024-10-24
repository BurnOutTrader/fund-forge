use std::collections::BTreeMap;
use async_trait::async_trait;
use ff_standard_lib::helpers::converters::{fund_forge_formatted_symbol_name};
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::server_features::server_side_datavendor::VendorApiResponse;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::enums::{MarketType, StrategyMode, SubscriptionResolutionType};
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol, SymbolName};
use ff_standard_lib::StreamName;
use tokio::sync::broadcast;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use tokio::time::sleep;
use std::time::Duration;
use rust_decimal_macros::dec;
use ff_standard_lib::server_features::database::DATA_STORAGE;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use crate::stream_tasks::{subscribe_stream, unsubscribe_stream};
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
        let available_subscription_1 = DataSubscription::new(SymbolName::from("AUD-CAD"), DataVendor::Test, Resolution::Instant, BaseDataType::Quotes, MarketType::Forex);
        let available_subscription_2 = DataSubscription::new(SymbolName::from("EUR-USD"), DataVendor::Test, Resolution::Instant, BaseDataType::Quotes, MarketType::Forex);

        if subscription != available_subscription_1 && subscription != available_subscription_2 {
            return DataServerResponse::SubscribeResponse{ success: false, subscription: subscription.clone(), reason: Some(format!("This subscription is not available with DataVendor::Test: {}", subscription))}
        }

        if !self.data_feed_broadcasters.contains_key(&subscription) {
            let (sender, receiver) = broadcast::channel(100);
            // We can use the subscribe stream function to pass the new receiver to our stream handler task for the stream_name.
            subscribe_stream(&stream_name, subscription.clone(), receiver).await;
            self.data_feed_broadcasters.insert(subscription.clone(), sender);
            println!("Subscribing: {}", subscription);
        } else {
            // If we already have a running task, we don't need a new one, we just subscribe to the broadcaster
            if let Some(broadcaster) = self.data_feed_broadcasters.get(&subscription) {
                let receiver = broadcaster.value().subscribe();
                // We can use the subscribe stream function to pass the new receiver to our stream handler task for the stream_name.
                subscribe_stream(&stream_name, subscription.clone(), receiver).await;
            }
            return DataServerResponse::SubscribeResponse{ success: true, subscription: subscription.clone(), reason: None}
        }

        println!("data_feed_subscribe Starting loop");
        let subscription_clone = subscription.clone();
        let subscription_clone_2 = subscription.clone();
        let broadcasters = self.data_feed_broadcasters.clone();
        let broadcaster = self.data_feed_broadcasters.get(&subscription).unwrap().value().clone();
        self.data_feed_tasks.insert(subscription.clone(), tokio::task::spawn(async move {
            let naive_dt_1 = NaiveDate::from_ymd_opt(2024, 6, 01).unwrap().and_hms_opt(0, 0, 0).unwrap();
            let from_time = Utc.from_utc_datetime(&naive_dt_1);

            let naive_dt_2 = NaiveDate::from_ymd_opt(2024, 6, 6).unwrap().and_hms_opt(0, 0, 0).unwrap();
            let to_time = Utc.from_utc_datetime(&naive_dt_2);

            let mut last_time = from_time;
            'main_loop: while last_time < to_time {
                let data = match DATA_STORAGE.get().expect("DATA_STORAGE not initialized").get_data_range(&subscription.symbol, &subscription.resolution, &subscription.base_data_type, from_time, to_time).await {
                    Ok(data) => data,
                    Err(e) => {
                        eprintln!("Failed to get test data: {}", e);
                        return
                    }
                };

                for mut base_data in data {
                    last_time = base_data.time_closed_utc();
                    match base_data {
                        BaseDataEnum::Quote(ref mut quote) => {
                            if broadcaster.receiver_count() > 0 {
                                quote.time = Utc::now().to_string();
                                match broadcaster.send(base_data) {
                                    Ok(_) => {}
                                    Err(_) => {}
                                }
                                sleep(Duration::from_millis(5)).await;
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
        }));
        DataServerResponse::SubscribeResponse{ success: true, subscription: subscription_clone_2.clone(), reason: None}
    }

    async fn data_feed_unsubscribe(&self,  _mode: StrategyMode, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
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
    async fn update_historical_data_for(subscription: DataSubscription, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Option<BTreeMap<i64, BaseDataEnum>>, FundForgeError> {
        todo!()
    }
}