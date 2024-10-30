use std::fs;
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
    pub(crate) max_concurrent_downloads: usize,
}

impl OandaSettings {
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let settings: OandaSettings = toml::from_str(&contents)?;
        Ok(settings)
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let toml_string = toml::to_string_pretty(self)?;
        fs::write(path, toml_string)?;
        Ok(())
    }
}
