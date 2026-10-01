//! Per-source progress, including HTTP bodies consumed on reqwest helper threads.
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub stage: String,
    pub completed: usize,
    pub total: Option<usize>,
    pub bytes_done: u64,
    pub bytes_total: Option<u64>,
    #[serde(default)]
    pub eta_seconds: Option<u64>,
}
#[derive(Default)]
struct ReportState {
    progress: Progress,
    last_emit: Option<std::time::Instant>,
}
const REPORT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);
#[derive(Clone)]
pub struct Reporter {
    state: Arc<Mutex<ReportState>>,
    body_started: Arc<Mutex<std::time::Instant>>,
    emit: Arc<dyn Fn(Progress) + Send + Sync>,
}
thread_local! { static CURRENT: RefCell<Option<Reporter>> = const { RefCell::new(None) }; }
pub struct Scope(Option<Reporter>);
impl Drop for Scope {
    fn drop(&mut self) {
        let active = CURRENT.with(|c| c.replace(self.0.take()));
        if let Some(reporter) = active {
            reporter.flush();
        }
    }
}
pub fn listen(emit: impl Fn(Progress) + Send + Sync + 'static) -> Scope {
    let reporter = Reporter {
        state: Default::default(),
        body_started: Arc::new(Mutex::new(std::time::Instant::now())),
        emit: Arc::new(emit),
    };
    Scope(CURRENT.with(|c| c.replace(Some(reporter))))
}
pub fn current() -> Option<Reporter> {
    CURRENT.with(|c| c.borrow().clone())
}
impl Reporter {
    fn update(&self, force: bool, change: impl FnOnce(&mut Progress)) {
        self.update_at(std::time::Instant::now(), force, change);
    }
    fn update_at(&self, now: std::time::Instant, force: bool, change: impl FnOnce(&mut Progress)) {
        if let Ok(mut state) = self.state.lock() {
            let completed_before = state.progress.completed;
            let bytes_before = state.progress.bytes_done;
            change(&mut state.progress);
            let p = &state.progress;
            let complete = p
                .total
                .is_some_and(|n| completed_before < n && p.completed >= n)
                || p.bytes_total
                    .is_some_and(|n| bytes_before < n && p.bytes_done >= n);
            if force
                || complete
                || state
                    .last_emit
                    .is_none_or(|t| now.duration_since(t) >= REPORT_INTERVAL)
            {
                state.last_emit = Some(now);
                (self.emit)(state.progress.clone());
            }
        }
    }
    fn flush(&self) {
        self.update(true, |_| {});
    }
    pub fn bytes(&self, n: usize) {
        let elapsed = self
            .body_started
            .lock()
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0);
        self.update(false, |s| {
            s.bytes_done = s.bytes_done.saturating_add(n as u64);
            s.eta_seconds = estimate(s.bytes_done, s.bytes_total, elapsed);
        });
    }
    pub fn body(&self, total: Option<u64>) {
        if let Ok(mut t) = self.body_started.lock() {
            *t = std::time::Instant::now();
        }
        self.update(true, |s| {
            s.bytes_done = 0;
            s.eta_seconds = None;
            s.bytes_total = total;
        });
    }
}
pub fn stage(stage: &str, total: Option<usize>) {
    if let Some(r) = current() {
        r.update(true, |s| {
            *s = Progress {
                stage: stage.into(),
                total,
                ..Default::default()
            }
        });
    }
}
pub fn advance() {
    if let Some(r) = current() {
        r.update(false, |s| s.completed += 1);
    }
}

fn estimate(done: u64, total: Option<u64>, elapsed: f64) -> Option<u64> {
    let total = total?;
    if done < 65536 || elapsed < 1.0 || !elapsed.is_finite() {
        return None;
    }
    Some(((total.saturating_sub(done) as f64 * elapsed / done as f64).ceil() as u64).min(86400))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn helper_thread_retains_source_and_scope_restores_previous_listener() {
        let a = Arc::new(Mutex::new(Vec::new()));
        let b = Arc::new(Mutex::new(Vec::new()));
        let out = a.clone();
        let outer = listen(move |p| out.lock().unwrap().push(p));
        stage("upload", Some(2));
        let reporter = current().unwrap();
        let out = b.clone();
        {
            let _inner = listen(move |p| out.lock().unwrap().push(p));
            stage("scan", None);
            advance();
        }
        std::thread::spawn(move || {
            assert!(current().is_none());
            reporter.body(Some(100));
            reporter.bytes(40);
        })
        .join()
        .unwrap();
        advance();
        drop(outer);
        assert_eq!(a.lock().unwrap().last().unwrap().bytes_done, 40);
        assert_eq!(a.lock().unwrap().last().unwrap().completed, 1);
        assert_eq!(b.lock().unwrap().last().unwrap().stage, "scan");
        assert!(current().is_none());
    }
    #[test]
    fn rapid_updates_are_coalesced_but_completion_and_scope_exit_are_exact() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let out = events.clone();
        let scope = listen(move |p| out.lock().unwrap().push(p));
        stage("scan", Some(10_001));
        let reporter = current().unwrap();
        let now = reporter.state.lock().unwrap().last_emit.unwrap();
        for _ in 0..10_000 {
            reporter.update_at(now, false, |p| p.completed += 1);
        }
        assert_eq!(events.lock().unwrap().len(), 1);
        reporter.update_at(now, false, |p| p.completed += 1);
        assert_eq!(events.lock().unwrap().last().unwrap().completed, 10_001);
        stage("download", None);
        reporter.body(Some(1_000));
        reporter.bytes(37);
        drop(scope);
        assert_eq!(events.lock().unwrap().last().unwrap().bytes_done, 37);
    }
    #[test]
    fn eta_requires_measurable_known_payload() {
        assert_eq!(estimate(10, Some(100), 2.0), None);
        assert_eq!(estimate(65536, None, 2.0), None);
        assert_eq!(estimate(65536, Some(131072), 0.2), None);
        assert_eq!(estimate(65536, Some(131072), 2.0), Some(2));
        assert_eq!(estimate(131072, Some(65536), 2.0), Some(0));
    }
}
