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
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::orders::OrderId;
use ff_standard_lib::standardized_types::position::Position;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol, SymbolName};
use crate::oanda_api::get::accounts::account_details::get_oanda_account_details;
use crate::oanda_api::get::accounts::account_list::get_oanda_accounts_list;
use crate::oanda_api::get::accounts::account_summary::get_oanda_account_summary;
use crate::oanda_api::get::instruments::get_oanda_instruments;
use crate::oanda_api::handlers::stream::handle_price_stream;
use crate::oanda_api::get::instruments::OandaInstrument;
use crate::oanda_api::get::positions::parse_oanda_position;
use crate::oanda_api::handlers::quotebar_streams::handle_quotebar_subscribers;
use crate::oanda_api::models::order::placement::OandaOrderUpdate;
use crate::ServerLaunchOptions;

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
    pub download_limiter: Arc<RateLimiter>,
    pub api_key: String,
    pub base_endpoint: String,
    pub stream_endpoint: String,
    pub instruments_map: Arc<DashMap<SymbolName, OandaInstrument>>,
    pub accounts: Vec<Account>,
    pub account_info: DashMap<AccountId, AccountInfo>,
    pub positions: DashMap<AccountId, DashMap<SymbolName, Position>>,
    pub instrument_symbol_map: Arc<DashMap<String, Symbol>>,
    pub quote_feed_broadcasters: Arc<DashMap<SymbolName, broadcast::Sender<BaseDataEnum>>>,
    pub quote_subscription_sender: Sender<Vec<SymbolName>>,
    pub oanda_id_map: DashMap<String, OrderId>,
    pub open_orders: DashMap<OrderId, ff_standard_lib::standardized_types::orders::Order>,
    pub id_stream_name_map: DashMap<OrderId , u16>,
    pub last_transaction_id: DashMap<AccountId, String>,
    pub quotebar_broadcasters: Arc<DashMap<DataSubscription, broadcast::Sender<BaseDataEnum>>>
}

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
        download_limiter: RateLimiter::new(60, Duration::from_secs(1)),
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
        quotebar_broadcasters: Arc::new(Default::default()),
        quote_subscription_sender: sender,
        oanda_id_map: Default::default(),
        open_orders: Default::default(),
        id_stream_name_map: Default::default(),
        last_transaction_id: Default::default(),
    };
    match get_oanda_accounts_list(&oanda_client).await {
        Ok(accounts) => oanda_client.accounts = accounts.clone(),
        Err(e) => eprintln!("Error getting accounts: {:?}", e)
    };
    if let Some(account) = oanda_client.accounts.get(0) {
        let instruments = get_oanda_instruments(&oanda_client, &account.account_id).await.unwrap_or_else(|| vec![]);
        for instrument in instruments {
            oanda_client.instrument_symbol_map.insert(instrument.name.clone(), Symbol::new(instrument.symbol_name.clone(), DataVendor::Oanda, instrument.market_type.clone()));
            oanda_client.instruments_map.insert(instrument.symbol_name.clone(), instrument);
        }
    }
    for account in &oanda_client.accounts {
        match get_oanda_account_summary(&oanda_client, &account.account_id).await {
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
        match get_oanda_account_details(&oanda_client, &account.account_id).await {
            Ok(details) => {
                //eprintln!("Oanda account details: {:?}", details);
                for position in details.positions {
                    match parse_oanda_position(position, account.clone()) {
                        Some(pos) => {
                            oanda_client.positions.entry(account.account_id.clone()).or_insert(Default::default()).insert(pos.symbol_name.clone(), pos);
                        }
                        None => {}
                    }
                }

            }
            Err(e) => eprintln!("Error getting oanda account positions: {}", e)
        }
    }
    let stream_limit = Arc::new(Semaphore::new(20));
    if let Some(account) = oanda_client.accounts.get(0) {
        handle_price_stream(oanda_client.streaming_client.clone(), oanda_client.instrument_symbol_map.clone(), oanda_client.instruments_map.clone(), oanda_client.quote_feed_broadcasters.clone(), receiver, account.clone(), stream_limit.clone(), oanda_client.stream_endpoint.clone(), oanda_client.api_key.clone());
    }
    let client =Arc::new(oanda_client);
    handle_quotebar_subscribers(client.clone(), client.accounts.get(0).unwrap().account_id.clone());
    eprintln!("Oanda client initialized");
    let _ = OANDA_CLIENT.set(client);
}

impl OandaClient {


    pub async fn get_order_by_client_id(
        &self,
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

}