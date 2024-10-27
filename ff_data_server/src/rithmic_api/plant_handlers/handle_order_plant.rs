use std::str::FromStr;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
#[allow(unused_imports)]
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::{AccountListUpdates, AccountPnLPositionUpdate, AccountRmsUpdates, BestBidOffer, BracketUpdates, DepthByOrder, DepthByOrderEndEvent, EndOfDayPrices, ExchangeOrderNotification, FrontMonthContractUpdate, IndicatorPrices, InstrumentPnLPositionUpdate, LastTrade, MarketMode, OpenInterest, OrderBook, OrderPriceLimits, QuoteStatistics, RequestAccountList, RequestAccountRmsInfo, RequestHeartbeat, RequestLoginInfo, RequestMarketDataUpdate, RequestPnLPositionSnapshot, RequestPnLPositionUpdates, RequestProductCodes, RequestProductRmsInfo, RequestReferenceData, RequestTickBarUpdate, RequestTimeBarUpdate, RequestVolumeProfileMinuteBars, ResponseAcceptAgreement, ResponseAccountList, ResponseAccountRmsInfo, ResponseAccountRmsUpdates, ResponseAuxilliaryReferenceData, ResponseBracketOrder, ResponseCancelAllOrders, ResponseCancelOrder, ResponseDepthByOrderSnapshot, ResponseDepthByOrderUpdates, ResponseEasyToBorrowList, ResponseExitPosition, ResponseFrontMonthContract, ResponseGetInstrumentByUnderlying, ResponseGetInstrumentByUnderlyingKeys, ResponseGetVolumeAtPrice, ResponseGiveTickSizeTypeTable, ResponseHeartbeat, ResponseLinkOrders, ResponseListAcceptedAgreements, ResponseListExchangePermissions, ResponseListUnacceptedAgreements, ResponseLogin, ResponseLoginInfo, ResponseLogout, ResponseMarketDataUpdate, ResponseMarketDataUpdateByUnderlying, ResponseModifyOrder, ResponseModifyOrderReferenceData, ResponseNewOrder, ResponseOcoOrder, ResponseOrderSessionConfig, ResponsePnLPositionSnapshot, ResponsePnLPositionUpdates, ResponseProductCodes, ResponseProductRmsInfo, ResponseReferenceData, ResponseReplayExecutions, ResponseResumeBars, ResponseRithmicSystemInfo, ResponseSearchSymbols, ResponseSetRithmicMrktDataSelfCertStatus, ResponseShowAgreement, ResponseShowBracketStops, ResponseShowBrackets, ResponseShowOrderHistory, ResponseShowOrderHistoryDates, ResponseShowOrderHistoryDetail, ResponseShowOrderHistorySummary, ResponseShowOrders, ResponseSubscribeForOrderUpdates, ResponseSubscribeToBracketUpdates, ResponseTickBarReplay, ResponseTickBarUpdate, ResponseTimeBarReplay, ResponseTimeBarUpdate, ResponseTradeRoutes, ResponseUpdateStopBracketLevel, ResponseUpdateTargetBracketLevel, ResponseVolumeProfileMinuteBars, RithmicOrderNotification, SymbolMarginRate, TickBar, TimeBar, TradeRoute, TradeStatistics, UpdateEasyToBorrowList};
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::Reject;
use lazy_static::lazy_static;
use prost::Message as ProstMessage;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use ff_standard_lib::messages::data_server_messaging::DataServerResponse;
use ff_standard_lib::standardized_types::accounts::{Account, AccountInfo};
#[allow(unused_imports)]
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::enums::{FuturesExchange, OrderSide};
use ff_standard_lib::standardized_types::accounts::Currency;
use ff_standard_lib::standardized_types::new_types::{Price, Volume};
use ff_standard_lib::standardized_types::orders::{OrderId, OrderUpdateEvent, OrderUpdateType};
use ff_standard_lib::StreamName;
use crate::request_handlers::RESPONSE_SENDERS;
use crate::rithmic_api::api_client::RithmicClient;
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_login::SysInfraType;
use crate::rithmic_api::plant_handlers::create_datetime;
use crate::rithmic_api::plant_handlers::handler_loop::send_updates;
use crate::rithmic_api::products::find_base_symbol;

