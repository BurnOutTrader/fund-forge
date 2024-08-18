use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::interval;

/// RateLimiter that allows up to `rate` operations per `per_duration`.
/// # Properties
/// * `semaphore` - The semaphore used to limit the rate
pub struct RateLimiter {
    pub semaphore: Arc<Semaphore>,
}

impl RateLimiter {
    /// Creates a new RateLimiter that allows up to `rate` operations per `per_duration`.
    pub fn new(rate: usize, per_duration: Duration) -> Arc<Self> {
        let semaphore = Arc::new(Semaphore::new(rate));
        // Clone the Arc for use in the replenishing task
        let replenishing_semaphore = Arc::clone(&semaphore);

        // Start a background task to replenish permits
        tokio::spawn(async move {
            let mut interval_timer = interval(per_duration);
            loop {
                interval_timer.tick().await;
                // Add permits back to the semaphore
                // Note: This doesn't exceed the original permit count
                for _ in 0..rate {
                    replenishing_semaphore.add_permits(1);
                }
            }
        });

        Arc::new(Self { semaphore })
    }

    /// Tries to acquire a permit from the rate limiter. If no permits are available, it waits.
    pub async fn acquire(&self) {
        self.semaphore.acquire().await.expect("Semaphore closed").forget();
    }
}
