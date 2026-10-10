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
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use tauri::Manager;

// Bounded multipart transport: a workspace that exceeds one encrypted object's
// per-part budget is split into several bounded part bundles plus one
// deterministic, complete index, all riding the existing encrypted cloud queue.
#[path = "portable_parts.rs"]
pub(crate) mod parts;
use self::parts::{PortableIndex, PortablePart};
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
fn portable_relative(path: &str) -> bool {
    crate::portable_paths::path_is_portable(path)
}
fn paths_do_not_collide<'a>(paths: impl Iterator<Item = &'a String>) -> bool {
    crate::portable_paths::paths_do_not_collide(paths.map(String::as_str))
}

fn allowed(path: &str) -> bool {
    if !portable_relative(path) {
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
// Hermes captures a different portable surface (identity, rules, builtin memory,
// whole skill packages and a sanitized `config.yaml`). Its workspace paths stay
// scoped to the hermes agent so every other adapter is unchanged.
fn allowed_for(agent: &str, path: &str) -> bool {
    (agent == "hermes" && crate::hermes_workspace::allowed(path)) || allowed(path)
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
                    || paths.iter().any(|p| !allowed_for(a, p))
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
            || !crate::model::agent_available(&self.agent)
            || self.files.len() > 256
            || self.files.values().map(String::len).sum::<usize>() > MAX_TOTAL
            || self.excluded.len() > 256
        {
            return Err("portable_limit".into());
        }
        for (path, text) in &self.files {
            if !allowed_for(&self.agent, path)
                || text.len() > MAX_FILE as usize
                || suspected_secret(text)
            {
                return Err("portable_unsafe".into());
            }
            // A received Hermes workspace manifest is validated under the same
            // rules as the workspace path, so a crafted manifest is refused here
            // too rather than restored.
            if self.agent == "hermes" && path == crate::hermes_workspace::MANIFEST {
                crate::hermes_workspace::validate_manifest(text)
                    .map_err(|_| "portable_unsafe".to_string())?;
            }
            if path == "config.yaml" && self.agent == "hermes" {
                if crate::hermes_workspace::sanitized_config(text)? != *text {
                    return Err("portable_unsafe".into());
                }
            } else if ["config.toml", "settings.json"].contains(&path.as_str())
                && sanitized(text, path.ends_with("toml"))? != *text
            {
                return Err("portable_unsafe".into());
            }
        }
        if !paths_do_not_collide(self.files.keys()) {
            return Err("portable_unsafe".into());
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
    if !crate::model::agent_available(agent) {
        return Err("portable_unsafe".into());
    }
    options.validate()?;
    let mut package = Package {
        schema: 1,
        agent: agent.into(),
        ..Default::default()
    };
    if agent == "agent-memory-os" {
        return Ok(package);
    }
    // Hermes Agent travels its workspace (identity, rules, curated memory, whole
    // skill packages and a sanitized config.yaml) alongside the native state.db.
    // Previously this returned empty; now it honours the settings/skills
    // selection so preview and explicit restore are populated.
    if agent == "hermes" {
        capture_hermes(root, options, &mut package)?;
        if let Some(paths) = options.excluded_paths.get(agent) {
            for path in paths {
                if package.files.remove(path).is_some() {
                    package.excluded.insert(path.clone(), "not_selected".into());
                }
            }
        }
        package.validate()?;
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
// Build the Hermes portable package from the sanitized workspace capture. The
// settings toggle gates identity/rules/memory/config/manifest; the skills toggle
// gates the whole skill tree. Binary skill assets are not text and are recorded
// as excluded rather than silently dropped.
fn capture_hermes(root: &Path, options: &Options, package: &mut Package) -> Result<()> {
    for (path, encoded) in crate::hermes_workspace::capture(root)? {
        if path.starts_with("skills/") {
            if !options.skills {
                continue;
            }
        } else if !options.settings {
            continue;
        }
        match STANDARD.decode(&encoded) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(text) => {
                    package.files.insert(path, text);
                }
                Err(_) => {
                    package.excluded.insert(path, "non_text".into());
                }
            },
            Err(_) => {
                package.excluded.insert(path, "non_text".into());
            }
        }
    }
    Ok(())
}

/// The workspace ready to transport: the legacy single package (one
/// `portable.json` object) when it fits one part's budget, or a bounded
/// multipart set (one deterministic index plus N bounded parts) when the
/// workspace is larger or carries binary assets.
enum Workspace {
    Legacy(Package),
    Parts(Box<PortableIndex>, BTreeMap<u32, PortablePart>),
}

fn workspace_kind(path: &str) -> &'static str {
    if path == "config.yaml" {
        "config"
    } else if path.starts_with("memories/") {
        "memory"
    } else if path == "SOUL.md" || path == "AGENTS.md" {
        "identity"
    } else if path.starts_with("bastet/") {
        "manifest"
    } else {
        "skill"
    }
}

/// The legacy single package, produced only when every captured file is text
/// and the whole set fits one part's budget. A larger or binary workspace takes
/// the multipart path instead — it is never truncated to fit.
fn single_package(
    agent: &str,
    files: &BTreeMap<String, (String, Vec<u8>)>,
    excluded: &BTreeMap<String, String>,
) -> Option<Package> {
    if files.len() > 256 || excluded.len() > 256 {
        return None;
    }
    let mut text = BTreeMap::new();
    let mut total = 0usize;
    for (path, (_, bytes)) in files {
        if bytes.len() as u64 > MAX_FILE {
            return None;
        }
        total = total.checked_add(bytes.len())?;
        if total > MAX_TOTAL {
            return None;
        }
        text.insert(path.clone(), std::str::from_utf8(bytes).ok()?.to_string());
    }
    let package = Package {
        schema: 1,
        agent: agent.into(),
        files: text,
        excluded: excluded.clone(),
    };
    package.validate().ok()?;
    Some(package)
}

/// Produce the transport workspace for an agent. Every agent except Hermes keeps
/// its existing single-package behaviour; Hermes carries the whole workspace
/// binary-safe and splits it into bounded parts when it exceeds one package.
fn capture_workspace(agent: &str, root: &Path, options: &Options) -> Result<Workspace> {
    if agent != "hermes" {
        return Ok(Workspace::Legacy(capture(agent, root, options)?));
    }
    let mut files: BTreeMap<String, (String, Vec<u8>)> = BTreeMap::new();
    let mut excluded: BTreeMap<String, String> = BTreeMap::new();
    for (path, encoded) in crate::hermes_workspace::capture(root)? {
        if path.starts_with("skills/") {
            if !options.skills {
                continue;
            }
        } else if !options.settings {
            continue;
        }
        match STANDARD.decode(&encoded) {
            Ok(bytes) => {
                files.insert(path.clone(), (workspace_kind(&path).to_string(), bytes));
            }
            Err(_) => {
                excluded.insert(path, "non_text".into());
            }
        }
    }
    if let Some(paths) = options.excluded_paths.get(agent) {
        for path in paths {
            if files.remove(path).is_some() {
                excluded.insert(path.clone(), "not_selected".into());
            }
        }
    }
    if let Some(package) = single_package(agent, &files, &excluded) {
        return Ok(Workspace::Legacy(package));
    }
    let (index, part_set) = PortableIndex::split(agent, files, excluded)?;
    Ok(Workspace::Parts(Box::new(index), part_set))
}

impl Workspace {
    /// The complete byte-level file set and exclusions (used by compare/preview).
    fn into_files(self) -> (String, BTreeMap<String, Vec<u8>>, BTreeMap<String, String>) {
        match self {
            Workspace::Legacy(package) => {
                let files = package
                    .files
                    .into_iter()
                    .map(|(path, text)| (path, text.into_bytes()))
                    .collect();
                (package.agent, files, package.excluded)
            }
            Workspace::Parts(index, parts) => {
                let files = index
                    .reassemble(&parts)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(path, (_kind, bytes))| (path, bytes))
                    .collect();
                (index.agent.clone(), files, index.excluded.clone())
            }
        }
    }

    /// A per-file candidate view for the preview command. Text files carry their
    /// content and non-text files are listed as excluded — a candidate list, not
    /// a transfer object, so it is not held to the single-bundle wire budget.
    fn preview_package(&self, agent: &str) -> Package {
        match self {
            Workspace::Legacy(package) => package.clone(),
            Workspace::Parts(index, parts) => {
                let mut files = BTreeMap::new();
                let mut excluded = index.excluded.clone();
                if let Ok(assembled) = index.reassemble(parts) {
                    for (path, (_kind, bytes)) in assembled {
                        match String::from_utf8(bytes) {
                            Ok(text) => {
                                files.insert(path, text);
                            }
                            Err(_) => {
                                excluded.insert(path, "non_text".into());
                            }
                        }
                    }
                }
                Package {
                    schema: 1,
                    agent: agent.into(),
                    files,
                    excluded,
                }
            }
        }
    }
}

/// A reassembled workspace ready to compare or restore. Bytes, not text, so a
/// binary skill asset survives a multipart round trip unchanged.
struct Loaded {
    files: BTreeMap<String, Vec<u8>>,
}

fn agent_stream(agent: &str, node: &str) -> Stream {
    Stream {
        agent: agent.into(),
        profile: node.into(),
        conversation: "portable-preferences".into(),
    }
}

/// Recover a received workspace by id, transparently handling the legacy
/// single-package format and the multipart index+parts format. A multipart
/// transfer is only returned when *every* part is present and verifies; a
/// partial or tampered set is refused (`portable_incomplete` /
/// `portable_tampered`), so readiness is never inferred from a partial transfer.
fn load_workspace(root: &Path, space: &str, agent: &str, id: &str) -> Result<Loaded> {
    if !crate::model::agent_available(agent) || !bundle::is_hash(id) {
        return Err("portable_unsafe".into());
    }
    let replica = Replica::open(
        &root
            .join(format!("portable-{agent}-{space}"))
            .join("replica"),
        space,
    )?;
    let all = replica.transport_bundles()?;
    let bundle = all.get(id).ok_or("portable_missing")?;
    if PortableIndex::is_legacy(bundle) {
        let package = decode_package(bundle, agent)?;
        return Ok(Loaded {
            files: package
                .files
                .into_iter()
                .map(|(path, text)| (path, text.into_bytes()))
                .collect(),
        });
    }
    let index = PortableIndex::from_bundle(bundle)?.ok_or("portable_format")?;
    if index.agent != agent {
        return Err("portable_unsafe".into());
    }
    let mut part_set: BTreeMap<u32, PortablePart> = BTreeMap::new();
    for candidate in all.values() {
        if let Some(number) = PortableIndex::part_number(candidate) {
            let entry = candidate
                .snapshot
                .files
                .values()
                .next()
                .ok_or("portable_format")?;
            let part: PortablePart =
                serde_json::from_str(&entry.content).map_err(|_| "portable_format")?;
            if part.transfer == index.transfer {
                part_set.insert(number, part);
            }
        }
    }
    let present: std::collections::BTreeSet<u32> = part_set.keys().copied().collect();
    if !index.readiness(&present).complete {
        return Err("portable_incomplete".into());
    }
    let assembled = index.reassemble(&part_set)?;
    Ok(Loaded {
        files: assembled
            .into_iter()
            .map(|(path, (_kind, bytes))| (path, bytes))
            .collect(),
    })
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
        match capture_workspace(agent, source, options)? {
            Workspace::Legacy(package) => {
                if !package.files.is_empty() {
                    let text = serde_json::to_string(&package).map_err(|_| "portable_format")?;
                    journal.base = Some(replica.export_from(
                        agent_stream(agent, &journal.node.clone()),
                        BTreeMap::from([("portable.json".into(), text)]),
                        journal.base.as_deref(),
                    )?);
                    storage::replace(
                        &path,
                        &serde_json::to_vec(&journal).map_err(|_| "local_store_damaged")?,
                    )?;
                }
            }
            // A workspace larger than one package's budget is exported as one
            // deterministic index plus bounded parts. Each part is an ordinary
            // bundle, so the encrypted queue's idempotent, de-duplicated, retried
            // transport carries them unchanged. Parts are independent roots (no
            // base chaining), and re-exporting identical content yields the same
            // bundle ids, so a repeated cycle is a no-op.
            Workspace::Parts(index, part_set) => {
                let entries = BTreeMap::from([(parts::INDEX_FILE.to_string(), index.to_json()?)]);
                replica.export_from(agent_stream(agent, &journal.node.clone()), entries, None)?;
                for part in part_set.values() {
                    let json = serde_json::to_string(part).map_err(|_| "portable_format")?;
                    let entries = BTreeMap::from([(
                        format!("{}{}.json", parts::PART_PREFIX, part.part),
                        json,
                    )]);
                    replica.export_from(
                        agent_stream(agent, &journal.node.clone()),
                        entries,
                        None,
                    )?;
                }
                journal.base = None;
                storage::replace(
                    &path,
                    &serde_json::to_vec(&journal).map_err(|_| "local_store_damaged")?,
                )?;
            }
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
    /// Serialized *actual* per-transfer readiness, derived from the bundles
    /// really present in this replica (never a sender flag or a guess). The UI
    /// renders exactly these counts; it never invents an "N of M".
    pub readiness: ReceivedReadiness,
}

/// Readiness of one received workspace entry.
///
/// - A multipart transfer: `complete` when the index validates *and* every
///   declared part is present and bound to this transfer; otherwise `present`
///   / `expected` / `missing` describe exactly what is still outstanding.
///   `complete` is presence + index consistency only — listing never reads a
///   part payload, so it never asserts byte-level integrity. Compare and
///   restore reassemble the parts and verify every chunk, so a
///   complete-but-tampered set still reads `complete` here and is refused
///   there with `portable_tampered`.
/// - The legacy single-package format is one complete object, so it is
///   `complete` (`legacy` = true) with `present == expected == 1`, verified
///   when parsed.
/// - `issue` is set when the entry could not be read at all, so a malformed
///   item is surfaced explicitly (with an actionable code) instead of being
///   silently dropped or failing the whole list.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivedReadiness {
    pub complete: bool,
    pub legacy: bool,
    pub present: usize,
    pub expected: usize,
    pub missing: Vec<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
}

impl ReceivedReadiness {
    fn unreadable(issue: String) -> Self {
        Self {
            complete: false,
            legacy: false,
            present: 0,
            expected: 0,
            missing: Vec::new(),
            issue: Some(issue),
        }
    }
}

/// The part numbers present per transfer id in one replica.
///
/// A part counts as present for a transfer only when it is a *well-formed*
/// [`PortablePart`] that declares that transfer id and its own file-number.
/// A substring scan of the raw content would let a forged or unparseable
/// bundle inflate the count (and so report a false "complete"), so presence is
/// never inferred from a substring.
fn parts_by_transfer(
    all: &BTreeMap<String, Bundle>,
) -> BTreeMap<String, std::collections::BTreeSet<u32>> {
    let mut by_transfer: BTreeMap<String, std::collections::BTreeSet<u32>> = BTreeMap::new();
    for candidate in all.values() {
        let Some(number) = PortableIndex::part_number(candidate) else {
            continue;
        };
        let Some(entry) = candidate.snapshot.files.values().next() else {
            continue;
        };
        let Ok(part) = serde_json::from_str::<PortablePart>(&entry.content) else {
            continue;
        };
        if part.part != number || !bundle::is_hash(&part.transfer) {
            continue;
        }
        by_transfer.entry(part.transfer).or_default().insert(number);
    }
    by_transfer
}

/// Classify one received head bundle and derive its readiness from the parts
/// actually present for its transfer.
///
/// `Ok(None)` means the bundle is a payload part (not itself a workspace
/// entry). `Err(code)` means the entry could not be read; the caller lists it
/// with an explicit issue rather than dropping it.
fn entry_readiness(
    present: &BTreeMap<String, std::collections::BTreeSet<u32>>,
    b: &Bundle,
    agent: &str,
) -> Result<Option<(usize, ReceivedReadiness)>> {
    if let Some(index) = PortableIndex::from_bundle(b)? {
        // Only parts that declare this transfer's id count as present; a
        // foreign or stale part is not evidence of readiness.
        let none = std::collections::BTreeSet::new();
        let here = present.get(&index.transfer).unwrap_or(&none);
        let readiness = index.readiness(here);
        return Ok(Some((
            index.files.len(),
            ReceivedReadiness {
                complete: readiness.complete,
                legacy: false,
                present: readiness.present,
                expected: readiness.expected,
                missing: readiness.missing,
                issue: None,
            },
        )));
    }
    if PortableIndex::is_legacy(b) {
        let package = decode_package(b, agent)?;
        return Ok(Some((
            package.files.len(),
            ReceivedReadiness {
                complete: true,
                legacy: true,
                present: 1,
                expected: 1,
                missing: Vec::new(),
                issue: None,
            },
        )));
    }
    Ok(None)
}
fn context(app: &tauri::AppHandle) -> Result<(PathBuf, Settings, String)> {
    let root = app
        .path()
        .app_config_dir()
        .map_err(|_| "store_unavailable")?;
    let settings = crate::model::load(&root.join("settings.json"))?.ok_or("invalid_settings")?;
    let binding = if settings.cloud_provider == "google-drive" {
        crate::cloud::wizard::Transaction::open(&root)?
            .state
            .binding
    } else {
        crate::cloud::folder::load(&root, &settings.cloud_provider)?.and_then(|s| s.binding)
    };
    Ok((root, settings, binding.ok_or("wizard_step_required")?.space))
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
pub async fn portable_preview(app: tauri::AppHandle, settings: Settings) -> Result<Vec<Package>> {
    tauri::async_runtime::spawn_blocking(move || {
        let config = app
            .path()
            .app_config_dir()
            .map_err(|_| "store_unavailable")?;
        let _scopes = crate::sandbox_access::provider_scopes(&config, &settings)?;
        crate::model::validate(&settings)?;
        let agents = crate::detect(Some(&settings));
        let (tasks, _) = crate::worker::parallel::plan(&settings.selected_agents, &agents);
        let mut options = settings.portable.clone();
        options.excluded_paths.clear(); // Preview includes candidates so each can be selected again.
        tasks
            .into_iter()
            .filter_map(|t| t.path.map(|p| (t.canonical, p)))
            .map(|(agent, path)| {
                capture_workspace(&agent, &path, &options)
                    .map(|workspace| workspace.preview_package(&agent))
            })
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
        for agent in crate::model::AGENTS
            .into_iter()
            .filter(|agent| crate::model::agent_available(agent))
        {
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
            // Present parts per transfer, computed once for the whole replica.
            let present_by_transfer = parts_by_transfer(&all);
            for b in all.values().filter(|b| !parents.contains(&b.id)) {
                let saved_at =
                    std::fs::metadata(path.join("replica/objects").join(format!("{}.json", b.id)))
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|v| v.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|v| v.as_secs());
                // Isolate a malformed item: it is listed with an explicit issue
                // rather than failing the whole command or being silently
                // dropped, and the valid transfers beside it are still listed.
                let (files, readiness) = match entry_readiness(&present_by_transfer, b, agent) {
                    Ok(Some(entry)) => entry,
                    // A part bundle is not itself a received workspace entry.
                    Ok(None) => continue,
                    Err(code) => (0, ReceivedReadiness::unreadable(code)),
                };
                entries.push(Received {
                    agent: agent.into(),
                    id: b.id.clone(),
                    files,
                    saved_at,
                    readiness,
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
        let loaded = load_workspace(&root, &space, &agent, &id)?;
        let agents = crate::detect(Some(&settings));
        let source = agents
            .iter()
            .find(|a| a.id == agent)
            .ok_or("source_missing")?;
        let _scope = crate::sandbox_access::access(&root, Path::new(&source.path))?;
        let (_, local, _) = capture_workspace(
            &agent,
            Path::new(&source.path),
            &Options {
                settings: true,
                skills: true,
                ..Default::default()
            },
        )?
        .into_files();
        let files = loaded
            .files
            .into_iter()
            .map(|(path, bytes)| review::diff(path.clone(), local.get(&path).cloned(), &bytes))
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
        let loaded = load_workspace(&root, &space, &agent, &id)?;
        let Some(folder) = rfd::FileDialog::new().pick_folder() else {
            return Ok(None);
        };
        let folder = crate::sandbox_access::grant_selected(&root, &folder)?;
        let _scope = crate::sandbox_access::access(&root, &folder)?;
        let stage = tempfile::tempdir_in(&folder).map_err(|_| "store_unavailable")?;
        for (path, bytes) in loaded.files {
            let destination = stage.path().join(path);
            std::fs::create_dir_all(destination.parent().ok_or("unsafe_store")?)
                .map_err(|_| "store_unavailable")?;
            storage::immutable(&destination, &bytes)?;
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
    #[cfg(feature = "mac-app-store")]
    #[test]
    fn store_channel_rejects_memory_portable_capture_before_reading_files() {
        let absent = Path::new("/definitely-not-a-memory-store");
        assert!(matches!(
            capture("agent-memory-os", absent, &Options::default()),
            Err(e) if e == "portable_unsafe"
        ));
    }
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
            ("skills/x/bad?.md", "text"),
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
    #[test]
    fn portable_paths_keep_unicode_but_reject_windows_forbidden_names() {
        assert!(allowed("skills/猫/説明.md"));
        for path in [
            "skills/cat/a<b.md",
            "skills/cat/a>b.md",
            "skills/cat/a\"b.md",
            "skills/cat/a|b.md",
            "skills/cat/a?b.md",
            "skills/cat/a*b.md",
            "skills/cat/a\u{1f}b.md",
            "skills/cat/CON.md",
        ] {
            assert!(!allowed(path), "{path:?}");
        }
        assert!(!allowed(&format!("skills/cat/{}.md", "a".repeat(253))));

        let root = tempfile::tempdir().unwrap();
        write(root.path(), "skills/猫/説明.md", "portable");
        #[cfg(unix)]
        write(root.path(), "skills/猫/bad?.md", "unsupported on Windows");
        let package = capture(
            "codex",
            root.path(),
            &Options {
                skills: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(package.files["skills/猫/説明.md"], "portable");
        #[cfg(unix)]
        assert_eq!(
            package.excluded["skills/猫/bad?.md"],
            "unsupported_or_sensitive_path"
        );
    }
    #[test]
    fn received_packages_reject_case_unicode_and_file_directory_aliases() {
        for paths in [
            vec!["skills/Cat/a.md", "skills/cat/a.md"],
            vec!["skills/Σ/a.md", "skills/ς/a.md"],
            vec!["skills/caf\u{e9}/a.md", "skills/cafe\u{301}/a.md"],
            vec!["skills/a.md", "skills/A.md/child.txt"],
        ] {
            let package = Package {
                schema: 1,
                agent: "codex".into(),
                files: paths
                    .iter()
                    .map(|path| ((*path).into(), "text".into()))
                    .collect(),
                ..Default::default()
            };
            assert!(package.validate().is_err(), "{paths:?}");
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
    #[test]
    fn hermes_workspace_is_captured_into_portable_package_and_respects_selection() {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "config.yaml",
            "model:\n  default: \"gpt-test\"\napi_key: LEAK\n",
        );
        write(root.path(), "SOUL.md", "identity");
        write(root.path(), "skills/cat/SKILL.md", "# Cat\n");

        // Nothing selected stays empty (default options).
        assert!(capture("hermes", root.path(), &Options::default())
            .unwrap()
            .files
            .is_empty());

        let options = Options {
            settings: true,
            skills: true,
            ..Default::default()
        };
        let package = capture("hermes", root.path(), &options).unwrap();
        assert!(package.files.contains_key("SOUL.md"));
        assert!(package.files.contains_key("config.yaml"));
        assert!(package.files.contains_key("skills/cat/SKILL.md"));
        assert!(package
            .files
            .contains_key(crate::hermes_workspace::MANIFEST));
        assert!(!package.files["config.yaml"].contains("LEAK"));

        // Selective capture: skills only drops the identity/config surface.
        let skills_only = Options {
            settings: false,
            skills: true,
            ..Default::default()
        };
        let package = capture("hermes", root.path(), &skills_only).unwrap();
        assert!(package.files.contains_key("skills/cat/SKILL.md"));
        assert!(!package.files.contains_key("SOUL.md"));
    }

    #[test]
    fn hermes_portable_receive_validates_the_manifest_and_preserves_provider() {
        // The captured, sanitized config keeps the (non-secret) provider selection.
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "config.yaml",
            "memory:\n  provider: \"agent-memory-os\"\napi_key: LEAK\n",
        );
        let package = capture(
            "hermes",
            root.path(),
            &Options {
                settings: true,
                skills: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(package.files["config.yaml"].contains("agent-memory-os"));
        assert!(!package.files["config.yaml"].contains("LEAK"));
        package.validate().unwrap();

        // A received package carrying a crafted workspace manifest (opaque secret
        // in command/args, or a generic credential) is refused, not restored.
        for text in [
            "{\"version\":1,\"agent\":\"hermes\",\"files\":[],\"issues\":[],\"dependencies\":[{\"kind\":\"mcp\",\"name\":\"evil\",\"detail\":\"install\",\"blocking\":true,\"args\":[\"--password\",\"hunter2\"]}]}",
            "{\"version\":1,\"agent\":\"hermes\",\"files\":[],\"issues\":[],\"dependencies\":[{\"kind\":\"env\",\"name\":\"LEAK_KEY\",\"detail\":\"sk-ant-LEAK\",\"blocking\":true}]}",
        ] {
            let package = Package {
                schema: 1,
                agent: "hermes".into(),
                files: BTreeMap::from([(
                    crate::hermes_workspace::MANIFEST.into(),
                    text.into(),
                )]),
                ..Default::default()
            };
            assert_eq!(package.validate().unwrap_err(), "portable_unsafe");
        }

        // Any received MCP command/args is refused on the portable receive path
        // too — even a secret-free legacy shape or an opaque positional token the
        // producer never emits and no keyword heuristic can judge.
        for dependency in [
            serde_json::json!({"kind": "mcp", "name": "demo", "detail": "install",
                "blocking": true, "command": "/usr/bin/demo",
                "args": ["serve", "--port", "9000"]}),
            serde_json::json!({"kind": "mcp", "name": "demo", "detail": "install",
                "blocking": true, "args": ["xY9zQ2wLp"]}),
            serde_json::json!({"kind": "mcp", "name": "demo", "detail": "install",
                "blocking": true, "args": ["-p", "hunter2"]}),
        ] {
            let manifest = serde_json::json!({
                "version": 1, "agent": "hermes", "files": [], "issues": [],
                "dependencies": [dependency],
            });
            let package = Package {
                schema: 1,
                agent: "hermes".into(),
                files: BTreeMap::from([(
                    crate::hermes_workspace::MANIFEST.into(),
                    serde_json::to_string(&manifest).unwrap(),
                )]),
                ..Default::default()
            };
            assert_eq!(package.validate().unwrap_err(), "portable_unsafe");
        }
    }

    #[test]
    fn a_small_hermes_home_keeps_the_legacy_single_package_wire_format() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        write(&source, "config.yaml", "model:\n  default: \"gpt-test\"\n");
        write(&source, "SOUL.md", "identity");
        write(&source, "skills/cat/SKILL.md", "# Cat\n");
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
            skills: true,
            ..Default::default()
        };
        let exported = cycle(
            &root.path().join("portable-hermes-space"),
            &source,
            "hermes",
            &options,
            &binding,
            &key,
            &remote,
            Direction::Upload,
            || false,
        )
        .unwrap();
        // One bundle, the 0.8 single-package format.
        assert_eq!(exported.published, 1);
    }

    /// The core integration proof: a Hermes workspace with far more files than
    /// one package's budget round-trips completely through the real encrypted
    /// queue (`cycle` -> `exchange_filtered`), with a missing part refusing
    /// readiness. Both ends are independent local replicas and the wire holds no
    /// plaintext.
    #[test]
    fn a_multi_part_hermes_workspace_round_trips_through_the_encrypted_queue() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        // 500 payload files plus a legitimately large asset (chunked).
        for i in 0..500 {
            write(
                &source,
                &format!("skills/s{i:04}/SKILL.md"),
                &format!("# skill {i}\n"),
            );
        }
        write(&source, "SOUL.md", "identity");
        let big = source.join("skills/big/data.bin");
        std::fs::create_dir_all(big.parent().unwrap()).unwrap();
        std::fs::write(&big, vec![9u8; 2 * 1024 * 1024]).unwrap();

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
            skills: true,
            ..Default::default()
        };
        let a = root.path().join("a");
        let b = root.path().join("b");
        std::fs::create_dir_all(a.join("portable-hermes-space")).unwrap();
        std::fs::create_dir_all(b.join("portable-hermes-space")).unwrap();

        let exported = cycle(
            &a.join("portable-hermes-space"),
            &source,
            "hermes",
            &options,
            &binding,
            &key,
            &remote,
            Direction::Upload,
            || false,
        )
        .unwrap();
        // One index plus at least two payload parts.
        assert!(exported.published > 2, "published {}", exported.published);
        // Nothing crosses the encrypted wire in clear.
        assert!(remote
            .wire
            .borrow()
            .values()
            .all(|bytes| !String::from_utf8_lossy(bytes).contains("skills/")));

        // A repeated upload publishes nothing new (idempotent, de-duplicated).
        let repeated = cycle(
            &a.join("portable-hermes-space"),
            &source,
            "hermes",
            &options,
            &binding,
            &key,
            &remote,
            Direction::Upload,
            || false,
        )
        .unwrap();
        assert_eq!(repeated.published, 0);

        // An independent receiving replica fetches every bundle.
        let received = cycle(
            &b.join("portable-hermes-space"),
            &source,
            "hermes",
            &options,
            &binding,
            &key,
            &remote,
            Direction::Download,
            || false,
        )
        .unwrap();
        assert!(received.received > 2, "received {}", received.received);

        // Reassemble from the receiver's replica: every file, byte-identical.
        let replica = Replica::open(&b.join("portable-hermes-space/replica"), "space").unwrap();
        let all = replica.transport_bundles().unwrap();
        let index_id = all
            .values()
            .find(|bundle| PortableIndex::from_bundle(bundle).unwrap().is_some())
            .expect("index bundle")
            .id
            .clone();
        drop(replica);
        let loaded = load_workspace(&b, "space", "hermes", &index_id).unwrap();
        // 500 skills + the large asset + SOUL.md + the workspace manifest.
        assert_eq!(loaded.files.len(), 503);
        assert_eq!(loaded.files["skills/s0000/SKILL.md"], b"# skill 0\n");
        assert_eq!(
            loaded.files["skills/big/data.bin"],
            vec![9u8; 2 * 1024 * 1024]
        );
    }

    use crate::sync::bundle::{Entry, Snapshot};
    fn bundle_stream() -> Stream {
        Stream {
            agent: "hermes".into(),
            profile: "node".into(),
            conversation: "portable-preferences".into(),
        }
    }
    fn bundles_of(
        index: &PortableIndex,
        parts: &BTreeMap<u32, PortablePart>,
    ) -> BTreeMap<String, Bundle> {
        let mut all = BTreeMap::new();
        let ib = index.bundle("space", "node", bundle_stream()).unwrap();
        all.insert(ib.id.clone(), ib);
        for part in parts.values() {
            let pb = part.bundle("space", "node", bundle_stream()).unwrap();
            all.insert(pb.id.clone(), pb);
        }
        all
    }
    fn index_id(all: &BTreeMap<String, Bundle>) -> String {
        all.values()
            .find(|b| PortableIndex::from_bundle(b).unwrap().is_some())
            .expect("index bundle")
            .id
            .clone()
    }
    fn many_files() -> BTreeMap<String, (String, Vec<u8>)> {
        (0..600)
            .map(|i| {
                (
                    format!("skills/s{i:04}/SKILL.md"),
                    ("skill".to_string(), vec![b'x'; 40]),
                )
            })
            .collect()
    }

    /// The received entry must expose *actual* serialized readiness: present /
    /// expected / missing derived from the parts really present, never a guess;
    /// and a missing part is named exactly.
    #[test]
    fn received_readiness_is_serialized_from_the_parts_actually_present() {
        let (index, parts) = PortableIndex::split("hermes", many_files(), BTreeMap::new()).unwrap();
        assert!(parts.len() > 1, "expected a multipart transfer");
        let full = bundles_of(&index, &parts);

        let head = &full[&index_id(&full)];
        let (files, complete) = entry_readiness(&parts_by_transfer(&full), head, "hermes")
            .unwrap()
            .unwrap();
        assert_eq!(files, 600);
        assert!(complete.complete && !complete.legacy && complete.missing.is_empty());
        assert_eq!(
            (complete.present, complete.expected),
            (parts.len(), parts.len())
        );
        // camelCase keys the frontend reads, and no fabricated issue.
        let json = serde_json::to_value(&complete).unwrap();
        assert_eq!(json["complete"], true);
        assert_eq!(json["legacy"], false);
        assert_eq!(json["missing"].as_array().unwrap().len(), 0);
        assert!(json.get("issue").is_none());
        // The serialized readiness is presence + index consistency only: it
        // exposes no "verified"/"ready" claim, because listing never reads a
        // part payload.
        assert!(json.get("ready").is_none());

        // One part absent: not complete, and exactly that part is reported missing.
        let dropped = *parts.keys().max().unwrap();
        let mut partial = full.clone();
        partial.retain(|_, b| PortableIndex::part_number(b) != Some(dropped));
        let head2 = &partial[&index_id(&partial)];
        let (_, waiting) = entry_readiness(&parts_by_transfer(&partial), head2, "hermes")
            .unwrap()
            .unwrap();
        assert!(!waiting.complete);
        assert_eq!(waiting.expected, parts.len());
        assert_eq!(waiting.present, parts.len() - 1);
        assert_eq!(waiting.missing, vec![dropped]);
    }

    /// A malformed item is surfaced with an explicit issue and does not hide a
    /// valid snapshot listed beside it.
    #[test]
    fn a_malformed_item_is_listed_with_an_issue_beside_a_valid_snapshot() {
        let legacy = Bundle::new(Snapshot {
            schema: 1,
            space: "space".into(),
            device: "node".into(),
            stream: Stream {
                agent: "codex".into(),
                profile: "node".into(),
                conversation: "portable-preferences".into(),
            },
            parents: vec![],
            files: BTreeMap::from([(
                "portable.json".into(),
                Entry::new(
                    serde_json::to_string(&Package {
                        schema: 1,
                        agent: "codex".into(),
                        files: BTreeMap::from([("skills/cat/SKILL.md".into(), "# Cat\n".into())]),
                        excluded: BTreeMap::new(),
                    })
                    .unwrap(),
                ),
            )]),
        })
        .unwrap();
        let mut all = BTreeMap::new();
        all.insert(legacy.id.clone(), legacy.clone());
        let (files, complete) = entry_readiness(&parts_by_transfer(&all), &legacy, "codex")
            .unwrap()
            .unwrap();
        assert_eq!(files, 1);
        assert!(complete.complete && complete.legacy && complete.missing.is_empty());

        // A malformed index-shaped bundle: refused with an explicit code, never
        // silently dropped, and the valid snapshot stays classifiable.
        let broken = Bundle::new(Snapshot {
            schema: 1,
            space: "space".into(),
            device: "node".into(),
            stream: bundle_stream(),
            parents: vec![],
            files: BTreeMap::from([(
                parts::INDEX_FILE.into(),
                Entry::new("{not a valid index".into()),
            )]),
        })
        .unwrap();
        all.insert(broken.id.clone(), broken.clone());
        let issue = entry_readiness(&parts_by_transfer(&all), &broken, "hermes").unwrap_err();
        assert!(!issue.is_empty());
        assert!(
            entry_readiness(&parts_by_transfer(&all), &legacy, "codex")
                .unwrap()
                .unwrap()
                .1
                .complete
        );
    }

    /// A complete-but-tampered set reads `complete` (presence + index only) but
    /// is refused when reassembled: the byte-level gate never claims validated
    /// without evidence.
    #[test]
    fn a_tampered_part_is_refused_by_reassembly_even_when_the_set_is_complete() {
        let (index, mut parts) =
            PortableIndex::split("hermes", many_files(), BTreeMap::new()).unwrap();
        let n = *parts.keys().next().unwrap();
        let part = parts.get_mut(&n).unwrap();
        let key = part.chunks.keys().next().unwrap().clone();
        part.chunks.get_mut(&key).unwrap()[0].content = STANDARD.encode(b"tampered");
        let present: std::collections::BTreeSet<u32> = parts.keys().copied().collect();
        assert!(
            index.readiness(&present).complete,
            "presence-complete but not byte-verified"
        );
        assert_eq!(index.reassemble(&parts).unwrap_err(), "portable_tampered");
    }

    /// A bundle whose file name looks like a part but whose content merely
    /// *contains* the victim transfer id as a substring (and does not parse as a
    /// well-formed part) must not count as a present part: presence is never
    /// inferred from a substring, so a forged bundle cannot make a partial
    /// transfer read `complete`.
    #[test]
    fn a_forged_part_substring_does_not_count_as_present() {
        let (index, parts) = PortableIndex::split("hermes", many_files(), BTreeMap::new()).unwrap();
        assert!(parts.len() > 1, "expected a multipart transfer");
        let mut all = bundles_of(&index, &parts);
        // Drop one real part, so the honest transfer is incomplete.
        let dropped = *parts.keys().max().unwrap();
        all.retain(|_, b| PortableIndex::part_number(b) != Some(dropped));
        let head_id = index_id(&all);
        // Baseline: genuinely incomplete.
        let (_, before) = entry_readiness(&parts_by_transfer(&all), &all[&head_id], "hermes")
            .unwrap()
            .unwrap();
        assert!(!before.complete);

        // Forge a bundle named exactly like the dropped part whose content embeds
        // the victim transfer id but is not a valid part payload.
        let forged = Bundle::new(Snapshot {
            schema: 1,
            space: "space".into(),
            device: "node".into(),
            stream: bundle_stream(),
            parents: vec![],
            files: BTreeMap::from([(
                format!("{}{}.json", parts::PART_PREFIX, dropped),
                Entry::new(format!("{{\"transfer\":\"{}\",\"junk\":", index.transfer)),
            )]),
        })
        .unwrap();
        all.insert(forged.id.clone(), forged);
        let (_, after) = entry_readiness(&parts_by_transfer(&all), &all[&head_id], "hermes")
            .unwrap()
            .unwrap();
        assert!(
            !after.complete,
            "a forged substring must not stand in for a missing part"
        );
        assert_eq!(after.missing, vec![dropped]);
    }
}
