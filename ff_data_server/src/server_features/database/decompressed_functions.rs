use std::{fs, io};
use std::fs::File;
use std::io::Read;
use chrono::{DateTime, Utc};
use flate2::read::GzDecoder;
use ff_standard_lib::messages::data_server_messaging::FundForgeError;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::Symbol;
use crate::server_features::database::hybrid_storage::HybridStorage;

impl HybridStorage {
    // Helper function to decompress and deserialize data from a mmap
    async fn decompress_and_deserialize(&self, mmap: &[u8]) -> io::Result<Vec<BaseDataEnum>> {
        // Create a decoder for the compressed data
        let mut decoder = GzDecoder::new(mmap);
        let mut decompressed = Vec::new();

        // Decompress the data
        decoder.read_to_end(&mut decompressed).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("Decompression error: {}", e))
        })?;

        // Deserialize the decompressed data
        BaseDataEnum::from_array_bytes(&decompressed)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Deserialization error: {}", e)))
    }


    pub async fn get_earliest_data_time(
        &self,
        symbol: &Symbol,
        resolution: &Resolution,
        data_type: &BaseDataType,
    ) -> Result<Option<DateTime<Utc>>, Box<dyn std::error::Error>> {
        let base_path = self.get_base_path(symbol, resolution, data_type, false);
        if !base_path.exists() {
            return Ok(None);
        }

        let mut earliest_time: Option<DateTime<Utc>> = None;

        // Get years in ascending order
        let mut years: Vec<_> = fs::read_dir(&base_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| {
                e.file_name()
                    .to_str()
                    .and_then(|s| s.parse::<i32>().ok())
                    .map(|year| (year, e.path()))
            })
            .collect();
        years.sort_unstable_by(|a, b| a.0.cmp(&b.0));  // Sort ascending

        for (_year, year_path) in years {
            // Get months in ascending order
            let mut months: Vec<_> = fs::read_dir(year_path)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| {
                    e.file_name()
                        .to_str()
                        .and_then(|s| s.parse::<u32>().ok())
                        .map(|month| (month, e.path()))
                })
                .collect();
            months.sort_unstable_by(|a, b| a.0.cmp(&b.0));  // Sort ascending

            for (_month, month_path) in months {
                // Get days in ascending order
                let mut days: Vec<_> = fs::read_dir(month_path)?
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().map_or(false, |ext| ext == "bin"))
                    .collect();
                days.sort_by_key(|e| e.path());

                for day in days {
                    let file_path = day.path();
                    let mut file = match File::open(&file_path) {
                        Ok(file) => file,
                        Err(e) => {
                            eprintln!("Error opening file {}: {}", file_path.display(), e);
                            continue;
                        }
                    };

                    let mut compressed_data = Vec::new();
                    if let Err(e) = file.read_to_end(&mut compressed_data) {
                        eprintln!("Error reading file {}: {}", file_path.display(), e);
                        continue;
                    }

                    // Decompress
                    let mut decoder = GzDecoder::new(&compressed_data[..]);
                    let mut decompressed = Vec::new();
                    match decoder.read_to_end(&mut decompressed) {
                        Ok(_) => {
                            match BaseDataEnum::from_array_bytes(&decompressed) {
                                Ok(day_data) => {
                                    if let Some(time) = day_data.into_iter()
                                        .map(|d| d.time_closed_utc())
                                        .min() {
                                        match earliest_time {
                                            None => earliest_time = Some(time),
                                            Some(current_earliest) if time < current_earliest => {
                                                earliest_time = Some(time)
                                            }
                                            _ => {}
                                        }
                                        // Found valid data, no need to check more files
                                        return Ok(earliest_time);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error deserializing data from {}: {}", file_path.display(), e);
                                    continue;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error decompressing data from {}: {}", file_path.display(), e);
                            continue;
                        }
                    }
                }
            }
        }

        Ok(earliest_time)
    }

    pub async fn get_latest_data_time(
        &self,
        symbol: &Symbol,
        resolution: &Resolution,
        data_type: &BaseDataType,
    ) -> Result<Option<DateTime<Utc>>, Box<dyn std::error::Error>> {
        let base_path = self.get_base_path(symbol, resolution, data_type, false);
        if !base_path.exists() {
            return Ok(None);
        }

        let mut latest_time: Option<DateTime<Utc>> = None;

        // Get years in descending order
        let mut years: Vec<_> = fs::read_dir(&base_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| {
                e.file_name()
                    .to_str()
                    .and_then(|s| s.parse::<i32>().ok())
                    .map(|year| (year, e.path()))
            })
            .collect();
        years.sort_unstable_by(|a, b| b.0.cmp(&a.0));  // Sort descending

        for (_year, year_path) in years {
            // Get months in descending order
            let mut months: Vec<_> = fs::read_dir(year_path)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| {
                    e.file_name()
                        .to_str()
                        .and_then(|s| s.parse::<u32>().ok())
                        .map(|month| (month, e.path()))
                })
                .collect();
            months.sort_unstable_by(|a, b| b.0.cmp(&a.0));  // Sort descending

            for (_month, month_path) in months {
                // Get days in descending order
                let mut days: Vec<_> = fs::read_dir(month_path)?
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().map_or(false, |ext| ext == "bin"))
                    .collect();
                days.sort_by_key(|e| std::cmp::Reverse(e.path()));

                for day in days {
                    let file_path = day.path();
                    // Read file directly instead of using mmap
                    let mut file = match File::open(&file_path) {
                        Ok(file) => file,
                        Err(e) => {
                            eprintln!("Error opening file {}: {}", file_path.display(), e);
                            continue;
                        }
                    };

                    let mut compressed_data = Vec::new();
                    if let Err(e) = file.read_to_end(&mut compressed_data) {
                        eprintln!("Error reading file {}: {}", file_path.display(), e);
                        continue;
                    }

                    // Decompress
                    let mut decoder = GzDecoder::new(&compressed_data[..]);
                    let mut decompressed = Vec::new();
                    match decoder.read_to_end(&mut decompressed) {
                        Ok(_) => {
                            match BaseDataEnum::from_array_bytes(&decompressed) {
                                Ok(day_data) => {
                                    if let Some(time) = day_data.into_iter()
                                        .map(|d| d.time_closed_utc())
                                        .max() {
                                        match latest_time {
                                            None => latest_time = Some(time),
                                            Some(current_latest) if time > current_latest => {
                                                latest_time = Some(time)
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error deserializing data from {}: {}", file_path.display(), e);
                                    continue;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error decompressing data from {}: {}", file_path.display(), e);
                            continue;
                        }
                    }
                }
            }
        }

        Ok(latest_time)
    }


    // Updated get_data_point_asof function
    pub async fn get_data_point_asof(
        &self,
        symbol: &Symbol,
        resolution: &Resolution,
        data_type: &BaseDataType,
        target_time: DateTime<Utc>,
    ) -> Result<Option<BaseDataEnum>, FundForgeError> {
        // Get the file path for the target date
        let file_path = self.get_file_path(symbol, resolution, data_type, &target_time, false);

        // If the file exists for the target date, check it first
        if file_path.exists() {
            if let Ok(mmap) = self.get_or_create_mmap(&file_path).await {
                if let Ok(day_data) = self.decompress_and_deserialize(&mmap[..]).await {
                    // Find the closest point before or at target_time
                    let result = day_data.into_iter()
                        .filter(|d| d.time_closed_utc() <= target_time)
                        .max_by_key(|d| d.time_closed_utc());

                    if let Some(point) = result {
                        return Ok(Some(point));
                    }
                }
            }
        }

        let mut current_date = target_time.date_naive();
        let mut attempts = 0;
        const MAX_ATTEMPTS: i32 = 30; // Limit how far back we'll look

        while let Some(prev_date) = current_date.pred_opt() {
            attempts += 1;
            if attempts > MAX_ATTEMPTS {
                break;
            }

            current_date = prev_date;
            let file_path = self.get_file_path(
                symbol,
                resolution,
                data_type,
                &DateTime::<Utc>::from_naive_utc_and_offset(
                    current_date.and_hms_opt(0, 0, 0).unwrap(),
                    Utc,
                ),
                false,
            );

            if !file_path.exists() {
                // Check if we've moved to a different month/year directory
                let month_path = file_path.parent().unwrap();
                let year_path = month_path.parent().unwrap();

                // If neither the month nor year directory exists, break to avoid unnecessary checks
                if !month_path.exists() && !year_path.exists() {
                    break;
                }
                continue;
            }

            if let Ok(mmap) = self.get_or_create_mmap(&file_path).await {
                match self.decompress_and_deserialize(&mmap[..]).await {
                    Ok(day_data) => {
                        let result = day_data.into_iter()
                            .filter(|d| d.time_closed_utc() <= target_time)
                            .max_by_key(|d| d.time_closed_utc());

                        if let Some(point) = result {
                            return Ok(Some(point));
                        }
                    }
                    Err(e) => {
                        eprintln!("Error processing file {}: {}", file_path.display(), e);
                        continue;
                    }
                }
            }
        }

        Ok(None)
    }
}