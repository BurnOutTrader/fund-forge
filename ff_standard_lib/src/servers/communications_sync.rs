use tokio::sync::{mpsc, Mutex};
use tokio_rustls::TlsStream;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::servers::communications_async::SendError;
use crate::standardized_types::data_server_messaging::FundForgeError;

/// With an 8-byte (64-bit) length field, you can represent data sizes up to approximately 17179869184 GB, which is equivalent to 16777216 TB, or 16 exabytes (EB).
const LENGTH: usize = 8;

/// A type that allows us to treat any kind of connection as the same, allowing us to switch from single machine to multi-machine modes and use docker, kubernetes or microservices with the same code.
pub enum SynchronousCommunicator {
    Channels(InternalCommunicator),
    TlsConnections(SecureExternalCommunicator),
}

impl SynchronousCommunicator {
    pub fn new(communicator: SynchronousCommunicator) -> Self {
        match communicator {
             SynchronousCommunicator::Channels(communicator) => return SynchronousCommunicator::Channels(communicator),
             SynchronousCommunicator::TlsConnections(communicator) => SynchronousCommunicator::TlsConnections(communicator),
        }
    }


    pub async fn send_and_receive(&self, data: Vec<u8>, server: bool) -> Result<Vec<u8>, FundForgeError> {
        match self {
            SynchronousCommunicator::Channels(communicator) => communicator.send_and_receive(data, server).await,
            SynchronousCommunicator::TlsConnections(communicator) => return communicator.send_and_receive(data).await,
        }
    }

    pub async fn send_no_response(&self, data: Vec<u8>, server: bool) -> Result<(), FundForgeError> {
         let result = match self {
              SynchronousCommunicator::Channels(communicator) => communicator.send(data, server).await,
              SynchronousCommunicator::TlsConnections(communicator) => communicator.send(data).await,
        };
        match result {
            Ok(_) => Ok(()),
            Err(e) => return Err(FundForgeError::ClientSideErrorDebug(format!("Failed to send data: {:?}", e))),
        }
    }

    pub async fn receive(&self, server: bool) -> Option<Vec<u8>> {
        match self {
            SynchronousCommunicator::Channels(communicator) => communicator.receive(server).await,
             SynchronousCommunicator::TlsConnections(communicator) =>  communicator.receive().await,
        }
    }
}

//todo.. this is essentially sending messages to itself, you could just copy the bytes into it and grab them, no need to transfer..idk i think it is pointless.
pub struct InternalCommunicator {
    client_sender: mpsc::Sender<Vec<u8>>,
    client_receiver: Mutex<mpsc::Receiver<Vec<u8>>>,
    server_sender: mpsc::Sender<Vec<u8>>,
    server_receiver: Mutex<mpsc::Receiver<Vec<u8>>>,
}

impl InternalCommunicator {
    pub fn new(client_buffer: usize, server_buffer: usize) -> Self {
        let (client_sender, server_receiver) = mpsc::channel(client_buffer);
        let server_receiver = Mutex::new(server_receiver);
        let (server_sender, client_receiver) = mpsc::channel(server_buffer);
        let client_receiver = Mutex::new(client_receiver);
        InternalCommunicator {
            client_sender,
            server_receiver,
            server_sender,
            client_receiver
        }
    }

    pub async fn send(&self, data: Vec<u8>, server: bool) -> Result<(), SendError> {
        match server {
            true => self.server_send(data).await,
            false => self.client_send(data).await,
        }
    }

    pub async fn send_and_receive(&self, data: Vec<u8>, server: bool) -> Result<Vec<u8>, FundForgeError> {
        match server {
            true => self.server_send_and_receive(data).await,
            false => self.client_send_and_receive(data).await,
        }
    }

    pub async fn receive(&self, server: bool) -> Option<Vec<u8>> {
        match server {
            true => self.receive_server().await,
            false => self.receive_client().await,
        }
    }

