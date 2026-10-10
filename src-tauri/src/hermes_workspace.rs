//! Hermes Agent workspace capture: user-selected skills, project rules, the
//! built-in curated memory and a strictly-allowlisted, sanitized configuration.
//!
//! Cross-host continuation for Hermes needs more than the native `state.db`
//! conversation store: the skills, identity/memory files and portable settings
//! that give a restored home its behaviour must travel too. Everything here is
//! captured as inert bytes (base64) and never executed; nothing is materialized
//! onto a live home except through an explicit restore into a new isolated
//! folder. No secret — key, token, credential, `.env`, `auth.json` or runtime
//! value — is ever included.
//!
//! Paths and the configuration allowlist are taken from the installed Hermes
//! source: `hermes_constants.get_hermes_home`, `agent.prompt_builder`
//! (`SOUL.md`, `AGENTS.md`), `tools.memory_tool_store` (`memories/MEMORY.md`,
//! `memories/USER.md`), `hermes_cli.config_defaults` and `tools.mcp_tool_config`
//! (`config.yaml`, `mcp_servers`). This module never reads the live user's
//! secrets; MCP `command`, `args` and `env` values are dropped and only the
//! server *name* plus its env variable *names* survive, as inert requirements.
//! Skill `platforms:` is retained as metadata and evaluated against the OS of the
//! *receiving* host — a sender cannot know it, so readiness never trusts the
//! producer's serialized verdict. Skill `deps:` / `prerequisites.commands` /
//! `required_environment_variables` / `required_credential_files` / `requires_apps`
//! and referenced support files become explicit blockers rather than silent
//! drops; a received MCP `command` / `args` is refused outright (a positional
//! token or `-p` flag can smuggle an opaque secret no heuristic can catch). On the
//! receiver, `hermes_requirements` probes the *verifiable* requirements (env
//! presence only; a command on the trusted local `PATH`, never executed) and
//! requires an explicit acknowledgment for the opaque ones. Credential files and
//! apps are named only by their owning skill — their paths and names are never
//! read or copied into the payload.

use crate::{
    review,
    sync::bundle::{self, Result},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

// Receiver-side requirement resolution (probe env/runtime/os locally; require
// explicit acknowledgment for opaque requirements). A child module, exposed via
// `#[path]`, so `lib.rs` stays untouched.
#[path = "hermes_requirements.rs"]
#[allow(dead_code)] // receive-side seam: wired by the readiness path after this module lands
pub(crate) mod hermes_requirements;

/// Inert requirements manifest shipped inside the workspace payload.
pub(crate) const MANIFEST: &str = "bastet/workspace-manifest.json";

const AGENT: &str = "hermes";
const SCHEMA: u32 = 1;
const MAX_FILE: u64 = 1024 * 1024;
const MAX_TOTAL: usize = 16 * 1024 * 1024;
const MAX_FILES: usize = 256;
const MAX_DEPTH: usize = 8;
/// Headroom kept free so the manifest plus captured files stay inside the totals.
const RESERVE: usize = 256 * 1024;

/// Identity file read from `HERMES_HOME` and project rules discovered by the
/// runtime's context-file loader.
const IDENTITY: [&str; 2] = ["SOUL.md", "AGENTS.md"];
/// Curated built-in memory (`tools.memory_tool_store`).
const MEMORY: [&str; 2] = ["memories/MEMORY.md", "memories/USER.md"];
const CONFIG: &str = "config.yaml";
/// Directories a skill scanner skips (VCS, dependency and cache trees).
const EXCLUDED_DIRS: [&str; 14] = [
    ".git",
    ".github",
    ".venv",
    "venv",
    "node_modules",
    "site-packages",
    "__pycache__",
    "target",
    ".tox",
    ".nox",
    ".archive",
    ".locks",
    ".pytest_cache",
    ".mypy_cache",
];
/// Portable, secret-free configuration leaves (dotted) taken verbatim from the
/// Hermes config schema. Keys, tokens, provider credentials, `auth`, `base_url`,
/// `mcp_servers` and every runtime/endpoint field are deliberately absent.
/// `model.provider` / `model.default` and `memory.provider` are non-secret
/// *selections*; when a selection needs a dropped definition (a custom provider
/// endpoint or an external memory binding) that becomes an explicit blocker.
const CONFIG_ALLOW: [&str; 17] = [
    "model.default",
    "model.provider",
    "memory.provider",
    "display.interface",
    "display.skin",
    "display.language",
    "display.compact",
    "display.show_reasoning",
    "display.streaming",
    "display.busy_input_mode",
    "display.background_process_notifications",
    "display.friendly_tool_labels",
    "display.timestamps",
    "display.timestamp_format",
    "display.persistent_output",
    "agent.max_turns",
    "skills.creation_nudge_interval",
];
/// `memory.provider` values Hermes treats as the built-in file memory
/// (`agent/memory_provider.py`); every other non-empty value is external.
const BUILTIN_MEMORY: [&str; 5] = ["", "default", "builtin", "built-in", "none"];
/// Skill support directories a body may reference (mirrors
/// `agent.skill_utils.SKILL_SUPPORT_DIRS`).
const SUPPORT_DIRS: [&str; 4] = ["references/", "templates/", "assets/", "scripts/"];

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

fn env_name_ok(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Hermes maps a skill `platforms:` value onto its host OS identifier
/// (`agent.skill_utils.PLATFORM_MAP`).
fn platform_key(value: &str) -> &str {
    match value {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    }
}

/// The OS the restored home will run on. Evaluated on the *receiving* host at
/// receive time: a producer cannot know it, so a received `os` requirement is
/// always re-scored here rather than trusted from the serialized verdict.
fn host_os() -> &'static str {
    platform_key(std::env::consts::OS)
}

/// A skill's `platforms:` list is satisfied when `os` is one of its declared
/// platforms. An unsupported target remains a blocker rather than a silent drop.
fn platform_supported_for(platforms: &[String], os: &str) -> bool {
    platforms.iter().any(|platform| {
        let normalized = platform.trim().to_lowercase();
        os.starts_with(platform_key(&normalized))
    })
}

/// The producer's provisional verdict for its own preview; the receiver always
/// re-evaluates with [`platform_supported_for`] against its own OS.
fn platform_supported(platforms: &[String]) -> bool {
    platform_supported_for(platforms, host_os())
}

/// Parse `config.yaml` into a JSON object view, or `None` on any malformed
/// document (callers fail closed rather than guessing).
fn config_object(text: &str) -> Option<serde_json::Map<String, serde_json::Value>> {
    let yaml: serde_yaml::Value = serde_yaml::from_str(text).ok()?;
    let json = serde_json::to_value(&yaml).ok()?;
    json.as_object().cloned()
}

fn config_scalar(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Bool(_) | serde_json::Value::Number(_) => true,
        serde_json::Value::String(s) => {
            s.len() <= 200
                && !s.contains(['\n', '\r', '\0'])
                && !s.chars().any(char::is_control)
                && !s.contains("${")
                && !suspected_secret(s)
        }
        _ => false,
    }
}

fn skill_member(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() < 3 || parts[0] != "skills" {
        return false;
    }
    let segments = &parts[1..];
    for (index, segment) in segments.iter().enumerate() {
        let lower = segment.to_lowercase();
        if lower.starts_with('.') || EXCLUDED_DIRS.contains(&lower.as_str()) {
            return false;
        }
        if ["auth", "credential", "token", "secret", "password"]
            .iter()
            .any(|needle| lower.contains(needle))
        {
            return false;
        }
        if index + 1 == segments.len() && lower.ends_with(".lock") {
            return false;
        }
    }
    true
}

/// A workspace-owned, portable path. `state.db` and other adapter files are not
/// workspace-owned; those are ignored by [`validate`] rather than accepted here.
pub(crate) fn allowed(path: &str) -> bool {
    if !crate::portable_paths::path_is_portable(path) {
        return false;
    }
    IDENTITY.contains(&path)
        || MEMORY.contains(&path)
        || path == CONFIG
        || path == MANIFEST
        || (path.starts_with("skills/") && skill_member(path))
}

fn workspace_rooted(path: &str) -> bool {
    IDENTITY.contains(&path)
        || MEMORY.contains(&path)
        || path == CONFIG
        || path == MANIFEST
        || path.starts_with("skills/")
        || path.starts_with("memories/")
        || path.starts_with("bastet/")
}

