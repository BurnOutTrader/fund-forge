use std::fmt;
use std::str::FromStr;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum FredDataSet {
    GrossNationalProductPerCapita,
}

impl fmt::Display for FredDataSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FredDataSet::GrossNationalProductPerCapita => write!(f, "GNPCA"),
        }
    }
}

impl FromStr for FredDataSet {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GNPCA" => Ok(FredDataSet::GrossNationalProductPerCapita),
            _ => Err(format!("Unknown dataset: {}", s)),
        }
    }
}