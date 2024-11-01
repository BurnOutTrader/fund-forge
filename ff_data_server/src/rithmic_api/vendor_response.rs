use std::collections::BTreeMap;
use async_trait::async_trait;
use chrono::{DateTime, Datelike, Duration, NaiveDateTime, Utc, Weekday};
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_login::SysInfraType;
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::{request_tick_bar_replay, RequestMarketDataUpdate, RequestTickBarReplay, RequestTimeBarReplay, RequestTimeBarUpdate};
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_time_bar_update::BarType;
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use crate::server_features::server_side_datavendor::VendorApiResponse;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::enums::{FuturesExchange, MarketType, StrategyMode, SubscriptionResolutionType};
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol, SymbolName};
use ff_standard_lib::StreamName;
use tokio::sync::broadcast;
use tokio::time::{timeout};
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::market_maps::product_trading_hours::get_futures_trading_hours;
use crate::rithmic_api::api_client::RithmicBrokerageClient;
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_tick_bar_replay::{Direction, TimeOrder};
use crate::rithmic_api::products::{get_available_symbol_names, get_exchange_by_symbol_name, get_symbol_info};
use crate::server_features::database::DATA_STORAGE;
use crate::stream_tasks::{subscribe_stream, unsubscribe_stream};

#[allow(dead_code)]
#[async_trait]
impl VendorApiResponse for RithmicBrokerageClient {
    async fn symbols_response(&self, _mode: StrategyMode, _stream_name: StreamName, market_type: MarketType, _time: Option<DateTime<Utc>>, callback_id: u64) -> DataServerResponse{
        let names = get_available_symbol_names();
        let mut symbols = Vec::new();
        for name in names {
            let exchange = match get_exchange_by_symbol_name(name) {
                Some(exchange) => exchange,
                None => continue
            };
            symbols.push(Symbol::new(name.clone(), self.data_vendor.clone(), MarketType::Futures(exchange)));
        }
        DataServerResponse::Symbols {
            callback_id,
            symbols,
            market_type,
        }
       /* match mode {
            StrategyMode::Backtest => {

            }
            StrategyMode::LivePaperTrading | StrategyMode::Live => {
                match market_type {
                    //todo, use this in a rithmic only fn, to get the toi products, just return the hardcoded list here.
                    MarketType::Futures(exchange) => {
                        let _req = RequestProductCodes {
                            template_id: 111 ,
                            user_msg: vec![stream_name.to_string(), callback_id.to_string()],
                            exchange: Some(exchange.to_string()),
                            give_toi_products_only: Some(true),
                        };
                    }
                    _ => return DataServerResponse::Error {callback_id, error: FundForgeError::ClientSideErrorDebug(format!("Incrorrect market type: {} for: {}", market_type, self.data_vendor))}
                }
            }
        }*/
    }

