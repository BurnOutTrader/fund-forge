use std::collections::BTreeMap;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
use ff_rithmic_api::rithmic_proto_objects::rti::{RequestMarketDataUpdate, RequestProductCodes};
use rust_decimal_macros::dec;
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::server_features::server_side_datavendor::VendorApiResponse;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::enums::{FuturesExchange, MarketType, StrategyMode, SubscriptionResolutionType};
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, SymbolName};
use ff_standard_lib::StreamName;
use tokio::sync::broadcast;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::rithmic_api::api_client::RithmicClient;
use crate::stream_tasks::{subscribe_stream, unsubscribe_stream};

#[allow(dead_code)]
#[async_trait]
impl VendorApiResponse for RithmicClient {
    async fn symbols_response(&self, mode: StrategyMode, stream_name: StreamName, market_type: MarketType, _time: Option<DateTime<Utc>>, callback_id: u64) -> DataServerResponse{
        const SYSTEM: SysInfraType = SysInfraType::TickerPlant;
        match mode {
            StrategyMode::Backtest => {

            }
            StrategyMode::LivePaperTrading | StrategyMode::Live => {
                match market_type {
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
        }

        todo!()
    }

    async fn resolutions_response(&self, mode: StrategyMode, _stream_name: StreamName, _market_type: MarketType, callback_id: u64) -> DataServerResponse {
        let subs = match mode {
            StrategyMode::Backtest => {
                vec![
                    SubscriptionResolutionType::new(Resolution::Instant, BaseDataType::Quotes),
                    SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks),
                ]
            }
            StrategyMode::LivePaperTrading | StrategyMode::Live => {
                vec![
                    SubscriptionResolutionType::new(Resolution::Instant, BaseDataType::Quotes),
                    SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks),
                ]
            }
        };
        DataServerResponse::Resolutions {
            callback_id,
            subscription_resolutions_types: subs,
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
        //todo get dynamically from server using stream name to fwd callback
        let accuracy = match symbol_name.as_str() {
            "MNQ" => 2,
            _ => todo!(),
        };
        DataServerResponse::DecimalAccuracy {
            callback_id,
            accuracy,
        }
    }

    async fn tick_size_response(&self, _mode: StrategyMode, _stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        //todo get dynamically from server using stream name to fwd callback
        let tick_size = match symbol_name.as_str() {
            "MNQ" => dec!(0.25),
            _ => todo!(),
        };
        DataServerResponse::TickSize {
            callback_id,
            tick_size,
        }
    }

    async fn data_feed_subscribe(&self, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
        let exchange = match subscription.market_type {
            MarketType::Futures(exchange) => {
                exchange.to_string()
            }
            _ => todo!()
        };


 /*       let req = RequestTimeBarUpdate {
        template_id: 200,
        user_msg: vec![],
        symbol: Some("NQ".to_string()),
        exchange: Some(Exchange::CME.to_string()),
        request: Some(Request::Subscribe.into()),
        bar_type: Some(1),
        bar_type_period: Some(5),
    };*/

        //todo if not working try resolution Instant
        let available_subscriptions = vec![
            DataSubscription::new(SymbolName::from("MNQ"), self.data_vendor.clone(), Resolution::Ticks(1), BaseDataType::Ticks, MarketType::Futures(FuturesExchange::CME)),
            DataSubscription::new(SymbolName::from("MNQ"), self.data_vendor.clone(), Resolution::Instant, BaseDataType::Quotes, MarketType::Futures(FuturesExchange::CME))
        ];
        if !available_subscriptions.contains(&subscription) {
            eprintln!("Rithmic Subscription Not Available: {:?}", subscription);
            return DataServerResponse::SubscribeResponse{ success: false, subscription: subscription.clone(), reason: Some(format!("This subscription is not available with DataVendor::Test: {}", subscription))}
        }

        let mut is_subscribed = true;
        //todo have a unique function per base data type.
        match subscription.base_data_type {
            BaseDataType::Ticks => {
                if let Some(broadcaster) = self.tick_feed_broadcasters.get(&subscription.symbol.name) {
                    let receiver = broadcaster.value().subscribe();
                    subscribe_stream(&stream_name, subscription.clone(), receiver).await;
                    is_subscribed = false;
                } else {
                    let (sender, receiver) = broadcast::channel(500);
                    self.tick_feed_broadcasters.insert(subscription.symbol.name.clone(), sender);
                    subscribe_stream(&stream_name, subscription.clone(), receiver).await;
                }
            }
            BaseDataType::Quotes => {
                if let Some(broadcaster) = self.quote_feed_broadcasters.get(&subscription.symbol.name) {
                    let receiver = broadcaster.value().subscribe();
                    subscribe_stream(&stream_name, subscription.clone(), receiver).await;
                    is_subscribed = false;
                } else {
                    let (sender, receiver) = broadcast::channel(500);
                    self.quote_feed_broadcasters.insert(subscription.symbol.name.clone(), sender);
                    subscribe_stream(&stream_name, subscription.clone(), receiver).await;
                }
            }
            BaseDataType::Candles => {
                if let Some(broadcaster) = self.candle_feed_broadcasters.get(&subscription.symbol.name) {
                    let receiver = broadcaster.value().subscribe();
                    subscribe_stream(&stream_name, subscription.clone(), receiver).await;
                    is_subscribed = false;
                } else {
                    let (sender, receiver) = broadcast::channel(500);
                    self.candle_feed_broadcasters.insert(subscription.symbol.name.clone(), sender);
                    subscribe_stream(&stream_name, subscription.clone(), receiver).await;
                }
            }
            _ => todo!("Handle gracefully by returning err")
        }

        if !is_subscribed {
            let bits = match subscription.base_data_type {
                BaseDataType::Ticks => 1,
                BaseDataType::Quotes => 2,
                _ => return DataServerResponse::SubscribeResponse{ success: false, subscription: subscription.clone(), reason: Some(format!("This subscription is not available with DataVendor::Test: {}", subscription))}
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
            //todo Fix ff_rithmic_api switch heartbeat fn. this causes a lock.
       /*     match self.client.switch_heartbeat_required(SysInfraType::TickerPlant, false).await {
                Ok(_) => {}
                Err(_) => {}
            }*/
            match self.send_message(SysInfraType::TickerPlant, req).await {
                Ok(_) => {
                    println!("Subscribed to new ticker subscription");
                }
                Err(e) => {
                    eprintln!("Error sending subscribe request to ticker plant: {}", e);
                }
            }
        }
        println!("{} Subscribed: {}", stream_name, subscription);
        DataServerResponse::SubscribeResponse{ success: true, subscription: subscription.clone(), reason: None}
    }

    async fn data_feed_unsubscribe(&self, _mode: StrategyMode, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
        let exchange = match subscription.market_type {
            MarketType::Futures(exchange) => exchange.to_string(),
            _ => return DataServerResponse::SubscribeResponse {
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
            broadcaster_map.remove(&symbol);

            let req = RequestMarketDataUpdate {
                template_id: 100,
                user_msg: vec![],
                symbol: Some(symbol.clone()),
                exchange: Some(exchange),
                request: Some(2), // 2 for unsubscribe
                update_bits: Some(bits),
            };

            if let Err(e) = self.send_message(SysInfraType::TickerPlant, req).await {
                return DataServerResponse::UnSubscribeResponse {
                    success: false,
                    subscription,
                    reason: Some(format!("Failed to send unsubscribe message: {}", e)),
                };
            }

            // Additional cleanup for quotes
            if subscription.base_data_type == BaseDataType::Quotes {
                self.ask_book.remove(&symbol);
                self.bid_book.remove(&symbol);
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
            base_data_types: vec![BaseDataType::Ticks, BaseDataType::Quotes],
        }
    }

    async fn logout_command_vendors(&self, stream_name: StreamName) {
        self.callbacks.remove(&stream_name);
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