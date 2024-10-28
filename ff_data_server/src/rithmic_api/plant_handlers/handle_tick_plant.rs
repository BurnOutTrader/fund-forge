use std::str::FromStr;
use std::sync::Arc;
#[allow(unused_imports)]
use std::time::Duration;
use chrono::{DateTime, TimeZone, Utc};
#[allow(unused_imports)]
#[allow(unused_imports)]
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::{AccountListUpdates, AccountPnLPositionUpdate, AccountRmsUpdates, BestBidOffer, BracketUpdates, DepthByOrder, DepthByOrderEndEvent, EndOfDayPrices, ExchangeOrderNotification, FrontMonthContractUpdate, IndicatorPrices, InstrumentPnLPositionUpdate, LastTrade, MarketMode, OpenInterest, OrderBook, OrderPriceLimits, QuoteStatistics, RequestAccountList, RequestAccountRmsInfo, RequestHeartbeat, RequestLoginInfo, RequestMarketDataUpdate, RequestPnLPositionSnapshot, RequestPnLPositionUpdates, RequestProductCodes, RequestProductRmsInfo, RequestReferenceData, RequestTickBarUpdate, RequestTimeBarUpdate, RequestVolumeProfileMinuteBars, ResponseAcceptAgreement, ResponseAccountList, ResponseAccountRmsInfo, ResponseAccountRmsUpdates, ResponseAuxilliaryReferenceData, ResponseBracketOrder, ResponseCancelAllOrders, ResponseCancelOrder, ResponseDepthByOrderSnapshot, ResponseDepthByOrderUpdates, ResponseEasyToBorrowList, ResponseExitPosition, ResponseFrontMonthContract, ResponseGetInstrumentByUnderlying, ResponseGetInstrumentByUnderlyingKeys, ResponseGetVolumeAtPrice, ResponseGiveTickSizeTypeTable, ResponseHeartbeat, ResponseLinkOrders, ResponseListAcceptedAgreements, ResponseListExchangePermissions, ResponseListUnacceptedAgreements, ResponseLogin, ResponseLoginInfo, ResponseLogout, ResponseMarketDataUpdate, ResponseMarketDataUpdateByUnderlying, ResponseModifyOrder, ResponseModifyOrderReferenceData, ResponseNewOrder, ResponseOcoOrder, ResponseOrderSessionConfig, ResponsePnLPositionSnapshot, ResponsePnLPositionUpdates, ResponseProductCodes, ResponseProductRmsInfo, ResponseReferenceData, ResponseReplayExecutions, ResponseResumeBars, ResponseRithmicSystemInfo, ResponseSearchSymbols, ResponseSetRithmicMrktDataSelfCertStatus, ResponseShowAgreement, ResponseShowBracketStops, ResponseShowBrackets, ResponseShowOrderHistory, ResponseShowOrderHistoryDates, ResponseShowOrderHistoryDetail, ResponseShowOrderHistorySummary, ResponseShowOrders, ResponseSubscribeForOrderUpdates, ResponseSubscribeToBracketUpdates, ResponseTickBarReplay, ResponseTickBarUpdate, ResponseTimeBarReplay, ResponseTimeBarUpdate, ResponseTradeRoutes, ResponseUpdateStopBracketLevel, ResponseUpdateTargetBracketLevel, ResponseVolumeProfileMinuteBars, RithmicOrderNotification, SymbolMarginRate, TickBar, TimeBar, TradeRoute, TradeStatistics, UpdateEasyToBorrowList};
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::Reject;
use prost::Message as ProstMessage;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
#[allow(unused_imports)]
use tokio::time::sleep;
use ff_standard_lib::messages::data_server_messaging::DataServerResponse;
#[allow(unused_imports)]
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::quote::Quote;
use ff_standard_lib::standardized_types::base_data::tick::{Aggressor, Tick};
use ff_standard_lib::standardized_types::enums::{FuturesExchange, MarketType};
use ff_standard_lib::standardized_types::subscriptions::Symbol;
use ff_standard_lib::standardized_types::symbol_info::FrontMonthInfo;
use ff_standard_lib::standardized_types::books::BookLevel;
use ff_standard_lib::StreamName;
use crate::rithmic_api::api_client::RithmicClient;
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_login::SysInfraType;

