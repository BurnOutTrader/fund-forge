use std::io::Cursor;
use std::sync::Arc;
use async_std::stream::StreamExt;
#[allow(unused_imports)]
use ff_rithmic_api::credentials::RithmicCredentials;
use ff_rithmic_api::errors::RithmicApiError;
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
#[allow(unused_imports)]
use ff_rithmic_api::rithmic_proto_objects::rti::{AccountListUpdates, AccountPnLPositionUpdate, AccountRmsUpdates, BestBidOffer, BracketUpdates, DepthByOrder, DepthByOrderEndEvent, EndOfDayPrices, ExchangeOrderNotification, FrontMonthContractUpdate, IndicatorPrices, InstrumentPnLPositionUpdate, LastTrade, MarketMode, OpenInterest, OrderBook, OrderPriceLimits, QuoteStatistics, RequestAccountList, RequestAccountRmsInfo, RequestHeartbeat, RequestLoginInfo, RequestMarketDataUpdate, RequestPnLPositionSnapshot, RequestPnLPositionUpdates, RequestProductCodes, RequestProductRmsInfo, RequestReferenceData, RequestTickBarUpdate, RequestTimeBarUpdate, RequestVolumeProfileMinuteBars, ResponseAcceptAgreement, ResponseAccountList, ResponseAccountRmsInfo, ResponseAccountRmsUpdates, ResponseAuxilliaryReferenceData, ResponseBracketOrder, ResponseCancelAllOrders, ResponseCancelOrder, ResponseDepthByOrderSnapshot, ResponseDepthByOrderUpdates, ResponseEasyToBorrowList, ResponseExitPosition, ResponseFrontMonthContract, ResponseGetInstrumentByUnderlying, ResponseGetInstrumentByUnderlyingKeys, ResponseGetVolumeAtPrice, ResponseGiveTickSizeTypeTable, ResponseHeartbeat, ResponseLinkOrders, ResponseListAcceptedAgreements, ResponseListExchangePermissions, ResponseListUnacceptedAgreements, ResponseLogin, ResponseLoginInfo, ResponseLogout, ResponseMarketDataUpdate, ResponseMarketDataUpdateByUnderlying, ResponseModifyOrder, ResponseModifyOrderReferenceData, ResponseNewOrder, ResponseOcoOrder, ResponseOrderSessionConfig, ResponsePnLPositionSnapshot, ResponsePnLPositionUpdates, ResponseProductCodes, ResponseProductRmsInfo, ResponseReferenceData, ResponseReplayExecutions, ResponseResumeBars, ResponseRithmicSystemInfo, ResponseSearchSymbols, ResponseSetRithmicMrktDataSelfCertStatus, ResponseShowAgreement, ResponseShowBracketStops, ResponseShowBrackets, ResponseShowOrderHistory, ResponseShowOrderHistoryDates, ResponseShowOrderHistoryDetail, ResponseShowOrderHistorySummary, ResponseShowOrders, ResponseSubscribeForOrderUpdates, ResponseSubscribeToBracketUpdates, ResponseTickBarReplay, ResponseTickBarUpdate, ResponseTimeBarReplay, ResponseTimeBarUpdate, ResponseTradeRoutes, ResponseUpdateStopBracketLevel, ResponseUpdateTargetBracketLevel, ResponseVolumeProfileMinuteBars, RithmicOrderNotification, SymbolMarginRate, TickBar, TimeBar, TradeRoute, TradeStatistics, UpdateEasyToBorrowList};
use ff_rithmic_api::rithmic_proto_objects::rti::{RequestShowOrders, RequestSubscribeForOrderUpdates};
use prost::{Message as ProstMessage};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use tokio::task::JoinHandle;
use tungstenite::{Message};
#[allow(unused_imports)]
use crate::standardized_types::broker_enum::Brokerage;
use crate::server_features::rithmic_api::api_client::RithmicClient;
use crate::strategies::ledgers::{AccountInfo, Currency};

