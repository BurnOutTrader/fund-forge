use std::hash::Hash;
use std::io::Cursor;
use std::thread::sleep;
use std::time::Duration;
use ff_rithmic_api::api_client::RithmicApiClient;
use ff_rithmic_api::credentials::RithmicCredentials;
use ff_rithmic_api::errors::RithmicApiError;
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
use ff_rithmic_api::rithmic_proto_objects::rti::RequestHeartbeat;
use tokio::sync::mpsc::Receiver;
use prost::{Message as RithmicMessage};

#[tokio::main]
async fn main() {
}
