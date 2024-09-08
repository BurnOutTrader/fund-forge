use crate::servers::communications_async::{SecondaryDataSender};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock};

pub enum BroadCastType {
    Sequential,
    Concurrent,
}

/// Manages subscriptions to incoming data, any concurrent process that needs a copy of this objects  primary data source can become a `SecondaryDataSubscriber` and thus will receive a copy of the data stream.
pub struct BytesBroadcaster {
    subscribers: Arc<RwLock<HashMap<usize, Arc<SecondaryDataSender>>>>,
    broadcast_type: BroadCastType,
}

impl BytesBroadcaster {
    pub fn new(broadcast_type: BroadCastType) -> Self {
        BytesBroadcaster {
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            broadcast_type,
        }
    }

    /// Clear all subscribers to signal that the broadcaster will be shut down.
    pub fn shut_down(&mut self) {
        self.subscribers = Arc::new(RwLock::new(HashMap::new()));
    }

    /// adds the subscriber to the subscriptions for this manager
    pub async fn subscribe(&self, subscriber: Arc<SecondaryDataSender>) -> usize {
        let mut subs = self.subscribers.write().await;
        // Clone the Arc to get a new reference to the same subscriber
        // No need to lock the subscriber here since you're not accessing its interior
        let id = subs.len() + 1;
        subs.insert(id, subscriber);
        id
    }

    pub async fn unsubscribe(&self, id: usize) {
        let mut subs = self.subscribers.write().await;
        subs.remove(&id);
    }

    pub async fn send_subscriber(&self, id: usize, data: Vec<u8>) {
        let subscribers = self.subscribers.clone();
        tokio::spawn(async move {
            if let Some(subscriber) = subscribers.read().await.get(&id) {
                match subscriber.send(&data).await {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
        });
    }

    pub async fn broadcast(&self, source_data: &Vec<u8>) {
        match self.broadcast_type {
            BroadCastType::Sequential => self.broadcast_sequential(source_data).await,
            BroadCastType::Concurrent => self.broadcast_concurrent(source_data.clone()).await,
        }
    }

    /// Sequential broadcast: broadcasts the data to all subscribers sequentially without concurrency or creating new tasks.
    async fn broadcast_sequential(&self, source_data: &Vec<u8>)  {
        let subs = self.subscribers.read().await;

        for subscriber in subs.values() {
            match subscriber.send(source_data).await {
                Ok(_) => {}
                Err(e) => {},
            }
        }

    }

    /// Concurrent broadcast: spawns a task for each subscriber to send the data concurrently.
    async fn broadcast_concurrent(&self, source_data: Vec<u8>) {
        let source_data = Arc::new(source_data);
        let subs = self.subscribers.read().await;
        for (_, subscriber) in &*subs {
            let mut subscriber = Arc::clone(subscriber);
            let data_clone = Arc::clone(&source_data);
            tokio::spawn(async move {
                match subscriber.send(&data_clone).await {
                    Ok(_) => {}
                    Err(_) => {}
                }
            });
        }
    }
}
