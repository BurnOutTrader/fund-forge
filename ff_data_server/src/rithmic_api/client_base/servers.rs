use std::collections::BTreeMap;
use std::fs;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use toml::Value;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use strum_macros::Display;
use crate::rithmic_api::client_base::errors::RithmicApiError;

#[derive(Serialize, Deserialize, Clone, Eq, Serialize_rkyv, Deserialize_rkyv,
    Archive, PartialEq, Debug, Hash, PartialOrd, Ord, Display)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum RithmicServer {
    Chicago,
    Sydney,
    SaoPaolo,
    Colo75,
    Frankfurt,
    HongKong,
    Ireland,
    Mumbai,
    Seoul,
    CapeTown,
    Tokyo,
    Singapore,
    Test
}

#[allow(dead_code)]
impl RithmicServer {
    fn from_bytes(archived: &[u8]) -> Result<RithmicServer, RithmicApiError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<RithmicServer>(archived) {
            //Ignore this warning: Trait `Deserialize<ResponseType, SharedDeserializeMap>` is not implemented for `AccountInfoType` [E0277]
            Ok(response) => Ok(response),
            Err(e) => Err(RithmicApiError::ClientErrorDebug(e.to_string())),
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let vec = rkyv::to_bytes::<_, 256>(self).unwrap();
        vec.into()
    }
}
impl FromStr for RithmicServer {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Chicago" => Ok(RithmicServer::Chicago),
            "Sydney" => Ok(RithmicServer::Sydney),
            "SaoPaolo" => Ok(RithmicServer::SaoPaolo),
            "Colo75" => Ok(RithmicServer::Colo75),
            "Frankfurt" => Ok(RithmicServer::Frankfurt),
            "HongKong" => Ok(RithmicServer::HongKong),
            "Ireland" => Ok(RithmicServer::Ireland),
            "Mumbai" => Ok(RithmicServer::Mumbai),
            "Seoul" => Ok(RithmicServer::Seoul),
            "CapeTown" => Ok(RithmicServer::CapeTown),
            "Tokyo" => Ok(RithmicServer::Tokyo),
            "Singapore" => Ok(RithmicServer::Singapore),
            "Test" => Ok(RithmicServer::Test),
            _ => Err(format!("Unknown RithmicServer: {}", s)),
        }
    }
}

pub(crate) fn server_domains(file_path: String) -> Result<BTreeMap<RithmicServer, String>, RithmicApiError> {
    // Read the TOML file
    let toml_str = fs::read_to_string(&file_path)
        .map_err(|e| RithmicApiError::Io(e))?;

    // Trim the string and check for UTF-8 BOM
    let cleaned_str = toml_str.trim().strip_prefix("\u{FEFF}").unwrap_or(&toml_str.trim());

    // Parse the TOML string
    let toml_value: Value = toml::from_str(cleaned_str)
        .map_err(|e| {
            println!("TOML parse error: {:?}", e);
            println!("TOML content:\n{}", cleaned_str);
            RithmicApiError::TomlParse(e)
        })?;

    // Extract the rithmic_servers table
    let rithmic_servers = toml_value.get("rithmic_servers")
        .and_then(|v| v.as_table())
        .ok_or_else(|| {
            println!("Missing or invalid 'rithmic_servers' table in TOML");
            RithmicApiError::InvalidConfig("Missing 'rithmic_servers' table".to_string())
        })?;

    // Convert the TOML table to a BTreeMap with RithmicServer enum as keys
    rithmic_servers
        .iter()
        .map(|(k, v)| {
            let server = RithmicServer::from_str(k)
                .map_err(|e| {
                    println!("Invalid server name: {}", k);
                    RithmicApiError::InvalidServerName(e)
                })?;
            let domain = v.as_str()
                .ok_or_else(|| {
                    println!("Invalid domain for server {}: {:?}", k, v);
                    RithmicApiError::InvalidConfig(format!("Invalid domain for server {}", k))
                })?
                .to_string();
            Ok((server, domain))
        })
        .collect()
}