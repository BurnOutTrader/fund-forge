use std::collections::{BTreeMap};
use std::io::{Cursor};
use dashmap::DashMap;
use prost::{Message as ProstMessage};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::{SplitSink};
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_login::SysInfraType;
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::{RequestLogin, RequestLogout, RequestRithmicSystemInfo, ResponseLogin, ResponseRithmicSystemInfo};
use prost::encoding::{decode_key, decode_varint, WireType};
use tokio::sync::RwLock;
use crate::rithmic_api::client_base::credentials::RithmicCredentials;
use crate::rithmic_api::client_base::errors::RithmicApiError;
use crate::rithmic_api::client_base::servers::{server_domains, RithmicServer};

pub const TEMPLATE_VERSION: &str = "5.29";

///Server uses Big Endian format for binary data
pub struct RithmicApiClient {
    /// Credentials used for this instance of the api. we can have multiple instances for different brokers.
    credentials: RithmicCredentials,

    pub fcm_id:RwLock<Option<String>>,

    pub ib_id:RwLock<Option<String>>,

    server_domains: BTreeMap<RithmicServer, String>,

    pub heartbeat_interval_seconds: DashMap<SysInfraType, u64>
}

impl RithmicApiClient {
    pub fn new(
        credentials: RithmicCredentials,
        server_domains_toml: String,
    ) -> Result<Self, RithmicApiError> {
        let server_domains = server_domains(server_domains_toml)?;
        Ok(Self {
            credentials,
            fcm_id: RwLock::new(None),
            ib_id: RwLock::new(None),
            server_domains,
            heartbeat_interval_seconds: DashMap::with_capacity(5),
        })
    }

    /// only used to register and login before splitting the stream.
    async fn send_single_protobuf_message<T: ProstMessage>(
        stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>, message: &T
    ) -> Result<(), RithmicApiError> {
        let mut buf = Vec::new();

        match message.encode(&mut buf) {
            Ok(_) => {}
            Err(e) => return Err(RithmicApiError::ServerErrorDebug(format!("Failed to encode RithmicMessage: {}", e)))
        }

        let length = buf.len() as u32;
        let mut prefixed_msg = length.to_be_bytes().to_vec();
        prefixed_msg.extend(buf);
        stream.send(Message::Binary(prefixed_msg)).await.map_err(|e| RithmicApiError::WebSocket(e))
    }

    /// Used to receive system and login response before splitting the stream.
    async fn read_single_protobuf_message<T: ProstMessage + Default>(
        stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>
    ) -> Result<T, RithmicApiError> {
        while let Some(msg) = stream.next().await { //todo change from while to if and test
            let msg = match msg {
                Ok(msg) => msg,
                Err(e) => return Err(RithmicApiError::ServerErrorDebug(format!("Failed to read RithmicMessage: {}", e)))
            };
            if let Message::Binary(data) = msg {
                //println!("Received binary data: {:?}", data);

                // Create a cursor for reading the data
                let mut cursor = Cursor::new(data);

                // Read the 4-byte length header
                let mut length_buf = [0u8; 4];
                let _ = tokio::io::AsyncReadExt::read_exact(&mut cursor, &mut length_buf).await.map_err(RithmicApiError::Io);
                let length = u32::from_be_bytes(length_buf) as usize;

                // Read the Protobuf message
                let mut message_buf = vec![0u8; length];
                tokio::io::AsyncReadExt::read_exact(&mut cursor, &mut message_buf).await.map_err(RithmicApiError::Io)?;

                // Decode the Protobuf message
                return match T::decode(&message_buf[..]) {
                    Ok(decoded_msg) => Ok(decoded_msg),
                    Err(e) => Err(RithmicApiError::ProtobufDecode(e)), // Use the ProtobufDecode variant
                }
            }
        }
        Err(RithmicApiError::ServerErrorDebug("No valid message received".to_string()))
    }

    pub async fn get_systems(
        &self,
        plant: SysInfraType,
    ) -> Result<Vec<String>, RithmicApiError> {
        if plant as i32 > 5 {
            return Err(RithmicApiError::ClientErrorDebug("Incorrect value for rithmic SysInfraType".to_string()))
        }
        let domain = match self.server_domains.get(&self.credentials.server_name) {
            None => return Err(RithmicApiError::ServerErrorDebug(format!("No server domain found, check server.toml for: {:?}", self.credentials.server_name))),
            Some(domain) => domain
        };
        // establish TCP connection to get the server details
        let (mut stream, response) = match connect_async(domain).await {
            Ok((stream, response)) => (stream, response),
            Err(e) => return Err(RithmicApiError::ServerErrorDebug(format!("Failed to connect to rithmic: {}", e)))
        };
        println!("Rithmic connection established: {:?}", response);
        // Rithmic System Info Request 16 From Client
        let request = RequestRithmicSystemInfo {
            template_id: 16,
            user_msg: vec![format!("{} Signing In", self.credentials.app_name)],
        };

        RithmicApiClient::send_single_protobuf_message(&mut stream, &request).await?;
        // Rithmic System Info Response 17
        // Step 2: Read the full message based on the length
        let message: ResponseRithmicSystemInfo = RithmicApiClient::read_single_protobuf_message(&mut stream).await?;
        println!("{:?}", message);
        // Now we have the system name we can do the handshake
        Ok(message.system_name)
    }

