use std::{fs, io};
use std::path::{Path, PathBuf};
use chrono::{DateTime, NaiveDate, Utc};
use serde_json::json;
use strum_macros::Display;
use crate::database::hybrid_storage::HybridStorage;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::subscriptions::DataSubscription;

#[derive(Debug, Display)]
pub enum ExportFormat {
    CSV,
    JSON
}

impl HybridStorage {
    pub async fn export_data(
        &self,
        subscription: &DataSubscription,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        export_folder: PathBuf,
        format: ExportFormat,
    ) -> io::Result<()> {
        // Get list of files in range
        let files = match self.get_files_in_range(
            &subscription.symbol,
            &subscription.resolution,
            &subscription.base_data_type,
            start,
            end,
        ).await {
            Ok(files) => files,
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Error getting files in range: {}", e)));
            }
        };

        // Create the export directory structure
        let base_path = export_folder
            .join(format!("{}", format))
            .join(subscription.symbol.data_vendor.to_string())
            .join(subscription.symbol.market_type.to_string())
            .join(subscription.symbol.name.to_string())
            .join(subscription.resolution.to_string())
            .join(subscription.base_data_type.to_string());

        fs::create_dir_all(&base_path)?;

        // Process each file individually
        for file_path in files {
            let date = get_date_from_filepath(&file_path)?;
            let export_file_name = match format {
                ExportFormat::CSV => format!("{}.csv", date.format("%Y%m%d")),
                ExportFormat::JSON => format!("{}.json", date.format("%Y%m%d")),
            };
            let export_path = base_path.join(export_file_name);

            // Skip if file exists
            if export_path.exists() {
                continue;
            }

            // Read and process file
            if let Ok(mmap) = self.get_or_create_mmap(&file_path, subscription.resolution.clone()).await {
                let aligned_data = mmap.as_ref().to_vec();
                let data = BaseDataEnum::from_array_bytes(&aligned_data)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

                match format {
                    ExportFormat::CSV => export_to_csv(&export_path, data, subscription.base_data_type.clone())?,
                    ExportFormat::JSON => export_to_json(&export_path, data, subscription.base_data_type.clone())?,
                }
            }

            // Clean up memory map
            if let Some((_key, mmap)) = self.mmap_cache.remove(&file_path.to_string_lossy().to_string()) {
                drop(mmap);
            }
        }

        Ok(())
    }
}

fn get_date_from_filepath(path: &Path) -> io::Result<NaiveDate> {
    let file_stem = path.file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid file name"))?;

    if file_stem.len() != 8 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid date format in filename"));
    }

    let year = &file_stem[0..4].parse::<i32>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid year"))?;
    let month = &file_stem[4..6].parse::<u32>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid month"))?;
    let day = &file_stem[6..8].parse::<u32>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid day"))?;

    NaiveDate::from_ymd_opt(*year, *month, *day)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid date"))
}

fn export_to_csv(path: &Path, data: Vec<BaseDataEnum>, data_type: BaseDataType) -> io::Result<()> {
    let mut writer = csv::Writer::from_path(path)?;

    match data_type {
        BaseDataType::Candles => {
            writer.write_record(&[
                "symbol", "time", "open", "high", "low", "close",
                "volume", "ask_volume", "bid_volume", "range"
            ])?;

            for item in data {
                if let BaseDataEnum::Candle(candle) = item {
                    writer.write_record(&[
                        candle.symbol.name.to_string(),
                        candle.time,
                        candle.open.to_string(),
                        candle.high.to_string(),
                        candle.low.to_string(),
                        candle.close.to_string(),
                        candle.volume.to_string(),
                        candle.ask_volume.to_string(),
                        candle.bid_volume.to_string(),
                        candle.range.to_string(),
                    ])?;
                }
            }
        },
        BaseDataType::Ticks => {
            writer.write_record(&[
                "symbol", "time", "price", "volume", "aggressor"
            ])?;

            for item in data {
                if let BaseDataEnum::Tick(tick) = item {
                    writer.write_record(&[
                        tick.symbol.name.to_string(),
                        tick.time,
                        tick.price.to_string(),
                        tick.volume.to_string(),
                        tick.aggressor.to_string(),
                    ])?;
                }
            }
        },
        _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Unsupported data type")),
    }

    writer.flush()?;
    Ok(())
}

