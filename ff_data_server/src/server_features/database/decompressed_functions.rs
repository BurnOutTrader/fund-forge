use std::{fs};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use ff_standard_lib::messages::data_server_messaging::FundForgeError;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::Symbol;
use crate::server_features::database::hybrid_storage::HybridStorage;

impl HybridStorage {
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

        // Get years in ascending order
        let mut years: Vec<_> = fs::read_dir(&base_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();
        years.sort_by_key(|e| e.file_name().to_str()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0));

        if let Some(year_dir) = years.first() {
            // Get months in ascending order
            let mut months: Vec<_> = fs::read_dir(year_dir.path())?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .collect();
            months.sort_by_key(|e| e.file_name().to_str()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0));

            if let Some(month_dir) = months.first() {
                // Get days in ascending order
                let mut days: Vec<_> = fs::read_dir(month_dir.path())?
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().map_or(false, |ext| ext == "bin"))
                    .collect();
                days.sort_by_key(|e| e.file_name().to_str().unwrap_or("").to_string());

                if let Some(earliest_file) = days.first() {
                    if let Ok(mmap) = self.get_or_create_mmap(&earliest_file.path()).await {
                        // Create a properly aligned copy
                        let aligned_data = mmap.as_ref().to_vec();
                        if let Ok(day_data) = BaseDataEnum::from_array_bytes(&aligned_data) {
                            return Ok(day_data.into_iter().map(|d| d.time_closed_utc()).min());
                        }
                    }
                }
            }
        }

        Ok(None)
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

        // Get years in descending order
        let mut years: Vec<_> = fs::read_dir(&base_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();
        years.sort_by_key(|e| e.file_name().to_str()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0));
        years.reverse();

        if let Some(year_dir) = years.first() {
            // Get months in descending order
            let mut months: Vec<_> = fs::read_dir(year_dir.path())?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .collect();
            months.sort_by_key(|e| e.file_name().to_str()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0));
            months.reverse();

            if let Some(month_dir) = months.first() {
                // Get days in descending order
                let mut days: Vec<_> = fs::read_dir(month_dir.path())?
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().map_or(false, |ext| ext == "bin"))
                    .collect();
                days.sort_by_key(|e| e.file_name().to_str().unwrap_or("").to_string());
                days.reverse();

                if let Some(latest_file) = days.first() {
                    if let Ok(mmap) = self.get_or_create_mmap(&latest_file.path()).await {
                        // Create a properly aligned copy
                        let aligned_data = mmap.as_ref().to_vec();
                        if let Ok(day_data) = BaseDataEnum::from_array_bytes(&aligned_data) {
                            return Ok(day_data.into_iter().map(|d| d.time_closed_utc()).max());
                        }
                    }
                }
            }
        }

        Ok(None)
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
                // Create a properly aligned copy of the memory-mapped data
                let aligned_data = mmap.as_ref().to_vec();
                if let Ok(day_data) = BaseDataEnum::from_array_bytes(&aligned_data) {
                    //eprintln!("Found data for {} in {}", target_time, file_path.display());
                    // Find the latest data point that satisfies the condition
                    if let Some(latest_data) = day_data
                        .into_iter()
                        .filter(|d| d.time_closed_utc() <= target_time)
                        .max_by_key(|d| d.time_closed_utc())
                    {
                        return Ok(Some(latest_data));
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
                // Create a properly aligned copy of the memory-mapped data
                let aligned_data = mmap.as_ref().to_vec();
                if let Ok(day_data) = BaseDataEnum::from_array_bytes(&aligned_data) {
                    //eprintln!("Found data for {}", current_date);
                    // Find the latest data point that satisfies the condition
                    if let Some(latest_data) = day_data
                        .into_iter()
                        .filter(|d| d.time_closed_utc() <= target_time)
                        .max_by_key(|d| d.time_closed_utc())
                    {
                        return Ok(Some(latest_data));
                    }
                }
            }
        }

        Ok(None)
    }

    pub async fn get_data_range(
        &self,
        symbol: &Symbol,
        resolution: &Resolution,
        data_type: &BaseDataType,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<BaseDataEnum>, FundForgeError> {
        let mut all_data = Vec::new();
        let base_path = self.get_base_path(symbol, resolution, data_type, false);
        if !base_path.exists() {
            return Ok(Vec::new());
        }

        let start_year = start.year();
        let end_year = end.year();

        for year in start_year..=end_year {
            let year_path = base_path.join(format!("{:04}", year));
            if !year_path.exists() { continue; }

            let start_month = if year == start_year { start.month() } else { 1 };
            let end_month = if year == end_year { end.month() } else { 12 };

            for month in start_month..=end_month {
                let month_path = year_path.join(format!("{:02}", month));
                if !month_path.exists() { continue; }

                let current_date = if year == start_year && month == start.month() {
                    start.date_naive()
                } else {
                    NaiveDate::from_ymd_opt(year, month, 1).unwrap()
                };

                let month_end = if year == end_year && month == end.month() {
                    end.date_naive()
                } else {
                    NaiveDate::from_ymd_opt(year, month + 1, 1)
                        .unwrap_or_else(|| NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap())
                        .pred_opt()
                        .unwrap()
                };

                let mut current_date = current_date;
                while current_date <= month_end {
                    let file_path = month_path.join(format!("{:04}{:02}{:02}.bin",
                                                            year,
                                                            month,
                                                            current_date.day()
                    ));

                    if file_path.exists() {
                        if let Ok(mmap) = self.get_or_create_mmap(&file_path).await {
                            // Create a properly aligned copy of the memory-mapped data
                            let aligned_data = mmap.as_ref().to_vec();
                            if let Ok(mut day_data) = BaseDataEnum::from_array_bytes(&aligned_data) {
                                // Filter data within the time range
                                day_data.retain(|d| {
                                    let time = d.time_closed_utc();
                                    time >= start && time <= end
                                });
                                all_data.extend(day_data);
                            } else {
                                eprintln!("Error deserializing data from {}", file_path.display());
                            }
                        }
                    }

                    current_date = match current_date.succ_opt() {
                        Some(date) => date,
                        None => break,
                    };
                }
            }
        }

        // Sort the data by timestamp before returning
        all_data.sort_by_key(|d| d.time_closed_utc());
        Ok(all_data)
    }
}