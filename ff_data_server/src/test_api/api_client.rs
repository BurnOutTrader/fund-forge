use std::sync::Arc;
use dashmap::DashMap;
use structopt::lazy_static::lazy_static;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
lazy_static! {
    pub static ref TEST_CLIENT: Arc<TestApiClient> = Arc::new(TestApiClient::new());
}

pub struct TestApiClient {
    pub(crate) data_feed_broadcasters: Arc<DashMap<DataSubscription, broadcast::Sender<BaseDataEnum>>>,
    pub(crate) data_feed_tasks: Arc<DashMap<DataSubscription, JoinHandle<()>>>
}

impl TestApiClient {
    fn new() -> Self {
        Self {
            data_feed_broadcasters: Default::default(),
            data_feed_tasks: Default::default(),
        }
    }

    pub fn shutdown(&self) {
        for task in self.data_feed_tasks.iter() {
            task.value().abort();
        }
        self.data_feed_broadcasters.clear();
    }
}

