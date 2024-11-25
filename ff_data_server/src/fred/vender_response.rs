use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use chrono_tz::America::Chicago;
use fred_rs::client::FredClient;
use fred_rs::series::observation::{Builder, Units, Frequency, Response};
use indicatif::ProgressBar;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use tokio::sync::OnceCell;
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::fundamental::Fundamental;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::enums::{Bias, MarketType, StrategyMode};
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol, SymbolName};
use ff_standard_lib::StreamName;
use crate::fred::models::FredDataSet;
use crate::server_features::server_side_datavendor::VendorApiResponse;

static FRED_CLIENT: OnceCell<Arc<FredApiClient>> = OnceCell::const_new();

pub fn get_fred_client() -> Option<Arc<FredApiClient>> {
    match FRED_CLIENT.get() {
        None => None,
        Some(c) => Some(c.clone())
    }
}

pub fn init_fred_client(data_folder: PathBuf) -> Result<(), FundForgeError> {
    let fred_client = FredApiClient::new(data_folder)?;
    FRED_CLIENT.set(Arc::new(fred_client)).unwrap();
    Ok(())
}

pub fn parse_fred_api_key(data_folder: PathBuf) -> Option<String> {

    let file_path = data_folder.join("credentials/fred_credentials/active/fred_credentials.toml");

    if !Path::new(&file_path).exists() {
        return None;
    }
    let credentials = match std::fs::read_to_string(file_path) {
        Ok(credentials) => credentials,
        Err(_) => return None
    };
    let credentials: toml::Value = match toml::from_str(&credentials) {
        Ok(credentials) => credentials,
        Err(_) => return None
    };
    match credentials["api_key"].as_str() {
        Some(api_key) => {
            //println!("FRED API key found: {}", api_key);
            Some(api_key.to_string())
        },
        None => None
    }
}

pub fn frequency_to_resolution(frequency: Frequency) -> Option<Resolution> {
    match frequency {
        Frequency::D => Some(Resolution::Day),
        Frequency::W => Some(Resolution::Week),
        Frequency::M => Some(Resolution::Month),
        Frequency::Q => Some(Resolution::Quarter),
        Frequency::A => Some(Resolution::Year),
        _ => None
    }
}

pub fn resolution_to_frequency(resolution: Resolution) -> Option<Frequency> {
    match resolution {
        Resolution::Day => Some(Frequency::D),
        Resolution::Week => Some(Frequency::W),
        Resolution::Month => Some(Frequency::M),
        Resolution::Quarter => Some(Frequency::Q),
        Resolution::Year => Some(Frequency::A),
        _ => None
    }
}

pub fn units_to_string(units: Units) -> String {
    match units {
        Units::LIN => "Linear".to_string(),
        Units::CHG => "Change".to_string(),
        Units::CH1 => "Year Over Year Change".to_string(),
        Units::PCH => "Percent Change".to_string(),
        Units::PC1 => "Year Over Year Percent Change".to_string(),
        Units::PCA => "Compounded Annual Rate of Change".to_string(),
        Units::CCH => "Continuously Compounded Rate of Change".to_string(),
        Units::CCA => "Continuously Compounded Annual Rate of Change".to_string(),
        Units::LOG => "Logarithm".to_string(),
    }
}

#[derive(Debug)]
pub struct FredApiClient {
    client: FredClient,
}

impl FredApiClient {
    pub fn new(data_folder: PathBuf) -> Result<Self, FundForgeError> {
        let api_key = parse_fred_api_key(data_folder).unwrap();
        let mut client = FredClient::new().unwrap();
        client.with_key(&api_key);
        Ok(Self {
            client
        })
    }
}

