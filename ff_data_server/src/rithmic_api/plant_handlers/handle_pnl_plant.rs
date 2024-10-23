use std::str::FromStr;
use std::sync::Arc;
use chrono::Utc;
use dashmap::DashMap;
#[allow(unused_imports)]
use ff_rithmic_api::credentials::RithmicCredentials;
#[allow(unused_imports)]
use ff_rithmic_api::rithmic_proto_objects::rti::{AccountListUpdates, AccountPnLPositionUpdate, AccountRmsUpdates, BestBidOffer, BracketUpdates, DepthByOrder, DepthByOrderEndEvent, EndOfDayPrices, ExchangeOrderNotification, FrontMonthContractUpdate, IndicatorPrices, InstrumentPnLPositionUpdate, LastTrade, MarketMode, OpenInterest, OrderBook, OrderPriceLimits, QuoteStatistics, RequestAccountList, RequestAccountRmsInfo, RequestHeartbeat, RequestLoginInfo, RequestMarketDataUpdate, RequestPnLPositionSnapshot, RequestPnLPositionUpdates, RequestProductCodes, RequestProductRmsInfo, RequestReferenceData, RequestTickBarUpdate, RequestTimeBarUpdate, RequestVolumeProfileMinuteBars, ResponseAcceptAgreement, ResponseAccountList, ResponseAccountRmsInfo, ResponseAccountRmsUpdates, ResponseAuxilliaryReferenceData, ResponseBracketOrder, ResponseCancelAllOrders, ResponseCancelOrder, ResponseDepthByOrderSnapshot, ResponseDepthByOrderUpdates, ResponseEasyToBorrowList, ResponseExitPosition, ResponseFrontMonthContract, ResponseGetInstrumentByUnderlying, ResponseGetInstrumentByUnderlyingKeys, ResponseGetVolumeAtPrice, ResponseGiveTickSizeTypeTable, ResponseHeartbeat, ResponseLinkOrders, ResponseListAcceptedAgreements, ResponseListExchangePermissions, ResponseListUnacceptedAgreements, ResponseLogin, ResponseLoginInfo, ResponseLogout, ResponseMarketDataUpdate, ResponseMarketDataUpdateByUnderlying, ResponseModifyOrder, ResponseModifyOrderReferenceData, ResponseNewOrder, ResponseOcoOrder, ResponseOrderSessionConfig, ResponsePnLPositionSnapshot, ResponsePnLPositionUpdates, ResponseProductCodes, ResponseProductRmsInfo, ResponseReferenceData, ResponseReplayExecutions, ResponseResumeBars, ResponseRithmicSystemInfo, ResponseSearchSymbols, ResponseSetRithmicMrktDataSelfCertStatus, ResponseShowAgreement, ResponseShowBracketStops, ResponseShowBrackets, ResponseShowOrderHistory, ResponseShowOrderHistoryDates, ResponseShowOrderHistoryDetail, ResponseShowOrderHistorySummary, ResponseShowOrders, ResponseSubscribeForOrderUpdates, ResponseSubscribeToBracketUpdates, ResponseTickBarReplay, ResponseTickBarUpdate, ResponseTimeBarReplay, ResponseTimeBarUpdate, ResponseTradeRoutes, ResponseUpdateStopBracketLevel, ResponseUpdateTargetBracketLevel, ResponseVolumeProfileMinuteBars, RithmicOrderNotification, SymbolMarginRate, TickBar, TimeBar, TradeRoute, TradeStatistics, UpdateEasyToBorrowList};
use ff_rithmic_api::rithmic_proto_objects::rti::Reject;
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
use lazy_static::lazy_static;
use prost::{Message as ProstMessage};
use rust_decimal::{Decimal};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal_macros::dec;
use ff_standard_lib::messages::data_server_messaging::DataServerResponse;
use ff_standard_lib::standardized_types::accounts::Account;
#[allow(unused_imports)]
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::enums::PositionSide;
use ff_standard_lib::standardized_types::new_types::Volume;
use ff_standard_lib::standardized_types::position::Position;
use ff_standard_lib::standardized_types::subscriptions::{SymbolCode};
use crate::rithmic_api::api_client::RithmicClient;
use crate::rithmic_api::plant_handlers::handler_loop::send_updates;

lazy_static! {
    pub static ref POSITIONS: DashMap<SymbolCode, Position> = DashMap::new();
    pub static ref POSITION_COUNT: DashMap<SymbolCode, u64> = DashMap::new();
}

