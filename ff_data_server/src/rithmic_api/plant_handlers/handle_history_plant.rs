use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;
use chrono::{DateTime, TimeZone, Utc};
use dashmap::DashMap;
use lazy_static::lazy_static;
#[allow(unused_imports)]
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::{AccountListUpdates, AccountPnLPositionUpdate, AccountRmsUpdates, BestBidOffer, BracketUpdates, DepthByOrder, DepthByOrderEndEvent, EndOfDayPrices, ExchangeOrderNotification, FrontMonthContractUpdate, IndicatorPrices, InstrumentPnLPositionUpdate, LastTrade, MarketMode, OpenInterest, OrderBook, OrderPriceLimits, QuoteStatistics, RequestAccountList, RequestAccountRmsInfo, RequestHeartbeat, RequestLoginInfo, RequestMarketDataUpdate, RequestPnLPositionSnapshot, RequestPnLPositionUpdates, RequestProductCodes, RequestProductRmsInfo, RequestReferenceData, RequestTickBarUpdate, RequestTimeBarUpdate, RequestVolumeProfileMinuteBars, ResponseAcceptAgreement, ResponseAccountList, ResponseAccountRmsInfo, ResponseAccountRmsUpdates, ResponseAuxilliaryReferenceData, ResponseBracketOrder, ResponseCancelAllOrders, ResponseCancelOrder, ResponseDepthByOrderSnapshot, ResponseDepthByOrderUpdates, ResponseEasyToBorrowList, ResponseExitPosition, ResponseFrontMonthContract, ResponseGetInstrumentByUnderlying, ResponseGetInstrumentByUnderlyingKeys, ResponseGetVolumeAtPrice, ResponseGiveTickSizeTypeTable, ResponseHeartbeat, ResponseLinkOrders, ResponseListAcceptedAgreements, ResponseListExchangePermissions, ResponseListUnacceptedAgreements, ResponseLogin, ResponseLoginInfo, ResponseLogout, ResponseMarketDataUpdate, ResponseMarketDataUpdateByUnderlying, ResponseModifyOrder, ResponseModifyOrderReferenceData, ResponseNewOrder, ResponseOcoOrder, ResponseOrderSessionConfig, ResponsePnLPositionSnapshot, ResponsePnLPositionUpdates, ResponseProductCodes, ResponseProductRmsInfo, ResponseReferenceData, ResponseReplayExecutions, ResponseResumeBars, ResponseRithmicSystemInfo, ResponseSearchSymbols, ResponseSetRithmicMrktDataSelfCertStatus, ResponseShowAgreement, ResponseShowBracketStops, ResponseShowBrackets, ResponseShowOrderHistory, ResponseShowOrderHistoryDates, ResponseShowOrderHistoryDetail, ResponseShowOrderHistorySummary, ResponseShowOrders, ResponseSubscribeForOrderUpdates, ResponseSubscribeToBracketUpdates, ResponseTickBarReplay, ResponseTickBarUpdate, ResponseTimeBarReplay, ResponseTimeBarUpdate, ResponseTradeRoutes, ResponseUpdateStopBracketLevel, ResponseUpdateTargetBracketLevel, ResponseVolumeProfileMinuteBars, RithmicOrderNotification, SymbolMarginRate, TickBar, TimeBar, TradeRoute, TradeStatistics, UpdateEasyToBorrowList};
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::Reject;
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_login::SysInfraType;
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::time_bar::BarType;
use prost::{Message as ProstMessage};
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive};
use rust_decimal_macros::dec;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::candle::Candle;
use ff_standard_lib::standardized_types::base_data::tick::{Aggressor, Tick};
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::enums::{FuturesExchange, MarketType};
use ff_standard_lib::standardized_types::new_types::{Price, Volume};
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::{CandleType, Symbol};
use crate::rithmic_api::api_client::RithmicBrokerageClient;

lazy_static! {
    pub static ref HISTORICAL_BUFFER: DashMap<u64, BTreeMap<DateTime<Utc>, BaseDataEnum>> = Default::default();
    pub static ref LAST_TIME: DashMap<u64, DateTime<Utc>> = Default::default();
}

