use crate::strategies::client_features::connection_settings::client_settings::ConnectionSettings;
use crate::messages::data_server_messaging::FundForgeError;
use rustls::pki_types::ServerName;
use rustls::ClientConfig;
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::{TlsConnector, TlsStream};

/// Initializes a TLS connection to a specified server address using a given CA certificate file and server name.
///
/// This function creates a TLS connection by first loading the CA certificate from the specified file,
/// then using it to create a client configuration. It then attempts to connect to the server at the given address
/// and initiates a TLS handshake using the specified server name for Server Name Indication (SNI).
///
/// # How It Works
/// The Historical data server is always a single Synchronous client, meaning that the stream remains unsplit and all communication consists of a synchronous `request -> response` sequence.
/// The Data Server establishes a connection to Individual broker or vendor api's for making data update requests, allowing us to run our individual api implementations as microservices for co-location purposes.
/// If a user configures only a single address for all vendors and brokers, the client or server will use that address for all communications.
/// If a user configures multiple addresses for all vendors and brokers, the client or server will use the specified address for all communications with that vendor and broker.
///
/// The `get_synchronous_client()` or`get_async_write_client()` and `get_async_read_client()` functions return a client based on the address specified by the user in the client_settings.toml file by correlating the vendor and broker name to the address specified in the client_settings.toml file.
/// If two vendors or brokers share an address a duplicate connection will not be made, instead the function will return the existing connection.
/// `get_async_client` returns a stream half that can either be written to or read from.
/// The only difference between the two types of clients is that the `get_synchronous_client()` function returns a client that is not split, and must be used in a synchronous `request -> response` sequence.
///
/// # Parameters
/// - `addr`: A reference to the `SocketAddr` of the server to connect to.
/// - `ca_file`: A reference to the `Path` of the CA certificate file used for verifying the server's certificate.
/// - `server_name`: A reference to a `String` containing the expected server name, used for SNI and certificate verification.
///
/// # Returns
/// - `Result<(TlsStream<TcpStream>), FundForgeError>`: On success, returns a `TlsStream` wrapped around a `TcpStream`,
///   representing the established TLS connection. On failure, returns a `FundForgeError` with details about the error.
///
/// # Errors
/// This function can return an error in several cases, including but not limited to:
/// - Failure to open or read the CA certificate file.
/// - Failure to parse the CA certificate.
/// - Failure to connect to the server address.
/// - Failure to complete the TLS handshake, including errors related to server name parsing or certificate verification.
async fn initialise_connection(
    addr: &SocketAddr,
    ca_file: &Path,
    server_name: &String,
) -> Result<TlsStream<TcpStream>, FundForgeError> {
    let mut root_cert_store = rustls::RootCertStore::empty();
    let file = match File::open(ca_file) {
        Err(e) => {
            return Err(FundForgeError::ClientSideErrorDebug(format!(
                "Failed to open CA certificate file: {}",
                e
            )))
        }
        Ok(file) => file,
    };
    let mut pem = BufReader::new(file);
    for cert in rustls_pemfile::certs(&mut pem) {
        match cert {
            Ok(cert) => {
                root_cert_store.add(cert).map_err(|_| {
                    FundForgeError::ClientSideErrorDebug("Failed to add CA certificate".to_string())
                })?;
            }
            Err(e) => {
                return Err(FundForgeError::ClientSideErrorDebug(format!(
                    "Failed to parse CA certificate: {}",
                    e
                )))
            }
        }
    }

    let config = ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));
    let stream = match TcpStream::connect(addr).await {
        Err(e) => {
            return Err(FundForgeError::ClientSideErrorDebug(format!(
                "Failed to connect to server: {}",
                e
            )))
        }
        Ok(stream) => stream,
    };

    let server_name = match ServerName::try_from(server_name.to_string()) {
        Ok(server_name) => server_name,
        Err(e) => {
            return Err(FundForgeError::ClientSideErrorDebug(format!(
                "Failed to parse server name: {}",
                e
            )));
        }
    };

    match connector.connect(server_name, stream).await {
        Ok(stream) => Ok(TlsStream::from(stream)),
        Err(e) => {
            return Err(FundForgeError::ClientSideErrorDebug(format!(
                "Failed to connect to server: {}",
                e
            )));
        }
    }
}

pub(crate) async fn create_async_api_client(
    settings: &ConnectionSettings,
    is_stream: bool
) -> Result<TlsStream<TcpStream>, FundForgeError> {
    let ca_path = Path::join(&settings.ssl_auth_folder, "rootCA.crt");
    let address = match is_stream {
        true => settings.stream_address.clone(),
        false => settings.address.clone()
    };
    let stream =
        match initialise_connection(&address, &ca_path, &settings.server_name).await {
            Ok(stream) => stream,
            Err(e) => {
                return Err(FundForgeError::ClientSideErrorDebug(format!(
                    "Failed to connect to server: {}",
                    e
                )));
            }
        };

    Ok(stream)
}