#[allow(unused, dead_code)]
pub async fn match_ticker_plant_id(
    template_id: i32, message_buf: Vec<u8>,
    client: Arc<RithmicClient>,
) {
    const PLANT: SysInfraType = SysInfraType::OrderPlant;
    match template_id {
        75 => {
            if let Ok(msg) = Reject::decode(&message_buf[..]) {
                // Login Response
                // From Server
                println!("Reject Response (Template ID: 11) from Server: {:?}", msg);
            }
        }
        11 => {
            if let Ok(msg) = ResponseLogin::decode(&message_buf[..]) {
                // Login Response
                // From Server
                println!("Login Response (Template ID: 11) from Server: {:?}", msg);
            }
        },
        13 => {
            if let Ok(msg) = ResponseLogout::decode(&message_buf[..]) {
                // Logout Response
                // From Server
                println!("Logout Response (Template ID: 13) from Server: {:?}", msg);
            }
        },
        15 => {
            if let Ok(msg) = ResponseReferenceData::decode(&message_buf[..]) {
                // Reference Data Response
                // From Server
                println!("Reference Data Response (Template ID: 15) from Server: {:?}", msg);
            }
        },
        17 => {
            if let Ok(msg) = ResponseRithmicSystemInfo::decode(&message_buf[..]) {
                // Rithmic System Info Response
                // From Server
                println!("Rithmic System Info Response (Template ID: 17) from Server: {:?}", msg);
            }
        },
        19 => {
            if let Ok(msg) = ResponseHeartbeat::decode(&message_buf[..]) {
                // Response Heartbeat
                // From Server
                //println!("Response Heartbeat (Template ID: 19) from Server: {:?}", msg);

                client.handle_response_heartbeat(PLANT, msg);
            }
        },
        101 => {
            if let Ok(msg) = ResponseMarketDataUpdate::decode(&message_buf[..]) {
                // Market Data Update Response
                // From Server
                println!("Market Data Update Response (Template ID: 101) from Server: {:?}", msg);
            }
        },
        103 => {
            if let Ok(msg) = ResponseGetInstrumentByUnderlying::decode(&message_buf[..]) {
                // Get Instrument by Underlying Response
                // From Server
                println!("Get Instrument by Underlying Response (Template ID: 103) from Server: {:?}", msg);
            }
        },
        104 => {
            if let Ok(msg) = ResponseGetInstrumentByUnderlyingKeys::decode(&message_buf[..]) {
                // Get Instrument by Underlying Keys Response
                // From Server
                println!("Get Instrument by Underlying Keys Response (Template ID: 104) from Server: {:?}", msg);
            }
        },
        106 => {
            if let Ok(msg) = ResponseMarketDataUpdateByUnderlying::decode(&message_buf[..]) {
                // Market Data Update by Underlying Response
                // From Server
                println!("Market Data Update by Underlying Response (Template ID: 106) from Server: {:?}", msg);
            }
        },
        108 => {
            if let Ok(msg) = ResponseGiveTickSizeTypeTable::decode(&message_buf[..]) {
                // Give Tick Size Type Table Response
                // From Server
                println!("Give Tick Size Type Table Response (Template ID: 108) from Server: {:?}", msg);
            }
        },
        110 => {
            if let Ok(msg) = ResponseSearchSymbols::decode(&message_buf[..]) {
                // Search Symbols Response
                // From Server
                println!("Search Symbols Response (Template ID: 110) from Server: {:?}", msg);
            }
        },
        112 => {
            if let Ok(msg) = ResponseProductCodes::decode(&message_buf[..]) {
                // Product Codes Response
                // From Server
                println!("Product Codes Response (Template ID: 112) from Server: {:?}", msg);
            }
        },
        114 => {
            if let Ok(msg) = ResponseFrontMonthContract::decode(&message_buf[..]) {
                // Front Month Contract Response
                // From Server
                //println!("Front Month Contract Response (Template ID: 114) from Server: {:?}", msg);
                if let Some(symbol) =  msg.symbol {
                    if let Some(exchange) = msg.exchange {
                        let exchange = match FuturesExchange::from_string(&exchange) {
                            Ok(exchange) => exchange,
                            Err(e) => {
                                eprintln!("{}", e);
                                return;
                            }
                        };
                        if let Some(trade_symbol) = msg.trading_symbol {
                            let front_month_info = FrontMonthInfo {
                                exchange,
                                symbol_name: symbol.clone(),
                                symbol_code: trade_symbol,
                            };
                            if let Some(stream_name) = msg.user_msg.get(0) {
                                let stream_name = match StreamName::from_str(&stream_name) {
                                    Ok(name) => name,
                                    Err(_) => return
                                };
                                if let Some(callback_id) = msg.user_msg.get(1) {
                                    let id = match u64::from_str(callback_id) {
                                        Ok(callback_id) => callback_id,
                                        Err(_) => return
                                    };
                                    let response = DataServerResponse::FrontMonthInfo {callback_id: id.clone(), info: front_month_info.clone()};
                                    client.send_callback(stream_name, id, response).await;
                                }
                            }
                        }
                    }
                }
            }
        },
        116 => {
            if let Ok(msg) = ResponseDepthByOrderSnapshot::decode(&message_buf[..]) {
                // Depth By Order Snapshot Response
                // From Server
                println!("Depth By Order Snapshot Response (Template ID: 116) from Server: {:?}", msg);
            }
        },
        118 => {
            if let Ok(msg) = ResponseDepthByOrderUpdates::decode(&message_buf[..]) {
                // Depth By Order Updates Response
                // From Server
                println!("Depth By Order Updates Response (Template ID: 118) from Server: {:?}", msg);
            }
        },
        120 => {
            if let Ok(msg) = ResponseGetVolumeAtPrice::decode(&message_buf[..]) {
                // Get Volume At Price Response
                // From Server
                println!("Get Volume At Price Response (Template ID: 120) from Server: {:?}", msg);
            }
        },
        122 => {
            if let Ok(msg) = ResponseAuxilliaryReferenceData::decode(&message_buf[..]) {
                // Auxiliary Reference Data Response
                // From Server
                println!("Auxiliary Reference Data Response (Template ID: 122) from Server: {:?}", msg);
            }
        },
        150 => {
            if let Ok(msg) = LastTrade::decode(&message_buf[..]) {
                // println!("Last Trade (Template ID: 150) from Server: {:?}", msg);
                handle_tick(client.clone(), msg).await;
            }
        },
        151 => {
            if let Ok(msg) = BestBidOffer::decode(&message_buf[..]) {
                // Best Bid Offer
                // From Server
                //println!("Best Bid Offer (Template ID: 151) from Server: {:?}", msg);
                handle_quote(client.clone(), msg).await;
            }
        },
        152 => {
            if let Ok(msg) = TradeStatistics::decode(&message_buf[..]) {
                // Trade Statistics
                // From Server
                println!("Trade Statistics (Template ID: 152) from Server: {:?}", msg);
            }
        },
        153 => {
            if let Ok(msg) = QuoteStatistics::decode(&message_buf[..]) {
                // Quote Statistics
                // From Server
                println!("Quote Statistics (Template ID: 153) from Server: {:?}", msg);
            }
        },
        154 => {
            if let Ok(msg) = IndicatorPrices::decode(&message_buf[..]) {
                // Indicator Prices
                // From Server
                println!("Indicator Prices (Template ID: 154) from Server: {:?}", msg);
            }
        },
        155 => {
            if let Ok(msg) = EndOfDayPrices::decode(&message_buf[..]) {
                // End Of Day Prices
                // From Server
                println!("End Of Day Prices (Template ID: 155) from Server: {:?}", msg);
            }
        },
        156 => {
            if let Ok(msg) = OrderBook::decode(&message_buf[..]) {
                // Order Book
                // From Server
                println!("Order Book (Template ID: 156) from Server: {:?}", msg);
            }
        },
        157 => {
            if let Ok(msg) = MarketMode::decode(&message_buf[..]) {
                // Market Mode
                // From Server
                println!("Market Mode (Template ID: 157) from Server: {:?}", msg);
            }
        },
        158 => {
            if let Ok(msg) = OpenInterest::decode(&message_buf[..]) {
                // Open Interest
                // From Server
                println!("Open Interest (Template ID: 158) from Server: {:?}", msg);
            }
        },
        159 => {
            if let Ok(msg) = FrontMonthContractUpdate::decode(&message_buf[..]) {
                // Front Month Contract Update
                // From Server
                println!("Front Month Contract Update (Template ID: 159) from Server: {:?}", msg);
            }
        },
        160 => {
            if let Ok(msg) = DepthByOrder::decode(&message_buf[..]) {
                // Depth By Order
                // From Server
                println!("Depth By Order (Template ID: 160) from Server: {:?}", msg);
            }
        },
        161 => {
            if let Ok(msg) = DepthByOrderEndEvent::decode(&message_buf[..]) {
                // Depth By Order End Event
                // From Server
                println!("DepthByOrderEndEvent (Template ID: 161) from Server: {:?}", msg);
            }
        },
        162 => {
            if let Ok(msg) = SymbolMarginRate::decode(&message_buf[..]) {
                // Symbol Margin Rate
                // From Server
                println!("Symbol Margin Rate (Template ID: 162) from Server: {:?}", msg);
            }
        },
        163 => {
            if let Ok(msg) = OrderPriceLimits::decode(&message_buf[..]) {
                // Order Price Limits
                // From Server
                println!("Order Price Limits (Template ID: 163) from Server: {:?}", msg);
            }
        },
        _ => println!("No match for template_id: {}", template_id)
    }
}

