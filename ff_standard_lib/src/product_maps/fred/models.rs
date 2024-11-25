use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::standardized_types::resolution::Resolution;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug, Copy)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum FredDataSetEnum {
    GrossNationalProductPerCapita,
}

pub fn fred_data_resolutions(data: FredDataSetEnum) -> Vec<Resolution> {
    match data {
        FredDataSetEnum::GrossNationalProductPerCapita => vec![Resolution::Year],
    }
}

pub fn fred_release_time_of_day(data: FredDataSetEnum) -> (u8, u8) {
    match data {
        _ => (7, 30),
    }
}

impl fmt::Display for FredDataSetEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FredDataSetEnum::GrossNationalProductPerCapita => write!(f, "GNPCA"),
        }
    }
}

impl FromStr for FredDataSetEnum {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GNPCA" => Ok(FredDataSetEnum::GrossNationalProductPerCapita),
            _ => Err(format!("Unknown dataset: {}", s)),
        }
    }
}

pub fn get_fed_api_country_format(country_code: &str) -> Option<&'static str> {
    let mut country_map: HashMap<&str, &str> = HashMap::new();

    // Example country code to full country name mappings
    country_map.insert("USA", "united states");
    country_map.insert("CAN", "canada");
    country_map.insert("DEU", "germany");
    country_map.insert("GBR", "united kingdom");
    country_map.insert("FRA", "france");
    country_map.insert("AUS", "australia");  // Add more as needed

    // Look up the country code in the map
    country_map.get(country_code).copied()
}