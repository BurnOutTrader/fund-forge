use std::sync::atomic::{AtomicI64, Ordering};
use chrono::{DateTime, Utc, TimeZone};
use lazy_static::lazy_static;

lazy_static! {
    static ref ATOMIC_TIMESTAMP_NS: AtomicI64 = AtomicI64::new(0);
}

#[inline(always)]
pub fn update_backtest_time(dt: DateTime<Utc>) {
    let nanos = dt.timestamp_nanos_opt().unwrap();
    ATOMIC_TIMESTAMP_NS.store(nanos, Ordering::Release);
}

#[inline(always)]
pub fn get_backtest_time() -> DateTime<Utc> {
    let nanos = ATOMIC_TIMESTAMP_NS.load(Ordering::Acquire);
    Utc.timestamp_nanos(nanos)
}

pub fn advance_engine_time(duration: chrono::Duration) {
    ATOMIC_TIMESTAMP_NS.fetch_add(duration.num_nanoseconds().unwrap_or(0), Ordering::AcqRel);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_update_and_read() {
        let test_time = Utc::now();
        update_backtest_time(test_time);
        assert_eq!(get_backtest_time(), test_time);
    }

    #[test]
    fn test_advance_time() {
        let initial_time = Utc::now();
        update_backtest_time(initial_time);
        advance_engine_time(chrono::Duration::seconds(5));
        assert_eq!(get_backtest_time(), initial_time + chrono::Duration::seconds(5));
    }

    #[test]
    fn test_concurrent_reads() {
        let test_time = Utc::now();
        update_backtest_time(test_time);

        let threads: Vec<_> = (0..5).map(|_| {
            thread::spawn(move || {
                for _ in 0..100 {
                    let read_time = get_backtest_time();
                    assert!((read_time - test_time).num_nanoseconds().unwrap().abs() < 1_000_000);
                }
            })
        }).collect();

        for t in threads {
            t.join().unwrap();
        }
    }
}