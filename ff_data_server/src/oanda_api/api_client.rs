use std::str::FromStr;
use reqwest::{Client, Error, Response};
use std::time::{Duration};
use futures::stream::{Stream};
use tokio::sync::{broadcast, OnceCell, Semaphore};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::oanda_api::settings::{OandaApiMode, OandaSettings};
use crate::rate_limiter::RateLimiter;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use futures_util::TryStreamExt;
use lazy_static::lazy_static;
use rust_decimal::Decimal;
use serde_json::{Value};
use tokio::sync::mpsc::{Receiver, Sender};
use ff_standard_lib::standardized_types::accounts::{Account, AccountId, AccountInfo};
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::quote::Quote;
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::subscriptions::{Symbol, SymbolName};
use crate::oanda_api::get_requests::{oanda_account_summary, oanda_accounts_list, oanda_instruments_download};
use crate::oanda_api::instruments::OandaInstrument;
use crate::oanda_api::models::position::OandaPosition;
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
    pub rate_limiter: Arc<RateLimiter>,
    pub api_key: String,
    pub base_endpoint: String,
    pub stream_endpoint: String,
    pub instruments_map: DashMap<SymbolName, OandaInstrument>,
    pub accounts: Vec<Account>,
    pub account_info: DashMap<AccountId, AccountInfo>,
    pub positions: DashMap<AccountId, OandaPosition>,
    pub instrument_symbol_map: Arc<DashMap<String, Symbol>>,
    pub quote_feed_broadcasters: Arc<DashMap<SymbolName, broadcast::Sender<BaseDataEnum>>>,
    pub subscription_sender: Sender<Vec<SymbolName>>,
}

impl OandaClient {
    pub async fn send_rest_request(&self, endpoint: &str) -> Result<Response, Error> {
        let url = format!("{}{}", self.base_endpoint, endpoint);

        // Acquire a permit asynchronously. The permit will be automatically released when dropped.
        let _permit = self.rate_limiter.acquire().await;

        match self.client.get(&url).header("Authorization", format!("Bearer {}", self.api_key)).send().await {
            Ok(response) => {
                Ok(response)
            }
            Err(e) => {
                OANDA_IS_CONNECTED.store(false, Ordering::SeqCst);
                Err(e)
            }
        }
    }
}

/// Establishes a streaming connection to the specified endpoint suffix.
/// This method respects the `stream_limit` semaphore.
pub async fn establish_stream(client: &Arc<Client>, stream_endpoint: &str, stream_endpoint_suffix: &str, stream_limit: &Arc<Semaphore>, api_key: &str) -> Result<impl Stream<Item = Result<Bytes, reqwest::Error>>, Error> {
    let url = format!("{}{}", stream_endpoint, stream_endpoint_suffix);

    // Acquire a stream permit asynchronously.
    let _stream_permit = stream_limit.acquire().await.expect("Failed to acquire stream permit");

    // Make a GET request to the streaming endpoint
    let response: Response = client.get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send().await?;

    // Return the stream of bytes to the caller
    Ok(response.bytes_stream())
}

