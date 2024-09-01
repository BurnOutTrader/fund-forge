use crate::servers::communications_async::{SecondaryDataSubscriber, SendError};
use futures::future::join_all;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

pub enum BroadCastType {
    Sequential,
    Concurrent,
}

/// Manages subscriptions to incoming data, any concurrent process that needs a copy of this objects  primary data source can become a `SecondaryDataSubscriber` and thus will receive a copy of the data stream.
pub struct BytesBroadcaster {
    subscribers: Arc<RwLock<HashMap<String, Arc<Mutex<SecondaryDataSubscriber>>>>>,
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
    pub async fn subscribe(&self, subscriber: SecondaryDataSubscriber) {
        let sender = Arc::new(Mutex::new(subscriber));
        let mut subs = self.subscribers.write().await;
        // Clone the Arc to get a new reference to the same subscriber
        // No need to lock the subscriber here since you're not accessing its interior
        let id = {
            let subscriber_guard = sender.lock().await;
            subscriber_guard.id.clone()
        };
        subs.insert(id, sender);
    }

    pub async fn unsubscribe(&self, id: &str) {
        let mut subs = self.subscribers.write().await;
        subs.remove(id);
    }

    pub async fn broadcast(&self, source_data: &Vec<u8>) -> Result<(), SendError> {
        match self.broadcast_type {
            BroadCastType::Sequential => self.broadcast_sequential(source_data).await,
            BroadCastType::Concurrent => self.broadcast_concurrent(source_data.clone()).await,
        }
    }

    /// Sequential broadcast: broadcasts the data to all subscribers sequentially without concurrency or creating new tasks.
    async fn broadcast_sequential(&self, source_data: &Vec<u8>) -> Result<(), SendError> {
        let subs = self.subscribers.read().await;
        let mut error_messages = Vec::new();

        for subscriber in subs.values() {
            let mut subscriber = subscriber.lock().await;
            match subscriber.send(source_data).await {
                Ok(_) => {}
                Err(e) => error_messages.push(e.msg),
            }
        }

        if error_messages.is_empty() {
            Ok(())
        } else {
            let combined_error_msg = error_messages.join("\n");
            Err(SendError {
                msg: combined_error_msg,
            })
        }
    }

    /// Concurrent broadcast: spawns a task for each subscriber to send the data concurrently.
    async fn broadcast_concurrent(&self, source_data: Vec<u8>) -> Result<(), SendError> {
        let source_data = Arc::new(source_data);
        let subs = self.subscribers.read().await;
        let futures: Vec<_> = subs
            .values()
            .map(|subscriber| {
                let subscriber = Arc::clone(subscriber);
                let data_clone = Arc::clone(&source_data);
                tokio::spawn(async move {
                    let mut subscriber = subscriber.lock().await;
                    subscriber.send(&data_clone).await
                })
            })
            .collect();

        let results = join_all(futures).await;
        let mut errors: Vec<String> = Vec::new();
        for result in results.into_iter() {
            if let Ok(Err(e)) = result {
                errors.push(e.msg);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            let combined_error_msg = errors.join("\n");
            Err(SendError {
                msg: combined_error_msg,
            })
        }
    }
}
