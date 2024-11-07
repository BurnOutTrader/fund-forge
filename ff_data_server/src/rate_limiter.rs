use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::interval;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct RateLimiter {
    pub semaphore: Arc<Semaphore>,
    current_permits: Arc<AtomicUsize>,
    max_permits: usize,
}

impl RateLimiter {
    pub fn new(rate: usize, per_duration: Duration) -> Arc<Self> {
        let semaphore = Arc::new(Semaphore::new(0));  // Start with 0 permits
        let current_permits = Arc::new(AtomicUsize::new(0));
        let rate_limiter = Arc::new(Self {
            semaphore,
            current_permits,
            max_permits: rate,
        });

        // Clone the Arc for use in the replenishing task
        let replenishing_rate_limiter = Arc::clone(&rate_limiter);

        // Start a background task to replenish permits
        tokio::spawn(async move {
            let mut interval_timer = interval(per_duration);
            loop {
                interval_timer.tick().await;

                // Reset permits to the maximum rate
                let current = replenishing_rate_limiter.current_permits.load(Ordering::Relaxed);
                let to_add = replenishing_rate_limiter.max_permits.saturating_sub(current);

                if to_add > 0 {
                    replenishing_rate_limiter.semaphore.add_permits(to_add);
                    replenishing_rate_limiter.current_permits.store(
                        replenishing_rate_limiter.max_permits,
                        Ordering::Relaxed
                    );
                }
            }
        });

        rate_limiter
    }

    pub async fn acquire(&self) {
        self.semaphore
            .acquire()
            .await
            .expect("Semaphore closed")
            .forget();

        self.current_permits.fetch_sub(1, Ordering::Relaxed);
    }
}