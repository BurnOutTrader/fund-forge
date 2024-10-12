use std::path::PathBuf;
use std::fs;
use base64::engine::general_purpose;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tokio::net::TcpStream;
use std::time::{SystemTime, UNIX_EPOCH};
use tungstenite::Message;
use serde_json::from_str;
use serde_derive::{Deserialize, Serialize};
use futures::{SinkExt, StreamExt};
use hmac::{Hmac, Mac};
use base64::Engine;
use sha2::Sha256;
use crate::helpers::get_data_folder;
use crate::messages::data_server_messaging::FundForgeError;
type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Serialize, Deserialize)]
struct LoginRequest {
    op: String,
    args: Vec<LoginArgs>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoginResponse {
    event: String,
    code: String,
    msg: String
}

#[derive(Debug, Serialize, Deserialize)]
struct LoginArgs {
    api_key: String,
    passphrase: String,
    timestamp: String,
    sign: String,
}

pub async fn login(credentials: &BitGetCredentials, stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>) -> Result<(), FundForgeError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();

    let sign = match generate_signature(&credentials.secret_key, &timestamp) {
        Ok(sign) => sign,
        Err(e) => {
            return Err(FundForgeError::ServerErrorDebug(format!("Error generating Bitget cryptographic key for sign in: {}", e)))
        }
    };

    let login_request = LoginRequest {
        op: "login".to_string(),
        args: vec![LoginArgs {
            api_key: credentials.api_key.clone(),
            passphrase: credentials.passphrase.clone(),
            timestamp: timestamp.clone(),
            sign,
        }],
    };

    let login_json = match serde_json::to_string(&login_request) {
        Ok(login_string) => login_string,
        Err(e) => return Err(FundForgeError::ServerErrorDebug(format!("Error parsing login json: {}", e)))
    };

    match stream.send(Message::Text(login_json)).await {
        Ok(_) => {}
        Err(e) => return Err(FundForgeError::ServerErrorDebug(format!("Error sending login json: {}", e)))
    };

    // Wait for login response
    if let Some(msg) = stream.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let response: LoginResponse = from_str(&text)
                    .map_err(|e| FundForgeError::ServerErrorDebug(format!("Failed to parse login response: {}", e)))?;

                match response.event.as_str() {
                    "login" => {
                        if response.code == "0" {
                            println!("Login successful");
                            Ok(())
                        } else {
                            Err(FundForgeError::ServerErrorDebug(format!("Login failed with code: {}, msg: {}", response.code, response.msg)))
                        }
                    },
                    "error" => Err(FundForgeError::ServerErrorDebug(format!("Login error: code {}, msg: {}", response.code, response.msg))),
                    _ => Err(FundForgeError::ServerErrorDebug(format!("Unexpected login response: {:?}", response))),
                }
            },
            Ok(_) => Err(FundForgeError::ServerErrorDebug("Received non-text message during login".into())),
            Err(e) => Err(FundForgeError::ServerErrorDebug(format!("WebSocket error during login: {}", e))),
        }
    } else {
        Err(FundForgeError::ServerErrorDebug("No response received for login request".into()))
    }
}

fn generate_signature(secret_key: &str, timestamp: &str) -> Result<String, Box<dyn std::error::Error>> {
    let message = format!("{}GET/user/verify", timestamp);

    // Create a new HMAC-SHA256 instance using the secret key
    let mut mac = HmacSha256::new_from_slice(secret_key.as_bytes())?;

    // Update the MAC instance with the message
    mac.update(message.as_bytes());

    // Finalize the MAC computation and get the result
    let result = mac.finalize().into_bytes();

    // Base64 encode the result
    Ok(general_purpose::STANDARD.encode(result))
}

pub fn get_bitget_credentials() -> Option<BitGetCredentials> {
    let file_path = PathBuf::from(get_data_folder())
        .join("bitget_credentials")
        .join("bitget_credentials.toml");

    if file_path.is_file() && file_path.extension().and_then(|s| s.to_str()) == Some("toml") {
        if let Ok(contents) = fs::read_to_string(file_path) {
            // Parse the TOML content into BitGetCredentials struct
            if let Ok(credentials) = toml::from_str::<BitGetCredentials>(&contents) {
                return Some(credentials);
            }
        }
    }

    None
}

#[derive(Deserialize, Serialize)]
pub struct BitGetCredentials {
    api_key: String,
    secret_key: String,
    passphrase: String,
}