type BasketId = String;
lazy_static! {
    pub static ref BASKET_ID_TO_ID_MAP: DashMap<Brokerage, DashMap<BasketId , OrderId>> = DashMap::new();
    pub static ref BASKET_TO_STREAM_NAME_MAP: DashMap<Brokerage, DashMap<BasketId , StreamName>> = DashMap::new();
    pub static ref ID_TO_STREAM_NAME_MAP: DashMap<Brokerage, DashMap<OrderId , u16>> = DashMap::new();
    pub static ref ID_TO_TAG: DashMap<Brokerage, DashMap<OrderId , String>> = DashMap::new();
    pub static ref ID_UPDATE_TYPE: DashMap<Brokerage, DashMap<OrderId , OrderUpdateType>> = DashMap::new();
}

#[allow(unused, dead_code)]
pub async fn match_order_plant_id(
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
                    client.account_info.insert(id.clone(), account_info);
                    client.request_updates(id.to_string()).await;
                    println!("Account Info Added: {}", id);
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
               //println!("New Order Response (Template ID: 313) from Server: {:?}", msg);
                if let (Some(basket_id), Some(ssboe), Some(usecs)) = (msg.basket_id, msg.ssboe, msg.usecs) {
                    let time = create_datetime(ssboe as i64, usecs as i64).to_string();

                    let order_id = match msg.user_tag {
                        None => return,
                        Some(order_id) => order_id
                    };
                    BASKET_ID_TO_ID_MAP.entry(client.brokerage.clone()).or_insert(DashMap::new()).insert(basket_id.clone(), order_id.clone());

                    let stream_name = match msg.user_msg.get(0) {
                        None => return,
                        Some(stream_name) => stream_name
                    };

                    let stream_name = u16::from_str(&stream_name).unwrap_or_default();

                    BASKET_TO_STREAM_NAME_MAP.entry(client.brokerage.clone()).or_insert(DashMap::new()).insert(basket_id,stream_name );
                    ID_TO_STREAM_NAME_MAP.entry(client.brokerage.clone()).or_insert(DashMap::new()).insert(order_id.clone(), stream_name);

                    let tag = match msg.user_msg.get(2) {
                        None => return,
                        Some(tag) => tag
                    };
                    ID_TO_TAG.entry(client.brokerage.clone()).or_insert(DashMap::new()).insert(order_id.clone(), tag.clone());

                    let account_id = match msg.user_msg.get(1) {
                        None => return,
                        Some(id) => id
                    };

                    let symbol_name = match msg.user_msg.get(3) {
                        None => return,
                        Some(name) => name.clone()
                    };

                    let symbol_code = match msg.user_msg.get(4) {
                        None => return,
                        Some(symbol_code) => symbol_code.clone()
                    };
                    // on sim accounts we don't need to do this, not sure about live.

                    /*let event = OrderUpdateEvent::OrderAccepted {
                        account: Account{brokerage: client.brokerage, account_id: account_id.clone()},
                        symbol_name,
                        symbol_code,
                        order_id: order_id.clone(),
                        tag: tag.to_owned(),
                        time,
                    };

                    send_order_update(client.brokerage, &order_id, event).await;*/
                }
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
                //todo I think these are only for rithmic web or r trader orders
                //println!("Rithmic Order Notification (Template ID: 351) from Server: {:?}", msg);
               /* if let (Some(basket_id), Some(ssboe), Some(usecs), Some(account_id), Some(notify_type), Some(order_id)) =
                    (msg.basket_id, msg.ssboe, msg.usecs, msg.account_id, msg.notify_type, msg.user_tag) {
                    let time = create_datetime(ssboe as i64, usecs as i64).to_string();
                    let notify_type = match NotifyType::try_from(notify_type) {
                        Err(e) => return,
                        Ok(notify) => notify
                    };

                    let tag = match ID_TO_TAG.get(&order_id) {
                        None => {
                            eprintln!("Tag not found for order: {}", order_id);
                            return;
                        },
                        Some(tag) => tag.value().clone()
                    };
                    match notify_type {
                        NotifyType::Open => {
                            //todo, we dont need to do this here
                        /*    let event = OrderUpdateEvent::OrderAccepted {
                                brokerage: client.brokerage.clone(),
                                account_id: AccountId::from(account_id),
                                order_id: order_id.clone(),
                                tag,
                                time,
                            };
                            send_order_update(&order_id, event).await;*/
                        },
                        NotifyType::Complete => {
                            if msg.completion_reason == Some("Fill".to_string()) {
                                if let (Some(price), Some(quantity)) = (msg.price, msg.quantity) {
                                    let price = match Decimal::from_f64_retain(price) {
                                        None => return,
                                        Some(p) => p
                                    };
                                    let quantity = match Decimal::from_i32(quantity) {
                                        None => return,
                                        Some(q) => q
                                    };
                                    let event = OrderUpdateEvent::OrderFilled {
                                        brokerage: client.brokerage.clone(),
                                        account_id: AccountId::from(account_id),
                                        order_id: order_id.clone(),
                                        price,
                                        quantity,
                                        tag,
                                        time,
                                    };
                                    send_order_update(&order_id, event).await;
                                }
                            }
                        },
                        NotifyType::Modified => {
                            // Assuming you have an OrderUpdated event
                            if let Some((order_id, update_type)) = ID_UPDATE_TYPE.remove(&order_id) {
                                let event = OrderUpdateEvent::OrderUpdated {
                                    brokerage: client.brokerage.clone(),
                                    account_id: AccountId::from(account_id),
                                    update_type,
                                    order_id: order_id.clone(),
                                    tag,
                                    time,
                                };
                                send_order_update(&order_id, event).await;
                            }
                        },
                        NotifyType::ModificationFailed | NotifyType::CancellationFailed => {
                            let event = OrderUpdateEvent::OrderUpdateRejected {
                                brokerage: client.brokerage.clone(),
                                account_id: AccountId::from(account_id),
                                order_id: order_id.clone(),
                                reason: msg.status.unwrap_or_default(),
                                time,
                            };
                            send_order_update(&order_id, event).await;
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
            /*
            Exchange Order Notification (Template ID: 352) from Server: ExchangeOrderNotification {
            template_id: 352, user_tag: Some("Sell Market: Rithmic:S1Sep246906077, MNQ, 4"), notify_type: Some(Fill),
            is_snapshot: Some(true), is_rithmic_internal_msg: None, report_type: Some("fill"), status: Some("complete"),
            basket_id: Some("234556404"), original_basket_id: None, linked_basket_ids: None, fcm_id: Some("TopstepTrader"),
            ib_id: Some("TopstepTrader"), user_id: Some("kevtaz"), account_id: Some("S1Sep246906077"), symbol: Some("MNQZ4"),
            exchange: Some("CME"), trade_exchange: Some("CME"), trade_route: Some("simulator"), exchange_order_id: Some("234556404"),
            tp_exchange_order_id: None, instrument_type: Some("Future"), quantity: Some(1), price: None, trigger_price: None,
            transaction_type: Some(Sell), duration: Some(Fok), price_type: Some(Market), orig_price_type: Some(Market), manual_or_auto: Some(Auto),
            bracket_type: None, confirmed_size: Some(1), confirmed_time: Some("02:34:50"), confirmed_date: Some("20241018"), confirmed_id: Some("1606359"),
            modified_size: None, modified_time: None, modified_date: None, modify_id: None, cancelled_size: Some(0), cancelled_time: None, cancelled_date: None,
            cancelled_id: None, fill_price: Some(20386.25), fill_size: Some(1), fill_time: Some("02:34:50"), fill_date: Some("20241018"), fill_id: Some("1606360"),
            avg_fill_price: Some(20386.25), total_fill_size: Some(1), total_unfilled_size: Some(0), trigger_id: None, trail_by_ticks: None, trail_by_price_id: None,
            sequence_number: Some("Z1XH2"), orig_sequence_number: Some("Z1XH2"), cor_sequence_number: Some("Z1XH2"), currency: Some("USD"), country_code: None, text: None,
            report_text: None, remarks: None, window_name: Some(""), originator_window_name: None, cancel_at_ssboe: None, cancel_at_usecs: None, cancel_after_secs: None,
            ssboe: Some(1729218890), usecs: Some(913270), exch_receipt_ssboe: None, exch_receipt_nsecs: None }
            */
            if let Ok(msg) = ExchangeOrderNotification::decode(&message_buf[..]) {
                //println!("Exchange Order Notification (Template ID: 352) from Server: {:?}", msg);
                if let (Some(basket_id), Some(ssboe), Some(usecs), Some(account_id), Some(notify_type), Some(user_tag)) =
                    (msg.basket_id, msg.ssboe, msg.usecs, msg.account_id, msg.notify_type, msg.user_tag) {
                    let time = create_datetime(ssboe as i64, usecs as i64).to_string();

                    //todo[Rithmic Api] Not sure if i should do this or not
                    match msg.is_snapshot {
                        None => {}
                        Some(some) => {
                            match some {
                                true => return,
                                false => {}
                            }
                        }
                    }
                    let order_id = if let Some(brokerage_map) = BASKET_ID_TO_ID_MAP.get(&client.brokerage) {
                        match brokerage_map.get(&basket_id) {
                            Some(id) => id.value().clone(),
                            None => {
                                //eprintln!("Order ID not found for basket: {}", basket_id);
                                return;
                            },
                        }
                    } else {
                        //eprintln!("Brokerage map not found for client: {:?}", client.brokerage);
                        return;
                    };

                    let tag = if let Some(brokerage_map) = ID_TO_TAG.get(&client.brokerage) {
                        match brokerage_map.value().get(&order_id) {
                            Some(tag) => tag.clone(),
                            None => {
                                //eprintln!("Tag not found for order: {}", order_id);
                                return;
                            },
                        }
                    } else {
                        //eprintln!("Brokerage map not found for client: {:?}", client.brokerage);
                        return;
                    };

                    let (symbol_name, symbol_code) = match msg.symbol {
                        None => return,
                        Some(code) => {
                            let symbol_name = match find_base_symbol(code.clone()) {
                                None => return,
                                Some(symbol_name) => symbol_name
                            };
                            (symbol_name, code)
                        }
                    };

                    let reason = msg.text
                        .or(msg.remarks)
                        .unwrap_or_else(|| "Cancelled".to_string());

                    match notify_type {
                        1 => {
                            let event = OrderUpdateEvent::OrderAccepted {
                                account: Account::new(client.brokerage, account_id.clone()),
                                symbol_name,
                                symbol_code,
                                order_id: order_id.clone(),
                                tag,
                                time: time.clone(),
                            };
                            send_order_update(client.brokerage, &order_id, event, time).await;
                        },
                        5 => {
                            if let (Some(fill_price), Some(fill_size), Some(total_unfilled_size)) =
                                (msg.fill_price, msg.fill_size, msg.total_unfilled_size) {
                                let price = match Price::from_f64_retain(fill_price) {
                                    Some(p) => p,
                                    None => return,
                                };
                                let fill_quantity = match Volume::from_f64_retain(fill_size as f64) {
                                    Some(q) => q,
                                    None => return,
                                };

                                let side = match msg.transaction_type {
                                    None => {
                                        eprintln!("NO SIDE ON TRANSACTION");
                                        return;
                                    },
                                    Some(tt) => {
                                        match tt {
                                            1 => OrderSide::Buy,
                                            2 | 3 => OrderSide::Sell,
                                            _ => return
                                        }
                                    }
                                };
                                if total_unfilled_size == 0 {
                                    let event = OrderUpdateEvent::OrderFilled {
                                        side,
                                        account: Account::new(client.brokerage, account_id.clone()),
                                        symbol_name,
                                        symbol_code,
                                        order_id: order_id.clone(),
                                        price,
                                        quantity: fill_quantity,
                                        tag,
                                        time: time.clone(),
                                    };
                                    send_order_update(client.brokerage, &order_id, event, time).await;
                                } else if total_unfilled_size > 0 {
                                    let event = OrderUpdateEvent::OrderPartiallyFilled {
                                        side,
                                        account: Account::new(client.brokerage, account_id.clone()),
                                        symbol_name,
                                        symbol_code,
                                        order_id: order_id.clone(),
                                        price,
                                        quantity: fill_quantity,
                                        tag,
                                        time: time.clone(),
                                    };
                                    send_order_update(client.brokerage, &order_id, event, time).await;
                                }
                            } else {
                                return;
                            }
                        },
                        3 => {
                            let event = OrderUpdateEvent::OrderCancelled {
                                account: Account::new(client.brokerage, account_id.clone()),
                                order_id: order_id.clone(),
                                symbol_name,
                                symbol_code,
                                tag,
                                time: time.clone(),
                                reason,
                            };
                            send_order_update(client.brokerage, &order_id, event, time).await;
                        },
                        6 => {
                            let event = OrderUpdateEvent::OrderRejected {
                                account: Account::new(client.brokerage, account_id.clone()),
                                order_id: order_id.clone(),
                                reason,
                                symbol_name,
                                symbol_code,
                                tag,
                                time: time.clone(),
                            };
                            send_order_update(client.brokerage, &order_id, event, time).await;
                        },
                       2 => {
                           if let Some(broker_map) = ID_UPDATE_TYPE.get(&client.brokerage) {
                               if let Some((_, update_type)) = broker_map.remove(&order_id) {
                                   let event = OrderUpdateEvent::OrderUpdated {
                                       account: Account::new(client.brokerage, account_id.clone()),
                                       order_id: order_id.clone(),
                                       update_type,
                                       symbol_name,
                                       symbol_code,
                                       tag,
                                       time: time.clone(),
                                       text: "User Request".to_string(),
                                   };
                                   send_order_update(client.brokerage, &order_id, event, time).await;
                               } else {
                                   return;
                               }
                           }
                        },
                        7 | 8 => {
                            let event =OrderUpdateEvent::OrderUpdateRejected {
                                account: Account::new(client.brokerage, account_id.clone()),
                                order_id: order_id.clone(),
                                reason: msg.status.unwrap_or_default(),
                                time: time.clone(),
                            };

                            send_order_update(client.brokerage, &order_id, event, time).await;
                        },
                        _ => return,  // Ignore other notification types
                    };

                    if let (Some(cash_value), Some(cash_available)) = (
                        client.account_balance.get(&account_id).map(|r| *r),
                        client.account_cash_available.get(&account_id).map(|r| *r)
                    ) {
                        let cash_used = cash_value - cash_available;

                        // Update the cash_used in the DashMap
                        client.account_cash_used.insert(account_id.clone(), cash_used);

                        send_updates(DataServerResponse::LiveAccountUpdates {
                            account: Account::new(client.brokerage, account_id),
                            cash_value,
                            cash_available,
                            cash_used,
                        }).await;
                    }
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

async fn send_order_update(brokerage: Brokerage, order_id: &OrderId, event: OrderUpdateEvent, time: String) {
    if let Some(broker_map) = ID_TO_STREAM_NAME_MAP.get(&brokerage) {
        if let Some(stream_name) = broker_map.value().get(order_id) {
            let order_event = DataServerResponse::OrderUpdates{event, time};
            if let Some(sender) = RESPONSE_SENDERS.get(&stream_name.value()) {
                match sender.send(order_event).await {
                    Ok(_) => {}
                    Err(e) => eprintln!("failed to forward ResponseNewOrder 313 to strategy stream {}", e)
                }
            }
        }
    }
}

