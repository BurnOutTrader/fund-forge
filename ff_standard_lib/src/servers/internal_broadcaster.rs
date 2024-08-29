use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;

/// Manages subscriptions to incoming data, any concurrent process that needs a copy of this objects  primary data source can become a `SecondaryDataSubscriber` and thus will receive a copy of the data stream.
pub struct StaticInternalBroadcaster<T: Clone + Send> {
    subscribers: RwLock<Vec<Sender<T>>>,
}

impl<T: Clone + Send> StaticInternalBroadcaster<T> {
    pub fn new() -> Self {
        StaticInternalBroadcaster {
            subscribers: Default::default(),
        }
    }

    /// Clear all subscribers to signal that the broadcaster will be shut down.
    pub async fn shut_down(&self) {
        *self.subscribers.write().await = Default::default();
    }

    /// adds the subscriber to the subscriptions for this manager
    pub async fn subscribe(&self, subscriber: Sender<T>) {
        self.subscribers.write().await.push(subscriber);
    }

    /// Sequential broadcast: broadcasts the data to all subscribers sequentially without concurrency or creating new tasks.
    pub async fn broadcast(&self, source_data: T) {
        let subscribers = self.subscribers.read().await;
        for subscriber in &*subscribers {
            match subscriber.send(source_data.clone()).await {
                Ok(_) => {}
                Err(_) => {},
            }
        }
    }
}