//! External-memory continuation for a restored Hermes home.
//!
//! Hermes may run its durable memory through the external AgentMemoryOS
//! provider (`memory.provider: agent-memory-os` in `config.yaml`) instead of,
//! or alongside, the built-in `memories/MEMORY.md` / `USER.md` files. Those
//! files are the workspace module's business; this module carries the *bound
//! external store* across hosts: the non-secret provider binding/identity plus
//! the official AgentMemoryOS export, and imports it into a NEW isolated store
//! on the receiving host. The live store is never read by a database reader and
//! never replaced or applied to.
//!
//! A capture is a self-describing base64 file map in one reserved namespace:
//!
//! * `bastet/memory-manifest.json` — provider, non-secret identity and
//!   preferences, the export shape (record count, authority changes) and
//!   machine-readable blockers. It never contains a local source home path or a
//!   token.
//! * `bastet/memory-export.jsonl` — the official `agent-memory sync export`
//!   output, base64, present only when the export was explicitly authorized.
//!
//! ## Authorization (explicitly bound, never a silent dump)
//!
//! The export is produced **only** when the profile's `config.yaml` selects the
//! `agent-memory-os` provider *and* an explicit, consistent memory home is
//! bound (Bastet's `settings.custom_paths["agent-memory-os"]`, the profile's
//! own `agent-memory-os.json` `home`, or the `AGENT_MEMORY_HOME` fallback).
//! Every other case is an explicit blocker, never a silent built-in
//! assumption. A missing or unreadable `config.yaml` fails closed (it is not
//! treated as "built-in").
//!
//! The CLI call used here is `agent-memory sync export` with **no** team/project
//! scope. By the provider's own contract that includes the profile's private
//! memories (`visibility: []`) and its team/project/ACL "authority" records —
//! this is the same-identity continuation the user asked for, not a raw database
//! copy, but it *is broad*: it is gated entirely on the explicit binding above,
//! never inferred, and the manifest records the record count and whether
//! authority changes are present.
//!
//! ## Restore
//!
//! [`restore`] validates the map, requires the recipient to have opted into the
//! AgentMemoryOS source **and** to have the provider plugin discoverable from
//! the isolated home, then runs the official CLI import into a NEW nested
//! `managed_home/.agent-memory` store it reserves atomically. It binds the
//! original `agent_id` in a local `agent-memory-os.json` (rewritten to the new
//! home) and then *re-exports the restored store* and requires the recalled
//! records, authority flags and normalized fingerprint to match the captured
//! export — a `memories.db` that merely exists is not recall, and the provider's
//! `is_available()` is *not* trusted. An existing destination, or a missing
//! plugin/identity/CLI, is reported, not forced. Received files are never
//! executed.
//!
//! ## CLI selection
//!
//! [`capture_with_context`]/[`restore_with_context`] take the app config root so
//! the CLI the user explicitly chose (`choose_memory_cli` →
//! `$root/memory-cli.json`) is honoured through [`crate::amos_runtime`]'s
//! `configured(root)`. The bare [`capture`]/[`restore`] wrappers keep the native
//! peer's signature and fall back to the autodiscovered executable.
//!
//! The module reuses [`crate::amos_runtime`]'s bounded CLI invocation and
//! [`crate::memory_adapter`]'s envelope validation for the export; the import
//! into a brand-new store mirrors the same bounds because
//! `amos_runtime::apply` requires the destination store to already exist.

use crate::{
    memory_adapter,
    model::Settings,
    portable_paths::{path_is_portable, paths_do_not_collide},
    sync::{bundle::Result, storage},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
#[cfg(not(feature = "mac-app-store"))]
use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// Inert memory manifest shipped inside the memory payload.
pub(crate) const MANIFEST: &str = "bastet/memory-manifest.json";
/// Official AgentMemoryOS export, base64, present only when authorized.
pub(crate) const EXPORT: &str = "bastet/memory-export.jsonl";
/// Non-secret provider binding the Hermes plugin reads from `HERMES_HOME`.
pub(crate) const PROVIDER_CONFIG: &str = "agent-memory-os.json";
/// Nested store this module creates inside a managed home; never the live home.
pub(crate) const MANAGED_DIR: &str = ".agent-memory";

const AGENT: &str = "hermes";
const SCHEMA: u32 = 1;
const PROVIDER_AMOS: &str = "agent-memory-os";
/// Hermes treats these (`agent/memory_provider.py`) as the built-in provider.
const BUILTIN: [&str; 5] = ["", "default", "builtin", "built-in", "none"];
/// Official AgentMemoryOS export bound (documented 8 MiB / 10,000 records).
const MAX_EXPORT: usize = 8 * 1024 * 1024;
const MAX_CONFIG: u64 = 1024 * 1024;
const MAX_BINDING: u64 = 256 * 1024;
const MAX_TOTAL: usize = 16 * 1024 * 1024;
const MAX_BLOCKERS: usize = 64;

fn suspected_secret(text: &str) -> bool {
    [
        "-----BEGIN PRIVATE KEY",
        "-----BEGIN RSA PRIVATE KEY",
        "-----BEGIN OPENSSH PRIVATE KEY",
        "sk-proj-",
        "sk-ant-",
        "ghp_",
        "github_pat_",
        "xoxb-",
        "AIza",
        "AKIA",
        "Bearer ",
    ]
    .iter()
    .any(|pattern| text.contains(pattern))
}

fn safe_text(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && !value.contains(['\n', '\r', '\0'])
        && !value.chars().any(char::is_control)
        && !value.contains("${")
        && !suspected_secret(value)
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn expand_home(value: &str) -> Option<PathBuf> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let path = if let Some(rest) = trimmed.strip_prefix("~/") {
        dirs::home_dir()?.join(rest)
    } else if trimmed == "~" {
        dirs::home_dir()?
    } else {
        PathBuf::from(trimmed)
    };
    path.is_absolute().then_some(path)
}

fn same_path(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a.to_string_lossy().trim_end_matches('/') == b.to_string_lossy().trim_end_matches('/'),
    }
}

/// A non-secret provider binding read from `$HERMES_HOME/agent-memory-os.json`.
#[derive(Debug, Default, Clone)]
struct Binding {
    home: Option<String>,
    agent_id: Option<String>,
    share_default: Option<String>,
    mirror_builtin: Option<bool>,
    capture_delegations: Option<bool>,
    prefetch_limit: Option<u64>,
    prefetch_max_tokens: Option<u64>,
}

fn binding_string(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<String> {
    obj.get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| safe_text(value, 200))
        .map(str::to_string)
}

fn read_binding(home: &Path) -> Option<Binding> {
    let bytes = storage::read(&home.join(PROVIDER_CONFIG), MAX_BINDING).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let obj = value.as_object()?;
    Some(Binding {
        home: binding_string(obj, "home"),
        agent_id: binding_string(obj, "agent_id"),
        share_default: binding_string(obj, "share_default"),
        mirror_builtin: obj
            .get("mirror_builtin")
            .and_then(serde_json::Value::as_bool),
        capture_delegations: obj
            .get("capture_delegations")
            .and_then(serde_json::Value::as_bool),
        prefetch_limit: obj
            .get("prefetch_limit")
            .and_then(serde_json::Value::as_u64),
        prefetch_max_tokens: obj
            .get("prefetch_max_tokens")
            .and_then(serde_json::Value::as_u64),
    })
}

/// The resolved memory provider of a profile's `config.yaml`.
enum Provider {
    /// Built-in file memory; no CLI is needed.
    Builtin,
    /// The external AgentMemoryOS provider.
    External,
    /// A named provider this build does not support.
    Unknown(String),
    /// `config.yaml` is absent, unreadable or ambiguous: fail closed.
    Unresolved,
}

fn read_provider(home: &Path) -> Provider {
    let Ok(bytes) = storage::read(&home.join("config.yaml"), MAX_CONFIG) else {
        return Provider::Unresolved;
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return Provider::Unresolved;
    };
    let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(text) else {
        return Provider::Unresolved;
    };
    if yaml.is_null() {
        // An explicitly empty configuration is the documented built-in default.
        return Provider::Builtin;
    }
    let Ok(json) = serde_json::to_value(&yaml) else {
        return Provider::Unresolved;
    };
    let Some(root) = json.as_object() else {
        return Provider::Unresolved;
    };
    let Some(memory) = root.get("memory") else {
        return Provider::Builtin;
    };
    let Some(memory) = memory.as_object() else {
        return Provider::Unresolved;
    };
    let provider = match memory.get("provider") {
        None | Some(serde_json::Value::Null) => return Provider::Builtin,
        Some(serde_json::Value::String(value)) => value.trim().to_string(),
        Some(_) => return Provider::Unresolved,
    };
    if BUILTIN.contains(&provider.as_str()) {
        Provider::Builtin
    } else if provider == PROVIDER_AMOS {
        Provider::External
    } else if safe_text(&provider, 64) {
        Provider::Unknown(provider)
    } else {
        Provider::Unresolved
    }
}

