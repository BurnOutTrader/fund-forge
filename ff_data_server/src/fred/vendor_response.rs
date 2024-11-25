use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use chrono_tz::America::Chicago;
use chrono_tz::Tz;
use fred_rs::client::FredClient;
use fred_rs::series::observation::{Builder, Units, Frequency, Response};
use indicatif::{ProgressBar, ProgressStyle};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use tokio::sync::{OnceCell};
use ff_standard_lib::messages::data_server_messaging::{DataServerResponse, FundForgeError};
use ff_standard_lib::product_maps::fred::models::{fred_release_time_of_day, get_fed_api_country_format, FredDataSetEnum};
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::fundamental::Fundamental;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::enums::{MarketType, StrategyMode};
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol, SymbolName};
use ff_standard_lib::StreamName;
use crate::server_features::database::hybrid_storage::{DATA_STORAGE};
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
            client,
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


    async fn update_historical_data(&self, symbol: Symbol, base_data_type: BaseDataType, resolution: Resolution, from: DateTime<Utc>, to: DateTime<Utc>, from_back: bool, progress_bar: ProgressBar) -> Result<(), FundForgeError> {
        const UNITS_ARR: [Units; 9] = [
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

        let fred_data = symbol.name.clone();
        let symbol = Symbol::new(
            fred_data.clone(),
            DataVendor::Fred,
            MarketType::Fundamentals
        );

        let length = (to - from).num_days() as u64;
        progress_bar.set_length(length);
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template("{prefix:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg} ({eta})")
                .unwrap()
                .progress_chars("=>-")
        );
        progress_bar.set_message(format!("Starting Download for ({}: {}) from: {}, to {}: Message will not change if no data found", resolution, base_data_type, from, to));
        let increment = match resolution {
            Resolution::Day => 1,
            Resolution::Week => 7,
            Resolution::Month => 30,
            Resolution::Quarter => 90,
            Resolution::Year => 365,
            _ => 1,
        };

        let database = DATA_STORAGE.get().unwrap();
        let mut data_map = BTreeMap::new();

        let (symbol_name, country_code) = match symbol.name.split('-').collect::<Vec<&str>>().as_slice() {
            [symbol_name, country_code] => (symbol_name.to_string(), country_code.to_string()),
            _ => {
                return Err(FundForgeError::ClientSideErrorDebug("Invalid symbol format, symbol name should be formatted GDP-USA".to_string()));
            }
        };

        let country_name = match get_fed_api_country_format(&country_code) {
            Some(country_code) => country_code,
            None => {
                return Err(FundForgeError::ClientSideErrorDebug("Invalid country code".to_string()));
            }
        };

        let fred_data_set = FredDataSetEnum::from_str(&fred_data).unwrap();

        let from_chicago_time = Chicago.from_utc_datetime(&from.naive_utc());
        let to_chicago_time = Chicago.from_utc_datetime(&to.naive_utc());
        let (release_hour, release_minutes) = fred_release_time_of_day(fred_data_set);

        for unit in UNITS_ARR.iter() {
            let mut last_date: DateTime<Tz> = from_chicago_time;
           while last_date < to_chicago_time {
               let from_string = last_date.format("%Y-%m-%d").to_string();

               // Set the arguments for the builder
               let mut builder = Builder::new();
               builder
                   .observation_start(&from_string)
                   .units(unit.clone())
                   .frequency(resolution_to_frequency(resolution).unwrap());


               // Make the request and pass in the builder to apply the arguments
               let resp: Response = match self.client.series_observation(&fred_data, Some(builder)) {
                   Ok(resp) => resp,
                   Err(msg) => {
                       println!("{}", msg);
                       continue;
                   },
               };

               if resp.observations.is_empty() {
                   if last_date >= to || last_date >= from {
                       break;
                   }
                   continue;
               }

               progress_bar.set_message(format!("Downloaded: {} Data Points for ({}: {}) from: {}, to {}", resp.observations.len(), resolution, base_data_type, from_chicago_time.to_utc(), to_chicago_time.to_utc()));

               // Print the response
               for data in resp.observations {
                   let time = NaiveDate::parse_from_str(&data.date, "%Y-%m-%d")
                       .expect("Invalid date format")
                       .and_hms_opt(7, 30, 0).unwrap();

                   let ct_time = Chicago.from_local_datetime(&time)
                       .earliest().unwrap();

                   let utc_time = ct_time.to_utc();

                   let value = match f64::from_str(&data.value) {
                       Ok(value) => value,
                       Err(_) => continue,
                   };

                   let values = BTreeMap::new();
                   if !data_map.contains_key(&utc_time) {
                       let fundamental = Fundamental::new(
                           symbol.clone(),
                           utc_time.to_string(),
                           frequency_to_resolution(resolution_to_frequency(resolution).unwrap()).unwrap(),
                           values,
                           None,
                           None,
                           fred_data.to_string(),
                       );
                       data_map.insert(utc_time, fundamental);
                   }
                   if let Some(fundamental) = data_map.get_mut(&utc_time) {
                       let value = Decimal::from_f64(value).unwrap();
                       fundamental.values.insert(units_to_string(unit.clone()), value);
                       //println!("{:?}", fundamental);
                   }

                   progress_bar.inc(increment);
                   last_date = ct_time;
               }
           }
        }
        if !data_map.is_empty() {
            let data_points: Vec<BaseDataEnum> = data_map.into_iter().map(|(_, v)| BaseDataEnum::Fundamental(v)).collect();
            match database.save_data_bulk(data_points).await {
                Ok(_) => {},
                Err(e) => {
                    return Err(FundForgeError::ServerErrorDebug(e.to_string()));
                }
            }
        }
        progress_bar.finish_and_clear();
        Ok(())
    }
}