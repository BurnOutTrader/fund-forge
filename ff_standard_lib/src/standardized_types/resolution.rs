use std::fmt;
use std::str::FromStr;
use chrono::Duration;
use serde_derive::{Deserialize, Serialize};
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};

///The resolution of a data point, which determines the time period it covers.

#[derive(Serialize, Deserialize, Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialOrd, Eq, Ord, PartialEq, Copy, Debug, Hash)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum Resolution {
    Instant,
    Ticks(u64),
    Seconds(u64),
    Minutes(u64),
    Hours(u64),
    Days(u64),
    Weeks(u64),
}

impl Default for Resolution {
    fn default() -> Self {
        Resolution::Instant
    }
}

impl FromStr for Resolution {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let resolution_string = s.to_uppercase();

        // Handle both "-" and "_" separators
        let parts: Vec<&str> = if resolution_string.contains('-') {
            resolution_string.split('-').collect()
        } else if resolution_string.contains('_') {
            resolution_string.split('_').collect()
        } else {
            return Err(format!("Invalid format: no separator found in {}", s));
        };

        if parts.len() != 2 {
            return Err(format!("Invalid format: expected 2 parts in {}", s));
        }

        let number = parts[0].parse::<u64>()
            .map_err(|_| format!("Invalid number in {}", s))?;

        // Trim any whitespace and get_requests first character
        match parts[1].trim().chars().next() {
            Some('I') => Ok(Resolution::Instant),
            Some('T') => Ok(Resolution::Ticks(number)),
            Some('S') => Ok(Resolution::Seconds(number)),
            Some('M') => Ok(Resolution::Minutes(number)),
            Some('H') => Ok(Resolution::Hours(number)),
            Some('D') => Ok(Resolution::Days(number)),
            Some('W') => Ok(Resolution::Weeks(number)),
            Some(c) => Err(format!("Invalid resolution type '{}' in {}", c, s)),
            None => Err(format!("Empty resolution type in {}", s)),
        }
    }
}

impl Resolution {
    /// Returns the number of seconds in the resolution
    /// Ticks always return 0 as this fn is used to determine close times of time series based data from the opening time

    pub fn as_duration(&self) -> Duration {
        match self {
            Resolution::Instant => Duration::zero(),
            Resolution::Ticks(_) => Duration::zero(),
            Resolution::Seconds(val) => Duration::seconds(*val as i64),
            Resolution::Minutes(val) => Duration::minutes(*val as i64),
            Resolution::Hours(val) => Duration::hours(*val as i64),
            Resolution::Days(val) => Duration::days(*val as i64),
            Resolution::Weeks(val) => Duration::weeks(*val as i64),
        }
    }

    pub fn number_of(&self) -> u64 {
        match self {
            Resolution::Instant => 0,
            Resolution::Ticks(val) => val.clone(),
            Resolution::Seconds(val) => val.clone(),
            Resolution::Minutes(val) => val.clone(),
            Resolution::Hours(val) => val.clone(),
            Resolution::Days(val) => val.clone(),
            Resolution::Weeks(val) => val.clone(),
        }
    }

    pub fn as_seconds(&self) -> i64 {
        let duration = self.as_duration();
        duration.num_seconds()
    }

    pub fn as_nanos(&self) -> i64 {
        self.as_duration().num_nanoseconds().unwrap()
    }

    pub fn as_millis(&self) -> u128 {
        self.as_duration().num_milliseconds() as u128
    }

    pub fn is_greater_or_equal(&self, other: &Resolution) -> bool {
        self.as_duration() >= other.as_duration()
    }

    pub fn to_string(&self) -> String {
        match self {
            Resolution::Instant => "I".to_string(),
            Resolution::Ticks(val) => format!("{}-T", val),
            Resolution::Seconds(val) => format!("{}-S", val),
            Resolution::Minutes(val) => format!("{}-M", val),
            Resolution::Hours(val) => format!("{}-H", val),
            Resolution::Days(val) => format!("{}-D", val),
            Resolution::Weeks(val) => format!("{}-W", val),
        }
    }
}

/// eg: Second(5) would represent a 5-Second resolution
impl fmt::Display for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Resolution::Instant => write!(f, "Instant"),
            Resolution::Ticks(val) => write!(f, "{}-Tick", val),
            Resolution::Seconds(val) => write!(f, "{}-Second", val),
            Resolution::Minutes(val) => write!(f, "{}-Minute", val),
            Resolution::Hours(val) => write!(f, "{}-Hour", val),
            Resolution::Days(val) => write!(f, "{}-Days", val),
            Resolution::Weeks(val) => write!(f, "{}-Week", val),
        }
    }
}