use reqwest::{Client, Error, Response};
use std::time::{Duration, Instant};
use futures::stream::{Stream};
use tokio::sync::{OnceCell, Semaphore};
use std::sync::Arc;
use tokio::time::sleep;
use crate::oanda_api::settings::{OandaApiMode, OandaSettings};
use crate::rate_limiter::RateLimiter;
use bytes::Bytes;
use chrono::Utc;
use dashmap::DashMap;
use rust_decimal::MathematicalOps;
use rust_decimal_macros::dec;
use uuid::Uuid;
use ff_standard_lib::helpers::converters::fund_forge_formatted_symbol_name;
use ff_standard_lib::standardized_types::accounts::{Account, AccountId, AccountInfo};
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use ff_standard_lib::standardized_types::enums::PositionSide;
use ff_standard_lib::standardized_types::position::Position;
use ff_standard_lib::standardized_types::subscriptions::SymbolName;
use ff_standard_lib::standardized_types::symbol_info::SymbolInfo;
use crate::oanda_api::get_requests::{get_oanda_account_details, oanda_accounts_list, oanda_instruments_download};
use crate::oanda_api::instruments::OandaInstrument;
use crate::oanda_api::models::position::OandaPosition;
use crate::ServerLaunchOptions;

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

pub(crate) async fn oanda_init(options: ServerLaunchOptions) {
    if options.disable_oanda_server != 0 {
        return;
    }
    let path = options.data_folder.clone()
        .join("credentials")
        .join("oanda_credentials")
        .join("active")
        .join("oanda_settings.toml");

    let settings: OandaSettings = match OandaSettings::from_file(path) {
        Some(s) => s,
        None => {
            eprintln!("No oanda settings retrieved");
            return;
        }
    };

    let client = Arc::new(Client::builder()
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", settings.api_key)).unwrap(),
            );
            headers
        })
        .http2_keep_alive_while_idle(true)
        .build().unwrap());

    let rate_limiter = RateLimiter::new(120, Duration::from_secs(1));
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
        stream_limit: Arc::new(Semaphore::new(20)),
        instruments: Default::default(),
        accounts: vec![],
        account_info: Default::default(),
        positions: Default::default(),
    };
    match oanda_accounts_list(&oanda_client).await {
        Ok(accounts) => oanda_client.accounts = accounts.clone(),
        Err(e) => eprintln!("Error getting accounts: {:?}", e)
    };
    if let Some(account) = oanda_client.accounts.get(0) {
        let instruments = oanda_instruments_download(&oanda_client, &account.account_id).await.unwrap_or_else(|| vec![]);
        for instrument in instruments {
            oanda_client.instruments.insert(instrument.symbol_name.clone(), instrument);
        }
    }
    for account in &oanda_client.accounts {
        match get_oanda_account_details(&oanda_client, &account.account_id).await {
            Ok(summary) => {
                let mut positions = vec![];
                for oanda_position in summary.positions {
                    let side = match oanda_position.long.units > dec!(0) {
                        true => PositionSide::Long,
                        false => PositionSide::Short
                    };

                    let (quantity, average_price, open_pnl) = match side {
                        PositionSide::Long => {
                            (oanda_position.long.units, oanda_position.long.average_price, oanda_position.long.unrealized_pl)
                        }
                        PositionSide::Short => {
                            (oanda_position.short.units, oanda_position.short.average_price, oanda_position.short.unrealized_pl)
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
                };
                oanda_client.account_info.insert(account.account_id.clone(), info);
            }
            Err(e) => eprintln!("Error getting oanda account info: {}", e)
        }
    }
    let _ = OANDA_CLIENT.set(Arc::new(oanda_client));
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
    pub stream_limit: Arc<Semaphore>,
    pub instruments: DashMap<SymbolName, OandaInstrument>,
    pub accounts: Vec<Account>,
    pub account_info: DashMap<AccountId, AccountInfo>,
    pub positions: DashMap<AccountId, OandaPosition>
}

impl OandaClient {
    pub async fn send_rest_request(&self, endpoint: &str) -> Result<Response, Error> {
        let url = format!("{}{}", self.base_endpoint, endpoint);

        // Acquire a permit asynchronously. The permit will be automatically released when dropped.
        let _permit = self.rate_limiter.acquire().await;

        let response = self.client.get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        Ok(response)
    }

    /// Establishes a streaming connection to the specified endpoint suffix.
    /// This method respects the `stream_limit` semaphore.
    pub async fn establish_stream(&self, stream_endpoint_suffix: &str) -> Result<impl Stream<Item = Result<Bytes, reqwest::Error>>, Error> {
        let url = format!("{}{}", self.stream_endpoint, stream_endpoint_suffix);

        // Acquire a stream permit asynchronously.
        let _stream_permit = self.stream_limit.acquire().await.expect("Failed to acquire stream permit");

        // Make a GET request to the streaming endpoint
        let response: Response = self.client.get(&url)
            .header("Authorization", format!("Bearer {}", &self.api_key))
            .send().await?;

        // Return the stream of bytes to the caller
        Ok(response.bytes_stream())
    }
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
#[allow(dead_code)]
async fn wait_until_next_minute() -> Instant {
    let now = Instant::now();
    let secs_since_the_minute = now.elapsed().as_secs() % 60;
    let delay = 60 - secs_since_the_minute;
    sleep(Duration::from_secs(delay)).await;
    Instant::now()
}
#[allow(dead_code)]
async fn wait_until_next_hour() -> Instant {
    let now = Instant::now();
    let secs_since_the_hour = now.elapsed().as_secs() % 3600;
    let delay = 3600 - secs_since_the_hour;
    sleep(Duration::from_secs(delay)).await;
    Instant::now()
}
