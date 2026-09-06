//! Opt-in portable preferences and skills. Received files stay inert until explicit recovery.
use crate::{
    cloud::{
        crypto::SpaceKey,
        drive::{Drive, ObjectKind},
        queue::{self, Binding, Objects},
    },
    model::Settings,
    review,
    sync::{
        bundle::{self, Bundle, Result, Stream},
        storage, Direction, Replica,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use tauri::Manager;
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Options {
    pub settings: bool,
    pub skills: bool,
    #[serde(default)]
    pub excluded_paths: BTreeMap<String, Vec<String>>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub schema: u32,
    pub agent: String,
    pub files: BTreeMap<String, String>,
    pub excluded: BTreeMap<String, String>,
}
const MAX_FILE: u64 = 1024 * 1024;
const MAX_TOTAL: usize = 16 * 1024 * 1024;
const KEYS: &[&str] = &[
    "model",
    "model_reasoning_effort",
    "model_reasoning_summary",
    "model_verbosity",
    "personality",
    "theme",
    "language",
    "locale",
    "outputStyle",
    "thinkingLevel",
    "defaultProvider",
    "defaultModel",
];
fn suspected_secret(text: &str) -> bool {
    [
        "-----BEGIN PRIVATE KEY",
        "-----BEGIN RSA PRIVATE KEY",
        "sk-proj-",
        "sk-ant-",
        "ghp_",
        "github_pat_",
        "xoxb-",
        "AIza",
        "Bearer ",
    ]
    .iter()
    .any(|p| text.contains(p))
}
fn safe_value(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Bool(_) | serde_json::Value::Number(_) => true,
        serde_json::Value::String(s) => {
            s.len() <= 160
                && !s.contains(['/', '\\', ':', '$', '`', '\n', '\r'])
                && !suspected_secret(s)
        }
        _ => false,
    }
}
fn allowed(path: &str) -> bool {
    if !crate::native_sessions::safe_relative(path) {
        return false;
    }
    if ["config.toml", "settings.json"].contains(&path) {
        return true;
    }
    let parts: Vec<_> = path.split('/').collect();
    parts.len() >= 2
        && ["skills", "shared-skills", "commands", "agents"].contains(&parts[0])
        && !parts.iter().any(|p| {
            let p = p.to_lowercase();
            p.starts_with('.')
                || ["auth", "credential", "token", "secret"]
                    .iter()
                    .any(|s| p.contains(s))
        })
        && Path::new(path)
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|s| {
                [
                    "md", "txt", "json", "yaml", "yml", "toml", "py", "js", "ts", "sh",
                ]
                .contains(&s)
            })
}
fn sanitized(text: &str, toml: bool) -> Result<String> {
    let value: serde_json::Value = if toml {
        serde_json::to_value(text.parse::<toml::Value>().map_err(|_| "portable_format")?)
            .map_err(|_| "portable_format")?
    } else {
        serde_json::from_str(text).map_err(|_| "portable_format")?
    };
    let map = value.as_object().ok_or("portable_format")?;
    let filtered = serde_json::Value::Object(
        map.iter()
            .filter(|(k, v)| KEYS.contains(&k.as_str()) && safe_value(v))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    );
    if toml {
        toml::to_string_pretty(&filtered).map_err(|_| "portable_format".into())
    } else {
        serde_json::to_string_pretty(&filtered).map_err(|_| "portable_format".into())
    }
}
impl Options {
    pub fn validate(&self) -> Result<()> {
        if self.excluded_paths.len() > 8
            || self.excluded_paths.iter().any(|(a, paths)| {
                !crate::model::AGENTS.contains(&a.as_str())
                    || paths.len() > 256
                    || paths.iter().any(|p| !allowed(p))
            })
        {
            return Err("invalid_settings".into());
        }
        Ok(())
    }
}
impl Package {
    fn validate(&self) -> Result<()> {
        if self.schema != 1
            || !crate::model::AGENTS.contains(&self.agent.as_str())
            || self.files.len() > 256
            || self.files.values().map(String::len).sum::<usize>() > MAX_TOTAL
            || self.excluded.len() > 256
        {
            return Err("portable_limit".into());
        }
        for (path, text) in &self.files {
            if !allowed(path) || text.len() > MAX_FILE as usize || suspected_secret(text) {
                return Err("portable_unsafe".into());
            }
            if ["config.toml", "settings.json"].contains(&path.as_str())
                && sanitized(text, path.ends_with("toml"))? != *text
            {
                return Err("portable_unsafe".into());
            }
        }
        Ok(())
    }
}
fn add(root: &Path, relative: &str, package: &mut Package) -> Result<()> {
    if package.files.len() + package.excluded.len() >= 256 {
        return Err("portable_limit".into());
    }
    if !allowed(relative) {
        package
            .excluded
            .insert(relative.into(), "unsupported_or_sensitive_path".into());
        return Ok(());
    }
    let Some(bytes) = review::local_file(root, relative, MAX_FILE)? else {
        return Ok(());
    };
    let Ok(text) = String::from_utf8(bytes) else {
        package.excluded.insert(relative.into(), "non_text".into());
        return Ok(());
    };
    let text = if ["config.toml", "settings.json"].contains(&relative) {
        sanitized(&text, relative.ends_with("toml"))?
    } else {
        text
    };
    if suspected_secret(&text) {
        package
            .excluded
            .insert(relative.into(), "suspected_credential".into());
    } else {
        package.files.insert(relative.into(), text);
    }
    Ok(())
}
fn walk(root: &Path, path: &Path, depth: usize, package: &mut Package) -> Result<()> {
    if depth > 8 || package.files.len() + package.excluded.len() >= 256 {
        return Err("portable_limit".into());
    }
    if !path.exists() {
        return Ok(());
    }
    let metadata = std::fs::symlink_metadata(path).map_err(|_| "store_unavailable")?;
    let relative = path
        .strip_prefix(root)
        .map_err(|_| "unsafe_store")?
        .to_string_lossy()
        .replace('\\', "/");
    if metadata.file_type().is_symlink() {
        package.excluded.insert(relative, "symlink".into());
        return Ok(());
    }
    if metadata.is_file() {
        return add(root, &relative, package);
    }
    if metadata.is_dir() {
        if path.file_name().is_some_and(|v| {
            let n = v.to_string_lossy().to_lowercase();
            n.starts_with('.') || ["node_modules", "__pycache__", "target"].contains(&n.as_str())
        }) {
            return Ok(());
        }
        for entry in std::fs::read_dir(path).map_err(|_| "store_unavailable")? {
            let path = entry.map_err(|_| "store_unavailable")?.path();
            walk(root, &path, depth + 1, package)?;
        }
    }
    Ok(())
}
pub fn capture(agent: &str, root: &Path, options: &Options) -> Result<Package> {
    options.validate()?;
    let mut package = Package {
        schema: 1,
        agent: agent.into(),
        ..Default::default()
    };
    if agent == "agent-memory-os" {
        return Ok(package);
    }
    if options.settings {
        for file in ["config.toml", "settings.json"] {
            add(root, file, &mut package)?;
        }
    }
    if options.skills {
        for dir in ["skills", "commands", "agents"] {
            walk(root, &root.join(dir), 0, &mut package)?;
        }
    }
    // Shared Codex skills are opted into only for the standard profile, never arbitrary test/custom roots.
    if options.skills && agent == "codex" {
        if let Some(home) = dirs::home_dir().filter(|h| h.join(".codex") == root) {
            capture_shared(&home.join(".agents"), &mut package)?;
        }
    }
    if let Some(paths) = options.excluded_paths.get(agent) {
        for path in paths {
            if package.files.remove(path).is_some() {
                package.excluded.insert(path.clone(), "not_selected".into());
            }
        }
    }
    package.validate()?;
    Ok(package)
}
fn capture_shared(shared: &Path, package: &mut Package) -> Result<()> {
    let mut extra = Package {
        schema: 1,
        agent: package.agent.clone(),
        ..Default::default()
    };
    walk(shared, &shared.join("skills"), 0, &mut extra)?;
    for (path, text) in extra.files {
        package
            .files
            .insert(path.replacen("skills/", "shared-skills/", 1), text);
    }
    for (path, reason) in extra.excluded {
        package
            .excluded
            .insert(path.replacen("skills/", "shared-skills/", 1), reason);
    }
    package.validate()
}
pub struct Transport<'a> {
    pub drive: &'a Drive,
    pub proof: &'a str,
}
impl Objects for Transport<'_> {
    fn revisions(&self, folder: &str) -> Result<Vec<(String, Option<String>)>> {
        Ok(self
            .drive
            .list_kind(folder, ObjectKind::Portable)?
            .into_iter()
            .map(|f| (f.id, f.version))
            .collect())
    }
    fn ids(&self, folder: &str) -> Result<Vec<String>> {
        Ok(self
            .drive
            .list_kind(folder, ObjectKind::Portable)?
            .into_iter()
            .map(|f| f.id)
            .collect())
    }
    fn allocate(&self) -> Result<String> {
        self.drive.allocate_id()
    }
    fn put(&self, f: &str, id: &str, k: &SpaceKey, b: &Bundle) -> Result<()> {
        self.drive
            .upload_kind(f, id, k, b, ObjectKind::Portable)
            .map(|_| ())
    }
    fn get(&self, f: &str, id: &str, space: &str, k: &SpaceKey) -> Result<Bundle> {
        self.drive.download_kind(
            f,
            id,
            space,
            k,
            if id == self.proof {
                ObjectKind::Session
            } else {
                ObjectKind::Portable
            },
        )
    }
}
#[derive(Default, Serialize, Deserialize)]
struct Journal {
    node: String,
    base: Option<String>,
}
#[allow(clippy::too_many_arguments)]
pub fn cycle(
    root: &Path,
    source: &Path,
    agent: &str,
    options: &Options,
    binding: &Binding,
    key: &SpaceKey,
    remote: &impl Objects,
    direction: Direction,
    stop: impl Fn() -> bool,
) -> Result<queue::Exchange> {
    storage::directory(root)?;
    let path = root.join("portable-journal.json");
    let mut journal: Journal = if path.exists() {
        serde_json::from_slice(&storage::read(&path, 65536)?).map_err(|_| "local_store_damaged")?
    } else {
        Journal {
            node: uuid::Uuid::new_v4().to_string(),
            base: None,
        }
    };
    let replica = Replica::open(&root.join("replica"), &binding.space)?;
    if !matches!(direction, Direction::Download) {
        crate::progress::stage("scan", None);
        let package = capture(agent, source, options)?;
        if !package.files.is_empty() {
            let text = serde_json::to_string(&package).map_err(|_| "portable_format")?;
            journal.base = Some(replica.export_from(
                Stream {
                    agent: agent.into(),
                    profile: journal.node.clone(),
                    conversation: "portable-preferences".into(),
                },
                BTreeMap::from([("portable.json".into(), text)]),
                journal.base.as_deref(),
            )?);
            storage::replace(
                &path,
                &serde_json::to_vec(&journal).map_err(|_| "local_store_damaged")?,
            )?;
        }
    }
    if stop() {
        return Err("sync_paused".into());
    }
    queue::exchange_filtered(
        &root.join("exchange"),
        &replica,
        binding,
        key,
        remote,
        direction,
        Some(agent),
    )
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Received {
    pub agent: String,
    pub id: String,
    pub files: usize,
    pub saved_at: Option<u64>,
}
fn context(app: &tauri::AppHandle) -> Result<(PathBuf, Settings, String)> {
    let root = app
        .path()
        .app_config_dir()
        .map_err(|_| "store_unavailable")?;
    let settings = crate::model::load(&root.join("settings.json"))?.ok_or("invalid_settings")?;
    let tx = crate::cloud::wizard::Transaction::open(&root)?;
    Ok((
        root,
        settings,
        tx.state.binding.ok_or("wizard_step_required")?.space,
    ))
}
fn load(root: &Path, space: &str, agent: &str, id: &str) -> Result<Package> {
    if !crate::model::AGENTS.contains(&agent) || !bundle::is_hash(id) {
        return Err("portable_unsafe".into());
    }
    let replica = Replica::open(
        &root
            .join(format!("portable-{agent}-{space}"))
            .join("replica"),
        space,
    )?;
    let all = replica.transport_bundles()?;
    let b = all.get(id).ok_or("portable_missing")?;
    decode_package(b, agent)
}
fn decode_package(b: &Bundle, agent: &str) -> Result<Package> {
    let package: Package = serde_json::from_str(
        &b.snapshot
            .files
            .get("portable.json")
            .ok_or("portable_format")?
            .content,
    )
    .map_err(|_| "portable_format")?;
    package.validate()?;
    if package.agent != agent || b.snapshot.stream.agent != agent {
        return Err("portable_unsafe".into());
    }
    Ok(package)
}
#[tauri::command]
pub async fn portable_preview(settings: Settings) -> Result<Vec<Package>> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::model::validate(&settings)?;
        let agents = crate::detect(Some(&settings));
        let (tasks, _) = crate::worker::parallel::plan(&settings.selected_agents, &agents);
        let mut options = settings.portable.clone();
        options.excluded_paths.clear(); // Preview includes candidates so each can be selected again.
        tasks
            .into_iter()
            .filter_map(|t| t.path.map(|p| (t.canonical, p)))
            .map(|(agent, path)| capture(&agent, &path, &options))
            .collect()
    })
    .await
    .map_err(|_| "store_unavailable".to_string())?
}
#[tauri::command]
pub async fn portable_list(app: tauri::AppHandle) -> Result<Vec<Received>> {
    tauri::async_runtime::spawn_blocking(move || {
        let (root, _, space) = context(&app)?;
        let mut entries = Vec::new();
        for agent in crate::model::AGENTS {
            let path = root.join(format!("portable-{agent}-{space}"));
            if !path.is_dir() {
                continue;
            }
            let replica = Replica::open(&path.join("replica"), &space)?;
            let all = replica.transport_bundles()?;
            let parents: std::collections::BTreeSet<_> = all
                .values()
                .flat_map(|b| b.snapshot.parents.iter().cloned())
                .collect();
            for b in all.values().filter(|b| !parents.contains(&b.id)) {
                let package = decode_package(b, agent)?;
                entries.push(Received {
                    agent: agent.into(),
                    id: b.id.clone(),
                    files: package.files.len(),
                    saved_at: std::fs::metadata(
                        path.join("replica/objects").join(format!("{}.json", b.id)),
                    )
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|v| v.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|v| v.as_secs()),
                });
            }
        }
        Ok(entries)
    })
    .await
    .map_err(|_| "store_unavailable".to_string())?
}
#[tauri::command]
pub async fn portable_compare(
    app: tauri::AppHandle,
    agent: String,
    id: String,
) -> Result<review::Comparison> {
    tauri::async_runtime::spawn_blocking(move || {
        let (root, settings, space) = context(&app)?;
        let package = load(&root, &space, &agent, &id)?;
        let agents = crate::detect(Some(&settings));
        let source = agents
            .iter()
            .find(|a| a.id == agent)
            .ok_or("source_missing")?;
        let local = capture(
            &agent,
            Path::new(&source.path),
            &Options {
                settings: true,
                skills: true,
                ..Default::default()
            },
        )?;
        let files = package
            .files
            .into_iter()
            .map(|(path, text)| {
                review::diff(
                    path.clone(),
                    local.files.get(&path).map(|v| v.as_bytes().to_vec()),
                    text.as_bytes(),
                )
            })
            .collect();
        review::comparison(&root, &format!("portable:{agent}:{id}"), files)
    })
    .await
    .map_err(|_| "store_unavailable".to_string())?
}
#[tauri::command]
pub async fn portable_restore(
    app: tauri::AppHandle,
    worker: tauri::State<'_, crate::worker::Worker>,
    agent: String,
    id: String,
) -> Result<Option<String>> {
    if worker.active() {
        return Err("sync_running".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let (root, _, space) = context(&app)?;
        let package = load(&root, &space, &agent, &id)?;
        let Some(folder) = rfd::FileDialog::new().pick_folder() else {
            return Ok(None);
        };
        let stage = tempfile::tempdir_in(&folder).map_err(|_| "store_unavailable")?;
        for (path, text) in package.files {
            let destination = stage.path().join(path);
            std::fs::create_dir_all(destination.parent().ok_or("unsafe_store")?)
                .map_err(|_| "store_unavailable")?;
            storage::immutable(&destination, text.as_bytes())?;
        }
        let target = folder.join(format!("Bastet-portable-{agent}-{}", uuid::Uuid::new_v4()));
        std::fs::rename(stage.path(), &target).map_err(|_| "store_unavailable")?;
        Ok(Some(target.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|_| "store_unavailable".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    fn write(root: &Path, name: &str, text: &str) {
        let p = root.join(name);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    #[test]
    fn explicit_preview_excludes_credentials_hooks_paths_and_selected_files() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "config.toml", "model = 'gpt-test'\nmodel_reasoning_effort = 'high'\napi_key = 'PRIVATE'\n[hooks]\ncommand = 'unsafe'\n");
        write(root.path(), "auth.json", "PRIVATE");
        write(
            root.path(),
            "skills/cat/SKILL.md",
            "# Cat\nOnly instructions.",
        );
        write(root.path(), "skills/cat/secret.txt", "PRIVATE");
        write(
            root.path(),
            "skills/cat/code.ts",
            "const credential = 'sk-proj-synthetic';",
        );
        write(
            root.path(),
            "skills/cat/node_modules/ignored.md",
            "unneeded",
        );
        assert!(capture("codex", root.path(), &Options::default())
            .unwrap()
            .files
            .is_empty());
        let mut options = Options {
            settings: true,
            skills: true,
            ..Default::default()
        };
        let p = capture("codex", root.path(), &options).unwrap();
        assert_eq!(p.files.len(), 2);
        assert!(!p.files["config.toml"].contains("PRIVATE"));
        assert!(!p.files["config.toml"].contains("hooks"));
        assert_eq!(p.excluded["skills/cat/code.ts"], "suspected_credential");
        options
            .excluded_paths
            .insert("codex".into(), vec!["skills/cat/SKILL.md".into()]);
        let p = capture("codex", root.path(), &options).unwrap();
        assert_eq!(p.files.len(), 1);
        options
            .excluded_paths
            .insert("codex".into(), vec!["../escape".into()]);
        assert!(options.validate().is_err());
    }
    #[test]
    fn shared_skills_remain_distinct_from_profile_skills() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "skills/cat/SKILL.md", "shared");
        let mut package = Package {
            schema: 1,
            agent: "codex".into(),
            files: BTreeMap::from([("skills/cat/SKILL.md".into(), "profile".into())]),
            ..Default::default()
        };
        capture_shared(root.path(), &mut package).unwrap();
        assert_eq!(package.files["skills/cat/SKILL.md"], "profile");
        assert_eq!(package.files["shared-skills/cat/SKILL.md"], "shared");
    }
    #[test]
    fn received_packages_reject_traversal_secrets_and_unsanitized_config() {
        for (path, text) in [
            ("../escape.md", "text"),
            ("skills/x/auth.md", "text"),
            ("settings.json", "{\"apiKey\":\"private\"}"),
            ("skills/x/SKILL.md", "Bearer synthetic"),
        ] {
            let p = Package {
                schema: 1,
                agent: "codex".into(),
                files: BTreeMap::from([(path.into(), text.into())]),
                ..Default::default()
            };
            assert!(p.validate().is_err(), "{path}");
        }
    }
    #[cfg(unix)]
    #[test]
    fn skills_do_not_follow_links_outside_source() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("skills")).unwrap();
        std::os::unix::fs::symlink("/tmp", root.path().join("skills/link")).unwrap();
        let p = capture(
            "codex",
            root.path(),
            &Options {
                skills: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(p.files.is_empty());
        assert_eq!(p.excluded["skills/link"], "symlink");
    }
    struct Remote {
        wire: RefCell<BTreeMap<String, Vec<u8>>>,
        next: Cell<usize>,
    }
    impl Objects for Remote {
        fn ids(&self, _: &str) -> Result<Vec<String>> {
            Ok(self.wire.borrow().keys().cloned().collect())
        }
        fn allocate(&self) -> Result<String> {
            self.next.set(self.next.get() + 1);
            Ok(format!("object-{}", self.next.get()))
        }
        fn put(&self, _: &str, id: &str, key: &SpaceKey, b: &Bundle) -> Result<()> {
            self.wire.borrow_mut().insert(id.into(), key.seal(b)?);
            Ok(())
        }
        fn get(&self, _: &str, id: &str, space: &str, key: &SpaceKey) -> Result<Bundle> {
            key.open(space, self.wire.borrow().get(id).ok_or("missing")?)
        }
    }
    #[test]
    fn encrypted_preferences_exchange_is_idempotent_and_never_installs_in_live_profile() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let receiving = root.path().join("receiving");
        write(
            &source,
            "settings.json",
            "{\"theme\":\"dark\",\"apiKey\":\"NEVER-SEND\"}",
        );
        write(&receiving, "settings.json", "{\"theme\":\"light\"}");
        let key = SpaceKey::generate().unwrap();
        let binding = Binding {
            folder: "folder".into(),
            space: "space".into(),
            proof: "proof".into(),
        };
        let remote = Remote {
            wire: RefCell::new(BTreeMap::from([(
                "proof".into(),
                key.seal(&queue::proof_bundle("space").unwrap()).unwrap(),
            )])),
            next: Cell::new(0),
        };
        let options = Options {
            settings: true,
            ..Default::default()
        };
        assert_eq!(
            cycle(
                &root.path().join("a"),
                &source,
                "codex",
                &options,
                &binding,
                &key,
                &remote,
                Direction::Upload,
                || false
            )
            .unwrap()
            .published,
            1
        );
        assert_eq!(
            cycle(
                &root.path().join("a"),
                &source,
                "codex",
                &options,
                &binding,
                &key,
                &remote,
                Direction::Upload,
                || false
            )
            .unwrap()
            .published,
            0
        );
        assert_eq!(
            cycle(
                &root.path().join("b"),
                &receiving,
                "codex",
                &options,
                &binding,
                &key,
                &remote,
                Direction::Download,
                || false
            )
            .unwrap()
            .received,
            1
        );
        assert_eq!(
            std::fs::read_to_string(receiving.join("settings.json")).unwrap(),
            "{\"theme\":\"light\"}"
        );
        assert!(remote
            .wire
            .borrow()
            .values()
            .all(|v| !String::from_utf8_lossy(v).contains("NEVER-SEND")));
        assert!(cycle(
            &root.path().join("a"),
            &source,
            "codex",
            &options,
            &binding,
            &key,
            &remote,
            Direction::Download,
            || true
        )
        .is_err());
    }
}
