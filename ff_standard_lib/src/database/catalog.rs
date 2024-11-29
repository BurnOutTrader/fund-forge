use std::{fs, io};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use chrono::{DateTime, Utc};
use crate::database::hybrid_storage::HybridStorage;
use crate::product_maps::rithmic::maps::get_futures_exchange;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::datavendor_enum::DataVendor;
use crate::standardized_types::enums::MarketType;
use crate::standardized_types::resolution::Resolution;
use crate::standardized_types::subscriptions::Symbol;

impl HybridStorage {
    pub async fn catalog_available_data(&self, export_path: PathBuf) -> io::Result<()> {
        let mut available_data = Vec::new();
        let rithmic_base = self.base_path.join("Rithmic").join("Futures");

        println!("Checking base path: {:?}", self.base_path);
        println!("Checking rithmic path: {:?}", rithmic_base);
        println!("rithmic_base exists: {}", rithmic_base.exists());

        if !rithmic_base.exists() {
            println!("Parent directory contents:");
            if let Ok(entries) = fs::read_dir(&self.base_path) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        println!("  {:?}", entry.path());
                    }
                }
            }
            return Err(io::Error::new(io::ErrorKind::NotFound, "No Rithmic data directory found"));
        }

        // Walk through symbols
        for symbol_entry in fs::read_dir(&rithmic_base)? {
            let symbol_entry = symbol_entry?;
            let symbol_name = symbol_entry.file_name().to_string_lossy().to_string();

            // Get exchange from symbol name, skip if not recognized
            let exchange = match get_futures_exchange(&symbol_name) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Skipping symbol {}: {}", symbol_name, e);
                    continue;
                }
            };
            let market_type = MarketType::Futures(exchange);
            let symbol = Symbol::new(
                symbol_name.clone(),
                DataVendor::Rithmic,
                market_type.clone()
            );

            // Walk through resolutions
            for resolution_entry in fs::read_dir(symbol_entry.path())? {
                let resolution_entry = resolution_entry?;
                let resolution = Resolution::from_str(
                    resolution_entry.file_name().to_str().unwrap_or("")
                ).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

                // Walk through data types
                for data_type_entry in fs::read_dir(resolution_entry.path())? {
                    let file_name = data_type_entry?.file_name(); // Unwraps the DirEntry from the Result
                    let file_name_str = file_name
                        .to_str()
                        .unwrap_or("")
                        .to_string();

                    let data_type_str = file_name_str.split('/').last().unwrap_or("");
                    println!("Data type string: {}", data_type_str);
                    let data_type = BaseDataType::from_str(data_type_str)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

                    println!("File name is: {}", file_name_str);

                    // Get earliest and latest data times
                    let earliest = self.get_earliest_data_time(
                        &symbol,
                        &resolution,
                        &data_type
                    ).await.map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

                    let latest = self.get_latest_data_time(
                        &symbol,
                        &resolution,
                        &data_type
                    ).await.map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

                    if let (Some(earliest), Some(latest)) = (earliest, latest) {
                        available_data.push(AvailableData {
                            symbol: symbol_name.clone(),
                            market_type: market_type.to_string(),
                            resolution: resolution.to_string(),
                            data_type: data_type.to_string(),
                            earliest_date: earliest,
                            latest_date: latest,
                        });
                    }
                }
            }
        }

        // Sort by symbol and earliest date
        available_data.sort_by(|a, b| {
            a.symbol.cmp(&b.symbol)
                .then(a.earliest_date.cmp(&b.earliest_date))
        });

        // Write to CSV
        let mut writer = csv::Writer::from_path(export_path)?;
        writer.write_record(&[
            "Symbol",
            "Market Type",
            "Resolution",
            "Data Type",
            "Earliest Date",
            "Latest Date",
            "Days of History",
        ])?;

        for data in available_data {
            let days_of_history = (data.latest_date - data.earliest_date).num_days();
            writer.write_record(&[
                data.symbol,
                data.market_type,
                data.resolution,
                data.data_type,
                data.earliest_date.to_rfc3339(),
                data.latest_date.to_rfc3339(),
                days_of_history.to_string(),
            ])?;
        }

        writer.flush()?;
        Ok(())
    }
}

