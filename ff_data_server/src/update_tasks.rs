use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;

/// defines a task specific to the updating historical data, used to avoid duplicating tasks and causing serialization conflicts
#[derive(Eq, PartialEq, Clone, Hash, Debug)]
pub struct UpdateTask {
    pub symbol: Symbol,
    pub resolution: Resolution,
}

lazy_static! {
    static ref DATA_UPDATE_TASKS: Arc<RwLock<HashMap<UpdateTask, Arc<Mutex<JoinHandle<()>>>>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

/// Creates a new task for updating historical data
///
/// # Arguments
///
/// * `vendor_download_function` - A function that takes a `DataSubscription` and a `OwnedSemaphorePermit` as input and returns a `Future` that downloads data from the specified vendor
/// * `permit` - A `OwnedSemaphorePermit` this is used to limit the number of concurrent downloads from a specific vendor, this is useful to limit load on the vendors api, and keep room for live trading.
/// * `subscription` - The subscription for which the data needs to be updated
///
/// # Returns
///
/// A `JoinHandle` that can be used to await the completion of the task
pub async fn data_update_task<F, Fut>(
    data_folder: PathBuf,
    vendor_download_function: F,
    subscription: DataSubscription,
) -> Arc<Mutex<JoinHandle<()>>>
where
    F: Fn(DataSubscription, PathBuf) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let update_task = UpdateTask {
        symbol: subscription.symbol.clone(),
        resolution: subscription.resolution.clone(),
    };

    let updating_subscriptions = DATA_UPDATE_TASKS.clone();
    let mut subscriptions = updating_subscriptions.write().await;

    if let Some(existing_task) = subscriptions.get(&update_task) {
        //if the task already exists, return it
        existing_task.clone()
    } else {
        let sub_task = update_task.clone();
        let task = Arc::new(Mutex::new(tokio::task::spawn(async move {
            // run the vendor download function if a task for this subscription does not exist
            let subscriptions_tasks = DATA_UPDATE_TASKS.clone();
            vendor_download_function(subscription, data_folder).await;
            //remove the task from the map on completion
            let mut subscriptions = subscriptions_tasks.write().await;
            subscriptions.remove(&sub_task);
        })));
        subscriptions.insert(update_task.clone(), task.clone());
        drop(subscriptions); // Drop the lock before awaiting the task
        task
    }
}