#[allow(dead_code, unused)]
pub async fn match_history_plant_id(
    template_id: i32, message_buf: Vec<u8>,
    client: Arc<RithmicBrokerageClient>,
) {
    const PLANT: SysInfraType = SysInfraType::HistoryPlant;
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
                if let (Some(ssboe), Some(usecs)) = (msg.ssboe, msg.usecs) {
                    // Convert heartbeat timestamp to DateTime<Utc>
                    let heartbeat_time = match Utc.timestamp_opt(ssboe as i64, (usecs * 1000) as u32) {
                        chrono::LocalResult::Single(dt) => dt,
                        _ => return, // Skip if timestamp is invalid
                    };

                    // Calculate latency
                    let now = Utc::now();
                    let latency = now.signed_duration_since(heartbeat_time);

                    // Store both the send time and latency
                    client.heartbeat_times.insert(PLANT, now);
                    client.heartbeat_latency.insert(PLANT, latency.num_milliseconds());
                }
            }
        },
        201 => {
            if let Ok(msg) = ResponseTimeBarUpdate::decode(&message_buf[..]) {
                // Time Bar Update Response
                // From Server
                println!("Time Bar Update Response (Template ID: 201) from Server: {:?}", msg);
            }
        },
        203 => {
            if let Ok(msg) = ResponseTimeBarReplay::decode(&message_buf[..]) {
                const BASE_DATA_TYPE: BaseDataType = BaseDataType::Candles;
                // Time Bar Replay Response
                //println!("Time Bar Replay Response (Template ID: 203) from Server: {:?}", msg);

                let mut finished = false;
                if !msg.rp_code.is_empty() || msg.rq_handler_rp_code.is_empty() {
                    finished = true;
                }

                let candle = match parse_time_bar(&msg) {
                    Some(candle) => Some(candle),
                    None => None,
                };
                let user_msg = u64::from_str(msg.user_msg.get(0).unwrap()).unwrap();
                if !HISTORICAL_BUFFER.contains_key(&user_msg) {
                    HISTORICAL_BUFFER.insert(user_msg, BTreeMap::new());
                }
                if !finished {
                    if let Some(mut buffer) = HISTORICAL_BUFFER.get_mut(&user_msg) {
                        // More messages coming, buffer the data
                        if let Some(candle) = candle {
                            buffer.insert(candle.time_utc(), BaseDataEnum::Candle(candle));
                        }
                        return
                    }
                } else if let Some((id, mut buffer)) = HISTORICAL_BUFFER.remove(&user_msg) {
                    if (msg.symbol.is_none() && buffer.len() == 0) || buffer.len() > 0 {
                        if let Some(candle) = candle {
                            buffer.insert(candle.time_utc(), BaseDataEnum::Candle(candle));
                        }
                        if let Some((_, mut sender)) = client.historical_callbacks.remove(&id) {
                            let _ = sender.send(buffer);
                        }
                    }
                }
            }
        },
        205 => {
            if let Ok(msg) = ResponseTickBarUpdate::decode(&message_buf[..]) {
                // Tick Bar Update Response
                // From Server
                println!("Tick Bar Update Response (Template ID: 205) from Server: {:?}", msg);
            }
        },
        207 => {
            const BASE_DATA_TYPE: BaseDataType = BaseDataType::Ticks;
            if let Ok(msg) = ResponseTickBarReplay::decode(&message_buf[..]) {
                // Tick Bar Replay Response
                // From Server
                //println!("Tick Bar Replay Response (Template ID: 207) from Server: {:?}", msg);

                let mut finished = false;
                if !msg.rp_code.is_empty() || msg.rq_handler_rp_code.is_empty() {
                    finished = true;
                }

                let tick = match parse_tick_response(&msg) {
                    Some(tick) => Some(tick),
                    None => None,
                };
                let user_msg = u64::from_str(msg.user_msg.get(0).unwrap()).unwrap();
                if !HISTORICAL_BUFFER.contains_key(&user_msg) {
                    HISTORICAL_BUFFER.insert(user_msg, BTreeMap::new());
                }
                if !finished {
                    if let Some(mut buffer) = HISTORICAL_BUFFER.get_mut(&user_msg) {
                        // More messages coming, buffer the data
                        if let Some(mut tick) = tick {
                            let time = tick.time_utc();
                            if let Some(last_time) = LAST_TIME.get(&user_msg) {
                                tick.time = (time + std::time::Duration::from_nanos(1)).to_string();
                            }
                            LAST_TIME.insert(user_msg, time);
                            buffer.insert(time, BaseDataEnum::Tick(tick));
                        }
                        return
                    }
                } else if let Some((id, mut buffer)) = HISTORICAL_BUFFER.remove(&user_msg) {
                    if (msg.symbol.is_none() && buffer.len() == 0) || buffer.len() > 0 {
                        if let Some(tick) = tick {
                            buffer.insert(tick.time_utc(), BaseDataEnum::Tick(tick));
                        }
                        if let Some((_, mut sender)) = client.historical_callbacks.remove(&id) {
                            let _ = sender.send(buffer);
                        }
                    }
                    LAST_TIME.remove(&id);
                }
            }
        },
        208 => {
            if let Ok(msg) = RequestVolumeProfileMinuteBars::decode(&message_buf[..]) {
                // Volume Profile Minute Bars Request
                // From Client
                println!("Volume Profile Minute Bars Request (Template ID: 208) from Client: {:?}", msg);
            }
        },
        209 => {
            if let Ok(msg) = ResponseVolumeProfileMinuteBars::decode(&message_buf[..]) {
                // Volume Profile Minute Bars Response
                // From Server
                println!("Volume Profile Minute Bars Response (Template ID: 209) from Server: {:?}", msg);
            }
        },
        211 => {
            if let Ok(msg) = ResponseResumeBars::decode(&message_buf[..]) {
                // Resume Bars Response
                // From Server
                println!("Resume Bars Response (Template ID: 211) from Server: {:?}", msg);
            }
        },
        250 => {
            if let Ok(msg) = TimeBar::decode(&message_buf[..]) {
                // Time Bar
                // From Server
                //println!("Time Bar (Template ID: 250) from Server: {:?}", msg);
                handle_candle(client.clone(), msg).await;
            }
        },
        251 => {
            if let Ok(msg) = TickBar::decode(&message_buf[..]) {
                // Tick Bar
                // From Server
                println!("Tick Bar (Template ID: 251) from Server: {:?}", msg);
            }
        },
        _ => println!("No match for template_id: {}", template_id)
    }
}