/// Explicit, consistent memory home plus identity. `None` means the store was
/// not explicitly authorized and must never be exported implicitly.
fn authorized_memory_home(settings: &Settings, binding: Option<&Binding>) -> Option<PathBuf> {
    let custom = settings
        .custom_paths
        .get(PROVIDER_AMOS)
        .and_then(|value| expand_home(value));
    let from_profile = binding
        .and_then(|binding| binding.home.as_deref())
        .and_then(expand_home);
    match (custom, from_profile) {
        (Some(a), Some(b)) => same_path(&a, &b).then_some(a),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        // Fallback only when nothing explicit is bound locally.
        (None, None) => env_nonempty("AGENT_MEMORY_HOME").and_then(|value| expand_home(&value)),
    }
}

fn identity(binding: Option<&Binding>) -> Option<String> {
    binding
        .and_then(|binding| binding.agent_id.clone())
        .or_else(|| env_nonempty("AGENT_MEMORY_AGENT_ID"))
        .filter(|value| safe_text(value, 200))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MemoryManifest {
    pub version: u32,
    pub agent: String,
    /// `"builtin"`, `"agent-memory-os"` or the recorded unknown provider name
    /// (`""` when the provider could not be resolved).
    pub provider: String,
    /// Original AgentMemoryOS identity; never a path or token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Non-secret provider preferences echoed from the local binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share_default: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_builtin: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_delegations: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefetch_limit: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefetch_max_tokens: Option<u64>,
    /// True only when `bastet/memory-export.jsonl` is part of the bundle.
    pub exported: bool,
    pub records: usize,
    pub contains_authority_changes: bool,
    /// Explicit, machine-readable reasons the memory continuation is not ready.
    pub blockers: Vec<String>,
}

impl Default for MemoryManifest {
    fn default() -> Self {
        Self {
            version: SCHEMA,
            agent: AGENT.into(),
            provider: String::new(),
            agent_id: None,
            share_default: None,
            mirror_builtin: None,
            capture_delegations: None,
            prefetch_limit: None,
            prefetch_max_tokens: None,
            exported: false,
            records: 0,
            contains_authority_changes: false,
            blockers: Vec::new(),
        }
    }
}

/// Outcome of a restore: whether the restored home can actually continue the
/// external memory, with the concrete store and any blockers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryReadiness {
    pub provider: String,
    pub ready: bool,
    pub exported: bool,
    pub imported: bool,
    pub records: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Nested store, relative to the managed home; never an absolute source path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store: Option<String>,
    pub contains_authority_changes: bool,
    pub blockers: Vec<String>,
}

fn not_ready(manifest: &MemoryManifest, reason: &str) -> MemoryReadiness {
    let mut blockers = manifest.blockers.clone();
    if !blockers.iter().any(|b| b == reason) {
        blockers.push(reason.into());
    }
    MemoryReadiness {
        provider: manifest.provider.clone(),
        ready: false,
        exported: manifest.exported,
        imported: false,
        records: manifest.records,
        agent_id: manifest.agent_id.clone(),
        store: None,
        contains_authority_changes: manifest.contains_authority_changes,
        blockers,
    }
}

/// The reserved memory namespace is a fixed, portable pair of files.
pub(crate) fn allowed(path: &str) -> bool {
    path_is_portable(path) && (path == MANIFEST || path == EXPORT)
}

/// Decode and return the manifest carried by a file map.
pub(crate) fn read_manifest(files: &BTreeMap<String, String>) -> Result<MemoryManifest> {
    let bytes = STANDARD
        .decode(files.get(MANIFEST).ok_or("memory_invalid")?)
        .map_err(|_| "memory_invalid".to_string())?;
    serde_json::from_slice(&bytes).map_err(|_| "memory_invalid".to_string())
}

/// Validate a captured memory map: reserved portable namespace, a well-formed
/// secret-free manifest and an export file that exactly matches the manifest's
/// recorded shape.
pub(crate) fn validate(files: &BTreeMap<String, String>) -> Result<()> {
    let mut total = 0usize;
    let mut paths: Vec<&str> = Vec::new();
    for (path, encoded) in files {
        if !allowed(path) {
            return Err("memory_unsafe".into());
        }
        let bytes = STANDARD.decode(encoded).map_err(|_| "memory_invalid")?;
        total = total.saturating_add(bytes.len());
        if total > MAX_TOTAL {
            return Err("memory_limit".into());
        }
        if let Ok(text) = std::str::from_utf8(&bytes) {
            if suspected_secret(text) {
                return Err("memory_unsafe".into());
            }
        }
        paths.push(path);
    }
    if !paths_do_not_collide(paths.into_iter()) {
        return Err("memory_unsafe".into());
    }
    let manifest = read_manifest(files)?;
    if manifest.version != SCHEMA || manifest.agent != AGENT {
        return Err("memory_invalid".into());
    }
    if !(manifest.provider.is_empty() || safe_text(&manifest.provider, 64)) {
        return Err("memory_unsafe".into());
    }
    if manifest.records > 10_000 || manifest.blockers.len() > MAX_BLOCKERS {
        return Err("memory_limit".into());
    }
    if manifest.blockers.iter().any(|b| !safe_text(b, 64)) {
        return Err("memory_unsafe".into());
    }
    if manifest
        .agent_id
        .as_deref()
        .is_some_and(|id| !safe_text(id, 200))
    {
        return Err("memory_unsafe".into());
    }
    if manifest
        .share_default
        .as_deref()
        .is_some_and(|v| !safe_text(v, 32))
    {
        return Err("memory_unsafe".into());
    }
    match (manifest.exported, files.get(EXPORT)) {
        (true, Some(encoded)) => {
            let bytes = STANDARD.decode(encoded).map_err(|_| "memory_invalid")?;
            if bytes.len() > MAX_EXPORT {
                return Err("memory_limit".into());
            }
            let text = std::str::from_utf8(&bytes).map_err(|_| "memory_invalid")?;
            let inspection = memory_adapter::inspect(text).map_err(|_| "memory_invalid")?;
            if inspection.records != manifest.records
                || inspection.contains_authority_changes != manifest.contains_authority_changes
            {
                return Err("memory_invalid".into());
            }
        }
        (false, None) => {}
        _ => return Err("memory_invalid".into()),
    }
    Ok(())
}

fn finish(manifest: MemoryManifest, export: Option<String>) -> Result<BTreeMap<String, String>> {
    let mut files: BTreeMap<String, String> = BTreeMap::new();
    let json = serde_json::to_vec(&manifest).map_err(|_| "memory_invalid")?;
    files.insert(MANIFEST.into(), STANDARD.encode(&json));
    if let Some(text) = export {
        files.insert(EXPORT.into(), STANDARD.encode(text.as_bytes()));
    }
    validate(&files)?;
    Ok(files)
}

/// Resolve the AgentMemoryOS CLI. When an app config `root` is supplied, the
/// CLI the user explicitly selected (`choose_memory_cli` → `memory-cli.json`) is
/// honoured via [`crate::amos_runtime::configured`]; otherwise the
/// autodiscovered executable is used.
#[cfg(not(feature = "mac-app-store"))]
fn resolve_cli(root: Option<&Path>) -> Option<PathBuf> {
    match root {
        Some(root) => crate::amos_runtime::configured(root).ok(),
        None => crate::amos_runtime::executable(),
    }
}
#[cfg(feature = "mac-app-store")]
fn resolve_cli(_root: Option<&Path>) -> Option<PathBuf> {
    None
}

/// Whether the AgentMemoryOS Hermes provider is discoverable from the isolated
/// home. Hermes reads `$HERMES_HOME/plugins/<name>/` and text-scans its
/// `__init__.py` for the provider contract (the same heuristic the provider's
/// own shim `agent-memory hermes install` satisfies). A missing plugin is
/// reported, never silently ready: a restored store no Hermes install can load
/// is not recall.
pub(crate) fn provider_plugin_present(managed_home: &Path) -> bool {
    let init = managed_home
        .join("plugins")
        .join(PROVIDER_AMOS)
        .join("__init__.py");
    match storage::read(&init, 16 * 1024) {
        Ok(bytes) => {
            let source = String::from_utf8_lossy(&bytes);
            source.contains("register_memory_provider") || source.contains("MemoryProvider")
        }
        Err(_) => false,
    }
}

