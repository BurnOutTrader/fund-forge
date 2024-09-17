use futures::future::join_all;
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

        // Create a vector of futures for each subscriber
        let mut tasks = Vec::with_capacity(subscribers.len());

        for (i, subscriber) in subscribers.iter().enumerate() {
            let data_clone = source_data.clone();
            let task = async move {
                // Try sending data to the subscriber
                let result = subscriber.send(data_clone).await;
                // Return the index of the failed subscriber (if any)
                result.map_err(|_| i)
            };
            tasks.push(task);
        }

        // Await all tasks concurrently
        let results = join_all(tasks).await;

        // Collect failed subscriber indices
        let mut failed_indices = Vec::new();
        for result in results {
            if let Err(index) = result {
                failed_indices.push(index);
            }
        }

        // Remove failed subscribers
        if !failed_indices.is_empty() {
            let mut subscribers = self.subscribers.write().await; // Acquire a write lock to modify the list
            for &i in failed_indices.iter().rev() {
                subscribers.remove(i);
            }
        }
    }
}