async fn handle_tick(client: Arc<RithmicClient>, msg: LastTrade) {
    let time = deserialize_time(&msg);
   // println!("{:?}", msg);
    let volume = match msg.trade_size {
        None => return,
        Some(size) => {
            match Decimal::from_i32(size) {
                None => return,
                Some(size) => size
            }
        }
    };

    let exchange = match msg.exchange {
        None => return,
        Some(exchange) => {
            match FuturesExchange::from_string(&exchange) {
                Ok(ex) => ex,
                Err(_e) => {
                    return
                }
            }
        }
    };

    let price = match msg.trade_price {
        None => return,
        Some(price) => match Decimal::from_f64(price) {
            None => return,
            Some(price) => price
        }
    };
    let side = match msg.aggressor {
        None => Aggressor::None,
        Some(aggressor) => {
            match aggressor {
                1 => Aggressor::Buy,
                2 => Aggressor::Sell,
                _ => Aggressor::None,
            }
        }
    };
    let symbol = match msg.symbol {
        None => return,
        Some(symbol) => symbol
    };

    let symbol = Symbol::new(symbol, client.data_vendor.clone(), MarketType::Futures(exchange));
    let tick = Tick::new(symbol.clone(), price, time.to_string(), volume, side);
    let mut remove_broadcaster = false;
    if let Some(broadcaster) = client.tick_feed_broadcasters.get(&tick.symbol.name) {
        match broadcaster.value().send(BaseDataEnum::Tick(tick.clone())) {
            Ok(_) => {}
            Err(_) => {
                if broadcaster.receiver_count() == 0 {
                    remove_broadcaster = true;
                }
            }
        }
    }

    if remove_broadcaster {
        if let Some((_, broadcaster)) = client.tick_feed_broadcasters.remove(&tick.symbol.name) {
            if broadcaster.receiver_count() == 0 {
                let req = RequestMarketDataUpdate {
                    template_id: 100,
                    user_msg: vec![],
                    symbol: Some(tick.symbol.name.clone()),
                    exchange: Some(exchange.to_string()),
                    request: Some(2), // 2 for unsubscribe
                    update_bits: Some(1), //1 ticks, 2 quotes
                };

                const PLANT: SysInfraType = SysInfraType::TickerPlant;
                client.send_message(&PLANT, req).await;
                println!("Unsubscribed: {} Ticks", tick.symbol.name.clone());
            }
        }
    }
}