#[allow(unused_variables)]
#[async_trait]
impl VendorApiResponse for FredApiClient {
    async fn symbols_response(&self, mode: StrategyMode, stream_name: StreamName, market_type: MarketType, time: Option<DateTime<Utc>>, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn resolutions_response(&self, mode: StrategyMode, stream_name: StreamName, market_type: MarketType, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn markets_response(&self, mode: StrategyMode, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn decimal_accuracy_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn tick_size_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn data_feed_subscribe(&self, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
        todo!()
    }

    async fn data_feed_unsubscribe(&self, stream_name: StreamName, subscription: DataSubscription) -> DataServerResponse {
        todo!()
    }

    async fn base_data_types_response(&self, mode: StrategyMode, stream_name: StreamName, callback_id: u64) -> DataServerResponse {
        todo!()
    }

    async fn logout_command_vendors(&self, stream_name: StreamName) {
        todo!()
    }

    async fn session_market_hours_response(&self, mode: StrategyMode, stream_name: StreamName, symbol_name: SymbolName, date_time: DateTime<Utc>, callback_id: u64) -> DataServerResponse {
        todo!()
    }


    async fn update_historical_data(&self, symbol: Symbol, base_data_type: BaseDataType, resolution: Resolution, from: DateTime<Utc>, to: DateTime<Utc>, from_back: bool, progress_bar: ProgressBar, is_bulk_download: bool) -> Result<(), FundForgeError> {
        todo!()
    }
}

#[tokio::test]
async fn test_fred_client() {
    let data_folder = std::path::PathBuf::from("/Volumes/KINGSTON/data");
    let api_key = parse_fred_api_key(data_folder).unwrap();
    let mut client = FredClient::new().unwrap();
    client.with_key(&api_key);
    // Create the argument builder


    let resolution = Resolution::Year;
    let units_vec = vec![
        Units::LIN,
        Units::CHG,
        Units::CH1,
        Units::PCH,
        Units::PC1,
        Units::PCA,
        Units::CCH,
        Units::CCA,
        Units::LOG,
    ];
    let fred_data = FredDataSet::GrossNationalProductPerCapita;

    let symbol = Symbol::new(
        fred_data.to_string(),
        DataVendor::Fred,
        MarketType::Fundamentals
    );

    let mut data_map = BTreeMap::new();

    for unit in units_vec {
        let mut builder = Builder::new();
        // Set the arguments for the builder
        builder
            .observation_start("1930-01-01")
            .units(unit)
            .frequency(resolution_to_frequency(resolution).unwrap());


        // Make the request and pass in the builder to apply the arguments
        let resp: Response = match client.series_observation(&fred_data.to_string(), Some(builder)) {
            Ok(resp) => resp,
            Err(msg) => {
                println!("{}", msg);
                return
            },
        };

        // Print the response
        for data in resp.observations {
            let time = NaiveDate::parse_from_str(&data.date, "%Y-%m-%d")
                .expect("Invalid date format")
                .and_hms_opt(0, 0, 0).unwrap();

            let ct_time = Chicago.from_local_datetime(&time)
                .earliest().unwrap();

            let time = ct_time.to_utc();

            let value = f64::from_str(&data.value).unwrap();

            let bias = if value > 0.0 {
                Bias::Bullish
            } else if value < 0.0 {
                Bias::Bearish
            } else {
                Bias::Neutral
            };

            let values = BTreeMap::new();
            if !data_map.contains_key(&time) {
                let fundamental = Fundamental::new(
                    symbol.clone(),
                    time.to_string(),
                    frequency_to_resolution(resolution_to_frequency(resolution).unwrap()).unwrap(),
                    values,
                    None,
                    None,
                    fred_data.to_string(),
                    bias,
                );
                data_map.insert(time, fundamental);
            }
            if let Some(fundamental) = data_map.get_mut(&time) {
                let value = Decimal::from_f64(value).unwrap();
                fundamental.values.insert(units_to_string(unit), value);
                //println!("{:?}", fundamental);
            }
        }
    }
    let (_, first_value) = data_map.iter().next().unwrap();
    println!("{:?}", first_value);
}