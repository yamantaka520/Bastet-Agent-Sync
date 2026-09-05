//! Explicitly started, non-overlapping AMOS exchange; unsupported sources never veto ready ones.
use crate::{
    amos_runtime,
    cloud::{
        desktop::CloudState,
        queue::{self, Binding, Objects},
        vault::{load_space_key, NativeStore},
        wizard::Transaction,
    },
    model::Settings,
    sync::{
        bundle::{self, Result, Stream},
        storage, Direction, Replica,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{Manager, State};
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub running: bool,
    pub phase: String,
    pub published: usize,
    pub received: usize,
    pub applied: usize,
    pub last_success: Option<u64>,
    pub error: Option<String>,
    pub skipped: Vec<String>,
}
#[derive(Default)]
struct Control {
    status: Status,
    stop: bool,
    wake: bool,
}
#[derive(Clone, Default)]
pub struct Worker(Arc<(Mutex<Control>, Condvar)>);
impl Worker {
    pub fn active(&self) -> bool {
        self.0 .0.lock().map(|c| c.status.running).unwrap_or(true)
    }
    fn stopped(&self) -> bool {
        self.0 .0.lock().map(|c| c.stop).unwrap_or(true)
    }
    fn update(&self, f: impl FnOnce(&mut Status)) {
        if let Ok(mut c) = self.0 .0.lock() {
            f(&mut c.status);
        }
    }
}
#[derive(Default, Serialize, Deserialize)]
struct Ledger {
    node: String,
    base: Option<String>,
    fingerprint: String,
    applied: BTreeSet<String>,
}
fn ledger(root: &Path) -> Result<Ledger> {
    let p = root.join("apply.json");
    if !p.exists() {
        return Ok(Ledger {
            node: uuid::Uuid::new_v4().to_string(),
            ..Default::default()
        });
    }
    let l: Ledger = serde_json::from_slice(&storage::read(&p, 1024 * 1024)?)
        .map_err(|_| "sync_journal_invalid")?;
    if !bundle::token(&l.node) || l.applied.len() > bundle::MAX_OBJECTS {
        return Err("sync_journal_invalid".into());
    }
    Ok(l)
}
fn save(root: &Path, l: &Ledger) -> Result<()> {
    storage::replace(
        &root.join("apply.json"),
        &serde_json::to_vec(l).map_err(|_| "sync_journal_invalid")?,
    )
}
pub trait Memory {
    fn export(&self, staging: &Path) -> Result<String>;
    fn apply(&self, staging: &Path, text: &str, id: &str) -> Result<()>;
}
struct Installed {
    cli: PathBuf,
    home: PathBuf,
}
impl Memory for Installed {
    fn export(&self, staging: &Path) -> Result<String> {
        amos_runtime::export(&self.cli, &self.home, staging)
    }
    fn apply(&self, staging: &Path, text: &str, id: &str) -> Result<()> {
        amos_runtime::apply(&self.cli, &self.home, staging, text, id)
    }
}
struct Cancellable<'a, R, F> {
    remote: &'a R,
    stop: &'a F,
}
impl<R: Objects, F: Fn() -> bool> Objects for Cancellable<'_, R, F> {
    fn ids(&self, f: &str) -> Result<Vec<String>> {
        if (self.stop)() {
            return Err("sync_paused".into());
        }
        self.remote.ids(f)
    }
    fn allocate(&self) -> Result<String> {
        if (self.stop)() {
            return Err("sync_paused".into());
        }
        self.remote.allocate()
    }
    fn put(
        &self,
        f: &str,
        id: &str,
        k: &crate::cloud::crypto::SpaceKey,
        b: &bundle::Bundle,
    ) -> Result<()> {
        if (self.stop)() {
            return Err("sync_paused".into());
        }
        self.remote.put(f, id, k, b)
    }
    fn get(
        &self,
        f: &str,
        id: &str,
        s: &str,
        k: &crate::cloud::crypto::SpaceKey,
    ) -> Result<bundle::Bundle> {
        if (self.stop)() {
            return Err("sync_paused".into());
        }
        self.remote.get(f, id, s, k)
    }
}
/// Per-device streams avoid claiming that one device's export is another's baseline.
/// The official AMOS merge owns timestamp/ACL/conflict semantics.
pub fn cycle(
    root: &Path,
    binding: &Binding,
    key: &crate::cloud::crypto::SpaceKey,
    remote: &impl Objects,
    memory: &impl Memory,
    direction: Direction,
    stop: impl Fn() -> bool,
) -> Result<(queue::Exchange, usize)> {
    storage::directory(root)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| "store_unavailable")?;
    }
    let replica = Replica::open(&root.join("replica"), &binding.space)?;
    let mut l = ledger(root)?;
    save(root, &l)?;
    if stop() {
        return Err("sync_paused".into());
    }
    if !matches!(direction, Direction::Download) {
        let text = memory.export(root)?;
        let fingerprint = amos_runtime::fingerprint(&text)?;
        if fingerprint != l.fingerprint {
            let id = replica.export_from(
                Stream {
                    agent: "agent-memory-os".into(),
                    profile: l.node.clone(),
                    conversation: "memory-store".into(),
                },
                crate::memory_adapter::capture_export(text)?,
                l.base.as_deref(),
            )?;
            l.base = Some(id.clone());
            l.fingerprint = fingerprint;
            l.applied.insert(id);
            save(root, &l)?;
        }
    }
    if stop() {
        return Err("sync_paused".into());
    }
    let report = queue::exchange_filtered(
        &root.join("exchange"),
        &replica,
        binding,
        key,
        &Cancellable {
            remote,
            stop: &stop,
        },
        direction,
        Some("agent-memory-os"),
    )?;
    let mut applied = 0;
    if !matches!(direction, Direction::Upload) {
        let bundles = replica.transport_bundles()?;
        // Parents first. Applying an older profile after its descendant would roll metadata back.
        let mut remaining = bundles
            .values()
            .filter(|b| b.snapshot.stream.agent == "agent-memory-os" && !l.applied.contains(&b.id))
            .collect::<Vec<_>>();
        while !remaining.is_empty() {
            if stop() {
                return Err("sync_paused".into());
            }
            let position = remaining
                .iter()
                .position(|b| b.snapshot.parents.iter().all(|p| l.applied.contains(p)))
                .ok_or("history_pending")?;
            let b = remaining.remove(position);
            if b.snapshot.stream.profile != l.node {
                memory.apply(root, &crate::memory_adapter::restore_export(b)?, &b.id)?;
                applied += 1;
            }
            l.applied.insert(b.id.clone());
            save(root, &l)?;
        }
    }
    Ok((report, applied))
}
fn run_once(
    app: &tauri::AppHandle,
    cloud: &CloudState,
    worker: &Worker,
    settings: &Settings,
    binding: &Binding,
    memory: &Installed,
) -> Result<(queue::Exchange, usize)> {
    let mut guard = cloud.0.try_lock().map_err(|_| "cloud_busy")?;
    let root = app
        .path()
        .app_config_dir()
        .map_err(|_| "store_unavailable")?;
    let t = Transaction::open(&root)?;
    if !t.state.complete || t.state.binding.as_ref() != Some(binding) {
        return Err("sync_setup_changed".into());
    }
    if !guard.as_ref().is_some_and(|d| d.is_connected()) {
        let config = crate::cloud::wizard_desktop::config(&t.state)?;
        let token = crate::cloud::oauth::reconnect(&config, &NativeStore)?;
        let drive = crate::cloud::drive::Drive::new(token)?;
        t.check_account(&drive.account()?)?;
        if t.state.account.is_none() {
            return Err("reauth_required".into());
        }
        *guard = Some(drive);
    }
    let key = load_space_key(&NativeStore, &binding.space)?;
    let direction = match settings.direction.as_str() {
        "upload" => Direction::Upload,
        "download" => Direction::Download,
        _ => Direction::Both,
    };
    cycle(
        &root.join(format!("memory-sync-{}", binding.space)),
        binding,
        &key,
        guard.as_ref().ok_or("reauth_required")?,
        memory,
        direction,
        || worker.stopped(),
    )
}
#[tauri::command]
pub fn sync_status(worker: State<'_, Worker>) -> Result<Status> {
    Ok(worker.0 .0.lock().map_err(|_| "sync_busy")?.status.clone())
}
#[tauri::command]
pub fn sync_pause(worker: State<'_, Worker>) -> Result<()> {
    let mut c = worker.0 .0.lock().map_err(|_| "sync_busy")?;
    c.stop = true;
    if c.status.running {
        c.status.phase = "pausing".into();
    }
    worker.0 .1.notify_all();
    Ok(())
}
#[tauri::command]
pub fn sync_now(worker: State<'_, Worker>) -> Result<()> {
    let mut c = worker.0 .0.lock().map_err(|_| "sync_busy")?;
    if !c.status.running {
        return Err("sync_not_started".into());
    }
    c.wake = true;
    worker.0 .1.notify_all();
    Ok(())
}
#[tauri::command]
pub async fn sync_start(
    app: tauri::AppHandle,
    worker: State<'_, Worker>,
    cloud: State<'_, CloudState>,
) -> Result<Status> {
    let worker = worker.inner().clone();
    let cloud = cloud.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let root = app
            .path()
            .app_config_dir()
            .map_err(|_| "store_unavailable")?;
        let settings =
            crate::model::load(&root.join("settings.json"))?.ok_or("settings_unreadable")?;
        crate::model::validate(&settings)?;
        let t = Transaction::open(&root)?;
        if !t.state.complete {
            return Err("wizard_step_required".into());
        }
        let binding = t.state.binding.clone().ok_or("wizard_step_required")?;
        if !settings
            .selected_agents
            .iter()
            .any(|s| s == "agent-memory-os")
        {
            return Err("no_ready_sources".into());
        }
        let agent = crate::detect(Some(&settings))
            .into_iter()
            .find(|a| a.id == "agent-memory-os")
            .ok_or("memory_store_missing")?;
        let memory = Installed {
            cli: amos_runtime::configured(&root)?,
            home: PathBuf::from(agent.path),
        };
        if !memory.home.join("memories.db").is_file() {
            return Err("memory_store_missing".into());
        }
        let status = Status {
            running: true,
            phase: "starting".into(),
            skipped: settings
                .selected_agents
                .iter()
                .filter(|s| *s != "agent-memory-os")
                .cloned()
                .collect(),
            ..Default::default()
        };
        {
            let mut c = worker.0 .0.lock().map_err(|_| "sync_busy")?;
            if c.status.running {
                return Err("sync_busy".into());
            }
            c.stop = false;
            c.wake = false;
            c.status = status.clone();
        }
        let thread_worker = worker.clone();
        if std::thread::Builder::new()
            .name("bastet-sync".into())
            .spawn(move || {
                let worker = thread_worker;
                let mut failures = 0u32;
                loop {
                    if worker.stopped() {
                        break;
                    }
                    worker.update(|s| {
                        s.phase = "syncing".into();
                        s.error = None;
                    });
                    let result = run_once(&app, &cloud, &worker, &settings, &binding, &memory);
                    let success = result.is_ok();
                    match result {
                        Ok((report, applied)) => {
                            failures = 0;
                            worker.update(|s| {
                                s.phase = "waiting".into();
                                s.published += report.published;
                                s.received += report.received;
                                s.applied += applied;
                                s.last_success = Some(
                                    SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs(),
                                );
                            });
                        }
                        Err(e) if e == "sync_paused" => break,
                        Err(e) => {
                            failures = failures.saturating_add(1);
                            worker.update(|s| {
                                s.phase = "error".into();
                                s.error = Some(e);
                            });
                        }
                    }
                    let seconds = if !success {
                        (15u64.saturating_mul(1u64 << failures.min(6))).min(900)
                    } else if settings.schedule == "near-realtime" {
                        15
                    } else {
                        settings.interval_seconds as u64
                    };
                    let mut c = match worker.0 .0.lock() {
                        Ok(c) => c,
                        Err(_) => return,
                    };
                    if c.stop {
                        break;
                    }
                    if settings.schedule == "manual" {
                        while !c.stop && !c.wake {
                            c = worker.0 .1.wait(c).unwrap();
                        }
                    } else {
                        c = worker
                            .0
                             .1
                            .wait_timeout_while(c, Duration::from_secs(seconds), |c| {
                                !c.stop && !c.wake
                            })
                            .unwrap()
                            .0;
                    }
                    c.wake = false;
                }
                worker.update(|s| {
                    s.running = false;
                    s.phase = "paused".into();
                });
            })
            .is_err()
        {
            worker.update(|s| {
                s.running = false;
                s.phase = "error".into();
                s.error = Some("sync_busy".into());
            });
            return Err("sync_busy".into());
        }
        Ok(status)
    })
    .await
    .map_err(|_| "sync_busy".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::collections::BTreeMap;
    struct Remote(
        RefCell<BTreeMap<String, crate::sync::bundle::Bundle>>,
        Cell<usize>,
    );
    impl Objects for Remote {
        fn ids(&self, _: &str) -> Result<Vec<String>> {
            Ok(self.0.borrow().keys().cloned().collect())
        }
        fn allocate(&self) -> Result<String> {
            let n = self.1.get();
            self.1.set(n + 1);
            Ok(format!("id-{n}"))
        }
        fn put(
            &self,
            _: &str,
            id: &str,
            _: &crate::cloud::crypto::SpaceKey,
            b: &crate::sync::bundle::Bundle,
        ) -> Result<()> {
            self.0.borrow_mut().insert(id.into(), b.clone());
            Ok(())
        }
        fn get(
            &self,
            _: &str,
            id: &str,
            _: &str,
            _: &crate::cloud::crypto::SpaceKey,
        ) -> Result<crate::sync::bundle::Bundle> {
            self.0
                .borrow()
                .get(id)
                .cloned()
                .ok_or("drive_not_found".into())
        }
    }
    struct Store(RefCell<String>, Cell<usize>, Cell<bool>);
    impl Memory for Store {
        fn export(&self, _: &Path) -> Result<String> {
            Ok(self.0.borrow().clone())
        }
        fn apply(&self, _: &Path, text: &str, _: &str) -> Result<()> {
            if self.2.get() {
                return Err("memory_import_failed".into());
            }
            *self.0.borrow_mut() = text.into();
            self.1.set(self.1.get() + 1);
            Ok(())
        }
    }
    fn text(id: &str) -> String {
        format!("{{\"kind\":\"bundle\",\"version\":3}}\n{{\"kind\":\"memory\",\"id\":\"{id}\"}}\n")
    }
    #[test]
    fn automatic_exchange_is_idempotent_and_failed_import_retries() {
        let d = tempfile::tempdir().unwrap();
        let binding = Binding {
            folder: "folder".into(),
            space: "space".into(),
            proof: "proof".into(),
        };
        let key = crate::cloud::crypto::SpaceKey::generate().unwrap();
        let remote = Remote(
            RefCell::new(BTreeMap::from([(
                "proof".into(),
                queue::proof_bundle("space").unwrap(),
            )])),
            Cell::new(0),
        );
        let a = Store(RefCell::new(text("a")), Cell::new(0), Cell::new(false));
        let b = Store(RefCell::new(text("b")), Cell::new(0), Cell::new(true));
        let ra = d.path().join("a");
        let rb = d.path().join("b");
        assert_eq!(
            cycle(&ra, &binding, &key, &remote, &a, Direction::Upload, || {
                false
            })
            .unwrap()
            .0
            .published,
            1
        );
        assert!(cycle(
            &rb,
            &binding,
            &key,
            &remote,
            &b,
            Direction::Download,
            || false
        )
        .is_err());
        assert_eq!(b.1.get(), 0);
        b.2.set(false);
        assert_eq!(
            cycle(
                &rb,
                &binding,
                &key,
                &remote,
                &b,
                Direction::Download,
                || false
            )
            .unwrap()
            .1,
            1
        );
        assert_eq!(
            cycle(
                &rb,
                &binding,
                &key,
                &remote,
                &b,
                Direction::Download,
                || false
            )
            .unwrap()
            .1,
            0
        );
        assert_eq!(
            cycle(&ra, &binding, &key, &remote, &a, Direction::Upload, || {
                false
            })
            .unwrap()
            .0
            .published,
            0
        );
        assert!(cycle(&ra, &binding, &key, &remote, &a, Direction::Both, || true).is_err());
        assert_eq!(remote.1.get(), 1);
    }
    #[test]
    fn selecting_unsupported_sources_does_not_block_memory() {
        let settings = Settings {
            selected_agents: crate::model::AGENTS.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        let p = crate::runtime_status::evaluate(Some(&settings), true);
        assert!(p.reasons.is_empty());
        assert_eq!(p.unsupported_agents.len(), 7);
    }
}