async fn handle_price_stream(
    client: Arc<Client>,
    instrument_symbol_map: Arc<DashMap<String, Symbol>>,
    quote_feed_broadcasters: Arc<DashMap<SymbolName, broadcast::Sender<BaseDataEnum>>>,
    mut subscription_receiver: Receiver<Vec<SymbolName>>,
    account: Account,
    stream_limit: Arc<Semaphore>,
    stream_endpoint: String,
    api_key: String
) {
    tokio::spawn(async move {
        let mut current_subscriptions = Vec::new();
        loop {
            tokio::select! {
                // Check for subscription updates
                Some(new_subscriptions) = subscription_receiver.recv() => {
                    // Only establish new stream if subscriptions changed
                    let mut cleaned_new_subscriptions = vec![];
                    for sub in new_subscriptions {
                        if let Some(instrument) = instrument_symbol_map.get(&sub) {
                            cleaned_new_subscriptions.push(instrument.name.clone());
                        }
                    }
                    if cleaned_new_subscriptions != current_subscriptions {
                        current_subscriptions = cleaned_new_subscriptions;
                        continue; // This will break the inner stream loop and create new stream
                    }
                }

                _ = async {
                    if current_subscriptions.is_empty() {
                        // Wait for subscriptions if none exist
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        return;
                    }
                    //accounts/<ACCOUNT>/pricing/stream?instruments=EUR_USD%2CUSD_CAD"
                    let suffix = format!("{}/pricing/stream?instruments={}", account.account_id, current_subscriptions.join("%2C"));

                    match establish_stream(&client, &stream_endpoint, &suffix, &stream_limit, &api_key).await {
                        Ok(mut stream) => {
                             OANDA_IS_CONNECTED.store(true, Ordering::SeqCst);
                            while let Ok(Some(chunk)) = stream.try_next().await {
                                if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                                    let json: Value = match serde_json::from_str(&text) {
                                        Ok(j) => j,
                                        Err(e) => {
                                            eprintln!("Failed to parse JSON: {}", e);
                                            continue;
                                        }
                                    };

                                    let raw_ask = match json["closeoutAsk"].as_str().ok_or("closeoutAsk") {
                                        Ok(ask) => ask,
                                        Err(e) => {
                                            eprintln!("Failed to parse closeoutAsk: {}", e);
                                            continue;
                                        }
                                    };
                                    let closeout_ask = match Decimal::from_str(raw_ask) {
                                        Ok(b) => b,
                                        Err(e) => {
                                            eprintln!("Failed to parse closeoutAsk: {}", e);
                                            continue;
                                        }
                                    };

                                    let raw_bid = match json["closeoutBid"].as_str().ok_or("closeoutBid") {
                                        Ok(bid) => bid,
                                        Err(e) => {
                                            eprintln!("Failed to parse closeoutBid: {}", e);
                                            continue;
                                        }
                                    };
                                    let closeout_bid = match Decimal::from_str(raw_bid) {
                                        Ok(b) => b,
                                        Err(e) => {
                                            eprintln!("Failed to parse closeoutBid: {}", e);
                                            continue;
                                        }
                                    };

                                     let raw_volume = match json["closeoutBid"].as_str().ok_or("liquidity") {
                                        Ok(vol) => vol,
                                        Err(e) => {
                                            eprintln!("Failed to parse closeoutBid: {}", e);
                                            continue;
                                        }
                                    };
                                    let volume = match Decimal::from_str(raw_volume) {
                                        Ok(b) => b,
                                        Err(e) => {
                                            eprintln!("Failed to parse closeoutBid: {}", e);
                                            continue;
                                        }
                                    };

                                    let instrument = match json["instrument"].as_str().ok_or("instrument") {
                                        Ok(i) => i,
                                        Err(e) => {
                                            eprintln!("Failed to parse instrument: {}", e);
                                            continue;
                                        }
                                    };

                                    let time_str = match json["time"].as_str().ok_or("time") {
                                        Ok(t) => t,
                                        Err(e) => {
                                            eprintln!("Failed to parse time: {}", e);
                                            continue;
                                        }
                                    };
                                    let time = match DateTime::parse_from_rfc3339(time_str) {
                                        Ok(t) => t.with_timezone(&Utc),
                                        Err(e) => {
                                            eprintln!("Failed to parse time: {}", e);
                                            continue;
                                        }
                                    };

                                    let symbol = match instrument_symbol_map.get(instrument) {
                                        Some(symbol) => symbol.clone(),
                                        None => {
                                            eprintln!("Symbol not found: {}", instrument);
                                            continue;
                                        }
                                    };

                                    // Broadcast if we have subscribers
                                    let mut dead_broadcast = false;
                                    if let Some(broadcaster) = quote_feed_broadcasters.get(&symbol.name) {
                                        let quote = BaseDataEnum::Quote(Quote::new(
                                            symbol.clone(),
                                            closeout_ask,
                                            closeout_bid,
                                            volume.clone(),
                                            volume,
                                            time.to_string()
                                        ));
                                        match broadcaster.send(quote) {
                                            Ok(_) => {}
                                            Err(_e) => {
                                                if broadcaster.receiver_count() == 0 {
                                                    dead_broadcast = true;
                                                }
                                            }
                                        }
                                    }
                                    if dead_broadcast {
                                        quote_feed_broadcasters.remove(&symbol.name);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Stream error: {}", e);
                             OANDA_IS_CONNECTED.store(false, Ordering::SeqCst);
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    }
                } => {}
            }
        }
    });
}

pub(crate) async fn oanda_init(options: ServerLaunchOptions) {
    if options.disable_oanda_server != 0 {
        return;
    }
    let path = options.data_folder.clone()
        .join("credentials")
        .join("oanda_credentials")
        .join("active")
        .join("oanda_credentials.toml");

    if !path.exists() {
        return;
    }

    let settings: OandaSettings = match OandaSettings::from_file(path) {
        Some(s) => s,
        None => {
            eprintln!("No oanda settings retrieved");
            return;
        }
    };

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
            OANDA_IS_CONNECTED.store(true, Ordering::SeqCst);
            client
        }
        Err(_) => {
            OANDA_IS_CONNECTED.store(false, Ordering::SeqCst);
            return;
        }
    };

    let client = Arc::new(client);

    let rate_limiter = RateLimiter::new(120, Duration::from_secs(1));
    let (sender, receiver) = tokio::sync::mpsc::channel(5);
    let mut oanda_client = OandaClient {
        client,
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
        handle_price_stream(oanda_client.client.clone(), oanda_client.instrument_symbol_map.clone(), oanda_client.quote_feed_broadcasters.clone(), receiver, account.clone(), stream_limit.clone(), oanda_client.stream_endpoint.clone(), oanda_client.api_key.clone()).await;
    }
    let _ = OANDA_CLIENT.set(Arc::new(oanda_client));
}

/*
pub async fn auto_update_timer(data_folder: PathBuf, multi_bar: Arc<Mutex<MultiProgress>>) {
    // Instruments update task
    let data_folder_clone = data_folder.clone(); // Clone for use in the async block
    tokio::spawn(async move {
        let instruments_updates_duration = Duration::from_secs(60); // Set this to the desired interval
        let mut instruments_update_interval = tokio::time::interval(instruments_updates_duration);
        loop {
            instruments_update_interval.tick().await;
            let account_id = oanda_accounts_list(get_oanda_client_ref()).await.unwrap().accounts[0].clone();
            let oanda_client_ref = get_oanda_client_ref().clone();
            let folder = data_folder_clone.clone(); // Clone for use inside the loop
            tokio::spawn(async move {
                oanda_instruments_download(&oanda_client_ref, &account_id, folder).await;
            });
        }
    });

    // minute updates task
    let data_folder_clone = data_folder.clone(); // Clone for use in the async block
    let multibar = multi_bar.clone();
    tokio::spawn(async move {
        wait_until_next_minute().await;
        let minute_resolution_updates_duration = Duration::from_secs(60); // Set this to the desired interval
        let mut minute_update_interval = tokio::time::interval(minute_resolution_updates_duration);
        loop {
            let multibar = multibar.clone();
            minute_update_interval.tick().await;
            let oanda_client_ref = get_oanda_client_ref().clone();
            let folder = data_folder_clone.clone(); // Clone for use inside the loop
            tokio::spawn(async move {
                let multibar = multibar.clone();
                match oanda_update_all_historical_data(folder, oanda_client_ref, Resolution::Seconds(5), BaseDataType::QuoteBars, multibar.clone()).await {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Error: {:?}", e);
                    }
                }
            });
        }
    });


    let data_folder_clone = data_folder.clone(); // Clone for use in the async block
    let multibar = multi_bar.clone();
    tokio::spawn(async move {
        wait_until_next_hour().await;
        let hourly_resolution_updates_duration = Duration::from_secs(60*60); // Set this to the desired interval
        let mut hourly_update_interval = tokio::time::interval(hourly_resolution_updates_duration);
        loop {
            let multibar = multibar.clone();
            hourly_update_interval.tick().await;
            let oanda_client_ref = get_oanda_client_ref().clone();
            let folder = data_folder_clone.clone(); // Clone for use inside the loop
            tokio::spawn(async move {
                let multibar = multibar.clone();
                oanda_update_all_historical_data(folder, oanda_client_ref, Resolution::Hours(1), BaseDataType::QuoteBars, multibar.clone()).await;
            });
        }
    });
}*/

//for deserializing positions
/*
let mut positions = vec![];
                for oanda_position in summary.positions {
                    let side = match oanda_position.long.units > dec!(0) {
                        true => PositionSide::Long,
                        false => PositionSide::Short
                    };

                    let (quantity, average_price, open_pnl) = match side {
                        PositionSide::Long => {
                            match oanda_position.long.average_price {
                                Some(price) => (oanda_position.long.units, price, oanda_position.long.unrealized_pl),
                                None => {
                                    eprintln!("Long position has no average price");
                                    continue;
                                }
                            }
                        }
                        PositionSide::Short => {
                            match oanda_position.short.average_price {
                                Some(price) => (oanda_position.short.units, price, oanda_position.short.unrealized_pl),
                                None => {
                                    eprintln!("Short position has no average price");
                                    continue;
                                }
                            }
                        }
                    };

                    let symbol = fund_forge_formatted_symbol_name(&oanda_position.instrument);

                    if let Some(instrument) = oanda_client.instruments.get(&symbol) {
                        let tick_size = dec!(1) / dec!(10).powi(instrument.display_precision as i64);
                        let info = SymbolInfo {
                            symbol_name: symbol.clone(),
                            pnl_currency: summary.currency, //todo need to do dynamically
                            value_per_tick: dec!(1), //todo might need a hard coded list, cant find dynamic info
                            tick_size,
                            decimal_accuracy: instrument.pip_location,
                        };

                        let guid = Uuid::new_v4();

                        // Return the generated position ID with both readable prefix and GUID
                        let id = format!(
                            "{}-{}",
                            side,
                            guid.to_string()
                        );

                        let position = Position {
                            symbol_name: symbol.clone(),
                            symbol_code: symbol.clone(),
                            account: Account { brokerage: Brokerage::Oanda, account_id: account.account_id.clone() },
                            side,
                            open_time: Utc::now().to_string(),
                            quantity_open: quantity,
                            quantity_closed: dec!(0),
                            close_time: None,
                            average_price,
                            open_pnl,
                            booked_pnl: dec!(0),
                            highest_recoded_price: Default::default(),
                            lowest_recoded_price: Default::default(),
                            average_exit_price: None,
                            is_closed: false,
                            position_id: id,
                            symbol_info: info,
                            pnl_currency: summary.currency, //todo dynamically get somehow for each position
                            tag: "External Position".to_string(),
                        };
                        positions.push(position);
                    }
                }

*/