/// Adapter-owned native files the workspace validator must not judge: the
/// per-conversation `state.db` store and its SQLite sidecars, plus the journal.
fn is_adapter_path(path: &str) -> bool {
    matches!(
        path,
        "state.db" | "state.db-wal" | "state.db-shm" | "state.db-journal"
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ManifestFile {
    pub path: String,
    pub kind: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ManifestIssue {
    pub path: String,
    /// `"excluded"` or `"missing"`.
    pub kind: String,
    pub reason: String,
    /// `true` when the drop keeps a faithful restore from completing.
    pub blocking: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ManifestDependency {
    /// `"os"`, `"env"`, `"credential"`, `"app"`, `"runtime"`, `"mcp"`, `"model"`,
    /// `"memory"` (or the legacy `"command"`).
    pub kind: String,
    pub name: String,
    pub detail: String,
    /// `true` when the recipient must satisfy this before a faithful resume.
    /// Computed on the producer; a received `os` dependency is re-evaluated
    /// against the receiving host (see [`WorkspaceManifest::blocks_on`]).
    pub blocking: bool,
    /// Legacy-only: never produced. Retained so an old manifest still parses; a
    /// received nonempty command is refused outright (no token heuristic).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Legacy-only: never produced. Any received nonempty argument is refused.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Environment variable *names* the recipient must bind; values never travel.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<String>,
    /// An `os` dependency's declared skill `platforms:` list, retained verbatim so
    /// the *receiving* host can re-evaluate it. A sender cannot know the receiver's
    /// OS, so readiness is never decided from the serialized `blocking` alone.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub platforms: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkspaceManifest {
    pub version: u32,
    pub agent: String,
    /// Captured workspace-owned files with their content hashes.
    pub files: Vec<ManifestFile>,
    /// Explicitly excluded or missing sources: never a silent drop.
    pub issues: Vec<ManifestIssue>,
    /// OS/runtime/env/MCP requirements the recipient must satisfy or bind.
    pub dependencies: Vec<ManifestDependency>,
}

impl Default for WorkspaceManifest {
    fn default() -> Self {
        Self {
            version: SCHEMA,
            agent: AGENT.into(),
            files: Vec::new(),
            issues: Vec::new(),
            dependencies: Vec::new(),
        }
    }
}

impl ManifestDependency {
    /// Whether this requirement blocks on the *receiving* host whose OS is
    /// `receiver_os`. An `os` dependency is re-evaluated from its retained
    /// `platforms` list; every other dependency uses the producer's verdict.
    #[allow(dead_code)] // receive-side seam: wired by the readiness path after this module lands
    pub(crate) fn blocks_on(&self, receiver_os: &str) -> bool {
        if self.kind == "os" && !self.platforms.is_empty() {
            !platform_supported_for(&self.platforms, receiver_os)
        } else {
            self.blocking
        }
    }
}

impl WorkspaceManifest {
    fn issue(&mut self, path: &str, kind: &str, reason: &str, blocking: bool) {
        self.issues.push(ManifestIssue {
            path: path.into(),
            kind: kind.into(),
            reason: reason.into(),
            blocking,
        });
    }

    fn validate(&self) -> Result<()> {
        if self.version != SCHEMA || self.agent != AGENT {
            return Err("workspace_format".into());
        }
        if self.files.len() > MAX_FILES
            || self.issues.len() > MAX_FILES
            || self.dependencies.len() > MAX_FILES
        {
            return Err("workspace_limit".into());
        }
        for file in &self.files {
            if !allowed(&file.path) || !bundle::is_hash(&file.sha256) || file.bytes > MAX_FILE {
                return Err("workspace_unsafe".into());
            }
        }
        for dep in &self.dependencies {
            if !safe_text(&dep.name, 200) || !safe_text(&dep.detail, 512) {
                return Err("workspace_unsafe".into());
            }
            // A compliant producer emits no MCP command/args at all. Any received
            // nonempty command or argument is refused outright: a positional token
            // or a short flag (`-p hunter2`) can carry an opaque secret that no
            // keyword heuristic can recognise, so partial "sensitive flag"
            // filtering is not a security boundary.
            if dep
                .command
                .as_deref()
                .is_some_and(|command| !command.is_empty())
                || !dep.args.is_empty()
            {
                return Err("workspace_unsafe".into());
            }
            if dep.env.iter().any(|env| !env_name_ok(env)) {
                return Err("workspace_unsafe".into());
            }
        }
        Ok(())
    }

    /// This manifest's dependencies with each `blocking` flag re-scored for a host
    /// whose OS is `receiver_os`. OS requirements are recomputed from the retained
    /// `platforms` list, never the sender's serialized verdict.
    #[allow(dead_code)] // receive-side seam: wired by the readiness path after this module lands
    pub(crate) fn resolved_for(&self, receiver_os: &str) -> Vec<ManifestDependency> {
        self.dependencies
            .iter()
            .map(|dep| {
                let mut resolved = dep.clone();
                resolved.blocking = dep.blocks_on(receiver_os);
                resolved
            })
            .collect()
    }
}

fn get_json_path<'a>(
    root: &'a serde_json::Map<String, serde_json::Value>,
    dotted: &str,
) -> Option<&'a serde_json::Value> {
    let mut segments = dotted.split('.');
    let first = segments.next()?;
    let mut current = root.get(first)?;
    for segment in segments {
        current = current.as_object()?.get(segment)?;
    }
    Some(current)
}

fn set_json_path(
    root: &mut serde_json::Map<String, serde_json::Value>,
    dotted: &str,
    value: serde_json::Value,
) {
    let segments: Vec<&str> = dotted.split('.').collect();
    let mut current = root;
    for (index, segment) in segments.iter().enumerate() {
        if index + 1 == segments.len() {
            current.insert((*segment).to_string(), value);
            return;
        }
        let entry = current
            .entry((*segment).to_string())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        if !entry.is_object() {
            *entry = serde_json::Value::Object(serde_json::Map::new());
        }
        current = entry.as_object_mut().expect("object inserted above");
    }
}

/// Parse `config.yaml` with a YAML parser (never a regex) and rebuild it from a
/// strict allowlist of leaf settings. Everything outside the allowlist — keys,
/// tokens, `auth`, `mcp_servers`, endpoints — is dropped, so the result is safe
/// to restore as a working baseline config the recipient then rebinds.
pub(crate) fn sanitized_config(text: &str) -> Result<String> {
    let yaml: serde_yaml::Value = serde_yaml::from_str(text).map_err(|_| "workspace_format")?;
    let json = serde_json::to_value(&yaml).map_err(|_| "workspace_format")?;
    let mut clean = serde_json::Map::new();
    if let Some(root) = json.as_object() {
        for dotted in CONFIG_ALLOW {
            if let Some(value) = get_json_path(root, dotted) {
                if config_scalar(value) {
                    set_json_path(&mut clean, dotted, value.clone());
                }
            }
        }
    }
    serde_yaml::to_string(&serde_json::Value::Object(clean)).map_err(|_| "workspace_format".into())
}

/// Split a SKILL.md into its YAML frontmatter and its body, mirroring
/// `agent.skill_utils.parse_frontmatter` (BOM stripped, `---` fenced).
fn split_frontmatter(content: &str) -> Option<(&str, &str)> {
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let rest = content.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    let body = rest[end + 4..].trim_start_matches(['\r', '\n']);
    Some((&rest[..end], body))
}

fn frontmatter(content: &str) -> Option<serde_json::Value> {
    let (yaml, _) = split_frontmatter(content)?;
    let value: serde_yaml::Value = serde_yaml::from_str(yaml).ok()?;
    serde_json::to_value(&value).ok()
}

/// Support-file paths a body references (`references/…`, `templates/…`,
/// `assets/…`, `scripts/…`), mirroring the linter's dangling-reference scan.
/// Globs and placeholders are skipped.
fn body_references(body: &str) -> Vec<String> {
    let mut references = Vec::new();
    for (index, _) in body.char_indices() {
        let Some(rest) = body.get(index..) else {
            continue;
        };
        let Some(dir) = SUPPORT_DIRS.iter().find(|dir| rest.starts_with(**dir)) else {
            continue;
        };
        let boundary = body[..index]
            .chars()
            .next_back()
            .is_none_or(|c| !(c.is_alphanumeric() || matches!(c, '_' | '.' | '-' | '/')));
        if !boundary {
            continue;
        }
        let mut end = index + dir.len();
        while let Some(c) = body[end..].chars().next() {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | '/') {
                end += c.len_utf8();
            } else {
                break;
            }
        }
        let reference = body[index..end].trim_end_matches(['.', '/']);
        if reference.len() > dir.len() && !reference.contains("..") {
            references.push(reference.to_string());
        }
    }
    references.sort();
    references.dedup();
    references
}

/// Required environment variable *names* declared by a skill, with optionality.
/// Mirrors `tools.skills_tool_setup._get_required_environment_variables`: the
/// `required_environment_variables` list plus legacy `prerequisites.env_vars`
/// and `setup.collect_secrets[].env_var`; first declaration wins.
fn required_env(frontmatter: &serde_json::Value) -> Vec<(String, bool)> {
    let mut raw: Vec<(String, bool)> = Vec::new();
    let entry = |item: &serde_json::Value, raw: &mut Vec<(String, bool)>| match item {
        serde_json::Value::String(name) => raw.push((name.clone(), false)),
        serde_json::Value::Object(object) => {
            let name = object
                .get("name")
                .or_else(|| object.get("env_var"))
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string();
            let optional = object
                .get("optional")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            raw.push((name, optional));
        }
        _ => {}
    };
    if let Some(declared) = frontmatter.get("required_environment_variables") {
        match declared {
            serde_json::Value::Array(items) => items.iter().for_each(|i| entry(i, &mut raw)),
            other => entry(other, &mut raw),
        }
    }
    if let Some(items) = frontmatter
        .get("prerequisites")
        .and_then(|value| value.get("env_vars"))
        .and_then(|value| value.as_array())
    {
        for item in items {
            entry(item, &mut raw);
        }
    }
    if let Some(items) = frontmatter
        .get("setup")
        .and_then(|value| value.get("collect_secrets"))
        .and_then(|value| value.as_array())
    {
        for item in items {
            if let Some(name) = item.get("env_var") {
                entry(name, &mut raw);
            }
        }
    }
    let mut seen = BTreeSet::new();
    let mut required = Vec::new();
    for (name, optional) in raw {
        let name = name.trim().to_string();
        if env_name_ok(&name) && seen.insert(name.clone()) {
            required.push((name, optional));
        }
    }
    required
}

fn shebang(text: &str) -> Option<String> {
    let line = text.lines().next()?.strip_prefix("#!")?.trim();
    let mut tokens = line.split_whitespace();
    let mut token = tokens.next()?;
    if token.rsplit('/').next() == Some("env") {
        token = tokens.next()?;
    }
    let name = token.rsplit('/').next().unwrap_or(token);
    if safe_text(name, 64) {
        Some(name.into())
    } else {
        None
    }
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().into_owned())
}

