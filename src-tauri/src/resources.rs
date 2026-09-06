//! Explicit resource budgets; zero bandwidth limit means unlimited.
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        LazyLock, Mutex,
    },
    time::{Duration, Instant},
};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Options {
    pub parallel: usize,
    pub upload_kib: u64,
    pub download_kib: u64,
    pub window_enabled: bool,
    pub start_minute: u32,
    pub end_minute: u32,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            parallel: 3,
            upload_kib: 0,
            download_kib: 0,
            window_enabled: false,
            start_minute: 0,
            end_minute: 0,
        }
    }
}
impl Options {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=6).contains(&self.parallel)
            || self.upload_kib > 1_048_576
            || self.download_kib > 1_048_576
            || self.start_minute >= 1440
            || self.end_minute >= 1440
        {
            Err("invalid_settings".into())
        } else {
            Ok(())
        }
    }
    pub fn allows(&self, minute: u32) -> bool {
        !self.window_enabled
            || self.start_minute == self.end_minute
            || if self.start_minute < self.end_minute {
                minute >= self.start_minute && minute < self.end_minute
            } else {
                minute >= self.start_minute || minute < self.end_minute
            }
    }
}
struct Budget {
    limits: [AtomicU64; 2],
    cancelled: AtomicBool,
    next: Mutex<[Instant; 2]>,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            limits: [AtomicU64::new(0), AtomicU64::new(0)],
            cancelled: AtomicBool::new(false),
            next: Mutex::new([Instant::now(); 2]),
        }
    }
}
impl Budget {
    fn configure(&self, options: &Options) {
        self.limits[0].store(options.upload_kib * 1024, Ordering::Relaxed);
        self.limits[1].store(options.download_kib * 1024, Ordering::Relaxed);
        self.cancelled.store(false, Ordering::Relaxed);
        *self.next.lock().unwrap_or_else(|e| e.into_inner()) = [Instant::now(); 2];
    }
    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
    fn clear(&self) {
        self.limits
            .iter()
            .for_each(|v| v.store(0, Ordering::Relaxed));
        self.cancelled.store(false, Ordering::Relaxed);
    }
    fn chunk(&self, direction: usize, size: usize) -> usize {
        let rate = self.limits[direction].load(Ordering::Relaxed);
        if rate == 0 {
            size
        } else {
            size.min((rate / 10).clamp(1, 16384) as usize)
        }
    }
    fn pace(&self, direction: usize, bytes: usize) -> std::io::Result<()> {
        let rate = self.limits[direction].load(Ordering::Relaxed);
        if rate == 0 || bytes == 0 {
            return Ok(());
        }
        let deadline = {
            let mut next = self.next.lock().unwrap_or_else(|e| e.into_inner());
            let now = Instant::now();
            next[direction] =
                next[direction].max(now) + Duration::from_secs_f64(bytes as f64 / rate as f64);
            next[direction]
        };
        while Instant::now() < deadline {
            if self.cancelled.load(Ordering::Relaxed) {
                return Err(std::io::Error::other("sync_paused"));
            }
            std::thread::sleep(
                deadline
                    .saturating_duration_since(Instant::now())
                    .min(Duration::from_millis(50)),
            );
        }
        Ok(())
    }
    // Request timeout includes pacing of one maximum-sized payload. Connection timeout remains separate.
    fn request_timeout(&self) -> Duration {
        let rate = self
            .limits
            .iter()
            .map(|v| v.load(Ordering::Relaxed))
            .filter(|r| *r > 0)
            .min();
        Duration::from_secs(
            60 + rate
                .map(|r| (6 * 64 * 1024 * 1024_u64).div_ceil(r))
                .unwrap_or(0),
        )
    }
}
static BUDGET: LazyLock<Budget> = LazyLock::new(Budget::default);
pub fn configure(options: &Options) {
    BUDGET.configure(options);
}
pub fn cancel() {
    BUDGET.cancel();
}
pub fn clear() {
    BUDGET.clear();
}
pub fn chunk(direction: usize, size: usize) -> usize {
    BUDGET.chunk(direction, size)
}
pub fn pace(direction: usize, bytes: usize) -> std::io::Result<()> {
    BUDGET.pace(direction, bytes)
}
pub fn request_timeout() -> Duration {
    BUDGET.request_timeout()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overnight_and_equal_windows_and_limits() {
        let mut o = Options {
            window_enabled: true,
            start_minute: 1320,
            end_minute: 360,
            ..Default::default()
        };
        assert!(o.allows(1380) && o.allows(0) && !o.allows(720) && !o.allows(360));
        o.end_minute = o.start_minute;
        assert!(o.allows(720));
        o.parallel = 7;
        assert!(o.validate().is_err());
    }
    #[test]
    fn aggregate_budget_paces_concurrent_readers_and_cancel_interrupts_waits() {
        let budget = Budget::default();
        budget.configure(&Options {
            upload_kib: 1,
            ..Default::default()
        });
        let start = Instant::now();
        std::thread::scope(|scope| {
            for _ in 0..2 {
                scope.spawn(|| budget.pace(0, 512).unwrap());
            }
        });
        assert!(start.elapsed() >= Duration::from_millis(900));
        assert_eq!(budget.chunk(0, 4096), 102);
        assert_eq!(budget.chunk(1, 4096), 4096);
        std::thread::scope(|scope| {
            let waiter = scope.spawn(|| budget.pace(0, 65536));
            std::thread::sleep(Duration::from_millis(100));
            budget.cancel();
            assert!(waiter.join().unwrap().is_err());
        });
        budget.clear();
        assert_eq!(budget.chunk(0, 4096), 4096);
    }
}
