use chrono::Utc;
use ff_standard_lib::communicators::communications_async::{SecondaryDataReceiver, SecondaryDataSender};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use rustls_pemfile::{certs, private_key};
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use structopt::StructOpt;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task;
use tokio::task::JoinHandle;
use tokio_rustls::TlsAcceptor;
use ff_standard_lib::messages::registry_messages::RegistrationRequest;
use ff_standard_lib::traits::bytes::Bytes;

pub mod handle_gui;
pub mod handle_strategies;

pub(crate) fn load_certs(path: &Path) -> io::Result<Vec<CertificateDer<'static>>> {
    let certificates = certs(&mut BufReader::new(File::open(path)?)).collect();
    certificates
}

pub(crate) fn load_keys(path: &Path) -> Option<PrivateKeyDer<'static>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return None,
    };
    private_key(&mut BufReader::new(file)).unwrap()
}

#[derive(Debug, StructOpt)]
struct ServerLaunchOptions {
    #[structopt(
        short = "l",
        long = "ssl_folder",
        parse(from_os_str),
        default_value = "./resources/keys"
    )]
    pub ssl_auth_folder: PathBuf,

    #[structopt(
        short = "a",
        long = "address",
        default_value = "127.0.0.1"
    )]
    pub listener_address: IpAddr,

    #[structopt(
        short = "p",
        long = "port",
        default_value = "8083"
    )]
    pub port: u16,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let options = ServerLaunchOptions::from_args();

    let cert = Path::join(&options.ssl_auth_folder, "cert.pem");
    let key = Path::join(&options.ssl_auth_folder, "key.pem");

    let certs = load_certs(&cert).unwrap();
    let key = load_keys(&key)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No keys found"))
        .unwrap();

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))
        .unwrap();

    let address = SocketAddr::new(options.listener_address, options.port);
    let async_server_handle = async_server(config.into(), address).await;

    let async_result = async_server_handle.await;

    if let Err(e) = async_result {
        eprintln!("Asynchronous server failed: {:?}", e);
    }

    Ok(())
}

pub(crate) async fn async_server(config: ServerConfig, addr: SocketAddr) -> JoinHandle<()> {
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

            let tls_stream = match acceptor.accept(stream).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("Failed to accept TLS connection: {:?}", e);
                    return;
                }
            };

            let (read_half, write_half) = tokio::io::split(tls_stream);
            let secondary_sender = SecondaryDataSender::Server(Mutex::new(write_half));
            let secondary_receiver = SecondaryDataReceiver::Server(read_half);
            registry_manage_async_requests(
                Arc::new(secondary_sender),
                Arc::new(Mutex::new(secondary_receiver)),
            )
            .await;

            println!("TLS connection established with {:?}", peer_addr);
        }
    })
}


/// this is used when launching in single machine
pub async fn registry_manage_async_requests(
    _sender: Arc<SecondaryDataSender>,
    receiver: Arc<Mutex<SecondaryDataReceiver>>,
) {
    tokio::spawn(async move {
        let receiver = receiver.clone();
        //let sender = sender;
        let binding = receiver.clone();
        let mut listener = binding.lock().await;
        'register_loop: while let Some(data) = listener.receive().await {
            let request = match RegistrationRequest::from_bytes(&data) {
                Ok(request) => request,
                Err(e) => {
                    println!("Failed to parse request: {:?}", e);
                    continue;
                }
            };
            match request {
                RegistrationRequest::Strategy(_owner, _mode, _subscriptions) => {
                    //handle_strategies(owner.clone(), sender, receiver, mode.clone()).await;
                    //let gui_response = RegistryGuiResponse::StrategyAdded(owner, mode, subscriptions);
                    //broadcast(gui_response.to_bytes()).await;
                    break 'register_loop;
                }
                RegistrationRequest::Gui => {
                    //handle_gui(sender, receiver).await;
                    break 'register_loop;
                }
            };
        }
    });
}