fn capture_with(
    cli: Option<&Path>,
    home: &Path,
    settings: &Settings,
) -> Result<BTreeMap<String, String>> {
    let mut manifest = MemoryManifest::default();
    let mut export: Option<String> = None;
    match read_provider(home) {
        Provider::Builtin => manifest.provider = "builtin".into(),
        Provider::Unknown(name) => {
            manifest.provider = name;
            manifest.blockers.push("memory_provider_unknown".into());
        }
        Provider::Unresolved => {
            // Fail closed: an absent/ambiguous config is never silently built-in.
            manifest.blockers.push("memory_provider_unresolved".into());
        }
        Provider::External => {
            manifest.provider = PROVIDER_AMOS.into();
            // The Mac App Store build ships built-in memory only: the external
            // channel is withheld up front and its CLI is never invoked.
            if cfg!(feature = "mac-app-store") {
                manifest.blockers.push("memory_unavailable_store".into());
                return finish(manifest, None);
            }
            let binding = read_binding(home);
            if let Some(binding) = &binding {
                manifest.share_default = binding.share_default.clone();
                manifest.mirror_builtin = binding.mirror_builtin;
                manifest.capture_delegations = binding.capture_delegations;
                manifest.prefetch_limit = binding.prefetch_limit;
                manifest.prefetch_max_tokens = binding.prefetch_max_tokens;
            }
            manifest.agent_id = identity(binding.as_ref());
            if let Some(amos_home) = authorized_memory_home(settings, binding.as_ref()) {
                if manifest.agent_id.is_none() {
                    manifest.blockers.push("memory_identity_missing".into());
                } else if !amos_home.join("memories.db").is_file() {
                    manifest.blockers.push("memory_store_missing".into());
                } else if let Some(cli) = cli {
                    let staging = tempfile::tempdir().map_err(|_| "store_unavailable")?;
                    match crate::amos_runtime::export(cli, &amos_home, staging.path()) {
                        Ok(text) => {
                            if text.len() > MAX_EXPORT {
                                return Err("memory_limit".into());
                            }
                            let inspection =
                                memory_adapter::inspect(&text).map_err(|_| "memory_invalid")?;
                            manifest.records = inspection.records;
                            manifest.contains_authority_changes =
                                inspection.contains_authority_changes;
                            manifest.exported = true;
                            export = Some(text);
                        }
                        Err(reason) if reason == "memory_store_missing" => {
                            manifest.blockers.push("memory_store_missing".into())
                        }
                        Err(_) => manifest.blockers.push("memory_export_failed".into()),
                    }
                } else {
                    manifest.blockers.push("memory_cli_missing".into());
                }
            } else {
                // No explicit, consistent home: never an implicit private export.
                manifest.blockers.push("memory_export_unauthorized".into());
            }
        }
    }
    finish(manifest, export)
}

/// Capture the external-memory continuation surface for a Hermes home, using the
/// autodiscovered CLI (native peer signature).
pub(crate) fn capture(home: &Path, settings: &Settings) -> Result<BTreeMap<String, String>> {
    capture_with(resolve_cli(None).as_deref(), home, settings)
}

/// Capture with an app config `root` so the user-selected CLI
/// (`choose_memory_cli` → `$root/memory-cli.json`) is honoured.
pub(crate) fn capture_with_context(
    root: &Path,
    home: &Path,
    settings: &Settings,
) -> Result<BTreeMap<String, String>> {
    capture_with(resolve_cli(Some(root)).as_deref(), home, settings)
}

/// The bounded official `agent-memory sync import` into a NEW store. Mirrors
/// [`crate::amos_runtime`]'s invocation and report validation; used only for an
/// isolated destination this module just created.
#[cfg(not(feature = "mac-app-store"))]
fn import_bundle(cli: &Path, home: &Path, text: &str, staging: &Path) -> Result<()> {
    let temp = tempfile::tempdir_in(staging).map_err(|_| "store_unavailable")?;
    let path = temp.path().join("import.jsonl");
    storage::immutable(&path, text.as_bytes())?;
    let output = tempfile::NamedTempFile::new().map_err(|_| "memory_cli_failed")?;
    let mut cmd = Command::new(cli);
    // Source-checkout virtual environments may expose the entry point without an
    // editable install.
    if let Some(repo) = cli.parent().and_then(Path::parent).and_then(Path::parent) {
        let source = repo.join("src");
        if source.join("agent_memory_os/cli.py").is_file() {
            cmd.env("PYTHONPATH", source);
        }
    }
    cmd.arg("--home")
        .arg(home)
        .arg("sync")
        .arg("import")
        .arg(&path);
    let mut child = cmd
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(output.reopen().map_err(|_| "memory_cli_failed")?)
        .spawn()
        .map_err(|_| "memory_cli_missing")?;
    let deadline = Instant::now() + Duration::from_secs(180);
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|_| "memory_cli_failed")? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("memory_cli_timeout".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    if !status.success() {
        return Err("memory_import_failed".into());
    }
    let report: serde_json::Value = serde_json::from_slice(&storage::read(output.path(), 65536)?)
        .map_err(|_| "memory_import_failed")?;
    if !report["memories_added"].is_u64()
        || report.as_object().is_none_or(|r| {
            r.iter().any(|(k, v)| {
                (k.contains("rejected") || k.contains("error")) && v.as_u64() != Some(0)
            })
        })
    {
        return Err("memory_import_failed".into());
    }
    Ok(())
}
#[cfg(feature = "mac-app-store")]
fn import_bundle(_cli: &Path, _home: &Path, _text: &str, _staging: &Path) -> Result<()> {
    Err("memory_unavailable_store".into())
}

/// Whether this build ships the external AgentMemoryOS channel. Never true in
/// the sandboxed Mac App Store build, where the external CLI is withheld.
pub(crate) fn external_available() -> bool {
    !cfg!(feature = "mac-app-store")
}

/// The exact, bounded `agent-memory hermes install --hermes-home <EXACT HOME>`
/// invocation the provider itself documents. It writes the Hermes provider shim
/// into the *isolated managed home* only, never a live profile: the CLI is the
/// trusted local install the user configured (`choose_memory_cli`), no received
/// code is executed and nothing is downloaded. A single fixed operation — no
/// shell, no arbitrary arguments — run under the same wall-clock/output bound as
/// the export/import calls. Installation is proven by reading the shim back,
/// never by the CLI's exit status alone.
#[cfg(not(feature = "mac-app-store"))]
pub(crate) fn install_provider(cli: &Path, managed_home: &Path, staging: &Path) -> Result<()> {
    if managed_home.as_os_str().is_empty() || !managed_home.is_absolute() {
        return Err("memory_invalid".into());
    }
    let metadata = std::fs::symlink_metadata(managed_home).map_err(|_| "store_unavailable")?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("unsafe_store".into());
    }
    let output = tempfile::NamedTempFile::new_in(staging).map_err(|_| "store_unavailable")?;
    let mut cmd = Command::new(cli);
    // Source-checkout virtual environments may expose the entry point without an
    // editable install.
    if let Some(repo) = cli.parent().and_then(Path::parent).and_then(Path::parent) {
        let source = repo.join("src");
        if source.join("agent_memory_os/cli.py").is_file() {
            cmd.env("PYTHONPATH", source);
        }
    }
    cmd.arg("hermes")
        .arg("install")
        .arg("--hermes-home")
        .arg(managed_home);
    let mut child = cmd
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(output.reopen().map_err(|_| "memory_cli_failed")?)
        .spawn()
        .map_err(|_| "memory_cli_missing")?;
    let deadline = Instant::now() + Duration::from_secs(180);
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|_| "memory_cli_failed")? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("memory_cli_timeout".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    if !status.success() {
        return Err("memory_provider_install_failed".into());
    }
    // The shim must actually be discoverable now: a zero exit that wrote nothing
    // (or wrote elsewhere) is not an installed provider.
    if !provider_plugin_present(managed_home) {
        return Err("memory_provider_not_installed".into());
    }
    Ok(())
}
#[cfg(feature = "mac-app-store")]
pub(crate) fn install_provider(_cli: &Path, _managed_home: &Path, _staging: &Path) -> Result<()> {
    Err("memory_unavailable_store".into())
}

