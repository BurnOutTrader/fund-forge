use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::interval;
use std::sync::atomic::{AtomicUsize, Ordering};
pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
    current_permits: Arc<AtomicUsize>,
    max_permits: usize,
}

impl RateLimiter {
    pub fn new(rate: usize, per_duration: Duration) -> Arc<Self> {
        // Start with zero permits
        let semaphore = Arc::new(Semaphore::new(0));
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

                // First, calculate how many permits we need to add
                let current = replenishing_rate_limiter.current_permits.load(Ordering::Acquire);
                let to_add = replenishing_rate_limiter.max_permits.saturating_sub(current);

                if to_add > 0 {
                    replenishing_rate_limiter.semaphore.add_permits(to_add);
                    replenishing_rate_limiter.current_permits.store(
                        replenishing_rate_limiter.max_permits,
                        Ordering::Release
                    );
                }
            }
        });

        // Initial permit allocation
        rate_limiter.semaphore.add_permits(rate);
        rate_limiter.current_permits.store(rate, Ordering::Release);

        rate_limiter
    }

    pub async fn acquire(&self) {
        self.semaphore
            .acquire()
            .await
            .expect("Semaphore closed")
            .forget();

        self.current_permits.fetch_sub(1, Ordering::Release);
    }

    #[cfg(test)]
    pub fn available_permits(&self) -> usize {
        self.current_permits.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Instant};
    use futures::future::join_all;

    #[tokio::test]
    async fn test_rate_limiter() {
        // Configure rate limiter: 200 permits per 100ms
        let rate = 200;
        let interval = Duration::from_millis(100);
        let rate_limiter = RateLimiter::new(rate, interval);

        // Let the rate limiter initialize
        sleep(interval).await;

        // Test 1: Burst test
        let start = Instant::now();
        let mut handles = vec![];

        // Launch exactly rate number of tasks
        for i in 0..rate {
            let rate_limiter = rate_limiter.clone();
            handles.push(tokio::spawn(async move {
                rate_limiter.acquire().await;
                (i, start.elapsed())
            }));
        }

        let results = join_all(handles).await;
        let elapsed_times: Vec<_> = results.into_iter()
            .filter_map(Result::ok)
            .collect();

        assert_eq!(elapsed_times.len(), rate, "Not all tasks completed");

        // Test 2: Verify rate limiting
        let start = Instant::now();
        let mut handles = vec![];

        // Try to run 2x our rate limit
        for i in 0..rate*2 {
            let rate_limiter = rate_limiter.clone();
            handles.push(tokio::spawn(async move {
                rate_limiter.acquire().await;
                (i, start.elapsed())
            }));
        }

        let results = join_all(handles).await;
        let mut elapsed_times: Vec<_> = results.into_iter()
            .filter_map(Result::ok)
            .map(|(_, elapsed)| elapsed)
            .collect();
        elapsed_times.sort();

        // Count operations in first interval
        let first_interval_count = elapsed_times.iter()
            .filter(|&&t| t < interval)
            .count();

        assert!(first_interval_count <= rate,
                "Too many operations in first interval: {}", first_interval_count);

        // Verify second batch starts after first interval
        assert!(elapsed_times[rate] >= interval,
                "Second batch started too early: {:?}", elapsed_times[rate]);
    }
}