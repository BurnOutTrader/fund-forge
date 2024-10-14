use std::error::Error;
use std::fs;
use std::path::PathBuf;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol};
use std::fs::File;
use std::io::{self, BufRead};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use ff_standard_lib::server_features::database::HybridStorage;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::quote::Quote;
use ff_standard_lib::standardized_types::enums::MarketType;
use ff_standard_lib::standardized_types::resolution::Resolution;

/// to parse free testing data from https://www.histdata.com/
/// 1. Put all the csv data into one folder
/// 2. Configure the subscription properties and directory path
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let your_folder_path: String = "/Users/kevmonaghan/Downloads".to_string();
    let symbol_name: String = "AUD-CAD".to_string();

    let symbol = Symbol {
        name: symbol_name, //CHANGE THIS
        market_type: MarketType::Forex,
        data_vendor: DataVendor::Test,
    };

    let dir_path = format!("{}/{}", your_folder_path, symbol.name); //CHANGE THIS
    let base_data_path = PathBuf::from(format!("{}/data/parsed", your_folder_path)); //CHANGE THIS

    if !base_data_path.exists() {
        fs::create_dir_all(&base_data_path)?;
    }

    // Define subscription
    let subscription = DataSubscription {
        symbol: symbol.clone(),
        resolution: Resolution::Instant,
        base_data_type: BaseDataType::Quotes,
        market_type: MarketType::Forex,
        candle_type: None,
    };

    let mut data : Vec<BaseDataEnum> = Vec::new();
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
                BaseDataType::QuoteBars => {},
                BaseDataType::Candles => {},
                BaseDataType::Fundamentals => {},
            }
        }
    }

    //new way
    let path = PathBuf::from(your_folder_path);
    let storage = HybridStorage::new(path);
    storage.save_data_bulk(data).await.unwrap();

    Ok(())
}

///to parse 'Tick' data from https://www.histdata.com/. in fund forge we use this kind of data as quotes.
fn load_csv_quotes(file_path: &str, symbol: Symbol) -> Result<Vec<BaseDataEnum>, Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let reader = io::BufReader::new(file);

    let mut quotes = Vec::new();
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
            bid: parts.get(1).expect("REASON").parse::<Decimal>()?,
            ask_volume: dec!(0.0),
            ask: parts.get(2).expect("REASON").parse::<Decimal>()?,
            time: utc_datetime.clone().to_string(),
            bid_volume: dec!(0.0),
        };
        quotes.push(BaseDataEnum::Quote(quote));
    }
    Ok(quotes)
}
