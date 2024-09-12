use async_trait::async_trait;
use lazy_static::lazy_static;
use serde_derive::Deserialize;
use crate::apis::brokerage::server_responses::BrokerApiResponse;
use crate::apis::rithmic::rithmic_proto_objects::rti::{RequestHeartbeat, RequestLogin, RequestLogout, RequestRithmicSystemInfo, ResponseLogin, ResponseLogout, ResponseRithmicSystemInfo};
use crate::apis::vendor::server_responses::VendorApiResponse;
use crate::standardized_types::accounts::ledgers::{AccountId, Currency};
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::data_server_messaging::{AddressString, FundForgeError, SynchronousResponseType};
use crate::standardized_types::enums::{MarketType, Resolution, SubscriptionResolutionType};
use crate::standardized_types::subscriptions::SymbolName;
use std::fs::File;
use std::io::{BufReader, Cursor};
use tokio::io;
use std::path::Path;
use prost::Message as ProstMessage;
use rustls::{ClientConfig, RootCertStore};
use rustls::pki_types::{CertificateDer, ServerName};
use rustls_pemfile::certs;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use futures_util::{SinkExt, StreamExt, TryFutureExt};
use std::error::Error;
use std::sync::Arc;
use tokio_rustls::{TlsConnector, TlsStream};
use tokio_tungstenite::tungstenite::WebSocket;
use crate::apis::rithmic::rithmic_proto_objects::rti::request_login::SysInfraType;

#[derive(Debug, Deserialize)]
pub struct Credentials {
    pub uri: String,
    pub user: String,
    pub system_name: String,
    pub password: String,
    pub app_name: String,
    pub app_version: String,
    pub aggregated_quotes: bool,
    pub template_version: String,
    pub pem: String,
    pub base_url: String
}
/*
lazy_static!(
    RITHMIC_API_CLIENT:
)*/

///Server uses Big Endian format for binary data
pub struct RithmicApiClient {
    credentials: Credentials,
}

impl RithmicApiClient {
    pub fn new(credentials: Credentials, ) -> Self {
        Self {
            credentials,
        }
    }
    pub(crate) fn load_certs(path: &Path) -> io::Result<Vec<CertificateDer<'static>>> {
        let certificates = certs(&mut BufReader::new(File::open(path)?)).collect();
        //println!("certs: {:?}", certs(&mut BufReader::new(File::open(path)?)).collect::<Vec<_>>());
        certificates
    }

    fn create_client_config(cert_path: &Path) -> ClientConfig {
        // Load the certificate
        let certs = RithmicApiClient::load_certs(cert_path);

        // Create a root certificate store and add the loaded certs
        let mut root_cert_store = RootCertStore::empty();
        for cert in &certs {
            root_cert_store.add(cert.first().unwrap().clone()).expect("failed to add certificate to root store");
        }

        // Initialize ClientConfig using safe defaults and configure as required
        ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth()
    }

    async fn send_single_protobuf_message<T: ProstMessage>(stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>, message: &T) -> Result<(), Box<dyn Error>> {
        let mut buf = Vec::new();
        message.encode(&mut buf)?;
        let length = buf.len() as u32;
        let mut prefixed_msg = length.to_be_bytes().to_vec();
        prefixed_msg.extend(buf);

        stream.send(Message::Binary(prefixed_msg)).await?;
        //println!("Message sent");
        Ok(())
    }

    async fn read_single_protobuf_message<T: ProstMessage + Default>(
        stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>
    ) -> Result<T, Box<dyn Error>> {
        while let Some(msg) = stream.next().await {
            let msg = msg?;
            if let Message::Binary(data) = msg {
                //println!("Received binary data: {:?}", data);

                // Create a cursor for reading the data
                let mut cursor = Cursor::new(data);

                // Read the 4-byte length header
                let mut length_buf = [0u8; 4];
                cursor.read_exact(&mut length_buf).map_err(|e| Box::new(e) as Box<dyn Error>).await?;
                let length = u32::from_be_bytes(length_buf) as usize;

                // Read the Protobuf message
                let mut message_buf = vec![0u8; length];
                cursor.read_exact(&mut message_buf).map_err(|e| Box::new(e) as Box<dyn Error>).await?;

                // Decode the Protobuf message
                let decoded_msg = T::decode(&message_buf[..]).map_err(|e| Box::new(e) as Box<dyn Error>)?;
                return Ok(decoded_msg);
            }
        }
        Err("No valid message received".into())
    }
    pub async fn connect_and_login(&self) -> Result<(), FundForgeError> {
        // establish TCP connection to get the server details
       let (mut stream, response) = connect_async(self.credentials.base_url.clone()).await.unwrap();

        println!("Rithmic connection: {:?}", response);

        // Rithmic System Info Request 16 From Client
        let request = RequestRithmicSystemInfo {
            template_id: 16,
            user_msg: vec!["Rust Fund Forge Signing In".to_string()],
        };

        //sleep(Duration::from_millis(10000));
        RithmicApiClient::send_single_protobuf_message(&mut stream, &request).await.unwrap();
        // Rithmic System Info Response 17
        // Step 2: Read the full message based on the length
        let message: ResponseRithmicSystemInfo = RithmicApiClient::read_single_protobuf_message(&mut stream).await.unwrap();

        // Now we have the system name we can do the handshake
        let rithmic_server_name = match message.system_name.first() {
            Some(name) => name.clone(),
            None => {
                return Err(FundForgeError::ServerErrorDebug(
                    "No system name found in response".to_string(),
                ));
            }
        };
        println!("{}", rithmic_server_name);
        stream.close(None).await.map_err(|e| Box::new(e) as Box<dyn Error>).unwrap();

        let (mut stream, response) = connect_async(self.credentials.base_url.clone()).await.unwrap();
        // After handshake, we can send confidential data
        // Login Request 10 From Client
        let login_request = RequestLogin {
            template_id: 10,
            template_version: Some(self.credentials.template_version.clone()),
            user_msg: vec![],
            user: Some(self.credentials.user.clone()),
            password: Some(self.credentials.password.clone()),
            app_name: Some(self.credentials.app_name.clone()),
            app_version: Some(self.credentials.app_version.clone()),
            system_name: Some(rithmic_server_name),
            infra_type: Some(1),
            mac_addr: vec![],
            os_version: None,
            os_platform: None,
            aggregated_quotes: Some(self.credentials.aggregated_quotes.clone()),
        };
        RithmicApiClient::send_single_protobuf_message(&mut stream, &login_request).await.unwrap();

        // Login Response 11 From Server
        let message: ResponseLogin = RithmicApiClient::read_single_protobuf_message(&mut stream).await.unwrap();
        println!("{:?}", message);
        RithmicApiClient::logout(&mut stream).await.unwrap();
        Ok(())
    }

    pub async fn logout(stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>) -> Result<(), FundForgeError> {
        //Logout Request 12
        let logout_request = RequestLogout {
            template_id: 12,
            user_msg: vec!["Rust Fund Forge Signing Out".to_string()],
        };
        RithmicApiClient::send_single_protobuf_message(stream, &logout_request).await.unwrap();
        //Logout Response 13
        let response: ResponseLogout = RithmicApiClient::read_single_protobuf_message(stream).await.unwrap();
        println!("Rithmic logout response: {:?}", response);
        stream.close(None).await.map_err(|e| Box::new(e) as Box<dyn Error>).unwrap();
       Ok(())
    }

    /// Use this when we don't have any active subscriptions to persist the connection
    pub async fn idle_heart_beat(&self) {
       // Heartbeats
       /* Heartbeats responses from the server are a way of monitoring the communication link between client and server.
       Upon making a successful login to the Rithmic Infrastructure, clients are expected to send at least a heartbeat request
       (if no other requests are sent) to the server in order to keep the connection active. The heartbeat interval is specified in the login response.
        If clients don’t subscribe to any updates, nor send any queries, including heartbeats, then over a threshold amount of time the server will terminate
        such connections for not keeping the link active.
        Heartbeat requests from clients are not required when the client application is already receiving updates or responses from the server within the threshold period.*/
    }
}

