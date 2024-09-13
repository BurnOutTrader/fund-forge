use std::hash::Hash;
use std::thread::sleep;
use std::time::Duration;
use ff_standard_lib::apis::rithmic::api_client::{Credentials, RithmicApiClient};
use ff_standard_lib::apis::rithmic::rithmic_proto_objects::rti::request_login::SysInfraType;

#[tokio::main]
async fn main() {
    let credentials = Credentials {
        uri: "".to_string(),
        user: "Kevinjamesmonaghan@outlook.com".to_string(),
        system_name: "".to_string(),
        password: "cDbJbQLV".to_string(),
        app_name: "Fund-Forge".to_string(),
        app_version: "1.0".to_string(),
        aggregated_quotes: false,
        template_version: "5.27".to_string(),
        pem: String::from("ff_standard_lib/src/apis/rithmic/rithmic_ssl_cert_auth_params.pem"),
        base_url: "wss://rituz00100.rithmic.com:443".to_string()
    };

    let rithmic_api = RithmicApiClient::new(credentials);

    /*
    pub enum SysInfraType {
       TickerPlant = 1,
       OrderPlant = 2,
       HistoryPlant = 3,
       PnlPlant = 4,
       RepositoryPlant = 5,
   }
   */

    // Use the path as needed
    let (ws_writer_ticker_plant, ws_reader_ticker_plant) = rithmic_api.connect_and_login(SysInfraType::TickerPlant).await.unwrap();

    let (ws_writer_history_plant, ws_reader_history_plant) = rithmic_api.connect_and_login(SysInfraType::HistoryPlant).await.unwrap();

    sleep(Duration::from_secs(5));
    match RithmicApiClient::shutdown_split_websocket(ws_writer_ticker_plant, ws_reader_ticker_plant).await {
        Ok(_) => println!("Success"),
        Err(e) => println!("{}", e)
    }

    match RithmicApiClient::shutdown_split_websocket(ws_writer_history_plant, ws_reader_history_plant).await {
        Ok(_) => println!("Success"),
        Err(e) => println!("{}", e)
    }
}