    async fn server_send_and_receive(&self, data: Vec<u8>) -> Result<Vec<u8>, FundForgeError> {
        match self.server_send(data).await {
            Ok(_) => {},
            Err(e) => return Err(FundForgeError::ClientSideErrorDebug(format!("Failed to send data: {:?}", e))),
        }
        match self.receive_server().await {
            Some(data) => return Ok(data),
            None => return Err(FundForgeError::ClientSideErrorDebug(format!("Failed to receive data"))),
        }
    }

    async fn client_send_and_receive(&self, data: Vec<u8>) -> Result<Vec<u8>, FundForgeError> {
        match self.client_send(data).await {
            Ok(_) => {},
            Err(e) => return Err(FundForgeError::ClientSideErrorDebug(format!("Failed to send data: {:?}", e))),
        }
        match self.receive_client().await {
            Some(data) => return Ok(data),
            None => return Err(FundForgeError::ClientSideErrorDebug(format!("No data received"))),
        }
    }

    async fn receive_client(&self) -> Option<Vec<u8>> {
        let mut receiver = self.client_receiver.lock().await;
        receiver.recv().await
    }

    async fn receive_server(&self) -> Option<Vec<u8>> {
        let mut receiver = self.server_receiver.lock().await;
        receiver.recv().await
    }

    async fn server_send(&self, data: Vec<u8>) -> Result<(), SendError> {
        match self.server_sender.send(data).await {
            Ok(_) => return Ok(()),
            Err(e) => return Err(SendError{msg: format!("Could not send data to client: {}", e)}),
        }
    }

    async fn client_send(&self, data: Vec<u8>) -> Result<(), SendError> {
        match  self.client_sender.send(data).await {
            Ok(_) => return Ok(()),
            Err(e) => return Err(SendError{msg: format!("Could not send data to client: {}", e)}),
        }
    }
}

pub struct SecureExternalCommunicator {
    communicator: Mutex<TlsStream<TcpStream>>,
}

impl SecureExternalCommunicator {

    pub fn new(communicator: Mutex<TlsStream<TcpStream>>) -> SecureExternalCommunicator {
        SecureExternalCommunicator {
            communicator,
        }
    }

    pub async fn send_and_receive(&self, data: Vec<u8>) -> Result<Vec<u8>, FundForgeError> {
        match self.send(data).await {
            Ok(_) => {},
            Err(e) => return Err(FundForgeError::ClientSideErrorDebug(format!("Failed to send data: {:?}", e))),
        }
        let mut communicator = self.communicator.lock().await;
        let mut length_buffer = [0u8; LENGTH];
        // Read the length header
        match communicator.read_exact(&mut length_buffer).await {
            Ok(_) =>{} ,
            Err(e) => {
                return Err(FundForgeError::ServerErrorDebug(format!("Failed to read the length header: {}", e)));
            }
        }
        // Convert the length buffer to a usize
        let length = u64::from_be_bytes(length_buffer) as usize;

        // Buffer for the actual message
        let mut message = vec![0u8; length];

        // Read the message
        match communicator.read_exact(&mut message).await {
            Ok(_) => {},
            Err(e) => {
                return Err(FundForgeError::ServerErrorDebug(format!("Failed to read the message: {}", e)));
            }
        }
        Ok(message)
    }

    async fn receive(&self) -> Option<Vec<u8>> {
        let mut length_bytes = vec![0u8; LENGTH];
        let mut communicator = self.communicator.lock().await;
        if communicator.read_exact(&mut length_bytes).await.is_ok() {
            let msg_length = u64::from_be_bytes(length_bytes.try_into().ok()?) as usize;
            let mut message_body = vec![0u8; msg_length];
            if communicator.read_exact(&mut message_body).await.is_ok() {
                return Some(message_body);
            }
        }
        None
    }

    async fn send(&self, data: Vec<u8>) -> Result<(), SendError> {
        let mut communicator = self.communicator.lock().await;

        let length = (data.len() as u64).to_be_bytes();

        // Write the length header
        match communicator.write_all(&length).await {
            Ok(_) => {},
            Err(e) => return Err(SendError { msg: e.to_string() }),
        }

        // Write the actual data
        match communicator.write_all(&data).await {
            Ok(_) => Ok(()),
            Err(e) => return Err(SendError { msg: e.to_string() }),
        }
    }
}
