use reqwest::{Client, Error, Response};
use std::time::Duration;
use tokio::sync::{broadcast, OnceCell, Semaphore};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::oanda_api::settings::{OandaApiMode, OandaSettings};
use crate::rate_limiter::RateLimiter;
use dashmap::DashMap;
use lazy_static::lazy_static;
use tokio::sync::mpsc::Sender;
use ff_standard_lib::messages::data_server_messaging::FundForgeError;
use ff_standard_lib::standardized_types::accounts::{Account, AccountId, AccountInfo};
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::{Symbol, SymbolName};
use crate::oanda_api::base_data_converters::{candle_from_candle, oanda_quotebar_from_candle};
use crate::oanda_api::get_requests::{oanda_account_summary, oanda_accounts_list, oanda_clean_instrument, oanda_instruments_download};
use crate::oanda_api::instruments::OandaInstrument;
use crate::oanda_api::models::position::OandaPosition;
use crate::oanda_api::support_and_conversions::resolution_to_oanda_interval;
use crate::server_features::database::DATA_STORAGE;
use crate::ServerLaunchOptions;
use crate::oanda_api::stream;

lazy_static! {
    pub static ref OANDA_IS_CONNECTED: AtomicBool = AtomicBool::new(false);
}

pub(crate) static OANDA_CLIENT: OnceCell<Arc<OandaClient>> = OnceCell::const_new();
pub fn get_oanda_client() -> Option<Arc<OandaClient>> {
    match OANDA_CLIENT.get() {
        None => None,
        Some(c) => Some(c.clone())
    }
}

pub fn get_oanda_client_ref() -> &'static Arc<OandaClient> {
    OANDA_CLIENT.get().expect("Oanda client not initialized")
}

/// http2 client for Oanda
///
/// # Properties
/// * `client` - The reqwest client
/// * `rate_limit` - The rate limit semaphore 120 per second
pub struct OandaClient {
    pub client: Arc<Client>,
    pub streaming_client: Arc<Client>,
    pub rate_limiter: Arc<RateLimiter>,
    pub api_key: String,
    pub base_endpoint: String,
    pub stream_endpoint: String,
    pub instruments_map: Arc<DashMap<SymbolName, OandaInstrument>>,
    pub accounts: Vec<Account>,
    pub account_info: DashMap<AccountId, AccountInfo>,
    pub positions: DashMap<AccountId, OandaPosition>,
    pub instrument_symbol_map: Arc<DashMap<String, Symbol>>,
    pub quote_feed_broadcasters: Arc<DashMap<SymbolName, broadcast::Sender<BaseDataEnum>>>,
    pub subscription_sender: Sender<Vec<SymbolName>>,
}
pub(crate) async fn oanda_init(options: ServerLaunchOptions) {
    if options.disable_oanda_server != 0 {
        OANDA_IS_CONNECTED.store(false, Ordering::SeqCst);
        return;
    }
    let path = options.data_folder.clone()
        .join("credentials")
        .join("oanda_credentials")
        .join("active")
        .join("oanda_credentials.toml");

    if !path.exists() {
        OANDA_IS_CONNECTED.store(false, Ordering::SeqCst);
        return;
    }

    let settings: OandaSettings = match OandaSettings::from_file(path) {
        Some(s) => s,
        None => {
            OANDA_IS_CONNECTED.store(false, Ordering::SeqCst);
            eprintln!("No oanda settings retrieved");
            return;
        }
    };

    // Create dedicated streaming client with different settings
    let streaming_client = match Client::builder()
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", settings.api_key)).unwrap(),
            );
            headers
        })
        .http2_keep_alive_while_idle(true)  // Important for streaming
        .http2_keep_alive_interval(Duration::from_secs(5))
        .http2_keep_alive_timeout(Duration::from_secs(20))
        .build()
    {
        Ok(client) => {
            eprintln!("Oanda streaming client connected");
            OANDA_IS_CONNECTED.store(true, Ordering::SeqCst);
            client
        }
        Err(_) => {
            eprintln!("Oanda streaming client failed to connect");
            OANDA_IS_CONNECTED.store(false, Ordering::SeqCst);
            return;
        }
    };
    let streaming_client = Arc::new(streaming_client);

    let client = match Client::builder().default_headers({
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", settings.api_key)).unwrap(),
        );
        headers
    })
        .http2_keep_alive_while_idle(true)
        .build()
    {
        Ok(client) => {
            eprintln!("Oanda client connected");
            client
        }
        Err(_) => {
            eprintln!("Oanda client failed to connect");
            OANDA_IS_CONNECTED.store(false, Ordering::SeqCst);
            return;
        }
    };
    let client = Arc::new(client);

    let rate_limiter = RateLimiter::new(120, Duration::from_secs(1));
    let (sender, receiver) = tokio::sync::mpsc::channel(5);
    let mut oanda_client = OandaClient {
        client,
        streaming_client,
        rate_limiter,
        api_key: settings.api_key.clone(),
        base_endpoint: match &settings.mode {
            OandaApiMode::Live => "https://api-fxtrade.oanda.com/v3",
            OandaApiMode::Practice => "https://api-fxpractice.oanda.com/v3",
        }.to_string(),
        stream_endpoint: match &settings.mode {
            OandaApiMode::Live => "https://stream-fxtrade.oanda.com/v3",
            OandaApiMode::Practice => "https://stream-fxpractice.oanda.com/v3",
        }.to_string(),
        instruments_map: Default::default(),
        accounts: vec![],
        account_info: Default::default(),
        positions: Default::default(),
        instrument_symbol_map: Default::default(),
        quote_feed_broadcasters: Arc::new(Default::default()),
        subscription_sender: sender,
    };
    match oanda_accounts_list(&oanda_client).await {
        Ok(accounts) => oanda_client.accounts = accounts.clone(),
        Err(e) => eprintln!("Error getting accounts: {:?}", e)
    };
    if let Some(account) = oanda_client.accounts.get(0) {
        let instruments = oanda_instruments_download(&oanda_client, &account.account_id).await.unwrap_or_else(|| vec![]);
        for instrument in instruments {
            oanda_client.instrument_symbol_map.insert(instrument.name.clone(), Symbol::new(instrument.symbol_name.clone(), DataVendor::Oanda, instrument.market_type.clone()));
            oanda_client.instruments_map.insert(instrument.symbol_name.clone(), instrument);
        }
    }
    for account in &oanda_client.accounts {
        match oanda_account_summary(&oanda_client, &account.account_id).await {
            Ok(summary) => {
                let info = AccountInfo {
                    account_id: account.account_id.clone(),
                    brokerage: Brokerage::Oanda,
                    cash_value: summary.balance,
                    cash_available: summary.margin_available,
                    currency: summary.currency,
                    open_pnl: summary.unrealized_pl,
                    booked_pnl: summary.pl,
                    day_open_pnl: Default::default(),
                    day_booked_pnl: Default::default(),
                    cash_used: summary.margin_used,
                    positions: vec![],
                    is_hedging: summary.hedging_enabled,
                    buy_limit: None,
                    sell_limit: None,
                    max_orders: None,
                    daily_max_loss: None,
                    daily_max_loss_reset_time: None,
                    leverage: 30
                };
                oanda_client.account_info.insert(account.account_id.clone(), info);
            }
            Err(e) => eprintln!("Error getting oanda account info: {}", e)
        }
    }
    let stream_limit = Arc::new(Semaphore::new(20));
    if let Some(account) = oanda_client.accounts.get(0) {
        stream::handle_price_stream(oanda_client.streaming_client.clone(), oanda_client.instrument_symbol_map.clone(), oanda_client.instruments_map.clone(), oanda_client.quote_feed_broadcasters.clone(), receiver, account.clone(), stream_limit.clone(), oanda_client.stream_endpoint.clone(), oanda_client.api_key.clone()).await;
    }
    eprintln!("Oanda client initialized");
    let _ = OANDA_CLIENT.set(Arc::new(oanda_client));
}