async fn handle_candle(client: Arc<RithmicBrokerageClient>, msg: TimeBar) {
    //println!("Time Bar (Template ID: 250) from Server: {:?}", msg);
    let time = match deserialize_candle_time(&msg) {
        None => return,
        Some(time) => time
    };

    let symbol = match msg.symbol {
        None => return,
        Some(symbol) => symbol,
    };

    // Deserialize the exchange field
    let exchange = match msg.exchange.as_deref().and_then(|e| FuturesExchange::from_string(e).ok()) {
        Some(ex) => ex,
        None => {
            eprintln!("Error deserializing Exchange for symbol {}", symbol);
            return;
        }
    };

    let mut remove_broadcaster = false;
    let period = match msg.period.clone() {
        Some(p) => match p.parse::<u64>().ok() {
            None => return,
            Some(period) => period
        },
        None => return,
    };
    // Retrieve broadcaster for the symbol
    if let Some(broadcaster) = client.candle_feed_broadcasters.get(&symbol) {
        // Construct the symbol object
        let symbol_obj = Symbol::new(
            symbol.clone(),
            client.data_vendor.clone(),
            MarketType::Futures(exchange),
        );

        let high = match msg.high_price.and_then(Decimal::from_f64) {
            Some(price) => price,
            None => return,  // Exit if high price is invalid
        };

        let low = match msg.low_price.and_then(Decimal::from_f64) {
            Some(price) => price,
            None => return,  // Exit if low price is invalid
        };

        let open = match msg.open_price.and_then(Decimal::from_f64) {
            Some(price) => price,
            None => return,  // Exit if open price is invalid
        };

        let close = match msg.close_price.and_then(Decimal::from_f64) {
            Some(price) => price,
            None => return,  // Exit if close price is invalid
        };

        let mut resolution = match msg.r#type.clone() {
            Some(val) => {
                match val {
                    1 => Resolution::Seconds(period as u64),
                    2 => Resolution::Minutes(period as u64),
                    _ => // Try parsing as i32 first
                        match i32::from_str(val.to_string().as_str()) {
                            Ok(1) => Resolution::Seconds(period as u64),
                            Ok(2) => Resolution::Minutes(period as u64),
                            // If parsing as number fails, try as bar type string
                            _ => match BarType::from_str_name(&val.to_string().as_str()) {
                                Some(BarType::SecondBar) => Resolution::Seconds(period),
                                Some(BarType::MinuteBar) => Resolution::Minutes(period),
                                Some(BarType::DailyBar) | Some(BarType::WeeklyBar) => return, // Unsupported bar types
                                None => return, // Unknown bar type
                            }
                        }
                }
            },
            None => return, // Exit if msg.r#type is None
        };

        // Convert resolution if needed
        if let Resolution::Minutes(mins) = resolution {
            if mins >= 60 && mins % 60 == 0 {
                resolution = Resolution::Hours(mins / 60);
            }
        } else if resolution == Resolution::Seconds(60) {
            resolution = Resolution::Minutes(1);
        }

        let ask_volume= msg.ask_volume.and_then(Decimal::from_u64).unwrap_or_else(|| dec!(0.0));

        let bid_volume= msg.bid_volume.and_then(Decimal::from_u64).unwrap_or_else(|| dec!(0.0));

        let range = high - low;

        // Construct the candle
        let data = BaseDataEnum::Candle(Candle {
            symbol: symbol_obj.clone(),
            high,
            low,
            open,
            close,
            volume: Decimal::from_u64(msg.volume.unwrap_or_default()).unwrap_or(Decimal::ZERO),
            ask_volume,
            bid_volume,
            range,
            time: time.to_string(),
            is_closed: true,
            resolution,
            candle_type: CandleType::CandleStick,
        });


        //println!("Candle: {:?}", data);
        // Send the candle data
        if let Err(_) = broadcaster.send(data) {
            if broadcaster.receiver_count() == 0 {
                remove_broadcaster = true;
            }
        }
    }

    if remove_broadcaster {
        let bar_type = match msg.r#type {
            Some(num) => num,
            None => return,              // Exit if `msg.r#type` is None
        };
        if let Some((_, broadcaster)) = client.candle_feed_broadcasters.remove(&symbol) {
            let period = match msg.period {
                Some(p) => match p.parse::<i32>().ok() {
                    None => return,
                    Some(period) => period
                },
                None => return,
            };

            if broadcaster.receiver_count() == 0 {
                let req = RequestTimeBarUpdate {
                    template_id: 200,
                    user_msg: vec![],
                    symbol: Some(symbol.clone()),
                    exchange: Some(exchange.to_string()),
                    request: Some(2),// 2 for unsubscribe
                    bar_type: Some(bar_type),
                    bar_type_period: Some(period),
                };

                const PLANT: SysInfraType = SysInfraType::HistoryPlant;
                client.send_message(&PLANT, req).await;
                println!("Unsubscribed {} Candles {}, {}", symbol, bar_type, period);
            }
        }
    }
}