fn deserialize_time(msg: &LastTrade) -> DateTime<Utc> {
    msg.source_ssboe
        .and_then(|ssboe| msg.source_nsecs.map(|nsecs| (ssboe, nsecs)))
        .and_then(|(ssboe, nsecs)| {
            Utc.timestamp_opt(ssboe as i64, nsecs as u32).single()
        })
        .or_else(|| {
            msg.ssboe
                .and_then(|ssboe| msg.usecs.map(|usecs| (ssboe, usecs)))
                .and_then(|(ssboe, usecs)| {
                    Utc.timestamp_opt(ssboe as i64, usecs as u32 * 1000).single()
                })
        })
        .unwrap_or_else(|| {
            //eprintln!("Warning: Using current time due to invalid timestamp in message");
            Utc::now()
        })
}

async fn handle_quote(client: Arc<RithmicClient>, msg: BestBidOffer) {
    let time = deserialize_quote_time(&msg);
    let symbol = match msg.symbol {
        None => return,
        Some(symbol) => symbol
    };

    let mut updated = false;

    // Handle ask update
    if let (Some(price), Some(volume)) = (msg.ask_price, msg.ask_size) {
        if let (Some(ask_price), Some(ask_volume)) = (Decimal::from_f64(price), Decimal::from_i32(volume)) {
            client.ask_book.entry(symbol.clone()).or_default().insert(0, BookLevel::new(0, ask_price, ask_volume));
            updated = true;
        }
    }

    // Handle bid update
    if let (Some(price), Some(volume)) = (msg.bid_price, msg.bid_size) {
        if let (Some(bid_price), Some(bid_volume)) = (Decimal::from_f64(price), Decimal::from_i32(volume)) {
            client.bid_book.entry(symbol.clone()).or_default().insert(0, BookLevel::new(0, bid_price, bid_volume));
            updated = true;
        }
    }

    if !updated {
        return;
    }

    let exchange = match msg.exchange.as_deref().and_then(|e| FuturesExchange::from_string(e).ok()) {
        Some(ex) => ex,
        None => {
            eprintln!("Error deserializing Exchange for symbol {}", symbol);
            return;
        }
    };

    let mut remove_broadcaster = false;
    if let Some(broadcaster) = client.quote_feed_broadcasters.get(&symbol) {
        let (ask, ask_volume) = client.ask_book
            .get(&symbol)
            .and_then(|book| book.get(&0).map(|level| (level.price, level.volume)))
            .unwrap_or_else(|| (Decimal::ZERO, Decimal::ZERO));

        let (bid, bid_volume) = client.bid_book
            .get(&symbol)
            .and_then(|book| book.get(&0).map(|level| (level.price, level.volume)))
            .unwrap_or_else(|| (Decimal::ZERO, Decimal::ZERO));

        // Return if we don't have both valid sides
        if ask == Decimal::ZERO || bid == Decimal::ZERO {
            return;
        }

        let symbol_obj = Symbol::new(symbol.clone(), client.data_vendor.clone(), MarketType::Futures(exchange));
        let data = BaseDataEnum::Quote(
            Quote {
                symbol: symbol_obj.clone(),
                ask,
                bid,
                ask_volume,
                bid_volume,
                time: time.to_string(),
            }
        );

        if let Err(_e) = broadcaster.send(data) {
            if broadcaster.receiver_count() == 0 {
                remove_broadcaster = true;
            }
        }
    }

    if remove_broadcaster {
        if let Some((_, broadcaster)) = client.quote_feed_broadcasters.remove(&symbol) {
            if broadcaster.receiver_count() == 0 {
                client.ask_book.remove(&symbol);
                client.bid_book.remove(&symbol);
                let req = RequestMarketDataUpdate {
                    template_id: 100,
                    user_msg: vec![],
                    symbol: Some(symbol.clone()),
                    exchange: Some(exchange.to_string()),
                    request: Some(2), // 2 for unsubscribe
                    update_bits: Some(2), //1 ticks, 2 quotes
                };

                const PLANT: SysInfraType = SysInfraType::TickerPlant;
                client.send_message(&PLANT, req).await;
                println!("Unsubscribed: {} Quotes", symbol);
            }
        }
    }
}

fn deserialize_quote_time(msg: &BestBidOffer) -> DateTime<Utc> {
    msg.ssboe
        .and_then(|ssboe| msg.usecs.map(|usecs| (ssboe, usecs)))
        .and_then(|(ssboe, usecs)| {
            Utc.timestamp_opt(ssboe as i64, usecs as u32 * 1000).single()
        })
        .unwrap_or_else(|| {
            //eprintln!("Warning: Using current time due to invalid timestamp in quote");
            Utc::now()
        })
}