use std::str::FromStr;
use std::sync::Arc;
use chrono::{TimeZone, Utc};
use dashmap::DashMap;
use indicatif::{ProgressBar, ProgressStyle};
#[allow(unused_imports)]
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::{AccountListUpdates, AccountPnLPositionUpdate, AccountRmsUpdates, BestBidOffer, BracketUpdates, DepthByOrder, DepthByOrderEndEvent, EndOfDayPrices, ExchangeOrderNotification, FrontMonthContractUpdate, IndicatorPrices, InstrumentPnLPositionUpdate, LastTrade, MarketMode, OpenInterest, OrderBook, OrderPriceLimits, QuoteStatistics, RequestAccountList, RequestAccountRmsInfo, RequestHeartbeat, RequestLoginInfo, RequestMarketDataUpdate, RequestPnLPositionSnapshot, RequestPnLPositionUpdates, RequestProductCodes, RequestProductRmsInfo, RequestReferenceData, RequestTickBarUpdate, RequestTimeBarUpdate, RequestVolumeProfileMinuteBars, ResponseAcceptAgreement, ResponseAccountList, ResponseAccountRmsInfo, ResponseAccountRmsUpdates, ResponseAuxilliaryReferenceData, ResponseBracketOrder, ResponseCancelAllOrders, ResponseCancelOrder, ResponseDepthByOrderSnapshot, ResponseDepthByOrderUpdates, ResponseEasyToBorrowList, ResponseExitPosition, ResponseFrontMonthContract, ResponseGetInstrumentByUnderlying, ResponseGetInstrumentByUnderlyingKeys, ResponseGetVolumeAtPrice, ResponseGiveTickSizeTypeTable, ResponseHeartbeat, ResponseLinkOrders, ResponseListAcceptedAgreements, ResponseListExchangePermissions, ResponseListUnacceptedAgreements, ResponseLogin, ResponseLoginInfo, ResponseLogout, ResponseMarketDataUpdate, ResponseMarketDataUpdateByUnderlying, ResponseModifyOrder, ResponseModifyOrderReferenceData, ResponseNewOrder, ResponseOcoOrder, ResponseOrderSessionConfig, ResponsePnLPositionSnapshot, ResponsePnLPositionUpdates, ResponseProductCodes, ResponseProductRmsInfo, ResponseReferenceData, ResponseReplayExecutions, ResponseResumeBars, ResponseRithmicSystemInfo, ResponseSearchSymbols, ResponseSetRithmicMrktDataSelfCertStatus, ResponseShowAgreement, ResponseShowBracketStops, ResponseShowBrackets, ResponseShowOrderHistory, ResponseShowOrderHistoryDates, ResponseShowOrderHistoryDetail, ResponseShowOrderHistorySummary, ResponseShowOrders, ResponseSubscribeForOrderUpdates, ResponseSubscribeToBracketUpdates, ResponseTickBarReplay, ResponseTickBarUpdate, ResponseTimeBarReplay, ResponseTimeBarUpdate, ResponseTradeRoutes, ResponseUpdateStopBracketLevel, ResponseUpdateTargetBracketLevel, ResponseVolumeProfileMinuteBars, RithmicOrderNotification, SymbolMarginRate, TickBar, TimeBar, TradeRoute, TradeStatistics, UpdateEasyToBorrowList};
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::Reject;
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_login::SysInfraType;
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
use ff_standard_lib::standardized_types::subscriptions::{SymbolCode};
use crate::rithmic_api::api_client::RithmicBrokerageClient;
use crate::rithmic_api::plant_handlers::create_datetime;
use crate::rithmic_api::plant_handlers::handler_loop::send_updates;
use crate::server_features::database::hybrid_storage::MULTIBAR;

lazy_static! {
    pub static ref PROGRESS_PNL: DashMap<(Brokerage, Account, SymbolCode), ProgressBar> = DashMap::new();
}

