use reqwest::{Error, Response};
use std::sync::Arc;
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use tokio::time::interval;
use std::time::Duration;
use ahash::AHashMap;
use dashmap::DashMap;
use chrono::{DateTime, Timelike, Utc};
use ff_standard_lib::standardized_types::accounts::AccountId;
use ff_standard_lib::standardized_types::orders::{OrderState, OrderUpdateEvent};
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol};
use tokio::sync::broadcast;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use std::sync::atomic::Ordering;
use rust_decimal_macros::dec;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::resolution::Resolution;
use crate::oanda_api::api_client::{OandaClient, OANDA_IS_CONNECTED};
use crate::oanda_api::base_data_converters::{candle_from_candle, oanda_quotebar_from_candle};
use crate::oanda_api::get::requests::oanda_clean_instrument;
use crate::oanda_api::models::account::account::AccountChangesResponse;
use crate::oanda_api::models::order::order_related::OandaOrderState;
use crate::oanda_api::models::order::placement::OandaOrderUpdate;
use crate::oanda_api::models::position::parse_oanda_position;
use crate::oanda_api::support_and_conversions::resolution_to_oanda_interval;
use crate::request_handlers::RESPONSE_SENDERS;

impl OandaClient {
    pub async fn send_rest_request(&self, endpoint: &str) -> Result<Response, Error> {
        let url = format!("{}{}", self.base_endpoint, endpoint);
        let _permit = self.rate_limiter.acquire().await;
        match self.client.get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
        {
            Ok(response) => {
                Ok(response)
            }
            Err(e) => {
                Err(e)
            }
        }
    }

    pub async fn poll_account_changes(
        self: &Arc<Self>,
        account_id: &str,
        last_transaction_id: &str,
    ) -> Result<AccountChangesResponse, FundForgeError> {
        let request_uri = format!(
            "/accounts/{}/changes?sinceTransactionID={}",
            account_id,
            last_transaction_id
        );

        let response = match self.send_rest_request(&request_uri).await {
            Ok(r) => r,
            Err(e) => {
                return Err(FundForgeError::ServerErrorDebug(
                    format!("Failed to poll account changes: {:?}", e)
                ));
            }
        };

        if !response.status().is_success() {
            return Err(FundForgeError::ServerErrorDebug(
                format!("Server returned error status: {}", response.status())
            ));
        }

        let content = response.text().await.map_err(|e| {
            FundForgeError::ServerErrorDebug(format!("Failed to read response content: {:?}", e))
        })?;

        let changes: AccountChangesResponse = serde_json::from_str(&content).map_err(|e| {
            FundForgeError::ServerErrorDebug(format!("Failed to parse JSON response: {:?}", e))
        })?;

        Ok(changes)
    }

    pub fn handle_account_updates(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(750));