/// Verify the restored store by *recall*, not existence: re-export it through
/// the official CLI and require the record count, authority flags and normalized
/// fingerprint to match the captured export. A `memories.db` that merely exists
/// (or is a truncated/foreign file) fails this and is rolled back.
#[cfg(not(feature = "mac-app-store"))]
fn verify_restored_store(
    cli: &Path,
    nested: &Path,
    staging: &Path,
    manifest: &MemoryManifest,
    captured: &str,
) -> Result<()> {
    let text =
        crate::amos_runtime::export(cli, nested, staging).map_err(|_| "memory_import_failed")?;
    let inspection = memory_adapter::inspect(&text).map_err(|_| "memory_import_failed")?;
    if inspection.records != manifest.records
        || inspection.contains_authority_changes != manifest.contains_authority_changes
        || crate::amos_runtime::fingerprint(&text).map_err(|_| "memory_import_failed")?
            != crate::amos_runtime::fingerprint(captured).map_err(|_| "memory_import_failed")?
    {
        return Err("memory_import_failed".into());
    }
    Ok(())
}
#[cfg(feature = "mac-app-store")]
fn verify_restored_store(
    _cli: &Path,
    _nested: &Path,
    _staging: &Path,
    _manifest: &MemoryManifest,
    _captured: &str,
) -> Result<()> {
    Err("memory_unavailable_store".into())
}

fn restore_with(
    cli: Option<&Path>,
    files: &BTreeMap<String, String>,
    managed_home: &Path,
    settings: &Settings,
) -> Result<MemoryReadiness> {
    validate(files)?;
    let manifest = read_manifest(files)?;
    if manifest.provider != PROVIDER_AMOS {
        // Built-in memory travels as workspace files; nothing external to import.
        let ready = manifest.blockers.is_empty() && manifest.provider == "builtin";
        return Ok(MemoryReadiness {
            provider: manifest.provider.clone(),
            ready,
            exported: false,
            imported: false,
            records: 0,
            agent_id: manifest.agent_id.clone(),
            store: None,
            contains_authority_changes: false,
            blockers: manifest.blockers.clone(),
        });
    }
    if !crate::model::agent_available(PROVIDER_AMOS) {
        return Ok(not_ready(&manifest, "memory_unavailable_store"));
    }
    // The recipient must have opted into the external-memory source. Authority
    // changes are permitted only inside the NEW isolated store this creates; the
    // existing approval/selection policy still governs any live application,
    // which this path never performs.
    if !settings.selected_agents.iter().any(|a| a == PROVIDER_AMOS)
        && !settings.custom_paths.contains_key(PROVIDER_AMOS)
    {
        return Ok(not_ready(&manifest, "memory_export_unauthorized"));
    }
    if !manifest.blockers.is_empty() {
        return Ok(not_ready(&manifest, "memory_export_unauthorized"));
    }
    let Some(agent_id) = manifest.agent_id.clone() else {
        return Ok(not_ready(&manifest, "memory_identity_missing"));
    };
    let Some(cli) = cli else {
        return Ok(not_ready(&manifest, "memory_cli_missing"));
    };
    let Some(encoded) = files.get(EXPORT) else {
        return Ok(not_ready(&manifest, "memory_export_unauthorized"));
    };
    let text = String::from_utf8(STANDARD.decode(encoded).map_err(|_| "memory_invalid")?)
        .map_err(|_| "memory_invalid".to_string())?;
    if managed_home.as_os_str().is_empty() || !managed_home.is_absolute() {
        return Err("memory_invalid".into());
    }
    let staging = tempfile::tempdir().map_err(|_| "store_unavailable")?;
    // A provider plugin Hermes cannot discover from the isolated home is not
    // recall; report it and place nothing so a later install can retry.
    if !provider_plugin_present(managed_home) {
        return Ok(not_ready(&manifest, "memory_provider_not_installed"));
    }
    let nested = managed_home.join(MANAGED_DIR);
    let binding = serde_json::json!({
        "home": nested.to_string_lossy(),
        "agent_id": agent_id,
        "share_default": manifest.share_default,
        "mirror_builtin": manifest.mirror_builtin,
        "capture_delegations": manifest.capture_delegations,
        "prefetch_limit": manifest.prefetch_limit,
        "prefetch_max_tokens": manifest.prefetch_max_tokens,
    });
    let binding_bytes = serde_json::to_vec_pretty(&binding).map_err(|_| "memory_invalid")?;
    // An already-present store is never replaced. If it still recalls the
    // captured snapshot the restore is already done (idempotent repeated sync);
    // if it has diverged (the recipient changed memory) the stale baseline is
    // reported, never re-imported over the newer store.
    if nested.join("memories.db").is_file() {
        if verify_restored_store(cli, &nested, staging.path(), &manifest, &text).is_ok() {
            storage::replace(&managed_home.join(PROVIDER_CONFIG), &binding_bytes)?;
            return Ok(MemoryReadiness {
                provider: PROVIDER_AMOS.into(),
                ready: true,
                exported: true,
                imported: false,
                records: manifest.records,
                agent_id: Some(agent_id),
                store: Some(MANAGED_DIR.into()),
                contains_authority_changes: manifest.contains_authority_changes,
                blockers: Vec::new(),
            });
        }
        return Ok(not_ready(&manifest, "memory_store_diverged"));
    }
    // A destination that exists but carries no verifiable store (a leftover or a
    // decoy) is refused, never replaced.
    if nested.exists() {
        return Err("restore_destination_exists".into());
    }
    // Atomic reservation of a brand-new store.
    match std::fs::create_dir(&nested) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err("restore_destination_exists".into());
        }
        Err(_) => return Err("store_unavailable".into()),
    }
    if let Err(reason) = import_bundle(cli, &nested, &text, staging.path()) {
        let _ = std::fs::remove_dir_all(&nested);
        return Err(reason);
    }
    // Do not trust the provider's availability flag or a bare file: verify recall
    // (re-export) before reporting ready, and roll the isolated store back if the
    // imported bytes do not recall the captured snapshot.
    if let Err(reason) = verify_restored_store(cli, &nested, staging.path(), &manifest, &text) {
        let _ = std::fs::remove_dir_all(&nested);
        return Err(reason);
    }
    if let Err(reason) = storage::replace(&managed_home.join(PROVIDER_CONFIG), &binding_bytes) {
        let _ = std::fs::remove_dir_all(&nested);
        return Err(reason);
    }
    Ok(MemoryReadiness {
        provider: PROVIDER_AMOS.into(),
        ready: true,
        exported: true,
        imported: true,
        records: manifest.records,
        agent_id: Some(agent_id),
        store: Some(MANAGED_DIR.into()),
        contains_authority_changes: manifest.contains_authority_changes,
        blockers: Vec::new(),
    })
}

/// Restore a captured memory map into a NEW nested store inside `managed_home`,
/// using the autodiscovered CLI (native peer signature).
pub(crate) fn restore(
    files: &BTreeMap<String, String>,
    managed_home: &Path,
    settings: &Settings,
) -> Result<MemoryReadiness> {
    restore_with(resolve_cli(None).as_deref(), files, managed_home, settings)
}

/// Restore with an app config `root` so the user-selected CLI
/// (`choose_memory_cli` → `$root/memory-cli.json`) is honoured.
pub(crate) fn restore_with_context(
    root: &Path,
    files: &BTreeMap<String, String>,
    managed_home: &Path,
    settings: &Settings,
) -> Result<MemoryReadiness> {
    restore_with(
        resolve_cli(Some(root)).as_deref(),
        files,
        managed_home,
        settings,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two memories plus a `team` authority record. Written into every synthetic
    /// source store and echoed verbatim by the fixture CLI.
    const FAKE_RECORDS: &str = r#"{"kind":"memory","id":"alpha","owner":"hermes-fixture","content":"ALPHA-FIXTURE","visibility":"[\"global\"]"}
{"kind":"memory","id":"beta","owner":"hermes-fixture","content":"BETA-FIXTURE","visibility":"[\"global\"]"}
{"kind":"team","id":"team-fixture","members":["agent-a"]}
"#;

    /// A content-faithful fixture CLI. `export` echoes the records imported into
    /// the home (so recall reflects the store, not a fixed blob); `import` copies
    /// the bundle body and appends to an import counter so a re-import is
    /// observable.
    const FAKE_CLI: &str = r#"#!/bin/sh
home=""
action=""
target=""
while [ $# -gt 0 ]; do
  case "$1" in
    --home) home="$2"; shift 2 ;;
    sync) action="$2"; target="$3"; shift 3 ;;
    backup) action="backup"; target="$2"; shift 2 ;;
    *) shift ;;
  esac
