use std::sync::{Arc};
use std::time::Duration;
use ahash::AHashMap;
use dashmap::DashMap;
use lazy_static::lazy_static;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, oneshot, watch, RwLock};
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep, Instant};
use tokio_rustls::server::TlsStream;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::bytes_trait::Bytes;
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
use ff_standard_lib::standardized_types::time_slices::TimeSlice;
use ff_standard_lib::StreamName;

lazy_static! {
    static ref STREAM_RECEIVERS: DashMap<u16 , Arc<DashMap<DataSubscription ,broadcast::Receiver<BaseDataEnum>>>> = DashMap::new();
    static ref SUBSCRIPTIONS: DashMap<u16 , Arc<RwLock<Vec<DataSubscription>>>> = DashMap::new();
    static ref STREAM_TASKS: DashMap<u16 , (JoinHandle<()>, JoinHandle<()>)> = DashMap::new();
}

pub fn shutdown_stream_tasks() {
    for stream in STREAM_TASKS.iter() {
        deregister_streamer(stream.key());
    }
}

pub async fn initialize_streamer(stream_name: StreamName, buffer: Duration, stream: TlsStream<TcpStream>) {
    let map = Arc::new(DashMap::new());
    let list = Arc::new(RwLock::new(vec![]));
    SUBSCRIPTIONS.insert(stream_name, list.clone());
    STREAM_RECEIVERS.insert(stream_name, map.clone());
    let handles = stream_handler(stream_name, buffer, stream, map, list).await;
    STREAM_TASKS.insert(stream_name, handles);
}

pub fn deregister_streamer(stream_name: &StreamName) {
    STREAM_RECEIVERS.remove(stream_name);
    if let Some((_, handles)) = STREAM_TASKS.remove(stream_name) {
        let (receiver_task, timeslice_task) = handles;
        receiver_task.abort();
        timeslice_task.abort();
        SUBSCRIPTIONS.remove(stream_name);
    }
}

pub async fn subscribe_stream(stream_name: &StreamName, subscription: DataSubscription, receiver: broadcast::Receiver<BaseDataEnum>) {
    if let Some(sender_ref) = STREAM_RECEIVERS.get(stream_name) {
        sender_ref.insert(subscription.clone(), receiver);
        let sub_list = SUBSCRIPTIONS.entry(stream_name.clone()).or_insert(Arc::new(RwLock::new(Vec::new())));
        let mut sub_list = sub_list.write().await;
        sub_list.push(subscription);
    }
}

pub async fn unsubscribe_stream(stream_name: &StreamName, subscription: &DataSubscription) {
    if let Some(sub_list) = SUBSCRIPTIONS.get(&stream_name) {
        let mut list = sub_list.write().await;
        list.retain(|sub| sub != subscription);
    }
}

const LENGTH: usize = 4;

