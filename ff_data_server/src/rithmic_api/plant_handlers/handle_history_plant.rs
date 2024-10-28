use std::sync::Arc;
use chrono::{DateTime, TimeZone, Utc};
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
use ff_standard_lib::standardized_types::base_data::candle::Candle;
#[allow(unused_imports)]
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::enums::{FuturesExchange, MarketType};
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::{CandleType, Symbol};
use crate::rithmic_api::api_client::RithmicClient;

#[allow(dead_code, unused)]
pub async fn match_history_plant_id(
    template_id: i32, message_buf: Vec<u8>,
    client: Arc<RithmicClient>,
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
                client.handle_response_heartbeat(PLANT, msg);
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
                // Time Bar Replay Response
                // From Server
                println!("Time Bar Replay Response (Template ID: 203) from Server: {:?}", msg);
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
            if let Ok(msg) = ResponseTickBarReplay::decode(&message_buf[..]) {
                // Tick Bar Replay Response
                // From Server
                println!("Tick Bar Replay Response (Template ID: 207) from Server: {:?}", msg);
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

async fn handle_candle(client: Arc<RithmicClient>, msg: TimeBar) {
    println!("Time Bar (Template ID: 250) from Server: {:?}", msg);
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

        let resolution = match msg.r#type.clone() {
            Some(num) => {
                let bar_type = match BarType::try_from(num) {
                    Ok(bar_type) => bar_type,
                    Err(_) => return, // Exit if bar type conversion fails
                };

                match (bar_type, period) {
                    (BarType::SecondBar, p) => Resolution::Seconds(p),
                    (BarType::MinuteBar, p) => Resolution::Minutes(p),
                    (BarType::DailyBar, _) | (BarType::WeeklyBar, _) => return, // Unsupported bar types
                }
            }
            None => return, // Exit if msg.r#type is None
        };

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