#[allow(dead_code, unused)]
pub async fn match_pnl_plant_id(
    template_id: i32, message_buf: Vec<u8>,
    client: Arc<RithmicBrokerageClient>
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
        401 => {
            if let Ok(msg) = ResponsePnLPositionUpdates::decode(&message_buf[..]) {
                // PnL Position Updates Response
                // From Server
                //println!("PnL Position Updates Response (Template ID: 401) from Server: {:?}", msg);
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

            // This is the worst designed message in the world, dont mess with this function if it works, just accept that you have to right shit code for a shit message and move on.
            if let Ok(msg) = InstrumentPnLPositionUpdate::decode(&message_buf[..]) {
               // println!("{:?}", msg);
                let symbol_code = match msg.symbol {
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
                                        client.closed_pnl.insert(symbol_code.clone(), closed_position_pnl);
                                    },
                                    Err(_) => {}
                                }
                            }
                        };
                       return
                    },
                    _ => {}
                }

                let ssboe = match msg.ssboe {
                    None => return,
                    Some(ssboe) => ssboe
                };

                let usecs = match msg.usecs {
                    None => return,
                    Some(usecs) => usecs
                };

                let time = create_datetime(ssboe as i64, usecs as i64).to_string();
                let account_id = match msg.account_id {
                    None => return,
                    Some(id) => id
                };

                //this line of code is critical for exit long and exit short to work
                let (side, quantity) = update_position(account_id.clone(), &symbol_code, msg.buy_qty, msg.sell_qty, &client);
                let key = (client.brokerage, Account::new(client.brokerage, account_id.clone()), symbol_code.clone());

                if let Some(symbol_name) = msg.product_code {
                    // fwd position updates to the client
                    if let (Some(pnl), Some(average_price)) = (msg.open_position_pnl.clone(), msg.avg_open_fill_price) {
                        //todo do this with a simple message, quantity open, and position side, symbol name, symbol code
                        let open_position_pnl = match f64::from_str(&pnl) {
                            Ok(open_position_pnl) => open_position_pnl,
                            Err(_) => return
                        };

                        let open_position_quantity = match msg.open_position_quantity {
                            None => return,
                            Some(open_position_quantity) => match f64::from_str(&open_position_quantity.to_string()) {
                                Ok(open_position_quantity) => open_position_quantity,
                                Err(_) => return
                            }
                        };

                        let position_update = DataServerResponse::LivePositionUpdates {
                            symbol_name: symbol_name.clone(),
                            symbol_code: symbol_code.clone(),
                            average_price,
                            account: Account::new(client.brokerage, account_id.clone()),
                            open_quantity: open_position_quantity,
                            side,
                            time,
                            open_pnl: open_position_pnl,
                        };
                        send_updates(position_update).await;
                    } else if side == PositionSide::Flat {
                        let position_update = DataServerResponse::LivePositionUpdates {
                            symbol_name: symbol_name.clone(),
                            symbol_code: symbol_code.clone(),
                            average_price: 0.0,
                            account: Account::new(client.brokerage, account_id.clone()),
                            open_quantity: 0.0,
                            side,
                            time,
                            open_pnl: 0.0,
                        };
                        send_updates(position_update).await;
                    }
                }

                // Update the progress pnl bar
                if let Some(buy_qty) = msg.buy_qty {
                    if side == PositionSide::Long {
                        if !PROGRESS_PNL.contains_key(&key) && buy_qty > 0 {
                            let message_bar = MULTIBAR.add(ProgressBar::new_spinner());
                            let prefix = format!("{} {}: {}", symbol_code, client.brokerage, account_id);
                            let bright_green_prefix = format!("\x1b[92m{}\x1b[0m", prefix);
                            // Set the colored prefix
                            message_bar.set_prefix(bright_green_prefix);
                            message_bar.set_style(
                                ProgressStyle::default_spinner()
                                    .template("{spinner:.green} {prefix} {msg}")
                                    .expect("Failed to set style"),
                            );
                            PROGRESS_PNL.insert(key.clone(), message_bar);
                        }
                    }
                }
                if let Some(sell_qty) = msg.sell_qty {
                    if side == PositionSide::Short {
                        let key = (client.brokerage, Account::new(client.brokerage, account_id.clone()), symbol_code.clone());
                        if !PROGRESS_PNL.contains_key(&key) && sell_qty > 0 {
                            let message_bar = MULTIBAR.add(ProgressBar::new_spinner());
                            let prefix = format!("{} {}: {}", symbol_code, client.brokerage, account_id);
                            let bright_green_prefix = format!("\x1b[92m{}\x1b[0m", prefix);
                            // Set the colored prefix
                            message_bar.set_prefix(bright_green_prefix);
                            message_bar.set_style(
                                ProgressStyle::default_spinner()
                                    .template("{spinner:.green} {prefix} {msg}")
                                    .expect("Failed to set style"),
                            );
                            PROGRESS_PNL.insert(key.clone(), message_bar);
                        }
                    }
                }

                if side != PositionSide::Flat {
                    if let Some(message_bar) = PROGRESS_PNL.get(&key) {
                        let msg = format!("Pnl: {:?}, Buy Quantity: {:?}, Sell Quantity: {:?}", msg.open_position_pnl, msg.buy_qty, msg.sell_qty);
                        message_bar.set_message(msg);
                    }
                } else if side == PositionSide::Flat {
                    if let Some((key, pb)) = PROGRESS_PNL.remove(&key) {
                        pb.finish_and_clear();
                    }
                }
                //println!("PNL Update Message: {:?}", msg);
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

                if let (Some(cash_value), Some(cash_available)) = (
                    client.account_balance.get(&id).map(|r| *r),
                    client.account_cash_available.get(&id).map(|r| *r)
                ) {
                    let cash_used = dec!(0.0); //cash_available - cash_value ;
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

#[allow(dead_code, unused)]
fn update_position(
    account_id: String,
    symbol_code: &SymbolCode,
    msg_buy_qty: Option<i32>,
    msg_sell_qty: Option<i32>,
    client: &Arc<RithmicBrokerageClient>
) -> (PositionSide, i32) {
    match (msg_buy_qty, msg_sell_qty) {
        (Some(buy_quantity), None) if buy_quantity > 0 => {
            client.long_quantity
                .entry(account_id.clone())
                .or_insert_with(DashMap::new)
                .insert(symbol_code.clone(), Volume::from_i32(buy_quantity).unwrap());
            client.short_quantity
                .entry(account_id.clone())
                .or_insert_with(DashMap::new)
                .remove(symbol_code);
            (PositionSide::Long, buy_quantity)
        }
        (None, Some(sell_qty)) if sell_qty > 0 => {
            client.short_quantity
                .entry(account_id.clone())
                .or_insert_with(DashMap::new)
                .insert(symbol_code.clone(), Volume::from_i32(sell_qty).unwrap());
            client.long_quantity
                .entry(account_id.clone())
                .or_insert_with(DashMap::new)
                .remove(symbol_code);
            (PositionSide::Short, sell_qty)
        }
        (Some(buy_quantity), Some(sell_qty)) => {
            if buy_quantity > 0 && sell_qty <= 0 {
                client.long_quantity
                    .entry(account_id.clone())
                    .or_insert_with(DashMap::new)
                    .insert(symbol_code.clone(), Volume::from_i32(buy_quantity).unwrap());
                client.short_quantity
                    .entry(account_id.clone())
                    .or_insert_with(DashMap::new)
                    .remove(symbol_code);
                (PositionSide::Long, buy_quantity)
            } else if buy_quantity <= 0 && sell_qty > 0 {
                client.short_quantity
                    .entry(account_id.clone())
                    .or_insert_with(DashMap::new)
                    .insert(symbol_code.clone(), Volume::from_i32(sell_qty.abs()).unwrap());
                client.long_quantity
                    .entry(account_id.clone())
                    .or_insert_with(DashMap::new)
                    .remove(symbol_code);
                (PositionSide::Short, sell_qty)
            } else {
                // Clear positions if neither buy nor sell is active
                client.long_quantity
                    .entry(account_id.clone())
                    .or_insert_with(DashMap::new)
                    .remove(symbol_code);
                client.short_quantity
                    .entry(account_id.clone())
                    .or_insert_with(DashMap::new)
                    .remove(symbol_code);
                (PositionSide::Flat, 0)
            }
        }
        _ => {
            // Clear positions for all other cases, including None, 0, or invalid values
            client.long_quantity
                .entry(account_id.clone())
                .or_insert_with(DashMap::new)
                .remove(symbol_code);
            client.short_quantity
                .entry(account_id.clone())
                .or_insert_with(DashMap::new)
                .remove(symbol_code);
            (PositionSide::Flat, 0)
        }
    }
}