pub async fn stream_handler(
    stream_name: StreamName,
    buffer: Duration,
    mut stream: TlsStream<TcpStream>,
    stream_receivers: Arc<DashMap<DataSubscription, broadcast::Receiver<BaseDataEnum>>>,
    subscriptions: Arc<RwLock<Vec<DataSubscription>>>,
) -> (JoinHandle<()>, JoinHandle<()>) {
    let (data_sender, mut data_receiver) = mpsc::channel::<TimeSlice>(10);
    let (tick_sender, tick_receiver) = watch::channel(());

    let receiver_task = tokio::spawn({
        let data_sender = data_sender.clone();
        let tick_receiver = tick_receiver.clone();
        let stream_receivers = stream_receivers.clone();
        let subscriptions = subscriptions.clone();
        async move {
            let mut interval = interval(buffer);
            let running_streams: Arc<RwLock<AHashMap<DataSubscription, oneshot::Sender<()>>>> = Arc::new(RwLock::new(AHashMap::new()));

            loop {
                interval.tick().await;
                tick_sender.send(()).unwrap();

                if stream_receivers.is_empty() {
                    sleep(Duration::from_millis(500)).await;
                    continue;
                }

                let mut to_remove = Vec::new();
                let mut to_add = Vec::new();
                let subscriptions = subscriptions.read().await;
                // Check for streams to remove
                {
                    let running_streams = running_streams.read().await;
                    for (sub, _) in running_streams.iter() {
                        if !subscriptions.contains(sub) {
                            to_remove.push(sub.clone());
                        }
                    }
                }

                // Check for new streams to add
                {
                    let running_streams = running_streams.read().await;
                    for entry in subscriptions.iter() {
                        let sub = entry.clone();
                        if !running_streams.contains_key(&sub) {
                            to_add.push(sub);
                        }
                    }
                }

                // Remove streams
                {
                    let mut running_streams = running_streams.write().await;
                    for sub in to_remove {
                        if let Some(sender) = running_streams.remove(&sub) {
                            let _ = sender.send(());
                        }
                    }
                }

                // Add new streams
                for sub in to_add {
                    if let Some((sub, rx)) = stream_receivers.remove(&sub) {
                        let data_sender = data_sender.clone();
                        let tick_receiver = tick_receiver.clone();
                        let (shutdown_sender, shutdown_receiver) = oneshot::channel();

                        tokio::spawn(async move {
                            process_receiver(rx, data_sender, tick_receiver, shutdown_receiver, buffer).await;
                        });
                        let mut running_streams = running_streams.write().await;
                        running_streams.insert(sub, shutdown_sender);
                    }
                }
            }
        }
    });

    let time_slice_task = tokio::spawn(async move {
        let mut time_slice = TimeSlice::new();
        let mut interval = interval(buffer);

        loop {
            interval.tick().await;

            // Collect any remaining data from receivers
            while let Ok(slice) = data_receiver.try_recv() {
                time_slice.extend(slice);
            }

            if !time_slice.is_empty() {
                let bytes = time_slice.to_bytes();
                let length = (bytes.len() as u32).to_be_bytes();
                let mut prefixed_msg = Vec::with_capacity(LENGTH + bytes.len());
                prefixed_msg.extend_from_slice(&length);
                prefixed_msg.extend_from_slice(&bytes);

                if let Err(e) = stream.write_all(&prefixed_msg).await {
                    eprintln!("Error writing to stream: {}", e);
                    break;
                }
                time_slice.clear();
            }
        }
        println!("Stream handler for {} has shut down", stream_name);
    });

    (receiver_task, time_slice_task)
}
async fn process_receiver(
    mut rx: broadcast::Receiver<BaseDataEnum>,
    data_sender: mpsc::Sender<TimeSlice>,
    mut tick_receiver: watch::Receiver<()>,
    mut shutdown: oneshot::Receiver<()>,
    buffer: Duration,
) {
    tokio::spawn(async move {
        let mut time_slice = TimeSlice::new();
        let mut last_send = Instant::now();

        loop {
            tokio::select! {
                _ = tick_receiver.changed() => {
                    if !time_slice.is_empty() {
                        if data_sender.send(time_slice).await.is_err() {
                            return; // Main task has been dropped
                        }
                        time_slice = TimeSlice::new();
                    }
                    last_send = Instant::now();
                }
                Ok(base_data_enum) = rx.recv() => {
                    time_slice.add(base_data_enum);
                    if last_send.elapsed() >= buffer {
                        if data_sender.send(time_slice).await.is_err() {
                            return; // Main task has been dropped
                        }
                        time_slice = TimeSlice::new();
                        last_send = Instant::now();
                    }
                }
                _ = &mut shutdown => {
                    println!("Shutdown signal received. Stopping process_receiver.");
                    // Perform any cleanup if necessary
                    if !time_slice.is_empty() {
                        // Send any remaining data before shutting down
                        let _ = data_sender.send(time_slice).await;
                    }
                    return;
                }
            }
        }
    });
}