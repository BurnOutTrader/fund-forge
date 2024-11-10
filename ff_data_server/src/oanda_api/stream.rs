use std::sync::Arc;
use dashmap::DashMap;
use ff_standard_lib::standardized_types::subscriptions::{Symbol, SymbolName};
use tokio::sync::{broadcast, Semaphore};
use ff_standard_lib::messages::data_server_messaging::FundForgeError;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use ff_standard_lib::standardized_types::base_data::quote::Quote;
use reqwest::{Client, Response};
use async_std::prelude::Stream;
use bytes::Bytes;
use std::sync::atomic::Ordering;
use tokio::sync::mpsc::Receiver;
use ff_standard_lib::standardized_types::accounts::Account;
use std::pin::Pin;
use std::str::FromStr;
use futures_util::TryStreamExt;
use crate::oanda_api::api_client::OANDA_IS_CONNECTED;
use crate::oanda_api::instruments::OandaInstrument;
use crate::oanda_api::models::pricing_common::PriceStreamResponse;
use crate::subscribe_server_shutdown;

/// Establishes a streaming connection to the specified endpoint suffix.
/// This method respects the `stream_limit` semaphore.
pub async fn establish_stream(
    client: &Arc<Client>,
    stream_endpoint: &str,
    stream_endpoint_suffix: &str,
    stream_limit: &Arc<Semaphore>,
    api_key: &str
) -> Result<impl Stream<Item = Result<Bytes, reqwest::Error>>, FundForgeError> {
    let url = format!("{}{}", stream_endpoint, stream_endpoint_suffix);
    eprintln!("Attempting to connect to URL: {}", url);

    // Acquire a stream permit asynchronously.
    let _stream_permit = stream_limit.acquire().await.expect("Failed to acquire stream permit");

    // Make a GET request to the streaming endpoint
    let response: Response = match client.get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("Stream request failed: {}", e);
            return Err(FundForgeError::ServerErrorDebug(format!("Stream request failed: {}", e)));
        }
    };

    // Check response status
    let status = response.status();
    eprintln!("Stream response status: {}", status);

    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Could not get error text".to_string());
        eprintln!("Stream request failed: {}", error_text);
        return Err(FundForgeError::ServerErrorDebug(format!(
            "Stream request failed with status {}: {}",
            status,
            error_text
        )));
    }

    // Log headers for debugging
    eprintln!("Response headers: {:?}", response.headers());

    OANDA_IS_CONNECTED.store(true, Ordering::SeqCst);
    Ok(response.bytes_stream())
}



