use std::fmt;
use std::str::FromStr;
use ff_rithmic_api::systems::RithmicSystem;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use serde_derive::{Deserialize, Serialize};
use crate::messages::data_server_messaging::FundForgeError;

#[derive(Serialize, Deserialize, Clone, Eq, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Hash, PartialOrd, Ord)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
/// A `DataVendor` enum is a company that provides the data that is used to feed the algorithm.
/// The `DataVendor` is used to specify the data vendor that is being used to feed a `Subscription`.
/// Each `DataVendor` implements its own logic to fetch the data from the source, this logic can be modified in the `ff_data_server` crate.
pub enum DataVendor {
    Test, //DO NOT CHANGE ORDER
    Rithmic(RithmicSystem),
    Bitget
}

impl fmt::Display for DataVendor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DataVendor::Test => "Test".to_string(),
            DataVendor::Rithmic(system) => format!("Rithmic {}", system.to_string()),
            DataVendor::Bitget => "Bitget".to_string(),
        };
        write!(f, "{}", s)
    }
}

impl FromStr for DataVendor {
    type Err = FundForgeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "Test" {
            Ok(DataVendor::Test)
        } else if s.starts_with("Rithmic") {
            let system_name = s.trim_start_matches("Rithmic ");
            if let Some(system) = RithmicSystem::from_string(system_name) {
                Ok(DataVendor::Rithmic(system))
            } else {
                Err(FundForgeError::ClientSideErrorDebug(format!(
                    "Unknown RithmicSystem string: {}",
                    system_name
                )))
            }
        } else if s == "BitGet" {
            Ok(DataVendor::Bitget)
        } else {
            Err(FundForgeError::ClientSideErrorDebug(format!(
                "Unknown DataVendor string: {}",
                s
            )))
        }
    }
}