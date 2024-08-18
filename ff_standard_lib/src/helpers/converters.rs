use chrono_tz::Tz;
use std::fmt::Error;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use chrono::{Datelike, DateTime, FixedOffset, NaiveDate, NaiveDateTime, Offset, TimeZone, Utc};


/// returns the fixed offset of the from utc time.
/// Since we are dealing with historical data, we need to adjust for daylight savings etc, so it is not good enough to just use the current offset, we need to pass in the historical date and get the offset at that time for the timezone.
pub fn offset_local_from_utc_time(time_zone: &Tz, utc_time: NaiveDateTime) -> FixedOffset {
    let tz_offset = time_zone.offset_from_utc_datetime(&utc_time);
    FixedOffset::east_opt(tz_offset.fix().local_minus_utc()).unwrap()
}

/// Returns the fixed offset of the from a utc timestamp.
/// Since we are dealing with historical data, we need to adjust for daylight savings etc, so it is not good enough to just use the current offset, we need to pass in the historical date and get the offset at that time for the timezone.
pub fn offset_local_from_utc_time_stamp(time_zone: &Tz, time_zone_time: i64, nanos: u32) -> FixedOffset {
    let time = DateTime::from_timestamp(time_zone_time, nanos).unwrap();
    let tz_offset = time_zone.offset_from_utc_datetime(&time.naive_utc());
    FixedOffset::east_opt(tz_offset.fix().local_minus_utc()).unwrap()
}

pub fn time_local_from_str(time_zone: &Tz, time: &str) -> DateTime<FixedOffset>{
    let utc_time: DateTime<Utc> = DateTime::from_str(&time).unwrap();
    time_convert_utc_datetime_to_fixed_offset(time_zone, utc_time)
}

/// Converts a UTC `NaiveDateTime` to `DateTime<FixedOffset>` for the given timezone.
/// This accounts for historical timezone changes, including DST.
pub fn time_convert_utc_naive_to_fixed_offset(time_zone: &Tz, utc_time: NaiveDateTime) -> DateTime<FixedOffset> {
    let timezone_aware_datetime = time_zone.from_utc_datetime(&utc_time);
    let fixed_offset = time_zone.offset_from_utc_datetime(&utc_time).fix();
    timezone_aware_datetime.with_timezone(&fixed_offset)
}

pub fn convert_to_utc(naive_date_time: NaiveDateTime, time_zone: Tz) -> DateTime<Utc> {
    // Get the offset from UTC for the given time zone and NaiveDateTime
    let offset = time_zone.offset_from_local_datetime(&naive_date_time).unwrap().fix();

    // Convert NaiveDateTime to DateTime<FixedOffset>
    let fixed_offset_date_time = DateTime::<FixedOffset>::from_local(naive_date_time, offset);
    
    // Convert DateTime<FixedOffset> to DateTime<Utc>
    fixed_offset_date_time.with_timezone(&Utc)
}

/// Converts a UTC Unix timestamp to `DateTime<FixedOffset>` for the given timezone.
/// This accounts for historical timezone changes, including DST.
pub fn time_convert_utc_timestamp_to_fixed_offset(time_zone: &Tz, utc_timestamp: i64, nanos: u32) -> DateTime<FixedOffset> {
    let utc_time = DateTime::from_timestamp(utc_timestamp, nanos).expect("Invalid timestamp");
    let timezone_aware_datetime = time_zone.from_utc_datetime(&utc_time.naive_utc());
    let fixed_offset = time_zone.offset_from_utc_datetime(&utc_time.naive_utc()).fix();
    timezone_aware_datetime.with_timezone(&fixed_offset)
}

/// Converts a `DateTime<Utc>` to `DateTime<FixedOffset>` for the given timezone.
/// This accounts for historical timezone changes, including DST.
pub fn time_convert_utc_datetime_to_fixed_offset(time_zone: &Tz, utc_datetime: DateTime<Utc>) -> DateTime<FixedOffset> {
    let naive_utc_time = utc_datetime.naive_utc();
    let timezone_aware_datetime = time_zone.from_utc_datetime(&naive_utc_time);
    let fixed_offset = time_zone.offset_from_utc_datetime(&naive_utc_time).fix();
    timezone_aware_datetime.with_timezone(&fixed_offset)
}

/// Loads a bytes object from a file path.
/// # Arguments
/// * `file_path` - A PathBuf object that represents the file path to the file to be loaded.
pub fn load_as_bytes(file_path: PathBuf) -> Result<Vec<u8>, Error> {
    let bytes = match fs::read(file_path){
        Ok(bytes) => bytes,
        Err(_e) => return Err(Error::default()),
    };
    Ok(bytes)
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
                },
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
    symbol.replace("/", "-").replace(":", "-").replace("?", "-").replace("_", "-").replace(" ", "-").to_uppercase()
}