#[async_trait]
impl VendorApiResponse for RithmicApiClient {
    async fn basedata_symbols_response(&self, market_type: MarketType) -> Result<SynchronousResponseType, FundForgeError> {
        match market_type {
            MarketType::Futures => todo!(),
            _ => Err(FundForgeError::ClientSideErrorDebug(format!("Unsupported market type: {}, for Rithmic", market_type)))
        }
    }

    async fn resolutions_response(&self, market_type: MarketType) -> Result<SynchronousResponseType, FundForgeError> {
        match market_type {
            MarketType::Futures => {
                Ok(SynchronousResponseType::Resolutions(vec![
                    SubscriptionResolutionType::new(Resolution::Ticks(1), BaseDataType::Ticks),
                    SubscriptionResolutionType::new(Resolution::Instant, BaseDataType::Quotes)
                ], MarketType::Futures))
            }
            _ => Err(FundForgeError::ClientSideErrorDebug(format!("Incorrect Market Type: {}, for Rithmic", market_type)))
        }
    }

    async fn markets_response(&self) -> Result<SynchronousResponseType, FundForgeError> {
        Ok(SynchronousResponseType::Markets(vec![MarketType::Futures]))
    }

    async fn decimal_accuracy_response(&self, symbol_name: SymbolName) -> Result<SynchronousResponseType, FundForgeError> {
        todo!("Need to serialize this list so we dont have to get it from server")
    }

    async fn tick_size_response(&self, symbol_name: SymbolName) -> Result<SynchronousResponseType, FundForgeError> {
        todo!()
    }
}

#[async_trait]
impl BrokerApiResponse for RithmicApiClient {
    async fn symbols_response(&self, market_type: MarketType) -> Result<SynchronousResponseType, FundForgeError> {
        todo!()
    }

    async fn account_currency_response(&self, account_id: AccountId) -> Result<SynchronousResponseType, FundForgeError> {
    /*    if self.has_account(&account_id) {

        }*/
        Ok(SynchronousResponseType::AccountCurrency(account_id, Currency::USD))
    }

    async fn account_info_response(&self, account_id: AccountId) -> Result<SynchronousResponseType, FundForgeError> {
        todo!()
    }

    async fn symbol_info_response(&self, symbol_name: SymbolName) -> Result<SynchronousResponseType, FundForgeError> {
        todo!()
    }
}