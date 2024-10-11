use dashmap::DashMap;
use tokio::sync::mpsc::Sender;


/// Manages subscriptions to incoming data, any concurrent process that needs a copy of this objects  primary data source can become a `SecondaryDataSubscriber` and thus will receive a copy of the data stream.
pub struct StaticInternalBroadcaster<T: Clone + Send> {
    subscribers: DashMap<String, Sender<T>>,
}

impl<T: Clone + Send> StaticInternalBroadcaster<T> {
    pub fn new() -> Self {
        StaticInternalBroadcaster {
            subscribers: Default::default(),
        }
    }

    /// Clear all subscribers to signal that the broadcaster will be shut down.
    pub fn shut_down(&self) {
        self.subscribers.clear();
    }

    /// adds the subscriber to the subscriptions for this manager
    pub fn subscribe(&self, name: String, subscriber: Sender<T>) {
        self.subscribers.insert(name, subscriber);
    }

    pub fn unsubscribe(&self, name: &str) {
        self.subscribers.remove(name);
    }

    pub fn has_subscribers(&self) -> bool {
        self.subscribers.is_empty()
    }


    /// Sequential broadcast: broadcasts the data to all subscribers sequentially without concurrency or creating new tasks.
    pub async fn broadcast(&self, source_data: T) {
        let mut closed =vec![];
        for sub in self.subscribers.iter() {
            match sub.value().send(source_data.clone()).await {
                Ok(_) => {}
                Err(_) => closed.push(sub.key().clone())
            }
        }
        if !closed.is_empty() {
            for name in closed {
                self.subscribers.remove(&name);
                println!("Failed broadcast recipient: {}", name)
            }
        }
    }
}