#[derive(Debug)]
struct AvailableData {
    symbol: String,
    market_type: String,
    resolution: String,
    data_type: String,
    earliest_date: DateTime<Utc>,
    latest_date: DateTime<Utc>,
}
#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;
    use super::*;
    use tempfile::TempDir;
    use crate::database::hybrid_storage::HybridStorage;
    use crate::server_launch_options::ServerLaunchOptions;
    use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
    use crate::standardized_types::base_data::candle::generate_5_day_candle_data;
    use crate::standardized_types::enums::FuturesExchange;

    #[tokio::test]
    async fn test_catalog_available_data() {
        // Create temporary directory structure first
        let temp_dir = TempDir::new().unwrap();
        println!("Temp dir created at: {:?}", temp_dir.path());

        // Now set up the storage with the existing directory
        let options = ServerLaunchOptions {
            data_folder: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let storage = HybridStorage::new(
            Duration::from_secs(3600),
            options,
            5,
            300
        );

        // Generate test data with Rithmic vendor and proper futures exchange
        let mut test_data = generate_5_day_candle_data();
        let resolution = Resolution::Hours(1);
        println!("Resolution string format: {}", resolution.to_string());

        for candle in &mut test_data {
            candle.symbol = Symbol::new(
                "ES".to_string(),
                DataVendor::Rithmic,
                MarketType::Futures(FuturesExchange::CME)
            );
            candle.resolution = resolution.clone();
        }

        // Save data first to create the directory structure and files
        storage.save_data_bulk(
            test_data.iter()
                .map(|c| BaseDataEnum::Candle(c.clone()))
                .collect()
        ).await.unwrap();

        // Print directory structure after saving
        let rithmic_path = temp_dir.path()
            .join("historical")
            .join("Rithmic")
            .join("Futures");
        println!("\nChecking directory structure after save:");
        fn print_dir(path: &Path, prefix: &str) -> io::Result<()> {
            if path.is_dir() {
                for entry in fs::read_dir(path)? {
                    let entry = entry?;
                    let path = entry.path();
                    println!("{}{}", prefix, path.display());
                    if path.is_dir() {
                        print_dir(&path, &format!("{}  ", prefix))?;
                    }
                }
            }
            Ok(())
        }
        print_dir(&temp_dir.path(), "").unwrap();

        // Now verify each part of the path
        let symbol_path = rithmic_path.join("ES");
        println!("\nChecking paths:");
        println!("Symbol path: {:?} exists: {}", symbol_path, symbol_path.exists());

        // List contents of symbol directory if it exists
        if symbol_path.exists() {
            println!("\nContents of symbol directory:");
            for entry in fs::read_dir(&symbol_path).unwrap() {
                let entry = entry.unwrap();
                println!("  {:?}", entry.path());
            }
        }

        let resolution_path = symbol_path.join(resolution.to_string());
        println!("Resolution path: {:?} exists: {}", resolution_path, resolution_path.exists());

        let data_type_path = resolution_path.join(BaseDataType::Candles.to_string());
        println!("Data type path: {:?} exists: {}", data_type_path, data_type_path.exists());

        // Create catalog
        let catalog_path = temp_dir.path().join("catalog.csv");
        match storage.catalog_available_data(catalog_path.clone()).await {
            Ok(_) => {
                assert!(catalog_path.exists());
                let mut reader = csv::Reader::from_path(&catalog_path).unwrap();
                let records: Vec<_> = reader.records().collect();
                assert!(!records.is_empty(), "Catalog should contain data");

                println!("\nCatalog contents:");
                let content = fs::read_to_string(&catalog_path).unwrap();
                println!("{}", content);

                if let Some(Ok(record)) = records.first() {
                    assert_eq!(record.get(0), Some("ES"));
                    assert_eq!(record.get(1), Some("Futures"));
                    assert_eq!(record.get(3), Some("Candles"));
                }
            },
            Err(e) => panic!("Failed to create catalog: {}", e),
        }
    }
}