done
case "$action" in
  export)
    { printf '%s\n' '{"kind":"bundle","version":3}'; cat "$home/records.jsonl" 2>/dev/null; } > "$target"
    ;;
  import)
    mkdir -p "$home"
    tail -n +2 "$target" > "$home/records.jsonl"
    { printf '%s\n' 'SQLite format 3 fake-store'; cat "$target"; } > "$home/memories.db"
    printf 'x\n' >> "$home/imports.count"
    n=$(grep -c '"kind":"memory"' "$target" 2>/dev/null); [ -n "$n" ] || n=0
    printf '{"memories_added": %s, "memories_updated": 0, "memories_skipped": 0, "links_added": 0, "links_merged": 0, "profiles_upserted": 0, "tombstones_applied": 0, "teams_upserted": 0, "projects_upserted": 0, "org_tombstones_applied": 0, "org_records_rejected": 0}\n' "$n"
    ;;
  backup)
    cp "$home/memories.db" "$target"
    ;;
  *)
    echo "unsupported" >&2
    exit 2
    ;;
esac
"#;

    /// A fixture CLI that imports "successfully" but whose store exports a
    /// *different* recall (one memory, no authority records), so the restored
    /// bytes must be rejected.
    const BAD_RESTORE_CLI: &str = r#"#!/bin/sh
home=""
action=""
target=""
while [ $# -gt 0 ]; do
  case "$1" in
    --home) home="$2"; shift 2 ;;
    sync) action="$2"; target="$3"; shift 3 ;;
    *) shift ;;
  esac
done
case "$action" in
  export)
    { printf '%s\n' '{"kind":"bundle","version":3}'; printf '%s\n' '{"kind":"memory","id":"other","owner":"hermes-fixture","content":"OTHER-FIXTURE","visibility":"[\"global\"]"}'; } > "$target"
    ;;
  import)
    mkdir -p "$home"
    { printf '%s\n' 'SQLite format 3 fake-store'; cat "$target"; } > "$home/memories.db"
    printf '{"memories_added": 3, "memories_updated": 0}\n'
    ;;
  *)
    echo "unsupported" >&2
    exit 2
    ;;
