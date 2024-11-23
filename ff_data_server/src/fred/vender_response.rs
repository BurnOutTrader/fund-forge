use std::path::{Path, PathBuf};
use fred_rs::client::FredClient;
use fred_rs::series::{Builder, Response};
use tokio::sync::OnceCell;

static FRED_CLIENT: OnceCell<FredClient> = OnceCell::const_new();

async fn get_fred_client() -> &'static FredClient {
    FRED_CLIENT.get_or_init(|| async {
        FredClient::new().unwrap() // Initialize the client
    }).await
}

pub fn parse_fred_api_key(data_folder: PathBuf) -> Option<String> {

    let file_path = data_folder.join("credentials/fred_credentials/active/fred_credentials.toml");

    if !Path::new(&file_path).exists() {
        return None;
    }
    let credentials = match std::fs::read_to_string(file_path) {
        Ok(credentials) => credentials,
        Err(_) => return None
    };
    let credentials: toml::Value = match toml::from_str(&credentials) {
        Ok(credentials) => credentials,
        Err(_) => return None
    };
    match credentials["api_key"].as_str() {
        Some(api_key) => {
            //println!("FRED API key found: {}", api_key);
            Some(api_key.to_string())
        },
        None => None
    }
}

#[tokio::test]
async fn test_fred_client() {
    let data_folder = std::path::PathBuf::from("./data");
    let api_key = parse_fred_api_key(data_folder).unwrap();
    let mut client = FredClient::new().unwrap();
    client.with_key(&api_key);
    let mut builder = Builder::new();

    builder.realtime_start("2000-01-01");

    let resp: Response = match client.series("UNRATE", Some(builder)) {
        Ok(resp) => resp,
        Err(msg) => {
            println!("{}", msg);
            return
        },
    };

    for item in resp.seriess {
        println!(
            "{}: {} {} {}",
            item.id,
            item.title,
            item.realtime_start,
            item.realtime_end
        );
    }
}