use std::sync::atomic::{AtomicI64, Ordering};
use chrono::{DateTime, TimeZone, Utc};
use lazy_static::lazy_static;

lazy_static! {
    static ref ATOMIC_TIMESTAMP_NS: AtomicI64 = AtomicI64::new(0);
}

#[inline(always)]
pub fn update_engine_time(dt: DateTime<Utc>) {
    let nanos = dt.timestamp_nanos_opt().unwrap();
    ATOMIC_TIMESTAMP_NS.store(nanos, Ordering::Release);
}

#[inline(always)]
pub fn read_engine_time() -> DateTime<Utc> {
    let nanos = ATOMIC_TIMESTAMP_NS.load(Ordering::Acquire);
    Utc.timestamp_nanos(nanos)
}

// Optional: Function to advance time by a duration
pub fn advance_engine_time(duration: chrono::Duration) {
    ATOMIC_TIMESTAMP_NS.fetch_add(duration.num_nanoseconds().unwrap_or(0), Ordering::AcqRel);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_update_and_read() {
        let test_time = Utc::now();
        update_engine_time(test_time);
        assert_eq!(read_engine_time(), test_time);
    }

    #[test]
    fn test_advance_time() {
        let initial_time = Utc::now();
        update_engine_time(initial_time);
        advance_engine_time(chrono::Duration::seconds(5));
        assert_eq!(read_engine_time(), initial_time + chrono::Duration::seconds(5));
    }

    #[test]
    fn test_concurrent_reads() {
        let test_time = Utc::now();
        update_engine_time(test_time);

        let threads: Vec<_> = (0..10).map(|_| {
            let test_time = test_time.clone();
            thread::spawn(move|| {
                for _ in 0..1000 {
                    assert_eq!(read_engine_time(), test_time);
                }
            })
        }).collect();

        for t in threads {
            t.join().unwrap();
        }
    }
}