esac
"#;

    fn write(root: &Path, name: &str, text: &str) {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn decode(files: &BTreeMap<String, String>, key: &str) -> Vec<u8> {
        STANDARD.decode(files.get(key).expect(key)).unwrap()
    }

    fn manifest_of(files: &BTreeMap<String, String>) -> MemoryManifest {
        read_manifest(files).unwrap()
    }

    fn amos_settings(memory_home: &Path) -> Settings {
        let mut custom_paths: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        custom_paths.insert(
            PROVIDER_AMOS.to_string(),
            memory_home.to_string_lossy().into_owned(),
        );
        Settings {
            selected_agents: vec![PROVIDER_AMOS.to_string()],
            custom_paths,
            ..Settings::default()
        }
    }

    #[cfg(unix)]
    fn write_cli(dir: &Path, name: &str, script: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[cfg(unix)]
    fn fake_cli(dir: &Path) -> PathBuf {
        write_cli(dir, "agent-memory", FAKE_CLI)
    }

    /// Materialize the AgentMemoryOS Hermes provider shim in an isolated home,
    /// matching what `agent-memory hermes install --hermes-home <home>` writes
    /// (a `plugins/agent-memory-os/__init__.py` carrying the provider contract).
    fn plugin_shim(managed_home: &Path) {
        let dir = managed_home.join("plugins").join(PROVIDER_AMOS);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("__init__.py"),
            "from agent_memory_os.hermes_plugin import AgentMemoryOSProvider\n\
             def register(ctx):\n    ctx.register_memory_provider(AgentMemoryOSProvider())\n",
        )
        .unwrap();
    }

    /// A source Hermes home whose provider is AgentMemoryOS and whose memory
    /// home carries a synthetic store.
    fn amos_source(root: &Path, memory_home: &Path) -> PathBuf {
        let home = root.join("hermes-home");
        std::fs::create_dir_all(&home).unwrap();
        write(
            &home,
            "config.yaml",
            "memory:\n  provider: \"agent-memory-os\"\ndisplay:\n  skin: default\napi_key: LEAK\n",
        );
        write(
            &home,
            "agent-memory-os.json",
            &format!(
                "{{\"home\": \"{}\", \"agent_id\": \"hermes-fixture\", \"share_default\": \"private\", \"prefetch_limit\": 8}}",
                memory_home.to_string_lossy()
            ),
        );
        // An unrelated credential file that must never travel.
        write(&home, "secrets.txt", "sk-ant-SECRET-should-never-travel\n");
        home
    }

    fn synthetic_store(dir: &Path) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("memories.db"), b"SQLite format 3\0SOURCE-STORE").unwrap();
        std::fs::write(dir.join("records.jsonl"), FAKE_RECORDS).unwrap();
        dir.to_path_buf()
    }

    #[test]
    fn allowed_accepts_only_the_reserved_memory_namespace() {
        assert!(allowed(MANIFEST));
        assert!(allowed(EXPORT));
        for bad in [
            "",
            "bastet",
            "bastet/memory-manifest.json.bak",
            "bastet/other.json",
            "bastet/memory-export.jsonl/child",
            "SOUL.md",
            "state.db",
            "../escape",
            "/etc/passwd",
            "bastet\\memory-manifest.json",
        ] {
            assert!(!allowed(bad), "should reject {bad:?}");
        }
    }

    #[test]
    fn builtin_provider_needs_no_cli_and_is_ready() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "config.yaml", "memory:\n  provider: builtin\n");
        let files = capture_with(None, root.path(), &Settings::default()).unwrap();
        validate(&files).unwrap();
        let manifest = manifest_of(&files);
        assert_eq!(manifest.provider, "builtin");
        assert!(!manifest.exported);
        assert!(manifest.blockers.is_empty());
        assert!(!files.contains_key(EXPORT));
        let readiness = restore_with(None, &files, root.path(), &Settings::default()).unwrap();
        assert!(readiness.ready);
        assert!(!readiness.imported);
    }

    #[test]
    fn empty_config_is_the_documented_builtin_default() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "config.yaml", "");
        let manifest = manifest_of(&capture_with(None, root.path(), &Settings::default()).unwrap());
        assert_eq!(manifest.provider, "builtin");
        assert!(manifest.blockers.is_empty());
    }

    #[test]
    fn unknown_provider_is_blocked() {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "config.yaml",
            "memory:\n  provider: \"honcho\"\n",
        );
        let files = capture_with(None, root.path(), &Settings::default()).unwrap();
        validate(&files).unwrap();
        let manifest = manifest_of(&files);
        assert_eq!(manifest.provider, "honcho");
        assert!(manifest
            .blockers
            .contains(&"memory_provider_unknown".to_string()));
        let readiness = restore_with(None, &files, root.path(), &Settings::default()).unwrap();
        assert!(!readiness.ready);
    }

    #[test]
    fn missing_config_fails_closed_and_is_never_silently_builtin() {
        let root = tempfile::tempdir().unwrap();
        let manifest = manifest_of(&capture_with(None, root.path(), &Settings::default()).unwrap());
        assert_eq!(manifest.provider, "");
        assert_ne!(manifest.provider, "builtin");
        assert!(manifest
            .blockers
            .contains(&"memory_provider_unresolved".to_string()));
    }

    #[test]
    fn malformed_config_fails_closed() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "config.yaml", "memory: [unterminated\n");
        let manifest = manifest_of(&capture_with(None, root.path(), &Settings::default()).unwrap());
        assert!(manifest
            .blockers
            .contains(&"memory_provider_unresolved".to_string()));
    }

    /// The Mac App Store build withholds the external AgentMemoryOS channel up
    /// front: capture reports `memory_unavailable_store`, carries no export and
    /// never invokes a CLI, and restoring that manifest places no store.
    #[cfg(feature = "mac-app-store")]
    #[test]
    fn store_withholds_external_memory_without_invoking_a_cli() {
        let root = tempfile::tempdir().unwrap();
        let store = synthetic_store(&root.path().join("store"));
        let home = amos_source(root.path(), &store);
        let settings = amos_settings(&store);
        // A bogus CLI path proves the external channel is withheld before any
        // process is spawned: capture still blocks and exports nothing.
        let bogus = root.path().join("not-a-real-agent-memory");
        let files = capture_with(Some(&bogus), &home, &settings).unwrap();
        validate(&files).unwrap();
        let manifest = manifest_of(&files);
        assert_eq!(manifest.provider, PROVIDER_AMOS);
        assert!(!manifest.exported);
        assert_eq!(manifest.records, 0);
        assert!(!files.contains_key(EXPORT));
        assert!(manifest
            .blockers
            .contains(&"memory_unavailable_store".to_string()));

        let managed = root.path().join("managed");
        std::fs::create_dir_all(&managed).unwrap();
        let readiness = restore_with(None, &files, &managed, &settings).unwrap();
        assert!(!readiness.ready && !readiness.imported && !readiness.exported);
        assert!(readiness
            .blockers
            .contains(&"memory_unavailable_store".to_string()));
        assert!(!managed.join(MANAGED_DIR).exists());
    }

    /// Built-in file memory is a local, first-class provider and stays available
    /// in the Mac App Store build: capture and restore keep working with no
    /// export, no import and no blockers.
    #[cfg(feature = "mac-app-store")]
    #[test]
    fn store_keeps_builtin_memory_available() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "config.yaml", "memory:\n  provider: builtin\n");
        let files = capture_with(None, root.path(), &Settings::default()).unwrap();
        validate(&files).unwrap();
        let manifest = manifest_of(&files);
        assert_eq!(manifest.provider, "builtin");
        assert!(!manifest.exported);
        assert!(manifest.blockers.is_empty());
        assert!(!files.contains_key(EXPORT));

        let readiness = restore_with(None, &files, root.path(), &Settings::default()).unwrap();
        assert!(readiness.ready && !readiness.imported && !readiness.exported);
        assert!(readiness.blockers.is_empty());
    }

    #[cfg(not(feature = "mac-app-store"))]
    #[test]
    fn external_provider_without_explicit_home_is_unauthorized() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let saved = std::env::var_os("AGENT_MEMORY_HOME");
        std::env::remove_var("AGENT_MEMORY_HOME");
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "config.yaml",
            "memory:\n  provider: \"agent-memory-os\"\n",
        );
        let files = capture_with(None, root.path(), &Settings::default()).unwrap();
        let manifest = manifest_of(&files);
        assert_eq!(manifest.provider, PROVIDER_AMOS);
        assert!(!manifest.exported);
        assert!(!files.contains_key(EXPORT));
        assert!(manifest
            .blockers
            .contains(&"memory_export_unauthorized".to_string()));
        if let Some(value) = saved {
            std::env::set_var("AGENT_MEMORY_HOME", value);
        } else {
            std::env::remove_var("AGENT_MEMORY_HOME");
        }
    }

    #[cfg(not(feature = "mac-app-store"))]
    #[test]
    fn conflicting_explicit_homes_are_unauthorized() {
        let root = tempfile::tempdir().unwrap();
        let a = synthetic_store(&root.path().join("store-a"));
        let b = synthetic_store(&root.path().join("store-b"));
        let home = amos_source(root.path(), &a);
        let settings = amos_settings(&b);
        let manifest = manifest_of(&capture_with(None, &home, &settings).unwrap());
        assert!(!manifest.exported);
        assert!(manifest
            .blockers
            .contains(&"memory_export_unauthorized".to_string()));
    }

    #[cfg(not(feature = "mac-app-store"))]
    #[test]
    fn external_provider_missing_store_is_an_explicit_blocker() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("absent-store");
        let home = amos_source(root.path(), &missing);
        let settings = amos_settings(&missing);
        let manifest = manifest_of(&capture_with(None, &home, &settings).unwrap());
        assert_eq!(manifest.provider, PROVIDER_AMOS);
        assert!(!manifest.exported);
        assert!(manifest
            .blockers
            .contains(&"memory_store_missing".to_string()));
    }

    #[cfg(not(feature = "mac-app-store"))]
    #[test]
    fn external_provider_missing_identity_is_an_explicit_blocker() {
        let root = tempfile::tempdir().unwrap();
        let store = synthetic_store(&root.path().join("store"));
        let home = root.path().join("hermes-home");
        std::fs::create_dir_all(&home).unwrap();
        write(
            &home,
            "config.yaml",
            "memory:\n  provider: \"agent-memory-os\"\n",
        );
        write(
            &home,
            "agent-memory-os.json",
            &format!("{{\"home\": \"{}\"}}", store.to_string_lossy()),
        );
        let settings = amos_settings(&store);
        let manifest = manifest_of(&capture_with(None, &home, &settings).unwrap());
        assert!(!manifest.exported);
        assert!(manifest.agent_id.is_none());
        assert!(manifest
            .blockers
            .contains(&"memory_identity_missing".to_string()));
    }

    #[cfg(all(unix, not(feature = "mac-app-store")))]
    #[test]
    fn external_provider_missing_cli_is_an_explicit_blocker() {
        let root = tempfile::tempdir().unwrap();
        let store = synthetic_store(&root.path().join("store"));
        let home = amos_source(root.path(), &store);
        let settings = amos_settings(&store);
        let manifest = manifest_of(&capture_with(None, &home, &settings).unwrap());
        assert!(!manifest.exported);
        assert!(manifest
            .blockers
            .contains(&"memory_cli_missing".to_string()));
    }

    #[cfg(all(unix, not(feature = "mac-app-store")))]
    #[test]
    fn external_capture_uses_official_export_and_strips_local_source() {
        let root = tempfile::tempdir().unwrap();
        let store = synthetic_store(&root.path().join("store"));
        let home = amos_source(root.path(), &store);
        let cli = fake_cli(root.path());
        let settings = amos_settings(&store);
        let files = capture_with(Some(&cli), &home, &settings).unwrap();
        validate(&files).unwrap();

        let manifest = manifest_of(&files);
        assert_eq!(manifest.provider, PROVIDER_AMOS);
        assert!(manifest.exported);
        // Two memories plus the team record the official export carries.
        assert_eq!(manifest.records, 3);
        assert!(manifest.contains_authority_changes);
        assert_eq!(manifest.agent_id.as_deref(), Some("hermes-fixture"));
        assert_eq!(manifest.share_default.as_deref(), Some("private"));
        assert!(manifest.blockers.is_empty());

        // The official export survives verbatim: content and ACL untouched.
        let export = String::from_utf8(decode(&files, EXPORT)).unwrap();
        assert!(export.contains("ALPHA-FIXTURE") && export.contains("BETA-FIXTURE"));
        assert!(export.contains("\"visibility\":\"[\\\"global\\\"]\""));
        assert!(export.starts_with("{\"kind\":\"bundle\",\"version\":3}"));

        // Portable metadata carries no local source home and no credential.
        let manifest_bytes = String::from_utf8(decode(&files, MANIFEST)).unwrap();
        assert!(!manifest_bytes.contains(&home.to_string_lossy().into_owned()));
        assert!(!manifest_bytes.contains(&store.to_string_lossy().into_owned()));
        for encoded in files.values() {
            let bytes = STANDARD.decode(encoded).unwrap();
            let text = String::from_utf8_lossy(&bytes);
            assert!(!text.contains("sk-ant-"), "{text}");
            assert!(!text.contains("SECRET-should-never-travel"));
        }
        // The source store itself is untouched.
        assert_eq!(
            std::fs::read(store.join("memories.db")).unwrap(),
            b"SQLite format 3\0SOURCE-STORE"
        );
    }

    #[test]
    fn validate_rejects_tampered_maps() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "config.yaml", "memory:\n  provider: builtin\n");
        let files = capture_with(None, root.path(), &Settings::default()).unwrap();
        validate(&files).unwrap();

        // An undeclared export file.
        let mut extra = files.clone();
        extra.insert(
            EXPORT.into(),
            STANDARD.encode("{\"kind\":\"bundle\",\"version\":3}\n"),
        );
        assert_eq!(validate(&extra).unwrap_err(), "memory_invalid");

        // Declared export with no file.
        let mut manifest = manifest_of(&files);
        manifest.exported = true;
        manifest.records = 1;
        let mut missing = BTreeMap::new();
        missing.insert(
            MANIFEST.into(),
            STANDARD.encode(serde_json::to_vec(&manifest).unwrap()),
        );
        assert_eq!(validate(&missing).unwrap_err(), "memory_invalid");

        // A path outside the namespace.
        let mut foreign = files.clone();
        foreign.insert("bastet/evil.json".into(), STANDARD.encode("x"));
        assert_eq!(validate(&foreign).unwrap_err(), "memory_unsafe");

        // A credential smuggled into the manifest.
        let mut manifest = manifest_of(&files);
        manifest.agent_id = Some("sk-ant-LEAK".into());
        let mut tainted = files.clone();
        tainted.insert(
            MANIFEST.into(),
            STANDARD.encode(serde_json::to_vec(&manifest).unwrap()),
        );
        assert_eq!(validate(&tainted).unwrap_err(), "memory_unsafe");

        // Empty map is not a valid capture.
        assert_eq!(validate(&BTreeMap::new()).unwrap_err(), "memory_invalid");
    }

    #[cfg(all(unix, not(feature = "mac-app-store")))]
    #[test]
    fn validate_cross_checks_export_shape() {
        let root = tempfile::tempdir().unwrap();
        let store = synthetic_store(&root.path().join("store"));
        let home = amos_source(root.path(), &store);
        let cli = fake_cli(root.path());
        let settings = amos_settings(&store);
        let files = capture_with(Some(&cli), &home, &settings).unwrap();

        let mut manifest = manifest_of(&files);
        manifest.records = 99;
        let mut tampered = files.clone();
        tampered.insert(
            MANIFEST.into(),
            STANDARD.encode(serde_json::to_vec(&manifest).unwrap()),
        );
        assert_eq!(validate(&tampered).unwrap_err(), "memory_invalid");

        let mut manifest = manifest_of(&files);
        manifest.contains_authority_changes = false;
        let mut tampered = files.clone();
        tampered.insert(
            MANIFEST.into(),
            STANDARD.encode(serde_json::to_vec(&manifest).unwrap()),
        );
        assert_eq!(validate(&tampered).unwrap_err(), "memory_invalid");
    }

    #[cfg(all(unix, not(feature = "mac-app-store")))]
    #[test]
    fn restore_imports_into_new_store_and_binds_original_identity() {
        let root = tempfile::tempdir().unwrap();
        let store = synthetic_store(&root.path().join("store"));
        let home = amos_source(root.path(), &store);
        let cli = fake_cli(root.path());
        let settings = amos_settings(&store);
        let files = capture_with(Some(&cli), &home, &settings).unwrap();

        let managed = root.path().join("managed");
        std::fs::create_dir_all(&managed).unwrap();
        plugin_shim(&managed);
        let source_db = std::fs::read(store.join("memories.db")).unwrap();

        let readiness = restore_with(Some(&cli), &files, &managed, &settings).unwrap();
        assert!(readiness.ready && readiness.imported && readiness.exported);
        assert_eq!(readiness.provider, PROVIDER_AMOS);
        assert_eq!(readiness.records, 3);
        assert_eq!(readiness.agent_id.as_deref(), Some("hermes-fixture"));
        assert_eq!(readiness.store.as_deref(), Some(MANAGED_DIR));
        assert!(readiness.contains_authority_changes);

        // The imported store really exists and carries the recalled content/ACL.
        let restored = managed.join(MANAGED_DIR).join("memories.db");
        let restored_bytes = std::fs::read(&restored).unwrap();
        let text = String::from_utf8_lossy(&restored_bytes);
        assert!(text.contains("ALPHA-FIXTURE") && text.contains("BETA-FIXTURE"));
        assert!(text.contains("\"visibility\":\"[\\\"global\\\"]\""));

        // The local binding keeps the original identity but the NEW home.
        let binding = std::fs::read_to_string(managed.join(PROVIDER_CONFIG)).unwrap();
        assert!(binding.contains("hermes-fixture"));
        assert!(binding.contains(&managed.join(MANAGED_DIR).to_string_lossy().into_owned()));
        assert!(!binding.contains(&store.to_string_lossy().into_owned()));
        assert!(!binding.contains("sk-ant-"));

        // The live source store is unchanged; a repeated restore of the SAME
        // snapshot is idempotent (recalls the existing store, no re-import).
        assert_eq!(std::fs::read(store.join("memories.db")).unwrap(), source_db);
        let again = restore_with(Some(&cli), &files, &managed, &settings).unwrap();
        assert!(again.ready && !again.imported);
    }

    #[cfg(all(unix, not(feature = "mac-app-store")))]
    #[test]
    fn restore_refuses_to_replace_a_concurrent_empty_destination() {
        let root = tempfile::tempdir().unwrap();
        let store = synthetic_store(&root.path().join("store"));
        let home = amos_source(root.path(), &store);
        let cli = fake_cli(root.path());
        let settings = amos_settings(&store);
        let files = capture_with(Some(&cli), &home, &settings).unwrap();

        let managed = root.path().join("managed");
        std::fs::create_dir_all(&managed).unwrap();
        plugin_shim(&managed);
        std::fs::create_dir_all(managed.join(MANAGED_DIR)).unwrap();
        assert_eq!(
            restore_with(Some(&cli), &files, &managed, &settings).unwrap_err(),
            "restore_destination_exists"
        );
        assert!(std::fs::read_dir(managed.join(MANAGED_DIR))
            .unwrap()
            .next()
            .is_none());
    }

    #[cfg(all(unix, not(feature = "mac-app-store")))]
    #[test]
    fn restore_requires_the_recipient_to_have_opted_in() {
        let root = tempfile::tempdir().unwrap();
        let store = synthetic_store(&root.path().join("store"));
        let home = amos_source(root.path(), &store);
        let cli = fake_cli(root.path());
        let files = capture_with(Some(&cli), &home, &amos_settings(&store)).unwrap();

        let managed = root.path().join("managed");
        std::fs::create_dir_all(&managed).unwrap();
        let readiness = restore_with(Some(&cli), &files, &managed, &Settings::default()).unwrap();
        assert!(!readiness.ready);
        assert!(readiness
            .blockers
            .contains(&"memory_export_unauthorized".to_string()));
        assert!(!managed.join(MANAGED_DIR).exists());
    }

    #[cfg(all(unix, not(feature = "mac-app-store")))]
    #[test]
    fn restore_without_cli_or_export_is_not_ready() {
        let root = tempfile::tempdir().unwrap();
        let store = synthetic_store(&root.path().join("store"));
        let home = amos_source(root.path(), &store);
        let cli = fake_cli(root.path());
        let settings = amos_settings(&store);
        let files = capture_with(Some(&cli), &home, &settings).unwrap();

        let managed = root.path().join("managed");
        std::fs::create_dir_all(&managed).unwrap();
        let readiness = restore_with(None, &files, &managed, &settings).unwrap();
        assert!(!readiness.ready);
        assert!(readiness
            .blockers
            .contains(&"memory_cli_missing".to_string()));
        assert!(!managed.join(MANAGED_DIR).exists());
    }

    #[cfg(all(unix, not(feature = "mac-app-store")))]
    #[ignore = "requires an installed AMOS CLI; runs only against isolated synthetic scratch stores"]
    #[test]
    fn installed_cli_round_trip_uses_synthetic_scratch_store() {
        let cli =
            PathBuf::from(std::env::var_os("BASTET_TEST_MEMORY_CLI").expect("test CLI required"));
        let fixture = include_str!("../../tests/fixtures/amos-v3.jsonl");
        let root = tempfile::tempdir().unwrap();
        let store = root.path().join("store");
        std::fs::create_dir_all(&store).unwrap();
        let staging = tempfile::tempdir().unwrap();
        // Bootstrap a synthetic source store with the official importer.
        import_bundle(&cli, &store, fixture, staging.path()).unwrap();

        let home = amos_source(root.path(), &store);
        let settings = amos_settings(&store);
        let files = capture_with(Some(&cli), &home, &settings).unwrap();
        validate(&files).unwrap();
        let manifest = manifest_of(&files);
        assert!(manifest.exported && manifest.contains_authority_changes);

        let managed = root.path().join("managed");
        std::fs::create_dir_all(&managed).unwrap();
        plugin_shim(&managed);
        let readiness = restore_with(Some(&cli), &files, &managed, &settings).unwrap();
        assert!(readiness.ready && readiness.imported);
        assert!(managed.join(MANAGED_DIR).join("memories.db").is_file());

        // Re-exporting the restored store preserves the recalled content and the
        // ACL/authority records, and no credential was introduced anywhere.
        let restored_text =
            crate::amos_runtime::export(&cli, &managed.join(MANAGED_DIR), staging.path()).unwrap();
        assert!(restored_text.contains("Legacy A") && restored_text.contains("Legacy B"));
        assert!(restored_text.contains("\"kind\":\"team\"") || restored_text.contains("legacy-v3"));
        assert_eq!(
            crate::amos_runtime::fingerprint(&restored_text).unwrap(),
            crate::amos_runtime::fingerprint(&String::from_utf8(decode(&files, EXPORT)).unwrap())
                .unwrap()
        );
    }

    /// The context API must honour the CLI the user selected in the app config
    /// (`choose_memory_cli` → `memory-cli.json`) instead of ambient PATH/venv
    /// autodiscovery, for both capture and restore.
    #[cfg(all(unix, not(feature = "mac-app-store")))]
    #[test]
    fn context_api_uses_the_configured_cli_over_path_discovery() {
        let root = tempfile::tempdir().unwrap();
        let store = synthetic_store(&root.path().join("store"));
        let home = amos_source(root.path(), &store);
        let settings = amos_settings(&store);
        let cli = fake_cli(root.path());
        let app_root = tempfile::tempdir().unwrap();
        write(
            app_root.path(),
            "memory-cli.json",
            &serde_json::to_string(&cli).unwrap(),
        );

        // The configured CLI (reachable only through the app root) drives capture.
        let files = capture_with_context(app_root.path(), &home, &settings).unwrap();
        let manifest = manifest_of(&files);
        assert!(manifest.exported && manifest.blockers.is_empty());

        // With no configured CLI the same context falls back to autodiscovery,
        // which cannot read this synthetic store: the configured path is what made
        // capture succeed.
        let bare_root = tempfile::tempdir().unwrap();
        let fallback =
            manifest_of(&capture_with_context(bare_root.path(), &home, &settings).unwrap());
        assert!(!fallback.exported);

        // ...and the same configured CLI drives restore.
        let managed = root.path().join("managed");
        std::fs::create_dir_all(&managed).unwrap();
        plugin_shim(&managed);
        let readiness = restore_with_context(app_root.path(), &files, &managed, &settings).unwrap();
        assert!(readiness.ready && readiness.imported);
    }

    /// Readiness is recall, not file presence: a store that imported "successfully"
    /// but cannot re-export the captured records must be rejected and rolled back.
    #[cfg(all(unix, not(feature = "mac-app-store")))]
    #[test]
    fn restore_rejects_a_store_that_does_not_recall_the_captured_records() {
        let root = tempfile::tempdir().unwrap();
        let store = synthetic_store(&root.path().join("store"));
        let home = amos_source(root.path(), &store);
        let settings = amos_settings(&store);
        let good = fake_cli(root.path());
        let files = capture_with(Some(&good), &home, &settings).unwrap();

        let managed = root.path().join("managed");
        std::fs::create_dir_all(&managed).unwrap();
        plugin_shim(&managed);
        let bad = write_cli(root.path(), "agent-memory-bad", BAD_RESTORE_CLI);
        assert_eq!(
            restore_with(Some(&bad), &files, &managed, &settings).unwrap_err(),
            "memory_import_failed"
        );
        assert!(!managed.join(MANAGED_DIR).exists());
    }

    /// Repeated sync of the SAME snapshot is idempotent: the second restore recalls
    /// the existing store and does not re-import.
    #[cfg(all(unix, not(feature = "mac-app-store")))]
    #[test]
    fn restore_same_snapshot_is_idempotent_and_does_not_reimport() {
        let root = tempfile::tempdir().unwrap();
        let store = synthetic_store(&root.path().join("store"));
        let home = amos_source(root.path(), &store);
        let cli = fake_cli(root.path());
        let settings = amos_settings(&store);
        let files = capture_with(Some(&cli), &home, &settings).unwrap();

        let managed = root.path().join("managed");
        std::fs::create_dir_all(&managed).unwrap();
        plugin_shim(&managed);

        let first = restore_with(Some(&cli), &files, &managed, &settings).unwrap();
        assert!(first.ready && first.imported);

        let second = restore_with(Some(&cli), &files, &managed, &settings).unwrap();
        assert!(
            second.ready && !second.imported,
            "repeat sync must be idempotent"
        );

        let nested = managed.join(MANAGED_DIR);
        let imports = std::fs::read_to_string(nested.join("imports.count")).unwrap();
        assert_eq!(imports.lines().count(), 1, "only the first restore imports");
    }

    /// A store the recipient has changed must never be overwritten by a stale
    /// baseline: the divergence is reported, not force-merged.
    #[cfg(all(unix, not(feature = "mac-app-store")))]
    #[test]
    fn restore_never_overwrites_a_diverged_store() {
        let root = tempfile::tempdir().unwrap();
        let store = synthetic_store(&root.path().join("store"));
        let home = amos_source(root.path(), &store);
        let cli = fake_cli(root.path());
        let settings = amos_settings(&store);
        let files = capture_with(Some(&cli), &home, &settings).unwrap();

        let managed = root.path().join("managed");
        std::fs::create_dir_all(&managed).unwrap();
        plugin_shim(&managed);
        assert!(
            restore_with(Some(&cli), &files, &managed, &settings)
                .unwrap()
                .imported
        );

        // The recipient adds a memory after the restore: the store has diverged.
        let nested = managed.join(MANAGED_DIR);
        let mut records = std::fs::read_to_string(nested.join("records.jsonl")).unwrap();
        records.push_str(
            "{\"kind\":\"memory\",\"id\":\"new\",\"owner\":\"local\",\"content\":\"RECIPIENT-ADDED\",\"visibility\":\"[]\"}\n",
        );
        std::fs::write(nested.join("records.jsonl"), &records).unwrap();

        let again = restore_with(Some(&cli), &files, &managed, &settings).unwrap();
        assert!(!again.ready);
        assert!(again
            .blockers
            .contains(&"memory_store_diverged".to_string()));
        // The stale baseline was NOT re-imported over the newer store.
        let after = std::fs::read_to_string(nested.join("records.jsonl")).unwrap();
        assert_eq!(after, records);
        assert!(after.contains("RECIPIENT-ADDED"));
    }

    /// The user's expectation: the automatic flow must report NOT ready when the
    /// provider plugin is absent from the isolated home, and place nothing so the
    /// recipient can install it and retry.
    #[cfg(all(unix, not(feature = "mac-app-store")))]
    #[test]
    fn restore_is_not_ready_without_the_provider_plugin() {
        let root = tempfile::tempdir().unwrap();
        let store = synthetic_store(&root.path().join("store"));
        let home = amos_source(root.path(), &store);
        let cli = fake_cli(root.path());
        let settings = amos_settings(&store);
        let files = capture_with(Some(&cli), &home, &settings).unwrap();

        let managed = root.path().join("managed");
        std::fs::create_dir_all(&managed).unwrap(); // no provider shim
        let readiness = restore_with(Some(&cli), &files, &managed, &settings).unwrap();
        assert!(!readiness.ready);
        assert!(readiness
            .blockers
            .contains(&"memory_provider_not_installed".to_string()));
        assert!(!managed.join(MANAGED_DIR).exists());
    }

    /// The fixed `agent-memory hermes install --hermes-home <home>` operation
    /// writes the provider shim; a zero exit that writes nothing must be refused.
    #[cfg(all(unix, not(feature = "mac-app-store")))]
    const INSTALL_CLI: &str = r#"#!/bin/sh
