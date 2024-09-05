use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use ff_standard_lib::standardized_types::subscriptions::{CandleType, DataSubscription, Symbol};
use std::fs::File;
use std::io::{self, BufRead};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use ff_standard_lib::apis::vendor::DataVendor;
use ff_standard_lib::helpers::decimal_calculators::round_to_tick_size;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::candle::Candle;
use ff_standard_lib::standardized_types::base_data::quote::Quote;
use ff_standard_lib::standardized_types::enums::{MarketType, Resolution};
use ff_standard_lib::standardized_types::subscriptions::CandleType::CandleStick;

/// to parse free testing data from https://www.histdata.com/
/// 1. Put all the csv data into one folder
/// 2. Configure the subscription properties and directory path
fn main() -> Result<(), Box<dyn Error>> {

    let symbol = Symbol {
        name: "CAD-JPY".to_string(),
        market_type: MarketType::Forex,
        data_vendor: DataVendor::Test,
    };

    // Define subscription
    let subscription = DataSubscription {
        symbol: symbol.clone(),
        resolution: Resolution::Instant,
        base_data_type: BaseDataType::Quotes,
        market_type: MarketType::Forex,
        candle_type: None,
    };


    let dir_path = format!("/Users/kevmonaghan/Downloads/{}", symbol.name);
    let base_data_path = PathBuf::from("/Users/kevmonaghan/Downloads/data/parsed");

    if !base_data_path.exists() {
        fs::create_dir_all(&base_data_path)?;
    }


/*    let subscription_quotes = DataSubscription {
        symbol: symbol.clone(),
        resolution: Resolution::Instant,
        base_data_type: BaseDataType::Quotes,
        market_type: MarketType::Forex,
        candle_type: None,
    };*/


    let mut data : BTreeMap<DateTime<Utc>, BaseDataEnum> = BTreeMap::new();
    for entry in fs::read_dir(dir_path)? {
        // Iterate through each file in the directory
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("csv") {
            let file_path = path.to_string_lossy().into_owned();
            println!("Processing file: {}", file_path);
            match &subscription.base_data_type {
                BaseDataType::Quotes => {
                    // Load and parse CSV
                    let ticks = load_csv_quotes(&file_path, symbol.clone())?;
                    data.extend(ticks);
                },
                BaseDataType::Ticks => {},
                BaseDataType::TradePrices => {},
                BaseDataType::QuoteBars => {},
                BaseDataType::Candles => {
                    let candles = load_csv_candles(&file_path, symbol.clone(), Resolution::Minutes(1))?;
                    data.extend(candles);
                },
                BaseDataType::Fundamentals => {},
            }
        }
    }


    // Save formatted data
    BaseDataEnum::format_and_save(&base_data_path, data, &subscription)
        .map_err(|e| {
            println!("Failed to save data for file {:?}: {}", base_data_path, e);
            e
        })?;

    Ok(())
}


///to parse 'Candle' data from https://www.histdata.com/.
/// Note this is only for data with decimal accuracy of 5 or range will be wrong.
fn load_csv_candles(file_path: &str, symbol: Symbol, resolution: Resolution) -> Result<BTreeMap<DateTime<Utc>, BaseDataEnum>, Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let reader = io::BufReader::new(file);

    let mut quotes = BTreeMap::new();
    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split(',').collect();

        let time = parts.get(0).expect("REASON").to_string();
        let format = "%Y%m%d %H%M%S";
        let naive_datetime = NaiveDateTime::parse_from_str(&time, format)?;

        // Convert NaiveDateTime to DateTime<Utc>
        let utc_datetime: DateTime<Utc> = Utc.from_utc_datetime(&naive_datetime);
        let high =parts.get(2).expect("REASON").parse::<f64>()?;
        let low = parts.get(3).expect("REASON").parse::<f64>()?;
        let candle = Candle {
            symbol: symbol.clone(),
            open: parts.get(1).expect("REASON").parse::<f64>()?,
            range: round_to_tick_size(high - low, 0.00001),
            high,
            low,
            close: parts.get(4).expect("REASON").parse::<f64>()?,
            volume: parts.get(5).expect("REASON").parse::<f64>()?,
            time: utc_datetime.to_string(),
            is_closed: true,
            resolution,
            candle_type: CandleType::CandleStick,
        };
        quotes.insert(utc_datetime, BaseDataEnum::Candle(candle));
    }
    Ok(quotes)
}



///to parse 'Tick' data from https://www.histdata.com/. in fund forge we use this kind of data as quotes.
fn load_csv_quotes(file_path: &str, symbol: Symbol) -> Result<BTreeMap<DateTime<Utc>, BaseDataEnum>, Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let reader = io::BufReader::new(file);

    let mut quotes = BTreeMap::new();
    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split(',').collect();

        let time = parts.get(0).expect("REASON").to_string();
        let format = "%Y%m%d %H%M%S%f"; // Format including milliseconds
        let naive_datetime = NaiveDateTime::parse_from_str(&time, format).unwrap();

        // Convert NaiveDateTime to DateTime<Utc>
        let utc_datetime: DateTime<Utc> = Utc.from_utc_datetime(&naive_datetime);

        let quote = Quote {
            symbol: symbol.clone(),
            bid: parts.get(1).expect("REASON").parse::<f64>()?,
            ask_volume: 0.0,
            ask: parts.get(2).expect("REASON").parse::<f64>()?,
            time: utc_datetime.to_string(),
            bid_volume: 0.0,
            book_level: 0,
        };
        quotes.insert(utc_datetime, BaseDataEnum::Quote(quote));
    }
    Ok(quotes)
}
