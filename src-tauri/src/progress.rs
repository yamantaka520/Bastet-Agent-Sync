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
#[derive(Clone)]
pub struct Reporter {
    state: Arc<Mutex<Progress>>,
    body_started: Arc<Mutex<std::time::Instant>>,
    emit: Arc<dyn Fn(Progress) + Send + Sync>,
}
thread_local! { static CURRENT: RefCell<Option<Reporter>> = const { RefCell::new(None) }; }
pub struct Scope(Option<Reporter>);
impl Drop for Scope {
    fn drop(&mut self) {
        CURRENT.with(|c| *c.borrow_mut() = self.0.take());
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
    fn update(&self, change: impl FnOnce(&mut Progress)) {
        if let Ok(mut state) = self.state.lock() {
            change(&mut state);
            (self.emit)(state.clone());
        }
    }
    pub fn bytes(&self, n: usize) {
        let elapsed = self
            .body_started
            .lock()
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0);
        self.update(|s| {
            s.bytes_done = s.bytes_done.saturating_add(n as u64);
            s.eta_seconds = estimate(s.bytes_done, s.bytes_total, elapsed);
        });
    }
    pub fn body(&self, total: Option<u64>) {
        if let Ok(mut t) = self.body_started.lock() {
            *t = std::time::Instant::now();
        }
        self.update(|s| {
            s.bytes_done = 0;
            s.eta_seconds = None;
            s.bytes_total = total;
        });
    }
}
pub fn stage(stage: &str, total: Option<usize>) {
    if let Some(r) = current() {
        r.update(|s| {
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
        r.update(|s| s.completed += 1);
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
        assert_eq!(a.lock().unwrap().last().unwrap().bytes_done, 40);
        assert_eq!(a.lock().unwrap().last().unwrap().completed, 1);
        assert_eq!(b.lock().unwrap().last().unwrap().stage, "scan");
        drop(outer);
        assert!(current().is_none());
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