fn export_to_json(path: &Path, data: Vec<BaseDataEnum>, data_type: BaseDataType) -> io::Result<()> {
    match data_type {
        BaseDataType::Candles => {
            let json_data: Vec<_> = data.into_iter()
                .filter_map(|item| {
                    if let BaseDataEnum::Candle(candle) = item {
                        Some(json!({
                            "symbol": candle.symbol.name,
                            "time": candle.time,
                            "open": candle.open,
                            "high": candle.high,
                            "low": candle.low,
                            "close": candle.close,
                            "volume": candle.volume,
                            "ask_volume": candle.ask_volume,
                            "bid_volume": candle.bid_volume,
                            "range": candle.range,
                        }))
                    } else {
                        None
                    }
                })
                .collect();

            let json_string = serde_json::to_string_pretty(&json_data)?;
            fs::write(path, json_string)?;
        },
        BaseDataType::Ticks => {
            let json_data: Vec<_> = data.into_iter()
                .filter_map(|item| {
                    if let BaseDataEnum::Tick(tick) = item {
                        Some(json!({
                            "symbol": tick.symbol.name,
                            "time": tick.time,
                            "price": tick.price,
                            "volume": tick.volume,
                            "aggressor": tick.aggressor.to_string(),
                        }))
                    } else {
                        None
                    }
                })
                .collect();

            let json_string = serde_json::to_string_pretty(&json_data)?;
            fs::write(path, json_string)?;
        },
        _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Unsupported data type")),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use super::*;
    use tempfile::TempDir;
    use crate::server_launch_options::ServerLaunchOptions;
    use crate::standardized_types::base_data::candle::generate_5_day_candle_data;
    use crate::standardized_types::base_data::traits::BaseData;

    fn setup_test_storage() -> (HybridStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let options = ServerLaunchOptions {
            data_folder: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let storage = HybridStorage::new(
            Duration::from_secs(3600),  // 1 hour cache duration
            options,
            5,                          // max concurrent downloads
            300                         // update seconds
        );

        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_export_data() {
        // Setup test environment

        let (storage, temp_dir) = setup_test_storage();

        // Generate test data
        let test_data = generate_5_day_candle_data().iter()
            .map(|c| BaseDataEnum::Candle(c.clone()))
            .collect::<Vec<_>>();

        // Save test data
        storage.save_data_bulk(test_data.clone()).await.unwrap();

        // Create export directory
        let export_dir = TempDir::new().unwrap();

        // Get time range from test data
        let start = test_data.first().unwrap().time_closed_utc();
        let end = test_data.last().unwrap().time_closed_utc();

        // Create subscription from first data point
        let subscription = test_data[0].subscription();

        // Test CSV export
        let csv_result = storage.export_data(
            &subscription,
            start,
            end,
            export_dir.path().to_path_buf(),
            ExportFormat::CSV
        ).await;
        assert!(csv_result.is_ok(), "CSV export failed: {:?}", csv_result);

        // Verify CSV files exist and are valid
        let csv_path = export_dir.path()
            .join("CSV")
            .join(subscription.symbol.data_vendor.to_string())
            .join(subscription.symbol.market_type.to_string())
            .join(subscription.symbol.name.to_string())
            .join(subscription.resolution.to_string())
            .join(subscription.base_data_type.to_string());

        assert!(csv_path.exists(), "CSV export directory not created");

        let csv_files: Vec<_> = std::fs::read_dir(&csv_path)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().and_then(|s| s.to_str()) == Some("csv"))
            .collect();

        assert!(!csv_files.is_empty(), "No CSV files were created");

        // Test JSON export
        let json_result = storage.export_data(
            &subscription,
            start,
            end,
            export_dir.path().to_path_buf(),
            ExportFormat::JSON
        ).await;
        assert!(json_result.is_ok(), "JSON export failed: {:?}", json_result);

        // Verify JSON files exist and are valid
        let json_path = export_dir.path()
            .join("JSON")
            .join(subscription.symbol.data_vendor.to_string())
            .join(subscription.symbol.market_type.to_string())
            .join(subscription.symbol.name.to_string())
            .join(subscription.resolution.to_string())
            .join(subscription.base_data_type.to_string());

        assert!(json_path.exists(), "JSON export directory not created");

        let json_files: Vec<_> = std::fs::read_dir(&json_path)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().and_then(|s| s.to_str()) == Some("json"))
            .collect();

        assert!(!json_files.is_empty(), "No JSON files were created");

        // Verify file contents
        for csv_file in csv_files {
            let reader = csv::Reader::from_path(csv_file.path()).unwrap();
            let record_count = reader.into_records().count();
            assert!(record_count > 0, "CSV file is empty");
        }

        for json_file in json_files {
            let content = std::fs::read_to_string(json_file.path()).unwrap();
            let json: serde_json::Value = serde_json::from_str(&content).unwrap();
            assert!(json.as_array().unwrap().len() > 0, "JSON file is empty");
        }

        // Cleanup will happen automatically when TempDir is dropped
    }
}