fn deserialize_candle_time(msg: &TimeBar) -> Option<DateTime<Utc>> {
    msg.marker
        .and_then(|marker| Utc.timestamp_opt(marker as i64, 0).single())
}

fn parse_tick_response(response: &ResponseTickBarReplay) -> Option<Tick> {
    // Early returns for required fields
    let symbol_name = response.symbol.as_ref()?;
    let exchange = response.exchange.as_ref()?;
    let volume = response.volume?;
    let price = response.close_price?;

    let price = match Decimal::from_f64(price) {
        Some(price) => price,
        None => return None,
    };

    let volume = match Decimal::from_u64(volume) {
        Some(volume) => volume,
        None => return None,
    };

    let bid_volume = response.bid_volume.unwrap_or_default();
    let ask_volume = response.ask_volume.unwrap_or_default();

    let aggressor =  match (bid_volume, ask_volume) {
        (bid, ask) if bid > ask => Aggressor::Buy,   // Volume executed at bid means aggressive buying
        (bid, ask) if ask > bid => Aggressor::Sell,  // Volume executed at ask means aggressive selling
        (bid, ask) if bid == ask && bid > 0 => Aggressor::None,  // Equal non-zero volume
        _ => Aggressor::None,  // Default case (including when both are 0)
    };

    // Get first timestamp (since both are identical for ticks)
    let ssboe = response.data_bar_ssboe.get(0)?;
    let usecs = response.data_bar_usecs.get(0)?;

    // Convert to DateTime
    let datetime = Utc.timestamp_opt(*ssboe as i64, *usecs as u32 * 1000).unwrap();

    let exchange = match FuturesExchange::from_string(exchange) {
        Ok(exchange) => exchange,
        Err(_) => return None,
    };

    Some(Tick {
        symbol: Symbol::new(symbol_name.clone(), DataVendor::Rithmic, MarketType::Futures(exchange)),
        price,
        time: datetime.to_string(),
        volume,
        aggressor,
    })
}

