use std::fmt;
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
}

impl Default for Resolution {
    fn default() -> Self {
        Resolution::Instant
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
        }
    }

    pub fn number_of(&self) -> u64 {
        match self {
            Resolution::Instant => 0,
            Resolution::Ticks(val) => val.clone(),
            Resolution::Seconds(val) => val.clone(),
            Resolution::Minutes(val) => val.clone(),
            Resolution::Hours(val) => val.clone(),
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

    pub fn from_str(resolution_string: &str) -> Option<Self> {
        let resolution_string = resolution_string.to_uppercase().clone();
        let split_string = resolution_string.split("_").collect::<Vec<&str>>();
        let (number, res) = split_string.split_at(1);
        let number = number[0].parse::<u64>().unwrap();
        match res[0] {
            "I" => Some(Resolution::Instant),
            "T" => Some(Resolution::Ticks(number)),
            "S" => Some(Resolution::Seconds(number)),
            "M" => Some(Resolution::Minutes(number)),
            "H" => Some(Resolution::Hours(number)),
            _ => None,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Resolution::Instant => "I".to_string(),
            Resolution::Ticks(val) => format!("{}-T", val),
            Resolution::Seconds(val) => format!("{}-S", val),
            Resolution::Minutes(val) => format!("{}-M", val),
            Resolution::Hours(val) => format!("{}-H", val),
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
        }
    }
}