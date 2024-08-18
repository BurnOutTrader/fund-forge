use tokio::sync::mpsc::Sender;

/// Manages subscriptions to incoming data, any concurrent process that needs a copy of this objects  primary data source can become a `SecondaryDataSubscriber` and thus will receive a copy of the data stream.
pub struct StaticInternalBroadcaster<T: Clone + Send> {
    subscribers: Vec<Sender<T>>,
}

impl<T: Clone + Send> StaticInternalBroadcaster<T> {
    pub fn new() -> Self {
        StaticInternalBroadcaster {
            subscribers: Vec::new(),
        }
    }

    /// Clear all subscribers to signal that the broadcaster will be shut down.
    pub fn shut_down(&mut self) {
        self.subscribers = Vec::new();
    }

    /// adds the subscriber to the subscriptions for this manager
    pub async fn subscribe(&mut self, subscriber: Sender<T>) {
        self.subscribers.push(subscriber);
    }

    /// Sequential broadcast: broadcasts the data to all subscribers sequentially without concurrency or creating new tasks.
    pub async fn broadcast(&self, source_data: T) {
        for subscriber in &self.subscribers {
            match subscriber.send(source_data.clone()).await {
                Ok(_) => {}
                Err(_) => {},
            }
        }
    }
}