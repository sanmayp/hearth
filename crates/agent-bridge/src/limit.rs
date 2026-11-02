//! Limit. It caps sends at 10 per minute.
//! It uses a sliding window per token.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Sliding-window rate limiter. One token shares one bucket.
pub struct Limiter {
    /// Times of recent accepted sends.
    hits: Mutex<Vec<Instant>>,
    /// Max sends per window.
    max: usize,
    /// Window length in seconds.
    window_secs: u64,
}

impl Limiter {
    /// Build a limiter. Prod uses 10 hits per 60 seconds.
    pub fn new(max: usize, window_secs: u64) -> Self {
        Self {
            hits: Mutex::new(vec![]),
            max,
            window_secs,
        }
    }

    /// Try one send. It returns retry seconds when full.
    pub fn try_send(&self) -> Result<(), u64> {
        let mut hits = self.hits.lock().unwrap();
        let now = Instant::now();
        let window = Duration::from_secs(self.window_secs);
        hits.retain(|t| now.duration_since(*t) < window);
        if hits.len() >= self.max {
            let oldest = hits.iter().min().copied().unwrap_or(now);
            let wait = window
                .checked_sub(now.duration_since(oldest))
                .unwrap_or_default();
            return Err(wait.as_secs() + 1);
        }
        hits.push(now);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenth_send_passes_and_eleventh_fails() {
        let limit = Limiter::new(10, 60);
        for _ in 0..10 {
            assert!(limit.try_send().is_ok());
        }
        let wait = limit.try_send().unwrap_err();
        assert!((1..=60).contains(&wait));
    }

    #[test]
    fn window_refills_after_time() {
        let limit = Limiter::new(1, 1);
        assert!(limit.try_send().is_ok());
        assert!(limit.try_send().is_err());
        std::thread::sleep(Duration::from_millis(1100));
        assert!(limit.try_send().is_ok());
    }
}
