use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, Read, Write};
use toml;
use ff_standard_lib::apis::rithmic::rithmic_systems::RithmicSystem;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::rithmic_api::client_base::errors::RithmicApiError;
use crate::rithmic_api::client_base::servers::RithmicServer;

#[derive(Serialize, Deserialize, Clone, Eq, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Hash, PartialOrd, Ord)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct RithmicCredentials {
    pub user: String,
    pub server_name: RithmicServer,
    pub system_name: RithmicSystem,
    pub app_name: String,
    pub app_version: String,
    pub password: String,
    pub fcm_id: Option<String>,
    pub ib_id: Option<String>,
    pub user_type: Option<i32>,
    pub subscribe_data: bool,
    pub aggregated_quotes: bool
}

#[allow(dead_code)]
impl RithmicCredentials {
    fn from_bytes(archived: &[u8]) -> Result<RithmicCredentials, RithmicApiError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<RithmicCredentials>(archived) {
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

impl RithmicCredentials {
    pub fn save_credentials_to_file(&self, file_path: &str) -> io::Result<()> {
        // Convert the credentials to TOML string
        let toml_string = toml::to_string(self).expect("Failed to serialize credentials");

        // Write the TOML string to the file
        let mut file = File::create(file_path)?;
        file.write_all(toml_string.as_bytes())?;

        Ok(())
    }

    pub fn load_credentials_from_file(file_path: &str) -> io::Result<RithmicCredentials> {
        // Read the TOML string from the file
        let mut file = File::open(file_path)?;
        let mut toml_string = String::new();
        file.read_to_string(&mut toml_string)?;

        // Parse the TOML string into Credentials
        let credentials: RithmicCredentials = toml::de::from_str(&toml_string)
            .expect("Failed to deserialize credentials");

        Ok(credentials)
    }

    pub fn file_name(&self) -> String {
        self.system_name.file_string()
    }
}