    /// Connect to the desired plant and sign in with your credentials.
    pub async fn connect_and_login (
        &self,
        plant: SysInfraType,
    ) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, RithmicApiError> {

        let domain = match self.server_domains.get(&self.credentials.server_name) {
            None => return Err(RithmicApiError::ServerErrorDebug(format!("No server domain found, check server.toml for: {:?}", self.credentials.server_name))),
            Some(domain) => domain
        };

        let (mut stream, _) = match connect_async(domain).await {
            Ok((stream, response)) => (stream, response),
            Err(e) => return Err(RithmicApiError::ServerErrorDebug(format!("Failed to connect to rithmic, for login: {}", e)))
        };

        let aggregated_quotes = match self.credentials.aggregated_quotes {
            true => Some(true),
            false => Some(false)
        };

        // After handshake, we can send confidential data
        // Login Request 10 From Client
        let login_request = RequestLogin {
            template_id: 10,
            template_version: Some(TEMPLATE_VERSION.to_string()),
            user_msg: vec![],
            user: Some(self.credentials.user.clone()),
            password: Some(self.credentials.password.clone()),
            app_name: Some(self.credentials.app_name.clone()),
            app_version: Some(self.credentials.app_version.clone()),
            system_name: Some(self.credentials.system_name.to_string()),
            infra_type: Some(plant as i32),
            mac_addr: vec![],
            os_version: None,
            os_platform: None,
            aggregated_quotes,
        };

        RithmicApiClient::send_single_protobuf_message(&mut stream, &login_request).await?;

        // Login Response 11 From Server
        let response: ResponseLogin = RithmicApiClient::read_single_protobuf_message(&mut stream).await?;
        println!("{:?}:{:?}", response, plant);
        if response.rp_code.is_empty() {
            eprintln!("{:?}",response);
        }
        if response.rp_code[0] != "0".to_string() {
            eprintln!("{:?}",response);
        }

        match response.fcm_id {
            Some(fcm_id) => *self.fcm_id.write().await = Some(fcm_id),
            None => {}
        }

        match response.ib_id {
            Some(fcm_id) => *self.ib_id.write().await = Some(fcm_id),
            None => {}
        }

        if let Some(handshake_duration) = response.heartbeat_interval {
            self.heartbeat_interval_seconds.insert(plant, handshake_duration as u64);
        }

        Ok(stream)
    }

    /// Send a message on the write half of the plant stream.
    pub async fn send_message<T: ProstMessage>(
        &self,
        mut write_stream: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
        message: T
    ) -> Result<(), RithmicApiError> {
        let mut buf = Vec::new();

        match message.encode(&mut buf) {
            Ok(_) => {}
            Err(e) => return Err(RithmicApiError::ServerErrorDebug(format!("Failed to encode RithmicMessage: {}", e)))
        }

        let length = buf.len() as u32;
        let mut prefixed_msg = length.to_be_bytes().to_vec();
        prefixed_msg.extend(buf);

        match write_stream.send(Message::Binary(prefixed_msg)).await {
            Ok(_) => {
                Ok(())
            },
            Err(e) => Err(RithmicApiError::Disconnected(e.to_string()))
        }
    }

    /// Signs out of rithmic with the specific plant safely shuts down the web socket. removing references from our api object.
    pub async fn shutdown_plant(
        &self,
        mut write_stream: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    ) -> Result<(), RithmicApiError> {
        //Logout Request 12
        let logout_request = RequestLogout {
            template_id: 12,
            user_msg: vec![format!("{} Signing Out", self.credentials.app_name)],
        };

        let mut buf = Vec::new();
        match logout_request.encode(&mut buf) {
            Ok(_) => {}
            Err(e) => return Err(RithmicApiError::ServerErrorDebug(format!("Failed to encode RithmicMessage: {}", e)))
        }


        let length = buf.len() as u32;
        let mut prefixed_msg = length.to_be_bytes().to_vec();
        prefixed_msg.extend(buf);

        match write_stream.send(Message::Binary(prefixed_msg)).await {
            Ok(_) => Ok(()),
            Err(e) => Err(RithmicApiError::Disconnected(e.to_string()))
        }
    }
}

/// Dynamically get the template_id field from a generic type T.
pub fn extract_template_id(bytes: &[u8]) -> Option<i32> {
    let mut cursor = Cursor::new(bytes);
    while let Ok((field_number, wire_type)) = decode_key(&mut cursor) {
        if field_number == 154467 && wire_type == WireType::Varint {
            // We've found the template_id field
            return decode_varint(&mut cursor).ok().map(|v| v as i32);
        } else {
            // Skip this field
            match wire_type {
                WireType::Varint => { let _ = decode_varint(&mut cursor); }
                WireType::SixtyFourBit => { let _ = cursor.set_position(cursor.position() + 8); }
                WireType::LengthDelimited => {
                    if let Ok(len) = decode_varint(&mut cursor) {
                        let _ = cursor.set_position(cursor.position() + len as u64);
                    } else {
                        return None; // Error decoding length
                    }
                }
                WireType::StartGroup | WireType::EndGroup => {} // These are deprecated and shouldn't appear
                WireType::ThirtyTwoBit => { let _ = cursor.set_position(cursor.position() + 4); }
            }
        }
    }

    None // template_id field not found
}