    async fn resolutions_response(&self, mode: StrategyMode, _stream_name: StreamName, _market_type: MarketType, callback_id: u64) -> DataServerResponse {
        let mut resolutions = Vec::new();
        match mode {
            StrategyMode::Backtest => {
                //todo, we need a better way to handle historical, primary data sources, we need a way to check for each symbol, which historical data is available.
                // to achieve this this fn should be split, resolutions should also be determined by symbol name when historical data is requested, so we can check the data we actually have available.
                resolutions.push(SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks));
            }
            StrategyMode::LivePaperTrading |  StrategyMode::Live => {
                resolutions.push(SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks));
                resolutions.push(SubscriptionResolutionType::new(Resolution::Instant, BaseDataType::Quotes));
                resolutions.push(SubscriptionResolutionType::new(Resolution::Seconds(1), BaseDataType::Candles));
            }
        }

        DataServerResponse::Resolutions {
            callback_id,
            subscription_resolutions_types: resolutions,
            market_type: MarketType::Forex,
        }
    }

    async fn markets_response(&self, _mode: StrategyMode, _stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        DataServerResponse::Markets {
            callback_id,
            markets: vec![
                MarketType::Futures(FuturesExchange::CME),
                MarketType::Futures(FuturesExchange::CBOT),
                MarketType::Futures(FuturesExchange::COMEX),
                MarketType::Futures(FuturesExchange::NYBOT),
                MarketType::Futures(FuturesExchange::NYMEX),
                MarketType::Futures(FuturesExchange::MGEX)
            ],
        }
    }

    async fn decimal_accuracy_response(&self, _mode: StrategyMode, _stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        let info = match get_symbol_info(&symbol_name) {
            Ok(info) => {
                info
            }
            Err(_e) => {
                return DataServerResponse::Error {callback_id, error: FundForgeError::ClientSideErrorDebug(format!("{} Accuracy Info not found with: {}", symbol_name, self.data_vendor))}
            }
        };
        DataServerResponse::DecimalAccuracy {
            callback_id,
            accuracy: info.decimal_accuracy,
        }
    }

    async fn tick_size_response(&self, _mode: StrategyMode, _stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        let info = match get_symbol_info(&symbol_name) {
            Ok(info) => {
                info
            }
            Err(_e) => {
                return DataServerResponse::Error {callback_id, error: FundForgeError::ClientSideErrorDebug(format!("{} Tick Size Info not found with: {}", symbol_name, self.data_vendor))}
            }
        };
        DataServerResponse::TickSize {
            callback_id,
            tick_size: info.tick_size,
        }
    }

    async fn data_feed_subscribe(&self, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
        let exchange = match subscription.market_type {
            MarketType::Futures(exchange) => {
                exchange.to_string()
            }
            _ => todo!()
        };

        let symbols = get_available_symbol_names();
        if !symbols.contains(&subscription.symbol.name) {
            return DataServerResponse::SubscribeResponse{ success: false, subscription: subscription.clone(), reason: Some(format!("This subscription is not available with {}: {}", subscription.symbol.data_vendor, subscription))}
        }

        let mut resolutions = Vec::new();
        resolutions.push(Resolution::Instant);
        resolutions.push(Resolution::Ticks(1));
        resolutions.push(Resolution::Seconds(1));
        //we can pass in live here because backtest never calls this fn

        if !resolutions.contains(&subscription.resolution) {
            return DataServerResponse::SubscribeResponse{ success: false, subscription: subscription.clone(), reason: Some(format!("This subscription is not available with {}: {}", subscription.symbol.data_vendor, subscription))}
        }

        const BASEDATA_TYPES: &[BaseDataType] = &[BaseDataType::Ticks, BaseDataType::Quotes, BaseDataType::Candles];
        if !BASEDATA_TYPES.contains(&subscription.base_data_type) {
            return DataServerResponse::SubscribeResponse{ success: false, subscription: subscription.clone(), reason: Some(format!("This subscription is not available with {}: {}", subscription.symbol.data_vendor, subscription))}
        };

        let mut is_subscribed = true;
        //todo have a unique function per base data type.
        match subscription.base_data_type {
            BaseDataType::Ticks => {
                if let Some(broadcaster) = self.tick_feed_broadcasters.get(&subscription.symbol.name) {
                    let receiver = broadcaster.value().subscribe();
                    subscribe_stream(&stream_name, subscription.clone(), receiver).await;
                } else {
                    let (sender, receiver) = broadcast::channel(500);
                    self.tick_feed_broadcasters.insert(subscription.symbol.name.clone(), sender);
                    subscribe_stream(&stream_name, subscription.clone(), receiver).await;
                    is_subscribed = false;
                }
            }
            BaseDataType::Quotes => {
                if let Some(broadcaster) = self.quote_feed_broadcasters.get(&subscription.symbol.name) {
                    let receiver = broadcaster.value().subscribe();
                    subscribe_stream(&stream_name, subscription.clone(), receiver).await;
                } else {
                    let (sender, receiver) = broadcast::channel(500);
                    self.quote_feed_broadcasters.insert(subscription.symbol.name.clone(), sender);
                    subscribe_stream(&stream_name, subscription.clone(), receiver).await;
                    self.ask_book.insert(subscription.symbol.name.clone(), BTreeMap::new());
                    self.ask_book.insert(subscription.symbol.name.clone(), BTreeMap::new());
                    is_subscribed = false;
                }
            }
            BaseDataType::Candles => {
                if let Some(broadcaster) = self.candle_feed_broadcasters.get(&subscription.symbol.name) {
                    let receiver = broadcaster.value().subscribe();
                    subscribe_stream(&stream_name, subscription.clone(), receiver).await;
                } else {
                    let (sender, receiver) = broadcast::channel(500);
                    self.candle_feed_broadcasters.insert(subscription.symbol.name.clone(), sender);
                    subscribe_stream(&stream_name, subscription.clone(), receiver).await;
                    is_subscribed = false;
                }
            }
            _ => todo!("Handle gracefully by returning err")
        }

        if !is_subscribed {
            if subscription.base_data_type == BaseDataType::Quotes || subscription.base_data_type == BaseDataType::Ticks {
                let bits = match subscription.base_data_type {
                    BaseDataType::Ticks => 1,
                    BaseDataType::Quotes => 2,
                    _ => return DataServerResponse::SubscribeResponse { success: false, subscription: subscription.clone(), reason: Some(format!("This subscription is not available with {}: {}", self.data_vendor, subscription)) }
                    //BaseDataType::Candles => {}
                };
                let req = RequestMarketDataUpdate {
                    template_id: 100,
                    user_msg: vec![],
                    symbol: Some(subscription.symbol.name.to_string()),
                    exchange: Some(exchange),
                    request: Some(1), //1 subscribe 2 unsubscribe
                    update_bits: Some(bits), //1 for ticks 2 for quotes
                };

                const PLANT: SysInfraType = SysInfraType::TickerPlant;
                self.send_message(&PLANT, req).await;
            } else if subscription.base_data_type == BaseDataType::Candles {
                let (num, res_type) = match subscription.resolution {
                    Resolution::Seconds(num) => (num as i32, BarType::SecondBar),
                    Resolution::Minutes(num) => (num as i32, BarType::MinuteBar),
                    _ => return DataServerResponse::SubscribeResponse { success: false, subscription: subscription.clone(), reason: Some(format!("This subscription is not available with {}: {}", self.data_vendor,subscription)) }
                };
                let req =RequestTimeBarUpdate {
                    template_id: 200,
                    user_msg: vec![],
                    symbol: Some(subscription.symbol.name.to_string()),
                    exchange: Some(exchange),
                    request: Some(1), //1 subscribe 2 unsubscribe
                    bar_type: Some(res_type.into()),
                    bar_type_period: Some(num),
                };
                const PLANT: SysInfraType = SysInfraType::HistoryPlant;
                self.send_message(&PLANT, req).await;
            }
        }
        println!("{} Subscribed: {}", stream_name, subscription);
        DataServerResponse::SubscribeResponse{ success: true, subscription: subscription.clone(), reason: None}
    }

    async fn data_feed_unsubscribe(&self, _mode: StrategyMode, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
        let exchange = match subscription.market_type {
            MarketType::Futures(exchange) => exchange.to_string(),
            _ => return DataServerResponse::UnSubscribeResponse {
                success: false,
                subscription: subscription.clone(),
                reason: Some(format!("Unsupported market type: {:?}", subscription.market_type)),
            },
        };

        unsubscribe_stream(&stream_name, &subscription).await;

        let (bits, broadcaster_map) = match subscription.base_data_type {
            BaseDataType::Ticks => (1, &self.tick_feed_broadcasters),
            BaseDataType::Quotes => (2, &self.quote_feed_broadcasters),
            BaseDataType::Candles => (3, &self.candle_feed_broadcasters),
            _ => return DataServerResponse::SubscribeResponse {
                success: false,
                subscription: subscription.clone(),
                reason: Some(format!("Unsupported data type: {:?}", subscription.base_data_type)),
            },
        };

        let symbol = subscription.symbol.name.clone();
        let mut should_disconnect = false;

        if let Some(broadcaster) = broadcaster_map.get_mut(&symbol) {
            should_disconnect = broadcaster.receiver_count() == 0;
        }

        if should_disconnect {
            if subscription.base_data_type == BaseDataType::Quotes || subscription.base_data_type == BaseDataType::Ticks {
                broadcaster_map.remove(&symbol);

                let req = RequestMarketDataUpdate {
                    template_id: 100,
                    user_msg: vec![],
                    symbol: Some(symbol.clone()),
                    exchange: Some(exchange),
                    request: Some(2), // 2 for unsubscribe
                    update_bits: Some(bits),
                };

                const PLANT: SysInfraType = SysInfraType::TickerPlant;
                self.send_message(&PLANT, req).await;

                // Additional cleanup for quotes
                if subscription.base_data_type == BaseDataType::Quotes {
                    self.ask_book.remove(&symbol);
                    self.bid_book.remove(&symbol);
                }
            } else if subscription.base_data_type == BaseDataType::Candles {
                let req =RequestTimeBarUpdate {
                    template_id: 200,
                    user_msg: vec![],
                    symbol: Some(subscription.symbol.name.to_string()),
                    exchange: Some(exchange),
                    request: Some(2), //1 subscribe 2 unsubscribe
                    bar_type: Some(BarType::SecondBar.into()),
                    bar_type_period: Some(1),
                };
                const PLANT: SysInfraType = SysInfraType::HistoryPlant;
                self.send_message(&PLANT, req).await;
            }
        }

        // Check if we need to switch heartbeat
        if self.tick_feed_broadcasters.is_empty() &&
            self.quote_feed_broadcasters.is_empty() &&
            self.candle_feed_broadcasters.is_empty()
        {
            //todo fix in ff_rithmic api this causes a lock
         /*   if let Err(e) = self.client.switch_heartbeat_required(SysInfraType::TickerPlant, true).await {
                eprintln!("Failed to switch heartbeat: {}", e);
            }*/
        }

        DataServerResponse::UnSubscribeResponse {
            success: true,
            subscription,
            reason: None,
        }
    }

    async fn base_data_types_response(&self, _mode: StrategyMode, _stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        //todo get dynamically from server using stream name to fwd callback
        DataServerResponse::BaseDataTypes {
            callback_id,
            base_data_types: vec![BaseDataType::Ticks, BaseDataType::Quotes, BaseDataType::Candles],
        }
    }

    async fn logout_command_vendors(&self, stream_name: StreamName) {
        self.callbacks.remove(&stream_name);
    }

    #[allow(unused)]
    async fn session_market_hours_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, date_time: DateTime<Utc>, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn update_historical_data_for(&self, symbol: Symbol, base_data_type: BaseDataType, resolution: Resolution) {
        const SYSTEM: SysInfraType = SysInfraType::HistoryPlant;
        let symbol_name = symbol.name.clone();
        let exchange = match get_exchange_by_symbol_name(&symbol_name) {
            Some(exchange) => exchange,
            None => return
        };

        // Get trading hours for the symbol
        let trading_hours = match get_futures_trading_hours(&symbol_name) {
            Some(hours) => hours,
            None => {
                println!("No trading hours found for symbol {}", symbol_name);
                return;
            }
        };

        // Create or get broadcaster with larger buffer to prevent lagging
        let mut receiver = match self.historical_data_broadcaster.get(&(symbol_name.clone(), base_data_type.clone())) {
            Some(broadcaster) => broadcaster.value().subscribe(),
            None => {
                let (sender, receiver) = broadcast::channel(5000);
                self.historical_data_broadcaster.insert((symbol_name.clone(), base_data_type.clone()), sender);
                receiver
            }
        };

        let earliest_rithmic_data = match base_data_type {
            BaseDataType::Ticks | BaseDataType::Candles => {
                let utc_time_string = "2019-06-02 20:00:00.000000";
                let utc_time_naive = NaiveDateTime::parse_from_str(utc_time_string, "%Y-%m-%d %H:%M:%S%.f").unwrap();
                DateTime::<Utc>::from_naive_utc_and_offset(utc_time_naive, Utc)
            }
            _ => return
        };

        let data_storage = DATA_STORAGE.get().unwrap();

        let mut window_start = match data_storage.get_latest_data_time(&symbol, &resolution, &base_data_type).await {
            Ok(earliest_date) => earliest_date.unwrap_or_else(|| earliest_rithmic_data),
            Err(_e) => earliest_rithmic_data
        };

        let mut data_map = BTreeMap::new();
        'main_loop: loop {
            let local_time = window_start.clone().with_timezone(&trading_hours.timezone);
            if local_time.weekday() == Weekday::Sat && trading_hours.saturday.open.is_none() && trading_hours.saturday.close.is_none() {
                if let Some(sunday_open) = trading_hours.sunday.open {
                    // Get the current Saturday's date
                    let sunday_date = local_time.date_naive();
                    // Create NaiveDateTime for Sunday market open
                    let sunday_market_open = sunday_date
                        .and_time(sunday_open - Duration::hours(1)) //todo start 1 hour before open in case historical timezones are incorrect
                        .and_local_timezone(trading_hours.timezone)
                        .unwrap()
                        .with_timezone(&Utc);
                    window_start = sunday_market_open;
                }
                continue;
            }

            // Calculate window end based on start time (always 1 hour)
            let window_end = window_start + Duration::hours(4);

            println!("Requesting Rithmic data for {} from {} to {}",
                     symbol_name, window_start, window_end);

            self.send_replay_request(base_data_type, resolution, symbol_name.clone(), exchange, window_start, window_end).await;

            let mut had_data = false;
            // Receive loop with timeout
            'msg_loop: loop {
                match timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
                    Ok(Ok(data)) => {
                        had_data = true;
                        data_map.insert(data.time_utc(), data);
                    },
                    Ok(Err(e)) => {
                        println!("Broadcast channel error: {}", e);
                        break 'main_loop;
                    },
                    Err(_) => { // Timeout case
                        break 'msg_loop;
                    }
                }
            }

            if let Some((&last_time, _)) = data_map.last_key_value() {
                window_start = last_time.clone();
                let save_data: Vec<BaseDataEnum> = data_map.into_values().collect();
                println!("Rithmic: Saving {} data points", save_data.len());
                if let Err(e) = DATA_STORAGE.get().unwrap().save_data_bulk(save_data).await {
                    eprintln!("Failed to save data: {}", e);
                }
                data_map = BTreeMap::new();
            } else {
                // If no new data, advance window to avoid re-requesting the same interval
                window_start = window_end;
            };

            // Check if we've caught up to the desired end or current time
            if (Utc::now() - window_end).num_seconds().abs() <= 1 {
                println!("Caught up to current time");
                break 'main_loop;
            }
        }
    }
}
