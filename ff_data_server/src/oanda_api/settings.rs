use std::fs;
use std::path::PathBuf;
use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub enum OandaApiMode {
    Live,
    Practice,
}

#[derive(Serialize, Deserialize)]
pub struct OandaSettings {
    pub(crate) api_key: String,
    pub(crate) mode: OandaApiMode,
}

impl OandaSettings {
    pub fn from_file(path: PathBuf) -> Option<Self> {
        let contents = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error reading oanda settings file: {}", e);
                return None;
            }
        };
        let settings: OandaSettings = match toml::from_str(&contents) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error parsing oanda settings: {}", e);
                return None;
            }
        };
        Some(settings)
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let toml_string = toml::to_string_pretty(self)?;
        fs::write(path, toml_string)?;
        Ok(())
    }
}
