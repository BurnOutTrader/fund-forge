use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::atomic::Ordering;
use async_trait::async_trait;
use chrono::{DateTime, Datelike, Duration, NaiveDateTime, Utc};
use indicatif::ProgressBar;
use rust_decimal::Decimal;
use tokio::sync::broadcast;
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
use crate::oanda_api::api_client::{OandaClient, OANDA_IS_CONNECTED};
use crate::oanda_api::base_data_converters::{candle_from_candle, oanda_quotebar_from_candle};
use crate::oanda_api::download::{generate_url, generate_urls};
use crate::oanda_api::get_requests::oanda_clean_instrument;
use crate::oanda_api::support_and_conversions::{add_time_to_date, resolution_to_oanda_interval};
use crate::server_features::database::DATA_STORAGE;
use crate::stream_tasks::{subscribe_stream, unsubscribe_stream};

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
        let subscription_resolutions_types = match mode {
            StrategyMode::Backtest => vec![SubscriptionResolutionType::new(Resolution::Seconds(5), BaseDataType::QuoteBars)],
            StrategyMode::LivePaperTrading | StrategyMode::Live => vec![SubscriptionResolutionType::new(Resolution::Instant, BaseDataType::Quotes)],
        };

        DataServerResponse::Resolutions {
            callback_id,
            market_type,
            subscription_resolutions_types,
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
        if !OANDA_IS_CONNECTED.load(Ordering::SeqCst) {
            return DataServerResponse::SubscribeResponse {
                success: false,
                subscription,
                reason: Some("Oanda is not connected".to_string()),
            };
        }
        if subscription.subscription_resolution_type() != SubscriptionResolutionType::new(Resolution::Instant, BaseDataType::Quotes) {
            return DataServerResponse::UnSubscribeResponse {
                success: false,
                subscription,
                reason: Some("Live Oanda only supports quotes".to_string()),
            };
        }

        let mut is_subscribed = true;
        if let Some(broadcaster) = self.quote_feed_broadcasters.get(&subscription.symbol.name) {
            let receiver = broadcaster.value().subscribe();
            subscribe_stream(&stream_name, subscription.clone(), receiver).await;
        } else {
            let (sender, receiver) = broadcast::channel(500);
            self.quote_feed_broadcasters.insert(subscription.symbol.name.clone(), sender);
            subscribe_stream(&stream_name, subscription.clone(), receiver).await;
            is_subscribed = false;
        }

        if !is_subscribed {
            let mut keys: Vec<SymbolName> = self.quote_feed_broadcasters.iter().map(|entry| entry.key().clone()).collect();
            if keys.len() == 20 {
                return DataServerResponse::UnSubscribeResponse {
                    success: false,
                    subscription,
                    reason: Some("Max number of subscriptions reached".to_string()),
                };
            }
            keys.push(subscription.symbol.name.clone());
            self.subscription_sender.send(keys).await;
        }
        DataServerResponse::SubscribeResponse {
            success: true,
            subscription,
            reason: None,
        }
    }

    #[allow(unused)]
    async fn data_feed_unsubscribe(&self, mode: StrategyMode, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
        unsubscribe_stream(&stream_name, &subscription).await;
        DataServerResponse::UnSubscribeResponse {
            success: true,
            subscription,
            reason: None,
        }
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
    async fn update_historical_data_for(&self, symbol: Symbol, base_data_type: BaseDataType, resolution: Resolution, progress_bar: ProgressBar) -> Result<(), FundForgeError>  {
        let earliest_oanda_data = || {
            let utc_time_string = "2005-01-01 00:00:00.000000";
            let utc_time_naive = NaiveDateTime::parse_from_str(utc_time_string, "%Y-%m-%d %H:%M:%S%.f").unwrap();
            DateTime::<Utc>::from_naive_utc_and_offset(utc_time_naive, Utc)
        };

        let data_storage = DATA_STORAGE.get().unwrap();
        // if we have data start from last time, else start from oanda's earliest date
        let mut last_bar_time = match data_storage.get_latest_data_time(&symbol, &resolution, &base_data_type).await {
            // if we have no data, we start from the earliest date available on oanda
            Err(_) => earliest_oanda_data(),
            // if we have data, we start from the last time in the data
            Ok(time) => match time {
                Some(time) => time,
                None => earliest_oanda_data()
            }
        };

        //let urls = generate_urls(symbol.clone(), resolution.clone(), base_data_type, last_bar_time, Utc::now()).await;
        let interval = resolution_to_oanda_interval(&resolution);
        let instrument  = oanda_clean_instrument(&symbol.name).await;

        let add_time = add_time_to_date(&interval);

        let mut num_days = ((Utc::now() - last_bar_time).num_seconds() / (60*60*5)).abs();
        progress_bar.set_length(num_days as u64);


        let mut new_data: BTreeMap<DateTime<Utc>, BaseDataEnum> = BTreeMap::new();
        let mut last_to_time = Utc::now() - Duration::seconds(5);
        loop {
            let to_time = last_bar_time + add_time;
            if to_time == last_to_time {
                break
            }
            last_to_time == to_time;
            let url = generate_url(&last_bar_time.naive_utc(), &to_time.naive_utc(), &instrument, &interval, &base_data_type);
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

            // First convert candles to a Vec for indexed access
            let candles_vec: Vec<_> = candles.into_iter().collect();
            let mut i = 0;

            let mut attempts = 0;
            while i < candles_vec.len() {
                let price_data = &candles_vec[i];
                let is_closed = price_data["complete"].as_bool().unwrap();
                if !is_closed {
                    i += 1;
                    continue;
                }

                let bar: BaseDataEnum = match base_data_type {
                    BaseDataType::QuoteBars => match oanda_quotebar_from_candle(price_data, symbol.clone(), resolution.clone()) {
                        Ok(quotebar) => BaseDataEnum::QuoteBar(quotebar),
                        Err(e) => {
                            i += 1;
                            continue
                        }
                    },
                    BaseDataType::Candles => match candle_from_candle(price_data, symbol.clone(), resolution.clone()) {
                        Ok(candle) => BaseDataEnum::Candle(candle),
                        Err(e) => {
                            i += 1;
                            continue
                        }
                    },
                    _ => {
                        i += 1;
                        continue
                    }
                };

                let new_bar_time = bar.time_utc();
                if last_bar_time.day() != new_bar_time.day() && !new_data.is_empty() {
                    let data_vec: Vec<BaseDataEnum> = new_data.values().map(|x| x.clone()).collect();
                    // Retry loop for saving data
                    const MAX_RETRIES: u32 = 3;
                    let mut retry_count = 0;
                    let save_result = 'save_loop: loop {
                        match data_storage.save_data_bulk(data_vec.clone()).await {
                            Ok(_) => break 'save_loop Ok(()),
                            Err(e) => {
                                retry_count += 1;
                                if retry_count >= MAX_RETRIES {
                                    break Err(e);
                                }
                                // Optional: Add delay between retries
                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            }
                        }
                    };

                    // Handle final save result
                    if let Err(e) = save_result {
                        // Return to the start of the current day's data
                        while i > 0 && bar.time_utc().day() == new_bar_time.day() {
                            i -= 1;
                        }
                        // Move forward one to start processing from the beginning of the failed day
                        i += 1;
                        continue;
                    }

                    new_data.clear();
                }

                last_bar_time = bar.time_utc();
                new_data.entry(new_bar_time).or_insert(bar);
                i += 1;
            }
            if to_time >= Utc::now() - Duration::seconds(5) {
                break;
            }
            progress_bar.inc(1);
        }
        let msg = format!("Oanda: Completed Download of data for: {}, {} {}", symbol.name, resolution, base_data_type);
        progress_bar.finish_with_message(msg);
        Ok(())
    }

    async fn update_historical_data_to(&self, symbol: Symbol, base_data_type: BaseDataType, resolution: Resolution, mut from: DateTime<Utc>, to: DateTime<Utc>, progress_bar: ProgressBar) -> Result<(), FundForgeError> {
        let earliest_oanda_data = {
            let utc_time_string = "2005-01-01 00:00:00.000000";
            let utc_time_naive = NaiveDateTime::parse_from_str(utc_time_string, "%Y-%m-%d %H:%M:%S%.f").unwrap();
            DateTime::<Utc>::from_naive_utc_and_offset(utc_time_naive, Utc)
        };

        if from < earliest_oanda_data {
            from = earliest_oanda_data;
        }

        let data_storage = DATA_STORAGE.get().unwrap();
        let urls = generate_urls(symbol.clone(), resolution.clone(), base_data_type, from, to).await;
        progress_bar.set_length(urls.len() as u64);

        let mut new_data: BTreeMap<DateTime<Utc>, BaseDataEnum> = BTreeMap::new();
        let mut last_bar_time = from;
        for url in &urls {
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

            // First convert candles to a Vec for indexed access
            let candles_vec: Vec<_> = candles.into_iter().collect();
            let mut i = 0;

            while i < candles_vec.len() {
                let price_data = &candles_vec[i];
                let is_closed = price_data["complete"].as_bool().unwrap();
                if !is_closed {
                    i += 1;
                    continue;
                }

                let bar: BaseDataEnum = match base_data_type {
                    BaseDataType::QuoteBars => match oanda_quotebar_from_candle(price_data, symbol.clone(), resolution.clone()) {
                        Ok(quotebar) => BaseDataEnum::QuoteBar(quotebar),
                        Err(_) => {
                            i += 1;
                            continue
                        }
                    },
                    BaseDataType::Candles => match candle_from_candle(price_data, symbol.clone(), resolution.clone()) {
                        Ok(candle) => BaseDataEnum::Candle(candle),
                        Err(_) => {
                            i += 1;
                            continue
                        }
                    },
                    _ => {
                        i += 1;
                        continue
                    }
                };

                let new_bar_time = bar.time_utc();
                if last_bar_time.day() != new_bar_time.day() && !new_data.is_empty() {
                    let data_vec: Vec<BaseDataEnum> = new_data.values().map(|x| x.clone()).collect();
                    // Retry loop for saving data
                    const MAX_RETRIES: u32 = 3;
                    let mut retry_count = 0;
                    let save_result = 'save_loop: loop {
                        match data_storage.save_data_bulk(data_vec.clone()).await {
                            Ok(_) => break 'save_loop Ok(()),
                            Err(e) => {
                                retry_count += 1;
                                if retry_count >= MAX_RETRIES {
                                    break Err(e);
                                }
                                // Optional: Add delay between retries
                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            }
                        }
                    };

                    // Handle final save result
                    if let Err(_) = save_result {
                        // Return to the start of the current day's data
                        while i > 0 && bar.time_utc().day() == new_bar_time.day() {
                            i -= 1;
                        }
                        // Move forward one to start processing from the beginning of the failed day
                        i += 1;
                        continue;
                    }

                    new_data.clear();
                }

                last_bar_time = bar.time_utc();
                new_data.entry(new_bar_time).or_insert(bar);
                i += 1;
            }
            progress_bar.inc(1);

            if last_bar_time >= to {
                break
            }
        }
        let msg = format!("Oanda: Completed Moving Historical Data Availability Backwards for: {}, {} {}", symbol.name, resolution, base_data_type);
        progress_bar.finish_with_message(msg);
        Ok(())
    }
}