impl OandaClient {
    pub async fn send_rest_request(&self, endpoint: &str) -> Result<Response, Error> {
        let url = format!("{}{}", self.base_endpoint, endpoint);
        let mut retries = 0;
        let max_retries = 50;
        let base_delay = Duration::from_secs(1);

        loop {
            // Acquire a permit asynchronously
            let _permit = self.rate_limiter.acquire().await;

            match self.client.get(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .send()
                .await
            {
                Ok(response) => {
                    OANDA_IS_CONNECTED.store(true, Ordering::SeqCst);
                    return Ok(response);
                }
                Err(e) => {
                    OANDA_IS_CONNECTED.store(false, Ordering::SeqCst);
                    if retries >= max_retries {
                        return Err(e);
                    }

                    // Exponential backoff with jitter
                    let delay = base_delay * 2u32.pow(retries as u32);
                    let jitter = Duration::from_millis(rand::random::<u64>() % 1000);
                    tokio::time::sleep(delay + jitter).await;

                    retries += 1;
                    eprintln!("REST request failed, attempt {}/{}: {}", retries, max_retries, e);
                    continue;
                }
            }
        }
    }

    pub async fn get_latest_bars(
        &self,
        symbol: Symbol,
        base_data_type: BaseDataType,
        resolution: Resolution,
        account_id: &str,
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
            "/accounts/{}/candles/latest?candleSpecifications={}&smooth=false&alignmentTimezone=UTC",
            account_id,
            candle_spec
        );

        let response = match self.send_rest_request(&url).await {
            Ok(response) => response,
            Err(e) => {
                return Err(FundForgeError::ClientSideErrorDebug(format!("Failed to get latest bars: {}", e)));
            }
        };

        if !response.status().is_success() {
            return Err(FundForgeError::ClientSideErrorDebug(format!(
                "Failed to get latest bars: HTTP {}",
                response.status()
            )));
        }

        let content = response.text().await.map_err(|e| {
            FundForgeError::ClientSideErrorDebug(format!("Failed to get response text: {}", e))
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

    pub async fn update_latest_bars(
        &self,
        symbol: Symbol,
        base_data_type: BaseDataType,
        resolution: Resolution,
    ) -> Result<(), FundForgeError> {
        let data_storage = DATA_STORAGE.get().unwrap();
        let account_id = if let Some(id) = self.accounts.get(0) {
            id.account_id.clone()
        } else {
            return Err(FundForgeError::ClientSideErrorDebug("No account ID found".to_string()));
        };


        let bars = self.get_latest_bars(symbol, base_data_type, resolution, &account_id).await?;

        if bars.is_empty() {
            return Ok(());
        }


        const MAX_RETRIES: u32 = 3;
        let mut retry_count = 0;
        while retry_count < MAX_RETRIES {
            match data_storage.save_data_bulk(bars.clone()).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= MAX_RETRIES {
                        return Err(FundForgeError::ClientSideErrorDebug(
                            format!("Failed to save latest bars after {} retries: {}", MAX_RETRIES, e)
                        ));
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }

        Ok(())
    }
}