            // Track last transaction ID for each account
            let mut last_transaction_ids: AHashMap<AccountId, String> = AHashMap::new();
            let accounts = self.accounts.clone();
            let account_info = self.account_info.clone();
            let open_orders = self.open_orders.clone();
            loop {
                interval.tick().await;

                for account in &accounts {
                    let account_id = &account.account_id;

                    // Get the last known transaction ID for this account
                    let last_transaction_id = last_transaction_ids
                        .get(account_id)
                        .cloned()
                        .unwrap_or_else(|| "1".to_string()); // Start from 1 if no previous ID

                    match OandaClient::poll_account_changes(&self, account_id, &last_transaction_id).await {
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
                                    if let Some(positions) = self.positions.get_mut(account_id) {
                                        positions.clear();
                                    }

                                    // Add updated positions
                                    for position in changes.changes.positions {
                                        if let Some(parsed_position) = parse_oanda_position(
                                            position,
                                            account.clone()
                                        ) {
                                            self.positions
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
                        match self.get_order_by_client_id(account_id, &order.value().id).await {
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

    pub async fn get_order_by_client_id(
        self: &Arc<Self>,
        account_id: &str,
        client_order_id: &str,
    ) -> Result<OandaOrderUpdate, FundForgeError> {
        let request_uri = format!(
            "/accounts/{}/orders/@{}",
            account_id,
            client_order_id
        );

        let response = self.send_rest_request(&request_uri).await
            .map_err(|e| FundForgeError::ServerErrorDebug(
                format!("Failed to get_requests order: {:?}", e)
            ))?;

        if !response.status().is_success() {
            return Err(FundForgeError::ServerErrorDebug(
                format!("Server returned error status: {}", response.status())
            ));
        }

        let content = response.text().await
            .map_err(|e| FundForgeError::ServerErrorDebug(
                format!("Failed to read response content: {:?}", e)
            ))?;

        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| FundForgeError::ServerErrorDebug(
                format!("Failed to parse JSON response: {:?}", e)
            ))?;

        // Extract the order directly from the "order" field
        let order: OandaOrderUpdate = serde_json::from_value(json["order"].clone())
            .map_err(|e| FundForgeError::ServerErrorDebug(
                format!("Failed to parse order data: {:?}", e)
            ))?;

        Ok(order)
    }

    // allows us to subscribe to quote bars by manually requesting bar updates every 5 seconds
    pub fn handle_quotebar_subscribers(
        self: Arc<Self>,
        account_id: AccountId,
    ) {
        tokio::spawn(async move {
            let last_closed_time: DashMap<DataSubscription, DateTime<Utc>> = DashMap::new();
            let quotebar_broadcasters: Arc<DashMap<DataSubscription, broadcast::Sender<BaseDataEnum>>> = self.quotebar_broadcasters.clone();

            loop {
                // Calculate delay until next 5-second boundary + 10ms
                let now = Utc::now();
                let next_five_seconds = now
                    .with_nanosecond(0).unwrap()
                    .checked_add_signed(chrono::Duration::seconds((5 - (now.second() % 5)) as i64))
                    .unwrap();
                let target_time = next_five_seconds + chrono::Duration::milliseconds(10);
                let delay = target_time.signed_duration_since(now);

                // Sleep until the next tick
                if delay.num_milliseconds() > 0 {
                    tokio::time::sleep(Duration::from_millis(delay.num_milliseconds() as u64)).await;
                }

                let mut to_remove = Vec::new();
                for broadcaster in quotebar_broadcasters.iter() {
                    let bars = match self.get_latest_bars(
                        &broadcaster.key().symbol,
                        broadcaster.key().base_data_type,
                        broadcaster.key().resolution,
                        &account_id,
                        2
                    ).await {
                        Ok(bars) => bars,
                        Err(e) => {
                            eprintln!("Failed to get_requests latest bars for quotebar subscriber: {}", e);
                            continue
                        }
                    };

                    for bar in bars {
                        if let Some(last_time) = last_closed_time.get(&broadcaster.key()) {
                            if bar.time_closed_utc() > *last_time.value() || !bar.is_closed() {
                                match broadcaster.value().send(bar.clone()) {
                                    Ok(_) => {}
                                    Err(_) => {
                                        if broadcaster.receiver_count() == 0 {
                                            to_remove.push(broadcaster.key().clone())
                                        }
                                    }
                                }
                                last_closed_time.insert(bar.subscription(), bar.time_closed_utc());
                            }
                        }
                    }
                }
                for key in to_remove {
                    quotebar_broadcasters.remove(&key);
                }
            }
        });
    }

    pub async fn send_download_request(&self, endpoint: &str) -> Result<Response, Error> {
        let url = format!("{}{}", self.base_endpoint, endpoint);
        // Acquire a permit asynchronously
        // Use a guard pattern to ensure we release permits properly
        let _rate_permit = self.rate_limiter.acquire().await;
        let _download_permit = if !self.quote_feed_broadcasters.is_empty() {
            Some(self.download_limiter.acquire().await)
        } else {
            None
        };

        match self.client.get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
        {
            Ok(response) => {
                OANDA_IS_CONNECTED.store(true, Ordering::SeqCst);
                Ok(response)
            }
            Err(e) => {
              Err(e)
            }
        }
    }

    pub async fn get_latest_bars(
        &self,
        symbol: &Symbol,
        base_data_type: BaseDataType,
        resolution: Resolution,
        account_id: &str,
        units: i32,
    ) -> Result<Vec<BaseDataEnum>, FundForgeError> {
        let interval = resolution_to_oanda_interval(&resolution)
            .ok_or_else(|| FundForgeError::ClientSideErrorDebug("Invalid resolution".to_string()))?;

        let instrument = oanda_clean_instrument(&symbol.name).await;

        // For bid/ask we use "BA" instead of separate "B" and "A" specifications
        let candle_spec = match base_data_type {
            BaseDataType::QuoteBars => format!("{}:{}:BA", instrument, interval),
            _ => return Err(FundForgeError::ClientSideErrorDebug("Unsupported data type".to_string())),
        };

        // Use UTC alignment
        let url = format!(
            "/accounts/{}/candles/latest?candleSpecifications={}&smooth=false&alignmentTimezone=UTC&units={}",
            account_id,
            candle_spec,
            units
        );

        let response = match self.send_rest_request(&url).await {
            Ok(response) => response,
            Err(e) => {
                return Err(FundForgeError::ClientSideErrorDebug(format!("Failed to get_requests latest bars: {}", e)));
            }
        };

        if !response.status().is_success() {
            return Err(FundForgeError::ClientSideErrorDebug(format!(
                "Failed to get_requests latest bars: HTTP {}",
                response.status()
            )));
        }

        let content = response.text().await.map_err(|e| {
            FundForgeError::ClientSideErrorDebug(format!("Failed to get_requests response text: {}", e))
        })?;

        let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            FundForgeError::ClientSideErrorDebug(format!("Failed to parse JSON: {}", e))
        })?;

        let latest_candles = json["latestCandles"].as_array().ok_or_else(|| {
            FundForgeError::ClientSideErrorDebug("No latestCandles array in response".to_string())
        })?;

        let mut bars = Vec::new();

        for candle_response in latest_candles {
            let candles = candle_response["candles"].as_array().ok_or_else(|| {
                FundForgeError::ClientSideErrorDebug("No candles array in response".to_string())
            })?;

            for price_data in candles {
                // Only process complete candles
                if !price_data["complete"].as_bool().unwrap_or(false) {
                    continue;
                }

                let bar: BaseDataEnum = match base_data_type {
                    BaseDataType::QuoteBars => {
                        match oanda_quotebar_from_candle(price_data, symbol.clone(), resolution.clone()) {
                            Ok(quotebar) => BaseDataEnum::QuoteBar(quotebar),
                            Err(e) => {
                                eprintln!("Failed to create quote bar: {}", e);
                                continue;
                            }
                        }
                    },
                    BaseDataType::Candles => {
                        match candle_from_candle(price_data, symbol.clone(), resolution.clone()) {
                            Ok(candle) => BaseDataEnum::Candle(candle),
                            Err(e) => {
                                eprintln!("Failed to create candle: {}", e);
                                continue;
                            }
                        }
                    },
                    _ => continue,
                };

                bars.push(bar);
            }
        }

        bars.sort_by_key(|bar| bar.time_utc());
        Ok(bars)
    }
}