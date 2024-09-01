use std::collections::HashMap;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = "/Users/kevmonaghan/Downloads/data/AUD-USD_T_LAST_202301.csv";

    let symbol = Symbol {
        name: "AUD-USD".to_string(),
        market_type: MarketType::Forex,
        data_vendor: DataVendor::Test,
    };

    let ticks = load_csv(file_path, symbol.clone())?;

    let data_map = store_data(ticks);
    let base_data_path = PathBuf::from("/Users/kevmonaghan/Downloads/data/parsed");

    if !base_data_path.exists() {
        std::fs::create_dir_all(&base_data_path)?;
    }

    let subscription = DataSubscription {
        symbol: symbol.clone(),
        resolution: Resolution::Ticks(1),
        base_data_type: BaseDataType::Ticks,
        market_type: MarketType::Forex,
    };

    BaseDataEnum::format_and_save(&base_data_path, data_map, &subscription).unwrap();

    Ok(())
}

use ff_standard_lib::standardized_types::base_data::tick::Tick;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

fn load_csv(file_path: &str, symbol: Symbol) -> Result<Vec<Tick>, Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let reader = io::BufReader::new(file);

    let mut ticks = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split(';').collect();
        if parts.len() < 3 {
            println!("Skipping line: {:?}", line);
            continue; // Skip lines with insufficient data
        }

        let time = parts[0].to_string();
        let format = "%Y%m%d %H%M%S"; // Format corresponding to "YYYYMMDD HHMMSS"
        let naive_datetime = NaiveDateTime::parse_from_str(&time, format)?;

        // Convert NaiveDateTime to DateTime<Utc>
        let utc_datetime: DateTime<Utc> = Utc.from_utc_datetime(&naive_datetime);

        let tick = Tick {
            symbol: symbol.clone(),
            price: parts[1].parse::<f64>()?,
            time: utc_datetime.to_string(),
            volume: parts[2].parse::<i64>()? as f64,
        };
        ticks.push(tick);
    }
    Ok(ticks)
}

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use ff_standard_lib::apis::vendor::DataVendor;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::enums::{MarketType, Resolution};
use std::collections::BTreeMap;
use std::str::FromStr;

fn store_data(ticks: Vec<Tick>) -> BTreeMap<DateTime<Utc>, BaseDataEnum> {
    let mut map = BTreeMap::new();

    for tick in ticks {
        let datetime = DateTime::from_str(&tick.time).unwrap();
        map.insert(datetime, BaseDataEnum::Tick(tick));
    }

    map
}
