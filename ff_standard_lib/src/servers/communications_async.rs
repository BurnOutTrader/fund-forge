use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::sync::mpsc::Receiver;
use tokio_rustls::server::{TlsStream as ServerTlsStream};
use tokio_rustls::TlsStream;

/// With an 8-byte (64-bit) length field, you can represent data sizes up to approximately 17179869184 GB, which is equivalent to 16777216 TB, or 16 exabytes (EB).
const LENGTH: usize = 8;

pub type SubscriberId = String;

/// Used for Subscriber pattern, to allow the creating process to subscribe to source data created or maintained by another process.
pub enum SecondaryDataReceiver {
    InternalReceiver(InternalReceiver),
    ExternalReceiver(ExternalReceiver),
    Server(ReadHalf<ServerTlsStream<TcpStream>>)
}

impl SecondaryDataReceiver {
    /// Receive the data from the
    pub async fn receive(&mut self) -> Option<Vec<u8>> {
        match self {
            SecondaryDataReceiver::InternalReceiver(receiver) => receiver.receive().await,
            SecondaryDataReceiver::ExternalReceiver(receiver) => receiver.receive().await,
            SecondaryDataReceiver::Server(receiver) => {
                let mut length_bytes = vec![0u8; LENGTH];
                if receiver.read_exact(&mut length_bytes).await.is_ok() {
                    let msg_length = u64::from_be_bytes(length_bytes.try_into().ok()?) as usize;
                    let mut message_body = vec![0u8; msg_length];
                    if receiver.read_exact(&mut message_body).await.is_ok() {
                        return Some(message_body);
                    }
                }
                None
            }
        }
    }
}

/// Internal Receiver implements a receive fn to receive data as bytes from an internal source over mspc.
pub struct InternalReceiver {
    receiver: Receiver<Vec<u8>>,
}

impl InternalReceiver {
    pub fn new(receiver: Receiver<Vec<u8>>) -> Self {
        Self {
            receiver
        }
    }
    async fn receive(&mut self) -> Option<Vec<u8>> {
        self.receiver.recv().await
    }
}

/// External Receiver implements a receive fn to receive data as bytes from an external source over Tcp.
pub struct ExternalReceiver {
    reader: ReadHalf<TlsStream<TcpStream>>,
}

impl ExternalReceiver {
    pub fn new(reader: ReadHalf<TlsStream<TcpStream>>) -> Self {
        Self { reader }
    }

    pub async fn receive(&mut self) -> Option<Vec<u8>> {
        let mut length_bytes = vec![0u8; LENGTH];
        if self.reader.read_exact(&mut length_bytes).await.is_ok() {
            let msg_length = u64::from_be_bytes(length_bytes.try_into().ok()?) as usize;
            let mut message_body = vec![0u8; msg_length];
            if self.reader.read_exact(&mut message_body).await.is_ok() {
                return Some(message_body);
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct SendError {
    pub msg: String,
}

pub enum SecondaryDataSender {
    InternalSender(Arc<mpsc::Sender<Vec<u8>>>),
    ExternalSender(Arc<Mutex<WriteHalf<TlsStream<TcpStream>>>>),
    Server(Mutex<WriteHalf<ServerTlsStream<TcpStream>>>)
}

impl SecondaryDataSender {
     pub async fn send(&self, data: &Vec<u8>) -> Result<(), SendError> {
        match self {
            SecondaryDataSender::InternalSender(sender) => {
                sender.send(data.clone()).await.map_err(|e| SendError { msg: e.to_string() })
            },
            SecondaryDataSender::ExternalSender(sender) => {
                // Lock the mutex to get mutable access
                let mut sender = sender.lock().await;

                // Convert the length of the data to a u64, then to bytes
                let len = data.len() as u64;
                let len_bytes = len.to_be_bytes();

                // Write the length header
                sender.write_all(&len_bytes).await.map_err(|e| SendError { msg: e.to_string() })?;

                // Write the actual data
                sender.write_all(&data).await.map_err(|e| SendError { msg: e.to_string() })
            }
            SecondaryDataSender::Server(sender) => {
                let len = data.len() as u64;
                let len_bytes = len.to_be_bytes();
                let mut sender = sender.lock().await;
                // Write the length header
                sender.write_all(&len_bytes).await.map_err(|e| SendError { msg: e.to_string() })?;

                // Write the actual data
                sender.write_all(&data).await.map_err(|e| SendError { msg: e.to_string() })
            }
        }
    }
}

/// Used to subscribe to a data stream controlled by a `DataSourceManager`. Any MessageSubscriber sent to a Manager will in turn become a secondary consumer of the Managers data source.
/// It also has a method to forward a message to the subscriber.
pub struct SecondaryDataSubscriber {
    pub id: SubscriberId,
    pub sender: SecondaryDataSender,
}

impl SecondaryDataSubscriber {
    pub fn new(id: SubscriberId, sender: SecondaryDataSender) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self { id, sender }))
    }

    pub  async fn send(&mut self, source_data: &Vec<u8>) -> Result<(), SendError> {
        self.sender.send(source_data).await
    }
}
