use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let dir_path = "/Users/kevmonaghan/Downloads/AUD-CAD";
    let base_data_path = PathBuf::from("/Users/kevmonaghan/Downloads/data/parsed");

    if !base_data_path.exists() {
        fs::create_dir_all(&base_data_path)?;
    }

    let symbol = Symbol {
        name: "AUD-CAD".to_string(),
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


    let mut data : BTreeMap<DateTime<Utc>, BaseDataEnum> = BTreeMap::new();
    // Iterate through each file in the directory
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();

        // Check if the path is a file and has a .csv extension
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("csv") {
            let file_path = path.to_string_lossy().into_owned();
            println!("Processing file: {}", file_path);

            // Load and parse CSV
            let ticks = load_csv(&file_path, symbol.clone())?;

            data.extend(ticks);
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


use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol};
use std::fs::File;
use std::io::{self, BufRead};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use ff_standard_lib::apis::vendor::DataVendor;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::quote::Quote;
use ff_standard_lib::standardized_types::enums::{MarketType, Resolution};

fn load_csv(file_path: &str, symbol: Symbol) -> Result<BTreeMap<DateTime<Utc>, BaseDataEnum>, Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let reader = io::BufReader::new(file);

    let mut quotes = BTreeMap::new();
    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split(',').collect();

        let time = parts.get(0).expect("REASON").to_string();
        let format = "%Y%m%d %H%M%S%3f"; // Format including milliseconds
        let naive_datetime = NaiveDateTime::parse_from_str(&time, format)?;

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
