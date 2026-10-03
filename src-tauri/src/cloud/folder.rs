//! Encrypted exchange through a user-selected, externally synchronized folder.
//! Completion here means a durable local handoff, never cloud delivery.
use super::{
    crypto::{SpaceKey, MAX_ENCRYPTED},
    queue::{self, Binding, Objects},
    vault::{load_space_key, save_space_key, NativeStore},
    Result,
};
use crate::{
    sandbox_access,
    sync::{
        bundle::{token, Bundle, MAX_OBJECTS},
        storage,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::Manager;
use zeroize::{Zeroize, Zeroizing};

const MANAGED: &str = "BastetAgentSyncData";

fn provider_ok(provider: &str) -> bool {
    matches!(provider, "icloud-drive" | "onedrive-folder")
}
fn config_file(config: &Path, provider: &str) -> Result<PathBuf> {
    if !provider_ok(provider) {
        return Err("invalid_cloud_provider".into());
    }
    Ok(config.join(format!("folder-{provider}.json")))
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FolderConfig {
    schema: u32,
    provider: String,
    path: String,
    pub binding: Option<Binding>,
    complete: bool,
}
impl FolderConfig {
    fn new(provider: &str, path: &Path) -> Self {
        Self {
            schema: 1,
            provider: provider.into(),
            path: path.to_string_lossy().into_owned(),
            binding: None,
            complete: false,
        }
    }
    pub fn path(&self) -> &Path {
        Path::new(&self.path)
    }
    pub fn complete(&self) -> bool {
        self.complete
    }
    fn validate(&self, provider: &str) -> Result<()> {
        if self.schema != 1
            || self.provider != provider
            || !provider_ok(provider)
            || !self.path().is_absolute()
            || self.path.len() > 4096
            || self.complete && self.binding.is_none()
        {
            return Err("folder_setup_invalid".into());
        }
        if let Some(binding) = &self.binding {
            binding.validate()?;
        }
        Ok(())
    }
}
pub fn load(config: &Path, provider: &str) -> Result<Option<FolderConfig>> {
    let file = config_file(config, provider)?;
    let bytes = match fs::symlink_metadata(&file) {
        Ok(_) => storage::read(&file, 65536)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("store_unavailable".into()),
    };
    let state: FolderConfig = serde_json::from_slice(&bytes).map_err(|_| "folder_setup_invalid")?;
    state.validate(provider)?;
    Ok(Some(state))
}
fn save(config: &Path, state: &FolderConfig) -> Result<()> {
    state.validate(&state.provider)?;
    storage::replace(
        &config_file(config, &state.provider)?,
        &serde_json::to_vec(state).map_err(|_| "folder_setup_invalid")?,
    )
}
pub fn setup_lock(config: &Path, provider: &str) -> Result<fs::File> {
    if !provider_ok(provider) {
        return Err("invalid_cloud_provider".into());
    }
    fs::create_dir_all(config).map_err(|_| "store_unavailable")?;
    storage::lock(&config.join(format!("folder-{provider}.lock")))
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderStatus {
    provider: String,
    path: Option<String>,
    space: Option<String>,
    complete: bool,
    handoff: &'static str,
}
fn status(provider: &str, state: Option<&FolderConfig>) -> FolderStatus {
    FolderStatus {
        provider: provider.into(),
        path: state.map(|s| s.path.clone()),
        space: state.and_then(|s| s.binding.as_ref().map(|b| b.space.clone())),
        complete: state.is_some_and(|s| s.complete),
        handoff: "local-folder",
    }
}

fn existing_dir(path: &Path) -> Result<()> {
    let m = fs::symlink_metadata(path).map_err(|_| "folder_unavailable")?;
    if !m.is_dir() || m.file_type().is_symlink() {
        return Err("folder_unavailable".into());
    }
    Ok(())
}
fn materialized_file(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "folder_object_missing"
        } else {
            "folder_unavailable"
        }
    })?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err("unsafe_store".into());
    }
    if meta.len() == 0 {
        return Err("folder_pending".into());
    }
    #[cfg(target_os = "macos")]
    {
        use std::os::macos::fs::MetadataExt;
        const SF_DATALESS: u32 = 0x4000_0000;
        if meta.st_flags() & SF_DATALESS != 0 {
            return Err("folder_pending".into());
        }
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_OFFLINE: u32 = 0x0000_1000;
        const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x0004_0000;
        const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
        if meta.file_attributes()
            & (FILE_ATTRIBUTE_OFFLINE
                | FILE_ATTRIBUTE_RECALL_ON_OPEN
                | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
            != 0
        {
            return Err("folder_pending".into());
        }
    }
    Ok(())
}
fn selected_root(state: &FolderConfig) -> Result<PathBuf> {
    existing_dir(state.path())?;
    fs::canonicalize(state.path()).map_err(|_| "folder_unavailable".into())
}
fn reject_source_overlap(config: &Path, selected: &Path) -> Result<()> {
    if let Some(mut settings) = crate::model::load(&config.join("settings.json"))? {
        settings.folder = selected.to_string_lossy().into_owned();
        crate::model::validate_overlap(&settings, &crate::detect(Some(&settings)))?;
    }
    Ok(())
}
fn managed(root: &Path) -> PathBuf {
    root.join(MANAGED)
}
fn space_root(root: &Path, binding: &Binding) -> Result<PathBuf> {
    binding.validate()?;
    existing_dir(root)?;
    let base = managed(root);
    existing_dir(&base)?;
    let folder = base.join(&binding.folder);
    existing_dir(&folder)?;
    Ok(folder)
}
fn kind_dir(kind: Kind) -> &'static str {
    match kind {
        Kind::Session => "sessions",
        Kind::Portable => "portable",
        Kind::Device => "devices",
    }
}
#[derive(Clone, Copy)]
pub enum Kind {
    Session,
    Portable,
    Device,
}
pub struct FolderObjects {
    root: PathBuf,
    binding: Binding,
}
impl FolderObjects {
    pub fn new(root: &Path, binding: &Binding) -> Result<Self> {
        space_root(root, binding)?;
        Ok(Self {
            root: root.into(),
            binding: binding.clone(),
        })
    }
    fn dir(&self, folder: &str, kind: Kind) -> Result<PathBuf> {
        if folder != self.binding.folder {
            return Err("space_mismatch".into());
        }
        let path = space_root(&self.root, &self.binding)?.join(kind_dir(kind));
        existing_dir(&path)?;
        Ok(path)
    }
    pub fn ids_kind(&self, folder: &str, kind: Kind) -> Result<Vec<String>> {
        let mut ids = Vec::new();
        for entry in fs::read_dir(self.dir(folder, kind)?).map_err(|_| "folder_unavailable")? {
            let entry = entry.map_err(|_| "folder_unavailable")?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if name.ends_with(".bas.icloud") || name.starts_with('.') && name.ends_with(".icloud") {
                return Err("folder_pending".into());
            }
            let Some(id) = name.strip_suffix(".bas") else {
                continue;
            };
            if !token(id) {
                return Err("invalid_envelope".into());
            }
            let file_type = entry.file_type().map_err(|_| "folder_unavailable")?;
            if !file_type.is_file() || file_type.is_symlink() {
                return Err("unsafe_store".into());
            }
            ids.push(id.to_owned());
            if ids.len() > MAX_OBJECTS + 1 {
                return Err("bundle_limit".into());
            }
        }
        ids.sort();
        Ok(ids)
    }
    pub fn put_kind(
        &self,
        folder: &str,
        id: &str,
        key: &SpaceKey,
        bundle: &Bundle,
        kind: Kind,
    ) -> Result<()> {
        if !token(id) || bundle.snapshot.space != self.binding.space {
            return Err("space_mismatch".into());
        }
        let path = self.dir(folder, kind)?.join(format!("{id}.bas"));
        if fs::symlink_metadata(&path).is_ok() {
            return if self.get_kind(folder, id, &self.binding.space, key, kind)? == *bundle {
                Ok(())
            } else {
                Err("folder_object_exists".into())
            };
        }
        let bytes = key.seal(bundle)?;
        match storage::immutable(&path, &bytes) {
            Ok(_) => Ok(()),
            Err(e) if e == "immutable_collision" => {
                if self.get_kind(folder, id, &self.binding.space, key, kind)? == *bundle {
                    Ok(())
                } else {
                    Err("folder_object_exists".into())
                }
            }
            Err(e) => Err(e),
        }
    }
    pub fn get_kind(
        &self,
        folder: &str,
        id: &str,
        space: &str,
        key: &SpaceKey,
        kind: Kind,
    ) -> Result<Bundle> {
        if !token(id) || space != self.binding.space {
            return Err("space_mismatch".into());
        }
        let path = self.dir(folder, kind)?.join(format!("{id}.bas"));
        materialized_file(&path)?;
        let bytes = storage::read(&path, MAX_ENCRYPTED as u64)?;
        key.open(space, &bytes)
    }
    pub fn usage(&self) -> Result<(u64, usize)> {
        let mut bytes = 0u64;
        let mut count = 0;
        for kind in [Kind::Session, Kind::Portable, Kind::Device] {
            for id in self.ids_kind(&self.binding.folder, kind)? {
                let p = self
                    .dir(&self.binding.folder, kind)?
                    .join(format!("{id}.bas"));
                bytes = bytes
                    .checked_add(
                        fs::symlink_metadata(p)
                            .map_err(|_| "folder_object_missing")?
                            .len(),
                    )
                    .ok_or("bundle_limit")?;
                count += 1;
            }
        }
        Ok((bytes, count))
    }
}
impl Objects for FolderObjects {
    fn ids(&self, folder: &str) -> Result<Vec<String>> {
        self.ids_kind(folder, Kind::Session)
    }
    fn allocate(&self) -> Result<String> {
        Ok(uuid::Uuid::new_v4().to_string())
    }
    fn put(&self, folder: &str, id: &str, key: &SpaceKey, bundle: &Bundle) -> Result<()> {
        self.put_kind(folder, id, key, bundle, Kind::Session)
    }
    fn get(&self, folder: &str, id: &str, space: &str, key: &SpaceKey) -> Result<Bundle> {
        self.get_kind(folder, id, space, key, Kind::Session)
    }
}
pub struct PortableObjects<'a> {
    pub folder: &'a FolderObjects,
    pub proof: &'a str,
}
impl Objects for PortableObjects<'_> {
    fn ids(&self, folder: &str) -> Result<Vec<String>> {
        self.folder.ids_kind(folder, Kind::Portable)
    }
    fn allocate(&self) -> Result<String> {
        self.folder.allocate()
    }
    fn put(&self, folder: &str, id: &str, key: &SpaceKey, bundle: &Bundle) -> Result<()> {
        self.folder
            .put_kind(folder, id, key, bundle, Kind::Portable)
    }
    fn get(&self, folder: &str, id: &str, space: &str, key: &SpaceKey) -> Result<Bundle> {
        self.folder.get_kind(
            folder,
            id,
            space,
            key,
            if id == self.proof {
                Kind::Session
            } else {
                Kind::Portable
            },
        )
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RecoveryKit {
    schema: u32,
    provider: String,
    binding: Binding,
    key: String,
}
impl Drop for RecoveryKit {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}
impl RecoveryKit {
    fn parse(bytes: &[u8], provider: &str) -> Result<Self> {
        if bytes.len() > 16384 {
            return Err("invalid_recovery_kit".into());
        }
        let kit: Self = serde_json::from_slice(bytes).map_err(|_| "invalid_recovery_kit")?;
        if kit.schema != 2 || kit.provider != provider {
            return Err("recovery_wrong_provider".into());
        }
        kit.binding.validate()?;
        SpaceKey::recover(&kit.key)?;
        Ok(kit)
    }
}
fn recovery_bytes(state: &FolderConfig) -> Result<Zeroizing<Vec<u8>>> {
    let binding = state.binding.clone().ok_or("wizard_step_required")?;
    let key = load_space_key(&NativeStore, &binding.space)?;
    Ok(Zeroizing::new(
        serde_json::to_vec_pretty(&RecoveryKit {
            schema: 2,
            provider: state.provider.clone(),
            binding,
            key: key.recovery_code().to_string(),
        })
        .map_err(|_| "invalid_recovery_kit")?,
    ))
}
fn choose_recovery_path() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_file_name("bastet-folder-recovery.json")
        .add_filter("JSON", &["json"])
        .save_file()
}
#[cfg(all(target_os = "macos", feature = "mac-app-store"))]
fn opened_file_path(file: &fs::File) -> Result<PathBuf> {
    use std::{
        ffi::CStr,
        os::{fd::AsRawFd, unix::ffi::OsStrExt},
    };
    unsafe extern "C" {
        fn fcntl(fd: std::os::raw::c_int, cmd: std::os::raw::c_int, ...) -> std::os::raw::c_int;
    }
    const F_GETPATH: i32 = 50;
    let mut bytes = [0i8; 1024];
    // SAFETY: F_GETPATH writes a NUL-terminated path into this 1024-byte
    // MAXPATHLEN buffer for a valid open file descriptor.
    if unsafe { fcntl(file.as_raw_fd(), F_GETPATH, bytes.as_mut_ptr()) } != 0 {
        return Err("recovery_export_failed".into());
    }
    // SAFETY: Successful F_GETPATH populates a NUL-terminated string.
    let path = unsafe { CStr::from_ptr(bytes.as_ptr()) };
    Ok(PathBuf::from(std::ffi::OsStr::from_bytes(path.to_bytes())))
}
fn export_kit_to(state: &FolderConfig, path: &Path) -> Result<()> {
    let bytes = recovery_bytes(state)?;
    let root = selected_root(state)?;
    #[cfg(not(all(target_os = "macos", feature = "mac-app-store")))]
    {
        let parent = fs::canonicalize(path.parent().ok_or("recovery_export_failed")?)
            .map_err(|_| "recovery_export_failed")?;
        if parent.starts_with(&root) {
            return Err("recovery_inside_sync_folder".into());
        }
    }
    #[cfg(feature = "mac-app-store")]
    {
        use std::io::Write;
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path).map_err(|_| "recovery_export_failed")?;
        #[cfg(target_os = "macos")]
        {
            let actual = match opened_file_path(&file) {
                Ok(actual) => actual,
                Err(error) => {
                    drop(file);
                    let _ = fs::remove_file(path); // only the empty file just created
                    return Err(error);
                }
            };
            if actual.starts_with(&root) {
                drop(file);
                let _ = fs::remove_file(&actual); // only the empty file just created
                return Err("recovery_inside_sync_folder".into());
            }
        }
        file.write_all(&bytes)
            .map_err(|_| "recovery_export_failed")?;
        file.sync_all().map_err(|_| "recovery_export_failed")?;
    }
    #[cfg(not(feature = "mac-app-store"))]
    storage::immutable(path, &bytes)?;
    let readback = Zeroizing::new(storage::read(path, 16384)?);
    if *readback != *bytes {
        return Err("recovery_export_failed".into());
    }
    RecoveryKit::parse(&readback, &state.provider)?;
    Ok(())
}
fn export_kit(state: &FolderConfig) -> Result<bool> {
    let Some(path) = choose_recovery_path() else {
        return Ok(false);
    };
    export_kit_to(state, &path)?;
    Ok(true)
}
pub(crate) fn create_dirs(root: &Path, binding: &Binding) -> Result<()> {
    storage::directory(&managed(root))?;
    let base = managed(root).join(&binding.folder);
    storage::directory(&base)?;
    for name in ["sessions", "portable", "devices"] {
        storage::directory(&base.join(name))?;
    }
    Ok(())
}
fn app_root(app: &tauri::AppHandle) -> Result<PathBuf> {
    app.path()
        .app_config_dir()
        .map_err(|_| "store_unavailable".into())
}
fn with_state(
    app: &tauri::AppHandle,
    provider: &str,
) -> Result<(PathBuf, FolderConfig, sandbox_access::Scope)> {
    let config = app_root(app)?;
    let state = load(&config, provider)?.ok_or("wizard_step_required")?;
    let scope = sandbox_access::access(&config, state.path())?;
    selected_root(&state)?;
    Ok((config, state, scope))
}
pub fn ready(
    config: &Path,
    provider: &str,
) -> Result<(FolderConfig, sandbox_access::Scope, FolderObjects)> {
    let state = load(config, provider)?.ok_or("wizard_step_required")?;
    if !state.complete {
        return Err("wizard_step_required".into());
    }
    let scope = sandbox_access::access(config, state.path())?;
    let root = selected_root(&state)?;
    let binding = state.binding.as_ref().ok_or("wizard_step_required")?;
    let remote = FolderObjects::new(&root, binding)?;
    let key = load_space_key(&NativeStore, &binding.space)?;
    if remote.get(&binding.folder, &binding.proof, &binding.space, &key)?
        != queue::proof_bundle(&binding.space)?
    {
        return Err("invalid_space_proof".into());
    }
    Ok((state, scope, remote))
}
pub fn probe_ready(config: &Path, provider: &str) -> Result<bool> {
    let Some(state) = load(config, provider)? else {
        return Ok(false);
    };
    if !state.complete {
        return Ok(false);
    }
    let _scope = sandbox_access::access(config, state.path())?;
    let root = selected_root(&state)?;
    let binding = state.binding.as_ref().ok_or("folder_setup_invalid")?;
    let remote = FolderObjects::new(&root, binding)?;
    let path = remote
        .dir(&binding.folder, Kind::Session)?
        .join(format!("{}.bas", binding.proof));
    materialized_file(&path)?;
    Ok(true)
}
#[tauri::command]
pub async fn folder_status(app: tauri::AppHandle, provider: String) -> Result<FolderStatus> {
    let config = app_root(&app)?;
    let state = load(&config, &provider)?;
    if let Some(s) = &state {
        let _scope = sandbox_access::access(&config, s.path())?;
        selected_root(s)?;
        if s.complete {
            probe_ready(&config, &provider)?;
        }
    }
    Ok(status(&provider, state.as_ref()))
}
#[tauri::command]
pub async fn folder_pick(
    app: tauri::AppHandle,
    worker: tauri::State<'_, crate::worker::Worker>,
    provider: String,
) -> Result<Option<FolderStatus>> {
    config_file(&app_root(&app)?, &provider)?;
    let worker = worker.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let Some(picked) = rfd::FileDialog::new().pick_folder() else {
            return Ok(None);
        };
        let config = app_root(&app)?;
        let _lock = setup_lock(&config, &provider)?;
        if worker.active() {
            return Err("sync_running".into());
        }
        let path = sandbox_access::grant_selected(&config, &picked)?;
        let _scope = sandbox_access::access(&config, &path)?;
        existing_dir(&path)?;
        let prior = load(&config, &provider)?;
        let state = prior
            .filter(|p| p.path() == path)
            .unwrap_or_else(|| FolderConfig::new(&provider, &path));
        save(&config, &state)?;
        Ok(Some(status(&provider, Some(&state))))
    })
    .await
    .map_err(|_| "folder_unavailable".to_string())?
}
#[tauri::command]
pub async fn folder_prepare(
    app: tauri::AppHandle,
    worker: tauri::State<'_, crate::worker::Worker>,
    provider: String,
) -> Result<Option<FolderStatus>> {
    let worker = worker.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let config = app_root(&app)?;
        let _lock = setup_lock(&config, &provider)?;
        if worker.active() {
            return Err("sync_running".into());
        }
        let (config, mut state, _scope) = with_state(&app, &provider)?;
        if state.complete {
            return Ok(Some(status(&provider, Some(&state))));
        }
        let root = selected_root(&state)?;
        reject_source_overlap(&config, &root)?;
        let Some(recovery_path) = choose_recovery_path() else {
            return Ok(None);
        };
        if state.binding.is_none() {
            let base = managed(&root);
            if base.exists() {
                existing_dir(&base)?;
                if fs::read_dir(&base)
                    .map_err(|_| "folder_unavailable")?
                    .next()
                    .is_some()
                {
                    return Err("folder_has_sync_objects".into());
                }
            }
            let binding = Binding {
                folder: uuid::Uuid::new_v4().to_string(),
                space: uuid::Uuid::new_v4().to_string(),
                proof: uuid::Uuid::new_v4().to_string(),
            };
            let key = SpaceKey::generate()?;
            save_space_key(&NativeStore, &binding.space, &key)?;
            state.binding = Some(binding);
            save(&config, &state)?;
        }
        export_kit_to(&state, &recovery_path)?;
        let binding = state.binding.as_ref().ok_or("wizard_step_required")?;
        create_dirs(&root, binding)?;
        let remote = FolderObjects::new(&root, binding)?;
        let key = load_space_key(&NativeStore, &binding.space)?;
        let proof = queue::proof_bundle(&binding.space)?;
        let ids = remote.ids(&binding.folder)?;
        if ids.iter().any(|id| id != &binding.proof) {
            return Err("folder_has_sync_objects".into());
        }
        remote.put(&binding.folder, &binding.proof, &key, &proof)?;
        if remote.get(&binding.folder, &binding.proof, &binding.space, &key)? != proof {
            return Err("invalid_space_proof".into());
        }
        state.complete = true;
        save(&config, &state)?;
        Ok(Some(status(&provider, Some(&state))))
    })
    .await
    .map_err(|_| "folder_unavailable".to_string())?
}
#[tauri::command]
pub async fn folder_join(
    app: tauri::AppHandle,
    worker: tauri::State<'_, crate::worker::Worker>,
    provider: String,
) -> Result<Option<FolderStatus>> {
    let worker = worker.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let config = app_root(&app)?;
        let _lock = setup_lock(&config, &provider)?;
        if worker.active() {
            return Err("sync_running".into());
        }
        let (config, mut state, _scope) = with_state(&app, &provider)?;
        let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .pick_file()
        else {
            return Ok(None);
        };
        let bytes = Zeroizing::new(storage::read(&path, 16384)?);
        let kit = RecoveryKit::parse(&bytes, &provider)?;
        if state.complete && state.binding.as_ref().is_some_and(|b| b != &kit.binding) {
            return Err("wizard_restart_required".into());
        }
        let key = SpaceKey::recover(&kit.key)?;
        let root = selected_root(&state)?;
        reject_source_overlap(&config, &root)?;
        let remote = FolderObjects::new(&root, &kit.binding)?;
        if remote.get(
            &kit.binding.folder,
            &kit.binding.proof,
            &kit.binding.space,
            &key,
        )? != queue::proof_bundle(&kit.binding.space)?
        {
            return Err("invalid_space_proof".into());
        }
        save_space_key(&NativeStore, &kit.binding.space, &key)?;
        state.binding = Some(kit.binding.clone());
        state.complete = true;
        save(&config, &state)?;
        Ok(Some(status(&provider, Some(&state))))
    })
    .await
    .map_err(|_| "folder_unavailable".to_string())?
}
#[tauri::command]
pub async fn folder_export_recovery(app: tauri::AppHandle, provider: String) -> Result<bool> {
    tauri::async_runtime::spawn_blocking(move || {
        let (_, state, _scope) = with_state(&app, &provider)?;
        if !state.complete {
            return Err("wizard_step_required".into());
        }
        export_kit(&state)
    })
    .await
    .map_err(|_| "folder_unavailable".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(target_os = "macos", feature = "mac-app-store"))]
    #[test]
    fn selected_file_descriptor_resolves_symlink_parent_before_secret_write() {
        let dir = tempfile::tempdir().unwrap();
        let actual = dir.path().join("synced");
        fs::create_dir(&actual).unwrap();
        let alias = dir.path().join("alias");
        std::os::unix::fs::symlink(&actual, &alias).unwrap();
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(alias.join("recovery.json"))
            .unwrap();
        let resolved = opened_file_path(&file).unwrap();
        assert!(resolved.starts_with(fs::canonicalize(&actual).unwrap()));
        assert_eq!(file.metadata().unwrap().len(), 0);
    }
    use crate::sync::{
        bundle::{Entry, Snapshot, Stream},
        Direction, Replica,
    };
    use std::collections::BTreeMap;

    fn fixture() -> (tempfile::TempDir, Binding, SpaceKey, FolderObjects) {
        let dir = tempfile::tempdir().unwrap();
        let binding = Binding {
            folder: "fixture-folder".into(),
            space: "fixture-space".into(),
            proof: "fixture-proof".into(),
        };
        create_dirs(dir.path(), &binding).unwrap();
        let remote = FolderObjects::new(dir.path(), &binding).unwrap();
        (dir, binding, SpaceKey::generate().unwrap(), remote)
    }
    fn bundle(space: &str, conversation: &str, parents: Vec<String>) -> Bundle {
        Bundle::new(Snapshot {
            schema: 1,
            space: space.into(),
            device: "fixture-device".into(),
            stream: Stream {
                agent: "codex".into(),
                profile: "fixture".into(),
                conversation: conversation.into(),
            },
            parents,
            files: BTreeMap::from([(
                "session.txt".into(),
                Entry::new(format!("fixture-{conversation}")),
            )]),
        })
        .unwrap()
    }
    #[test]
    fn encrypted_immutable_retry_and_concurrent_branches() {
        let (_dir, binding, key, remote) = fixture();
        let parent = bundle(&binding.space, "parent", vec![]);
        let a = bundle(&binding.space, "branch-a", vec![parent.id.clone()]);
        let b = bundle(&binding.space, "branch-b", vec![parent.id.clone()]);
        let id = "id-parent";
        remote.put(&binding.folder, id, &key, &parent).unwrap();
        // Sealing is randomized, but a retry of the same allocated id is idempotent.
        remote.put(&binding.folder, id, &key, &parent).unwrap();
        remote.put(&binding.folder, "id-a", &key, &a).unwrap();
        remote.put(&binding.folder, "id-b", &key, &b).unwrap();
        assert_eq!(remote.ids(&binding.folder).unwrap(), ["id-a", "id-b", id]);
        assert_eq!(
            remote
                .get(&binding.folder, "id-a", &binding.space, &key)
                .unwrap(),
            a
        );
        assert_eq!(
            remote
                .get(&binding.folder, "id-b", &binding.space, &key)
                .unwrap(),
            b
        );
        assert_eq!(
            remote.put(&binding.folder, id, &key, &a),
            Err("folder_object_exists".into())
        );
        let path = remote
            .dir(&binding.folder, Kind::Session)
            .unwrap()
            .join("id-a.bas");
        assert!(!String::from_utf8_lossy(&fs::read(path).unwrap()).contains("fixture-branch-a"));
    }
    #[test]
    fn missing_partial_wrong_key_and_wrong_space_fail_closed() {
        let (_dir, binding, key, remote) = fixture();
        let proof = queue::proof_bundle(&binding.space).unwrap();
        assert_eq!(
            remote.get(&binding.folder, &binding.proof, &binding.space, &key),
            Err("folder_object_missing".into())
        );
        remote
            .put(&binding.folder, &binding.proof, &key, &proof)
            .unwrap();
        assert_eq!(
            remote.get(
                &binding.folder,
                &binding.proof,
                &binding.space,
                &SpaceKey::generate().unwrap()
            ),
            Err("decrypt_failed".into())
        );
        assert_eq!(
            remote.get(&binding.folder, &binding.proof, "other-space", &key),
            Err("space_mismatch".into())
        );
        let path = remote
            .dir(&binding.folder, Kind::Session)
            .unwrap()
            .join("partial.bas");
        fs::write(&path, b"partial").unwrap();
        assert!(remote
            .get(&binding.folder, "partial", &binding.space, &key)
            .is_err());
        assert!(remote
            .ids(&binding.folder)
            .unwrap()
            .contains(&"partial".into()));
        let session_dir = remote.dir(&binding.folder, Kind::Session).unwrap();
        fs::write(session_dir.join("empty.bas"), b"").unwrap();
        assert_eq!(
            remote.get(&binding.folder, "empty", &binding.space, &key),
            Err("folder_pending".into())
        );
        fs::write(session_dir.join(".cloud.bas.icloud"), b"").unwrap();
        assert_eq!(remote.ids(&binding.folder), Err("folder_pending".into()));
    }
    #[test]
    fn missing_namespace_or_symlinked_object_is_never_recreated() {
        let (dir, binding, key, remote) = fixture();
        fs::remove_dir_all(dir.path().join(MANAGED).join(&binding.folder)).unwrap();
        assert_eq!(
            remote.ids(&binding.folder),
            Err("folder_unavailable".into())
        );
        assert!(!dir.path().join(MANAGED).join(&binding.folder).exists());
        create_dirs(dir.path(), &binding).unwrap();
        #[cfg(unix)]
        {
            let target = dir.path().join("outside");
            fs::write(&target, b"outside").unwrap();
            std::os::unix::fs::symlink(
                target,
                remote
                    .dir(&binding.folder, Kind::Session)
                    .unwrap()
                    .join("alias.bas"),
            )
            .unwrap();
            assert_eq!(remote.ids(&binding.folder), Err("unsafe_store".into()));
            assert!(remote
                .put(
                    &binding.folder,
                    "alias",
                    &key,
                    &queue::proof_bundle(&binding.space).unwrap()
                )
                .is_err());
        }
    }
    #[test]
    fn provider_and_key_kits_cannot_cross_transport() {
        let binding = Binding {
            folder: "folder".into(),
            space: "space".into(),
            proof: "proof".into(),
        };
        let key = SpaceKey::generate().unwrap();
        let bytes = serde_json::to_vec(&RecoveryKit {
            schema: 2,
            provider: "icloud-drive".into(),
            binding,
            key: key.recovery_code().to_string(),
        })
        .unwrap();
        assert!(RecoveryKit::parse(&bytes, "icloud-drive").is_ok());
        assert_eq!(
            RecoveryKit::parse(&bytes, "onedrive-folder").err(),
            Some("recovery_wrong_provider".into())
        );
        assert!(crate::cloud::wizard::RecoveryKit::parse(&bytes).is_err());
    }
    #[test]
    fn fixture_exchange_roundtrip_requires_proof_and_keeps_branches() {
        let (dir, binding, key, remote) = fixture();
        let a = Replica::open(&dir.path().join("replica-a"), &binding.space).unwrap();
        let b = Replica::open(&dir.path().join("replica-b"), &binding.space).unwrap();
        a.export_from(
            Stream {
                agent: "codex".into(),
                profile: "fixture".into(),
                conversation: "c".into(),
            },
            BTreeMap::from([("sample.txt".into(), "a".into())]),
            None,
        )
        .unwrap();
        let qa = dir.path().join("queue-a");
        assert_eq!(
            queue::exchange(&qa, &a, &binding, &key, &remote, Direction::Upload).unwrap_err(),
            "folder_object_missing"
        );
        remote
            .put(
                &binding.folder,
                &binding.proof,
                &key,
                &queue::proof_bundle(&binding.space).unwrap(),
            )
            .unwrap();
        assert_eq!(
            queue::exchange(&qa, &a, &binding, &key, &remote, Direction::Upload)
                .unwrap()
                .published,
            1
        );
        assert_eq!(
            queue::exchange(
                &dir.path().join("queue-b"),
                &b,
                &binding,
                &key,
                &remote,
                Direction::Download
            )
            .unwrap()
            .received,
            1
        );
        assert_eq!(b.transport_bundles().unwrap().len(), 1);
        assert_eq!(
            queue::exchange(&qa, &a, &binding, &key, &remote, Direction::Both)
                .unwrap()
                .published,
            0
        );
    }
}