pub fn handle_price_stream(
    client: Arc<Client>,
    instrument_symbol_map: Arc<DashMap<String, Symbol>>,
    instruments_map: Arc<DashMap<SymbolName, OandaInstrument>>,
    quote_feed_broadcasters: Arc<DashMap<SymbolName, broadcast::Sender<BaseDataEnum>>>,
    mut subscription_receiver: Receiver<Vec<SymbolName>>,
    account: Account,
    stream_limit: Arc<Semaphore>,
    stream_endpoint: String,
    api_key: String
) {
    tokio::spawn(async move {
        let mut current_stream: Option<Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>> = None;
        let mut current_subscriptions = Vec::new();
        let mut shutdown_receiver = subscribe_server_shutdown();

        loop {
            tokio::select! {
                Ok(_) = shutdown_receiver.recv() => break,

                Some(new_subscriptions) = subscription_receiver.recv() => {
                    let mut cleaned_new_subscriptions = vec![];

                    // Check for new active subscriptions
                    for sub in new_subscriptions {
                        if let Some(instrument) = instruments_map.get(&sub) {
                            // Add to cleaned subs if there's an active broadcaster or it's a new subscription
                            if let Some(broadcaster) = quote_feed_broadcasters.get(&sub) {
                                if broadcaster.receiver_count() > 0 {
                                    cleaned_new_subscriptions.push(instrument.name.clone());
                                }
                            }
                        }
                    }

                    // Always check if current subscriptions still have active broadcasters
                    current_subscriptions.retain(|sub| {
                        if let Some(broadcaster) = quote_feed_broadcasters.get(sub) {
                            broadcaster.receiver_count() > 0
                        } else {
                            false
                        }
                    });

                    // Merge current and new subscriptions without duplicates
                    for sub in cleaned_new_subscriptions {
                        if !current_subscriptions.contains(&sub) {
                            current_subscriptions.push(sub);
                        }
                    }

                    // If we have any subscriptions, ensure stream is active
                    if !current_subscriptions.is_empty() {
                        let suffix = format!("/accounts/{}/pricing/stream?instruments={}",
                            account.account_id,
                            current_subscriptions.join("%2C")
                        );

                        // Only create new stream if we don't have one or subscriptions changed
                        if current_stream.is_none() {
                            match establish_stream(&client, &stream_endpoint, &suffix, &stream_limit, &api_key).await {
                                Ok(stream) => {
                                    current_stream = Some(Box::pin(stream));
                                    OANDA_IS_CONNECTED.store(true, Ordering::SeqCst);
                                }
                                Err(e) => {
                                    eprintln!("Failed to establish stream: {}", e);
                                    OANDA_IS_CONNECTED.store(false, Ordering::SeqCst);
                                }
                            }
                        }
                    }
                }

                Some(chunk_result) = async {
                    match &mut current_stream {
                        Some(stream) => stream.try_next().await.transpose(),
                        None => {
                            // Check for any active subscriptions periodically
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                            if !current_subscriptions.is_empty() {
                                let suffix = format!("/accounts/{}/pricing/stream?instruments={}",
                                    account.account_id,
                                    current_subscriptions.join("%2C")
                                );

                                match establish_stream(&client, &stream_endpoint, &suffix, &stream_limit, &api_key).await {
                                    Ok(stream) => {
                                        current_stream = Some(Box::pin(stream));
                                        OANDA_IS_CONNECTED.store(true, Ordering::SeqCst);
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to establish stream: {}", e);
                                        OANDA_IS_CONNECTED.store(false, Ordering::SeqCst);
                                    }
                                }
                            }
                            Some(Ok(Bytes::new()))
                        }
                    }
                } => {
                    match chunk_result {
                        Ok(chunk) => {
                            if chunk.is_empty() {
                                continue;
                            }

                            let text = match String::from_utf8(chunk.to_vec()) {
                                Ok(t) => t,
                                Err(e) => {
                                    eprintln!("Invalid UTF-8 in chunk: {}", e);
                                    continue;
                                }
                            };

                            if text.starts_with("<html>") || text.contains("404 Not Found") {
                                eprintln!("Received HTML error response, clearing stream");
                                current_stream = None;
                                OANDA_IS_CONNECTED.store(false, Ordering::SeqCst);
                                continue;
                            }

                            match process_stream_data(
                                &text,
                                &instrument_symbol_map,
                                &quote_feed_broadcasters
                            ).await {
                                Ok(_) => {},
                                Err(e) => {
                                    match e {
                                        FundForgeError::ConnectionNotFound(_) => {
                                            // Only remove dead subscriptions, keep active ones
                                            current_subscriptions.retain(|sub| {
                                                if let Some(broadcaster) = quote_feed_broadcasters.get(sub) {
                                                    broadcaster.receiver_count() > 0
                                                } else {
                                                    false
                                                }
                                            });

                                            // Drop stream to force reconnection with current subscriptions
                                            current_stream = None;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            current_stream = None;
                            OANDA_IS_CONNECTED.store(false, Ordering::SeqCst);
                        }
                    }
                }
            }
        }
    });
}

async fn process_stream_data(
    text: &str,
    instrument_symbol_map: &Arc<DashMap<String, Symbol>>,
    quote_feed_broadcasters: &Arc<DashMap<SymbolName, broadcast::Sender<BaseDataEnum>>>
) -> Result<(), FundForgeError> {
    // Parse the incoming JSON
    let price_data: PriceStreamResponse = match serde_json::from_str(text) {
        Ok(data) => data,
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(format!("Failed to parse JSON: {}", e).into()));
        }
    };

    // Skip heartbeat messages
    if price_data.r#type.as_deref() == Some("HEARTBEAT") {
        return Ok(());
    }

    // Get the best prices from the order book
    let (best_ask, best_ask_liquidity) = price_data.asks.first()
        .map(|bucket| (bucket.price, bucket.liquidity))
        .unwrap_or_else(|| {
            // Fallback to closeout prices if no order book
            let price = Decimal::from_str(&price_data.closeout_ask).unwrap_or_default();
            (price, Decimal::default())
        });

    let (best_bid, best_bid_liquidity) = price_data.bids.first()
        .map(|bucket| (bucket.price, bucket.liquidity))
        .unwrap_or_else(|| {
            // Fallback to closeout prices if no order book
            let price = Decimal::from_str(&price_data.closeout_bid).unwrap_or_default();
            (price, Decimal::default())
        });

    // Parse timestamp
    let time = match DateTime::parse_from_rfc3339(&price_data.time) {
        Ok(time) => time.with_timezone(&Utc),
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(format!("Failed to parse timestamp: {}", e).into()));
        }
    }.with_timezone(&Utc);

    // Look up the symbol
    let symbol = match instrument_symbol_map.get(&price_data.instrument) {
        Some(symbol) => symbol.clone(),
        None => {
            return Err(FundForgeError::ServerErrorDebug(format!("Symbol not found in map: {}", price_data.instrument).into()));
        }
    };

    // Check if we have subscribers and broadcast if we do
    let mut remove_broadcaster = false;

    if let Some(broadcaster) = quote_feed_broadcasters.get(&symbol.name) {
        let quote = BaseDataEnum::Quote(Quote::new(
            symbol.clone(),
            best_ask,
            best_bid,
            best_ask_liquidity,
            best_bid_liquidity,
            time.to_string(),
        ));

        match broadcaster.send(quote) {
            Ok(_) => {}
            Err(_) => {
                // Mark broadcaster for removal if there are no receivers
                if broadcaster.receiver_count() == 0 {
                    remove_broadcaster = true;
                }
            }
        }
    }

    if remove_broadcaster {
        quote_feed_broadcasters.remove(&symbol.name);
        return Err(FundForgeError::ConnectionNotFound("Broadcaster removed due to no receivers".into()));
    }

    Ok(())
}