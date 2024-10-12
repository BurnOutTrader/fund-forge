use std::sync::Arc;
use std::time::Duration;
use ahash::AHashMap;
use dashmap::DashMap;
use futures::future::join_all;
use lazy_static::lazy_static;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, Mutex};
use tokio::time::interval;
use tokio_rustls::server::TlsStream;
use crate::server_features::StreamName;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::bytes_trait::Bytes;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::standardized_types::time_slices::TimeSlice;

lazy_static! {
    static ref STREAM_RECEIVERS: DashMap<u16 , Arc<Mutex<AHashMap<DataSubscription ,broadcast::Receiver<BaseDataEnum>>>>> = DashMap::new();
}

pub async fn initialize_streamer(stream_name: StreamName, buffer: Duration, stream: TlsStream<TcpStream>) {
    let map = Arc::new(Mutex::new(AHashMap::new()));
    stream_handler(stream_name, buffer, stream, map.clone()).await;
    STREAM_RECEIVERS.insert(stream_name, map);
}

pub fn deregister_streamer(stream_name: &StreamName) {
    STREAM_RECEIVERS.remove(stream_name);
}

pub async fn subscribe_stream(stream_name: &StreamName, subscription: DataSubscription, receiver: broadcast::Receiver<BaseDataEnum>) {
    if let Some(sender_ref) = STREAM_RECEIVERS.get(stream_name) {
        let mut write_ref = sender_ref.lock().await;
        write_ref.insert(subscription, receiver);
    }
}

pub async fn unsubscribe_stream(stream_name: &StreamName, subscription: &DataSubscription) {
    if let Some(sender_ref) = STREAM_RECEIVERS.get(stream_name) {
        let mut write_ref = sender_ref.lock().await;
        write_ref.remove(subscription);
    }
}

const LENGTH: usize = 4;
pub async fn stream_handler(
    stream_name: StreamName,
    buffer: Duration,
    mut stream: TlsStream<TcpStream>,
    stream_receivers: Arc<Mutex<AHashMap<DataSubscription, broadcast::Receiver<BaseDataEnum>>>>
) {
    tokio::spawn(async move {
        let mut time_slice = TimeSlice::new();
        let mut interval = interval(buffer);
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let bytes = time_slice.to_bytes();
                    let length: [u8; LENGTH] = (bytes.len() as u32).to_be_bytes();
                    let mut prefixed_msg = Vec::new();
                    prefixed_msg.extend_from_slice(&length);
                    prefixed_msg.extend_from_slice(&bytes);

                    if let Err(e) = stream.write_all(&prefixed_msg).await {
                        eprintln!("Error writing to stream: {}", e);
                        break;
                    }
                    time_slice = TimeSlice::new();
                }
                data = async {
                    let mut receivers = stream_receivers.lock().await;
                    let futures = receivers.iter_mut().map(|(_, rx)| async move {
                        rx.recv().await.ok()
                    });
                    join_all(futures).await
                } => {
                    for base_data_enum in data.into_iter().flatten() {
                        time_slice.add(base_data_enum);
                    }
                }
            }
        }
        deregister_streamer(&stream_name);
    });
}