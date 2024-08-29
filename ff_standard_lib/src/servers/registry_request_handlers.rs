use std::sync::Arc;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use tokio::sync::Mutex;
use crate::servers::communications_async::{SecondaryDataReceiver, SecondaryDataSender};
use crate::servers::communications_sync::SynchronousCommunicator;
use crate::standardized_types::data_server_messaging::{FundForgeError};
use crate::standardized_types::strategy_events::EventTimeSlice;
use crate::traits::bytes::Bytes;

pub async fn registry_manage_sequential_requests(communicator: Arc<SynchronousCommunicator>) {
    tokio::spawn(async move {
        while let Some(data) = communicator.receive(true).await {
            let request = match EventRequest::from_bytes(&data) {
                Ok(request) => request,
                Err(e) => {
                    println!("Failed to parse request: {:?}", e);
                    continue;
                }
            };
            println!("Parsed request: {:?}", data);
        }
    });
}

/// this is used when launching in single machine
pub async fn registry_manage_async_requests(sender: Arc<Mutex<SecondaryDataSender>>, receiver: Arc<Mutex<SecondaryDataReceiver>>) {
    tokio::spawn(async move {
        let mut receiver = receiver.lock().await;
        while let Some(data) = receiver.receive().await {
            let request = match EventRequest::from_bytes(&data) {
                Ok(request) => request,
                Err(e) => {
                    println!("Failed to parse request: {:?}", e);
                    continue;
                }
            };
            println!("Parsed request: {:?}", request);
        }
    });
}

// this is used when launching in multi-machine
/*pub async fn registry_manage_async_external(tls_stream: tokio_rustls::server::TlsStream<TcpStream>) {
    let (mut read_half, write_half) = tokio::io::split(tls_stream);
    tokio::spawn(async move {
        while let Some(data) = read_half.read(&mut []).await {
            let request = match EventRequest::from_bytes(&data) {
                Ok(request) => request,
                Err(e) => {
                    println!("Failed to parse request: {:?}", e);
                    continue;
                }
            };
            println!("Parsed request: {:?}", data);
        }
    });
}*/

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
    compare(PartialEq),
    check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum EventRequest {
    StrategyEventUpdates(EventTimeSlice)
}

impl Bytes<Self> for EventRequest {
    fn to_bytes(&self) -> Vec<u8> {
        let vec = rkyv::to_bytes::<_, 100000>(self).unwrap();
        vec.into()
    }

    fn from_bytes(archived: &[u8]) -> Result<EventRequest, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<EventRequest>(archived) { //Ignore this warning: Trait `Deserialize<UiStreamResponse, SharedDeserializeMap>` is not implemented for `ArchivedUiStreamResponse` [E0277] 
            Ok(response) => Ok(response),
            Err(e) => {
                Err(FundForgeError::ClientSideErrorDebug(e.to_string()))
            }
        }
    }
}

