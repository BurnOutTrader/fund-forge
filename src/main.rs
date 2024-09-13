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
    rithmic_api.connect_and_login(SysInfraType::TickerPlant).await.unwrap();
    rithmic_api.connect_and_login(SysInfraType::HistoryPlant).await.unwrap();
    rithmic_api.connect_and_login(SysInfraType::OrderPlant).await.unwrap();
    rithmic_api.connect_and_login(SysInfraType::PnlPlant).await.unwrap();
    rithmic_api.connect_and_login(SysInfraType::RepositoryPlant).await.unwrap();

    sleep(Duration::from_secs(15));
    rithmic_api.shutdown_all().await.unwrap();
}