#[allow(dead_code)]
pub async fn handle_responses_from_order_plant(
    client: Arc<RithmicClient>,
)  -> JoinHandle<()> {
    let handle = tokio::task::spawn(async move {
        const PLANT: SysInfraType = SysInfraType::OrderPlant;
        let reader = client.readers.get(&PLANT).unwrap().value().clone();
        let mut reader = reader.lock().await;
        while let Some(message) = reader.next().await {
            match message {
                Ok(message) => {
                    match message {
                        // Tungstenite messages, if you use ProstMessage here you will get a trait related compile time error
                        Message::Text(text) => {
                            println!("{}", text)
                        }
                        Message::Binary(bytes) => {
                            // spawn a new task so that we can handle next message faster.
                            let client = client.clone();
                            tokio::task::spawn(async move {
                                //messages will be forwarded here
                                let mut cursor = Cursor::new(bytes);
                                // Read the 4-byte length header
                                let mut length_buf = [0u8; 4];
                                let _ = tokio::io::AsyncReadExt::read_exact(&mut cursor, &mut length_buf).await.map_err(RithmicApiError::Io);
                                let length = u32::from_be_bytes(length_buf) as usize;
                                //println!("Length: {}", length);

                                // Read the Protobuf message
                                let mut message_buf = vec![0u8; length];

                                match tokio::io::AsyncReadExt::read_exact(&mut cursor, &mut message_buf).await.map_err(RithmicApiError::Io) {
                                    Ok(_) => {}
                                    Err(e) => eprintln!("Failed to read_extract message: {}", e)
                                }
                                if let Some(template_id) = client.client.extract_template_id(&message_buf) {
                                    //println!("Extracted template_id: {}", template_id);
                                    // Now you can use the template_id to determine which type to decode into the concrete types
                                    match template_id {
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
                                                println!("Response Heartbeat (Template ID: 19) from Server: {:?}", msg);
                                            }
                                        },
                                        301 => {
                                            if let Ok(msg) = ResponseLoginInfo::decode(&message_buf[..]) {
                                                // Account List Response
                                                // From Server
                                                println!("Response Login Info (Template ID: 303) from Server: {:?}", msg);
                                            }
                                        }
                                        303 => {
                                            if let Ok(msg) = ResponseAccountList::decode(&message_buf[..]) {
                                                // Account List Response
                                                // From Server
                                                println!("Account List Response (Template ID: 303) from Server: {:?}", msg);
                                            }
                                        },
                                        305 => {
                                            if let Ok(msg) = ResponseAccountRmsInfo::decode(&message_buf[..]) {
                                                //println!("Response Account Rms Info (Template ID: 305) from Server: {:?}", msg);
                                                if let Some(id) = &msg.account_id {
                                                    let mut account_info = AccountInfo {
                                                        account_id: id.to_string(),
                                                        brokerage: client.brokerage.clone(),
                                                        cash_value: Default::default(),
                                                        cash_available: Default::default(),
                                                        currency: Currency::USD,
                                                        cash_used: Default::default(),
                                                        positions: vec![],
                                                        is_hedging: false,
                                                        buy_limit: None,
                                                        sell_limit: None,
                                                        max_orders: None,
                                                        daily_max_loss: None,
                                                        daily_max_loss_reset_time: None,
                                                    };
                                                    if let Some(ref currency) = msg.currency {
                                                        account_info.currency = Currency::from_str(&currency);
                                                    }
                                                    if let Some(ref max_size) = msg.buy_limit {
                                                        account_info.buy_limit = Some(Decimal::from_u32(max_size.clone() as u32).unwrap());
                                                    }
                                                    if let Some(ref max_size) = msg.sell_limit {
                                                        account_info.sell_limit = Some(Decimal::from_u32(max_size.clone() as u32).unwrap());
                                                    }
                                                    if let Some(ref max_orders) = msg.max_order_quantity {
                                                        account_info.max_orders = Some(Decimal::from_u32(max_orders.clone() as u32).unwrap());
                                                    }
                                                    if let Some(ref max_loss) = msg.loss_limit {
                                                        account_info.daily_max_loss = Some(Decimal::from_u32(max_loss.clone() as u32).unwrap());
                                                    }
                                                    client.accounts.insert(id.clone(), account_info);
                                                    let req = RequestPnLPositionSnapshot {
                                                        template_id: 402,
                                                        user_msg: vec![],
                                                        fcm_id: client.fcm_id.clone(),
                                                        ib_id: client.ib_id.clone(),
                                                        account_id: Some(id.clone()),
                                                    };
                                                    match client.send_message(SysInfraType::PnlPlant, req).await {
                                                        Ok(_) => {}
                                                        Err(_) => {}
                                                    }
                                                    let req = RequestPnLPositionUpdates {
                                                        template_id: 400 ,
                                                        user_msg: vec![],
                                                        request: Some(1),
                                                        fcm_id: client.fcm_id.clone(),
                                                        ib_id: client.ib_id.clone(),
                                                        account_id: Some(id.clone()),
                                                    };
                                                    match client.send_message(SysInfraType::PnlPlant, req).await {
                                                        Ok(_) => {}
                                                        Err(_) => {}
                                                    }
                                                    let req = RequestShowOrders {
                                                        template_id: 320,
                                                        user_msg: vec![],
                                                        fcm_id: client.fcm_id.clone(),
                                                        ib_id: client.ib_id.clone(),
                                                        account_id: Some(id.clone()),
                                                    };
                                                    match client.send_message(SysInfraType::OrderPlant, req).await {
                                                        Ok(_) => {}
                                                        Err(_) => {}
                                                    }
                                                    let req = RequestSubscribeForOrderUpdates {
                                                        template_id,
                                                        user_msg: vec![],
                                                        fcm_id: client.fcm_id.clone(),
                                                        ib_id: client.ib_id.clone(),
                                                        account_id: Some(id.clone()),
                                                    };
                                                    match client.send_message(SysInfraType::OrderPlant, req).await {
                                                        Ok(_) => {}
                                                        Err(_) => {}
                                                    }
                                                }
                                            }
                                        },
                                        307 => {
                                            if let Ok(msg) = ResponseProductRmsInfo::decode(&message_buf[..]) {
                                                // Product RMS Info Response
                                                // From Server
                                                println!("Product RMS Info Response (Template ID: 307) from Server: {:?}", msg);
                                            }
                                        },
                                        309 => {
                                            if let Ok(msg) = ResponseSubscribeForOrderUpdates::decode(&message_buf[..]) {
                                                // Subscribe For Order Updates Response
                                                // From Server
                                                println!("Subscribe For Order Updates Response (Template ID: 309) from Server: {:?}", msg);
                                            }
                                        },
                                        311 => {
                                            if let Ok(msg) = ResponseTradeRoutes::decode(&message_buf[..]) {
                                                // Trade Routes Response
                                                // From Server
                                                println!("Trade Routes Response (Template ID: 311) from Server: {:?}", msg);
                                            }
                                        },
                                        313 => {
                                            if let Ok(msg) = ResponseNewOrder::decode(&message_buf[..]) {
                                                // New Order Response
                                                // From Server
                                                println!("New Order Response (Template ID: 313) from Server: {:?}", msg);
                                            }
                                        },
                                        315 => {
                                            if let Ok(msg) = ResponseModifyOrder::decode(&message_buf[..]) {
                                                // Modify Order Response
                                                // From Server
                                                println!("Modify Order Response (Template ID: 315) from Server: {:?}", msg);
                                            }
                                        },
                                        317 => {
                                            if let Ok(msg) = ResponseCancelOrder::decode(&message_buf[..]) {
                                                // Cancel Order Response
                                                // From Server
                                                println!("Cancel Order Response (Template ID: 317) from Server: {:?}", msg);
                                            }
                                        },
                                        319 => {
                                            if let Ok(msg) = ResponseShowOrderHistoryDates::decode(&message_buf[..]) {
                                                // Show Order History Dates Response
                                                // From Server
                                                println!("Show Order History Dates Response (Template ID: 319) from Server: {:?}", msg);
                                            }
                                        },
                                        321 => {
                                            if let Ok(msg) = ResponseShowOrders::decode(&message_buf[..]) {
                                                // Show Orders Response
                                                // From Server
                                                println!("Show Orders Response (Template ID: 321) from Server: {:?}", msg);
                                            }
                                        },
                                        323 => {
                                            if let Ok(msg) = ResponseShowOrderHistory::decode(&message_buf[..]) {
                                                // Show Order History Response
                                                // From Server
                                                println!("Show Order History Response (Template ID: 323) from Server: {:?}", msg);
                                            }
                                        },
                                        325 => {
                                            if let Ok(msg) = ResponseShowOrderHistorySummary::decode(&message_buf[..]) {
                                                // Show Order History Summary Response
                                                // From Server
                                                println!("Show Order History Summary Response (Template ID: 325) from Server: {:?}", msg);
                                            }
                                        },
                                        327 => {
                                            if let Ok(msg) = ResponseShowOrderHistoryDetail::decode(&message_buf[..]) {
                                                // Show Order History Detail Response
                                                // From Server
                                                println!("Show Order History Detail Response (Template ID: 327) from Server: {:?}", msg);
                                            }
                                        },
                                        329 => {
                                            if let Ok(msg) = ResponseOcoOrder::decode(&message_buf[..]) {
                                                // OCO Order Response
                                                // From Server
                                                println!("OCO Order Response (Template ID: 329) from Server: {:?}", msg);
                                            }
                                        },
                                        331 => {
                                            if let Ok(msg) = ResponseBracketOrder::decode(&message_buf[..]) {
                                                // Bracket Order Response
                                                // From Server
                                                println!("Bracket Order Response (Template ID: 331) from Server: {:?}", msg);
                                            }
                                        },
                                        333 => {
                                            if let Ok(msg) = ResponseUpdateTargetBracketLevel::decode(&message_buf[..]) {
                                                // Update Target Bracket Level Response
                                                // From Server
                                                println!("Update Target Bracket Level Response (Template ID: 333) from Server: {:?}", msg);
                                            }
                                        },
                                        335 => {
                                            if let Ok(msg) = ResponseUpdateStopBracketLevel::decode(&message_buf[..]) {
                                                // Update Stop Bracket Level Response
                                                // From Server
                                                println!("Update Stop Bracket Level Response (Template ID: 335) from Server: {:?}", msg);
                                            }
                                        },
                                        337 => {
                                            if let Ok(msg) = ResponseSubscribeToBracketUpdates::decode(&message_buf[..]) {
                                                // Subscribe To Bracket Updates Response
                                                // From Server
                                                println!("Subscribe To Bracket Updates Response (Template ID: 337) from Server: {:?}", msg);
                                            }
                                        },
                                        339 => {
                                            if let Ok(msg) = ResponseShowBrackets::decode(&message_buf[..]) {
                                                // Show Brackets Response
                                                // From Server
                                                println!("Show Brackets Response (Template ID: 339) from Server: {:?}", msg);
                                            }
                                        },
                                        341 => {
                                            if let Ok(msg) = ResponseShowBracketStops::decode(&message_buf[..]) {
                                                // Show Bracket Stops Response
                                                // From Server
                                                println!("Show Bracket Stops Response (Template ID: 341) from Server: {:?}", msg);
                                            }
                                        },
                                        343 => {
                                            if let Ok(msg) = ResponseListExchangePermissions::decode(&message_buf[..]) {
                                                // List Exchange Permissions Response
                                                // From Server
                                                println!("List Exchange Permissions Response (Template ID: 343) from Server: {:?}", msg);
                                            }
                                        },
                                        345 => {
                                            if let Ok(msg) = ResponseLinkOrders::decode(&message_buf[..]) {
                                                // Link Orders Response
                                                // From Server
                                                println!("Link Orders Response (Template ID: 345) from Server: {:?}", msg);
                                            }
                                        },
                                        347 => {
                                            if let Ok(msg) = ResponseCancelAllOrders::decode(&message_buf[..]) {
                                                // Cancel All Orders Response
                                                // From Server
                                                println!("Cancel All Orders Response (Template ID: 347) from Server: {:?}", msg);
                                            }
                                        },
                                        349 => {
                                            if let Ok(msg) = ResponseEasyToBorrowList::decode(&message_buf[..]) {
                                                // Easy To Borrow List Response
                                                // From Server
                                                println!("Easy To Borrow List Response (Template ID: 349) from Server: {:?}", msg);
                                            }
                                        },
                                        350 => {
                                            if let Ok(msg) = TradeRoute::decode(&message_buf[..]) {
                                                // Trade Route
                                                // From Server
                                                println!("Trade Route (Template ID: 350) from Server: {:?}", msg);
                                            }
                                        },
                                        351 => {
                                            if let Ok(msg) = RithmicOrderNotification::decode(&message_buf[..]) {
                                                // Rithmic Order Notification
                                                // From Server
                                                println!("Rithmic Order Notification (Template ID: 351) from Server: {:?}", msg);
                                            }
                                        },
                                        352 => {
                                            if let Ok(msg) = ExchangeOrderNotification::decode(&message_buf[..]) {
                                                // Exchange Order Notification
                                                // From Server
                                                println!("Exchange Order Notification (Template ID: 352) from Server: {:?}", msg);
                                            }
                                        },
                                        353 => {
                                            if let Ok(msg) = BracketUpdates::decode(&message_buf[..]) {
                                                // Bracket Updates
                                                // From Server
                                                println!("Bracket Updates (Template ID: 353) from Server: {:?}", msg);
                                            }
                                        },
                                        354 => {
                                            if let Ok(msg) = AccountListUpdates::decode(&message_buf[..]) {
                                                // Account List Updates
                                                // From Server
                                                println!("Account List Updates (Template ID: 354) from Server: {:?}", msg);
                                            }
                                        },
                                        355 => {
                                            if let Ok(msg) = UpdateEasyToBorrowList::decode(&message_buf[..]) {
                                                // Update Easy To Borrow List
                                                // From Server
                                                println!("Update Easy To Borrow List (Template ID: 355) from Server: {:?}", msg);
                                            }
                                        },
                                        3501 => {
                                            if let Ok(msg) = ResponseModifyOrderReferenceData::decode(&message_buf[..]) {
                                                // Modify Order Reference Data Response
                                                // From Server
                                                println!("Modify Order Reference Data Response (Template ID: 3501) from Server: {:?}", msg);
                                            }
                                        },
                                        3503 => {
                                            if let Ok(msg) = ResponseOrderSessionConfig::decode(&message_buf[..]) {
                                                // Order Session Config Response
                                                // From Server
                                                println!("Order Session Config Response (Template ID: 3503) from Server: {:?}", msg);
                                            }
                                        },
                                        3505 => {
                                            if let Ok(msg) = ResponseExitPosition::decode(&message_buf[..]) {
                                                // Exit Position Response
                                                // From Server
                                                println!("Exit Position Response (Template ID: 3505) from Server: {:?}", msg);
                                            }
                                        },
                                        3507 => {
                                            if let Ok(msg) = ResponseReplayExecutions::decode(&message_buf[..]) {
                                                // Replay Executions Response
                                                // From Server
                                                println!("Replay Executions Response (Template ID: 3507) from Server: {:?}", msg);
                                            }
                                        },
                                        3509 => {
                                            if let Ok(msg) = ResponseAccountRmsUpdates::decode(&message_buf[..]) {
                                                // Account RMS Updates Response
                                                // From Server
                                                println!("Account RMS Updates Response (Template ID: 3509) from Server: {:?}", msg);
                                            }
                                        },
                                        356 => {
                                            if let Ok(msg) = AccountRmsUpdates::decode(&message_buf[..]) {
                                                // Account RMS Updates
                                                // From Server
                                                println!("Account RMS Updates (Template ID: 356) from Server: {:?}", msg);
                                            }
                                        },

                                        _ => println!("No match for template_id: {}", template_id)
                                    }
                                }
                            });
                        }
                        Message::Ping(ping) => {
                            println!("{:?}", ping)
                        }
                        Message::Pong(pong) => {
                            println!("{:?}", pong)
                        }
                        Message::Close(close) => {
                            // receive this message when market is closed.
                            // received: Ok(Close(Some(CloseFrame { code: Normal, reason: "normal closure" })))
                            println!("{:?}", close)
                        }
                        Message::Frame(frame) => {
                            //This message is sent on weekends, you can use this message to schedule a reconnection attempt for market open.
                            /* Example of received market closed message
                                Some(CloseFrame { code: Normal, reason: "normal closure" })
                                Error: ServerErrorDebug("Failed to send RithmicMessage, possible disconnect, try reconnecting to plant TickerPlant: Trying to work with closed connection")
                            */
                            println!("{}", frame)
                        }
                    }
                }
                Err(e) => {
                    eprintln!("failed to receive message: {}", e)
                }
            }
        }
    });
    handle
}
