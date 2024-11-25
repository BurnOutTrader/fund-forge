use std::fmt;
use std::str::FromStr;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::standardized_types::resolution::Resolution;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug, Copy)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum FredDataSet {
    GrossNationalProductPerCapita,
    GrossDomesticProduct,
}

pub fn fred_data_resolutions(data: FredDataSet) -> Vec<Resolution> {
    match data {
        FredDataSet::GrossNationalProductPerCapita => vec![Resolution::Year],
        FredDataSet::GrossDomesticProduct => vec![Resolution::Year],
    }
}

impl fmt::Display for FredDataSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FredDataSet::GrossNationalProductPerCapita => write!(f, "GNPCA"),
            FredDataSet::GrossDomesticProduct => write!(f, "GDP"),
        }
    }
}

impl FromStr for FredDataSet {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GNPCA" => Ok(FredDataSet::GrossNationalProductPerCapita),
            "GDP" => Ok(FredDataSet::GrossDomesticProduct),
            _ => Err(format!("Unknown dataset: {}", s)),
        }
    }
}