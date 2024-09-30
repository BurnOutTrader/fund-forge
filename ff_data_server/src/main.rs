use chrono::Utc;
use ff_standard_lib::server_connections::ConnectionType;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use rustls_pemfile::{certs, private_key};
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use structopt::StructOpt;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::task;
use tokio::task::JoinHandle;
use tokio_rustls::{TlsAcceptor};
use ff_standard_lib::servers::settings::client_settings::initialise_settings;
use ff_standard_lib::standardized_types::data_server_messaging::DataServerRequest;
use ff_standard_lib::standardized_types::enums::StrategyMode;
use crate::request_handlers::data_server_manage_async_requests;

pub mod request_handlers;
mod tests;

#[derive(Debug)]
pub enum StartUpMode {
    Toml,
    CmdLine,
}

#[derive(Debug, StructOpt)]
struct ServerLaunchOptions {
    /// Sets the data folder
    #[structopt(
        short = "f",
        long = "data_folder",
        parse(from_os_str),
        default_value = "./data"
    )]
    pub data_folder: PathBuf,

    #[structopt(
        short = "l",
        long = "ssl_folder",
        parse(from_os_str),
        default_value = "./resources/keys"
    )]
    pub ssl_auth_folder: PathBuf,

    #[structopt(
        short = "s",
        long = "synchronous_address",
        default_value = "0.0.0.0:8080"
    )]
    pub listener_address: String,

    #[structopt(short = "p", long = "async_address", default_value = "0.0.0.0:8081")]
    pub async_listener_address: String,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let settings_map = initialise_settings().unwrap();
    let options = ServerLaunchOptions::from_args();
    let settings = settings_map.get(&ConnectionType::Default).unwrap();

    let cert = Path::join(&options.ssl_auth_folder, "cert.pem");
    let key = Path::join(&options.ssl_auth_folder, "key.pem");

    let certs = load_certs(&cert)?;
    let key = load_keys(&key)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No keys found"))?;

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;

    let async_server_handle = async_server(config.into(), settings.address).await;

    let async_result = async_server_handle.await;

    if let Err(e) = async_result {
        eprintln!("Asynchronous server failed: {:?}", e);
    }

    Ok(())
}

pub(crate) async fn async_server(config: ServerConfig, addr: SocketAddr) -> JoinHandle<()> {
    const LENGTH: usize = 8;
    task::spawn(async move {
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = match TcpListener::bind(&addr).await {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Failed to listen on {}: {}", addr, e);
                return;
            }
        };
        println!("Listening on: {}", addr);

        loop {
            let (stream, peer_addr) = match listener.accept().await {
                Ok((stream, peer_addr)) => (stream, peer_addr),
                Err(e) => {
                    eprintln!("Failed to accept TLS connection: {:?}", e);
                    continue;
                }
            };
            println!("{}, peer_addr: {:?}", Utc::now(), peer_addr);
            let acceptor = acceptor.clone();

            let mut tls_stream = match acceptor.accept(stream).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("Failed to accept TLS connection: {:?}", e);
                    return;
                }
            };

            let mut length_bytes = [0u8; LENGTH];
            let mut mode = StrategyMode::Backtest;
            'registration_loop: while let Ok(_) = tls_stream.read_exact(&mut length_bytes).await {
                // Parse the length from the header
                let msg_length = u64::from_be_bytes(length_bytes) as usize;
                let mut message_body = vec![0u8; msg_length];

                // Read the message body based on the length
                match tls_stream.read_exact(&mut message_body).await {
                    Ok(_) => {},
                    Err(e) => {
                        eprintln!("Error reading message body: {}", e);
                        continue;
                    }
                }

                // Parse the request from the message body
                let request = match DataServerRequest::from_bytes(&message_body) {
                    Ok(req) => req,
                    Err(e) => {
                        eprintln!("Failed to parse request: {:?}", e);
                        continue;
                    }
                };
                println!("{:?}", request);
                // Handle the request and generate a response
                match request {
                    DataServerRequest::Register(registered_mode) => {
                        mode = registered_mode;
                        break 'registration_loop
                    },
                    _ => eprintln!("Strategy Did not register a Strategy mode")
                }
            }
            println!("TLS connection established with {:?}", peer_addr);

            data_server_manage_async_requests(
                mode,
                tls_stream
            )
            .await;
        }
    })
}

pub(crate) fn load_certs(path: &Path) -> io::Result<Vec<CertificateDer<'static>>> {
    let certificates = certs(&mut BufReader::new(File::open(path)?)).collect();
    //println!("certs: {:?}", certs(&mut BufReader::new(File::open(path)?)).collect::<Vec<_>>());
    certificates
}

pub(crate) fn load_keys(path: &Path) -> Option<PrivateKeyDer<'static>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return None,
    };
    private_key(&mut BufReader::new(file)).unwrap()
}