fn insert(
    files: &mut BTreeMap<String, String>,
    manifest: &mut WorkspaceManifest,
    total: &mut usize,
    path: &str,
    kind: &str,
    bytes: &[u8],
) -> Result<()> {
    if files.len() + 1 >= MAX_FILES {
        manifest.issue(path, "excluded", "file_count_limit", true);
        return Ok(());
    }
    if bytes.len() as u64 > MAX_FILE {
        manifest.issue(path, "excluded", "oversized", true);
        return Ok(());
    }
    if *total + bytes.len() + RESERVE > MAX_TOTAL {
        manifest.issue(path, "excluded", "total_limit", true);
        return Ok(());
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        if suspected_secret(text) {
            manifest.issue(path, "excluded", "suspected_credential", false);
            return Ok(());
        }
    }
    *total += bytes.len();
    manifest.files.push(ManifestFile {
        path: path.into(),
        kind: kind.into(),
        bytes: bytes.len() as u64,
        sha256: bundle::hash(bytes),
    });
    files.insert(path.into(), STANDARD.encode(bytes));
    Ok(())
}

fn capture_plain(
    root: &Path,
    rel: &str,
    kind: &str,
    files: &mut BTreeMap<String, String>,
    manifest: &mut WorkspaceManifest,
    total: &mut usize,
) -> Result<()> {
    match review::local_file(root, rel, MAX_FILE) {
        Ok(Some(bytes)) => insert(files, manifest, total, rel, kind, &bytes),
        Ok(None) => {
            manifest.issue(rel, "missing", "missing", false);
            Ok(())
        }
        Err(reason) if reason == "bundle_limit" => {
            manifest.issue(rel, "excluded", "oversized", true);
            Ok(())
        }
        Err(reason) if reason == "unsafe_store" => {
            manifest.issue(rel, "excluded", "symlink", false);
            Ok(())
        }
        Err(reason) => Err(reason),
    }
}

fn walk(
    root: &Path,
    dir: &Path,
    depth: usize,
    files: &mut BTreeMap<String, String>,
    manifest: &mut WorkspaceManifest,
    total: &mut usize,
) -> Result<()> {
    if depth > MAX_DEPTH {
        manifest.issue(&relative(root, dir), "excluded", "depth_limit", true);
        return Ok(());
    }
    let mut children: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|_| "store_unavailable")?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|_| "store_unavailable".to_string())
        })
        .collect::<Result<Vec<_>>>()?;
    children.sort();
    for child in children {
        let rel = relative(root, &child);
        let Ok(meta) = std::fs::symlink_metadata(&child) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            manifest.issue(&rel, "excluded", "symlink", false);
        } else if meta.is_dir() {
            let name = child
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if name.starts_with('.') || EXCLUDED_DIRS.contains(&name.as_str()) {
                manifest.issue(&rel, "excluded", "excluded_directory", false);
            } else {
                walk(root, &child, depth + 1, files, manifest, total)?;
            }
        } else if meta.is_file() {
            if !allowed(&rel) {
                manifest.issue(&rel, "excluded", "unsupported_or_sensitive_path", false);
                continue;
            }
            match review::local_file(root, &rel, MAX_FILE) {
                Ok(Some(bytes)) => insert(files, manifest, total, &rel, "skill", &bytes)?,
                Ok(None) => {}
                Err(reason) if reason == "bundle_limit" => {
                    manifest.issue(&rel, "excluded", "oversized", true);
                }
                Err(reason) if reason == "unsafe_store" => {
                    manifest.issue(&rel, "excluded", "symlink", false);
                }
                Err(reason) => return Err(reason),
            }
        }
    }
    Ok(())
}

fn walk_skills(
    root: &Path,
    files: &mut BTreeMap<String, String>,
    manifest: &mut WorkspaceManifest,
    total: &mut usize,
) -> Result<()> {
    let dir = root.join("skills");
    match std::fs::symlink_metadata(&dir) {
        Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => {
            walk(root, &dir, 0, files, manifest, total)
        }
        Ok(_) => {
            manifest.issue("skills", "excluded", "unsafe_store", false);
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            manifest.issue("skills", "missing", "missing", false);
            Ok(())
        }
        Err(_) => Err("store_unavailable".into()),
    }
}

/// The MCP server *name* and its env variable *names* only. The command, its
/// arguments and every env value never travel — they are dropped, and each
/// server is a required (blocking) binding the recipient must satisfy.
fn collect_mcp(manifest: &mut WorkspaceManifest, config_text: &str) {
    let Some(root) = config_object(config_text) else {
        return;
    };
    let Some(servers) = root.get("mcp_servers").and_then(|v| v.as_object()) else {
        return;
    };
    for (name, cfg) in servers {
        if !safe_text(name, 128) {
            continue;
        }
        let env = cfg
            .get("env")
            .and_then(|value| value.as_object())
            .map(|env| env.keys().filter(|key| env_name_ok(key)).cloned().collect())
            .unwrap_or_default();
        manifest.dependencies.push(ManifestDependency {
            kind: "mcp".into(),
            name: name.clone(),
            detail: "Recipient must install and bind this MCP server; only the server \
                     name and env variable names travel, never the command, arguments \
                     or env values."
                .into(),
            blocking: true,
            command: None,
            args: Vec::new(),
            env,
            platforms: Vec::new(),
        });
    }
}

