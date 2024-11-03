use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::atomic::Ordering;
use async_trait::async_trait;
use chrono::{DateTime, Datelike, Duration, Utc};
use indicatif::{ProgressBar, ProgressStyle};
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
use crate::oanda_api::download::{generate_url};
use crate::oanda_api::get_requests::oanda_clean_instrument;
use crate::oanda_api::support_and_conversions::{add_time_to_date, resolution_to_oanda_interval};
use crate::server_features::database::DATA_STORAGE;
use crate::stream_tasks::{subscribe_stream, unsubscribe_stream};

#[async_trait]
impl VendorApiResponse for OandaClient {
    #[allow(unused)]
    async fn symbols_response(&self, mode: StrategyMode, stream_name: StreamName, market_type: MarketType, time: Option<DateTime<Utc>>, callback_id: u64) -> DataServerResponse {
        let mut symbols: Vec<Symbol> = Vec::new();
        for symbol in &self.instruments_map {
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
        if let Some(instrument) = self.instruments_map.get(&symbol_name) {
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
        let instrument = match self.instruments_map.get(&symbol_name) {
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
    async fn update_historical_data(&self, symbol: Symbol, base_data_type: BaseDataType, resolution: Resolution, from: DateTime<Utc>, to: DateTime<Utc>, progress_bar: ProgressBar) -> Result<(), FundForgeError> {
        let data_storage = DATA_STORAGE.get().unwrap();
        let interval = resolution_to_oanda_interval(&resolution);
        let instrument = oanda_clean_instrument(&symbol.name).await;
        let add_time = add_time_to_date(&interval);

        let mut num_days = ((Utc::now() - from).num_seconds() / (60*60*5)).abs();
        progress_bar.set_length(num_days as u64);
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template("{prefix:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg} ({eta})")
                .unwrap()
                .progress_chars("=>-")
        );
        progress_bar.set_prefix(symbol.name.clone());

        let mut new_data: BTreeMap<DateTime<Utc>, BaseDataEnum> = BTreeMap::new();
        let current_time = Utc::now() - Duration::seconds(5);

        // Keep track of empty responses to prevent infinite loops
        let mut consecutive_empty_responses = 0;
        const MAX_EMPTY_RESPONSES: u32 = 20;
        let mut last_bar_time = from;
        loop {
            let to_time = (last_bar_time + add_time).min(current_time);

            // Break if we've reached or passed current time
            if last_bar_time >= current_time {
                break;
            }
            progress_bar.set_message(format!("Downloading: ({}: {}) from: {}, to {}", resolution, base_data_type, last_bar_time, Utc::now().format("%Y-%m-%d %H:%M:%S")));
            let url = generate_url(&last_bar_time.naive_utc(), &to_time.naive_utc(), &instrument, &interval, &base_data_type);
            let response = match self.send_rest_request(&url).await {
                Ok(resp) => resp,
                Err(_) => {
                    // On error, advance time window and continue
                    last_bar_time = to_time;
                    continue;
                }
            };

            if !response.status().is_success() {
                last_bar_time = to_time;
                continue;
            }

            let content = response.text().await.unwrap();
            let json: serde_json::Value = serde_json::from_str(&content).unwrap();
            let candles = json["candles"].as_array().unwrap();

            if candles.is_empty() {
                consecutive_empty_responses += 1;
                if consecutive_empty_responses >= MAX_EMPTY_RESPONSES {
                    // If we get multiple empty responses, assume we've reached the end of available data
                    break;
                }
                // Advance time window even when no data is found
                last_bar_time = to_time;
                continue;
            }

            // Reset empty response counter when we get data
            consecutive_empty_responses = 0;

            // Process candles
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
                    let data_vec: Vec<BaseDataEnum> = new_data.values().cloned().collect();

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
                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            }
                        }
                    };

                    if let Err(e) = save_result {
                        while i > 0 && bar.time_utc().day() == new_bar_time.day() {
                            i -= 1;
                        }
                        i += 1;
                        continue;
                    }

                    new_data.clear();
                }

                last_bar_time = bar.time_utc();
                new_data.entry(new_bar_time).or_insert(bar);
                i += 1;
            }

            // Update last_bar_time to the end of the current window if no data was processed
            if last_bar_time < to_time {
                last_bar_time = to_time;
            }

            progress_bar.inc(1);
        }

        // Save any remaining data
        if !new_data.is_empty() {
            let data_vec: Vec<BaseDataEnum> = new_data.values().cloned().collect();
            match data_storage.save_data_bulk(data_vec).await {
                Ok(_) => {}
                Err(_) => {}
            }
        }
        progress_bar.finish_and_clear();
        Ok(())
    }
}