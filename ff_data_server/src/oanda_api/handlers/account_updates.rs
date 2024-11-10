use std::sync::Arc;
use ff_standard_lib::messages::data_server_messaging::DataServerResponse;
use tokio::time::interval;
use std::time::Duration;
use ahash::AHashMap;
use dashmap::DashMap;
use chrono::Utc;
use ff_standard_lib::standardized_types::accounts::AccountId;
use ff_standard_lib::standardized_types::orders::{OrderState, OrderUpdateEvent};
use rust_decimal_macros::dec;
use crate::oanda_api::api_client::OandaClient;
use crate::oanda_api::get::accounts::account_changes::get_account_changes;
use crate::oanda_api::get::positions::parse_oanda_position;
use crate::oanda_api::models::order::order_related::OandaOrderState;
use crate::request_handlers::RESPONSE_SENDERS;

pub fn handle_account_updates(client: Arc<OandaClient>) {
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_millis(750));

        // Track last transaction ID for each account
        let mut last_transaction_ids: AHashMap<AccountId, String> = ahash::AHashMap::new();
        let accounts = client.accounts.clone();
        let account_info = client.account_info.clone();
        let open_orders = client.open_orders.clone();
        loop {
            interval.tick().await;

            for account in &accounts {
                let account_id = &account.account_id;

                // Get the last known transaction ID for this account
                let last_transaction_id = last_transaction_ids
                    .get(account_id)
                    .cloned()
                    .unwrap_or_else(|| "1".to_string()); // Start from 1 if no previous ID

                match get_account_changes(&client, account_id, &last_transaction_id).await {
                    Ok(changes) => {
                        // Update account info if we have it
                        if let Some(mut account_info) = account_info.get_mut(account_id) {
                            // Update account state
                            account_info.cash_available = changes.state.margin_available;
                            account_info.open_pnl = changes.state.unrealized_pl;
                            account_info.cash_used = changes.state.margin_used;

                            // Process position changes
                            if !changes.changes.positions.is_empty() {
                                // Clear existing positions for this account
                                if let Some(positions) = client.positions.get_mut(account_id) {
                                    positions.clear();
                                }

                                // Add updated positions
                                for position in changes.changes.positions {
                                    if let Some(parsed_position) = parse_oanda_position(
                                        position,
                                        account.clone()
                                    ) {
                                        client.positions
                                            .entry(account_id.clone())
                                            .or_insert_with(DashMap::new)
                                            .insert(
                                                parsed_position.symbol_name.clone(),
                                                parsed_position.clone()
                                            );
                                        let message = DataServerResponse::LivePositionUpdates {
                                            account: account.clone(),
                                            position: parsed_position,
                                            time: Utc::now().to_string(),
                                        };
                                        for stream_name in RESPONSE_SENDERS.iter() {
                                            match stream_name.value().send(message.clone()).await {
                                                Ok(_) => {}
                                                Err(e) => {
                                                    eprintln!("failed to forward ResponseNewOrder 313 to strategy stream {}", e);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Update the last transaction ID
                        last_transaction_ids.insert(
                            account_id.clone(),
                            changes.last_transaction_id
                        );

                        if let Some(account_info) = account_info.get(account_id) {
                            let account_updates = DataServerResponse::LiveAccountUpdates {
                                account: account.clone(),
                                cash_value: account_info.cash_value,
                                cash_available: account_info.cash_available,
                                cash_used: account_info.cash_used,
                            };
                            for stream_name in RESPONSE_SENDERS.iter() {
                                match stream_name.value().send(account_updates.clone()).await {
                                    Ok(_) => {}
                                    Err(e) => {
                                        eprintln!("failed to forward ResponseNewOrder 313 to strategy stream {}", e);

                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error polling account changes: {}", e);
                    }
                }

                // now handle open orders
                let mut to_remove = Vec::new();
                for mut order in open_orders.iter_mut() {
                    match client.get_order_by_client_id(account_id, &order.value().id).await {
                        Ok(oanda_order) => {
                            let order_state = match oanda_order.state {
                                OandaOrderState::Pending => OrderState::Accepted,
                                OandaOrderState::Filled => OrderState::Filled,
                                OandaOrderState::Triggered => OrderState::Accepted,
                                OandaOrderState::Cancelled => OrderState::Cancelled
                            };

                            if order_state != order.state {
                                order.state = order_state.clone();
                                let message = match order_state {
                                    OrderState::Filled => {
                                        order.quantity_filled = order.quantity_open.clone();
                                        order.quantity_open = dec!(0);
                                        to_remove.push(order.key().clone());
                                        DataServerResponse::OrderUpdates {
                                            event: OrderUpdateEvent::OrderFilled {
                                                account: order.account.clone(),
                                                symbol_name: order.symbol_name.clone(),
                                                symbol_code: order.symbol_name.clone(),
                                                order_id: order.key().clone(),
                                                side: order.side.clone(),
                                                price: Default::default(),
                                                quantity: Default::default(),
                                                tag: order.tag.clone(),
                                                time: Utc::now().to_string(),
                                            },
                                            time: Utc::now().to_string(),
                                        }
                                    }
                                    OrderState::Accepted => {
                                        DataServerResponse::OrderUpdates {
                                            event: OrderUpdateEvent::OrderAccepted {
                                                account: order.account.clone(),
                                                symbol_name: order.symbol_name.clone(),
                                                symbol_code: order.symbol_name.clone(),
                                                order_id: order.key().clone(),
                                                tag: order.tag.clone(),
                                                time: Utc::now().to_string(),
                                            },
                                            time: Utc::now().to_string(),
                                        }
                                    }
                                    OrderState::Cancelled => {
                                        to_remove.push(order.key().clone());
                                        DataServerResponse::OrderUpdates {
                                            event: OrderUpdateEvent::OrderCancelled {
                                                account: order.account.clone(),
                                                symbol_name: order.symbol_name.clone(),
                                                symbol_code: order.symbol_name.clone(),
                                                order_id: order.key().clone(),
                                                reason: "Oanda provides no reason".to_string(),
                                                tag: order.tag.clone(),
                                                time: Utc::now().to_string(),
                                            },
                                            time: Utc::now().to_string(),
                                        }
                                    }
                                    _ => continue
                                };
                                for stream_name in RESPONSE_SENDERS.iter() {
                                    match stream_name.value().send(message.clone()).await {
                                        Ok(_) => {}
                                        Err(e) => {
                                            eprintln!("failed to forward ResponseNewOrder 313 to strategy stream {}", e);
                                        }
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            to_remove.push(order.key().clone());
                            eprintln!("Failed to get_requests order: {}", e);
                            continue;
                        }
                    };
                }
                for key in to_remove {
                    open_orders.remove(&key);
                }
            }
        }
    });
}

