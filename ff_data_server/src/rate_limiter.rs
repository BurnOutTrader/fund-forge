use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::interval;
use std::sync::atomic::{AtomicUsize, Ordering};
pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
    available_permits: Arc<AtomicUsize>,
    max_permits: usize,
}

impl RateLimiter {
    pub fn new(rate: usize, per_duration: Duration) -> Arc<Self> {
        // Start with max permits
        let semaphore = Arc::new(Semaphore::new(rate));
        let available_permits = Arc::new(AtomicUsize::new(rate));
        let rate_limiter = Arc::new(Self {
            semaphore,
            available_permits,
            max_permits: rate,
        });

        // Clone the Arc for use in the replenishing task
        let replenishing_rate_limiter = Arc::clone(&rate_limiter);

        // Start a background task to replenish permits
        tokio::spawn(async move {
            let mut interval_timer = interval(per_duration);
            interval_timer.tick().await; // Skip first tick

            loop {
                interval_timer.tick().await;

                // First drain any remaining permits
                let _ = replenishing_rate_limiter.available_permits.swap(0, Ordering::SeqCst);
                replenishing_rate_limiter.semaphore.add_permits(rate);
                replenishing_rate_limiter.available_permits.store(rate, Ordering::SeqCst);
            }
        });

        rate_limiter
    }

    pub async fn acquire(&self) {
        // First check if we have permits available
        if self.available_permits.fetch_sub(1, Ordering::SeqCst) <= 0 {
            // If we decremented to or below 0, add 1 back since we couldn't actually take a permit
            self.available_permits.fetch_add(1, Ordering::SeqCst);

            // Now wait for the next interval
            self.semaphore
                .acquire()
                .await
                .expect("Semaphore closed")
                .forget();

            // Successfully acquired, now decrement the count
            self.available_permits.fetch_sub(1, Ordering::SeqCst);
        } else {
            // We got a permit from the counter, also take one from semaphore
            self.semaphore
                .acquire()
                .await
                .expect("Semaphore closed")
                .forget();
        }
    }

    #[cfg(test)]
    pub fn available(&self) -> usize {
        self.available_permits.load(Ordering::SeqCst)
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

        // Test 1: Speed test - verify we can achieve close to max rate
        let test_duration = Duration::from_secs(1);
        let start = Instant::now();
        let mut count = 0;

        while start.elapsed() < test_duration {
            rate_limiter.acquire().await;
            count += 1;
        }

        // Calculate actual rate (ops per 100ms)
        let elapsed = start.elapsed();
        let actual_rate = (count as f64 * interval.as_secs_f64()) / elapsed.as_secs_f64();

        // We should achieve at least 90% of target rate
        let min_acceptable = rate as f64 * 0.90;
        println!("Achieved rate: {}/100ms (minimum: {}/100ms)", actual_rate, min_acceptable);
        assert!(actual_rate >= min_acceptable,
                "Rate too low: {}/100ms, expected at least {}/100ms", actual_rate, min_acceptable);

        // Test 2: Verify we can't exceed rate limit
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

        // Also verify first batch is close to max rate
        assert!(first_interval_count >= (rate as f64 * 0.90) as usize,
                "First interval too slow: got {} ops, expected at least {}",
                first_interval_count, (rate as f64 * 0.90) as usize);
    }
}