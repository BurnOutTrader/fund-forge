use ahash::AHashMap;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

/// Manages subscriptions to incoming data, any concurrent process that needs a copy of this objects  primary data source can become a `SecondaryDataSubscriber` and thus will receive a copy of the data stream.
pub struct StaticInternalBroadcaster<T: Clone + Send> {
    subscribers: Mutex<AHashMap<String, Sender<T>>>,
}

impl<T: Clone + Send> StaticInternalBroadcaster<T> {
    pub fn new() -> Self {
        StaticInternalBroadcaster {
            subscribers: Default::default(),
        }
    }

    /// Clear all subscribers to signal that the broadcaster will be shut down.
    pub async fn shut_down(&self) {
        let mut subscribers = self.subscribers.lock().await;
        subscribers.clear();
    }

    /// adds the subscriber to the subscriptions for this manager
    pub async fn subscribe(&self, name: String, subscriber: Sender<T>) {
        let mut subscribers = self.subscribers.lock().await;
        subscribers.insert(name, subscriber);
    }

    pub async fn unsubscribe(&self, name: &str) {
        let mut subscribers = self.subscribers.lock().await;
        subscribers.remove(name);
    }

    pub async fn has_subscribers(&self) -> bool {
        let subscribers = self.subscribers.lock().await;
        !subscribers.is_empty()
    }


    /// Sequential broadcast: broadcasts the data to all subscribers sequentially without concurrency or creating new tasks.
    pub async fn broadcast(&self, source_data: T) {
        let mut subscribers = self.subscribers.lock().await;
        let mut closed =vec![];
        for (name, subscriber) in subscribers.iter() {
            match subscriber.send(source_data.clone()).await {
                Ok(_) => {}
                Err(_) => closed.push(name.clone())
            }
        }
        if !closed.is_empty() {
            for name in closed {
                subscribers.remove(&name);
                println!("Failed broadcast recipient: {}", name)
            }
        }
    }
}