#[allow(dead_code, unused)]
pub async fn match_pnl_plant_id(
    template_id: i32, message_buf: Vec<u8>,
    client: Arc<RithmicClient>
) {
    const PLANT: SysInfraType = SysInfraType::PnlPlant;
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
        401 => {
            if let Ok(msg) = ResponsePnLPositionUpdates::decode(&message_buf[..]) {
                // PnL Position Updates Response
                // From Server
                println!("PnL Position Updates Response (Template ID: 401) from Server: {:?}", msg);
            }
        },
        403 => {
            if let Ok(msg) = ResponsePnLPositionSnapshot::decode(&message_buf[..]) {
                // PnL Position Snapshot Response
                // From Server
                println!("PnL Position Snapshot Response (Template ID: 403) from Server: {:?}", msg);

            }
        },
        450 => {
            /*
            Instrument PnL Position Update (Template ID: 450) from Server: InstrumentPnLPositionUpdate { template_id: 450, is_snapshot: None,
            fcm_id: Some("TopstepTrader"), ib_id: Some("TopstepTrader"), account_id: Some("S1Sep246906077"), symbol: Some("MNQZ4"),
            exchange: Some("CME"), product_code: Some("MNQ"), instrument_type: Some("Future"), fill_buy_qty: Some(80), fill_sell_qty: Some(83),
            order_buy_qty: Some(0), order_sell_qty: Some(0), buy_qty: Some(-3), sell_qty: Some(3), avg_open_fill_price: Some(20457.41666667),
            day_open_pnl: Some(-11.0), day_closed_pnl: Some(-246.5), day_pnl: Some(-257.5), day_open_pnl_offset: Some(0.0), day_closed_pnl_offset: Some(0.0),
            mtm_security: Some("-257.50"), open_long_options_value: Some("0.00"), open_short_options_value: Some("0.00"), closed_options_value: Some("0.00"),
            option_cash_reserved: Some("0.00"), open_position_pnl: Some("-11.00"), open_position_quantity: Some(3), closed_position_pnl: Some("-246.50"),
            closed_position_quantity: Some(160), net_quantity: Some(-3), ssboe: Some(1729238967), usecs: Some(596000) }
            */
            if let Ok(msg) = InstrumentPnLPositionUpdate::decode(&message_buf[..]) {
                let symbol = match msg.symbol {
                    None => return,
                    Some(ref s) => s
                };

                match msg.is_snapshot {
                    Some(true) => {
                        match msg.closed_position_pnl {
                            None => {},
                            Some(closed_position_pnl) => {
                                match Decimal::from_str(&closed_position_pnl) {
                                    Ok(closed_position_pnl) => {
                                        client.closed_pnl.insert(symbol.clone(), closed_position_pnl);
                                    },
                                    Err(_) => {}
                                }
                            }
                        };
                       return
                    },
                    _ => {}
                }
                println!("Rithmic Pnl Update: {:?}, Pnl: {:?}, Buy Quantity: {:?}, Sell Quantity: {:?}", msg.symbol, msg.open_position_pnl, msg.buy_qty, msg.sell_qty);
                let account_id = match msg.account_id {
                    None => return,
                    Some(id) => id
                };


                let (side, net_quantity) = if let Some(net_quantity) = msg.net_quantity {
                    match net_quantity {
                        n if n > 0 => (Some(PositionSide::Long), net_quantity),
                        n if n < 0 => (Some(PositionSide::Short), net_quantity),
                        _ => (None, net_quantity),
                    }
                } else {
                    (None, 0) // Default case if net_quantity is None
                };

                //if buy and sell quantity = 0 position is closed

                // Update long positions
                if net_quantity > 0 {
                    if let Some(open_position_quantity) = msg.open_position_quantity {
                        println!("Net Quantity is greater than 0, updating long position");
                        client.long_quantity
                            .entry(account_id.clone())
                            .or_insert_with(DashMap::new)
                            .insert(symbol.clone(), Volume::from_i32(open_position_quantity).unwrap());
                    }
                    client.short_quantity
                        .entry(account_id.clone())
                        .or_insert_with(DashMap::new)
                        .remove(symbol);
                } else if net_quantity < 0 {
                    println!("Net Quantity is negative, updating short position");
                    if let Some(open_position_quantity) = msg.open_position_quantity {
                        client.short_quantity
                            .entry(account_id.clone())
                            .or_insert_with(DashMap::new)
                            .insert(symbol.clone(), Volume::from_i32(open_position_quantity).unwrap());
                    }
                    client.long_quantity
                        .entry(account_id.clone())
                        .or_insert_with(DashMap::new)
                        .remove(symbol);
                } else {
                    println!("Net Quantity is 0, removing position");
                    client.long_quantity
                        .entry(account_id.clone())
                        .or_insert_with(DashMap::new)
                        .remove(symbol);
                    client.short_quantity
                        .entry(account_id.clone())
                        .or_insert_with(DashMap::new)
                        .remove(symbol);

                    if let Some((symbol_code, mut position)) = POSITIONS.remove(symbol) {
                        position.quantity_closed += position.quantity_open;
                        position.quantity_open = dec!(0);
                        position.open_pnl = dec!(0);
                        position.is_closed = true;
                        position.close_time = Some(Utc::now().to_string());
                        if let Some(closed_pnl) = client.closed_pnl.get(symbol) {
                            match msg.closed_position_pnl {
                                None => {},
                                Some(closed_position_pnl) => {
                                    match Decimal::from_str(&closed_position_pnl) {
                                        Ok(closed_position_pnl) => {
                                            position.booked_pnl = closed_position_pnl - *closed_pnl;
                                            client.closed_pnl.insert(symbol.clone(), closed_position_pnl);
                                        },
                                        Err(_) => {}
                                    }
                                }
                            };
                        }

                        send_updates(DataServerResponse::LivePositionUpdates {
                            account: Account::new(client.brokerage, account_id.clone()),
                            position
                        }).await;
                        return
                    }

                    if let Some(open_position_quantity) = msg.open_position_quantity {
                        if let (Some(symbol_name), Some(symbol_code), Some(open_pnl), Some(open_quantity), Some(average_price), Some(side)) = (&msg.product_code, &msg.symbol, &msg.open_position_pnl, &msg.open_position_quantity, &msg.avg_open_fill_price, side) {
                            let open_pnl = match Decimal::from_str(open_pnl) {
                                Ok(open_pnl) => open_pnl,
                                Err(_) => return
                            };
                            let average_price = match Decimal::from_f64_retain(*average_price) {
                                Some(average_price) => average_price,
                                None => return
                            };

                            let symbol_info = match client.symbol_info.get(symbol) {
                                None => return,
                                Some(info) => info.value().clone()
                            };

                            let mut count_entry = POSITION_COUNT.entry(symbol.clone()).or_insert(1);
                            *count_entry = count_entry.wrapping_add(1); // Allow overflow back to 0
                            if *count_entry == 0 {
                                *count_entry = 1; // Prevent the count from being 0, reset to 1
                            }

                            let tag = match client.last_tag.get(&account_id) {
                                None => return,
                                Some(tag) => match tag.value().get(symbol) {
                                    None => return,
                                    Some(tag) => tag.clone()
                                }
                            };

                            let position = Position {
                                pnl_currency: symbol_info.pnl_currency.clone(),
                                symbol_name: symbol_name.clone(),
                                symbol_code: symbol_code.clone(),
                                account: Account {
                                    brokerage: client.brokerage,
                                    account_id: account_id.clone(),
                                },
                                side,
                                open_time: Utc::now().to_string(),
                                quantity_open: Volume::from_i32(open_position_quantity).unwrap(),
                                quantity_closed: Default::default(),
                                close_time: None,
                                average_price,
                                open_pnl,
                                booked_pnl: dec!(0),
                                highest_recoded_price: average_price,
                                lowest_recoded_price: average_price,
                                average_exit_price: None,
                                is_closed: false,
                                position_id: client.generate_id(symbol_code, side, count_entry.value().clone(), &account_id),
                                symbol_info,
                                tag,
                            };

                            POSITIONS.insert(symbol.clone(), position.clone());

                            let position_update = DataServerResponse::LivePositionUpdates {
                                account: Account::new(client.brokerage, account_id.clone()),
                                position
                            };
                            send_updates(position_update).await;
                        }
                    }
                }
            }
        },
        451 => {
            if let Ok(msg) = AccountPnLPositionUpdate::decode(&message_buf[..]) {
                // Account PnL Position Update
                // From Server
                //println!("Account PnL Position Update (Template ID: 451) from Server: {:?}", msg);
                /*Account PnL Position Update (Template ID: 451) from Server: AccountPnLPositionUpdate {
                template_id: 451, is_snapshot: Some(true), fcm_id: Some("TopstepTrader"),
                ib_id: Some("TopstepTrader"), account_id: Some("S1Sep246906077"), fill_buy_qty: Some(0),
                fill_sell_qty: Some(0), order_buy_qty: Some(0), order_sell_qty: Some(0), buy_qty: Some(0), sell_qty: Some(0),
                open_long_options_value: Some("0.00"), open_short_options_value: Some("0.00"),
                closed_options_value: Some("0.00"), option_cash_reserved: Some("0.00"), rms_account_commission: None,
                open_position_pnl: Some("0.00"), open_position_quantity: Some(0), closed_position_pnl: Some("0.00"),
                closed_position_quantity: Some(0), net_quantity: Some(0), excess_buy_margin: Some(""), margin_balance: Some(""),
                min_margin_balance: Some(""), min_account_balance: Some("0.00"), account_balance: Some("49992.40"), cash_on_hand: Some("49992.40"),
                option_closed_pnl: Some("0.00"), percent_maximum_allowable_loss: Some(""), option_open_pnl: Some("0.00"), mtm_account: Some("0.00"),
                available_buying_power: Some(""), used_buying_power: Some(""), reserved_buying_power: Some(""), excess_sell_margin: Some(""),
                day_open_pnl: Some("0.00"), day_closed_pnl: Some("0.00"), day_pnl: Some("0.00"), day_open_pnl_offset: Some("0.00"),
                day_closed_pnl_offset: Some("0.00"), ssboe: Some(1729083215), usecs: Some(883000) }*/

                let id = match msg.account_id {
                    None => return,
                    Some(id) => id
                };

                //match msg.

                match msg.account_balance {
                    None => {},
                    Some(account_balance) => {
                        match Decimal::from_str(&account_balance) {
                            Ok(account_balance) =>{
                                client.account_balance.insert(id.clone(), account_balance);
                            },
                            Err(_) => {}
                        }
                    }
                };

                match msg.cash_on_hand {
                    None => {},
                    Some(cash_on_hand) => {
                        match Decimal::from_str(&cash_on_hand) {
                            Ok(cash_on_hand) => {
                                client.account_cash_available.insert(id.clone(), cash_on_hand);
                            },
                            Err(_) => {}
                        }
                    }
                };

                match msg.open_position_pnl {
                    None => {},
                    Some(open_pnl) => {
                        match Decimal::from_str(&open_pnl) {
                            Ok(open_pnl) => {
                                client.open_pnl.insert(id.clone(), open_pnl);
                            },
                            Err(_) => {}
                        }
                    }
                };

                match msg.closed_position_pnl {
                    None => {},
                    Some(closed_position_pnl) => {
                        match Decimal::from_str(&closed_position_pnl) {
                            Ok(closed_position_pnl) => {
                                client.closed_pnl.insert(id.clone(), closed_position_pnl);
                            },
                            Err(_) => {}
                        }
                    }
                };

                match msg.day_closed_pnl {
                    None => {},
                    Some(day_closed_pnl) => {
                        match Decimal::from_str(&day_closed_pnl) {
                            Ok(day_closed_pnl) => {
                                client.day_closed_pnl.insert(id.clone(), day_closed_pnl);
                            },
                            Err(_) => {}
                        }
                    }
                };

                match msg.day_open_pnl {
                    None => {},
                    Some(day_open_pnl) => {
                        match Decimal::from_str(&day_open_pnl) {
                            Ok(day_open_pnl) => {
                                client.day_open_pnl.insert(id.clone(), day_open_pnl);
                            },
                            Err(_) => {}
                        }
                    }
                };
                if let (Some(cash_value), Some(cash_available)) = (
                    client.account_balance.get(&id).map(|r| *r),
                    client.account_cash_available.get(&id).map(|r| *r)
                ) {
                    let cash_used = cash_value - cash_available;

                    // Update the cash_used in the DashMap
                    client.account_cash_used.insert(id.clone(), cash_used);

                    send_updates(DataServerResponse::LiveAccountUpdates {
                        account: Account::new(client.brokerage, id),
                        cash_value,
                        cash_available,
                        cash_used,
                    }).await;
                }
            }
        },
        _ => println!("No match for template_id: {}", template_id)
    }
}