hermes_home=""
mode=""
while [ $# -gt 0 ]; do
  case "$1" in
    hermes) mode="hermes"; shift ;;
    install) mode="install"; shift ;;
    --hermes-home) hermes_home="$2"; shift 2 ;;
    --home) shift 2 ;;
    *) shift ;;
  esac
done
if [ "$mode" = "install" ] && [ -n "$hermes_home" ]; then
  mkdir -p "$hermes_home/plugins/agent-memory-os"
  printf 'def register(ctx):\n    ctx.register_memory_provider(0)\n' > "$hermes_home/plugins/agent-memory-os/__init__.py"
  exit 0
fi
exit 2
"#;

    #[cfg(all(unix, not(feature = "mac-app-store")))]
    #[test]
    fn install_provider_is_bounded_and_proves_the_shim_by_readback() {
        let root = tempfile::tempdir().unwrap();
        let cli = write_cli(root.path(), "agent-memory", INSTALL_CLI);
        let managed = root.path().join("managed");
        std::fs::create_dir_all(&managed).unwrap();
        assert!(!provider_plugin_present(&managed));
        install_provider(&cli, &managed, root.path()).unwrap();
        assert!(
            provider_plugin_present(&managed),
            "the shim must be discoverable after install"
        );

        // A CLI that exits 0 but writes nothing is not an installed provider.
        let noop = write_cli(root.path(), "noop", "#!/bin/sh\nexit 0\n");
        let other = root.path().join("other");
        std::fs::create_dir_all(&other).unwrap();
        assert_eq!(
            install_provider(&noop, &other, root.path()).unwrap_err(),
            "memory_provider_not_installed"
        );

        // A symlinked managed home is refused, never followed.
        #[cfg(unix)]
        {
            let link = root.path().join("link");
            std::os::unix::fs::symlink(&managed, &link).unwrap();
            assert_eq!(
                install_provider(&cli, &link, root.path()).unwrap_err(),
                "unsafe_store"
            );
        }
        // A missing CLI is reported, not run.
        let absent = root.path().join("no-such-cli");
        assert_eq!(
            install_provider(&absent, &managed, root.path()).unwrap_err(),
            "memory_cli_missing"
        );
    }

    /// The Mac App Store build never installs a provider and never invokes a CLI.
    #[cfg(feature = "mac-app-store")]
    #[test]
    fn store_never_installs_a_provider_or_runs_a_cli() {
        let absent = Path::new("/definitely-not-an-amos-installation");
        assert_eq!(
            install_provider(absent, absent, absent).unwrap_err(),
            "memory_unavailable_store"
        );
        assert!(!external_available());
    }

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