fn parse_time_bar(response: &ResponseTimeBarReplay) -> Option<Candle> {
    //println!("ResponseTimeBarReplay: {:?}", response);
    // Check if all required price fields are present
    let (open, high, low, close) = match (
        response.open_price,
        response.high_price,
        response.low_price,
        response.close_price,
    ) {
        (Some(o), Some(h), Some(l), Some(c)) => (o, h, l, c),
        _ => return None,
    };

    let symbol = response.symbol.as_ref()?;
    let volume = response.volume?;
    let bid_volume = response.bid_volume.unwrap_or_default();
    let ask_volume = response.ask_volume.unwrap_or_default();
    let range = high - low;
    let exchange = response.exchange.as_ref()?;
    let exchange = match FuturesExchange::from_string(exchange) {
        Ok(exchange) => exchange,
        Err(_) => return None,
    };

    // Get timestamp from marker field
    let marker = response.marker?;
    let datetime = Utc.timestamp_opt(marker as i64, 0).single()?;

    let period = match response.period.clone() {
        Some(p) => match p.parse::<i32>().ok() {
            None => return None,
            Some(period) => period
        },
        None => return None,
    };

    let mut resolution = match response.r#type.clone() {
        Some(type_str) => {
            match type_str {
                1 => Resolution::Seconds(period as u64),
                2 => Resolution::Minutes(period as u64),
                // Try parsing as BarType first
                _ => match BarType::from_str_name(type_str.to_string().as_str()) {
                    Some(BarType::MinuteBar) => Resolution::Minutes(period as u64),
                    Some(BarType::SecondBar) => Resolution::Seconds(period as u64),
                    None => {
                        // Fall back to numeric checks if not a recognized bar type string
                        match type_str {
                            1 => Resolution::Seconds(period as u64),
                            2 => Resolution::Minutes(period as u64),
                            _ => return None, // Unsupported bar types
                        }
                    }
                    _ => {
                        match i32::from_str(type_str.to_string().as_str()) {
                            Ok(1) => Resolution::Seconds(period as u64),
                            Ok(2) => Resolution::Minutes(period as u64),
                            _ => return None, // Unsupported bar types
                        }
                    }, // Unsupported bar types
                }
            }
        }
        None => return None,
    };

    // Convert resolution if needed
    if let Resolution::Minutes(mins) = resolution {
        if mins >= 60 && mins % 60 == 0 {
            resolution = Resolution::Hours(mins / 60);
        }
    } else if resolution == Resolution::Seconds(60) {
        resolution = Resolution::Minutes(1);
    }

    //println!("resolution: {:?}", resolution);

    Some(Candle {
        symbol: Symbol::new(symbol.clone(), DataVendor::Rithmic, MarketType::Futures(exchange)),
        high: Price::from_f64(high).unwrap(),
        low: Price::from_f64(low).unwrap(),
        open: Price::from_f64(open).unwrap(),
        close: Price::from_f64(close).unwrap(),
        volume: Volume::from_u64(volume).unwrap(),
        ask_volume:Volume::from_u64(ask_volume).unwrap(),
        bid_volume: Volume::from_u64(bid_volume).unwrap(),
        range: Price::from_f64(range).unwrap(),
        time: datetime.to_string(),
        is_closed: true,
        resolution,
        candle_type: CandleType::CandleStick,
    })
}