/// A custom model provider definition (top-level `providers:` map and/or a
/// non-empty `model.base_url` / `model.api_key`) is not portable and is dropped
/// by the sanitizer. The restored config still names the provider/model, so
/// without an explicit blocker it would *look* ready while being unconfigured.
fn collect_model(manifest: &mut WorkspaceManifest, config_text: &str) {
    let Some(root) = config_object(config_text) else {
        return;
    };
    let model = root.get("model").and_then(|v| v.as_object());
    let provider = model
        .and_then(|model| model.get("provider"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let custom = root
        .get("providers")
        .and_then(|value| value.as_object())
        .is_some_and(|providers| !providers.is_empty());
    let endpoint = model.is_some_and(|model| {
        ["base_url", "api_key"].iter().any(|key| {
            model
                .get(*key)
                .and_then(|value| value.as_str())
                .is_some_and(|value| !value.trim().is_empty())
        })
    });
    if !custom && !endpoint {
        return;
    }
    let name = provider.unwrap_or("model").to_string();
    if !safe_text(&name, 200) {
        return;
    }
    manifest.dependencies.push(ManifestDependency {
        kind: "model".into(),
        name,
        detail: "A custom model provider definition (endpoint and/or login material) \
                 is not portable and was dropped; the recipient must configure this \
                 provider/model before the restored route is usable."
            .into(),
        blocking: true,
        command: None,
        args: Vec::new(),
        env: Vec::new(),
        platforms: Vec::new(),
    });
}

/// `memory.provider` is a non-secret *selection* preserved in the config, but an
/// external provider also needs a bound store and identity that live outside the
/// workspace payload (handled by the memory continuation). Until those are
/// bound, the selection is an explicit blocker — never a silent built-in.
fn collect_memory(manifest: &mut WorkspaceManifest, config_text: &str) {
    let Some(root) = config_object(config_text) else {
        return;
    };
    let provider = root
        .get("memory")
        .and_then(|value| value.as_object())
        .and_then(|memory| memory.get("provider"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .unwrap_or("");
    if provider.is_empty() || BUILTIN_MEMORY.contains(&provider) || !safe_text(provider, 64) {
        return;
    }
    manifest.dependencies.push(ManifestDependency {
        kind: "memory".into(),
        name: provider.to_string(),
        detail: "An external memory provider is selected; its bound store and identity \
                 are not part of the workspace payload and the recipient must bind \
                 them before memory continuation."
            .into(),
        blocking: true,
        command: None,
        args: Vec::new(),
        env: Vec::new(),
        platforms: Vec::new(),
    });
}

/// True when a skill declares any `required_credential_files` entry. The declared
/// values (a path or a `{path|name}` map) are deliberately never read — only the
/// presence of a declaration is inspected, so no credential name/path is copied.
fn declares_credential_files(frontmatter: &serde_json::Value) -> bool {
    match frontmatter.get("required_credential_files") {
        Some(serde_json::Value::String(value)) => !value.trim().is_empty(),
        Some(serde_json::Value::Array(items)) => items.iter().any(|item| match item {
            serde_json::Value::String(value) => !value.trim().is_empty(),
            serde_json::Value::Object(map) => ["path", "name"].iter().any(|key| {
                map.get(*key)
                    .and_then(|value| value.as_str())
                    .is_some_and(|value| !value.trim().is_empty())
            }),
            _ => false,
        }),
        _ => false,
    }
}

/// True when a skill declares any `requires_apps` entry. The app names are never
/// echoed into the payload; only the presence of the requirement is inspected.
fn declares_apps(frontmatter: &serde_json::Value) -> bool {
    match frontmatter.get("requires_apps") {
        Some(serde_json::Value::String(value)) => !value.trim().is_empty(),
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .any(|item| item.as_str().is_some_and(|value| !value.trim().is_empty())),
        _ => false,
    }
}

/// Command names declared by a skill's `prerequisites.commands` (a string or a
/// list of strings / `{name|command}` maps). Returns bare, trimmed names; the
/// caller applies the safety filter.
fn command_declarations(value: &serde_json::Value) -> Vec<String> {
    let items: Vec<String> = match value {
        serde_json::Value::String(name) => vec![name.clone()],
        serde_json::Value::Array(values) => values
            .iter()
            .filter_map(|item| match item {
                serde_json::Value::String(name) => Some(name.clone()),
                serde_json::Value::Object(map) => map
                    .get("name")
                    .or_else(|| map.get("command"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    items
        .into_iter()
        .map(|name| name.trim().to_string())
        .collect()
}

fn collect_frontmatter(
    manifest: &mut WorkspaceManifest,
    skill: &str,
    content: &str,
    env_seen: &mut BTreeSet<String>,
    runtime_seen: &mut BTreeSet<String>,
) {
    let Some(frontmatter) = frontmatter(content) else {
        return;
    };
    if let Some(platforms) = frontmatter.get("platforms") {
        let list: Vec<String> = match platforms {
            serde_json::Value::String(s) => vec![s.clone()],
            serde_json::Value::Array(items) => items
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            _ => Vec::new(),
        };
        if !list.is_empty() {
            // A declared platform list is a supported-platform requirement, and the
            // list itself travels so the *receiving* host re-evaluates it against
            // its own OS. `blocking` here is only the producer's provisional verdict.
            manifest.dependencies.push(ManifestDependency {
                kind: "os".into(),
                name: skill.into(),
                detail: format!("skill declares platforms: {}", list.join(", ")),
                blocking: !platform_supported(&list),
                command: None,
                args: Vec::new(),
                env: Vec::new(),
                platforms: list,
            });
        }
    }
    for (name, optional) in required_env(&frontmatter) {
        if env_seen.insert(name.clone()) {
            manifest.dependencies.push(ManifestDependency {
                kind: "env".into(),
                name,
                detail: format!("declared by skill {skill}; the recipient must provide this value"),
                blocking: !optional,
                command: None,
                args: Vec::new(),
                env: Vec::new(),
                platforms: Vec::new(),
            });
        }
    }
    // `deps:` (pm tool packages) are required tools, not merely advisory.
    if let Some(deps) = frontmatter.get("deps") {
        let names: Vec<String> = match deps {
            serde_json::Value::String(name) => vec![name.clone()],
            serde_json::Value::Array(items) => items
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            _ => Vec::new(),
        };
        for name in names {
            let name = name.trim().to_string();
            if name.is_empty() || !safe_text(&name, 128) || !runtime_seen.insert(name.clone()) {
                continue;
            }
            manifest.dependencies.push(ManifestDependency {
                kind: "runtime".into(),
                name,
                detail: format!("tool package required by skill {skill}"),
                blocking: true,
                command: None,
                args: Vec::new(),
                env: Vec::new(),
                platforms: Vec::new(),
            });
        }
    }
    // `prerequisites.commands` is a declared host-command requirement (the
    // installed `_skill_readiness` fields `required_commands` /
    // `missing_required_commands`, e.g. `prerequisites: {commands: [npx]}`).
    // Previously dropped silently; now an explicit `runtime` requirement the
    // *receiver* resolves by scanning its own trusted local PATH — never by
    // running the command.
    if let Some(commands) = frontmatter
        .get("prerequisites")
        .and_then(|value| value.get("commands"))
    {
        for name in command_declarations(commands) {
            if !safe_text(&name, 128)
                || name.chars().any(char::is_whitespace)
                || !runtime_seen.insert(name.clone())
            {
                continue;
            }
            manifest.dependencies.push(ManifestDependency {
                kind: "runtime".into(),
                name,
                detail: format!("command required by skill {skill} (prerequisites.commands)"),
                blocking: true,
                command: None,
                args: Vec::new(),
                env: Vec::new(),
                platforms: Vec::new(),
            });
        }
    }
    // `required_credential_files` and `requires_apps` are real recipient
    // prerequisites (installed loader: `tools.skills_tool._skill_readiness` and
    // `agent.skill_utils.skill_matches_apps`). The user exempts credential
    // *transfer*, not the *check*: emit an explicit blocking dependency, but keep it
    // name/path-free — the declared credential paths and app names are never read or
    // copied into the payload; only the owning skill is named, so the recipient
    // knows which skill to provision on its own host before continuing.
    if declares_credential_files(&frontmatter) {
        manifest.dependencies.push(ManifestDependency {
            kind: "credential".into(),
            name: skill.into(),
            detail: format!(
                "skill {skill} declares required credential files; their contents are never \
                 read or transferred and the recipient must provision them on this computer \
                 before continuing"
            ),
            blocking: true,
            command: None,
            args: Vec::new(),
            env: Vec::new(),
            platforms: Vec::new(),
        });
    }
    if declares_apps(&frontmatter) {
        manifest.dependencies.push(ManifestDependency {
            kind: "app".into(),
            name: skill.into(),
            detail: format!(
                "skill {skill} requires host apps (plugins) that are not part of the payload; \
                 the recipient must enable each required app on this computer before continuing"
            ),
            blocking: true,
            command: None,
            args: Vec::new(),
            env: Vec::new(),
            platforms: Vec::new(),
        });
    }
}

fn collect_dependencies(manifest: &mut WorkspaceManifest, files: &BTreeMap<String, String>) {
    let mut env_seen = BTreeSet::new();
    let mut runtime_seen = BTreeSet::new();
    let mut skill_seen = BTreeSet::new();
    // A skill that ships its own `scripts/` directory owns any `scripts/…` its
    // body cites; a bare citation of a repo-root script is not a skill asset
    // (mirrors `tools.skill_linter`'s dangling-reference scan).
    let mut ships_scripts = BTreeSet::new();
    for path in files.keys() {
        if let Some(rest) = path.strip_prefix("skills/") {
            if let Some((skill, _)) = rest.split_once("/scripts/") {
                ships_scripts.insert(skill.to_string());
            }
        }
    }
    for (path, encoded) in files {
        if !path.starts_with("skills/") {
            continue;
        }
        let Ok(bytes) = STANDARD.decode(encoded) else {
            continue;
        };
        let Ok(text) = std::str::from_utf8(&bytes) else {
            continue;
        };
        if let Some(dir) = path.strip_suffix("/SKILL.md") {
            let skill = dir.trim_start_matches("skills/").to_string();
            if skill_seen.insert(skill.clone()) {
                collect_frontmatter(manifest, &skill, text, &mut env_seen, &mut runtime_seen);
            }
            let body = split_frontmatter(text).map_or(text, |(_, body)| body);
            for reference in body_references(body) {
                let full = format!("{dir}/{reference}");
                if files.contains_key(&full)
                    || (reference.starts_with("scripts/") && !ships_scripts.contains(&skill))
                {
                    continue;
                }
                // Already blocked by the walk (oversized/symlink/…); do not
                // double-report the same path.
                if manifest
                    .issues
                    .iter()
                    .any(|issue| issue.path == full && issue.blocking)
                {
                    continue;
                }
                // A referenced support file that cannot be restored is an
                // explicit blocking dependency, never a silent drop.
                manifest.issue(&full, "missing", "missing_required_skill_asset", true);
            }
        }
        if text.starts_with("#!") {
            if let Some(interpreter) = shebang(text) {
                if runtime_seen.insert(interpreter.clone()) {
                    manifest.dependencies.push(ManifestDependency {
                        kind: "runtime".into(),
                        name: interpreter,
                        detail: format!("interpreter required by a skill script ({path})"),
                        blocking: true,
                        command: None,
                        args: Vec::new(),
                        env: Vec::new(),
                        platforms: Vec::new(),
                    });
                }
            }
        }
    }
}

/// Capture the Hermes workspace as an inert base64 file map. Keys are
/// workspace-relative paths (`SOUL.md`, `AGENTS.md`, `memories/*`, `skills/**`,
/// the sanitized `config.yaml`, and the requirements manifest). Oversized,
/// symlinked, sensitive or missing sources are recorded as explicit
/// blocking/warning issues in the manifest — never silently dropped.
pub(crate) fn capture(root: &Path) -> Result<BTreeMap<String, String>> {
    let mut files: BTreeMap<String, String> = BTreeMap::new();
    let mut manifest = WorkspaceManifest::default();
    let mut total = 0usize;

    match review::local_file(root, CONFIG, MAX_FILE) {
        Ok(Some(bytes)) => match std::str::from_utf8(&bytes) {
            Ok(text) => {
                collect_mcp(&mut manifest, text);
                collect_model(&mut manifest, text);
                collect_memory(&mut manifest, text);
                match sanitized_config(text) {
                    Ok(clean) => insert(
                        &mut files,
                        &mut manifest,
                        &mut total,
                        CONFIG,
                        "config",
                        clean.as_bytes(),
                    )?,
                    Err(_) => manifest.issue(CONFIG, "excluded", "unparseable_config", true),
                }
            }
            Err(_) => manifest.issue(CONFIG, "excluded", "non_text", false),
        },
        Ok(None) => manifest.issue(CONFIG, "missing", "missing", false),
        Err(reason) if reason == "bundle_limit" => {
            manifest.issue(CONFIG, "excluded", "oversized", true)
        }
        Err(reason) if reason == "unsafe_store" => {
            manifest.issue(CONFIG, "excluded", "symlink", false)
        }
        Err(reason) => return Err(reason),
    }

    for path in IDENTITY {
        capture_plain(
            root,
            path,
            "identity",
            &mut files,
            &mut manifest,
            &mut total,
        )?;
    }
    for path in MEMORY {
        capture_plain(root, path, "memory", &mut files, &mut manifest, &mut total)?;
    }
    walk_skills(root, &mut files, &mut manifest, &mut total)?;
    collect_dependencies(&mut manifest, &files);

    let json = serde_json::to_vec(&manifest).map_err(|_| "workspace_format")?;
    files.insert(MANIFEST.into(), STANDARD.encode(&json));

    validate(&files)?;
    Ok(files)
}

/// Validate only the workspace-owned entries of a combined manifest map.
/// `state.db` and other adapter files are ignored; every other entry must be a
/// safe, portable, collision-free workspace path whose content is bounded and
/// secret-free, and whose `config.yaml` — when present — is a sanitized fixed
/// point.
pub(crate) fn validate(files: &BTreeMap<String, String>) -> Result<()> {
    let mut total = 0usize;
    let mut workspace: Vec<&str> = Vec::new();
    for (path, encoded) in files {
        if is_adapter_path(path) {
            continue;
        }
        if !workspace_rooted(path) || !allowed(path) {
            return Err("workspace_unsafe".into());
        }
        let bytes = STANDARD.decode(encoded).map_err(|_| "workspace_format")?;
        if bytes.len() as u64 > MAX_FILE {
            return Err("workspace_limit".into());
        }
        total += bytes.len();
        if let Ok(text) = std::str::from_utf8(&bytes) {
            if suspected_secret(text) {
                return Err("workspace_unsafe".into());
            }
        }
        match path.as_str() {
            CONFIG => {
                let text = std::str::from_utf8(&bytes).map_err(|_| "workspace_format")?;
                if sanitized_config(text)? != text {
                    return Err("workspace_unsafe".into());
                }
            }
            MANIFEST => {
                let text = std::str::from_utf8(&bytes).map_err(|_| "workspace_format")?;
                validate_manifest(text)?;
            }
            _ => {}
        }
        workspace.push(path);
    }
    if total > MAX_TOTAL || workspace.len() > MAX_FILES {
        return Err("workspace_limit".into());
    }
    if !crate::portable_paths::paths_do_not_collide(workspace.iter().copied()) {
        return Err("workspace_unsafe".into());
    }
    Ok(())
}

/// Validate a received workspace manifest's serialized JSON. Shared by
/// [`validate`] and the portable receive path so a crafted manifest is rejected
/// under exactly the same rules on both.
pub(crate) fn validate_manifest(text: &str) -> Result<()> {
    let manifest: WorkspaceManifest = serde_json::from_str(text).map_err(|_| "workspace_format")?;
    manifest.validate()
}

/// Re-score a received workspace manifest for the host that will run the restored
/// home. `receiver_os` is that host's OS identifier (`std::env::consts::OS`, mapped
/// through the same table as [`host_os`]). Validation is unchanged and OS-agnostic;
/// this only re-evaluates readiness, so an `os` requirement is judged where the
/// home actually runs rather than from the sender's serialized verdict.
#[allow(dead_code)] // receive-side seam: wired by the readiness path after this module lands
pub(crate) fn receive_dependencies(
    text: &str,
    receiver_os: &str,
) -> Result<Vec<ManifestDependency>> {
    let manifest: WorkspaceManifest = serde_json::from_str(text).map_err(|_| "workspace_format")?;
    manifest.validate()?;
    Ok(manifest.resolved_for(receiver_os))
}

/// [`receive_dependencies`] evaluated against the *local* host. The OS check runs
/// where the restored home will run, and the *verifiable* requirements — `env`
/// (variable presence only) and `runtime` (a command on the trusted local
/// `PATH`, never executed) — are probed here rather than trusting the sender.
/// Opaque requirements (app/credential/mcp/model/…) stay blocking: with no home
/// to read acknowledgments from, they are conservatively unresolved. This keeps
/// a missing verifiable dependency a blocker, never a silent warning.
#[allow(dead_code)] // receive-side seam: wired by the readiness path after this module lands
pub(crate) fn receive_dependencies_here(text: &str) -> Result<Vec<ManifestDependency>> {
    let manifest: WorkspaceManifest = serde_json::from_str(text).map_err(|_| "workspace_format")?;
    manifest.validate()?;
    let receiver = hermes_requirements::Receiver::local(None);
    let evidence = hermes_requirements::resolve_manifest(
        &manifest,
        &receiver,
        &hermes_requirements::manifest_fingerprint(text),
    )?;
    Ok(manifest
        .dependencies
        .iter()
        .zip(evidence.requirements)
        .map(|(dep, resolved)| {
            let mut scored = dep.clone();
            scored.blocking = resolved.blocking;
            scored
        })
        .collect())
}

/// The full receiver-side verdict for a restored home: each dependency resolved
/// against the local host (env presence, `PATH` command, OS) with opaque
/// requirements permitted only after an explicit acknowledgment scoped to the
/// managed `home` and bound to this exact manifest. Read-only; the caller (the
/// native readiness path) maps [`hermes_requirements::RequirementsEvidence`]
/// into blockers / warnings / verified checks.
#[allow(dead_code)] // receive-side seam: wired by the readiness path after this module lands
pub(crate) fn receive_dependencies_for_home(
    text: &str,
    home: &Path,
) -> Result<hermes_requirements::RequirementsEvidence> {
    hermes_requirements::receive_for_home(text, home)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, name: &str, bytes: &[u8]) {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    fn decode(files: &BTreeMap<String, String>, path: &str) -> Vec<u8> {
        STANDARD.decode(files.get(path).unwrap()).unwrap()
    }

    fn manifest_of(files: &BTreeMap<String, String>) -> WorkspaceManifest {
        serde_json::from_slice(&decode(files, MANIFEST)).unwrap()
    }

    const CONFIG_YAML: &str = "model:\n  provider: \"auto\"\n  default: \"anthropic/claude-opus-4.6\"\ndisplay:\n  skin: default\n  language: en\napi_key: SUPERSECRET\nmcp_servers:\n  demo:\n    command: /usr/bin/demo\n    args: [\"serve\", \"--port\", \"9000\"]\n    env:\n      DEMO_TOKEN: \"should-not-travel\"\n";

    #[test]
    fn roundtrip_skill_package_with_script_asset_reference_and_rules() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "SOUL.md", b"# soul\nidentity");
        write(root.path(), "AGENTS.md", b"# rules\nproject rules");
        write(root.path(), "memories/MEMORY.md", b"- memory entry");
        write(root.path(), "memories/USER.md", b"- user entry");
        write(root.path(), "config.yaml", CONFIG_YAML.as_bytes());
        write(
            root.path(),
            "skills/cat/SKILL.md",
            b"---\nname: cat\nplatforms: [linux, macos]\nrequired_environment_variables:\n  - name: CAT_API_KEY\n---\n# Cat\nrun scripts/run.sh",
        );
        write(
            root.path(),
            "skills/cat/scripts/run.sh",
            b"#!/bin/sh\necho cat\n",
        );
        write(root.path(), "skills/cat/references/guide.md", b"# Guide\n");
        write(
            root.path(),
            "skills/cat/assets/logo.png",
            &[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x01],
        );

        let files = capture(root.path()).unwrap();
        validate(&files).unwrap();

        assert_eq!(decode(&files, "SOUL.md"), b"# soul\nidentity");
        assert_eq!(decode(&files, "AGENTS.md"), b"# rules\nproject rules");
        assert_eq!(decode(&files, "memories/MEMORY.md"), b"- memory entry");
        assert_eq!(decode(&files, "memories/USER.md"), b"- user entry");
        assert_eq!(
            decode(&files, "skills/cat/scripts/run.sh"),
            b"#!/bin/sh\necho cat\n"
        );
        assert_eq!(
            decode(&files, "skills/cat/references/guide.md"),
            b"# Guide\n"
        );
        assert_eq!(
            &decode(&files, "skills/cat/assets/logo.png")[0..4],
            [0x89, 0x50, 0x4e, 0x47]
        );

        // No secret — key, token or MCP env value — survives anywhere.
        for encoded in files.values() {
            assert!(!encoded.contains("SUPERSECRET"));
            let raw = STANDARD.decode(encoded).unwrap();
            let text = String::from_utf8_lossy(&raw);
            assert!(!text.contains("should-not-travel"), "{text}");
            assert!(!text.contains("api_key"));
        }

        // The restored configuration is usable, allowlisted and secret-free.
        let config = String::from_utf8(decode(&files, "config.yaml")).unwrap();
        assert!(config.contains("claude-opus-4.6"));
        assert!(config.contains("skin"));
        assert!(!config.contains("mcp_servers"));
        assert!(!config.contains("api_key"));

        // The manifest records source hashes and inert requirements.
        let manifest = manifest_of(&files);
        assert_eq!(manifest.agent, "hermes");
        assert!(manifest
            .files
            .iter()
            .any(|f| f.path == "skills/cat/assets/logo.png" && bundle::is_hash(&f.sha256)));
        assert!(manifest
            .dependencies
            .iter()
            .any(|d| d.kind == "os" && d.detail.contains("linux")));
        assert!(manifest
            .dependencies
            .iter()
            .any(|d| d.kind == "env" && d.name == "CAT_API_KEY"));
        assert!(manifest
            .dependencies
            .iter()
            .any(|d| d.kind == "runtime" && d.name == "sh"));
        let mcp = manifest
            .dependencies
            .iter()
            .find(|d| d.kind == "mcp")
            .unwrap();
        assert_eq!(mcp.name, "demo");
        // No command, argument or env *value* ever travels: only the server name
        // and the env variable names.
        assert!(mcp.command.is_none());
        assert!(mcp.args.is_empty());
        assert_eq!(mcp.env, vec!["DEMO_TOKEN".to_string()]);
        assert!(mcp.blocking);
        // Required env and runtime dependencies are explicit blockers too.
        assert!(manifest
            .dependencies
            .iter()
            .any(|d| d.kind == "env" && d.name == "CAT_API_KEY" && d.blocking));
        assert!(manifest
            .dependencies
            .iter()
            .any(|d| d.kind == "runtime" && d.name == "sh" && d.blocking));
    }

    #[test]
    fn suspected_credentials_are_excluded_and_rejected_on_receive() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "skills/cat/SKILL.md", b"# Cat\n");
        write(
            root.path(),
            "skills/cat/scripts/leak.sh",
            b"#!/bin/sh\nexport TOKEN=sk-ant-abcdef\n",
        );
        let files = capture(root.path()).unwrap();
        validate(&files).unwrap();
        assert!(!files.contains_key("skills/cat/scripts/leak.sh"));
        let manifest = manifest_of(&files);
        assert!(manifest.issues.iter().any(|i| {
            i.path == "skills/cat/scripts/leak.sh"
                && i.reason == "suspected_credential"
                && !i.blocking
        }));
        for encoded in files.values() {
            let raw = STANDARD.decode(encoded).unwrap();
            assert!(!String::from_utf8_lossy(&raw).contains("sk-ant-"));
        }

        // A received workspace file carrying a secret is refused outright.
        let mut tampered = files.clone();
        tampered.insert(
            "skills/cat/scripts/leak.sh".into(),
            STANDARD.encode(b"sk-ant-abcdef"),
        );
        assert_eq!(validate(&tampered).unwrap_err(), "workspace_unsafe");
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_traversal_and_collisions_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("skills/cat")).unwrap();
        write(root.path(), "skills/cat/SKILL.md", b"# Cat\n");
        std::os::unix::fs::symlink(
            root.path().join("elsewhere"),
            root.path().join("skills/cat/assets"),
        )
        .unwrap();
        let files = capture(root.path()).unwrap();
        validate(&files).unwrap();
        let manifest = manifest_of(&files);
        assert!(manifest
            .issues
            .iter()
            .any(|i| i.path == "skills/cat/assets" && i.reason == "symlink"));

        let mut traversal = BTreeMap::new();
        traversal.insert("../escape.md".into(), STANDARD.encode(b"x"));
        assert_eq!(validate(&traversal).unwrap_err(), "workspace_unsafe");

        let mut collide = BTreeMap::new();
        collide.insert("skills/Cat/SKILL.md".into(), STANDARD.encode(b"a"));
        collide.insert("skills/cat/SKILL.md".into(), STANDARD.encode(b"b"));
        assert_eq!(validate(&collide).unwrap_err(), "workspace_unsafe");

        let mut unknown = BTreeMap::new();
        unknown.insert("other.db".into(), STANDARD.encode(b"x"));
        assert_eq!(validate(&unknown).unwrap_err(), "workspace_unsafe");
    }

    #[test]
    fn adapter_files_are_ignored_but_everything_else_is_checked() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "SOUL.md", b"identity");
        let mut files = capture(root.path()).unwrap();
        files.insert("state.db".into(), STANDARD.encode(b"SQLite format 3\0"));
        files.insert("state.db-wal".into(), STANDARD.encode(b"wal"));
        validate(&files).unwrap();
    }

    #[test]
    fn oversize_and_missing_sources_are_explicit_never_silent() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "skills/cat/SKILL.md", b"# Cat\n");
        write(
            root.path(),
            "skills/cat/assets/big.bin",
            &vec![0u8; MAX_FILE as usize + 1],
        );
        // SOUL.md, memories and config.yaml are intentionally absent.
        let files = capture(root.path()).unwrap();
        validate(&files).unwrap();
        let manifest = manifest_of(&files);
        assert!(manifest.issues.iter().any(|i| {
            i.path == "skills/cat/assets/big.bin" && i.reason == "oversized" && i.blocking
        }));
        assert!(manifest
            .issues
            .iter()
            .any(|i| i.path == "SOUL.md" && i.kind == "missing"));
        assert!(manifest
            .issues
            .iter()
            .any(|i| i.path == "memories/MEMORY.md" && i.kind == "missing"));
        assert!(manifest
            .issues
            .iter()
            .any(|i| i.path == "config.yaml" && i.kind == "missing"));
        assert!(!files.contains_key("skills/cat/assets/big.bin"));

        let mut tampered = files.clone();
        tampered.insert(
            "skills/cat/assets/big.bin".into(),
            STANDARD.encode(vec![0u8; MAX_FILE as usize + 1]),
        );
        assert_eq!(validate(&tampered).unwrap_err(), "workspace_limit");
    }

    #[test]
    fn received_config_must_be_a_sanitized_fixed_point() {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "config.yaml",
            b"display:\n  skin: dracula\napi_key: LEAK\n",
        );
        let files = capture(root.path()).unwrap();
        validate(&files).unwrap();
        let config = String::from_utf8(decode(&files, "config.yaml")).unwrap();
        assert!(config.contains("dracula"));

        let mut tampered = files.clone();
        tampered.insert(
            "config.yaml".into(),
            STANDARD.encode(b"display:\n  skin: dracula\napi_key: LEAK\n"),
        );
        assert_eq!(validate(&tampered).unwrap_err(), "workspace_unsafe");
    }

    #[test]
    fn capture_of_missing_home_is_explicit_and_valid() {
        let root = tempfile::tempdir().unwrap();
        let files = capture(root.path()).unwrap();
        validate(&files).unwrap();
        let manifest = manifest_of(&files);
        assert!(manifest
            .issues
            .iter()
            .any(|i| i.path == "skills" && i.kind == "missing"));
        assert!(manifest.files.is_empty());
        assert_eq!(manifest.version, SCHEMA);
    }

    #[test]
    fn sanitized_config_is_yaml_parsed_and_idempotent() {
        let once = sanitized_config(CONFIG_YAML).unwrap();
        assert_eq!(sanitized_config(&once).unwrap(), once);
        assert!(!once.contains("api_key"));
        assert!(!once.contains("mcp_servers"));
        // A malformed document is refused rather than silently emptied.
        assert!(sanitized_config("model: [unterminated\n").is_err());
    }

    #[test]
    fn mcp_command_args_and_env_values_never_travel_and_received_secrets_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "config.yaml",
            b"model:\n  default: \"vendor/model\"\nmcp_servers:\n  danger:\n    command: /usr/bin/danger\n    args: [\"serve\", \"--password\", \"hunter2\"]\n    env:\n      DANGER_TOKEN: \"should-not-travel\"\n",
        );
        let files = capture(root.path()).unwrap();
        validate(&files).unwrap();
        for encoded in files.values() {
            let text = String::from_utf8_lossy(&STANDARD.decode(encoded).unwrap()).into_owned();
            assert!(!text.contains("hunter2"), "{text}");
            assert!(!text.contains("should-not-travel"), "{text}");
            assert!(!text.contains("--password"), "{text}");
            assert!(!text.contains("/usr/bin/danger"), "{text}");
        }
        let manifest = manifest_of(&files);
        let mcp = manifest
            .dependencies
            .iter()
            .find(|d| d.kind == "mcp")
            .unwrap();
        assert_eq!(mcp.name, "danger");
        assert!(mcp.command.is_none());
        assert!(mcp.args.is_empty());
        assert_eq!(mcp.env, vec!["DANGER_TOKEN".to_string()]);
        assert!(mcp.blocking);

        // A compliant producer never emits MCP command/args; any received
        // command or nonempty argument is refused outright. A positional token or
        // a short flag can carry an opaque secret that no keyword heuristic can
        // recognise, so partial "sensitive flag" filtering is not a boundary.
        for dependency in [
            serde_json::json!({"kind": "mcp", "name": "danger", "detail": "install",
                "blocking": true, "command": "/usr/bin/x", "args": ["--password", "hunter2"]}),
            serde_json::json!({"kind": "mcp", "name": "danger", "detail": "install",
                "blocking": true, "args": ["--client-secret=hunter2"]}),
            serde_json::json!({"kind": "mcp", "name": "danger", "detail": "install",
                "blocking": true, "command": "/usr/bin/x --api-key hunter2"}),
            // An arbitrary opaque positional token has no credential keyword to match.
            serde_json::json!({"kind": "mcp", "name": "danger", "detail": "install",
                "blocking": true, "args": ["xY9zQ2wLp"]}),
            // Short `-p` password flags a keyword heuristic misses entirely.
            serde_json::json!({"kind": "mcp", "name": "danger", "detail": "install",
                "blocking": true, "args": ["-p", "hunter2"]}),
            serde_json::json!({"kind": "mcp", "name": "danger", "detail": "install",
                "blocking": true, "args": ["-p=hunter2"]}),
            // Even an innocuous-looking command/args is refused: only the producer
            // dropping them entirely is accepted.
            serde_json::json!({"kind": "mcp", "name": "demo", "detail": "install",
                "blocking": true, "command": "/usr/bin/demo",
                "args": ["serve", "--port", "9000"]}),
            serde_json::json!({"kind": "env", "name": "DANGER_TOKEN",
                "detail": "sk-ant-LEAK", "blocking": true}),
        ] {
            let manifest = serde_json::json!({
                "version": 1, "agent": "hermes", "files": [], "issues": [],
                "dependencies": [dependency],
            });
            let mut crafted = BTreeMap::new();
            crafted.insert(
                MANIFEST.into(),
                STANDARD.encode(serde_json::to_vec(&manifest).unwrap()),
            );
            assert_eq!(validate(&crafted).unwrap_err(), "workspace_unsafe");
        }

        // A dependency with the command/args fields absent stays valid.
        let clean = serde_json::json!({
            "version": 1, "agent": "hermes", "files": [], "issues": [],
            "dependencies": [{"kind": "mcp", "name": "demo", "detail": "install",
                "blocking": true}],
        });
        let mut accepted = BTreeMap::new();
        accepted.insert(
            MANIFEST.into(),
            STANDARD.encode(serde_json::to_vec(&clean).unwrap()),
        );
        validate(&accepted).unwrap();
    }

    #[test]
    fn memory_provider_selection_is_preserved_and_external_binding_is_a_blocker() {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "config.yaml",
            b"memory:\n  provider: \"agent-memory-os\"\ndisplay:\n  skin: default\n",
        );
        let files = capture(root.path()).unwrap();
        validate(&files).unwrap();
        let config = String::from_utf8(decode(&files, "config.yaml")).unwrap();
        assert!(config.contains("agent-memory-os"));
        // The selection is a secret-free scalar and a sanitizer fixed point.
        assert_eq!(sanitized_config(&config).unwrap(), config);
        let manifest = manifest_of(&files);
        let memory = manifest
            .dependencies
            .iter()
            .find(|d| d.kind == "memory")
            .unwrap();
        assert_eq!(memory.name, "agent-memory-os");
        assert!(memory.blocking);

        // The built-in selection carries no external dependency at all.
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "config.yaml",
            b"memory:\n  provider: builtin\n",
        );
        let manifest = manifest_of(&capture(root.path()).unwrap());
        assert!(!manifest.dependencies.iter().any(|d| d.kind == "memory"));
    }

    #[test]
    fn custom_model_provider_definition_requires_explicit_setup() {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "config.yaml",
            b"model:\n  provider: \"mycustom\"\n  default: \"vendor/model\"\n  base_url: \"https://user:hunter2@example.invalid/v1\"\n  api_key: \"should-not-travel\"\nproviders:\n  mycustom:\n    base_url: \"https://user:hunter2@example.invalid/v1\"\n    api_key: \"should-not-travel\"\n",
        );
        let files = capture(root.path()).unwrap();
        validate(&files).unwrap();
        for encoded in files.values() {
            let text = String::from_utf8_lossy(&STANDARD.decode(encoded).unwrap()).into_owned();
            assert!(!text.contains("hunter2"), "{text}");
            assert!(!text.contains("should-not-travel"), "{text}");
            assert!(!text.contains("base_url"), "{text}");
            assert!(!text.contains("providers"), "{text}");
        }
        let config = String::from_utf8(decode(&files, "config.yaml")).unwrap();
        assert!(config.contains("mycustom"));
        let manifest = manifest_of(&files);
        let model = manifest
            .dependencies
            .iter()
            .find(|d| d.kind == "model")
            .unwrap();
        assert_eq!(model.name, "mycustom");
        assert!(model.blocking);

        // A registry provider with no dropped definition needs no setup dependency.
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "config.yaml",
            b"model:\n  provider: \"openrouter\"\n  default: \"vendor/model\"\n",
        );
        let manifest = manifest_of(&capture(root.path()).unwrap());
        assert!(!manifest.dependencies.iter().any(|d| d.kind == "model"));
    }

    #[test]
    fn missing_required_skill_assets_block_while_builtin_absence_only_warns() {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "skills/cat/SKILL.md",
            b"---\nname: cat\n---\n# Cat\nsee references/missing.md and scripts/run.sh",
        );
        // The skill ships its own scripts directory, so `scripts/run.sh` is
        // skill-owned; the referenced files are absent either way.
        write(root.path(), "skills/cat/scripts/keep.sh", b"#!/bin/sh\n");
        let files = capture(root.path()).unwrap();
        validate(&files).unwrap();
        let manifest = manifest_of(&files);
        assert!(manifest.issues.iter().any(|i| {
            i.path == "skills/cat/references/missing.md"
                && i.reason == "missing_required_skill_asset"
                && i.blocking
        }));
        assert!(manifest.issues.iter().any(|i| {
            i.path == "skills/cat/scripts/run.sh"
                && i.reason == "missing_required_skill_asset"
                && i.blocking
        }));
        // Built-in identity/rules/memory absence stays a non-blocking warning.
        for path in ["SOUL.md", "memories/MEMORY.md"] {
            assert!(manifest
                .issues
                .iter()
                .any(|i| i.path == path && i.kind == "missing" && !i.blocking));
        }

        // A skill without its own scripts directory cites a repo-root script:
        // not a skill-owned asset, so not a missing dependency.
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "skills/dev/SKILL.md",
            b"---\nname: dev\n---\n# Dev\nrun scripts/repo-tool.sh",
        );
        let manifest = manifest_of(&capture(root.path()).unwrap());
        assert!(!manifest
            .issues
            .iter()
            .any(|i| i.path == "skills/dev/scripts/repo-tool.sh"));
    }

    #[test]
    fn platform_requirement_is_evaluated_against_the_target_os() {
        fn host_platform() -> &'static str {
            match std::env::consts::OS {
                "macos" => "macos",
                "windows" => "windows",
                _ => "linux",
            }
        }
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "skills/here/SKILL.md",
            format!(
                "---\nname: here\nplatforms: [{}]\n---\n# Here\n",
                host_platform()
            )
            .as_bytes(),
        );
        let manifest = manifest_of(&capture(root.path()).unwrap());
        let os = manifest
            .dependencies
            .iter()
            .find(|d| d.kind == "os")
            .unwrap();
        assert!(!os.blocking, "the target OS is a declared platform");
        assert_eq!(os.platforms, vec![host_platform().to_string()]);

        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "skills/other/SKILL.md",
            b"---\nname: other\nplatforms: [plan9]\n---\n# Other\n",
        );
        let manifest = manifest_of(&capture(root.path()).unwrap());
        let os = manifest
            .dependencies
            .iter()
            .find(|d| d.kind == "os")
            .unwrap();
        assert!(os.blocking, "an unsupported target OS blocks the resume");
        assert_eq!(os.platforms, vec!["plan9".to_string()]);
    }

    #[test]
    fn required_environment_variables_block_and_optional_ones_only_warn() {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "skills/envs/SKILL.md",
            b"---\nname: envs\nrequired_environment_variables:\n  - name: REQUIRED_KEY\n  - name: OPTIONAL_KEY\n    optional: true\nprerequisites:\n  env_vars: [LEGACY_KEY]\n---\n# Envs\n",
        );
        let manifest = manifest_of(&capture(root.path()).unwrap());
        for name in ["REQUIRED_KEY", "LEGACY_KEY"] {
            assert!(
                manifest
                    .dependencies
                    .iter()
                    .any(|d| d.kind == "env" && d.name == name && d.blocking),
                "{name}"
            );
        }
        assert!(manifest
            .dependencies
            .iter()
            .any(|d| d.kind == "env" && d.name == "OPTIONAL_KEY" && !d.blocking));
    }

    #[test]
    fn credential_files_and_required_apps_are_name_path_free_blockers() {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "skills/vault/SKILL.md",
            b"---\nname: vault\nrequired_credential_files:\n  - mcp-tokens/github.json\n  - path: creds/service.pem\nrequires_apps: [thing-mcp, no-such-app]\n---\n# Vault\n",
        );
        // The declared credential file is never read: give it content and a name
        // that would be obvious if either leaked into the payload.
        write(
            root.path(),
            "mcp-tokens/github.json",
            b"CREDENTIAL-SENTINEL",
        );
        let files = capture(root.path()).unwrap();
        validate(&files).unwrap();

        // The credential file is never read or captured, and no captured file
        // carries its bytes.
        assert!(!files.contains_key("mcp-tokens/github.json"));
        for encoded in files.values() {
            let text = String::from_utf8_lossy(&STANDARD.decode(encoded).unwrap()).into_owned();
            assert!(!text.contains("CREDENTIAL-SENTINEL"), "{text}");
        }

        let manifest_text = String::from_utf8(decode(&files, MANIFEST)).unwrap();
        let manifest = manifest_of(&files);
        let credential = manifest
            .dependencies
            .iter()
            .find(|d| d.kind == "credential")
            .expect("credential dependency");
        assert!(credential.blocking);
        assert_eq!(credential.name, "vault");
        assert!(credential.command.is_none() && credential.args.is_empty());
        let app = manifest
            .dependencies
            .iter()
            .find(|d| d.kind == "app")
            .expect("app dependency");
        assert!(app.blocking);
        assert_eq!(app.name, "vault");
        // The emitted dependency is name/path-free: the declared credential paths
        // and the app names are never echoed into the manifest.
        for needle in ["github.json", "service.pem", "thing-mcp", "no-such-app"] {
            assert!(
                !manifest_text.contains(needle),
                "{needle} in {manifest_text}"
            );
        }

        // A skill declaring neither emits neither dependency.
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "skills/plain/SKILL.md",
            b"---\nname: plain\n---\n# Plain\n",
        );
        let manifest = manifest_of(&capture(root.path()).unwrap());
        assert!(!manifest
            .dependencies
            .iter()
            .any(|d| d.kind == "credential" || d.kind == "app"));
    }

    #[test]
    fn platform_requirement_is_re_evaluated_for_the_receiver_os() {
        // The declared platform list is retained as structured metadata, so the
        // receiving host can judge it instead of trusting the sender's verdict.
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "skills/linuxonly/SKILL.md",
            b"---\nname: linuxonly\nplatforms: [linux]\n---\n# L\n",
        );
        let files = capture(root.path()).unwrap();
        let manifest = manifest_of(&files);
        let os = manifest
            .dependencies
            .iter()
            .find(|d| d.kind == "os")
            .unwrap();
        assert_eq!(os.platforms, vec!["linux".to_string()]);
        assert!(!os.blocks_on("linux"));
        assert!(os.blocks_on("win32"));
        assert!(os.blocks_on("darwin"));

        // A sender verdict serialized into the manifest does not decide the
        // receiver: the same manifest blocks on win32 and stays clear on linux.
        let text = String::from_utf8(decode(&files, MANIFEST)).unwrap();
        assert!(receive_dependencies(&text, "win32")
            .unwrap()
            .iter()
            .any(|d| d.kind == "os" && d.blocking));
        assert!(!receive_dependencies(&text, "linux")
            .unwrap()
            .iter()
            .any(|d| d.kind == "os" && d.blocking));
    }

    #[test]
    fn received_os_requirement_is_scored_on_the_receiver_not_the_sender() {
        // Simulate two senders on different OSes serializing opposite verdicts for
        // the same linux-only skill requirement. The receiver is what decides.
        let manifest = |blocking: bool| {
            serde_json::json!({
                "version": 1, "agent": "hermes", "files": [], "issues": [],
                "dependencies": [{
                    "kind": "os", "name": "cat",
                    "detail": "skill declares platforms: linux",
                    "blocking": blocking, "platforms": ["linux"],
                }],
            })
        };
        let sender_on_windows = serde_json::to_string(&manifest(true)).unwrap();
        let sender_on_linux = serde_json::to_string(&manifest(false)).unwrap();
        for text in [&sender_on_windows, &sender_on_linux] {
            // A linux receiver is ready regardless of the sender's recorded verdict.
            assert!(!receive_dependencies(text, "linux")
                .unwrap()
                .iter()
                .any(|d| d.blocking));
            // A win32 receiver blocks regardless of the sender's recorded verdict.
            assert!(receive_dependencies(text, "win32")
                .unwrap()
                .iter()
                .any(|d| d.blocking));
        }
        // Non-OS dependencies keep the producer's verdict under retargeting.
        let mixed = serde_json::to_string(&serde_json::json!({
            "version": 1, "agent": "hermes", "files": [], "issues": [],
            "dependencies": [{"kind": "env", "name": "KEY", "detail": "provide",
                "blocking": true}],
        }))
        .unwrap();
        assert!(receive_dependencies(&mixed, "linux")
            .unwrap()
            .iter()
            .any(|d| d.kind == "env" && d.blocking));
    }

    #[test]
    fn prerequisite_commands_are_preserved_as_blocking_runtime_requirements() {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "skills/watch/SKILL.md",
            b"---\nname: watch\nprerequisites:\n  commands: [blogwatcher-cli, npx]\n---\n# Watch\n",
        );
        let files = capture(root.path()).unwrap();
        validate(&files).unwrap();
        let manifest = manifest_of(&files);
        for name in ["blogwatcher-cli", "npx"] {
            assert!(
                manifest
                    .dependencies
                    .iter()
                    .any(|d| d.kind == "runtime" && d.name == name && d.blocking),
                "prerequisites.commands entry {name} must travel, not be dropped"
            );
        }
        assert!(manifest
            .dependencies
            .iter()
            .any(|d| d.kind == "runtime" && d.detail.contains("prerequisites.commands")));
    }

    #[test]
    fn receiver_probes_verifiable_requirements_and_keeps_opaque_ones_blocking() {
        // `sh` is resolved by scanning PATH (not the sender's verdict); an absent
        // environment variable and an opaque credential stay blockers.
        let text = serde_json::to_string(&serde_json::json!({
            "version": 1, "agent": "hermes", "files": [], "issues": [],
            "dependencies": [
                {"kind": "env", "name": "BASTET_ABSENT_REQUIREMENT_XYZ",
                 "detail": "provide", "blocking": false},
                {"kind": "credential", "name": "vault",
                 "detail": "provision", "blocking": true},
            ],
        }))
        .unwrap();
        let deps = receive_dependencies_here(&text).unwrap();
        let env = deps
            .iter()
            .find(|d| d.kind == "env")
            .expect("env dependency");
        assert!(env.blocking, "an absent variable stays a blocker");
        let credential = deps
            .iter()
            .find(|d| d.kind == "credential")
            .expect("credential dependency");
        assert!(
            credential.blocking,
            "an opaque requirement without an acknowledgment still blocks"
        );
    }
}
