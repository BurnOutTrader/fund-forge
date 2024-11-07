use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{interval, Duration};

pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
    max_tokens: usize,
}

impl RateLimiter {
    pub fn new(max_tokens: usize, refill_interval: Duration) -> Arc<Self> {
        let rate_limiter = Arc::new(Self {
            semaphore: Arc::new(Semaphore::new(max_tokens)),
            max_tokens,
        });

        let rate_limiter_clone = Arc::clone(&rate_limiter);
        tokio::spawn(async move {
            let mut interval = interval(refill_interval);
            loop {
                interval.tick().await;
                let available = rate_limiter_clone.semaphore.available_permits();
                let to_add = rate_limiter_clone.max_tokens.saturating_sub(available);
                rate_limiter_clone.semaphore.add_permits(to_add);
            }
        });

        rate_limiter
    }

    /// Blocks until a permit is available, ensuring we stay within rate limits.
    pub async fn acquire(&self) {
        self.semaphore.acquire().await.expect("Semaphore closed").forget();
    }

    #[cfg(test)]
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use super::*;
    use tokio::time::{sleep, Duration, Instant};
    use futures::future::join_all;


    #[tokio::test]
    async fn test_initial_burst() {
        let rate_limiter = RateLimiter::new(5, Duration::from_millis(100));

        // Should allow an initial burst up to max tokens
        for _ in 0..5 {
            rate_limiter.acquire().await;
        }
    }

    #[tokio::test]
    async fn test_refill_tokens() {
        let rate_limiter = RateLimiter::new(2, Duration::from_millis(100));

        // Deplete the tokens
        rate_limiter.acquire().await;
        rate_limiter.acquire().await;

        // Wait for a token to refill
        sleep(Duration::from_millis(110)).await;
        rate_limiter.acquire().await; // This should succeed after refill

        // Wait for another refill
        sleep(Duration::from_millis(110)).await;
        rate_limiter.acquire().await; // This should succeed again after refill
    }

    #[tokio::test]
    async fn test_sustained_throughput() {
        let rate = 200;
        let interval = Duration::from_millis(100);
        let rate_limiter = RateLimiter::new(rate, interval);

        // Initial burst
        for _ in 0..rate {
            rate_limiter.acquire().await;
        }

        // Measure throughput over multiple intervals
        let start = Instant::now();
        let mut handles = Vec::new();
        for _ in 0..rate * 5 {
            let rate_limiter = rate_limiter.clone();
            handles.push(tokio::spawn(async move {
                rate_limiter.acquire().await;
                Instant::now()
            }));
        }

        let times: Vec<_> = join_all(handles).await.into_iter().filter_map(Result::ok).collect();

        let elapsed = start.elapsed();
        let intervals = elapsed.as_secs_f64() / interval.as_secs_f64();
        let expected_count = (intervals * rate as f64) as usize;
        assert!(times.len() >= expected_count * 9 / 10, "Throughput too low");
    }

    #[tokio::test]
    async fn test_sustained_throughput_over_one_minute() {
        let rate = 200;
        let interval = Duration::from_millis(100);
        let rate_limiter = RateLimiter::new(rate, interval);

        // Allow the rate limiter to stabilize
        sleep(interval).await;

        // Run for 1 minute (60,000 ms) with checks at each interval
        let test_duration = Duration::from_secs(60);
        let intervals = (test_duration.as_millis() / interval.as_millis()) as usize;

        let counter = Arc::new(AtomicUsize::new(0));
        let mut throughput_data = Vec::new();

        for i in 0..intervals {
            let interval_start = Instant::now();
            let mut handles = Vec::new();

            // Attempt to acquire tokens up to the rate limit in each interval
            for _ in 0..rate {
                let rate_limiter = rate_limiter.clone();
                let counter = counter.clone();
                handles.push(tokio::spawn(async move {
                    rate_limiter.acquire().await;
                    counter.fetch_add(1, Ordering::Relaxed);
                }));
            }

            // Wait for the interval to complete
            sleep(interval.saturating_sub(interval_start.elapsed())).await;

            // Record the throughput for this interval
            let interval_count = counter.swap(0, Ordering::Relaxed);
            throughput_data.push(interval_count);

            // Output interval results for monitoring
            println!("Interval {}: {} operations", i + 1, interval_count);

            // Check that throughput is within an acceptable range (90%-100% of rate)
            let min_expected_throughput = (rate as f64 * 0.9) as usize;
            assert!(
                interval_count >= min_expected_throughput,
                "Interval {} throughput too low: {} (expected at least {})",
                i + 1,
                interval_count,
                min_expected_throughput
            );
            assert!(
                interval_count <= rate,
                "Interval {} exceeded rate limit: {} (expected at most {})",
                i + 1,
                interval_count,
                rate
            );
        }

        // Final summary and checks
        let average_throughput: f64 = throughput_data.iter().copied().sum::<usize>() as f64 / intervals as f64;
        println!("\nFinal 1-Minute Summary:");
        println!("Average throughput per interval: {:.1}", average_throughput);
        assert!(
            average_throughput >= (rate as f64 * 0.9),
            "Average throughput too low: {:.1} (should be at least {:.1})",
            average_throughput,
            rate as f64 * 0.9
        );
    }
}