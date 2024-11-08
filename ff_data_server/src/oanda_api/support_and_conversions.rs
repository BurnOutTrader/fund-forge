use std::fmt;
use std::str::FromStr;
use chrono::{Duration};
use structopt::StructOpt;
use ff_standard_lib::standardized_types::resolution::Resolution;
///The resolutions supported by Oanda Brokerage oanda
#[derive(PartialEq, Ord, PartialOrd, Eq, Clone, Debug, StructOpt)]
pub(crate) enum Interval {
    S5,
    M1,
    H1,
}

impl Interval {
    #[allow(dead_code)]
    pub(crate) fn oanda_interval_to_resolution(&self) -> Resolution {
        match self {
            Interval::S5 => Resolution::Seconds(5),
            Interval::M1 => Resolution::Minutes(1),
            Interval::H1 => Resolution::Hours(1),
        }
    }
}

pub(crate) fn resolution_to_oanda_interval(resolution: &Resolution) -> Option<Interval> {
    match resolution {
        Resolution::Seconds(5) => Some(Interval::S5),
        Resolution::Minutes(1) => Some(Interval::M1),
        Resolution::Hours(1) => Some(Interval::H1),
        _ => None,
    }
}

pub(crate) fn add_time_to_date(resolution: &Interval) -> Duration {
    match resolution {
        Interval::S5 => Duration::hours(6) + Duration::minutes(50),
        Interval::M1 => Duration::days(5),
        Interval::H1 => Duration::days(200),
    }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            Interval::S5 => "S5",
            Interval::M1 => "M1",
            Interval::H1 => "H1",
        })
    }
}

impl FromStr for Interval {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "S5" => Ok(Interval::S5),
            "M1" => Ok(Interval::M1),
            "H1" => Ok(Interval::H1),
            _ => Err(format!("{} is not a valid resolution", s)),
        }
    }
}