use crate::servers::communications_async::SendError;
use crate::standardized_types::data_server_messaging::FundForgeError;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{Mutex};
use tokio_rustls::TlsStream;

/// With an 8-byte (64-bit) length field, you can represent data sizes up to approximately 17179869184 GB, which is equivalent to 16777216 TB, or 16 exabytes (EB).
const LENGTH: usize = 8;
pub struct SecureExternalCommunicator {
    communicator: Mutex<TlsStream<TcpStream>>,
}

impl SecureExternalCommunicator {
    pub fn new(communicator: Mutex<TlsStream<TcpStream>>) -> SecureExternalCommunicator {
        SecureExternalCommunicator { communicator }
    }

    pub async fn send_and_receive(&self, data: Vec<u8>) -> Result<Vec<u8>, FundForgeError> {
        match self.send(data).await {
            Ok(_) => {}
            Err(e) => {
                return Err(FundForgeError::ClientSideErrorDebug(format!(
                    "Failed to send data: {:?}",
                    e
                )))
            }
        }
        let mut communicator = self.communicator.lock().await;
        let mut length_buffer = [0u8; LENGTH];
        // Read the length header
        match communicator.read_exact(&mut length_buffer).await {
            Ok(_) => {}
            Err(e) => {
                return Err(FundForgeError::ServerErrorDebug(format!(
                    "Failed to read the length header: {}",
                    e
                )));
            }
        }
        // Convert the length buffer to a usize
        let length = u64::from_be_bytes(length_buffer) as usize;

        // Buffer for the actual message
        let mut message = vec![0u8; length];

        // Read the message
        match communicator.read_exact(&mut message).await {
            Ok(_) => {}
            Err(e) => {
                return Err(FundForgeError::ServerErrorDebug(format!(
                    "Failed to read the message: {}",
                    e
                )));
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
            Ok(_) => {}
            Err(e) => return Err(SendError { msg: e.to_string() }),
        }

        // Write the actual data
        match communicator.write_all(&data).await {
            Ok(_) => Ok(()),
            Err(e) => return Err(SendError { msg: e.to_string() }),
        }
    }
}
