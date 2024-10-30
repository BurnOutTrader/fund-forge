use std::collections::BTreeMap;
use std::str::FromStr;
use async_trait::async_trait;
use chrono::{DateTime, Datelike, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::server_features::server_side_datavendor::VendorApiResponse;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::enums::{MarketType, StrategyMode, SubscriptionResolutionType};
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol, SymbolName};
use ff_standard_lib::StreamName;
use crate::oanda_api::api_client::OandaClient;
use crate::oanda_api::base_data_converters::{candle_from_candle, oanda_quotebar_from_candle};
use crate::oanda_api::download::generate_urls;
use crate::server_features::database::DATA_STORAGE;

#[async_trait]
impl VendorApiResponse for OandaClient {
    #[allow(unused)]
    async fn symbols_response(&self, mode: StrategyMode, stream_name: StreamName, market_type: MarketType, time: Option<DateTime<Utc>>, callback_id: u64) -> DataServerResponse {
        let mut symbols: Vec<Symbol> = Vec::new();
        for symbol in &self.instruments {
            let symbol = Symbol::new(symbol.key().clone(), DataVendor::Oanda, symbol.value().market_type.clone());
            symbols.push(symbol);
        }
        DataServerResponse::Symbols {
            callback_id,
            symbols,
            market_type,
        }
    }
    #[allow(unused)]
    async fn resolutions_response(&self, mode: StrategyMode, stream_name: StreamName, market_type: MarketType, callback_id: u64) -> DataServerResponse {
        let resolutions = vec![
            SubscriptionResolutionType::new(Resolution::Seconds(5), BaseDataType::QuoteBars),
        ];
        DataServerResponse::Resolutions {
            callback_id,
            market_type,
            subscription_resolutions_types: resolutions,
        }
    }
    #[allow(unused)]
    async fn markets_response(&self, mode: StrategyMode, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        DataServerResponse::Markets {
            callback_id,
            markets: vec![MarketType::CFD, MarketType::Forex],
        }
    }
    #[allow(unused)]
    async fn decimal_accuracy_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        if let Some(instrument) = self.instruments.get(&symbol_name) {
            DataServerResponse::DecimalAccuracy {
                callback_id,
                accuracy: instrument.display_precision.clone(),
            }
        } else {
            DataServerResponse::Error {
                callback_id,
                error: FundForgeError::ClientSideErrorDebug(format!("Oanda Symbol not found: {}", symbol_name)),
            }
        }
    }
    #[allow(unused)]
    async fn tick_size_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        let instrument = match self.instruments.get(&symbol_name) {
            Some(i) => i,
            None => return DataServerResponse::Error{callback_id, error: FundForgeError::ClientSideErrorDebug(format!("Instrument not found: {}", symbol_name))},
        };

        // Using string formatting with error handling
        let tick_size = match Decimal::from_str(&format!("0.{:0>precision$}1", "", precision = instrument.display_precision as usize)) {
            Ok(size) => size,
            Err(e) => return DataServerResponse::Error{callback_id, error: FundForgeError::ClientSideErrorDebug(format!("Failed to calculate tick size: {}", e))},
        };

        DataServerResponse::TickSize{
            callback_id,
            tick_size,
        }
    }
    #[allow(unused)]
    async fn data_feed_subscribe(&self, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
        todo!()
    }

    #[allow(unused)]
    async fn data_feed_unsubscribe(&self, mode: StrategyMode, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
        todo!()
    }

    #[allow(unused)]
    async fn base_data_types_response(&self, mode: StrategyMode, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        DataServerResponse::BaseDataTypes {
            callback_id,
            base_data_types: vec![BaseDataType::QuoteBars],
        }
    }

    #[allow(unused)]
    async fn logout_command_vendors(&self, stream_name: StreamName) {
        todo!()
    }

    #[allow(unused)]
    async fn session_market_hours_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, date_time: DateTime<Utc>, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    #[allow(unused)]
    async fn update_historical_data_for(&self, symbol: Symbol, base_data_type: BaseDataType, resolution: Resolution) {
        println!("Downloading historical data for: {}", symbol.name);
        let earliest_oanda_data = || {
            let utc_time_string = "2005-01-01 00:00:00.000000";
            let utc_time_naive = NaiveDateTime::parse_from_str(utc_time_string, "%Y-%m-%d %H:%M:%S%.f").unwrap();
            DateTime::<Utc>::from_naive_utc_and_offset(utc_time_naive, Utc)
        };
        // if we have data start from last time, else start from oanda's earliest date
        let mut last_bar_time = match DATA_STORAGE.get().unwrap().get_latest_data_time(&symbol, &resolution, &base_data_type).await {
            // if we have no data, we start from the earliest date available on oanda
            Err(_) => earliest_oanda_data(),
            // if we have data, we start from the last time in the data
            Ok(time) => match time {
                Some(time) => time,
                None => earliest_oanda_data()
            }
        };

        let urls = generate_urls(symbol.clone(), resolution.clone(), base_data_type, &last_bar_time).await;

        let mut new_data: BTreeMap<DateTime<Utc>, BaseDataEnum> = BTreeMap::new();
        for url in &urls {
            println!("Downloading data from: {}", url);
            let response = self.send_rest_request(&url).await.unwrap();

            if !response.status().is_success() {
                continue;
            }

            let content = response.text().await.unwrap();
            let json: serde_json::Value = serde_json::from_str(&content).unwrap();
            let candles = json["candles"].as_array().unwrap();

            if candles.len() == 0 {
                continue;
            }

            for price_data in candles {
                let is_closed = price_data["complete"].as_bool().unwrap();
                if !is_closed {
                    continue;
                }
                let bar: BaseDataEnum = match base_data_type {
                    BaseDataType::QuoteBars => match oanda_quotebar_from_candle(&price_data, symbol.clone(), resolution.clone()) {
                        Ok(quotebar) => BaseDataEnum::QuoteBar(quotebar),
                        Err(e) => {
                            println!("Error processing quotebar: {}", e);
                            continue
                        }
                    },
                    BaseDataType::Candles => match candle_from_candle(&price_data, symbol.clone(), resolution.clone()) {
                        Ok(candle) => BaseDataEnum::Candle(candle),
                        Err(e) => {
                            println!("Error processing candle: {}", e);
                            continue
                        }
                    },
                    _ => {
                        println!("price_data_type: History not supported for broker");
                        continue
                    }
                };

                // we need to use this time_object_unchanged() when downloading to be sure we are not changing the data on download and also again when using.

                let new_bar_time = bar.time_utc();
                if last_bar_time.day() != new_bar_time.day() && !new_data.is_empty() {
                    let data_vec: Vec<BaseDataEnum> = new_data.values().map(|x| x.clone()).collect();
                    match DATA_STORAGE.get().unwrap().save_data_bulk(data_vec).await {
                        Ok(_) => {},
                        Err(e) => eprintln!("Error saving oanda data: {}", e)
                    }
                    new_data.clear();
                }

                last_bar_time = bar.time_utc();
                new_data.entry(new_bar_time).or_insert(bar);
            }
        }
    }
}