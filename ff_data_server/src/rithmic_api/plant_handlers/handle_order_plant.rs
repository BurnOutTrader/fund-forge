use std::sync::Arc;
use chrono::{DateTime, Utc};
#[allow(unused_imports)]
use ff_rithmic_api::credentials::RithmicCredentials;
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
#[allow(unused_imports)]
use ff_rithmic_api::rithmic_proto_objects::rti::{AccountListUpdates, AccountPnLPositionUpdate, AccountRmsUpdates, BestBidOffer, BracketUpdates, DepthByOrder, DepthByOrderEndEvent, EndOfDayPrices, ExchangeOrderNotification, FrontMonthContractUpdate, IndicatorPrices, InstrumentPnLPositionUpdate, LastTrade, MarketMode, OpenInterest, OrderBook, OrderPriceLimits, QuoteStatistics, RequestAccountList, RequestAccountRmsInfo, RequestHeartbeat, RequestLoginInfo, RequestMarketDataUpdate, RequestPnLPositionSnapshot, RequestPnLPositionUpdates, RequestProductCodes, RequestProductRmsInfo, RequestReferenceData, RequestTickBarUpdate, RequestTimeBarUpdate, RequestVolumeProfileMinuteBars, ResponseAcceptAgreement, ResponseAccountList, ResponseAccountRmsInfo, ResponseAccountRmsUpdates, ResponseAuxilliaryReferenceData, ResponseBracketOrder, ResponseCancelAllOrders, ResponseCancelOrder, ResponseDepthByOrderSnapshot, ResponseDepthByOrderUpdates, ResponseEasyToBorrowList, ResponseExitPosition, ResponseFrontMonthContract, ResponseGetInstrumentByUnderlying, ResponseGetInstrumentByUnderlyingKeys, ResponseGetVolumeAtPrice, ResponseGiveTickSizeTypeTable, ResponseHeartbeat, ResponseLinkOrders, ResponseListAcceptedAgreements, ResponseListExchangePermissions, ResponseListUnacceptedAgreements, ResponseLogin, ResponseLoginInfo, ResponseLogout, ResponseMarketDataUpdate, ResponseMarketDataUpdateByUnderlying, ResponseModifyOrder, ResponseModifyOrderReferenceData, ResponseNewOrder, ResponseOcoOrder, ResponseOrderSessionConfig, ResponsePnLPositionSnapshot, ResponsePnLPositionUpdates, ResponseProductCodes, ResponseProductRmsInfo, ResponseReferenceData, ResponseReplayExecutions, ResponseResumeBars, ResponseRithmicSystemInfo, ResponseSearchSymbols, ResponseSetRithmicMrktDataSelfCertStatus, ResponseShowAgreement, ResponseShowBracketStops, ResponseShowBrackets, ResponseShowOrderHistory, ResponseShowOrderHistoryDates, ResponseShowOrderHistoryDetail, ResponseShowOrderHistorySummary, ResponseShowOrders, ResponseSubscribeForOrderUpdates, ResponseSubscribeToBracketUpdates, ResponseTickBarReplay, ResponseTickBarUpdate, ResponseTimeBarReplay, ResponseTimeBarUpdate, ResponseTradeRoutes, ResponseUpdateStopBracketLevel, ResponseUpdateTargetBracketLevel, ResponseVolumeProfileMinuteBars, RithmicOrderNotification, SymbolMarginRate, TickBar, TimeBar, TradeRoute, TradeStatistics, UpdateEasyToBorrowList};
use ff_rithmic_api::rithmic_proto_objects::rti::Reject;
use prost::Message as ProstMessage;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use ff_standard_lib::standardized_types::accounts::AccountInfo;
#[allow(unused_imports)]
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::enums::FuturesExchange;
use ff_standard_lib::standardized_types::accounts::Currency;
use crate::rithmic_api::api_client::RithmicClient;


