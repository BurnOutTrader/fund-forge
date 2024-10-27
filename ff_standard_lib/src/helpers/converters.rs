use crate::standardized_types::resolution::Resolution;
use crate::standardized_types::subscriptions::DataSubscription;
use chrono::{DateTime, Datelike, Duration, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use std::fmt::Error;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::fmt::Write;

/// Convert a NaiveDateTime to the exact same Date and time Tz DateTime object
///
/// Note this function does not adjust the hour or date, this just creates a DateTime<Tz> object and assumes you know the correct time zone
///
/// # Instructions For Parsing to Utc from a Random TimeZone (Tz)
/// 1. Convert time to a TZ aware object based on the data time zone using `let data_time_local = naive_date_time_to_tz(naive_date_time: NaiveDateTime, time_zone: Tz);`
/// 2. Convert to utc time by using `let data_time_utc = data_time_local.to_utc();`
/// 3. Convert to a string by using `data_time_utc.to_string();`  for `BaseDataEnum` variants
///
///  # Instructions For Parsing to Utc from a Utc String
/// 1. If your data is already in UTC time you can just use `let data_time_utc = naive_date_time_to_utc(naive_date_time: NaiveDateTime);`
/// 2. Convert to a string by using `data_time_utc.to_string();` for `BaseDataEnum` variants
pub fn naive_date_time_to_tz(naive_date_time: NaiveDateTime, time_zone: Tz) -> DateTime<Tz> {
    resolve_market_datetime_in_timezone(time_zone, naive_date_time)
}

/// Convert a NaiveDateTime to the exact same Date and time Utc DateTime object
///
/// Note this function does not adjust the hour or date, this just creates a DateTime<Utc> object and assumes you know the correct time zone
///
/// # Instructions For Parsing to Utc from a Random TimeZone (Tz)
/// 1. Convert time to a TZ aware object based on the data time zone using `let data_time_local = naive_date_time_to_tz(naive_date_time: NaiveDateTime, time_zone: Tz);`
/// 2. Convert to utc time by using `let data_time_utc = data_time_local.to_utc();`
/// 3. Convert to a string by using `data_time_utc.to_string();`  for `BaseDataEnum` variants
///
///  # Instructions For Parsing to Utc from a Utc String
/// 1. If your data is already in UTC time you can just use `let data_time_utc = naive_date_time_to_utc(naive_date_time: NaiveDateTime);`
/// 2. Convert to a string by using `data_time_utc.to_string();` for `BaseDataEnum` variants
pub fn naive_date_time_to_utc(naive_date_time: NaiveDateTime) -> DateTime<Utc> {
    Utc.from_local_datetime(&naive_date_time).unwrap().to_utc()
}

/// Convert utc time string to local time, adjusting the actual hour
pub fn time_local_from_utc_str(time_zone: &Tz, time: &str) -> DateTime<Tz> {
    let utc_time: DateTime<Utc> = DateTime::from_str(&time).unwrap();
    time_zone.from_utc_datetime(&utc_time.naive_utc())
}

/// Convert utc time to local time, adjusting the actual hour
pub fn time_convert_utc_to_local(
    time_zone: &Tz,
    utc_time: DateTime<Utc>,
) -> DateTime<Tz> {
    time_zone.from_utc_datetime(&utc_time.naive_utc())
}

/// Loads a bytes object from a file path.
/// # Arguments
/// * `file_path` - A PathBuf object that represents the file path to the file to be loaded.
pub fn load_as_bytes(file_path: PathBuf) -> Result<Vec<u8>, Error> {
    let bytes = match fs::read(file_path) {
        Ok(bytes) => bytes,
        Err(_e) => return Err(Error::default()),
    };
    Ok(bytes)
}

pub fn resolve_market_datetime_in_timezone(
    tz: Tz,
    naive_dt: NaiveDateTime,
) -> DateTime<Tz>{
    match tz.from_local_datetime(&naive_dt) {
        LocalResult::Single(dt) => dt,
        LocalResult::Ambiguous(dt1, _dt2) => {
            // During fall back, take the first occurrence
            dt1
        }
        LocalResult::None => {
            // During spring forward, skip to the next valid time
            // This maintains market schedule alignment
            tz.from_local_datetime(&(naive_dt + Duration::hours(1)))
                .earliest()
                .unwrap()
        }
    }
}

/// Converts a datetime string to a timestamp, if the string is already a timestamp it will return the timestamp.
/// Most common broker and vendor datetime formats are supported.
///
/// # Arguments
/// * `datetime_string` - A string that represents the datetime to be converted to a timestamp.
///
/// # Returns
/// * A Result object that contains the timestamp if the conversion was successful, otherwise it will contain an Error object.
pub fn timestamp_from_str(datetime_string: &str) -> Result<i64, Error> {
    match i64::from_str(datetime_string) {
        Ok(datetime) => return Ok(datetime),
        _ => {
            match NaiveDateTime::parse_from_str(datetime_string, "%Y-%m-%dT%H:%M:%SZ") {
                Ok(datetime) => return Ok(datetime.and_utc().timestamp()),
                Err(_e) => {
                    let formats = &[
                        // ISO 8601 / RFC 3339 variations
                        "%Y-%m-%dT%H:%M:%S%.fZ",
                        "%Y-%m-%dT%H:%M:%S%.f%:z",
                        "%Y-%m-%dT%H:%M:%S%z",
                        "%Y-%m-%dT%H:%M:%SZ",
                        // RFC 2822
                        "%a, %d %b %Y %H:%M:%S %z",
                        // American formats
                        "%m/%d/%Y %H:%M:%S",
                        "%m/%d/%Y %I:%M:%S %p",
                        "%m-%d-%Y %H:%M",
                        // European formats
                        "%d/%m/%Y %H:%M:%S",
                        "%d/%m/%Y %I:%M:%S %p",
                        "%d-%m-%Y %H:%M",
                        // Other common formats
                        "%Y-%m-%d %H:%M:%S",
                        "%Y.%m.%d %H:%M:%S",
                        "%b %d, %Y, %H:%M:%S",
                        // Add more formats as needed
                    ];
                    for format in formats {
                        match NaiveDateTime::parse_from_str(datetime_string, format) {
                            Ok(datetime) => return Ok(datetime.and_utc().timestamp()),
                            Err(_e) => continue,
                        }
                    }
                    Err(Error::default())
                }
            }
        }
    }
}

/// Returns the next month from the last time input, it will be the first day of the next month at hms 00:00:00
/// Returns the next month from the last time input, it will be the first day of the next month at hms 00:00:00

pub fn next_month(last_time: &DateTime<Utc>) -> DateTime<Utc> {
    let naive_date = last_time.date_naive();

    // Calculate the next month's date directly, without optional handling.
    let next_month_naive_date = if naive_date.month() == 12 {
        // If it's December, increment the year and set the month to January.
        NaiveDate::from_ymd_opt(naive_date.year() + 1, 1, 1).unwrap()
    } else {
        // For any other month, simply increment the month.
        // We can confidently use unwrap here since the logic ensures valid dates.
        NaiveDate::from_ymd_opt(naive_date.year(), naive_date.month() + 1, 1).unwrap()
    };

    // Set time to 0 hours, 0 minutes, and 0 seconds for the start of the month.
    let next_month_date_time = next_month_naive_date.and_hms_opt(0, 0, 0).unwrap();

    // Since the conversion is direct and we are not modifying time, use UTC directly.
    let next_month_utc_time = Utc.from_utc_datetime(&next_month_date_time);

    next_month_utc_time
}

pub fn fund_forge_formatted_symbol_name(symbol: &str) -> String {
    symbol
        .replace("/", "-")
        .replace(":", "-")
        .replace("?", "-")
        .replace("_", "-")
        .replace(" ", "-")
        .to_uppercase()
}

//Returns the open time for a bar, where we only have the current time.
pub fn open_time(subscription: &DataSubscription, time: DateTime<Utc>) -> DateTime<Utc> {
    match subscription.resolution {
        Resolution::Seconds(interval) => {
            let timestamp = time.timestamp();
            let rounded_timestamp = timestamp - (timestamp % interval as i64);
            Utc.timestamp_opt(rounded_timestamp, 0).unwrap()
        }
        Resolution::Minutes(interval) => {
            let minute = (time.minute() as u64 / interval) * interval;
            time.with_minute(minute as u32)
                .unwrap()
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap()
        }
        Resolution::Hours(interval) => {
            let hour = (time.hour() as u64 / interval) * interval;
            time.with_hour(hour as u32)
                .unwrap()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap()
        }
        _ => time, // Handle other cases if necessary
    }
}

pub fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.num_seconds();
    let nanos = duration.subsec_nanos();

    let days = total_seconds / 86400; // 86400 seconds in a day
    let hours = (total_seconds % 86400) / 3600; // 3600 seconds in an hour
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    let mut result = String::new();
    if days > 0 {
        write!(result, "{}d ", days).unwrap();
    }
    if hours > 0 || !result.is_empty() {
        write!(result, "{}h ", hours).unwrap();
    }
    if minutes > 0 || !result.is_empty() {
        write!(result, "{}m ", minutes).unwrap();
    }
    if seconds > 0 || !result.is_empty() {
        write!(result, "{}s ", seconds).unwrap();
    }
    write!(result, "{}ns", nanos).unwrap();

    result.trim().to_string()
}
