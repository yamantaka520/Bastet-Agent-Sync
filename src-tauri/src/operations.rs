//! Bounded local audit history, encrypted device reports and rebuildable cache maintenance.
use crate::{
    cloud::{
        crypto::SpaceKey,
        drive::{Drive, ObjectKind},
        queue::Binding,
    },
    model::Settings,
    native_sessions::SourceStatus,
    sync::{
        bundle::{self, Bundle, Entry, Result, Snapshot, Stream},
        storage,
    },
    worker::Worker,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::Manager;
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn read_json<T: for<'a> Deserialize<'a> + Default>(path: &Path, limit: u64) -> Result<T> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(_) => Err("store_unavailable".into()),
        Ok(_) => serde_json::from_slice(&storage::read(path, limit)?)
            .map_err(|_| "local_store_damaged".into()),
    }
}
fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    storage::replace(
        path,
        &serde_json::to_vec(value).map_err(|_| "local_store_damaged")?,
    )
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    pub started: u64,
    pub finished: u64,
    pub outcome: String,
    pub error: Option<String>,
    pub sources: Vec<SourceStatus>,
}
pub fn record(root: &Path, item: History) -> Result<()> {
    let path = root.join("sync-history.json");
    let mut history: Vec<History> = read_json(&path, 16 * 1024 * 1024)?;
    history.push(item);
    if history.len() > 500 {
        history.drain(..history.len() - 500);
    }
    write_json(&path, &history)
}
#[derive(Default, Serialize, Deserialize)]
struct Identity {
    id: String,
    files: BTreeMap<String, String>,
}
fn identity(root: &Path) -> Result<Identity> {
    let path = root.join("device-identity.json");
    let mut identity: Identity = read_json(&path, 65536)?;
    if identity.id.is_empty() {
        identity.id = uuid::Uuid::new_v4().to_string();
        write_json(&path, &identity)?;
    }
    if !bundle::token(&identity.id) {
        return Err("local_store_damaged".into());
    }
    Ok(identity)
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceReport {
    pub id: String,
    pub name: String,
    pub os: String,
    pub version: String,
    pub reported_at: u64,
    pub observed_at: u64,
    pub outcome: String,
    pub agents: Vec<String>,
}
impl DeviceReport {
    fn validate(&self) -> Result<()> {
        if !bundle::token(&self.id)
            || self.name.chars().count() > 80
            || self.name.chars().any(char::is_control)
            || self.os.len() > 32
            || self.version.len() > 32
            || !["complete", "partial", "error"].contains(&self.outcome.as_str())
            || self.agents.len() > 8
            || self
                .agents
                .iter()
                .any(|a| !crate::model::AGENTS.contains(&a.as_str()))
        {
            Err("device_report_invalid".into())
        } else {
            Ok(())
        }
    }
}
pub fn device_exchange(
    root: &Path,
    settings: &Settings,
    binding: &Binding,
    key: &SpaceKey,
    drive: &Drive,
    outcome: &str,
    stop: impl Fn() -> bool,
) -> Result<()> {
    let path = root.join(format!("devices-{}.json", binding.space));
    let mut reports: Vec<DeviceReport> = read_json(&path, 1024 * 1024)?;
    let mut identity = identity(root)?;
    // Successful refreshes, including download-only mode, are bounded to once per minute.
    let recent = std::fs::metadata(&path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .is_some_and(|t| t.as_secs().saturating_add(60) > now());
    if recent {
        return Ok(());
    }
    if stop() {
        return Err("sync_paused".into());
    }
    if settings.direction != "download" {
        let report = DeviceReport {
            id: identity.id.clone(),
            name: settings.device_name.clone(),
            os: std::env::consts::OS.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            reported_at: now(),
            observed_at: 0,
            outcome: outcome.into(),
            agents: settings.selected_agents.clone(),
        };
        report.validate()?;
        let b = Bundle::new(Snapshot {
            schema: 1,
            space: binding.space.clone(),
            device: identity.id.clone(),
            stream: Stream {
                agent: "codex".into(),
                profile: identity.id.clone(),
                conversation: "bastet-device-report".into(),
            },
            parents: vec![],
            files: BTreeMap::from([(
                "device.json".into(),
                Entry::new(serde_json::to_string(&report).map_err(|_| "device_report_invalid")?),
            )]),
        })?;
        let file = match identity.files.get(&binding.space) {
            Some(f) => f.clone(),
            None => {
                let file = drive.allocate_id()?;
                identity.files.insert(binding.space.clone(), file.clone());
                write_json(&root.join("device-identity.json"), &identity)?;
                file
            }
        };
        match drive.update_device(&binding.folder, &file, &binding.space, key, &b) {
            Ok(()) => {}
            Err(e) if e == "drive_not_found" => {
                drive.upload_kind(&binding.folder, &file, key, &b, ObjectKind::Device)?;
            }
            Err(e) => return Err(e),
        }
        let mut local = report;
        local.observed_at = now();
        reports.retain(|r| r.id != local.id);
        reports.push(local);
    }
    if settings.direction != "upload" {
        let files = drive.list_kind(&binding.folder, ObjectKind::Device)?;
        if files.len() > 128 {
            return Err("device_limit".into());
        }
        for file in files {
            if stop() {
                return Err("sync_paused".into());
            }
            let b = drive.download_kind(
                &binding.folder,
                &file.id,
                &binding.space,
                key,
                ObjectKind::Device,
            )?;
            let mut report: DeviceReport = serde_json::from_str(
                &b.snapshot
                    .files
                    .get("device.json")
                    .ok_or("device_report_invalid")?
                    .content,
            )
            .map_err(|_| "device_report_invalid")?;
            report.validate()?;
            if report.id != b.snapshot.stream.profile
                || b.snapshot.stream.conversation != "bastet-device-report"
            {
                return Err("device_report_invalid".into());
            }
            report.observed_at = now();
            reports.retain(|r| r.id != report.id);
            reports.push(report);
        }
    }
    reports.sort_by_key(|a| std::cmp::Reverse(a.observed_at));
    reports.truncate(128);
    write_json(&path, &reports)
}
#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageUsage {
    pub local_bytes: u64,
    pub cache_bytes: u64,
    pub files: usize,
}
fn sizes(
    path: &Path,
    usage: &mut StorageUsage,
    cache: bool,
    depth: usize,
    visited: &mut usize,
) -> Result<()> {
    *visited += 1;
    if depth > 32 || *visited > 100_000 {
        return Err("storage_scan_limit".into());
    }
    let meta = std::fs::symlink_metadata(path).map_err(|_| "store_unavailable")?;
    if meta.file_type().is_symlink() {
        return Ok(());
    }
    if meta.is_file() {
        usage.files += 1;
        if usage.files > 100_000 {
            return Err("storage_scan_limit".into());
        }
        usage.local_bytes = usage.local_bytes.saturating_add(meta.len());
        if cache {
            usage.cache_bytes = usage.cache_bytes.saturating_add(meta.len());
        }
    } else if meta.is_dir() {
        for p in std::fs::read_dir(path).map_err(|_| "store_unavailable")? {
            let p = p.map_err(|_| "store_unavailable")?.path();
            sizes(
                &p,
                usage,
                cache
                    || p.file_name()
                        .is_some_and(|s| s.to_string_lossy().starts_with("drive-cache-")),
                depth + 1,
                visited,
            )?;
        }
    }
    Ok(())
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub history: Vec<History>,
    pub devices: Vec<DeviceReport>,
}
fn root(app: &tauri::AppHandle) -> Result<PathBuf> {
    app.path()
        .app_config_dir()
        .map_err(|_| "store_unavailable".into())
}
#[tauri::command]
pub async fn operations_view(app: tauri::AppHandle) -> Result<View> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = root(&app)?;
        let history = read_json(&root.join("sync-history.json"), 16 * 1024 * 1024)?;
        let tx = crate::cloud::wizard::Transaction::open(&root)?;
        let devices = match tx.state.binding {
            Some(b) => read_json(&root.join(format!("devices-{}.json", b.space)), 1024 * 1024)?,
            None => Vec::new(),
        };
        Ok(View { history, devices })
    })
    .await
    .map_err(|_| "store_unavailable".to_string())?
}
#[tauri::command]
pub async fn storage_usage(app: tauri::AppHandle) -> Result<StorageUsage> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut usage = StorageUsage::default();
        sizes(&root(&app)?, &mut usage, false, 0, &mut 0)?;
        Ok(usage)
    })
    .await
    .map_err(|_| "store_unavailable".to_string())?
}
#[tauri::command]
pub async fn clear_download_cache(
    app: tauri::AppHandle,
    worker: tauri::State<'_, Worker>,
    cloud: tauri::State<'_, crate::cloud::desktop::CloudState>,
) -> Result<usize> {
    if worker.active() {
        return Err("sync_running".into());
    }
    let cloud = cloud.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = cloud.0.try_lock().map_err(|_| "cloud_busy")?;
        let root = root(&app)?;
        let tx = crate::cloud::wizard::Transaction::open(&root)?;
        let binding = tx.state.binding.ok_or("wizard_step_required")?;
        let path = root.join(format!("drive-cache-{}", binding.space));
        clear_cache(&path)
    })
    .await
    .map_err(|_| "store_unavailable".to_string())?
}
fn clear_cache(path: &Path) -> Result<usize> {
    if !path.exists() {
        return Ok(0);
    }
    storage::directory(path)?;
    let mut deleted = 0;
    for e in std::fs::read_dir(path).map_err(|_| "store_unavailable")? {
        let e = e.map_err(|_| "store_unavailable")?;
        let name = e.file_name();
        let name = name.to_string_lossy();
        if !name.strip_suffix(".json").is_some_and(bundle::is_hash)
            || !e.file_type().map_err(|_| "store_unavailable")?.is_file()
        {
            continue;
        }
        std::fs::remove_file(e.path()).map_err(|_| "store_unavailable")?;
        deleted += 1;
    }
    Ok(deleted)
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudUsage {
    pub bytes: u64,
    pub objects: usize,
    pub measured_at: u64,
}
#[tauri::command]
pub async fn cloud_storage_usage(
    app: tauri::AppHandle,
    cloud: tauri::State<'_, crate::cloud::desktop::CloudState>,
) -> Result<CloudUsage> {
    let cloud = cloud.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let guard = cloud.0.try_lock().map_err(|_| "cloud_busy")?;
        let drive = guard.as_ref().ok_or("reauth_required")?;
        let tx = crate::cloud::wizard::Transaction::open(&root(&app)?)?;
        let b = tx.state.binding.ok_or("wizard_step_required")?;
        let mut objects = Vec::new();
        for kind in [
            ObjectKind::Session,
            ObjectKind::Device,
            ObjectKind::Portable,
        ] {
            objects.extend(drive.list_kind(&b.folder, kind)?);
        }
        let bytes = objects.iter().try_fold(0u64, |n, f| {
            f.size
                .as_ref()
                .and_then(|s| s.parse::<u64>().ok())
                .and_then(|v| n.checked_add(v))
                .ok_or("drive_size_unavailable")
        })?;
        Ok(CloudUsage {
            bytes,
            objects: objects.len(),
            measured_at: now(),
        })
    })
    .await
    .map_err(|_| "store_unavailable".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_keeps_latest_500_across_reopens_and_reports_corruption() {
        let root = tempfile::tempdir().unwrap();
        for i in 0..503 {
            record(
                root.path(),
                History {
                    started: i,
                    finished: i + 1,
                    outcome: "complete".into(),
                    ..Default::default()
                },
            )
            .unwrap();
        }
        let history: Vec<History> =
            read_json(&root.path().join("sync-history.json"), 16 * 1024 * 1024).unwrap();
        assert_eq!(history.len(), 500);
        assert_eq!(history[0].started, 3);
        assert_eq!(history[499].started, 502);
        std::fs::write(root.path().join("sync-history.json"), b"damaged").unwrap();
        assert!(record(root.path(), History::default()).is_err());
    }
    #[test]
    fn cleanup_deletes_only_rebuildable_hash_objects_and_preserves_other_data() {
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("drive-cache-space");
        std::fs::create_dir(&cache).unwrap();
        std::fs::write(cache.join(format!("{}.json", "a".repeat(64))), b"cache").unwrap();
        std::fs::write(cache.join("journal.json"), b"keep").unwrap();
        std::fs::write(root.path().join("replica.json"), b"keep").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            root.path().join("replica.json"),
            cache.join(format!("{}.json", "b".repeat(64))),
        )
        .unwrap();
        let mut usage = StorageUsage::default();
        sizes(root.path(), &mut usage, false, 0, &mut 0).unwrap();
        assert_eq!(usage.cache_bytes, 9);
        assert_eq!(usage.local_bytes, 13);
        assert_eq!(clear_cache(&cache).unwrap(), 1);
        assert_eq!(clear_cache(&cache).unwrap(), 0);
        assert!(cache.join("journal.json").is_file());
        assert!(root.path().join("replica.json").is_file());
    }
    #[test]
    fn identity_survives_restart_and_invalid_reports_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(
            identity(root.path()).unwrap().id,
            identity(root.path()).unwrap().id
        );
        let mut report = DeviceReport {
            id: "device".into(),
            name: "Cat".into(),
            outcome: "complete".into(),
            agents: vec!["codex".into()],
            ..Default::default()
        };
        assert!(report.validate().is_ok());
        report.agents.push("unknown".into());
        assert!(report.validate().is_err());
        report.agents.clear();
        report.name = "a\nb".into();
        assert!(report.validate().is_err());
    }
}