#[allow(unused, dead_code)]
pub async fn match_order_plant_id(
    template_id: i32, message_buf: Vec<u8>,
    client: Arc<RithmicClient>,
) {
    // Helper function to create DateTime<Utc> from ssboe and usecs
    let create_datetime = |ssboe: i64, usecs: i64| -> DateTime<Utc> {
        let nanosecs = usecs * 1000; // Convert microseconds to nanoseconds
        DateTime::from_timestamp(ssboe, nanosecs as u32).unwrap()
    };
    const PLANT: SysInfraType = SysInfraType::OrderPlant;
    match template_id {
        75 => {
            if let Ok(msg) = Reject::decode(&message_buf[..]) {
                // Login Response
                // From Server
                println!("Reject Response (Template ID: 11) from Server: {:?}", msg);
            }
        }
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
                if let Some(id) = msg.account_id {
                    let mut account_info = AccountInfo {
                        account_id: id.to_string(),
                        brokerage: client.brokerage.clone(),
                        cash_value: Default::default(),
                        cash_available: Default::default(),
                        currency: Currency::USD,
                        open_pnl: Default::default(),
                        booked_pnl: Default::default(),
                        day_open_pnl: Default::default(),
                        day_booked_pnl: Default::default(),
                        cash_used: Default::default(),
                        positions: vec![],
                        is_hedging: false,
                        leverage: 0,
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
                    tokio::spawn(async move {
                        client.account_info.insert(id.clone(), account_info);
                        client.request_updates().await;
                    });
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
                //println!("Trade Routes Response (Template ID: 311) from Server: {:?}", msg);

                if let Some(route) = msg.trade_route {
                    if let Some(exchange) = msg.exchange {
                        let exchange = match FuturesExchange::from_string(&exchange) {
                            Ok(exchange) => exchange,
                            Err(_) => return,
                        };
                        client.default_trade_route.insert(exchange, route);
                    }
                }
            }
        },
        313 => {
            if let Ok(msg) = ResponseNewOrder::decode(&message_buf[..]) {
                // New Order Response
                // From Server
                println!("New Order Response (Template ID: 313) from Server: {:?}", msg);
              /*  if let (Some(basket_id), Some(ssboe), Some(usecs)) = (msg.basket_id, msg.ssboe, msg.usecs) {
                    let time = create_datetime(ssboe as i64, usecs as i64).to_string();
                    let order_event = OrderUpdateEvent::OrderAccepted {
                        brokerage: client.brokerage.clone(),
                        account_id: client.account_id.clone(),
                        order_id: OrderId::from(basket_id),
                        tag: msg.user_tag.unwrap_or_default(),
                        time,
                    };
                    // You can handle the order_event here
                }*/
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
            // Rithmic Order Notification
            // From Server
            /*
            Rithmic Order Notification (Template ID: 351) from Server: RithmicOrderNotification { template_id: 351, user_tag: None, notify_type: Some(OrderRcvdFromClnt),
            is_snapshot: None, status: Some("Order received from client"), basket_id: Some("233651480"), original_basket_id: None, linked_basket_ids: None,
            fcm_id: Some("TopstepTrader"), ib_id: Some("TopstepTrader"), user_id: Some("kevtaz"), account_id: Some("S1Sep246906077"), symbol: Some("M6AZ4"),
            exchange: Some("CME"), trade_exchange: Some("CME"), trade_route: Some("simulator"), exchange_order_id: None, instrument_type: None,
            completion_reason: None, quantity: Some(2), quan_release_pending: None, price: None, trigger_price: None, transaction_type: Some(Sell),
            duration: Some(Day), price_type: Some(Market), orig_price_type: Some(Market), manual_or_auto: Some(Manual), bracket_type: None,
            avg_fill_price: None, total_fill_size: None, total_unfilled_size: None, trail_by_ticks: None, trail_by_price_id: None, sequence_number: None,
            orig_sequence_number: None, cor_sequence_number: None, currency: None, country_code: None, text: None, report_text: None, remarks: None,
            window_name: Some("Quote Board, Sell Button, Confirm "), originator_window_name: None, cancel_at_ssboe: None, cancel_at_usecs: None, cancel_after_secs: None,
            ssboe: Some(1729085413), usecs: Some(477767) }
            */
            if let Ok(msg) = RithmicOrderNotification::decode(&message_buf[..]) {
                //println!("Rithmic Order Notification (Template ID: 351) from Server: {:?}", msg);
                /*if let (Some(basket_id), Some(ssboe), Some(usecs), Some(account_id), Some(notify_type), Some(order_id)) =
                    (msg.basket_id, msg.ssboe, msg.usecs, msg.account_id, msg.notify_type, msg.user_tag) {
                    let time = create_datetime(ssboe, usecs).to_string();
                    let order_event = match notify_type {
                        NotifyType::OrderRcvdFromClnt | NotifyType::OrderRcvdByExchGtwy | NotifyType::OrderSentToExch => {
                            OrderUpdateEvent::OrderAccepted {
                                brokerage: client.brokerage.clone(),
                                account_id: AccountId::from(account_id),
                                order_id: OrderId::from(basket_id),
                                tag: msg.user_tag.unwrap_or_default(),
                                time,
                            }
                        },
                        NotifyType::Open => {
                            OrderUpdateEvent::OrderAccepted {
                                brokerage: client.brokerage.clone(),
                                account_id: AccountId::from(account_id),
                                order_id: OrderId::from(basket_id),
                                tag: msg.user_tag.unwrap_or_default(),
                                time,
                            }
                        },
                        NotifyType::Complete => {
                            if msg.completion_reason == Some("F".to_string()) {
                                OrderUpdateEvent::OrderFilled {
                                    brokerage: client.brokerage.clone(),
                                    account_id: AccountId::from(account_id),
                                    order_id: OrderId::from(basket_id),
                                    tag: msg.user_tag.unwrap_or_default(),
                                    time,
                                }
                            } else {
                                OrderUpdateEvent::OrderCancelled {
                                    brokerage: client.brokerage.clone(),
                                    account_id: AccountId::from(account_id),
                                    order_id: OrderId::from(basket_id),
                                    tag: msg.user_tag.unwrap_or_default(),
                                    time,
                                }
                            }
                        },
                        NotifyType::CancelRcvdFromClnt | NotifyType::CancelRcvdByExchGtwy | NotifyType::CancelSentToExch => {
                            OrderUpdateEvent::OrderCancelled {
                                brokerage: client.brokerage.clone(),
                                account_id: AccountId::from(account_id),
                                order_id: OrderId::from(basket_id),
                                tag: msg.user_tag.unwrap_or_default(),
                                time,
                            }
                        },
                        NotifyType::ModifyRcvdFromClnt | NotifyType::ModifyRcvdByExchGtwy | NotifyType::ModifySentToExch | NotifyType::Modified => {
                            // Assuming you have an OrderUpdated event
                            OrderUpdateEvent::OrderUpdated {
                                brokerage: client.brokerage.clone(),
                                account_id: AccountId::from(account_id),
                                order_id: OrderId::from(basket_id),
                                order: msg.into(), // You might need to implement a conversion from RithmicOrderNotification to Order
                                tag: msg.user_tag.unwrap_or_default(),
                                time,
                            }
                        },
                        NotifyType::ModificationFailed | NotifyType::CancellationFailed => {
                            OrderUpdateEvent::OrderUpdateRejected {
                                brokerage: client.brokerage.clone(),
                                account_id: AccountId::from(account_id),
                                order_id: OrderId::from(basket_id),
                                reason: msg.status.unwrap_or_default(),
                                time,
                            }
                        },
                        _ => return,  // Ignore other notification types
                    };
                    // You can handle the order_event here
                }*/
            }
        },

        352 => {
            // Exchange Order Notification
            // From Server
            //println!("Exchange Order Notification (Template ID: 352) from Server: {:?}", msg);
            if let Ok(msg) = ExchangeOrderNotification::decode(&message_buf[..]) {
                println!("Exchange Order Notification (Template ID: 352) from Server: {:?}", msg);
                if let (Some(basket_id), Some(ssboe), Some(usecs), Some(account_id), Some(notify_type)) =
                    (msg.basket_id, msg.ssboe, msg.usecs, msg.account_id, msg.notify_type) {
                    //let time = create_datetime(ssboe, usecs).to_string();
                    /*let order_event = match notify_type {
                        NotifyType::Fill => {
                            if msg.total_unfilled_size == Some(0) {
                                OrderUpdateEvent::OrderFilled {
                                    brokerage: client.brokerage.clone(),
                                    account_id: AccountId(account_id),
                                    order_id: OrderId(basket_id),
                                    tag: msg.user_tag.unwrap_or_default(),
                                    time,
                                }
                            } else {
                                OrderUpdateEvent::OrderPartiallyFilled {
                                    brokerage: client.brokerage.clone(),
                                    account_id: AccountId(account_id),
                                    order_id: OrderId(basket_id),
                                    tag: msg.user_tag.unwrap_or_default(),
                                    time,
                                }
                            }
                        },
                        _ => return,  // Ignore other notification types
                    };*/
                    // You can handle the order_event here
                }
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
