use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::atomic::Ordering;
use async_trait::async_trait;
use chrono::{DateTime, Datelike, Duration, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use rust_decimal::Decimal;
use tokio::sync::broadcast;
use tokio::time::timeout;
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
use crate::oanda_api::support_and_conversions::oanda_quotebar_from_candle;
use crate::oanda_api::download::{generate_url};
use crate::oanda_api::support_and_conversions::oanda_clean_instrument;
use crate::oanda_api::support_and_conversions::{add_time_to_date, resolution_to_oanda_interval};
use crate::server_features::database::DATA_STORAGE;
use crate::stream_tasks::{subscribe_stream, unsubscribe_stream};

#[async_trait]
impl VendorApiResponse for OandaClient {

    async fn symbols_response(&self, _mode: StrategyMode, _stream_name: StreamName, market_type: MarketType, _time: Option<DateTime<Utc>>, callback_id: u64) -> DataServerResponse {
        let mut symbols: Vec<Symbol> = Vec::new();
        for symbol in self.instruments_map.iter() {
            let symbol = Symbol::new(symbol.key().clone(), DataVendor::Oanda, symbol.value().market_type.clone());
            symbols.push(symbol);
        }
        DataServerResponse::Symbols {
            callback_id,
            symbols,
            market_type,
        }
    }

    async fn resolutions_response(&self, mode: StrategyMode, _stream_name: StreamName, market_type: MarketType, callback_id: u64) -> DataServerResponse {
        let subscription_resolutions_types = match mode {
            StrategyMode::Backtest => vec![SubscriptionResolutionType::new(Resolution::Seconds(5), BaseDataType::QuoteBars), SubscriptionResolutionType::new(Resolution::Minutes(1), BaseDataType::QuoteBars),SubscriptionResolutionType::new(Resolution::Hours(1), BaseDataType::QuoteBars)],
            StrategyMode::LivePaperTrading | StrategyMode::Live => vec![SubscriptionResolutionType::new(Resolution::Instant, BaseDataType::Quotes), SubscriptionResolutionType::new(Resolution::Seconds(5), BaseDataType::QuoteBars), SubscriptionResolutionType::new(Resolution::Minutes(1), BaseDataType::QuoteBars),SubscriptionResolutionType::new(Resolution::Hours(1), BaseDataType::QuoteBars)],
        };

        DataServerResponse::Resolutions {
            callback_id,
            market_type,
            subscription_resolutions_types,
        }
    }

    async fn markets_response(&self, _mode: StrategyMode, _stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        DataServerResponse::Markets {
            callback_id,
            markets: vec![MarketType::CFD, MarketType::Forex],
        }
    }


    async fn decimal_accuracy_response(&self,_mode: StrategyMode, _stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
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

    async fn tick_size_response(&self, _mode: StrategyMode, _stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
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

    async fn data_feed_subscribe(&self, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
        if subscription.base_data_type == BaseDataType::QuoteBars {
            if subscription.resolution != Resolution::Seconds(5) && subscription.resolution != Resolution::Minutes(1) && subscription.resolution != Resolution::Hours(1) {
                return DataServerResponse::UnSubscribeResponse {
                    success: false,
                    reason: Some(format!("Live subscription does not support: Resolution {}, Subscribe to lower resolution and use consolidator", subscription.resolution)),
                    subscription,
                };
            }
            if let Some(broadcaster) = self.quotebar_broadcasters.get(&subscription) {
                let receiver = broadcaster.value().subscribe();
                subscribe_stream(&stream_name, subscription.clone(), receiver).await;
            } else {
                let (sender, receiver) = broadcast::channel(20);
                self.quotebar_broadcasters.insert(subscription.clone(), sender);
                subscribe_stream(&stream_name, subscription.clone(), receiver).await;
            }

            return DataServerResponse::SubscribeResponse {
                success: true,
                subscription,
                reason: None,
            }
        }

        if subscription.resolution != Resolution::Instant {
            return DataServerResponse::UnSubscribeResponse {
                success: false,
                reason: Some(format!("Oanda subscription error: {:?}, please report bug", subscription)),
                subscription,
            };
        }

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
            if self.quote_feed_broadcasters.len() == 20 {
                return DataServerResponse::UnSubscribeResponse {
                    success: false,
                    subscription,
                    reason: Some("Max number of subscriptions reached".to_string()),
                };
            }
            let (sender, receiver) = broadcast::channel(500);
            self.quote_feed_broadcasters.insert(subscription.symbol.name.clone(), sender);
            subscribe_stream(&stream_name, subscription.clone(), receiver).await;
            is_subscribed = false;
        }

        if !is_subscribed {
            let keys: Vec<SymbolName> = self.quote_feed_broadcasters.iter().map(|entry| entry.key().clone()).collect();
            let _ = self.quote_subscription_sender.send(keys).await;
        }
        DataServerResponse::SubscribeResponse {
            success: true,
            subscription,
            reason: None,
        }
    }


    async fn data_feed_unsubscribe(&self, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
        unsubscribe_stream(&stream_name, &subscription).await;
        DataServerResponse::UnSubscribeResponse {
            success: true,
            subscription,
            reason: None,
        }
    }

    async fn base_data_types_response(&self, mode: StrategyMode, _stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        match mode {
            StrategyMode::Backtest => {
                DataServerResponse::BaseDataTypes {
                    callback_id,
                    base_data_types: vec![BaseDataType::QuoteBars],
                }
            }
            StrategyMode::Live | StrategyMode::LivePaperTrading => {
                DataServerResponse::BaseDataTypes {
                    callback_id,
                    base_data_types: vec![BaseDataType::QuoteBars, BaseDataType::Quotes],
                }
            }
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

    async fn update_historical_data(
        &self,
        symbol: Symbol,
        base_data_type: BaseDataType,
        resolution: Resolution,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        from_back: bool,
        progress_bar: ProgressBar,
        is_bulk_download: bool
    ) -> Result<(), FundForgeError> {
        let data_storage = DATA_STORAGE.get().unwrap();
        let interval = match resolution_to_oanda_interval(&resolution) {
            Some(interval) => interval,
            None => return Err(FundForgeError::ClientSideErrorDebug("Invalid resolution".to_string())),
        };
        let instrument = oanda_clean_instrument(&symbol.name).await;
        let add_time = add_time_to_date(&interval);

        let num_days = (from - to).abs().num_days();
        progress_bar.set_length(num_days as u64);
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template("{prefix:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg} ({eta})")
                .unwrap()
                .progress_chars("=>-")
        );


        let mut new_data: BTreeMap<DateTime<Utc>, BaseDataEnum> = BTreeMap::new();
        let current_time = Utc::now() - Duration::seconds(5);

        let mut consecutive_empty_responses = 0;
        const MAX_EMPTY_RESPONSES: u32 = 100;
        const TIME_NEGATIVE: std::time::Duration = std::time::Duration::from_secs(60 * 15);
        const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
        let mut last_bar_time = from;

        'main_loop: loop {
            let start = last_bar_time;
            let to_time = (start + add_time).min(current_time);

            if last_bar_time >= to - TIME_NEGATIVE {
                break;
            }

            progress_bar.set_message(format!(
                "Downloading: ({}: {}) from: {}, to {}",
                resolution,
                base_data_type,
                last_bar_time,
                to_time.format("%Y-%m-%d %H:%M:%S")
            ));

            let url = generate_url(&start.naive_utc(), &to_time.naive_utc(), &instrument, &interval, &base_data_type);

            // Add timeout to request
            let response = match timeout(REQUEST_TIMEOUT, self.send_download_request(&url)).await {
                Ok(result) => match result {
                    Ok(resp) => resp,
                    Err(_) => {
                        consecutive_empty_responses += 1;
                        progress_bar.set_message(format!("Error downloading data for: {} from: {}, to: {}", symbol.name, last_bar_time, to_time));
                        if consecutive_empty_responses >= MAX_EMPTY_RESPONSES {
                            break 'main_loop;
                        }
                        // Add delay before retry
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        continue;
                    }
                },
                Err(_) => {
                    progress_bar.set_message(format!("Request timeout for: {} from: {}, to: {}", symbol.name, last_bar_time, to_time));
                    consecutive_empty_responses += 1;
                    if consecutive_empty_responses >= MAX_EMPTY_RESPONSES {
                        break 'main_loop;
                    }
                    continue;
                }
            };

            if !response.status().is_success() {
                consecutive_empty_responses += 1;
                if consecutive_empty_responses >= MAX_EMPTY_RESPONSES {
                    break 'main_loop;
                }
                continue;
            }

            // Add timeout to reading response
            let content = match response.text().await {
                Ok(text) => text,
                Err(e) => {
                    progress_bar.set_message(format!("Error reading response body: {} for: {} from: {}, to: {}", e, symbol.name, last_bar_time, to_time));
                    consecutive_empty_responses += 1;
                    if consecutive_empty_responses >= MAX_EMPTY_RESPONSES {
                        break 'main_loop;
                    }
                    continue;
                }
            };

            let json: serde_json::Value = serde_json::from_str(&content).unwrap();
            let candles = json["candles"].as_array().unwrap();

            // Process candles
            let candles_vec: Vec<_> = candles.into_iter().collect();

            if !candles_vec.is_empty() {
                for candle in candles_vec {
                    let is_closed = candle["complete"].as_bool().unwrap();
                    if !is_closed {
                        break 'main_loop;
                    }

                    let bar: BaseDataEnum = match base_data_type {
                        BaseDataType::QuoteBars => match oanda_quotebar_from_candle(candle, symbol.clone(), resolution.clone()) {
                            Ok(quotebar) => BaseDataEnum::QuoteBar(quotebar),
                            Err(_) => break 'main_loop
                        },
                        _ => break 'main_loop
                    };

                    let new_bar_time = bar.time_utc();
                    if last_bar_time.day() != new_bar_time.day() && !new_data.is_empty() {
                        let data_vec: Vec<BaseDataEnum> = new_data.values().cloned().collect();
                        match data_storage.save_data_bulk(data_vec.clone(), is_bulk_download).await {
                            Ok(_) => {
                                progress_bar.inc(1);
                            },
                            Err(e) => {
                                progress_bar.set_message(format!("Error saving data batch: {}", e));
                                break 'main_loop;
                            }
                        }
                        new_data.clear();
                    }

                    last_bar_time = bar.time_utc();
                    new_data.entry(new_bar_time).or_insert(bar);
                    // Reset counter when we get_requests data
                }
            }
            if start == last_bar_time {
                last_bar_time = to_time;
                if last_bar_time.day() != start.day() {
                    progress_bar.inc(1);
                }
            }
            else {
                consecutive_empty_responses = 0;
            }
        }

        // Save any remaining data
        if !new_data.is_empty() {
            let data_vec: Vec<BaseDataEnum> = new_data.values().cloned().collect();
            if let Err(e) = data_storage.save_data_bulk(data_vec, is_bulk_download).await {
                progress_bar.set_message(format!("Error saving final data batch: {}", e));
            }
        }

        if !from_back {
            let duration_since_last_bar = Utc::now() - last_bar_time;
            let units = duration_since_last_bar.num_seconds() / resolution.as_seconds();
            if let Some(account) = self.accounts.get(0) {
                let bars = self.get_latest_bars(&symbol, base_data_type, resolution, &account.account_id, (units + 3) as i32).await?;
                if let Err(e) = data_storage.save_data_bulk(bars, is_bulk_download).await {
                    progress_bar.set_message(format!("Error saving final data batch: {}", e));
                }
            }
        }

        progress_bar.finish_and_clear();
        Ok(())
    }
}