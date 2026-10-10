//! Allowlisted session snapshots. Receiving never writes into a live agent profile.
#[cfg(test)]
mod hermes_tests;
#[cfg(test)]
mod perf_tests;
#[cfg(test)]
mod real_drive_perf_tests;
use crate::{
    cloud::{
        crypto::SpaceKey,
        queue::{self, Binding, Objects},
    },
    sync::{
        bundle::{self, Result, Stream},
        storage, Direction, Replica,
    },
};
use base64::{engine::general_purpose::STANDARD, Engine};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use tauri::Manager;
const MAX_RAW: u64 = 512 * 1024 * 1024;
const MAX_PACKED: u64 = 384 * 1024 * 1024;
const PART_SIZE: usize = 23 * 1024 * 1024;
#[cfg(test)]
std::thread_local! {
    static ENCODE_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Manifest {
    pub version: u32,
    pub agent: String,
    pub session: String,
    pub cwd: String,
    pub files: BTreeMap<String, String>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceStatus {
    #[serde(default)]
    pub progress: Option<crate::progress::Progress>,
    pub agent: String,
    pub state: String,
    pub captured: usize,
    pub available: usize,
    pub published: usize,
    pub received: usize,
    pub restored: usize,
    pub issues: BTreeMap<String, usize>,
}
#[derive(Default, Serialize, Deserialize)]
struct Journal {
    node: String,
    bases: BTreeMap<String, String>,
    #[serde(default)]
    stamps: BTreeMap<String, String>,
    #[serde(default)]
    content: BTreeMap<String, ContentBaseline>,
}
#[derive(Serialize, Deserialize)]
struct ContentBaseline {
    fingerprint: String,
    bundle: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Handoff {
    // The path is a new, explicitly restored profile, never an active default store.
    path: PathBuf,
    agent: String,
    session: String,
    main_file: String,
    cwd: String,
    stream_profile: String,
    base: String,
    fingerprint: String,
    // The NEW isolated project working directory restored alongside the profile,
    // when the received snapshot carried one. Never the receiver's mapped
    // worktree. Defaults absent so pre-project registries still parse.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    project: Option<PathBuf>,
    // Readiness blockers recorded for that project working tree; surfaced so a
    // copied project sidecar is never reported ready.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    project_blockers: Vec<String>,
    // Real external-memory readiness recorded when the managed home was
    // restored (verified by re-export, never a synthetic ready). Defaults absent
    // so pre-memory registries still parse.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    memory: Option<crate::hermes_memory::MemoryReadiness>,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Handoffs {
    version: u32,
    entries: Vec<Handoff>,
}
fn manifest_fingerprint(m: &Manifest) -> Result<String> {
    Ok(bundle::hash(json(&m.files)?.as_bytes()))
}
fn manifest_content_fingerprint(m: &Manifest) -> Result<String> {
    // Include session metadata as well as every captured file and companion.
    struct Hasher(Sha256);
    impl Write for Hasher {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut hasher = Hasher(Sha256::new());
    serde_json::to_writer(&mut hasher, m).map_err(|_| "session_invalid")?;
    Ok(format!("{:x}", hasher.0.finalize()))
}
fn handoffs_path(root: &Path) -> PathBuf {
    root.join("native-handoffs.json")
}
fn load_handoffs(root: &Path) -> Result<Handoffs> {
    let p = handoffs_path(root);
    if !p.exists() {
        return Ok(Handoffs {
            version: 1,
            entries: vec![],
        });
    }
    let registry: Handoffs = serde_json::from_slice(&storage::read(&p, 8 * 1024 * 1024)?)
        .map_err(|_| "sync_journal_invalid")?;
    if registry.version != 1 || registry.entries.len() > 4096 {
        return Err("sync_journal_invalid".into());
    }
    Ok(registry)
}

fn handoff_parent_scope(
    config: &Path,
    parent: &Path,
) -> Result<Option<crate::sandbox_access::Scope>> {
    // Managed profiles created by auto_prepare are inside app-private data and
    // never went through a picker. Canonical containment prevents a symlink in
    // that tree from making an external directory look private.
    if let (Ok(private), Ok(candidate)) = (fs::canonicalize(config), fs::canonicalize(parent)) {
        if candidate.starts_with(private) {
            return Ok(None);
        }
    }
    crate::sandbox_access::access(config, parent).map(Some)
}
fn save_handoffs(root: &Path, registry: &Handoffs) -> Result<()> {
    storage::replace(&handoffs_path(root), json(registry)?.as_bytes())
}
fn main_file(m: &Manifest) -> Result<String> {
    m.files
        .keys()
        .find(|path| match m.agent.as_str() {
            "grok" => path.ends_with("/updates.jsonl"),
            "claude" | "claude-code" => path.ends_with(".jsonl") && !path.contains("/subagents/"),
            "agy" => path.ends_with(".db"),
            "hermes" => path.as_str() == "state.db",
            _ => path.ends_with(".jsonl"),
        })
        .cloned()
        .ok_or_else(|| "session_invalid".into())
}
fn json<T: Serialize>(v: &T) -> Result<String> {
    serde_json::to_string(v).map_err(|_| "session_invalid".into())
}
pub(crate) fn safe_relative(value: &str) -> bool {
    crate::portable_paths::path_is_portable(value)
}
fn manifest_paths_safe(m: &Manifest) -> bool {
    m.files.keys().all(|path| allowed(&m.agent, path))
        && crate::portable_paths::paths_do_not_collide(m.files.keys().map(String::as_str))
}
fn paths_overlap(a: &Path, b: &Path) -> bool {
    let a = fs::canonicalize(a).unwrap_or_else(|_| a.to_path_buf());
    let b = fs::canonicalize(b).unwrap_or_else(|_| b.to_path_buf());
    a.starts_with(&b) || b.starts_with(&a)
}
#[cfg(feature = "mac-app-store")]
fn restore_overlaps_source(folder: &Path, source: &Path, config: &Path) -> bool {
    if !source.is_absolute()
        || source
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return true;
    }
    if let Ok(_scope) = crate::sandbox_access::access(config, source) {
        return paths_overlap(folder, source);
    }
    // An ungranted root cannot be stat-ed merely for this check.
    let source = crate::sandbox_access::lexical_overlap_path(source);
    folder.starts_with(&source) || source.starts_with(folder)
}
fn allowed(agent: &str, path: &str) -> bool {
    if !safe_relative(path) {
        return false;
    }
    let parts = path.split('/').collect::<Vec<_>>();
    match agent {
        "codex" | "chatgpt-work" => {
            matches!(parts[0], "sessions" | "archived_sessions") && path.ends_with(".jsonl")
        }
        "claude" | "claude-code" => parts[0] == "projects" && path.ends_with(".jsonl"),
        "pi" => parts[0] == "sessions" && path.ends_with(".jsonl"),
        "agy" => {
            parts.len() == 2
                && parts[0] == "conversations"
                && parts[1].ends_with(".db")
                && bundle::token(parts[1].trim_end_matches(".db"))
        }
        "grok" => {
            parts[0] == "sessions"
                && ((parts.len() == 3 && parts[2] == ".cwd")
                    || (parts.len() == 4
                        && bundle::token(parts[2])
                        && [
                            "summary.json",
                            "updates.jsonl",
                            "chat_history.jsonl",
                            "plan.json",
                            "signals.json",
                            "rewind_points.jsonl",
                        ]
                        .contains(&parts[3])))
        }
        // Hermes Agent keeps every session in one SQLite store. An exported
        // bundle carries the selected session's lineage graph as `state.db`,
        // the merged portable workspace and the content-hashed referenced
        // attachments.
        "hermes" => {
            path == "state.db"
                || crate::hermes_workspace::allowed(path)
                || crate::hermes_project::allowed(path)
                || crate::hermes_memory::allowed(path)
                || hermes_attachment_path(path)
                || path == HERMES_ASYNC_INDEX
        }
        _ => false,
    }
}
fn children(root: &Path) -> Result<Vec<PathBuf>> {
    storage::directory(root)?;
    let mut paths = fs::read_dir(root)
        .map_err(|_| "source_unreadable")?
        .map(|e| {
            e.map(|e| e.path())
                .map_err(|_| "source_unreadable".to_string())
        })
        .collect::<Result<Vec<_>>>()?;
    paths.sort();
    Ok(paths)
}
fn walk(root: &Path, depth: usize, out: &mut Vec<PathBuf>) -> Result<()> {
    if depth == 0 {
        return Ok(());
    }
    for p in children(root)? {
        let m = fs::symlink_metadata(&p).map_err(|_| "source_unreadable")?;
        if m.file_type().is_symlink() {
            continue;
        }
        if m.is_dir() {
            walk(&p, depth - 1, out)?;
        } else if m.is_file() {
            out.push(p);
            if out.len() > 20000 {
                return Err("session_limit".into());
            }
        }
    }
    Ok(())
}
fn stable(path: &Path) -> Result<Vec<u8>> {
    let before = fs::metadata(path).map_err(|_| "source_unreadable")?;
    let b = storage::read(path, MAX_RAW)?;
    let after = fs::metadata(path).map_err(|_| "source_unreadable")?;
    if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
        return Err("source_changing".into());
    }
    Ok(b)
}
fn lines(bytes: &[u8]) -> Result<()> {
    for line in std::str::from_utf8(bytes)
        .map_err(|_| "session_invalid")?
        .lines()
        .filter(|s| !s.trim().is_empty())
    {
        serde_json::from_str::<serde_json::Value>(line).map_err(|_| "session_invalid")?;
    }
    Ok(())
}
fn sqlite_snapshot(path: &Path, staging: &Path) -> Result<Vec<u8>> {
    if fs::symlink_metadata(path)
        .map_err(|_| "source_unreadable")?
        .file_type()
        .is_symlink()
    {
        return Err("unsafe_store".into());
    }
    let source =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| "source_unreadable")?;
    let tables: i64=source.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('trajectory_meta','steps')",[],|r|r.get(0)).map_err(|_|"session_invalid")?;
    if tables != 2 {
        return Err("session_format_unsupported".into());
    }
    let tmp = tempfile::NamedTempFile::new_in(staging).map_err(|_| "store_unavailable")?;
    let mut target = rusqlite::Connection::open(tmp.path()).map_err(|_| "store_unavailable")?;
    let backup =
        rusqlite::backup::Backup::new(&source, &mut target).map_err(|_| "source_changing")?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        match backup.step(128).map_err(|_| "source_changing")? {
            rusqlite::backup::StepResult::Done => break,
            _ if std::time::Instant::now() > deadline => return Err("source_changing".into()),
            _ => std::thread::sleep(std::time::Duration::from_millis(10)),
        }
    }
    drop(backup);
    drop(target);
    storage::read(tmp.path(), MAX_RAW)
}
// ── Hermes Agent: one SQLite store (state.db) holds every session ──────────
// The live store is never copied raw and never written. A consistent
// point-in-time image is taken with the SQLite online backup API, then a fresh,
// minimal, KNOWN store is rebuilt containing exactly one session. Rebuilding
// (rather than copying and deleting) guarantees that other sessions, foreign
// tables, FTS indexes/triggers/views, gateway routing and runtime leases cannot
// leak into an export, and makes the export a pure function of the target
// session's own data so an unrelated session's edits never change it.
//
// The schema below is sourced from the installed Hermes source
// (`hermes_state_common.py`, SCHEMA_VERSION 31): tables + indexes only. FTS
// objects, triggers and views are deliberately absent — Hermes recreates them
// when it next opens the restored store, and excluding them keeps a received
// database free of foreign SQL that would run on open.
const HERMES_SCHEMA_VERSION: i64 = 31;
const HERMES_TABLES: [&str; 13] = [
    "schema_version",
    "system_prompts",
    "sessions",
    "messages",
    "session_model_usage",
    "state_meta",
    "gateway_routing",
    "gateway_hygiene_state",
    "conversation_generations",
    "gateway_heartbeats",
    "compression_locks",
    "session_turn_leases",
    "async_delegations",
];
const HERMES_SCHEMA_DDL: &str = r#"
CREATE TABLE schema_version (version INTEGER NOT NULL);
CREATE TABLE system_prompts (hash TEXT PRIMARY KEY, prompt TEXT NOT NULL);
CREATE TABLE sessions (
    id TEXT PRIMARY KEY, source TEXT NOT NULL, created_source TEXT, user_id TEXT,
    session_key TEXT, chat_id TEXT, chat_type TEXT, thread_id TEXT, display_name TEXT,
    origin_json TEXT, expiry_finalized INTEGER DEFAULT 0, model TEXT, model_config TEXT,
    system_prompt TEXT, system_prompt_hash TEXT, parent_session_id TEXT,
    started_at REAL NOT NULL, ended_at REAL, end_reason TEXT, message_count INTEGER DEFAULT 0,
    tool_call_count INTEGER DEFAULT 0, input_tokens INTEGER DEFAULT 0,
    output_tokens INTEGER DEFAULT 0, cache_read_tokens INTEGER DEFAULT 0,
    cache_write_tokens INTEGER DEFAULT 0, reasoning_tokens INTEGER DEFAULT 0, cwd TEXT,
    git_branch TEXT, git_repo_root TEXT, git_metadata_generation INTEGER NOT NULL DEFAULT 0,
    billing_provider TEXT, billing_base_url TEXT, billing_mode TEXT,
    estimated_cost_usd REAL, actual_cost_usd REAL, cost_status TEXT, cost_source TEXT,
    pricing_version TEXT, title TEXT, title_source TEXT, last_activity_at REAL,
    last_activity_description TEXT, last_activity_provenance TEXT, api_call_count INTEGER DEFAULT 0,
    handoff_state TEXT, handoff_platform TEXT, handoff_error TEXT,
    compression_failure_cooldown_until REAL, compression_failure_error TEXT,
    compression_fallback_streak INTEGER NOT NULL DEFAULT 0,
    compression_ineffective_count INTEGER NOT NULL DEFAULT 0,
    compression_recovery_deadline REAL, compression_overload_streak INTEGER NOT NULL DEFAULT 0,
    profile_name TEXT, transport_profile TEXT, rewind_count INTEGER NOT NULL DEFAULT 0,
    archived INTEGER NOT NULL DEFAULT 0, auto_archived INTEGER NOT NULL DEFAULT 0,
    pinned INTEGER NOT NULL DEFAULT 0, hidden INTEGER NOT NULL DEFAULT 0, last_read_at REAL,
    tool_names TEXT,
    FOREIGN KEY (parent_session_id) REFERENCES sessions(id),
    FOREIGN KEY (system_prompt_hash) REFERENCES system_prompts(hash)
);
CREATE TABLE messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT, session_id TEXT NOT NULL REFERENCES sessions(id),
    role TEXT NOT NULL, content TEXT, tool_call_id TEXT, tool_calls TEXT, tool_name TEXT,
    effect_disposition TEXT, timestamp REAL NOT NULL, token_count INTEGER, finish_reason TEXT,
    reasoning TEXT, reasoning_content TEXT, reasoning_details TEXT, codex_reasoning_items TEXT,
    codex_message_items TEXT, platform_message_id TEXT, observed INTEGER DEFAULT 0,
    _compressed_summary INTEGER NOT NULL DEFAULT 0, active INTEGER NOT NULL DEFAULT 1,
    compacted INTEGER NOT NULL DEFAULT 0, api_content TEXT, display_kind TEXT,
    display_metadata TEXT, display_identity BLOB, display_order INTEGER, message_uid TEXT,
    absorbed_message_uids TEXT, tool_call_uids TEXT, tool_call_uid TEXT
);
CREATE TABLE session_model_usage (
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE, model TEXT NOT NULL,
    billing_provider TEXT NOT NULL DEFAULT '', billing_base_url TEXT NOT NULL DEFAULT '',
    billing_mode TEXT NOT NULL DEFAULT '', task TEXT NOT NULL DEFAULT '',
    api_call_count INTEGER NOT NULL DEFAULT 0, input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0, cache_read_tokens INTEGER NOT NULL DEFAULT 0,
    cache_write_tokens INTEGER NOT NULL DEFAULT 0, reasoning_tokens INTEGER NOT NULL DEFAULT 0,
    estimated_cost_usd REAL NOT NULL DEFAULT 0, actual_cost_usd REAL NOT NULL DEFAULT 0,
    cost_status TEXT, cost_source TEXT, first_seen REAL, last_seen REAL,
    PRIMARY KEY (session_id, model, billing_provider, billing_base_url, billing_mode, task)
);
CREATE TABLE state_meta (key TEXT PRIMARY KEY, value TEXT);
CREATE TABLE gateway_routing (
    scope TEXT NOT NULL DEFAULT '', session_key TEXT NOT NULL, entry_json TEXT NOT NULL,
    updated_at REAL NOT NULL, PRIMARY KEY (scope, session_key)
);
CREATE TABLE gateway_hygiene_state (
    session_key TEXT PRIMARY KEY, failure_streak INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE conversation_generations (
    source TEXT NOT NULL, session_key TEXT NOT NULL, generation INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (source, session_key)
);
CREATE TABLE gateway_heartbeats (
    backend_id TEXT PRIMARY KEY, pid INTEGER NOT NULL, started_at REAL NOT NULL,
    last_heartbeat REAL NOT NULL, profile TEXT NOT NULL DEFAULT '', host TEXT NOT NULL DEFAULT ''
);
CREATE TABLE compression_locks (
    session_id TEXT PRIMARY KEY, holder TEXT NOT NULL, acquired_at REAL NOT NULL,
    expires_at REAL NOT NULL
);
CREATE TABLE session_turn_leases (
    conversation_id TEXT PRIMARY KEY, holder TEXT NOT NULL, acquired_at REAL NOT NULL,
    expires_at REAL NOT NULL
);
CREATE TABLE async_delegations (
    delegation_id TEXT PRIMARY KEY, origin_session TEXT NOT NULL,
    origin_ui_session_id TEXT NOT NULL DEFAULT '', parent_session_id TEXT, state TEXT NOT NULL,
    dispatched_at REAL NOT NULL, completed_at REAL, updated_at REAL NOT NULL,
    event_json TEXT, result_json TEXT, delivery_state TEXT NOT NULL DEFAULT 'pending',
    delivery_attempts INTEGER NOT NULL DEFAULT 0, delivered_at REAL, owner_pid INTEGER,
    owner_started_at INTEGER, task_json TEXT, delivery_claim TEXT, delivery_claimed_at REAL,
    origin_session_id TEXT NOT NULL DEFAULT ''
);
CREATE INDEX idx_sessions_source ON sessions(source);
CREATE INDEX idx_sessions_source_id ON sessions(source, id);
CREATE INDEX idx_sessions_parent ON sessions(parent_session_id);
CREATE INDEX idx_sessions_started ON sessions(started_at DESC);
CREATE INDEX idx_messages_session ON messages(session_id, timestamp);
CREATE INDEX idx_messages_session_id ON messages(session_id, id);
CREATE INDEX idx_messages_assistant_calls_by_session
    ON messages(session_id) WHERE role = 'assistant' AND tool_calls IS NOT NULL;
CREATE INDEX idx_session_model_usage_session ON session_model_usage(session_id);
CREATE INDEX idx_session_model_usage_model ON session_model_usage(model);
"#;
fn hermes_stat(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path).map_err(|_| "source_unreadable")?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err("unsafe_store".into());
    }
    Ok(())
}
fn hermes_columns(conn: &rusqlite::Connection, table: &str) -> Result<Vec<String>> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info('{table}')"))
        .map_err(|_| "session_invalid")?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|_| "session_invalid")?;
    let mut names = Vec::new();
    for row in rows {
        names.push(row.map_err(|_| "session_invalid")?);
    }
    Ok(names)
}
fn hermes_is_store(conn: &rusqlite::Connection) -> Result<()> {
    let tables: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN \
             ('sessions','messages','system_prompts','schema_version')",
            [],
            |r| r.get(0),
        )
        .map_err(|_| "session_invalid")?;
    if tables != 4 {
        return Err("session_format_unsupported".into());
    }
    let (count, version): (i64, Option<i64>) = conn
        .query_row(
            "SELECT count(*), max(version) FROM schema_version",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| "session_invalid")?;
    if count != 1 || version != Some(HERMES_SCHEMA_VERSION) {
        return Err("session_format_unsupported".into());
    }
    // A shape we could not read back faithfully is refused rather than copied
    // with silently missing data.
    if !hermes_columns(conn, "sessions")?.iter().any(|c| c == "id")
        || !hermes_columns(conn, "messages")?
            .iter()
            .any(|c| c == "session_id")
    {
        return Err("session_format_unsupported".into());
    }
    Ok(())
}
fn hermes_open_readonly(path: &Path) -> Result<rusqlite::Connection> {
    rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| "source_unreadable".into())
}
// Consistent point-in-time image of the live store via the online backup API.
fn hermes_backup_bytes(source: &Path, staging: &Path) -> Result<Vec<u8>> {
    hermes_stat(source)?;
    let source_conn = hermes_open_readonly(source)?;
    hermes_is_store(&source_conn)?;
    let tmp = tempfile::NamedTempFile::new_in(staging).map_err(|_| "store_unavailable")?;
    let mut target = rusqlite::Connection::open(tmp.path()).map_err(|_| "store_unavailable")?;
    let backup =
        rusqlite::backup::Backup::new(&source_conn, &mut target).map_err(|_| "source_changing")?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        match backup.step(4096).map_err(|_| "source_changing")? {
            rusqlite::backup::StepResult::Done => break,
            _ if std::time::Instant::now() > deadline => return Err("source_changing".into()),
            _ => std::thread::sleep(std::time::Duration::from_millis(5)),
        }
    }
    drop(backup);
    drop(target);
    storage::read(tmp.path(), MAX_RAW)
}
// Copy known columns only. Reject schema drift rather than silently discarding
// message context or filling missing fields while claiming native fidelity.
fn hermes_copy(
    src: &rusqlite::Connection,
    dst: &rusqlite::Connection,
    table: &str,
    filter: &str,
    bind: &[rusqlite::types::Value],
    overrides: &[(&str, rusqlite::types::Value)],
) -> Result<usize> {
    let target = hermes_columns(dst, table)?;
    let available = hermes_columns(src, table)?;
    if available.len() != target.len() || available.iter().any(|name| !target.contains(name)) {
        return Err("session_format_unsupported".into());
    }
    let columns = target
        .iter()
        .filter(|column| available.iter().any(|name| name == *column))
        .cloned()
        .collect::<Vec<_>>();
    if columns.is_empty() {
        return Err("session_format_unsupported".into());
    }
    let list = columns.join(",");
    // Both the table name and the filter are compile-time constants and the
    // column list only ever contains names from our own canonical schema, so
    // nothing here is attacker-controlled SQL.
    let mut select = src
        .prepare(&format!("SELECT {list} FROM {table} WHERE {filter}"))
        .map_err(|_| "session_invalid")?;
    let rows = select
        .query_map(rusqlite::params_from_iter(bind.iter()), |row| {
            let mut values = Vec::with_capacity(columns.len());
            for index in 0..columns.len() {
                values.push(row.get::<_, rusqlite::types::Value>(index)?);
            }
            Ok(values)
        })
        .map_err(|_| "session_invalid")?;
    let placeholders = (1..=columns.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(",");
    let insert = format!("INSERT INTO {table} ({list}) VALUES ({placeholders})");
    let mut copied = 0usize;
    for row in rows {
        let mut values = row.map_err(|_| "session_invalid")?;
        for (name, value) in overrides {
            if let Some(position) = columns.iter().position(|column| column == name) {
                values[position] = value.clone();
            }
        }
        dst.execute(&insert, rusqlite::params_from_iter(values.iter()))
            .map_err(|_| "session_invalid")?;
        copied += 1;
    }
    Ok(copied)
}
// ── Hermes lineage graph ───────────────────────────────────────────────────
// A Hermes store keeps every session in one database and links them with
// `parent_session_id`. The connected, *required* lineage of a session — its
// ancestor chain plus its surviving lineage children — must travel with it so
// every reference resolves on the receiver. Child edges are explicit and match
// the installed source's own predicates (`hermes_state_common.py`):
//   * continuation — a compression continuation (parent `end_reason`
//     'compression'); a child that is itself a branch/delegate/reset fork or a
//     tool session is excluded and reached by its own explicit edge;
//   * branch — a `_branched_from` marker or the legacy 'branched' heuristic;
//   * reset — a `_reset_from` marker or the legacy same-key reset heuristic;
//   * delegated — a `_delegate_from` marker (an async delegation child) whose
//     completed task/result travels with the graph.
// The walk is bounded and cycle-safe: a self-parent or a parent cycle terminates
// on the visited set and never loops forever.
const HERMES_GRAPH_MAX: usize = 256;

fn sql_placeholders(n: usize) -> String {
    (1..=n)
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(",")
}
fn hermes_session_exists(conn: &rusqlite::Connection, id: &str) -> Result<bool> {
    match conn.query_row("SELECT 1 FROM sessions WHERE id = ?1", [id], |_| Ok(())) {
        Ok(()) => Ok(true),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(_) => Err("session_invalid".into()),
    }
}
fn hermes_session_cwd(conn: &rusqlite::Connection, id: &str) -> Result<String> {
    conn.query_row(
        "SELECT COALESCE(cwd,'') FROM sessions WHERE id = ?1",
        [id],
        |r| r.get::<_, String>(0),
    )
    .map_err(|_| "session_invalid".into())
}
fn hermes_parent_of(conn: &rusqlite::Connection, id: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT parent_session_id FROM sessions WHERE id = ?1",
        [id],
        |r| r.get::<_, Option<String>>(0),
    )
    .map_err(|_| "session_invalid".into())
}
// ── Explicit lineage edge types ────────────────────────────────────────────
// Each required child edge is one of four explicit types, matching the installed
// source's own predicate. A delegate child (an async sub-agent delegation) is a
// *delegated* edge, never a compression continuation; the compression edge
// excludes branch/delegate/reset/tool children exactly as the installed
// `_expand_compression_lineage_ids` child_filter does. `json_extract` is guarded
// by `json_valid` (inside a CASE) so a malformed `model_config` row can never
// make the scan raise.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LineageEdge {
    Continuation,
    Branch,
    Reset,
    Delegated,
}

const HERMES_RESET_END_REASONS_SQL: &str =
    "'session_reset', 'session_switch', 'idle', 'daily', 'suspended', 'resume_pending_expired'";

/// A non-throwing JSON marker lookup for `<alias>.model_config`, mirroring
/// `hermes_state_common._sql_json_extract`.
fn hermes_marker(alias: &str, marker: &str) -> String {
    let col = format!("{alias}.model_config");
    format!(
        "(SELECT json_extract(CASE WHEN json_valid({col}) THEN {col} ELSE json_object() END, '$.{marker}'))"
    )
}

/// `_reset_from` marker or the legacy same-session-key reset heuristic
/// (`hermes_state_common._RESET_CHILD_SQL`).
fn hermes_reset_child_sql(alias: &str) -> String {
    format!(
        "({} IS NOT NULL OR EXISTS (SELECT 1 FROM sessions p \
         WHERE p.id = {alias}.parent_session_id AND p.end_reason IN ({}) \
         AND {alias}.session_key IS NOT NULL AND {alias}.session_key != '' \
         AND {alias}.session_key = p.session_key))",
        hermes_marker(alias, "_reset_from"),
        HERMES_RESET_END_REASONS_SQL,
    )
}

/// `_branched_from` marker or the legacy parent-ended-`branched` heuristic
/// (`hermes_state_common._BRANCH_CHILD_SQL`).
fn hermes_branch_child_sql(alias: &str) -> String {
    format!(
        "({} IS NOT NULL OR EXISTS (SELECT 1 FROM sessions p \
         WHERE p.id = {alias}.parent_session_id AND p.end_reason = 'branched' \
         AND {alias}.started_at >= COALESCE(p.ended_at, p.started_at)))",
        hermes_marker(alias, "_branched_from"),
    )
}

/// A child that is NOT a compression continuation of its parent: a branch,
/// delegate or reset fork, or a tool session (`_non_continuation_child_sql`).
fn hermes_non_continuation_sql(alias: &str) -> String {
    format!(
        "({} IS NULL AND {} IS NULL AND NOT {} AND COALESCE({alias}.source, '') != 'tool')",
        hermes_marker(alias, "_branched_from"),
        hermes_marker(alias, "_delegate_from"),
        hermes_reset_child_sql(alias),
    )
}

/// Required children of `parent` reached by one explicit edge type.
fn hermes_edge_children(
    conn: &rusqlite::Connection,
    parent: &str,
    edge: LineageEdge,
) -> Result<Vec<String>> {
    let predicate = match edge {
        LineageEdge::Continuation => format!(
            "p.end_reason = 'compression' AND {}",
            hermes_non_continuation_sql("c")
        ),
        LineageEdge::Branch => hermes_branch_child_sql("c"),
        LineageEdge::Reset => hermes_reset_child_sql("c"),
        LineageEdge::Delegated => format!("{} IS NOT NULL", hermes_marker("c", "_delegate_from")),
    };
    // `predicate` is built only from the canonical marker/predicate helpers
    // above; the child id is bound, never interpolated.
    let mut stmt = conn
        .prepare(&format!(
            "SELECT c.id FROM sessions c JOIN sessions p ON p.id = c.parent_session_id \
             WHERE c.parent_session_id = ?1 AND ({predicate}) ORDER BY c.id"
        ))
        .map_err(|_| "session_invalid")?;
    let rows = stmt
        .query_map([parent], |r| r.get::<_, String>(0))
        .map_err(|_| "session_invalid")?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|_| "session_invalid")?);
        if out.len() > HERMES_GRAPH_MAX {
            return Err("session_limit".into());
        }
    }
    Ok(out)
}

/// Required lineage children of a parent, deduplicated across every explicit
/// edge type. A delegate child is reached only by the *delegated* edge; a
/// compression continuation excludes any child that is itself a branch, delegate
/// or reset fork or a tool session.
fn hermes_lineage_children(conn: &rusqlite::Connection, id: &str) -> Result<Vec<String>> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out = Vec::new();
    for edge in [
        LineageEdge::Continuation,
        LineageEdge::Branch,
        LineageEdge::Reset,
        LineageEdge::Delegated,
    ] {
        for child in hermes_edge_children(conn, id, edge)? {
            if seen.insert(child.clone()) {
                out.push(child);
            }
        }
    }
    out.sort();
    Ok(out)
}
// Connected required lineage of `root`. Returns the sorted session ids plus any
// non-fatal issue codes. A parent id the store references but does not contain
// cannot be captured; it is reported (`hermes_parent_missing`) and the dangling
// reference is cleared rather than silently followed.
fn hermes_graph_sessions(
    conn: &rusqlite::Connection,
    root: &str,
) -> Result<(Vec<String>, Vec<String>)> {
    if !bundle::token(root) || !hermes_session_exists(conn, root)? {
        return Err("session_invalid".into());
    }
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut order: Vec<String> = Vec::new();
    let mut issues: Vec<String> = Vec::new();
    let mut current = Some(root.to_string());
    while let Some(id) = current {
        if !seen.insert(id.clone()) || seen.len() > HERMES_GRAPH_MAX {
            break;
        }
        order.push(id.clone());
        match hermes_parent_of(conn, &id)? {
            Some(parent) if !parent.is_empty() && parent != id => {
                if !hermes_session_exists(conn, &parent)? {
                    issues.push("hermes_parent_missing".into());
                    break;
                }
                current = Some(parent);
            }
            _ => break,
        }
    }
    let mut queue = order.clone();
    while let Some(id) = queue.pop() {
        for child in hermes_lineage_children(conn, &id)? {
            if seen.insert(child.clone()) {
                if seen.len() > HERMES_GRAPH_MAX {
                    return Err("session_limit".into());
                }
                order.push(child.clone());
                queue.push(child);
            }
        }
    }
    order.sort();
    Ok((order, issues))
}
fn hermes_assert_graph_consistent(conn: &rusqlite::Connection) -> Result<()> {
    let dangling: i64 = conn
        .query_row(
            "SELECT count(*) FROM sessions c WHERE c.parent_session_id IS NOT NULL \
             AND c.parent_session_id != '' \
             AND NOT EXISTS (SELECT 1 FROM sessions p WHERE p.id = c.parent_session_id)",
            [],
            |r| r.get(0),
        )
        .map_err(|_| "session_invalid")?;
    if dangling != 0 {
        return Err("session_format_unsupported".into());
    }
    Ok(())
}
// Rebuild a fresh canonical store holding the connected required lineage of
// `root`. The result has no other sessions, no foreign tables, no triggers/views
// and no FTS leftovers; runtime/gateway tables exist but are empty. The root's
// own `cwd` is overridden to the (possibly remapped) value while every other
// session's recorded metadata is preserved verbatim. `parent_session_id` is kept
// whenever its parent is inside the exported graph and only cleared when the
// store itself lacks the parent (a dangling foreign key).
fn hermes_canonicalize_with_issues(
    source: &rusqlite::Connection,
    session: &str,
    cwd: &str,
    staging: &Path,
) -> Result<(Vec<u8>, Vec<String>)> {
    if !bundle::token(session) {
        return Err("session_invalid".into());
    }
    hermes_is_store(source)?;
    let (graph, issues) = hermes_graph_sessions(source, session)?;
    let tmp = tempfile::NamedTempFile::new_in(staging)
        .map_err(|_| "store_unavailable")?
        .into_temp_path();
    {
        let target = rusqlite::Connection::open(&tmp).map_err(|_| "store_unavailable")?;
        // rusqlite ships SQLite with foreign_keys ON; turn it off so a store whose
        // rows reference a prompt outside this isolated export still copies
        // cleanly. Rows are inserted parent-first anyway.
        target
            .execute_batch("PRAGMA foreign_keys=OFF;")
            .map_err(|_| "store_unavailable")?;
        target
            .execute_batch(HERMES_SCHEMA_DDL)
            .map_err(|_| "store_unavailable")?;
        target
            .execute(
                "INSERT INTO schema_version(version) VALUES (?1)",
                [HERMES_SCHEMA_VERSION],
            )
            .map_err(|_| "store_unavailable")?;
        let ids = graph
            .iter()
            .map(|id| rusqlite::types::Value::Text(id.clone()))
            .collect::<Vec<_>>();
        hermes_copy(
            source,
            &target,
            "system_prompts",
            &format!(
                "hash IN (SELECT system_prompt_hash FROM sessions WHERE id IN ({}))",
                sql_placeholders(graph.len())
            ),
            &ids,
            &[],
        )?;
        for id in &graph {
            let mut overrides: Vec<(&str, rusqlite::types::Value)> = Vec::new();
            if id == session {
                overrides.push(("cwd", rusqlite::types::Value::Text(cwd.to_string())));
            }
            let parent = hermes_parent_of(source, id)?;
            let parent_inside = parent
                .as_deref()
                .is_some_and(|value| !value.is_empty() && graph.iter().any(|g| g == value));
            if parent.as_deref().is_some_and(|value| !value.is_empty()) && !parent_inside {
                overrides.push(("parent_session_id", rusqlite::types::Value::Null));
            }
            let idv = vec![rusqlite::types::Value::Text(id.clone())];
            if hermes_copy(source, &target, "sessions", "id = ?1", &idv, &overrides)? != 1 {
                return Err("session_invalid".into());
            }
            hermes_copy(source, &target, "messages", "session_id = ?1", &idv, &[])?;
            hermes_copy(
                source,
                &target,
                "session_model_usage",
                "session_id = ?1",
                &idv,
                &[],
            )?;
        }
        target
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;")
            .map_err(|_| "session_invalid")?;
        // The connection is dropped here so every page (and any rollback/WAL
        // flush) is on disk before the file is read back; Windows will not let a
        // file with a live exclusive SQLite handle be read.
    }
    Ok((storage::read(&tmp, MAX_RAW)?, issues))
}
fn hermes_canonicalize(
    source: &rusqlite::Connection,
    session: &str,
    cwd: &str,
    staging: &Path,
) -> Result<Vec<u8>> {
    hermes_canonicalize_with_issues(source, session, cwd, staging).map(|(bytes, _)| bytes)
}
fn hermes_with_image<T>(
    bytes: &[u8],
    staging: &Path,
    action: impl FnOnce(&rusqlite::Connection) -> Result<T>,
) -> Result<T> {
    let path = tempfile::NamedTempFile::new_in(staging)
        .map_err(|_| "store_unavailable")?
        .into_temp_path();
    fs::write(&path, bytes).map_err(|_| "store_unavailable")?;
    let conn = hermes_open_readonly(&path)?;
    let out = action(&conn)?;
    drop(conn);
    drop(path);
    Ok(out)
}
fn hermes_sessions_of(conn: &rusqlite::Connection) -> Result<Vec<(String, String)>> {
    hermes_is_store(conn)?;
    let mut stmt = conn
        .prepare("SELECT id, COALESCE(cwd,'') FROM sessions ORDER BY id")
        .map_err(|_| "session_invalid")?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(|_| "session_invalid")?;
    let mut out = Vec::new();
    for row in rows {
        let (id, cwd) = row.map_err(|_| "session_invalid")?;
        if !bundle::token(&id) {
            continue;
        }
        out.push((id, cwd));
        if out.len() > 20000 {
            return Err("session_limit".into());
        }
    }
    Ok(out)
}
// A received store must be the exact canonical shape we produce: only allowlisted
// tables and no trigger or view. A foreign trigger would run arbitrary SQL the
// moment the user opens the restored profile, and a leftover FTS index would leak
// another session's content.
fn hermes_assert_canonical(bytes: &[u8], staging: &Path) -> Result<()> {
    hermes_with_image(bytes, staging, |conn| {
        let unsafe_objects: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type IN ('trigger','view')",
                [],
                |r| r.get(0),
            )
            .map_err(|_| "session_invalid")?;
        if unsafe_objects != 0 {
            return Err("session_format_unsupported".into());
        }
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .map_err(|_| "session_invalid")?;
        let names = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|_| "session_invalid")?;
        for name in names {
            let name = name.map_err(|_| "session_invalid")?;
            if name.starts_with("sqlite_") {
                continue;
            }
            if !HERMES_TABLES.contains(&name.as_str()) {
                return Err("session_format_unsupported".into());
            }
        }
        Ok(())
    })
}
pub(crate) fn validate_hermes_manifest(manifest: &Manifest) -> Result<()> {
    if manifest.agent != "hermes" || !manifest.files.contains_key("state.db") {
        return Err("project_mapping_format_unsupported".into());
    }
    if !manifest_paths_safe(manifest) {
        return Err("project_mapping_format_unsupported".into());
    }
    let bytes = STANDARD
        .decode(manifest.files.get("state.db").ok_or("session_invalid")?)
        .map_err(|_| "session_invalid")?;
    if !bytes.starts_with(b"SQLite format 3\0") {
        return Err("session_invalid".into());
    }
    let dir = tempfile::tempdir().map_err(|_| "store_unavailable")?;
    hermes_with_image(&bytes, dir.path(), |conn| {
        // The manifest's session id is the unambiguous root of the exported
        // lineage graph, even when the store holds many sessions.
        if !hermes_session_exists(conn, &manifest.session)? {
            return Err("project_mapping_format_unsupported".into());
        }
        if hermes_session_cwd(conn, &manifest.session)? != manifest.cwd {
            return Err("project_mapping_format_unsupported".into());
        }
        hermes_assert_graph_consistent(conn)?;
        Ok(())
    })?;
    hermes_assert_canonical(&bytes, dir.path())?;
    hermes_validate_workspace_files(&manifest.files)?;
    hermes_validate_attachments(&manifest.files)?;
    Ok(())
}
// Rebuild the received store into the canonical schema even when no mapping
// applies: a bundle that predates the rebuild exporter, or a tampered one, can
// therefore never place a foreign trigger, view or extra table into a managed
// profile. Writes always happen against this rebuilt, known-schema database.
// Non-native companion files (portable workspace and content-hashed attachments)
// are preserved; only `state.db` is rebuilt.
pub(crate) fn transform_hermes_manifest(
    manifest: &Manifest,
    mappings: &[crate::project_mapping::Mapping],
) -> Result<Manifest> {
    let bytes = STANDARD
        .decode(
            manifest
                .files
                .get("state.db")
                .ok_or("project_mapping_format_unsupported")?,
        )
        .map_err(|_| "session_invalid")?;
    if !bytes.starts_with(b"SQLite format 3\0") {
        return Err("session_invalid".into());
    }
    let dir = tempfile::tempdir().map_err(|_| "store_unavailable")?;
    let session = manifest.session.clone();
    let (cwd, rebuilt) = hermes_with_image(&bytes, dir.path(), |conn| {
        if !hermes_session_exists(conn, &session)? {
            return Err("project_mapping_format_unsupported".into());
        }
        let recorded = hermes_session_cwd(conn, &session)?;
        let cwd = if mappings.is_empty() || recorded.is_empty() {
            recorded
        } else {
            crate::project_mapping::remap_path(&recorded, mappings)?.unwrap_or(recorded)
        };
        let rebuilt = hermes_canonicalize(conn, &session, &cwd, dir.path())?;
        Ok((cwd, rebuilt))
    })?;
    // Project-rooted attachment references follow their project mapping
    // (Windows/POSIX aware, via the shared mapping code).
    let rebuilt = hermes_rewrite_references(&rebuilt, dir.path(), |kind, target| {
        if kind != "file" && kind != "folder" {
            return Ok(None);
        }
        if !hermes_is_absolute_target(target) {
            return Ok(None);
        }
        crate::project_mapping::remap_path(target, mappings)
    })?;
    let mut files = manifest.files.clone();
    files.insert("state.db".into(), STANDARD.encode(rebuilt));
    Ok(Manifest {
        cwd,
        files,
        ..manifest.clone()
    })
}

// ── Content-hashed referenced attachments ──────────────────────────────────
// A user message may explicitly reference a local file with `@file:<path>` (or
// `@folder:`), the same syntax the installed runtime expands
// (`agent/context_references.py`). Only those *explicitly referenced* files are
// captured, and only when they sit under a selected root (the Hermes home or a
// recorded session cwd) — there is no directory enumeration. Symlinks and
// credential files are refused, sizes are bounded and every blob is keyed by its
// SHA-256. Unresolvable references are reported as explicit problems, never
// silently dropped.
const HERMES_ATTACHMENT_MAX: u64 = 8 * 1024 * 1024;
const HERMES_ATTACHMENT_TOTAL: usize = 32 * 1024 * 1024;
const HERMES_ATTACHMENT_FILES: usize = 64;
const HERMES_ATTACHMENT_INDEX: &str = "attachments/index.json";
/// Inert async-delegation sidecar. Finished `async_delegations` rows travel here
/// as data only — never into `state.db`'s live `async_delegations` table — so a
/// received snapshot can never re-lease or replay a runtime job.
const HERMES_ASYNC_INDEX: &str = "bastet/hermes-async-delegations.json";
const HERMES_ASYNC_MAX: usize = 1024 * 1024;
const HERMES_ASYNC_RECORDS: usize = 512;
const HERMES_ASYNC_FIELD_MAX: usize = 128 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AttachmentEntry {
    reference: String,
    kind: String,
    target: String,
    source_path: String,
    sha256: String,
    key: String,
    bytes: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AttachmentProblem {
    reference: String,
    kind: String,
    reason: String,
    blocking: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AttachmentIndex {
    version: u32,
    agent: String,
    root: String,
    entries: Vec<AttachmentEntry>,
    problems: Vec<AttachmentProblem>,
}
struct AttachmentCapture {
    files: BTreeMap<String, String>,
    entries: Vec<AttachmentEntry>,
    problems: Vec<AttachmentProblem>,
}
fn hermes_attachment_path(path: &str) -> bool {
    if !crate::portable_paths::path_is_portable(path) {
        return false;
    }
    if path == HERMES_ATTACHMENT_INDEX {
        return true;
    }
    let mut parts = path.split('/');
    if parts.next() != Some("attachments") {
        return false;
    }
    let Some(sha) = parts.next() else {
        return false;
    };
    if !bundle::is_hash(sha) {
        return false;
    }
    let Some(name) = parts.next() else {
        return false;
    };
    parts.next().is_none() && !name.is_empty()
}
fn hermes_trim_reference(value: &str) -> &str {
    let mut trimmed = value.trim_end_matches([',', '.', ';', '!', '?']);
    while let Some(last) = trimmed.chars().last() {
        let (open, close) = match last {
            ')' => ('(', ')'),
            ']' => ('[', ']'),
            '}' => ('{', '}'),
            _ => break,
        };
        let _ = close;
        if trimmed.matches(last).count() > trimmed.matches(open).count() {
            trimmed = &trimmed[..trimmed.len() - last.len_utf8()];
        } else {
            break;
        }
    }
    trimmed
}
fn hermes_strip_line_range(value: &str) -> &str {
    if let Some(index) = value.rfind(':') {
        let suffix = &value[index + 1..];
        let digits = !suffix.is_empty()
            && suffix
                .split('-')
                .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()));
        if digits {
            return &value[..index];
        }
    }
    value
}
// Explicit `@file:` / `@folder:` references with their byte span in `content`.
fn hermes_reference_spans(content: &str) -> Vec<(usize, usize, String, String)> {
    let bytes = content.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'@' {
            i += 1;
            continue;
        }
        if i > 0 {
            let prev = bytes[i - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' || prev == b'/' {
                i += 1;
                continue;
            }
        }
        let after_at = i + 1;
        let (kind, prefix) = if content[after_at..].starts_with("file:") {
            ("file", 5)
        } else if content[after_at..].starts_with("folder:") {
            ("folder", 7)
        } else if content[after_at..].starts_with("image:") {
            // The desktop/gateway persists attached images as `@image:<path>`
            // (tui_gateway/session_history.py, gateway/platforms/event.py).
            ("image", 6)
        } else if content[after_at..].starts_with("url:") {
            // `@url:` is a remote reference, never a local file.
            ("url", 4)
        } else {
            i += 1;
            continue;
        };
        let mut j = after_at + prefix;
        let (vstart, vend);
        if j < bytes.len() && matches!(bytes[j], b'`' | b'"' | b'\'') {
            let quote = bytes[j];
            vstart = j + 1;
            match content[vstart..].find(quote as char) {
                Some(offset) => {
                    vend = vstart + offset;
                    j = vend + 1;
                }
                None => {
                    i += 1;
                    continue;
                }
            }
        } else {
            vstart = j;
            let mut k = vstart;
            while k < bytes.len() && !bytes[k].is_ascii_whitespace() {
                k += 1;
            }
            vend = k;
            j = k;
        }
        let raw = hermes_trim_reference(&content[vstart..vend]);
        let target = if kind == "file" {
            hermes_strip_line_range(raw).trim()
        } else {
            raw.trim()
        };
        if !target.is_empty() {
            spans.push((i, j, kind.to_string(), target.to_string()));
        }
        i = i.max(j).max(i + 1);
        if j <= i {
            i += 1;
        }
    }
    spans
}
fn hermes_references(text: &str) -> Vec<(String, String, String)> {
    hermes_reference_spans(text)
        .into_iter()
        .map(|(_, _, kind, target)| {
            let reference = format!("@{kind}:{target}");
            (reference, kind, target)
        })
        .collect()
}
// A target the mapping code can resolve on this host. Windows drive and UNC
// forms count as absolute even on POSIX, so a Windows-authored reference is
// remapped when received on POSIX (and vice versa).
fn hermes_is_absolute_target(target: &str) -> bool {
    if Path::new(target).is_absolute() {
        return true;
    }
    let bytes = target.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'/' || bytes[2] == b'\\')
    {
        return true;
    }
    target.starts_with("\\\\") || target.starts_with("//")
}
fn hermes_quote_reference(value: &str) -> String {
    if value.chars().any(|c| {
        c.is_whitespace()
            || matches!(
                c,
                '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | '"' | '\'' | '`'
            )
    }) {
        if !value.contains('`') {
            return format!("`{value}`");
        }
        if !value.contains('"') {
            return format!("\"{value}\"");
        }
    }
    value.to_string()
}
// Rewrite explicit `@file:`/`@folder:` targets in every message. `f` returns the
// replacement target (or None to leave the token untouched). Returns the original
// bytes when nothing changed so deduplication is preserved.
fn hermes_rewrite_references(
    bytes: &[u8],
    staging: &Path,
    f: impl Fn(&str, &str) -> Result<Option<String>>,
) -> Result<Vec<u8>> {
    let path = tempfile::NamedTempFile::new_in(staging)
        .map_err(|_| "store_unavailable")?
        .into_temp_path();
    fs::write(&path, bytes).map_err(|_| "store_unavailable")?;
    let conn = rusqlite::Connection::open(&path).map_err(|_| "store_unavailable")?;
    let mut updates: Vec<(i64, Option<String>, Option<String>)> = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT id, content, api_content FROM messages")
            .map_err(|_| "session_invalid")?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(|_| "session_invalid")?;
        for row in rows {
            let (id, content, api) = row.map_err(|_| "session_invalid")?;
            let new_content = content
                .as_deref()
                .map(|text| hermes_rewrite_text(text, &f))
                .transpose()?
                .flatten();
            let new_api = api
                .as_deref()
                .map(|text| hermes_rewrite_text(text, &f))
                .transpose()?
                .flatten();
            if new_content.is_some() || new_api.is_some() {
                updates.push((id, new_content.or(content), new_api.or(api)));
            }
        }
    }
    if updates.is_empty() {
        return Ok(bytes.to_vec());
    }
    for (id, content, api) in &updates {
        conn.execute(
            "UPDATE messages SET content=?2, api_content=?3 WHERE id=?1",
            rusqlite::params![id, content, api],
        )
        .map_err(|_| "session_invalid")?;
    }
    drop(conn);
    storage::read(&path, MAX_RAW)
}
fn hermes_rewrite_text(
    text: &str,
    f: &impl Fn(&str, &str) -> Result<Option<String>>,
) -> Result<Option<String>> {
    let spans = hermes_reference_spans(text);
    if spans.is_empty() {
        return Ok(None);
    }
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    let mut changed = false;
    for (start, end, kind, target) in spans {
        if start < last {
            continue;
        }
        out.push_str(&text[last..start]);
        match f(&kind, &target)? {
            Some(new_target) => {
                out.push_str(&format!("@{kind}:{}", hermes_quote_reference(&new_target)));
                changed = true;
            }
            None => out.push_str(&text[start..end]),
        }
        last = end;
    }
    out.push_str(&text[last..]);
    Ok(changed.then_some(out))
}
fn hermes_attachment_name(path: &Path) -> String {
    let raw = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "attachment".into());
    let cleaned: String = raw
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*' | '/') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned.trim_end_matches([' ', '.']);
    let candidate = if trimmed.is_empty() || trimmed.len() > 200 {
        "attachment".to_string()
    } else {
        trimmed.to_string()
    };
    if crate::portable_paths::path_is_portable(&candidate) {
        candidate
    } else {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .filter(|e| e.bytes().all(|b| b.is_ascii_alphanumeric()))
            .map(|e| format!(".{e}"))
            .unwrap_or_default();
        format!("attachment{ext}")
    }
}
fn hermes_credential_name(path: &Path) -> bool {
    const DENY: [&str; 14] = [
        ".env",
        "auth.json",
        ".netrc",
        ".pgpass",
        ".npmrc",
        ".pypirc",
        ".bashrc",
        ".zshrc",
        ".profile",
        ".bash_profile",
        ".zprofile",
        "config.yaml",
        ".envrc",
        ".htpasswd",
    ];
    if path
        .components()
        .any(|c| c.as_os_str() == ".ssh" || c.as_os_str() == ".gnupg")
    {
        return true;
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    DENY.contains(&name.as_str())
        || name.contains("credential")
        || name.contains("token")
        || name.contains("secret")
        || name.ends_with(".pem")
        || name.ends_with(".key")
        || name.starts_with("id_rsa")
        || name.starts_with("id_ed25519")
}
fn hermes_expand_target(target: &str) -> String {
    if target == "~" {
        if let Some(home) = std::env::var_os("HOME") {
            return home.to_string_lossy().into_owned();
        }
    }
    if let Some(rest) = target
        .strip_prefix("~/")
        .or_else(|| target.strip_prefix("~\\"))
    {
        if let Some(home) = std::env::var_os("HOME") {
            return Path::new(&home).join(rest).to_string_lossy().into_owned();
        }
    }
    target.to_string()
}
// Lexical normalisation: reject `..` escapes, keep everything else as written so
// the recorded path is the one the user referenced.
fn hermes_lexical(path: &Path) -> Result<PathBuf> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => return Err("outside_roots".into()),
            std::path::Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    Ok(out)
}
fn hermes_under(child: &Path, root: &Path) -> bool {
    child == root || child.starts_with(root)
}
enum AttachmentOutcome {
    Captured(PathBuf),
    Problem(&'static str),
}
// Read-denial sets ported from the installed `agent/file_safety.py` and
// `agent/context_references.py`. Exact credential files and directory prefixes
// live under both the active HERMES_HOME and its global root; `HOME_CREDENTIAL_*`
// cover the OS user's home; `PROJECT_ENV_*` cover secret env files anywhere.
const HERMES_CREDENTIAL_FILES: [&str; 8] = [
    "auth.json",
    "auth.lock",
    ".anthropic_oauth.json",
    ".env",
    "webhook_subscriptions.json",
    "auth/google_oauth.json",
    "cache/bws_cache.json",
    "cache/bws_cache.enc.json",
];
const HERMES_PROTECTED_SUBPATHS: [&str; 6] = [
    "state.db",
    "sessions",
    "mcp-tokens",
    "pairing",
    "vault",
    "browser-profile",
];
const HOME_CREDENTIAL_DIRS: [&str; 8] = [
    ".ssh",
    ".aws",
    ".gnupg",
    ".kube",
    ".docker",
    ".azure",
    ".config/gh",
    ".config/gcloud",
];
const HOME_CREDENTIAL_FILES: [&str; 8] = [
    ".ssh/authorized_keys",
    ".ssh/id_rsa",
    ".ssh/id_ed25519",
    ".netrc",
    ".pgpass",
    ".npmrc",
    ".pypirc",
    ".git-credentials",
];
const PROJECT_ENV_BASENAMES: [&str; 7] = [
    ".env",
    ".env.local",
    ".env.development",
    ".env.production",
    ".env.test",
    ".env.staging",
    ".envrc",
];
const SYSTEM_DENY_PREFIXES: [&str; 5] = [
    "/etc/passwd",
    "/etc/shadow",
    "/etc/sudoers",
    "/etc/sudoers.d",
    "/etc/systemd",
];
// Hermes runtime cache dirs that legitimately hold user-referenced files. Only
// these (plus the session's own project working directory) are readable roots —
// never the entire home, which would expose the credential stores above.
const HERMES_CACHE_DIRS: [&str; 11] = [
    "cache/documents",
    "cache/images",
    "cache/audio",
    "cache/videos",
    "cache/screenshots",
    "cache/web",
    "cache/delegation",
    "cache/spillover",
    "images",
    "attachments",
    "composer-pastes",
];
// The Hermes home and, for a profile home (`<root>/profiles/<name>`), its global
// root, so credential stores at either level stay guarded.
fn hermes_home_roots(home: &Path) -> Vec<PathBuf> {
    let mut roots = vec![home.to_path_buf()];
    if let Some(parent) = home.parent() {
        if parent.file_name().is_some_and(|n| n == "profiles") {
            if let Some(root) = parent.parent() {
                roots.push(root.to_path_buf());
            }
        }
    }
    roots.sort();
    roots.dedup();
    roots
}
// Windows NT/device-namespace prefixes: never legitimate targets, and resolving
// one can trigger an outbound SMB (NTLM) authentication, so they are refused on
// the RAW string before any path resolution.
fn hermes_nt_namespace(target: &str) -> bool {
    let s = target.replace('/', "\\");
    if s.starts_with("\\\\??\\") || s.starts_with("\\\\.\\") {
        return true;
    }
    if let Some(rest) = s.strip_prefix("\\\\?\\") {
        let upper = rest.to_ascii_uppercase();
        if upper.starts_with("UNC\\") || upper.starts_with("GLOBALROOT\\") {
            return true;
        }
    }
    false
}
// Canonical read denial for an already-resolved path.
fn hermes_read_denied(canonical: &Path, home: &Path, home_dir: Option<&Path>) -> bool {
    if canonical
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .is_some_and(|n| PROJECT_ENV_BASENAMES.contains(&n.as_str()))
    {
        return true;
    }
    for root in hermes_home_roots(home) {
        for name in HERMES_CREDENTIAL_FILES {
            if canonical == root.join(name) {
                return true;
            }
        }
        for sub in HERMES_PROTECTED_SUBPATHS {
            if hermes_under(canonical, &root.join(sub)) {
                return true;
            }
        }
        if hermes_under(canonical, &root.join("skills").join(".hub")) {
            return true;
        }
    }
    if let Some(home_dir) = home_dir {
        for dir in HOME_CREDENTIAL_DIRS {
            if hermes_under(canonical, &home_dir.join(dir)) {
                return true;
            }
        }
        for name in HOME_CREDENTIAL_FILES {
            if canonical == home_dir.join(name) {
                return true;
            }
        }
    }
    for prefix in SYSTEM_DENY_PREFIXES {
        if hermes_under(canonical, Path::new(prefix)) {
            return true;
        }
    }
    hermes_credential_name(canonical)
}
fn hermes_resolve_attachment(
    home: Option<&Path>,
    cwd: Option<&Path>,
    target: &str,
) -> AttachmentOutcome {
    if target.is_empty() {
        return AttachmentOutcome::Problem("unsupported");
    }
    if hermes_nt_namespace(target) {
        return AttachmentOutcome::Problem("credential");
    }
    let expanded = hermes_expand_target(target);
    let candidate = if Path::new(&expanded).is_absolute() {
        PathBuf::from(&expanded)
    } else {
        match cwd {
            Some(cwd) => cwd.join(&expanded),
            // A relative reference can only resolve under an authorized project
            // root; without one it is outside every allowed root.
            None => return AttachmentOutcome::Problem("outside_roots"),
        }
    };
    let Ok(candidate) = hermes_lexical(&candidate) else {
        return AttachmentOutcome::Problem("outside_roots");
    };
    let Ok(meta) = fs::symlink_metadata(&candidate) else {
        return AttachmentOutcome::Problem("missing");
    };
    if meta.file_type().is_symlink() {
        return AttachmentOutcome::Problem("symlink");
    }
    if !meta.is_file() {
        return AttachmentOutcome::Problem("unsupported");
    }
    if hermes_credential_name(&candidate) {
        return AttachmentOutcome::Problem("credential");
    }
    let Ok(canonical) = fs::canonicalize(&candidate) else {
        return AttachmentOutcome::Problem("unreadable");
    };
    if let Some(home) = home {
        let home_dir = std::env::var_os("HOME").map(PathBuf::from);
        if hermes_read_denied(&canonical, home, home_dir.as_deref()) {
            return AttachmentOutcome::Problem("credential");
        }
    }
    // Only the Hermes runtime cache dirs and the session's own project working
    // directory are readable roots — never the entire home (which would expose
    // state.db, mcp-tokens, vault, browser-profile, oauth/credential stores) and
    // never an arbitrary absolute path.
    let in_home_cache = home.is_some_and(|home| {
        HERMES_CACHE_DIRS
            .iter()
            .any(|sub| hermes_under(&canonical, &home.join(sub)))
    });
    let in_project = cwd.is_some_and(|root| hermes_under(&canonical, root));
    if !in_home_cache && !in_project {
        return AttachmentOutcome::Problem("outside_roots");
    }
    AttachmentOutcome::Captured(candidate)
}
fn hermes_problem_code(reason: &str) -> &'static str {
    match reason {
        "missing" => "hermes_attachment_missing",
        "symlink" => "hermes_attachment_symlink",
        "credential" => "hermes_attachment_credential",
        "outside_roots" => "hermes_attachment_outside_roots",
        "oversized" => "hermes_attachment_oversized",
        "limit" => "hermes_attachment_limit",
        "unreadable" => "hermes_attachment_unreadable",
        _ => "hermes_attachment_unsupported",
    }
}
fn hermes_capture_attachments(
    home: &Path,
    image: &rusqlite::Connection,
    root_session: &str,
    _staging: &Path,
    authorized_roots: &[PathBuf],
) -> Result<AttachmentCapture> {
    let (graph, _) = hermes_graph_sessions(image, root_session)?;
    let home_real = fs::canonicalize(home).ok();
    let mut files = BTreeMap::new();
    let mut entries: Vec<AttachmentEntry> = Vec::new();
    let mut problems: Vec<AttachmentProblem> = Vec::new();
    let mut seen_keys: BTreeSet<String> = BTreeSet::new();
    let mut seen_refs: BTreeSet<(String, String)> = BTreeSet::new();
    let mut total = 0usize;
    for session in &graph {
        let cwd = hermes_session_cwd(image, session)?;
        // A recorded session cwd is an attachment read root ONLY when an
        // explicit, user-authorized project mapping covers it — exactly like
        // project working-tree capture. An arbitrary persisted cwd is never a
        // grant and is never read; references under it are reported as
        // `outside_roots`, never silently captured.
        let cwd_real = authorized_attachment_root(&cwd, authorized_roots);
        let mut stmt = image
            .prepare("SELECT content, api_content FROM messages WHERE session_id = ?1")
            .map_err(|_| "session_invalid")?;
        let rows = stmt
            .query_map([session], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                ))
            })
            .map_err(|_| "session_invalid")?;
        for row in rows {
            let (content, api) = row.map_err(|_| "session_invalid")?;
            for text in [content, api].into_iter().flatten() {
                for (reference, kind, target) in hermes_references(&text) {
                    // Only local file/folder/image references are capturable;
                    // `@url:` (and any other remote token) is not a file read.
                    if !matches!(kind.as_str(), "file" | "folder" | "image") {
                        continue;
                    }
                    if !seen_refs.insert((kind.clone(), reference.clone())) {
                        continue;
                    }
                    match hermes_resolve_attachment(
                        home_real.as_deref(),
                        cwd_real.as_deref(),
                        &target,
                    ) {
                        AttachmentOutcome::Problem(reason) => {
                            problems.push(AttachmentProblem {
                                reference,
                                kind,
                                reason: reason.into(),
                                blocking: reason == "oversized" || reason == "limit",
                            });
                        }
                        AttachmentOutcome::Captured(path) => {
                            let source_path = path.to_string_lossy().into_owned();
                            let bytes = match storage::read(&path, HERMES_ATTACHMENT_MAX) {
                                Ok(bytes) => bytes,
                                Err(code) if code == "bundle_limit" => {
                                    problems.push(AttachmentProblem {
                                        reference,
                                        kind,
                                        reason: "oversized".into(),
                                        blocking: true,
                                    });
                                    continue;
                                }
                                Err(code) if code == "unsafe_store" => {
                                    problems.push(AttachmentProblem {
                                        reference,
                                        kind,
                                        reason: "symlink".into(),
                                        blocking: false,
                                    });
                                    continue;
                                }
                                Err(_) => {
                                    problems.push(AttachmentProblem {
                                        reference,
                                        kind,
                                        reason: "unreadable".into(),
                                        blocking: false,
                                    });
                                    continue;
                                }
                            };
                            if total + bytes.len() > HERMES_ATTACHMENT_TOTAL
                                || entries.len() >= HERMES_ATTACHMENT_FILES
                            {
                                problems.push(AttachmentProblem {
                                    reference,
                                    kind,
                                    reason: "limit".into(),
                                    blocking: true,
                                });
                                continue;
                            }
                            let sha256 = bundle::hash(&bytes);
                            let key =
                                format!("attachments/{sha256}/{}", hermes_attachment_name(&path));
                            if !hermes_attachment_path(&key) {
                                problems.push(AttachmentProblem {
                                    reference,
                                    kind,
                                    reason: "unsupported".into(),
                                    blocking: false,
                                });
                                continue;
                            }
                            if seen_keys.insert(key.clone()) {
                                total += bytes.len();
                                files.insert(key.clone(), STANDARD.encode(&bytes));
                            }
                            entries.push(AttachmentEntry {
                                reference,
                                kind,
                                target,
                                source_path,
                                sha256,
                                key,
                                bytes: bytes.len() as u64,
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(AttachmentCapture {
        files,
        entries,
        problems,
    })
}
// ── Inert async-delegation sidecar ─────────────────────────────────────────
// `delegate_task(background=true)` records each async job in `async_delegations`
// (`tools/async_delegation.py`). A finished job's task/result is durable user
// knowledge; an *in-flight* job is a running process that must never be claimed
// "transferred". The canonical store rebuild already leaves the live table empty,
// so a received snapshot can never re-lease or replay a job (Hermes replays
// pending rows on restart). Finished jobs therefore travel as an inert,
// read-only sidecar materialized into the isolated home but never imported into
// any table; in-flight jobs are reported as an explicit blocker.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AsyncDelegationRecord {
    delegation_id: String,
    origin_session: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    origin_session_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    origin_ui_session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parent_session_id: Option<String>,
    state: String,
    delivery_state: String,
    dispatched_at: f64,
    updated_at: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    completed_at: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    delivered_at: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    task: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AsyncDelegationIndex {
    version: u32,
    agent: String,
    root: String,
    /// Always `true`: this sidecar is data only and is never imported into the
    /// live `async_delegations` table (no lease, no replay).
    inert: bool,
    records: Vec<AsyncDelegationRecord>,
    blockers: Vec<String>,
}

fn hermes_suspected_secret(text: &str) -> bool {
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

/// A bounded, secret-free JSON field. An oversize or secret-bearing value is
/// dropped rather than shipped.
fn hermes_async_json(text: Option<String>) -> Option<serde_json::Value> {
    let text = text?;
    if text.is_empty() || text.len() > HERMES_ASYNC_FIELD_MAX || hermes_suspected_secret(&text) {
        return None;
    }
    serde_json::from_str(&text).ok()
}

fn hermes_table_exists(conn: &rusqlite::Connection, table: &str) -> Result<bool> {
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name = ?1",
            [table],
            |r| r.get(0),
        )
        .map_err(|_| "session_invalid")?;
    Ok(count == 1)
}

/// The inert async-delegation sidecar for an exported graph, plus the blocker
/// codes it contributes. Absent when the store has no related delegation rows.
fn hermes_capture_async_delegations(
    image: &rusqlite::Connection,
    root_session: &str,
) -> Result<(BTreeMap<String, String>, Vec<String>)> {
    let empty = BTreeMap::new();
    if !hermes_table_exists(image, "async_delegations")? {
        return Ok((empty, Vec::new()));
    }
    let (graph, _) = hermes_graph_sessions(image, root_session)?;
    if graph.is_empty() {
        return Ok((empty, Vec::new()));
    }
    let ids = graph
        .iter()
        .map(|id| rusqlite::types::Value::Text(id.clone()))
        .collect::<Vec<_>>();
    let placeholders = sql_placeholders(graph.len());
    let mut stmt = image
        .prepare(&format!(
            "SELECT delegation_id, origin_session, origin_session_id, origin_ui_session_id, \
                    parent_session_id, state, delivery_state, dispatched_at, updated_at, \
                    completed_at, delivered_at, task_json, result_json \
             FROM async_delegations \
             WHERE origin_session IN ({placeholders}) \
                OR origin_session_id IN ({placeholders}) \
                OR parent_session_id IN ({placeholders}) \
             ORDER BY delegation_id"
        ))
        .map_err(|_| "session_invalid")?;
    // The numbered placeholders `?1..?n` are reused across the three IN clauses,
    // so each id is bound exactly once.
    let rows = stmt
        .query_map(rusqlite::params_from_iter(ids.iter()), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, f64>(7)?,
                r.get::<_, f64>(8)?,
                r.get::<_, Option<f64>>(9)?,
                r.get::<_, Option<f64>>(10)?,
                r.get::<_, Option<String>>(11)?,
                r.get::<_, Option<String>>(12)?,
            ))
        })
        .map_err(|_| "session_invalid")?;
    let mut records: Vec<AsyncDelegationRecord> = Vec::new();
    let mut blockers: Vec<String> = Vec::new();
    let mut incomplete = false;
    let mut limited = false;
    for row in rows {
        let (
            delegation_id,
            origin_session,
            origin_session_id,
            origin_ui_session_id,
            parent_session_id,
            state,
            delivery_state,
            dispatched_at,
            updated_at,
            completed_at,
            delivered_at,
            task_json,
            result_json,
        ) = row.map_err(|_| "session_invalid")?;
        // An in-flight job is a running process, never a transferred outcome.
        if matches!(state.as_str(), "running" | "finalizing") {
            incomplete = true;
            continue;
        }
        if records.len() >= HERMES_ASYNC_RECORDS {
            limited = true;
            break;
        }
        records.push(AsyncDelegationRecord {
            delegation_id,
            origin_session,
            origin_session_id,
            origin_ui_session_id,
            parent_session_id,
            state,
            delivery_state,
            dispatched_at,
            updated_at,
            completed_at,
            delivered_at,
            task: hermes_async_json(task_json),
            result: hermes_async_json(result_json),
        });
    }
    if incomplete {
        blockers.push("hermes_async_delegation_incomplete".into());
    }
    if limited {
        blockers.push("hermes_async_delegation_limit".into());
    }
    if records.is_empty() && blockers.is_empty() {
        return Ok((empty, Vec::new()));
    }
    blockers.sort();
    blockers.dedup();
    let index = AsyncDelegationIndex {
        version: 1,
        agent: "hermes".into(),
        root: root_session.into(),
        inert: true,
        records,
        blockers: blockers.clone(),
    };
    let json = serde_json::to_vec(&index).map_err(|_| "session_invalid")?;
    if json.len() > HERMES_ASYNC_MAX {
        return Err("session_limit".into());
    }
    let mut files = BTreeMap::new();
    files.insert(HERMES_ASYNC_INDEX.into(), STANDARD.encode(&json));
    hermes_validate_async_delegations(&files)?;
    Ok((files, blockers))
}

fn hermes_validate_async_delegations(files: &BTreeMap<String, String>) -> Result<()> {
    let Some(encoded) = files.get(HERMES_ASYNC_INDEX) else {
        return Ok(());
    };
    let bytes = STANDARD.decode(encoded).map_err(|_| "session_invalid")?;
    if bytes.len() > HERMES_ASYNC_MAX {
        return Err("session_limit".into());
    }
    let index: AsyncDelegationIndex =
        serde_json::from_slice(&bytes).map_err(|_| "session_invalid")?;
    if index.version != 1
        || index.agent != "hermes"
        || !index.inert
        || !bundle::token(&index.root)
        || index.records.len() > HERMES_ASYNC_RECORDS
        || index.blockers.len() > 64
    {
        return Err("session_invalid".into());
    }
    for record in &index.records {
        if !bundle::token(&record.delegation_id) || !bundle::token(&record.origin_session) {
            return Err("session_invalid".into());
        }
        // Only finished outcomes travel; a running job can never be claimed.
        if matches!(record.state.as_str(), "running" | "finalizing") {
            return Err("session_invalid".into());
        }
        for value in [&record.task, &record.result].into_iter().flatten() {
            if hermes_suspected_secret(&value.to_string()) {
                return Err("session_secret".into());
            }
        }
    }
    for blocker in &index.blockers {
        if blocker.is_empty() || blocker.len() > 64 {
            return Err("session_invalid".into());
        }
    }
    Ok(())
}

/// An already-restored inert sidecar living in the home (`bastet/…`), re-read so
/// a re-capture of a managed home preserves outcomes without reactivating any
/// runtime row. An absent, oversize or invalid file contributes nothing.
fn hermes_home_async(home: &Path) -> Result<BTreeMap<String, String>> {
    let path = home.join(HERMES_ASYNC_INDEX);
    let Ok(bytes) = storage::read(&path, HERMES_ASYNC_MAX as u64) else {
        return Ok(BTreeMap::new());
    };
    let mut files = BTreeMap::new();
    files.insert(HERMES_ASYNC_INDEX.to_string(), STANDARD.encode(&bytes));
    if hermes_validate_async_delegations(&files).is_err() {
        return Ok(BTreeMap::new());
    }
    Ok(files)
}

fn hermes_validate_attachments(files: &BTreeMap<String, String>) -> Result<()> {
    let mut total = 0usize;
    let mut blobs = 0usize;
    let mut listed: BTreeSet<String> = BTreeSet::new();
    for (path, encoded) in files {
        if !hermes_attachment_path(path) {
            continue;
        }
        let bytes = STANDARD.decode(encoded).map_err(|_| "session_invalid")?;
        if path == HERMES_ATTACHMENT_INDEX {
            let index: AttachmentIndex =
                serde_json::from_slice(&bytes).map_err(|_| "session_invalid")?;
            if index.version != 1
                || index.agent != "hermes"
                || !bundle::token(&index.root)
                || index.entries.len() > HERMES_ATTACHMENT_FILES
                || index.problems.len() > HERMES_ATTACHMENT_FILES
            {
                return Err("session_invalid".into());
            }
            for entry in &index.entries {
                if !bundle::is_hash(&entry.sha256) || !hermes_attachment_path(&entry.key) {
                    return Err("session_invalid".into());
                }
                listed.insert(entry.key.clone());
            }
            continue;
        }
        let sha = path.split('/').nth(1).ok_or("session_invalid")?;
        if bundle::hash(&bytes) != sha {
            return Err("session_invalid".into());
        }
        total += bytes.len();
        blobs += 1;
        if total > HERMES_ATTACHMENT_TOTAL || blobs > HERMES_ATTACHMENT_FILES {
            return Err("session_limit".into());
        }
    }
    // Every indexed attachment must ship its blob.
    for key in &listed {
        if !files.contains_key(key) {
            return Err("session_invalid".into());
        }
    }
    Ok(())
}
fn hermes_validate_workspace_files(files: &BTreeMap<String, String>) -> Result<()> {
    // Each companion namespace has its own validator; the workspace validator
    // must never judge project/memory entries (it would reject them). Validate
    // every namespace over exactly its own entries.
    let subset = files
        .iter()
        .filter(|(path, _)| {
            !hermes_attachment_path(path)
                && !crate::hermes_project::allowed(path)
                && !crate::hermes_memory::allowed(path)
                && path.as_str() != HERMES_ASYNC_INDEX
        })
        .map(|(path, encoded)| (path.clone(), encoded.clone()))
        .collect::<BTreeMap<_, _>>();
    crate::hermes_workspace::validate(&subset)?;
    let project = project_subset(files);
    if !project.is_empty() {
        crate::hermes_project::validate(&project)?;
    }
    let memory = memory_subset(files);
    if !memory.is_empty() {
        crate::hermes_memory::validate(&memory)?;
    }
    hermes_validate_async_delegations(files)?;
    Ok(())
}
// Once the restore destination is known, rewrite references that point at a
// captured attachment so they resolve inside the restored home's `attachments/`
// directory. Absolute references covered by a project mapping were already
// remapped by `transform_hermes_manifest`; relative references follow the mapped
// cwd. Matching is by the exact recorded target and by the resolved source path.
pub(crate) fn finalize_hermes_attachments(
    manifest: &Manifest,
    destination: &Path,
) -> Result<Manifest> {
    if manifest.agent != "hermes" {
        return Ok(manifest.clone());
    }
    let Some(index_b64) = manifest.files.get(HERMES_ATTACHMENT_INDEX) else {
        return Ok(manifest.clone());
    };
    let index: AttachmentIndex =
        serde_json::from_slice(&STANDARD.decode(index_b64).map_err(|_| "session_invalid")?)
            .map_err(|_| "session_invalid")?;
    let mut mapping: BTreeMap<String, String> = BTreeMap::new();
    for entry in &index.entries {
        let restored = destination.join(&entry.key).to_string_lossy().into_owned();
        mapping.insert(entry.target.clone(), restored.clone());
        mapping.insert(entry.source_path.clone(), restored);
    }
    if mapping.is_empty() {
        return Ok(manifest.clone());
    }
    let bytes = STANDARD
        .decode(manifest.files.get("state.db").ok_or("session_invalid")?)
        .map_err(|_| "session_invalid")?;
    let dir = tempfile::tempdir().map_err(|_| "store_unavailable")?;
    let rewritten = hermes_rewrite_references(&bytes, dir.path(), |_, target| {
        if let Some(restored) = mapping.get(target) {
            return Ok(Some(restored.clone()));
        }
        if hermes_is_absolute_target(target) {
            if let Ok(canonical) = fs::canonicalize(target) {
                if let Some(restored) = mapping.get(&canonical.to_string_lossy().into_owned()) {
                    return Ok(Some(restored.clone()));
                }
            }
        }
        Ok(None)
    })?;
    let mut files = manifest.files.clone();
    files.insert("state.db".into(), STANDARD.encode(rewritten));
    Ok(Manifest {
        files,
        ..manifest.clone()
    })
}
// Root session id → managed profile path, read-only, so the UI can resolve a
// restored conversation without mutating any store.
#[allow(dead_code)] // integration seam used by the UI/worker after this module lands
pub(crate) fn managed_profile_for_session(root: &Path, session: &str) -> Result<Option<String>> {
    if !bundle::token(session) {
        return Err("session_invalid".into());
    }
    let registry = load_handoffs(root)?;
    Ok(registry
        .entries
        .iter()
        .find(|h| h.agent == "hermes" && h.session == session)
        .map(|h| h.path.to_string_lossy().into_owned()))
}

// ── Project working-tree continuity ────────────────────────────────────────
// The native conversation snapshot also carries the *selected* project working
// tree as an inert base64 namespace (`bastet/project-*`, owned by
// `hermes_project`). It is captured only for a user-authorized project root —
// never by crawling an arbitrary persisted `cwd` — and restored into a NEW
// isolated directory, never into the receiver's mapped worktree. Readiness
// blockers recorded by `hermes_project` persist into source/received issues: a
// copied sidecar is never reported ready merely because its bytes arrived.

/// Machine-readable project-continuity readiness for a Hermes snapshot.
enum ProjectReadiness {
    /// The snapshot carries no project working tree at all.
    Absent,
    /// The project working tree fully reconstructs the captured work.
    Ready,
    /// Blocking codes that must persist into source/received issues.
    Blocked(Vec<String>),
}

/// The project namespace of a combined Hermes file map.
fn project_subset(files: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    files
        .iter()
        .filter(|(path, _)| crate::hermes_project::allowed(path))
        .map(|(path, encoded)| (path.clone(), encoded.clone()))
        .collect()
}

/// Blocker codes from a project manifest. A sidecar that arrived but is not
/// `ready` (missing files, a selected subset, a captured limit, an authorization
/// gate) must be reported, never accepted because it was copied.
fn project_blockers(manifest: &crate::hermes_project::ProjectManifest) -> Vec<String> {
    let mut blockers = manifest.blockers.clone();
    if manifest.baseline_required && blockers.is_empty() {
        blockers.push("project_baseline_required".into());
    }
    blockers.sort();
    blockers.dedup();
    blockers
}

/// Readiness of the project namespace in a combined Hermes file map.
fn project_readiness(files: &BTreeMap<String, String>) -> Result<ProjectReadiness> {
    let subset = project_subset(files);
    if subset.is_empty() {
        return Ok(ProjectReadiness::Absent);
    }
    let manifest = crate::hermes_project::read_manifest(&subset)?;
    if crate::hermes_project::ready(&manifest) {
        Ok(ProjectReadiness::Ready)
    } else {
        Ok(ProjectReadiness::Blocked(project_blockers(&manifest)))
    }
}

/// The blocker codes the project namespace contributes to an issue report.
fn project_issue_codes(project: &BTreeMap<String, String>) -> Result<Vec<String>> {
    match project_readiness(project)? {
        ProjectReadiness::Blocked(codes) => Ok(codes),
        _ => Ok(Vec::new()),
    }
}

/// A project namespace carrying only a blocker and no captured files. Used for a
/// session that references a working directory the user has not authorized, so
/// the snapshot's readiness blocks and asks for a mapping instead of silently
/// exporting — or omitting — a project tree.
fn blocked_project_sidecar(cwd: &str, code: &str) -> Result<BTreeMap<String, String>> {
    let project: String = cwd
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("project")
        .chars()
        .filter(|c| !c.is_control())
        .take(128)
        .collect();
    let manifest = crate::hermes_project::ProjectManifest {
        version: 1,
        kind: "files".into(),
        project,
        repo: None,
        baseline_required: true,
        files: Vec::new(),
        excluded: Vec::new(),
        blockers: vec![code.to_string()],
    };
    let json = serde_json::to_vec(&manifest).map_err(|_| "project_invalid")?;
    Ok(BTreeMap::from([(
        crate::hermes_project::MANIFEST_PATH.to_string(),
        STANDARD.encode(json),
    )]))
}

/// The user-authorized project root for a session cwd, if any. Authorization is
/// explicit: the cwd must be an existing plain directory covered by one of the
/// user's project mappings (a whole-component ancestor). An arbitrary persisted
/// cwd is never a grant and is never read.
fn authorized_project_root(
    cwd: &str,
    mappings: &[crate::project_mapping::Mapping],
) -> Option<PathBuf> {
    if cwd.is_empty() || mappings.is_empty() {
        return None;
    }
    crate::project_mapping::remap_path(cwd, mappings)
        .ok()
        .flatten()?;
    let meta = fs::symlink_metadata(cwd).ok()?;
    (meta.is_dir() && !meta.file_type().is_symlink()).then(|| PathBuf::from(cwd))
}

/// A recorded session cwd is an attachment read root only when it is covered by
/// an explicit, user-authorized project root (a mapping target). An arbitrary
/// persisted cwd is never a grant: relative references cannot resolve and every
/// reference is reported `outside_roots`, exactly like project capture.
fn authorized_attachment_root(cwd: &str, roots: &[PathBuf]) -> Option<PathBuf> {
    if cwd.is_empty() || roots.is_empty() {
        return None;
    }
    let canonical = fs::canonicalize(cwd).ok()?;
    if !canonical.is_dir() {
        return None;
    }
    roots
        .iter()
        .any(|root| hermes_under(&canonical, root))
        .then_some(canonical)
}

/// Capture one project working tree (or its blocker sidecar), converting a
/// bounded `hermes_project` refusal into an explicit blocker rather than
/// aborting the whole conversation snapshot.
fn capture_authorized_project(root: &Path, cwd: &str) -> BTreeMap<String, String> {
    match crate::hermes_project::capture(root) {
        Ok(files) => files,
        Err(code) => blocked_project_sidecar(cwd, &code).unwrap_or_default(),
    }
}

/// Capture the project namespace for a session cwd. An unauthorized (or empty)
/// cwd is never read: it yields a blocker sidecar or nothing.
fn capture_project(
    cwd: &str,
    mappings: &[crate::project_mapping::Mapping],
) -> Result<BTreeMap<String, String>> {
    if cwd.is_empty() {
        return Ok(BTreeMap::new());
    }
    match authorized_project_root(cwd, mappings) {
        Some(root) => Ok(capture_authorized_project(&root, cwd)),
        None => blocked_project_sidecar(cwd, "project_mapping_required"),
    }
}

/// Outcome of materializing a captured project working tree on receive.
struct ProjectRestore {
    /// The NEW isolated working directory, when the project had files to place.
    workdir: Option<PathBuf>,
    /// Readiness blockers to persist into received issues.
    blockers: Vec<String>,
}

/// Restore a received Hermes snapshot's captured project working tree into a NEW
/// isolated directory (`project_destination`) and re-root the session cwd and
/// every project-rooted reference to that directory. The project entries are
/// removed from the managed HOME manifest, so the receiver's mapped worktree is
/// never written. A blocking sidecar (no files) is recorded, not materialized.
fn restore_hermes_project(
    manifest: &Manifest,
    project_destination: &Path,
) -> Result<(Manifest, Option<ProjectRestore>)> {
    if manifest.agent != "hermes" {
        return Ok((manifest.clone(), None));
    }
    let subset = project_subset(&manifest.files);
    if subset.is_empty() {
        return Ok((manifest.clone(), None));
    }
    let project = crate::hermes_project::read_manifest(&subset)?;
    let blockers = project_blockers(&project);
    let mut files = manifest.files.clone();
    for path in subset.keys() {
        files.remove(path);
    }
    let adjusted = Manifest {
        files,
        ..manifest.clone()
    };
    // An authorization gate, an explicit empty selection, or a session with no
    // usable cwd carries no placeable tree: record readiness, materialize none.
    if project.files.is_empty() || adjusted.cwd.is_empty() {
        return Ok((
            adjusted,
            Some(ProjectRestore {
                workdir: None,
                blockers,
            }),
        ));
    }
    if !project_destination.exists() {
        if let Some(parent) = project_destination.parent() {
            storage::directory(parent)?;
        }
        crate::hermes_project::restore_to_new_directory(&subset, project_destination)?;
    }
    // Re-root to the canonical path so the recorded cwd, the DB value and the
    // reported workdir agree on every platform (macOS `/var` is a symlink).
    let workdir = fs::canonicalize(project_destination).map_err(|_| "store_unavailable")?;
    let target = workdir.to_string_lossy().into_owned();
    let rerooted = crate::project_mapping::transform_manifest(
        &adjusted,
        &[crate::project_mapping::Mapping {
            source: adjusted.cwd.clone(),
            target,
        }],
    )?;
    Ok((
        rerooted,
        Some(ProjectRestore {
            workdir: Some(workdir),
            blockers,
        }),
    ))
}

/// The memory namespace of a combined Hermes file map.
fn memory_subset(files: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    files
        .iter()
        .filter(|(path, _)| crate::hermes_memory::allowed(path))
        .map(|(path, encoded)| (path.clone(), encoded.clone()))
        .collect()
}

/// Memory-continuation readiness from a Hermes manifest's file map.
enum MemoryStatus {
    Ready,
    Blocked(Vec<String>),
    Absent,
}
fn memory_readiness(files: &BTreeMap<String, String>) -> MemoryStatus {
    let subset = memory_subset(files);
    if subset.is_empty() {
        return MemoryStatus::Absent;
    }
    match crate::hermes_memory::read_manifest(&subset) {
        Ok(manifest) => {
            if !manifest.blockers.is_empty() {
                MemoryStatus::Blocked(manifest.blockers)
            } else if manifest.provider == "builtin" {
                // Built-in file memory travels with the workspace payload;
                // there is nothing external to import.
                MemoryStatus::Ready
            } else if manifest.provider == "agent-memory-os" {
                if manifest.exported {
                    MemoryStatus::Ready
                } else {
                    MemoryStatus::Blocked(vec!["memory_export_missing".into()])
                }
            } else if manifest.provider.is_empty() {
                MemoryStatus::Blocked(vec!["memory_provider_unresolved".into()])
            } else {
                MemoryStatus::Blocked(vec!["memory_provider_unknown".into()])
            }
        }
        Err(_) => MemoryStatus::Blocked(vec!["memory_invalid".into()]),
    }
}

/// Aggregated continuation readiness the UI gates copy/resume on. A copied
/// packet is never `ready`: workspace blocking deps/issues, attachment problems,
/// project readiness and the memory binding each contribute blockers/warnings,
/// and a missing recipient binding keeps the state `blocked`/`unverified`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Continuation {
    /// `"blocked"`, `"unverified"` or `"ready"`.
    state: String,
    blockers: Vec<String>,
    warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    project_path: Option<String>,
    verified: Vec<String>,
}

fn continuation_for(manifest: &Manifest, project_path: Option<String>) -> Continuation {
    let mut blockers: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut verified: Vec<String> = Vec::new();

    // External-memory readiness is computed first so a verified binding can
    // resolve the workspace's own memory dependency instead of blocking forever.
    let memory = memory_readiness(&manifest.files);

    // Workspace: blocking dependencies/issues block; the rest warn.
    match manifest.files.get(crate::hermes_workspace::MANIFEST) {
        Some(encoded) => match STANDARD
            .decode(encoded)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
        {
            Some(text) => {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                    for issue in value["issues"].as_array().into_iter().flatten() {
                        let code =
                            format!("workspace_{}", issue["reason"].as_str().unwrap_or("issue"));
                        if issue["blocking"].as_bool().unwrap_or(false) {
                            blockers.push(code);
                        } else {
                            warnings.push(code);
                        }
                    }
                }
                // Dependencies are re-scored for the RECEIVING host: an `os`
                // requirement is judged where the restored home will actually run,
                // never from the sender's serialized verdict. New kinds
                // (`credential`, `app`, ...) yield `workspace_dependency_<kind>`.
                match crate::hermes_workspace::receive_dependencies_here(&text) {
                    Ok(deps) => {
                        for dep in deps {
                            // A verified external-memory binding resolves the
                            // workspace memory dependency; it must not stay blocked
                            // after local setup.
                            if dep.kind == "memory" && matches!(memory, MemoryStatus::Ready) {
                                verified.push("external_memory_bound".into());
                                continue;
                            }
                            let code = format!("workspace_dependency_{}", dep.kind);
                            if dep.blocking {
                                blockers.push(code);
                            } else {
                                warnings.push(code);
                            }
                        }
                        verified.push("workspace".into());
                    }
                    Err(_) => blockers.push("workspace_invalid".into()),
                }
            }
            None => blockers.push("workspace_invalid".into()),
        },
        None => blockers.push("workspace_missing".into()),
    }

    // Attachments: blocking problems block; the rest warn.
    if let Some(encoded) = manifest.files.get("attachments/index.json") {
        if let Some(value) = STANDARD
            .decode(encoded)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        {
            for problem in value["problems"].as_array().into_iter().flatten() {
                let code = format!(
                    "attachment_{}",
                    problem["reason"].as_str().unwrap_or("problem")
                );
                if problem["blocking"].as_bool().unwrap_or(false) {
                    blockers.push(code);
                } else {
                    warnings.push(code);
                }
            }
            verified.push("attachments".into());
        }
    }

    // Project continuity + the restored (new, isolated) working directory.
    match project_readiness(&manifest.files) {
        Ok(ProjectReadiness::Ready) => verified.push("project".into()),
        Ok(ProjectReadiness::Absent) => warnings.push("project_absent".into()),
        Ok(ProjectReadiness::Blocked(codes)) => blockers.extend(codes),
        Err(_) => blockers.push("project_invalid".into()),
    }

    // External memory: a missing or unresolved binding is never ready.
    match memory {
        MemoryStatus::Ready => verified.push("memory".into()),
        MemoryStatus::Blocked(codes) => blockers.extend(codes),
        MemoryStatus::Absent => blockers.push("memory_unverified".into()),
    }

    blockers.sort();
    blockers.dedup();
    warnings.sort();
    warnings.dedup();
    verified.sort();
    verified.dedup();
    let state = if !blockers.is_empty() {
        "blocked"
    } else if !warnings.is_empty() {
        "unverified"
    } else {
        "ready"
    };
    Continuation {
        state: state.into(),
        blockers,
        warnings,
        project_path,
        verified,
    }
}

/// Overwrite a continuation's memory verdict with a restored managed home's real
/// readiness: a verified binding clears the capture-side memory blockers and adds
/// the verified facts; otherwise the readiness blockers are surfaced verbatim.
fn apply_memory_readiness(
    continuation: &mut Continuation,
    readiness: &crate::hermes_memory::MemoryReadiness,
) {
    continuation
        .blockers
        .retain(|code| !code.starts_with("memory_"));
    continuation
        .verified
        .retain(|fact| fact != "memory" && fact != "external_memory_bound");
    if readiness.ready {
        continuation.verified.push("memory".into());
        continuation.verified.push("external_memory_bound".into());
    } else if readiness.blockers.is_empty() {
        continuation.blockers.push("memory_unverified".into());
    } else {
        continuation
            .blockers
            .extend(readiness.blockers.iter().cloned());
    }
    continuation.blockers.sort();
    continuation.blockers.dedup();
    continuation.verified.sort();
    continuation.verified.dedup();
    continuation.state = if !continuation.blockers.is_empty() {
        "blocked"
    } else if !continuation.warnings.is_empty() {
        "unverified"
    } else {
        "ready"
    }
    .into();
}

/// App-config root plus settings, threaded so a cycle can capture — and a
/// receive can restore — the external-memory continuation with the user's
/// configured AgentMemoryOS CLI and explicit authorization.
#[derive(Clone, Copy)]
pub struct MemoryContext<'a> {
    pub app_root: &'a Path,
    pub settings: &'a crate::model::Settings,
}

/// Capture the external-memory continuation for one authorized selected home,
/// exactly once per cycle. [`crate::hermes_memory::capture_with_context`] fails
/// closed: an unresolved or unauthorized provider yields a manifest carrying a
/// blocker and no export — never a silent private dump.
fn capture_memory_for_cycle(
    home: &Path,
    memory: Option<MemoryContext>,
) -> Result<(BTreeMap<String, String>, Vec<String>)> {
    let Some(ctx) = memory else {
        return Ok((BTreeMap::new(), Vec::new()));
    };
    match crate::hermes_memory::capture_with_context(ctx.app_root, home, ctx.settings) {
        Ok(files) => {
            let manifest = crate::hermes_memory::read_manifest(&files)?;
            Ok((files, manifest.blockers))
        }
        Err(code) => Ok((BTreeMap::new(), vec![code])),
    }
}

/// Restore the received external-memory continuation into the isolated managed
/// home and report the *real* readiness (verified by re-export, never a
/// synthetic ready). Absent memory, or a home with no provider binding, yields
/// `None`.
fn hermes_restore_memory(
    memory: Option<MemoryContext>,
    files: &BTreeMap<String, String>,
    managed_home: &Path,
) -> Result<Option<crate::hermes_memory::MemoryReadiness>> {
    let Some(ctx) = memory else {
        return Ok(None);
    };
    let subset = memory_subset(files);
    if subset.is_empty() {
        return Ok(None);
    }
    let provider = crate::hermes_memory::read_manifest(&subset)
        .map(|manifest| manifest.provider)
        .unwrap_or_default();
    match crate::hermes_memory::restore_with_context(
        ctx.app_root,
        &subset,
        managed_home,
        ctx.settings,
    ) {
        Ok(readiness) => Ok(Some(readiness)),
        // A restore that cannot run (store unavailable, invalid payload) is a
        // real, reported blocker — never a fabricated ready.
        Err(code) => Ok(Some(crate::hermes_memory::MemoryReadiness {
            provider,
            ready: false,
            exported: true,
            imported: false,
            records: 0,
            agent_id: None,
            store: None,
            contains_authority_changes: false,
            blockers: vec![code],
        })),
    }
}

/// The JSON text of the received workspace manifest, if the snapshot carries one.
fn workspace_manifest_text(files: &BTreeMap<String, String>) -> Option<String> {
    files
        .get(crate::hermes_workspace::MANIFEST)
        .and_then(|encoded| STANDARD.decode(encoded).ok())
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

/// Resolve the received workspace requirements against the *receiving* host via
/// the shared receiver-side resolver, and replace the coarse dependency codes in
/// a continuation report with its verdict: `env`/`runtime`/`os` are probed
/// locally (never executed), while opaque requirements (MCP/model/app/
/// credential) stay blocking until the recipient explicitly acknowledges them
/// for this exact manifest — and even then they are warnings, never verified.
/// The external-memory dependency stays governed by the memory binding, so its
/// codes are left to `continuation_for`/`apply_memory_readiness`.
fn apply_requirements_evidence(
    continuation: &mut Continuation,
    files: &BTreeMap<String, String>,
    home: &Path,
) {
    let Some(text) = workspace_manifest_text(files) else {
        return;
    };
    let Ok(evidence) = crate::hermes_workspace::hermes_requirements::receive_for_home(&text, home)
    else {
        return;
    };
    let memory_code = |code: &str| code.starts_with("workspace_dependency_memory");
    let dependency = |code: &String| code.starts_with("workspace_dependency_");
    continuation
        .blockers
        .retain(|code| !(dependency(code) && !memory_code(code)));
    continuation
        .warnings
        .retain(|code| !(dependency(code) && !memory_code(code)));
    continuation
        .verified
        .retain(|code| !(dependency(code) && !memory_code(code)));
    for code in evidence.blockers {
        if !memory_code(&code) {
            continuation.blockers.push(code);
        }
    }
    for code in evidence.warnings {
        if !memory_code(&code) {
            continuation.warnings.push(code);
        }
    }
    for code in evidence.verified {
        if !memory_code(&code) {
            continuation.verified.push(code);
        }
    }
    continuation.blockers.sort();
    continuation.blockers.dedup();
    continuation.warnings.sort();
    continuation.warnings.dedup();
    continuation.verified.sort();
    continuation.verified.dedup();
    continuation.state = if !continuation.blockers.is_empty() {
        "blocked"
    } else if !continuation.warnings.is_empty() {
        "unverified"
    } else {
        "ready"
    }
    .into();
}

/// Resolve a registered managed home from the app's own registry by the
/// received version id (`base`) and agent. The caller can never supply a path:
/// an unregistered id is refused, and a registered path that is a symlink or no
/// longer a directory is refused rather than followed.
fn resolve_registered_home(registry: &Handoffs, agent: &str, id: &str) -> Result<(PathBuf, usize)> {
    if !bundle::is_hash(id) {
        return Err("session_invalid".into());
    }
    let index = registry
        .entries
        .iter()
        .position(|h| h.agent == agent && h.base == id)
        .ok_or("session_invalid")?;
    let managed = registry.entries[index].path.clone();
    let metadata = fs::symlink_metadata(&managed).map_err(|_| "session_invalid")?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("unsafe_store".into());
    }
    Ok((managed, index))
}

/// Outcome of an explicit continuation recheck (and optional provider setup or
/// requirement acknowledgment), returned to the UI so it can refresh the report.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecheckResult {
    /// The authoritative, freshly recomputed continuation report.
    continuation: Continuation,
    /// The real external-memory readiness after the recheck, when the snapshot
    /// carries external memory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    memory: Option<crate::hermes_memory::MemoryReadiness>,
    /// Opaque requirement codes acknowledged by this call (empty otherwise).
    acknowledged: Vec<String>,
    /// Whether the isolated managed home now carries a discoverable provider
    /// plugin (read back from disk, never inferred from a CLI exit status).
    provider_installed: bool,
}

/// Re-run a received Hermes version's continuation readiness against its
/// *already registered* isolated managed home. The target is resolved from the
/// registry by version id — never a caller-supplied path — and only that home is
/// touched; no new managed profile is ever created. The explicit user actions
/// this backs are: install the trusted local provider (`agent-memory hermes
/// install --hermes-home <EXACT HOME>`, nothing else) into that home, re-run the
/// official memory import/verify, and/or record a hash-bound acknowledgment of
/// the opaque workspace requirements. Every result is persisted on the registry
/// entry and re-derived from the received snapshot, so the backend report stays
/// authoritative and a version is never marked ready while a binding is
/// unverified.
#[allow(clippy::too_many_arguments)]
fn recheck_continuation_with(
    sessions_root: &Path,
    app_root: &Path,
    space: &str,
    settings: &crate::model::Settings,
    agent: &str,
    id: &str,
    acknowledge: bool,
    setup_provider: bool,
    staging: &Path,
) -> Result<RecheckResult> {
    if agent != "hermes" {
        return Err("session_invalid".into());
    }
    let mut registry = load_handoffs(sessions_root)?;
    let (managed, index) = resolve_registered_home(&registry, agent, id)?;
    // The registered home must be the isolated profile this app created: an
    // app-private managed profile, or an explicitly re-granted external folder.
    handoff_parent_scope(
        sessions_root.parent().ok_or("store_unavailable")?,
        managed.parent().ok_or("sandbox_reauthorize")?,
    )?;
    // The received snapshot stays the authoritative source of the requirements.
    let replica = Replica::open(&sessions_root.join("replica"), space)?;
    let all = replica.transport_bundles()?;
    let b = all.get(id).ok_or("session_invalid")?;
    b.validate()?;
    let incoming = unpack(b, &all)?;
    if incoming.agent != "hermes"
        || bundle::hash(incoming.session.as_bytes()) != b.snapshot.stream.conversation
    {
        return Err("session_invalid".into());
    }
    let project_path = registry.entries[index]
        .project
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned());

    // The provider plugin is proven by reading the shim back from this exact
    // home; a build without the external channel never runs a CLI.
    if setup_provider {
        #[cfg(feature = "mac-app-store")]
        {
            let _ = staging;
            return Err("memory_unavailable_store".into());
        }
        #[cfg(not(feature = "mac-app-store"))]
        {
            // Only the known external provider may be set up; anything else is
            // refused rather than handed an arbitrary CLI invocation.
            let provider = crate::hermes_memory::read_manifest(&memory_subset(&incoming.files))
                .map(|manifest| manifest.provider)
                .unwrap_or_default();
            if provider != "agent-memory-os" {
                return Err("memory_unavailable_store".into());
            }
            let cli = crate::amos_runtime::configured(app_root)?;
            crate::hermes_memory::install_provider(&cli, &managed, staging)?;
        }
    }
    let provider_installed = crate::hermes_memory::provider_plugin_present(&managed);

    // Re-run the official external-memory import/verify against the SAME home.
    // An already-changed store is preserved (never overwritten) and reported.
    let readiness = hermes_restore_memory(
        Some(MemoryContext { app_root, settings }),
        &incoming.files,
        &managed,
    )?;

    let mut continuation = continuation_for(&incoming, project_path);
    if let Some(readiness) = readiness.as_ref() {
        apply_memory_readiness(&mut continuation, readiness);
    }

    // The recipient may explicitly acknowledge the opaque requirements for this
    // exact manifest; the shared resolver stores that acknowledgment in the
    // managed home and is re-run so acknowledged items show as honest warnings
    // while verifiable misses stay blocking. Nothing here is ever marked
    // verified, and a changed manifest invalidates the acknowledgment.
    let mut acknowledged: Vec<String> = Vec::new();
    if acknowledge {
        if let Some(text) = workspace_manifest_text(&incoming.files) {
            let requirements =
                crate::hermes_workspace::hermes_requirements::receive_for_home(&text, &managed)
                    .map(|evidence| evidence.requirements)
                    .unwrap_or_default();
            for requirement in requirements {
                if requirement.status == "unverified" {
                    // A verifiable requirement is refused by the store itself.
                    let _ = crate::hermes_workspace::hermes_requirements::acknowledge(
                        &managed,
                        &text,
                        &requirement.kind,
                        &requirement.name,
                    );
                }
            }
            acknowledged =
                crate::hermes_workspace::hermes_requirements::receive_for_home(&text, &managed)
                    .map(|evidence| evidence.warnings)
                    .unwrap_or_default();
        }
    }
    apply_requirements_evidence(&mut continuation, &incoming.files, &managed);

    registry.entries[index].memory = readiness.clone();
    save_handoffs(sessions_root, &registry)?;
    Ok(RecheckResult {
        continuation,
        memory: readiness,
        acknowledged,
        provider_installed,
    })
}

// Assemble the full native manifest for `root_session`: the rebuilt lineage
// graph as `state.db`, the merged portable workspace and the content-hashed
// referenced attachments (plus their index and any problems).
#[allow(clippy::too_many_arguments)]
fn capture_hermes_manifest(
    home: &Path,
    image: &rusqlite::Connection,
    root_session: &str,
    cwd: &str,
    staging: &Path,
    workspace: &BTreeMap<String, String>,
    project: Option<&BTreeMap<String, String>>,
    attachment_roots: &[PathBuf],
    memory: &BTreeMap<String, String>,
) -> Result<(Manifest, Vec<String>)> {
    let (bytes, mut issues) = hermes_canonicalize_with_issues(image, root_session, cwd, staging)?;
    let mut files = BTreeMap::new();
    files.insert("state.db".into(), STANDARD.encode(bytes));
    for (path, encoded) in workspace {
        if !crate::hermes_workspace::allowed(path) {
            return Err("workspace_unsafe".into());
        }
        files.insert(path.clone(), encoded.clone());
    }
    // The project working tree (or its authorization/readiness blocker). Its
    // blockers join the conversation's issues so a blocked snapshot is never
    // reported ready merely because the sidecar was captured.
    if let Some(project) = project {
        for (path, encoded) in project {
            if !crate::hermes_project::allowed(path) {
                return Err("project_invalid".into());
            }
            files.insert(path.clone(), encoded.clone());
        }
        for code in project_issue_codes(project)? {
            issues.push(code);
        }
    }
    let attachments =
        hermes_capture_attachments(home, image, root_session, staging, attachment_roots)?;
    for problem in &attachments.problems {
        issues.push(hermes_problem_code(&problem.reason).into());
    }
    for (path, encoded) in attachments.files {
        files.insert(path, encoded);
    }
    if !attachments.entries.is_empty() {
        let index = AttachmentIndex {
            version: 1,
            agent: "hermes".into(),
            root: root_session.into(),
            entries: attachments.entries,
            problems: attachments.problems,
        };
        let json = serde_json::to_vec(&index).map_err(|_| "session_invalid")?;
        files.insert(HERMES_ATTACHMENT_INDEX.into(), STANDARD.encode(json));
    }
    // Finished async-delegation outcomes travel as an inert sidecar (never into
    // the live table). A restored managed home already carries that sidecar on
    // disk; read it back so a re-capture preserves the outcomes even though the
    // managed store's own `async_delegations` table is (correctly) empty. A
    // table-derived record supersedes a stale on-disk one.
    let mut async_files = hermes_home_async(home)?;
    let (derived, async_blockers) = hermes_capture_async_delegations(image, root_session)?;
    for code in async_blockers {
        issues.push(code);
    }
    for (path, encoded) in derived {
        async_files.insert(path, encoded);
    }
    for (path, encoded) in async_files {
        files.insert(path, encoded);
    }
    // External-memory continuation (bound provider + official export, or an
    // explicit blocker). Captured once per cycle from the selected home and
    // merged into every session snapshot; blockers join the session's issues.
    if !memory.is_empty() {
        for (path, encoded) in memory {
            if !crate::hermes_memory::allowed(path) {
                return Err("memory_unsafe".into());
            }
            files.insert(path.clone(), encoded.clone());
        }
        let manifest = crate::hermes_memory::read_manifest(memory)?;
        for code in manifest.blockers {
            issues.push(code);
        }
    }
    hermes_validate_workspace_files(&files)?;
    Ok((
        Manifest {
            version: 1,
            agent: "hermes".into(),
            session: root_session.into(),
            cwd: cwd.into(),
            files,
        },
        issues,
    ))
}
#[allow(clippy::too_many_arguments)]
fn capture_hermes_session(
    home: &Path,
    image: &rusqlite::Connection,
    session: &str,
    cwd: &str,
    staging: &Path,
    workspace: &BTreeMap<String, String>,
    project: Option<&BTreeMap<String, String>>,
    attachment_roots: &[PathBuf],
    memory: &BTreeMap<String, String>,
) -> Result<(Manifest, Vec<String>)> {
    capture_hermes_manifest(
        home,
        image,
        session,
        cwd,
        staging,
        workspace,
        project,
        attachment_roots,
        memory,
    )
}
#[allow(clippy::too_many_arguments)]
fn capture_hermes_store(
    home: &Path,
    db: &Path,
    session: &str,
    staging: &Path,
    project: Option<&BTreeMap<String, String>>,
    attachment_roots: &[PathBuf],
    memory: &BTreeMap<String, String>,
) -> Result<Manifest> {
    let snapshot = hermes_backup_bytes(db, staging)?;
    let workspace = crate::hermes_workspace::capture(home)?;
    let (manifest, _issues) = hermes_with_image(&snapshot, staging, |conn| {
        let cwd = hermes_session_cwd(conn, session)?;
        capture_hermes_manifest(
            home,
            conn,
            session,
            &cwd,
            staging,
            &workspace,
            project,
            attachment_roots,
            memory,
        )
    })?;
    Ok(manifest)
}
fn encode(manifest: &Manifest) -> Result<String> {
    #[cfg(test)]
    ENCODE_CALLS.with(|calls| calls.set(calls.get() + 1));
    if !manifest_paths_safe(manifest) {
        return Err("session_invalid".into());
    }
    if manifest.files.values().map(|s| s.len() as u64).sum::<u64>() > MAX_RAW - 1024 * 1024 {
        return Err("session_limit".into());
    }
    let mut gzip = GzEncoder::new(Vec::new(), Compression::fast());
    serde_json::to_writer(&mut gzip, manifest).map_err(|_| "session_invalid")?;
    let bytes = gzip.finish().map_err(|_| "session_invalid")?;
    if bytes.len() as u64 > MAX_PACKED {
        return Err("session_limit".into());
    }
    Ok(STANDARD.encode(bytes))
}
fn decode(text: &str) -> Result<Manifest> {
    if text.len() as u64 > MAX_PACKED * 4 / 3 + 4 {
        return Err("session_limit".into());
    }
    let bytes = STANDARD.decode(text).map_err(|_| "session_invalid")?;
    let mut raw = Vec::new();
    GzDecoder::new(bytes.as_slice())
        .take(MAX_RAW + 1)
        .read_to_end(&mut raw)
        .map_err(|_| "session_invalid")?;
    if raw.len() as u64 > MAX_RAW {
        return Err("session_limit".into());
    }
    let m: Manifest = serde_json::from_slice(&raw).map_err(|_| "session_invalid")?;
    if m.version != 1
        || !crate::model::AGENTS.contains(&m.agent.as_str())
        || m.agent == "agent-memory-os"
        || !bundle::token(&m.session)
        || m.files.is_empty()
        || m.files.len() > 2048
        || !manifest_paths_safe(&m)
    {
        return Err("session_invalid".into());
    }
    Ok(m)
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Parts {
    sha256: String,
    ids: Vec<String>,
}
fn cached_native_baseline_complete(
    batch: &crate::sync::ExportBatch<'_>,
    stream: &Stream,
    id: &str,
) -> bool {
    let Some(root) = batch.baseline(stream, id) else {
        return false;
    };
    if !root.snapshot.files.contains_key("session.json") {
        return false;
    }
    if root.snapshot.files.contains_key("session.gz.b64") {
        return !root.snapshot.files.contains_key("session.parts.json");
    }
    let Some(parts) = root.snapshot.files.get("session.parts.json") else {
        return false;
    };
    let Ok(parts) = serde_json::from_str::<Parts>(&parts.content) else {
        return false;
    };
    if parts.ids.is_empty() || parts.ids.len() > 64 || !bundle::is_hash(&parts.sha256) {
        return false;
    }
    let mut hash = Sha256::new();
    let mut packed_len = 0u64;
    for (i, part_id) in parts.ids.iter().enumerate() {
        let part_stream = Stream {
            agent: stream.agent.clone(),
            profile: stream.profile.clone(),
            conversation: format!("{}-p{i}", stream.conversation),
        };
        let Some(part) = batch.baseline(&part_stream, part_id) else {
            return false;
        };
        let Some(content) = part.snapshot.files.get("session.part.b64") else {
            return false;
        };
        packed_len += content.content.len() as u64;
        if packed_len > MAX_PACKED * 4 / 3 + 4 {
            return false;
        }
        hash.update(content.content.as_bytes());
    }
    format!("{:x}", hash.finalize()) == parts.sha256
}
fn publish(
    m: &Manifest,
    batch: &mut crate::sync::ExportBatch<'_>,
    journal: &mut Journal,
    part_size: usize,
) -> Result<String> {
    let conversation = bundle::hash(m.session.as_bytes());
    let packed = encode(m)?;
    let mut files = BTreeMap::from([(
        "session.json".into(),
        json(&serde_json::json!({"session":m.session,"cwd":m.cwd,"agent":m.agent}))?,
    )]);
    if packed.len() <= part_size {
        files.insert("session.gz.b64".into(), packed);
    } else {
        let mut ids = vec![];
        for (i, chunk) in packed.as_bytes().chunks(part_size).enumerate() {
            let part_conversation = format!("{conversation}-p{i}");
            let id = batch.export_from(
                Stream {
                    agent: m.agent.clone(),
                    profile: journal.node.clone(),
                    conversation: part_conversation.clone(),
                },
                BTreeMap::from([(
                    "session.part.b64".into(),
                    String::from_utf8(chunk.to_vec()).map_err(|_| "session_invalid")?,
                )]),
                journal.bases.get(&part_conversation).map(|s| s.as_str()),
            )?;
            journal.bases.insert(part_conversation, id.clone());
            ids.push(id);
        }
        files.insert(
            "session.parts.json".into(),
            json(&Parts {
                sha256: bundle::hash(packed.as_bytes()),
                ids,
            })?,
        );
    }
    let id = batch.export_from(
        Stream {
            agent: m.agent.clone(),
            profile: journal.node.clone(),
            conversation: conversation.clone(),
        },
        files,
        journal.bases.get(&conversation).map(|s| s.as_str()),
    )?;
    journal.bases.insert(conversation, id.clone());
    Ok(id)
}
fn unpack(b: &bundle::Bundle, all: &BTreeMap<String, bundle::Bundle>) -> Result<Manifest> {
    if let Some(entry) = b.snapshot.files.get("session.gz.b64") {
        return decode(&entry.content);
    }
    let parts: Parts = serde_json::from_str(
        &b.snapshot
            .files
            .get("session.parts.json")
            .ok_or("session_invalid")?
            .content,
    )
    .map_err(|_| "session_invalid")?;
    if parts.ids.is_empty() || parts.ids.len() > 64 || !bundle::is_hash(&parts.sha256) {
        return Err("session_invalid".into());
    }
    let mut packed = String::new();
    for (i, id) in parts.ids.iter().enumerate() {
        let p = all.get(id).ok_or("session_parts_pending")?;
        if p.id != *id
            || p.snapshot.stream.agent != b.snapshot.stream.agent
            || p.snapshot.stream.profile != b.snapshot.stream.profile
            || p.snapshot.stream.conversation != format!("{}-p{i}", b.snapshot.stream.conversation)
        {
            return Err("session_invalid".into());
        }
        p.validate()?;
        let text = &p
            .snapshot
            .files
            .get("session.part.b64")
            .ok_or("session_invalid")?
            .content;
        if packed.len() as u64 + text.len() as u64 > MAX_PACKED * 4 / 3 + 4 {
            return Err("session_limit".into());
        }
        packed.push_str(text);
    }
    if bundle::hash(packed.as_bytes()) != parts.sha256 {
        return Err("session_invalid".into());
    }
    decode(&packed)
}
fn capture_file(agent: &str, root: &Path, path: &Path, staging: &Path) -> Result<Manifest> {
    // Hermes never reaches here: its store needs a known root session id, so
    // capture_handoff routes it through `capture_hermes_store`.
    let mut files = BTreeMap::new();
    let bytes = if agent == "agy" {
        sqlite_snapshot(path, staging)?
    } else {
        stable(path)?
    };
    let mut cwd = String::new();
    let session = if agent == "agy" {
        path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or("session_invalid")?
            .to_string()
    } else {
        lines(&bytes)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| "session_invalid")?;
        let first: serde_json::Value =
            serde_json::from_str(text.lines().next().ok_or("session_invalid")?)
                .map_err(|_| "session_invalid")?;
        let meta = if matches!(agent, "codex" | "chatgpt-work") {
            if first["type"] != "session_meta" {
                return Err("session_format_unsupported".into());
            }
            first["payload"].clone()
        } else if agent == "pi" {
            if first["type"] != "session" || !matches!(first["version"].as_u64(), Some(1..=3)) {
                return Err("session_format_unsupported".into());
            }
            first
        } else {
            let mut found = None;
            for line in text.lines().filter(|s| !s.trim().is_empty()) {
                let v: serde_json::Value =
                    serde_json::from_str(line).map_err(|_| "session_invalid")?;
                if v["sessionId"].is_string() {
                    if found.is_none() {
                        found = Some(v.clone());
                    }
                    if v["cwd"].is_string() {
                        found = Some(v);
                        break;
                    }
                }
            }
            found.ok_or("session_invalid")?
        };
        cwd = meta["cwd"].as_str().unwrap_or("").into();
        meta[if matches!(agent, "claude" | "claude-code") {
            "sessionId"
        } else {
            "id"
        }]
        .as_str()
        .ok_or("session_invalid")?
        .to_string()
    };
    if !bundle::token(&session) {
        return Err("session_invalid".into());
    }
    let relative = path
        .strip_prefix(root)
        .map_err(|_| "session_invalid")?
        .to_str()
        .ok_or("session_invalid")?
        .replace('\\', "/");
    if !safe_relative(&relative) {
        return Err("session_invalid".into());
    }
    files.insert(relative, STANDARD.encode(bytes));
    if matches!(agent, "claude" | "claude-code") {
        let subagents = path.with_extension("").join("subagents");
        if subagents.is_dir() {
            let mut children = vec![];
            walk(&subagents, 2, &mut children)?;
            for child in children
                .into_iter()
                .filter(|p| p.extension().is_some_and(|s| s == "jsonl"))
            {
                let content = stable(&child)?;
                lines(&content)?;
                let relative = child
                    .strip_prefix(root)
                    .map_err(|_| "session_invalid")?
                    .to_str()
                    .ok_or("session_invalid")?
                    .replace('\\', "/");
                if !allowed(agent, &relative) {
                    return Err("session_invalid".into());
                }
                files.insert(relative, STANDARD.encode(content));
            }
        }
    }

    Ok(Manifest {
        version: 1,
        agent: agent.into(),
        session,
        cwd,
        files,
    })
}
fn capture_grok(root: &Path, directory: &Path) -> Result<Manifest> {
    let session = directory
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("session_invalid")?
        .to_string();
    if !bundle::token(&session) {
        return Err("session_invalid".into());
    }
    let mut files = BTreeMap::new();
    let mut cwd = String::new();
    for name in [
        "summary.json",
        "updates.jsonl",
        "chat_history.jsonl",
        "plan.json",
        "signals.json",
        "rewind_points.jsonl",
    ] {
        let p = directory.join(name);
        if !p.exists() {
            continue;
        }
        let bytes = stable(&p)?;
        if name.ends_with("jsonl") {
            lines(&bytes)?;
        } else {
            let v: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|_| "session_invalid")?;
            if name == "summary.json" {
                cwd = v["info"]["cwd"]
                    .as_str()
                    .or_else(|| v["cwd"].as_str())
                    .unwrap_or("")
                    .into();
            }
        }
        files.insert(
            p.strip_prefix(root)
                .map_err(|_| "session_invalid")?
                .to_str()
                .ok_or("session_invalid")?
                .replace('\\', "/"),
            STANDARD.encode(bytes),
        );
    }
    if !directory.join("updates.jsonl").is_file() || !directory.join("summary.json").is_file() {
        return Err("session_invalid".into());
    }
    let group = directory
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .ok_or("session_invalid")?;
    if group != crate::project_mapping::grok_group_for_cwd(&cwd) {
        return Err("session_format_unsupported".into());
    }
    let marker = directory.parent().ok_or("session_invalid")?.join(".cwd");
    if crate::project_mapping::grok_long_cwd(&cwd) && !marker.is_file() {
        return Err("session_format_unsupported".into());
    }
    if marker.exists() && crate::project_mapping::grok_long_cwd(&cwd) {
        let bytes = stable(&marker)?;
        if bytes != cwd.as_bytes() {
            return Err("session_invalid".into());
        }
        let relative = marker
            .strip_prefix(root)
            .map_err(|_| "session_invalid")?
            .to_str()
            .ok_or("session_invalid")?
            .replace('\\', "/");
        if !allowed("grok", &relative) {
            return Err("session_invalid".into());
        }
        files.insert(relative, STANDARD.encode(bytes));
    }
    Ok(Manifest {
        version: 1,
        agent: "grok".into(),
        session,
        cwd,
        files,
    })
}
fn capture_handoff(
    h: &Handoff,
    staging: &Path,
    with_project: bool,
    memory: Option<MemoryContext>,
) -> Result<Manifest> {
    let _parent_scope = handoff_parent_scope(
        staging.parent().ok_or("store_unavailable")?,
        h.path.parent().ok_or("sandbox_reauthorize")?,
    )?;
    if !safe_relative(&h.main_file) || !allowed(&h.agent, &h.main_file) {
        return Err("sync_journal_invalid".into());
    }
    let metadata = fs::symlink_metadata(&h.path).map_err(|_| "source_missing")?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("unsafe_store".into());
    }
    let file = h.path.join(&h.main_file);
    safe_profile_file(&h.path, &h.main_file)?;
    let m = if h.agent == "grok" {
        capture_grok(&h.path, file.parent().ok_or("session_invalid")?)?
    } else if h.agent == "hermes" {
        // Re-capture the managed home's store with the handoff's own root
        // session id, keeping the lineage graph and re-capturing the (possibly
        // grown) portable workspace and referenced attachments. The project
        // working tree is collected from its registered restored directory —
        // never the receiver's mapped worktree, and never an unregistered cwd.
        let project = if with_project {
            h.project
                .as_deref()
                .map(|dir| capture_authorized_project(dir, &dir.to_string_lossy()))
        } else {
            None
        };
        // The registered restored project directory is the only authorized
        // attachment root for a managed handoff; an unregistered cwd is never
        // read.
        let attachment_roots: Vec<PathBuf> = h
            .project
            .as_deref()
            .and_then(|dir| fs::canonicalize(dir).ok())
            .into_iter()
            .collect();
        // Re-capture the external memory from the MANAGED home (its own bound
        // store), so edits continue the restored memory, never the original
        // source.
        let memory_map = capture_memory_for_cycle(&h.path, memory)?.0;
        capture_hermes_store(
            &h.path,
            &file,
            &h.session,
            staging,
            project.as_ref(),
            &attachment_roots,
            &memory_map,
        )?
    } else {
        capture_file(&h.agent, &h.path, &file, staging)?
    };
    if m.session != h.session || m.agent != h.agent {
        return Err("session_conflict".into());
    }
    if m.cwd != h.cwd {
        return Err("project_mapping_required".into());
    }
    // Project entries are a self-describing namespace, not profile files; only
    // the real profile files are resolved back inside the managed home.
    for relative in m
        .files
        .keys()
        .filter(|path| !crate::hermes_project::allowed(path))
    {
        safe_profile_file(&h.path, relative)?;
    }
    Ok(m)
}
fn safe_profile_file(root: &Path, relative: &str) -> Result<()> {
    if !safe_relative(relative) {
        return Err("unsafe_store".into());
    }
    let mut path = root.to_path_buf();
    let components = relative.split('/').collect::<Vec<_>>();
    for (index, component) in components.iter().enumerate() {
        path.push(component);
        let metadata = fs::symlink_metadata(&path).map_err(|_| "source_missing")?;
        if metadata.file_type().is_symlink()
            || (index + 1 < components.len() && !metadata.is_dir())
            || (index + 1 == components.len() && !metadata.is_file())
        {
            return Err("unsafe_store".into());
        }
    }
    Ok(())
}
fn profile_files(
    base: &Path,
    root: &Path,
    depth: usize,
    files: &mut BTreeSet<String>,
) -> Result<()> {
    if depth == 0 {
        return Err("session_limit".into());
    }
    for entry in fs::read_dir(root).map_err(|_| "source_unreadable")? {
        let path = entry.map_err(|_| "source_unreadable")?.path();
        let metadata = fs::symlink_metadata(&path).map_err(|_| "source_unreadable")?;
        if metadata.file_type().is_symlink() {
            return Err("unsafe_store".into());
        }
        if metadata.is_dir() {
            profile_files(base, &path, depth - 1, files)?;
        } else if metadata.is_file() {
            let relative = path.strip_prefix(base).map_err(|_| "unsafe_store")?;
            files.insert(relative.to_string_lossy().replace('\\', "/"));
        } else {
            return Err("unsafe_store".into());
        }
        if files.len() > 2048 {
            return Err("session_limit".into());
        }
    }
    Ok(())
}
fn publish_handoffs(
    root: &Path,
    agent: &str,
    batch: &mut crate::sync::ExportBatch<'_>,
    status: &mut SourceStatus,
    memory: Option<MemoryContext>,
    stop: &impl Fn() -> bool,
) -> Result<()> {
    let mut registry = load_handoffs(root)?;
    for index in 0..registry.entries.len() {
        if registry.entries[index].agent != agent {
            continue;
        }
        if stop() {
            return Err("sync_paused".into());
        }
        let result: Result<()> = (|| {
            let h = &registry.entries[index];
            let manifest = capture_handoff(h, root, true, memory)?;
            let blockers = project_issue_codes(&project_subset(&manifest.files))?;
            let fingerprint = manifest_fingerprint(&manifest)?;
            if fingerprint == h.fingerprint {
                status.captured += 1;
                return Ok(());
            }
            let conversation = bundle::hash(manifest.session.as_bytes());
            let mut journal = Journal {
                node: h.stream_profile.clone(),
                bases: BTreeMap::from([(conversation, h.base.clone())]),
                stamps: BTreeMap::new(),
                content: BTreeMap::new(),
            };
            let id = publish(&manifest, batch, &mut journal, PART_SIZE)?;
            let entry = &mut registry.entries[index];
            entry.base = id;
            entry.fingerprint = fingerprint;
            entry.project_blockers = blockers.clone();
            save_handoffs(root, &registry)?;
            for code in blockers {
                *status.issues.entry(code).or_default() += 1;
            }
            status.captured += 1;
            Ok(())
        })();
        if let Err(code) = result {
            *status.issues.entry(code).or_default() += 1;
        }
    }
    Ok(())
}
fn auto_prepare(
    root: &Path,
    bundle: &bundle::Bundle,
    incoming: &Manifest,
    mappings: &[crate::project_mapping::Mapping],
    memory: Option<MemoryContext>,
) -> Result<(usize, Vec<String>)> {
    if bundle.snapshot.stream.agent == "agy" {
        // A copied database is recoverable data, not a verified standalone CLI profile.
        return Ok((0, Vec::new()));
    }
    if incoming.agent != bundle.snapshot.stream.agent
        || bundle::hash(incoming.session.as_bytes()) != bundle.snapshot.stream.conversation
    {
        return Err("session_invalid".into());
    }
    crate::project_mapping::validate_provider_group(incoming)?;
    if usable_cwd(&incoming.cwd, mappings).is_none() {
        return Err("project_mapping_required".into());
    }
    let manifest = crate::project_mapping::transform_manifest(incoming, mappings)?;
    crate::project_mapping::validate_provider_group(&manifest)?;
    let destination = root.join("managed-profiles").join(&bundle.id);
    // The captured project working tree is materialized into a NEW isolated
    // directory — never the receiver's mapped worktree — and the session cwd is
    // re-rooted to it before attachment references are finalized.
    let project_destination = root.join("managed-projects").join(&bundle.id);
    let (manifest, project_restore) = restore_hermes_project(&manifest, &project_destination)?;
    let issues = project_restore
        .as_ref()
        .map(|restore| restore.blockers.clone())
        .unwrap_or_default();
    let project = project_restore
        .as_ref()
        .and_then(|restore| restore.workdir.clone());
    // Rewrite captured attachment references to the restored home now that its
    // absolute path is known (project-rooted references already followed the
    // restored working tree above).
    let manifest = if manifest.agent == "hermes" {
        finalize_hermes_attachments(&manifest, &destination)?
    } else {
        manifest
    };
    let registry = load_handoffs(root)?;
    if let Some(h) = registry.entries.iter().find(|h| h.base == bundle.id) {
        capture_handoff(h, root, true, memory)?;
        return Ok((0, issues));
    }
    if let Some(h) = registry.entries.iter().find(|h| h.path == destination) {
        capture_handoff(h, root, true, memory)?;
        return Ok((0, issues));
    }
    if destination.exists() {
        // Recover the narrow crash window after the profile rename but before
        // the registry write, provided the new profile is still pristine.
        storage::directory(&destination)?;
        let mut found = BTreeSet::new();
        profile_files(&destination, &destination, 8, &mut found)?;
        if found != manifest.files.keys().cloned().collect() {
            return Err("session_conflict".into());
        }
        for (relative, encoded) in &manifest.files {
            if !allowed(&manifest.agent, relative)
                || safe_profile_file(&destination, relative).is_err()
                || storage::read(&destination.join(relative), MAX_RAW)?
                    != STANDARD.decode(encoded).map_err(|_| "session_invalid")?
            {
                return Err("session_conflict".into());
            }
        }
    } else {
        storage::directory(&root.join("managed-profiles"))?;
        restore_manifest(&manifest, &destination)?;
    }
    // Restore the external memory into the isolated managed home and record the
    // real readiness (verified by re-export, never a synthetic ready).
    let memory_readiness = hermes_restore_memory(memory, &manifest.files, &destination)?;
    register_handoff(
        root,
        bundle,
        &manifest,
        &destination,
        project.as_deref(),
        &issues,
        memory_readiness.as_ref(),
    )?;
    Ok((1, issues))
}
// Explicit dependencies keep the native cycle fixture-testable without a Tauri runtime.
#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub fn cycle(
    root: &Path,
    binding: &Binding,
    key: &SpaceKey,
    remote: &impl Objects,
    agent: &str,
    source: &Path,
    direction: Direction,
    stop: impl Fn() -> bool,
) -> Result<SourceStatus> {
    cycle_with_mappings(
        root,
        binding,
        key,
        remote,
        agent,
        source,
        direction,
        &[],
        stop,
    )
}
#[allow(dead_code, clippy::too_many_arguments)]
pub fn cycle_with_mappings(
    root: &Path,
    binding: &Binding,
    key: &SpaceKey,
    remote: &impl Objects,
    agent: &str,
    source: &Path,
    direction: Direction,
    mappings: &[crate::project_mapping::Mapping],
    stop: impl Fn() -> bool,
) -> Result<SourceStatus> {
    cycle_with_context(
        root, binding, key, remote, agent, source, direction, mappings, None, stop,
    )
}
/// The full cycle, additionally threading the app-config root and settings so a
/// Hermes cycle captures — and a Hermes receive restores — the external-memory
/// continuation with the user's configured CLI and explicit authorization.
#[allow(clippy::too_many_arguments)]
pub fn cycle_with_context(
    root: &Path,
    binding: &Binding,
    key: &SpaceKey,
    remote: &impl Objects,
    agent: &str,
    source: &Path,
    direction: Direction,
    mappings: &[crate::project_mapping::Mapping],
    memory: Option<MemoryContext>,
    stop: impl Fn() -> bool,
) -> Result<SourceStatus> {
    crate::project_mapping::validate_mappings(mappings)?;
    storage::directory(root)?;
    if paths_overlap(source, root)
        || load_handoffs(root)?
            .entries
            .iter()
            .any(|h| paths_overlap(source, &h.path))
    {
        return Err("overlapping_folder".into());
    }
    let replica = Replica::open(&root.join("replica"), &binding.space)?;
    let jp = root.join("native-journal.json");
    let mut journal: Journal = if jp.exists() {
        serde_json::from_slice(&storage::read(&jp, 8 * 1024 * 1024)?)
            .map_err(|_| "sync_journal_invalid")?
    } else {
        Journal {
            node: uuid::Uuid::new_v4().to_string(),
            ..Default::default()
        }
    };
    let mut status = SourceStatus {
        agent: agent.into(),
        state: "complete".into(),
        ..Default::default()
    };
    let mut missing_source = false;
    crate::progress::stage("scan", None);
    if !matches!(direction, Direction::Download) {
        let mut batch = replica.export_batch()?;
        if !source.is_dir() {
            missing_source = true;
        } else if agent == "hermes" {
            // One store, many sessions: take exactly ONE consistent snapshot of the
            // live state.db per cycle and enumerate the sessions from that same
            // image, so every export in the cycle reflects a single point in time
            // and a session captured later in the loop cannot observe a store that
            // changed after the snapshot.
            let db = source.join("state.db");
            if !db.is_file() {
                missing_source = true;
            } else {
                let snapshot = hermes_backup_bytes(&db, root)?;
                let image = tempfile::NamedTempFile::new_in(root)
                    .map_err(|_| "store_unavailable")?
                    .into_temp_path();
                fs::write(&image, &snapshot).map_err(|_| "store_unavailable")?;
                let image_conn = hermes_open_readonly(&image)?;
                let sessions = hermes_sessions_of(&image_conn)?;
                // Capture the portable workspace once for the whole cycle; every
                // session's snapshot merges the same inert file map.
                let workspace = crate::hermes_workspace::capture(source)?;
                // Capture the external-memory continuation once per authorized
                // selected home, for the whole cycle; every session's snapshot
                // merges the same map (or the same explicit blockers).
                let (memory_map, _memory_issues) = capture_memory_for_cycle(source, memory)?;
                // Capture each unique user-authorized project working tree once
                // per cycle; sessions that share a cwd reuse the same inert map.
                let mut project_cache: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
                crate::progress::stage("scan", Some(sessions.len()));
                for (session, cwd) in sessions {
                    crate::progress::advance();
                    if stop() {
                        return Err("sync_paused".into());
                    }
                    if !project_cache.contains_key(&cwd) {
                        let captured = capture_project(&cwd, mappings)?;
                        project_cache.insert(cwd.clone(), captured);
                    }
                    let project = &project_cache[&cwd];
                    // Referenced attachments may only be read from an
                    // explicitly authorized project root, never an arbitrary
                    // recorded cwd.
                    let attachment_roots: Vec<PathBuf> = authorized_project_root(&cwd, mappings)
                        .and_then(|root| fs::canonicalize(root).ok())
                        .into_iter()
                        .collect();
                    let stamp_key = bundle::hash(session.as_bytes());
                    let result: Result<Vec<String>> = (|| {
                        let (m, issues) = capture_hermes_session(
                            source,
                            &image_conn,
                            &session,
                            &cwd,
                            root,
                            &workspace,
                            Some(project),
                            &attachment_roots,
                            &memory_map,
                        )?;
                        let content_fingerprint = manifest_content_fingerprint(&m)?;
                        let conversation = bundle::hash(m.session.as_bytes());
                        let stream = Stream {
                            agent: m.agent.clone(),
                            profile: journal.node.clone(),
                            conversation: conversation.clone(),
                        };
                        if journal.content.get(&stamp_key).is_some_and(|cached| {
                            cached.fingerprint == content_fingerprint
                                && journal.bases.get(&conversation) == Some(&cached.bundle)
                                && cached_native_baseline_complete(&batch, &stream, &cached.bundle)
                        }) {
                            return Ok(issues);
                        }
                        let id = publish(&m, &mut batch, &mut journal, PART_SIZE)?;
                        journal.content.insert(
                            stamp_key,
                            ContentBaseline {
                                fingerprint: content_fingerprint,
                                bundle: id,
                            },
                        );
                        storage::replace(&jp, json(&journal)?.as_bytes())?;
                        Ok(issues)
                    })();
                    match result {
                        Ok(issues) => {
                            for code in issues {
                                *status.issues.entry(code).or_default() += 1;
                            }
                            status.captured += 1;
                        }
                        Err(e) => {
                            *status.issues.entry(e).or_default() += 1;
                        }
                    }
                }
                drop(image_conn);
                drop(image);
            }
        } else {
            let sub = match agent {
                "claude" | "claude-code" => "projects",
                "agy" => "conversations",
                _ => "sessions",
            };
            let dir = source.join(sub);
            let mut paths = Vec::new();
            if dir.is_dir() {
                walk(&dir, 8, &mut paths)?;
            }
            if matches!(agent, "codex" | "chatgpt-work")
                && source.join("archived_sessions").is_dir()
            {
                walk(&source.join("archived_sessions"), 5, &mut paths)?;
            }
            crate::progress::stage("scan", Some(paths.len()));
            for p in paths {
                crate::progress::advance();
                if stop() {
                    return Err("sync_paused".into());
                }
                let candidate = if agent == "grok" {
                    p.file_name().is_some_and(|n| n == "updates.jsonl")
                } else {
                    p.extension()
                        .is_some_and(|s| s == if agent == "agy" { "db" } else { "jsonl" })
                };
                if !candidate {
                    continue;
                }
                if matches!(agent, "claude" | "claude-code")
                    && p.components().any(|c| c.as_os_str() == "subagents")
                {
                    continue;
                }

                let stamp_key = bundle::hash(p.to_string_lossy().as_bytes());
                let stamp = fs::metadata(&p)
                    .ok()
                    .map(|m| format!("{}:{:?}", m.len(), m.modified().ok()))
                    .unwrap_or_default();
                // SQLite WAL and Grok companions need a full consistent capture each cycle.
                if !matches!(agent, "agy" | "grok" | "claude" | "claude-code")
                    && journal.stamps.get(&stamp_key) == Some(&stamp)
                {
                    status.captured += 1;
                    continue;
                }
                let result: Result<()> = (|| {
                    let m = if agent == "grok" {
                        capture_grok(source, p.parent().ok_or("session_invalid")?)?
                    } else {
                        capture_file(agent, source, &p, root)?
                    };
                    let content_fingerprint = manifest_content_fingerprint(&m)?;
                    let conversation = bundle::hash(m.session.as_bytes());
                    let stream = Stream {
                        agent: m.agent.clone(),
                        profile: journal.node.clone(),
                        conversation: conversation.clone(),
                    };
                    if journal.content.get(&stamp_key).is_some_and(|cached| {
                        cached.fingerprint == content_fingerprint
                            && journal.bases.get(&conversation) == Some(&cached.bundle)
                            && cached_native_baseline_complete(&batch, &stream, &cached.bundle)
                    }) {
                        // Stable capture still ran, including SQLite WAL and companion files.
                        // The validated baseline lets us avoid gzip and part construction.
                        return Ok(());
                    }
                    let id = publish(&m, &mut batch, &mut journal, PART_SIZE)?;
                    journal.content.insert(
                        stamp_key.clone(),
                        ContentBaseline {
                            fingerprint: content_fingerprint,
                            bundle: id,
                        },
                    );
                    journal.stamps.insert(stamp_key, stamp);
                    storage::replace(&jp, json(&journal)?.as_bytes())?;
                    Ok(())
                })();
                match result {
                    Ok(()) => status.captured += 1,
                    Err(e) => {
                        *status.issues.entry(e).or_default() += 1;
                    }
                }
            }
        }
        // Explicitly restored profiles have their own baseline and are never
        // scanned as part of the default agent store. Edits form a new child
        // snapshot of the bundle the user restored.
        publish_handoffs(root, agent, &mut batch, &mut status, memory, &stop)?;
    }
    if stop() {
        return Err("sync_paused".into());
    }
    let exchange = queue::exchange_filtered(
        &root.join("exchange"),
        &replica,
        binding,
        key,
        remote,
        direction,
        Some(agent),
    )?;
    status.published = exchange.published;
    status.received = exchange.received;
    if exchange.foreign_objects > 0 {
        status
            .issues
            .insert("foreign_space_objects".into(), exchange.foreign_objects);
    }
    let all = replica.transport_bundles()?;
    let parents: std::collections::BTreeSet<_> = all
        .values()
        .flat_map(|b| b.snapshot.parents.iter().cloned())
        .collect();
    if !matches!(direction, Direction::Upload) {
        crate::progress::stage("restore", None);
        for b in all.values().filter(|b| {
            b.snapshot.stream.agent == agent
                && b.snapshot.device != replica.device_id()
                && !parents.contains(&b.id)
                && b.snapshot.files.contains_key("session.json")
        }) {
            if stop() {
                return Err("sync_paused".into());
            }
            status.available += 1;
            crate::progress::advance();
            let result = (|| {
                let m = unpack(b, &all)?;
                if m.agent != agent
                    || bundle::hash(m.session.as_bytes()) != b.snapshot.stream.conversation
                {
                    return Err("session_invalid".into());
                }
                auto_prepare(root, b, &m, mappings, memory)
            })();
            match result {
                Ok((n, issues)) => {
                    status.restored += n;
                    for code in issues {
                        *status.issues.entry(code).or_default() += 1;
                    }
                }
                Err(e) => {
                    *status.issues.entry(e).or_default() += 1;
                }
            }
        }
    }
    if missing_source && status.captured == 0 && status.restored == 0 {
        status.issues.insert("source_missing".into(), 1);
    }
    if !status.issues.is_empty() {
        status.state = "partial".into();
    } else if status.captured == 0 && status.available == 0 {
        status.state = "empty".into();
    }
    Ok(status)
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Received {
    id: String,
    agent: String,
    session: String,
    cwd: String,
    local_saved_at: Option<u64>,
    parent_ids: Vec<String>,
    origin_device: String,
    generation: usize,
    branch_count: usize,
    managed_profile: Option<String>,
    mapped_cwd: Option<String>,
    /// The NEW isolated project working directory restored with this version,
    /// used as the resume command's working directory. Never the receiver's
    /// mapped worktree.
    project_workdir: Option<String>,
    /// Project-continuity readiness blockers that must be shown, so a copied
    /// project sidecar is never presented as ready.
    project_blockers: Vec<String>,
    /// Hermes-only aggregated continuation readiness (workspace dependencies,
    /// attachment problems, project and memory binding). Absent for every other
    /// agent so their existing contract is unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    continuation: Option<Continuation>,
}
fn usable_cwd(cwd: &str, mappings: &[crate::project_mapping::Mapping]) -> Option<String> {
    if cwd.is_empty() {
        return None;
    }
    let mapped = crate::project_mapping::remap_path(cwd, mappings)
        .ok()
        .flatten();
    // A historical absolute cwd is not a user-selected project grant.
    #[cfg(feature = "mac-app-store")]
    mapped.as_ref()?;
    let candidate = mapped.unwrap_or_else(|| cwd.to_owned());
    Path::new(&candidate).is_dir().then_some(candidate)
}
fn generation(id: &str, all: &BTreeMap<String, bundle::Bundle>, depth: usize) -> usize {
    if depth > all.len() {
        return 0;
    }
    all.get(id)
        .map(|bundle| {
            1 + bundle
                .snapshot
                .parents
                .iter()
                .map(|parent| generation(parent, all, depth + 1))
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0)
}
fn native_root(app: &tauri::AppHandle) -> Result<(PathBuf, String)> {
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
    Ok((root, binding.ok_or("wizard_step_required")?.space))
}
#[tauri::command]
pub async fn list_received_sessions(app: tauri::AppHandle) -> Result<Vec<Received>> {
    tauri::async_runtime::spawn_blocking(move || {
        let (root, space) = native_root(&app)?;
        let mut entries = vec![];
        let settings =
            crate::model::load(&root.join("settings.json"))?.ok_or("invalid_settings")?;
        let mut mapping_scopes = Vec::new();
        let mut permitted_mappings = Vec::new();
        for mapping in &settings.project_mappings {
            if let Ok(scope) = crate::sandbox_access::access(&root, Path::new(&mapping.target)) {
                mapping_scopes.push(scope);
                permitted_mappings.push(mapping.clone());
            }
        }
        for agent in crate::model::AGENTS
            .iter()
            .filter(|a| **a != "agent-memory-os")
        {
            let p = root.join(format!("sessions-{agent}-{space}"));
            if !p.is_dir() {
                continue;
            }
            let replica = Replica::open(&p.join("replica"), &space)?;
            let all = replica.transport_bundles()?;
            let handoffs = load_handoffs(&p)?;
            let parents: std::collections::BTreeSet<_> = all
                .values()
                .flat_map(|b| b.snapshot.parents.iter().cloned())
                .collect();
            for b in all.values().filter(|b| !parents.contains(&b.id)) {
                if b.snapshot.stream.agent != *agent {
                    continue;
                }
                if let Some(meta) = b.snapshot.files.get("session.json") {
                    let v: serde_json::Value =
                        serde_json::from_str(&meta.content).map_err(|_| "session_invalid")?;
                    let managed = handoffs.entries.iter().find(|h| {
                        h.base == b.id
                            && h.path.parent().is_some_and(|parent| {
                                handoff_parent_scope(&root, parent)
                                    .is_ok_and(|_scope| capture_handoff(h, &p, false, None).is_ok())
                            })
                    });
                    let branch_count = all
                        .values()
                        .filter(|other| {
                            other.snapshot.stream == b.snapshot.stream
                                && !parents.contains(&other.id)
                                && other.snapshot.files.contains_key("session.json")
                        })
                        .count();
                    entries.push(Received {
                        id: b.id.clone(),
                        agent: agent.to_string(),
                        session: v["session"].as_str().unwrap_or("").into(),
                        cwd: v["cwd"].as_str().unwrap_or("").into(),
                        // Local immutable object save time, not conversation creation time.
                        local_saved_at: fs::symlink_metadata(
                            p.join("replica/objects").join(format!("{}.json", b.id)),
                        )
                        .ok()
                        .filter(|m| m.is_file() && !m.file_type().is_symlink())
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs()),
                        parent_ids: b.snapshot.parents.clone(),
                        origin_device: b.snapshot.device.clone(),
                        generation: generation(&b.id, &all, 0),
                        branch_count,
                        managed_profile: managed.map(|h| h.path.to_string_lossy().into_owned()),
                        mapped_cwd: managed
                            .and_then(|h| {
                                #[cfg(feature = "mac-app-store")]
                                {
                                    let _ = h;
                                    None
                                }
                                #[cfg(not(feature = "mac-app-store"))]
                                {
                                    Path::new(&h.cwd).is_dir().then(|| h.cwd.clone())
                                }
                            })
                            .or_else(|| {
                                usable_cwd(v["cwd"].as_str().unwrap_or(""), &permitted_mappings)
                            }),
                        project_workdir: managed
                            .and_then(|h| h.project.as_ref())
                            .map(|dir| dir.to_string_lossy().into_owned()),
                        project_blockers: managed
                            .map(|h| h.project_blockers.clone())
                            .unwrap_or_default(),
                        // Hermes-only: aggregate the received snapshot's
                        // continuation prerequisites so the UI can gate
                        // copy/resume; a copied packet is never automatically
                        // ready.
                        continuation: if b.snapshot.stream.agent == "hermes" {
                            let project_path = managed
                                .and_then(|h| h.project.as_ref())
                                .map(|p| p.to_string_lossy().into_owned());
                            let unpacked = unpack(b, &all);
                            let mut continuation = match &unpacked {
                                Ok(received) => continuation_for(received, project_path),
                                Err(_) => Continuation {
                                    state: "blocked".into(),
                                    blockers: vec!["session_invalid".into()],
                                    warnings: Vec::new(),
                                    project_path,
                                    verified: Vec::new(),
                                },
                            };
                            // A restored version carries the real, host-resolved
                            // requirement verdict: env/runtime/os are probed
                            // locally, and opaque requirements are honored only
                            // when explicitly acknowledged for this manifest.
                            if let (Ok(received), Some(h)) = (&unpacked, managed) {
                                apply_requirements_evidence(
                                    &mut continuation,
                                    &received.files,
                                    &h.path,
                                );
                            }
                            // A restored managed home carries the real memory
                            // readiness (verified by re-export); it supersedes
                            // the capture-side memory verdict so a completed
                            // restore is never reported as still unverified.
                            if let Some(readiness) = managed.and_then(|h| h.memory.as_ref()) {
                                apply_memory_readiness(&mut continuation, readiness);
                            }
                            Some(continuation)
                        } else {
                            None
                        },
                    });
                }
            }
        }
        Ok(entries)
    })
    .await
    .map_err(|_| "store_unavailable".to_string())?
}
/// Restore into a NEW child profile. Never replace a session in an existing/live store.
#[cfg(test)]
pub fn restore(
    bundle: &bundle::Bundle,
    all: &BTreeMap<String, bundle::Bundle>,
    destination: &Path,
) -> Result<Manifest> {
    bundle.validate()?;
    let m = unpack(bundle, all)?;
    if m.agent != bundle.snapshot.stream.agent
        || bundle::hash(m.session.as_bytes()) != bundle.snapshot.stream.conversation
    {
        return Err("session_invalid".into());
    }
    restore_manifest(&m, destination)?;
    Ok(m)
}
fn restore_manifest(m: &Manifest, destination: &Path) -> Result<()> {
    if !manifest_paths_safe(m) {
        return Err("session_invalid".into());
    }
    if destination.exists() {
        return Err("restore_destination_exists".into());
    }
    let parent = destination.parent().ok_or("unsafe_store")?;
    let stage = tempfile::tempdir_in(parent).map_err(|_| "store_unavailable")?;
    for (relative, text) in &m.files {
        // All restored content remains inert until the user opens it in the chosen agent.
        if !allowed(&m.agent, relative) {
            return Err("session_invalid".into());
        }
        let path = stage.path().join(relative);
        fs::create_dir_all(path.parent().ok_or("unsafe_store")?)
            .map_err(|_| "store_unavailable")?;
        let bytes = STANDARD.decode(text).map_err(|_| "session_invalid")?;
        storage::immutable(&path, &bytes)?;
    }
    let mut identities = BTreeSet::new();
    for relative in m.files.keys() {
        let canonical =
            fs::canonicalize(stage.path().join(relative)).map_err(|_| "unsafe_store")?;
        if !identities.insert(canonical) {
            return Err("session_invalid".into());
        }
    }
    // Unique child name and atomic rename keep partially restored profiles invisible.
    fs::rename(stage.path(), destination).map_err(|_| "restore_destination_exists")?;
    Ok(())
}
fn register_handoff(
    root: &Path,
    bundle: &bundle::Bundle,
    manifest: &Manifest,
    destination: &Path,
    project: Option<&Path>,
    project_blockers: &[String],
    memory: Option<&crate::hermes_memory::MemoryReadiness>,
) -> Result<()> {
    let mut registry = load_handoffs(root)?;
    if registry.entries.iter().any(|h| h.path == destination) {
        return Err("restore_destination_exists".into());
    }
    registry.entries.push(Handoff {
        path: destination.to_path_buf(),
        agent: manifest.agent.clone(),
        session: manifest.session.clone(),
        main_file: main_file(manifest)?,
        cwd: manifest.cwd.clone(),
        stream_profile: bundle.snapshot.stream.profile.clone(),
        base: bundle.id.clone(),
        fingerprint: manifest_fingerprint(manifest)?,
        project: project.map(Path::to_path_buf),
        project_blockers: project_blockers.to_vec(),
        memory: memory.cloned(),
    });
    save_handoffs(root, &registry)
}
#[tauri::command]
pub async fn restore_received_session(
    app: tauri::AppHandle,
    worker: tauri::State<'_, crate::worker::Worker>,
    agent: String,
    id: String,
) -> Result<Option<String>> {
    if worker.active() {
        return Err("sync_running".into());
    }
    if !crate::model::agent_available(&agent) || !bundle::is_hash(&id) {
        return Err("session_invalid".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let Some(picked) = rfd::FileDialog::new().pick_folder() else {
            return Ok(None);
        };
        let (root, space) = native_root(&app)?;
        let folder = crate::sandbox_access::grant_selected(&root, &picked)?;
        let _restore_scope = crate::sandbox_access::access(&root, &folder)?;
        let settings =
            crate::model::load(&root.join("settings.json"))?.ok_or("invalid_settings")?;
        if paths_overlap(&folder, &root) {
            return Err("overlapping_folder".into());
        }
        let sources = crate::detect(Some(&settings));
        #[cfg(feature = "mac-app-store")]
        let overlapping_source = sources
            .iter()
            .any(|source| restore_overlaps_source(&folder, Path::new(&source.path), &root));
        #[cfg(not(feature = "mac-app-store"))]
        let overlapping_source = sources
            .iter()
            .any(|source| paths_overlap(&folder, Path::new(&source.path)));
        if overlapping_source {
            return Err("overlapping_folder".into());
        }
        let replica = Replica::open(
            &root
                .join(format!("sessions-{agent}-{space}"))
                .join("replica"),
            &space,
        )?;
        let all = replica.transport_bundles()?;
        let b = all.get(&id).ok_or("session_invalid")?;
        b.validate()?;
        let incoming = unpack(b, &all)?;
        if incoming.agent != agent
            || bundle::hash(incoming.session.as_bytes()) != b.snapshot.stream.conversation
        {
            return Err("session_invalid".into());
        }
        #[cfg(feature = "mac-app-store")]
        let relevant_mappings = if agent == "agy" {
            Vec::new()
        } else {
            crate::project_mapping::relevant_mapping(&incoming.cwd, &settings.project_mappings)?
        };
        #[cfg(not(feature = "mac-app-store"))]
        let relevant_mappings = settings.project_mappings.clone();
        let _mapping_scopes = crate::sandbox_access::mapping_scopes(&root, &relevant_mappings)?;
        let manifest = if agent == "agy" {
            incoming
        } else {
            crate::project_mapping::validate_provider_group(&incoming)?;
            if usable_cwd(&incoming.cwd, &relevant_mappings).is_none() {
                return Err("project_mapping_required".into());
            }
            let mapped = crate::project_mapping::transform_manifest(&incoming, &relevant_mappings)?;
            crate::project_mapping::validate_provider_group(&mapped)?;
            mapped
        };
        let restore_id = uuid::Uuid::new_v4();
        let target = folder.join(format!("Bastet-{agent}-{restore_id}"));
        // The captured project working tree is materialized into a NEW isolated
        // sibling directory — never the receiver's mapped worktree — and the
        // session cwd is re-rooted to it before attachment references finalize.
        let project_destination = folder.join(format!("Bastet-{agent}-{restore_id}-project"));
        let (manifest, project_restore) = restore_hermes_project(&manifest, &project_destination)?;
        let manifest = if manifest.agent == "hermes" {
            finalize_hermes_attachments(&manifest, &target)?
        } else {
            manifest
        };
        restore_manifest(&manifest, &target)?;
        if agent != "agy" {
            let blockers = project_restore
                .as_ref()
                .map(|restore| restore.blockers.as_slice())
                .unwrap_or(&[]);
            let project = project_restore
                .as_ref()
                .and_then(|restore| restore.workdir.as_deref());
            // Restore the external memory into the NEW isolated home and record
            // the real readiness; a failure is an explicit blocker, never a
            // synthetic ready.
            let memory_readiness = hermes_restore_memory(
                Some(MemoryContext {
                    app_root: &root,
                    settings: &settings,
                }),
                &manifest.files,
                &target,
            )?;
            register_handoff(
                &root.join(format!("sessions-{agent}-{space}")),
                b,
                &manifest,
                &target,
                project,
                blockers,
                memory_readiness.as_ref(),
            )?;
        }
        Ok(Some(target.to_string_lossy().into()))
    })
    .await
    .map_err(|_| "store_unavailable".to_string())?
}

fn compare_session(
    app: &tauri::AppHandle,
    agent: &str,
    id: &str,
    source_agent: Option<&str>,
) -> Result<crate::review::Comparison> {
    if !crate::model::agent_available(agent) || !bundle::is_hash(id) {
        return Err("session_invalid".into());
    }
    let (root, space) = native_root(app)?;
    let settings = crate::model::load(&root.join("settings.json"))?.ok_or("invalid_settings")?;
    let agents = crate::detect(Some(&settings));
    let canonical = if agent == "claude" {
        "claude-code"
    } else if agent == "chatgpt-work" {
        "codex"
    } else {
        agent
    };
    let chosen = source_agent.unwrap_or(canonical);
    if chosen != canonical && !(canonical == "codex" && chosen == "chatgpt-work") {
        return Err("source_missing".into());
    }
    let source = agents
        .iter()
        .find(|a| a.id == chosen)
        .ok_or("source_missing")?;
    let _source_scope = crate::sandbox_access::access(&root, Path::new(&source.path))?;
    let replica = Replica::open(
        &root
            .join(format!("sessions-{canonical}-{space}"))
            .join("replica"),
        &space,
    )?;
    let all = replica.transport_bundles()?;
    let b = all.get(id).ok_or("session_invalid")?;
    let manifest = unpack(b, &all)?;
    if manifest.agent != canonical || manifest.files.len() > 2048 {
        return Err("session_invalid".into());
    }
    let mut files = Vec::new();
    for (path, encoded) in manifest.files {
        if !allowed(canonical, &path) {
            return Err("session_invalid".into());
        }
        let incoming = STANDARD.decode(encoded).map_err(|_| "session_invalid")?;
        files.push(crate::review::diff(
            path.clone(),
            crate::review::local_file(Path::new(&source.path), &path, MAX_RAW)?,
            &incoming,
        ));
    }
    crate::review::comparison(&root, &format!("{canonical}:{id}"), files)
}
#[tauri::command]
pub async fn compare_received_session(
    app: tauri::AppHandle,
    agent: String,
    id: String,
    source_agent: Option<String>,
) -> Result<crate::review::Comparison> {
    tauri::async_runtime::spawn_blocking(move || {
        compare_session(&app, &agent, &id, source_agent.as_deref())
    })
    .await
    .map_err(|_| "store_unavailable".to_string())?
}
#[tauri::command]
pub async fn review_received_session(
    app: tauri::AppHandle,
    agent: String,
    id: String,
    fingerprint: String,
    source_agent: Option<String>,
) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let comparison = compare_session(&app, &agent, &id, source_agent.as_deref())?;
        if comparison.fingerprint != fingerprint {
            return Err("source_changing".into());
        }
        let (root, _) = native_root(&app)?;
        let canonical = match agent.as_str() {
            "claude" => "claude-code",
            "chatgpt-work" => "codex",
            a => a,
        };
        crate::review::mark(&root, format!("{canonical}:{id}"), &fingerprint)
    })
    .await
    .map_err(|_| "store_unavailable".to_string())?
}

fn run_recheck(
    app: &tauri::AppHandle,
    agent: &str,
    id: &str,
    acknowledge: bool,
    setup_provider: bool,
) -> Result<RecheckResult> {
    let (root, space) = native_root(app)?;
    let settings = crate::model::load(&root.join("settings.json"))?.ok_or("invalid_settings")?;
    let sessions_root = root.join(format!("sessions-{agent}-{space}"));
    let staging = tempfile::tempdir().map_err(|_| "store_unavailable")?;
    recheck_continuation_with(
        &sessions_root,
        &root,
        &space,
        &settings,
        agent,
        id,
        acknowledge,
        setup_provider,
        staging.path(),
    )
}

/// Explicit, user-initiated continuation recheck of an already-restored Hermes
/// version. The target is resolved from the app registry by version id (never a
/// caller-supplied path) and only that isolated managed home is touched: no new
/// managed profile is created. Re-runs the dependency and memory readiness,
/// applies the official provider import when the provider is now installed, and
/// — when asked — records a hash-bound acknowledgment of the opaque workspace
/// requirements. In a build without the external channel this is non-CLI: the
/// provider setup path is refused and the memory verdict is reported as
/// unavailable, never inferred ready. Refused while sync is running.
#[tauri::command]
pub async fn recheck_continuation(
    app: tauri::AppHandle,
    worker: tauri::State<'_, crate::worker::Worker>,
    agent: String,
    id: String,
    acknowledge_requirements: Option<bool>,
) -> Result<RecheckResult> {
    if worker.active() {
        return Err("sync_running".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        run_recheck(
            &app,
            &agent,
            &id,
            acknowledge_requirements.unwrap_or(false),
            false,
        )
    })
    .await
    .map_err(|_| "store_unavailable".to_string())?
}

/// Explicit, recipient-approved memory provider setup for an already-restored
/// Hermes version: runs the trusted configured local CLI's fixed
/// `hermes install --hermes-home <EXACT MANAGED HOME>` operation into the
/// registered isolated home only, then re-runs the official memory
/// import/verify. Never available in a build without the external channel.
#[cfg(not(feature = "mac-app-store"))]
#[tauri::command]
pub async fn setup_memory_provider(
    app: tauri::AppHandle,
    worker: tauri::State<'_, crate::worker::Worker>,
    agent: String,
    id: String,
) -> Result<RecheckResult> {
    if worker.active() {
        return Err("sync_running".into());
    }
    tauri::async_runtime::spawn_blocking(move || run_recheck(&app, &agent, &id, false, true))
        .await
        .map_err(|_| "store_unavailable".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::collections::BTreeSet;
    use std::io::Write;

    #[cfg(feature = "mac-app-store")]
    #[test]
    fn store_return_capture_requires_restore_parent_grant() {
        let config = tempfile::tempdir().unwrap();
        let root = config.path().join("sessions-pi-space");
        storage::directory(&root).unwrap();
        let managed = root.join("managed-profiles");
        storage::directory(&managed).unwrap();
        let external = tempfile::tempdir().unwrap();
        assert!(handoff_parent_scope(config.path(), &managed)
            .unwrap()
            .is_none());
        assert_eq!(
            handoff_parent_scope(config.path(), external.path()).err(),
            Some("sandbox_reauthorize".into())
        );
        #[cfg(unix)]
        {
            let alias = config.path().join("outside-link");
            std::os::unix::fs::symlink(external.path(), &alias).unwrap();
            assert_eq!(
                handoff_parent_scope(config.path(), &alias).err(),
                Some("sandbox_reauthorize".into())
            );
        }
    }

    #[cfg(feature = "mac-app-store")]
    #[test]
    fn revoked_external_handoff_is_partial_while_private_handoff_continues() {
        let config = tempfile::tempdir().unwrap();
        let root = config.path().join("sync");
        let managed = root.join("managed-profiles/healthy");
        let session = "019f0000-0000-7000-8000-000000000001";
        let cwd = "/fixture";
        cross_fixture("pi", &managed, cwd, session, "healthy");
        let mut healthy = Handoff {
            path: managed,
            agent: "pi".into(),
            session: session.into(),
            main_file: cross_relative("pi", cwd, session),
            cwd: cwd.into(),
            stream_profile: "healthy".into(),
            base: "baseline".into(),
            fingerprint: String::new(),
            project: None,
            project_blockers: Vec::new(),
            memory: None,
        };
        healthy.fingerprint =
            manifest_fingerprint(&capture_handoff(&healthy, &root, true, None).unwrap()).unwrap();
        let external = tempfile::tempdir().unwrap();
        let stale = Handoff {
            path: external.path().join("old-restore"),
            fingerprint: "unchanged".into(),
            ..healthy.clone()
        };
        let registry = Handoffs {
            version: 1,
            entries: vec![stale, healthy],
        };
        save_handoffs(&root, &registry).unwrap();
        let before = fs::read(handoffs_path(&root)).unwrap();
        let binding = Binding {
            folder: "folder".into(),
            space: "space".into(),
            proof: "proof".into(),
        };
        let key = SpaceKey::generate().unwrap();
        let remote = Remote(
            RefCell::new(BTreeMap::from([(
                "proof".into(),
                queue::proof_bundle("space").unwrap(),
            )])),
            Cell::new(0),
        );
        let result = cycle(
            &root,
            &binding,
            &key,
            &remote,
            "pi",
            &config.path().join("missing"),
            Direction::Upload,
            || false,
        )
        .unwrap();
        assert_eq!(result.captured, 1);
        assert_eq!(result.issues.get("sandbox_reauthorize"), Some(&1));
        assert_eq!(result.state, "partial");
        assert_eq!(fs::read(handoffs_path(&root)).unwrap(), before);
    }
    #[derive(Serialize, Deserialize)]
    struct ExchangeCase {
        agent: String,
        profile: String,
        session: String,
        marker: String,
        cwd: String,
        head: String,
    }
    #[derive(Serialize, Deserialize)]
    struct ExchangeManifest {
        cases: Vec<ExchangeCase>,
    }
    struct Remote(RefCell<BTreeMap<String, bundle::Bundle>>, Cell<usize>);
    impl Objects for Remote {
        fn ids(&self, _: &str) -> Result<Vec<String>> {
            Ok(self.0.borrow().keys().cloned().collect())
        }
        fn allocate(&self) -> Result<String> {
            let id = self.1.get();
            self.1.set(id + 1);
            Ok(format!("id-{id}"))
        }
        fn put(&self, _: &str, id: &str, _: &SpaceKey, b: &bundle::Bundle) -> Result<()> {
            self.0.borrow_mut().insert(id.into(), b.clone());
            Ok(())
        }
        fn get(&self, _: &str, id: &str, _: &str, _: &SpaceKey) -> Result<bundle::Bundle> {
            self.0.borrow().get(id).cloned().ok_or("missing".into())
        }
    }
    fn write(root: &Path, path: &str, bytes: &[u8]) -> PathBuf {
        let p = root.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, bytes).unwrap();
        p
    }
    fn fixture(agent: &str, root: &Path) {
        let id = "019f0000-0000-7000-8000-000000000001";
        match agent {
            "codex" | "chatgpt-work" => {
                write(root,&format!("sessions/2026/09/05/rollout-{id}.jsonl"),format!("{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{id}\",\"cwd\":\"/project\"}}}}\n{{\"type\":\"response_item\",\"payload\":{{\"text\":\"fixture\"}}}}\n").as_bytes());
            }
            "claude" | "claude-code" => {
                write(root,&format!("projects/-project/{id}.jsonl"),format!("{{\"type\":\"user\",\"sessionId\":\"{id}\",\"cwd\":\"/project\",\"message\":{{\"role\":\"user\",\"content\":\"fixture\"}}}}\n").as_bytes());
            }
            "pi" => {
                write(root,&format!("sessions/--project--/2026-{id}.jsonl"),format!("{{\"type\":\"session\",\"version\":3,\"id\":\"{id}\",\"cwd\":\"/project\"}}\n").as_bytes());
            }
            "grok" => {
                let group = crate::project_mapping::grok_group_for_cwd("/project");
                write(
                    root,
                    &format!("sessions/{group}/{id}/updates.jsonl"),
                    format!("{}\n", serde_json::json!({"method":"session/update","params":{"sessionId":id,"update":{"sessionUpdate":"user_message_chunk","content":{"type":"text","text":"fixture"}}}})).as_bytes(),
                );
                write(
                    root,
                    &format!("sessions/{group}/{id}/summary.json"),
                    serde_json::json!({"info":{"id":id,"cwd":"/project"}})
                        .to_string()
                        .as_bytes(),
                );
            }
            "agy" => {
                let p = write(root, &format!("conversations/{id}.db"), b"");
                let c = rusqlite::Connection::open(p).unwrap();
                c.execute_batch("CREATE TABLE trajectory_meta(trajectory_id TEXT PRIMARY KEY);CREATE TABLE steps(idx INTEGER PRIMARY KEY,data BLOB); INSERT INTO steps VALUES(1,X'0102');").unwrap();
            }
            _ => unreachable!(),
        }
        write(root, "auth.json", b"secret-must-not-travel");
    }
    #[test]
    fn full_capture_skips_compression_only_for_unchanged_validated_baseline() {
        for agent in ["claude-code", "grok", "agy"] {
            let temp = tempfile::tempdir().unwrap();
            let home = temp.path().join("home");
            fixture(agent, &home);
            let root = temp.path().join("sync");
            let binding = Binding {
                folder: "folder".into(),
                space: "space".into(),
                proof: "proof".into(),
            };
            let key = SpaceKey::generate().unwrap();
            let remote = Remote(
                RefCell::new(BTreeMap::from([(
                    "proof".into(),
                    queue::proof_bundle("space").unwrap(),
                )])),
                Cell::new(0),
            );
            ENCODE_CALLS.with(|calls| calls.set(0));
            let first = cycle(
                &root,
                &binding,
                &key,
                &remote,
                agent,
                &home,
                Direction::Upload,
                || false,
            )
            .unwrap();
            assert_eq!(first.published, 1, "{agent}: {:?}", first.issues);
            assert_eq!(ENCODE_CALLS.with(Cell::get), 1, "{agent}");
            let journal_path = root.join("native-journal.json");
            let first_journal = fs::read(&journal_path).unwrap();
            let journal: Journal = serde_json::from_slice(&first_journal).unwrap();
            let base = journal.bases.values().next().unwrap().clone();
            let second = cycle(
                &root,
                &binding,
                &key,
                &remote,
                agent,
                &home,
                Direction::Upload,
                || false,
            )
            .unwrap();
            assert_eq!(second.captured, 1, "{agent}: {:?}", second.issues);
            assert_eq!(second.published, 0, "{agent}");
            assert_eq!(ENCODE_CALLS.with(Cell::get), 1, "{agent}");
            assert_eq!(fs::read(&journal_path).unwrap(), first_journal, "{agent}");

            let session = "019f0000-0000-7000-8000-000000000001";
            let mut agy_connection = None;
            match agent {
                "claude-code" => {
                    write(
                        &home,
                        &format!("projects/-project/{session}/subagents/child.jsonl"),
                        b"{\"type\":\"user\",\"message\":{\"content\":\"new companion\"}}\n",
                    );
                }
                "grok" => {
                    let group = crate::project_mapping::grok_group_for_cwd("/project");
                    write(
                        &home,
                        &format!("sessions/{group}/{session}/summary.json"),
                        serde_json::json!({"info":{"id":session,"cwd":"/project"},"summary":"changed companion"})
                            .to_string()
                            .as_bytes(),
                    );
                }
                "agy" => {
                    let file = home.join(format!("conversations/{session}.db"));
                    let connection = rusqlite::Connection::open(file).unwrap();
                    connection
                        .execute_batch(
                            "PRAGMA journal_mode=WAL; INSERT INTO steps VALUES(2,X'0304');",
                        )
                        .unwrap();
                    agy_connection = Some(connection);
                }
                _ => unreachable!(),
            }
            let changed = cycle(
                &root,
                &binding,
                &key,
                &remote,
                agent,
                &home,
                Direction::Upload,
                || false,
            )
            .unwrap();
            assert_eq!(changed.published, 1, "{agent}: {:?}", changed.issues);
            assert_eq!(ENCODE_CALLS.with(Cell::get), 2, "{agent}");
            let all = Replica::open(&root.join("replica"), "space")
                .unwrap()
                .transport_bundles()
                .unwrap();
            let child = all
                .values()
                .find(|b| b.snapshot.parents == vec![base.clone()])
                .unwrap();
            assert_eq!(child.snapshot.stream.agent, agent);
            drop(agy_connection);
        }
    }
    #[test]
    fn missing_cached_baseline_never_skips_compression() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        fixture("claude-code", &home);
        let root = temp.path().join("sync");
        let binding = Binding {
            folder: "folder".into(),
            space: "space".into(),
            proof: "proof".into(),
        };
        let key = SpaceKey::generate().unwrap();
        let remote = Remote(
            RefCell::new(BTreeMap::from([(
                "proof".into(),
                queue::proof_bundle("space").unwrap(),
            )])),
            Cell::new(0),
        );
        ENCODE_CALLS.with(|calls| calls.set(0));
        cycle(
            &root,
            &binding,
            &key,
            &remote,
            "claude-code",
            &home,
            Direction::Upload,
            || false,
        )
        .unwrap();
        let journal: Journal =
            serde_json::from_slice(&fs::read(root.join("native-journal.json")).unwrap()).unwrap();
        let base = journal.bases.values().next().unwrap();
        fs::remove_file(root.join("replica/objects").join(format!("{base}.json"))).unwrap();
        let result = cycle(
            &root,
            &binding,
            &key,
            &remote,
            "claude-code",
            &home,
            Direction::Upload,
            || false,
        )
        .unwrap();
        assert_eq!(ENCODE_CALLS.with(Cell::get), 2);
        assert_eq!(result.issues.get("unknown_baseline"), Some(&1));
        assert_eq!(result.published, 0);
    }
    #[test]
    #[ignore = "requires installed Grok and explicit synthetic fixture root; never reads default profiles"]
    fn installed_native_cli_restored_fixture() {
        use std::process::Command;
        let fixtures = PathBuf::from(
            std::env::var("BASTET_NATIVE_FIXTURES").expect("explicit synthetic fixtures"),
        );
        let temp = tempfile::tempdir().unwrap();
        let key = SpaceKey::generate().unwrap();
        for agent in ["grok", "agy"] {
            let source = fixtures.join(agent);
            let manifest = if agent == "grok" {
                let group = fs::read_dir(source.join("sessions"))
                    .unwrap()
                    .next()
                    .unwrap()
                    .unwrap()
                    .path();
                let session = fs::read_dir(group).unwrap().next().unwrap().unwrap().path();
                capture_grok(&source, &session).unwrap()
            } else {
                let file = fs::read_dir(source.join("conversations"))
                    .unwrap()
                    .next()
                    .unwrap()
                    .unwrap()
                    .path();
                capture_file(agent, &source, &file, temp.path()).unwrap()
            };
            let replica =
                Replica::open(&temp.path().join(format!("replica-{agent}")), "space").unwrap();
            let mut journal = Journal {
                node: "fixture".into(),
                ..Default::default()
            };
            let mut batch = replica.export_batch().unwrap();
            let id = publish(&manifest, &mut batch, &mut journal, PART_SIZE).unwrap();
            drop(batch);
            let all = replica.transport_bundles().unwrap();
            let received = all
                .iter()
                .map(|(id, b)| {
                    (
                        id.clone(),
                        key.open("space", &key.seal(b).unwrap()).unwrap(),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            let target = temp.path().join(format!("restored-{agent}"));
            restore(&received[&id], &received, &target).unwrap();
            if agent == "grok" {
                let cli = std::env::var("BASTET_GROK_CLI").expect("explicit installed CLI");
                let output = temp.path().join("export.md");
                let status = Command::new(cli)
                    .env("GROK_HOME", &target)
                    .args(["export", &manifest.session])
                    .arg(&output)
                    .status()
                    .unwrap();
                assert!(status.success());
                let text = fs::read_to_string(output).unwrap();
                assert!(
                    text.contains("BASTET_RESTORE_SENTINEL_USER")
                        && text.contains("BASTET_RESTORE_SENTINEL_AGENT")
                );
            } else {
                let path = target.join(manifest.files.keys().next().unwrap());
                let db = rusqlite::Connection::open_with_flags(
                    path,
                    rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                )
                .unwrap();
                let integrity: String = db
                    .query_row("PRAGMA integrity_check", [], |r| r.get(0))
                    .unwrap();
                assert_eq!(integrity, "ok");
                let count: i64 = db
                    .query_row("SELECT count(*) FROM steps", [], |r| r.get(0))
                    .unwrap();
                assert!(count > 0);
            }
        }
    }
    #[test]
    fn seven_sources_transfer_restore_and_do_not_upload_unchanged_again() {
        for agent in [
            "claude",
            "claude-code",
            "codex",
            "chatgpt-work",
            "pi",
            "grok",
            "agy",
        ] {
            let temp = tempfile::tempdir().unwrap();
            let home = temp.path().join("home");
            fixture(agent, &home);
            let binding = Binding {
                folder: "folder".into(),
                space: "space".into(),
                proof: "proof".into(),
            };
            let key = SpaceKey::generate().unwrap();
            let remote = Remote(
                RefCell::new(BTreeMap::from([(
                    "proof".into(),
                    queue::proof_bundle("space").unwrap(),
                )])),
                Cell::new(0),
            );
            let a = temp.path().join("a");
            let b = temp.path().join("b");
            let r = cycle(
                &a,
                &binding,
                &key,
                &remote,
                agent,
                &home,
                Direction::Upload,
                || false,
            )
            .unwrap();
            assert_eq!(r.published, 1, "{agent}: {:?}", r.issues);
            assert_eq!(
                cycle(
                    &a,
                    &binding,
                    &key,
                    &remote,
                    agent,
                    &home,
                    Direction::Upload,
                    || false
                )
                .unwrap()
                .published,
                0,
                "{agent}"
            );
            let r = cycle(
                &b,
                &binding,
                &key,
                &remote,
                agent,
                &temp.path().join("absent"),
                Direction::Download,
                || false,
            )
            .unwrap();
            assert_eq!(r.received, 1);
            // The synthetic cwd is intentionally absent on the receiving
            // device. The snapshot remains available until a project mapping
            // is configured; Agy is database recovery only.
            assert_eq!(r.restored, 0);
            assert_eq!(r.available, 1);
            if agent != "agy" {
                assert_eq!(r.issues.get("project_mapping_required"), Some(&1));
            }
            let rep = Replica::open(&b.join("replica"), "space").unwrap();
            let all = rep.transport_bundles().unwrap();
            let snapshot = all
                .values()
                .find(|s| s.snapshot.stream.agent == agent)
                .unwrap();
            let dest = temp.path().join("restored");
            let manifest = restore(snapshot, &all, &dest).unwrap();
            assert!(!dest.join("auth.json").exists());
            for (p, text) in &manifest.files {
                assert_eq!(
                    fs::read(dest.join(p)).unwrap(),
                    STANDARD.decode(text).unwrap()
                );
            }
            assert!(restore(snapshot, &all, &dest).is_err());
            drop(rep);
            assert_eq!(
                cycle(
                    &b,
                    &binding,
                    &key,
                    &remote,
                    agent,
                    &dest,
                    Direction::Download,
                    || false
                )
                .unwrap()
                .received,
                0
            );
        }
    }
    #[test]
    fn folder_transport_captures_receives_and_restores_managed_pi_fixture() {
        use crate::cloud::folder::{create_dirs, FolderObjects};
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("pi-source");
        fixture("pi", &home);
        let binding = Binding {
            folder: "fixture-folder".into(),
            space: "fixture-space".into(),
            proof: "fixture-proof".into(),
        };
        let cloud = temp.path().join("selected-cloud-folder");
        fs::create_dir(&cloud).unwrap();
        create_dirs(&cloud, &binding).unwrap();
        let remote = FolderObjects::new(&cloud, &binding).unwrap();
        let key = SpaceKey::generate().unwrap();
        remote
            .put(
                &binding.folder,
                &binding.proof,
                &key,
                &queue::proof_bundle(&binding.space).unwrap(),
            )
            .unwrap();
        let a = temp.path().join("a-sync");
        let b = temp.path().join("b-sync");
        let sent = cycle(
            &a,
            &binding,
            &key,
            &remote,
            "pi",
            &home,
            Direction::Upload,
            || false,
        )
        .unwrap();
        assert_eq!(sent.published, 1, "{:?}", sent.issues);
        let received = cycle(
            &b,
            &binding,
            &key,
            &remote,
            "pi",
            &temp.path().join("absent-pi"),
            Direction::Download,
            || false,
        )
        .unwrap();
        assert_eq!(received.received, 1, "{:?}", received.issues);
        assert_eq!(received.available, 1);
        let all = Replica::open(&b.join("replica"), &binding.space)
            .unwrap()
            .transport_bundles()
            .unwrap();
        let snapshot = all
            .values()
            .find(|s| s.snapshot.stream.agent == "pi")
            .unwrap();
        let target = temp.path().join("managed-pi-profile");
        let manifest = restore(snapshot, &all, &target).unwrap();
        register_handoff(&b, snapshot, &manifest, &target, None, &[], None).unwrap();
        assert_eq!(load_handoffs(&b).unwrap().entries.len(), 1);
        assert!(!target.join("auth.json").exists());
        let restored = fs::read(target.join(manifest.files.keys().next().unwrap())).unwrap();
        assert!(
            String::from_utf8_lossy(&restored).contains("fixture")
                || String::from_utf8_lossy(&restored).contains("session")
        );
        assert_eq!(
            cycle(
                &a,
                &binding,
                &key,
                &remote,
                "pi",
                &home,
                Direction::Upload,
                || false
            )
            .unwrap()
            .published,
            0
        );
    }
    #[test]
    fn managed_handoff_roundtrip_keeps_causal_parents_and_original_files() {
        use std::io::Write;
        let temp = tempfile::tempdir().unwrap();
        let a_home = temp.path().join("a-home");
        let b_home = temp.path().join("b-home");
        let a_root = temp.path().join("a-sync");
        let b_root = temp.path().join("b-sync");
        fixture("pi", &a_home);
        fs::create_dir_all(&b_home).unwrap();
        let original = fs::read(
            a_home.join("sessions/--project--/2026-019f0000-0000-7000-8000-000000000001.jsonl"),
        )
        .unwrap();
        let binding = Binding {
            folder: "folder".into(),
            space: "space".into(),
            proof: "proof".into(),
        };
        let key = SpaceKey::generate().unwrap();
        let remote = Remote(
            RefCell::new(BTreeMap::from([(
                "proof".into(),
                queue::proof_bundle("space").unwrap(),
            )])),
            Cell::new(0),
        );
        cycle(
            &a_root,
            &binding,
            &key,
            &remote,
            "pi",
            &a_home,
            Direction::Upload,
            || false,
        )
        .unwrap();
        cycle(
            &b_root,
            &binding,
            &key,
            &remote,
            "pi",
            &b_home,
            Direction::Download,
            || false,
        )
        .unwrap();
        let b_replica = Replica::open(&b_root.join("replica"), "space").unwrap();
        let b_all = b_replica.transport_bundles().unwrap();
        let base = b_all
            .values()
            .find(|b| b.snapshot.files.contains_key("session.json"))
            .unwrap()
            .clone();
        let base_manifest = unpack(&base, &b_all).unwrap();
        drop(b_replica);
        let b_profile = b_root.join("b-branch");
        restore_manifest(&base_manifest, &b_profile).unwrap();
        register_handoff(&b_root, &base, &base_manifest, &b_profile, None, &[], None).unwrap();
        let branch_file = b_profile.join(main_file(&base_manifest).unwrap());
        assert_eq!(
            cycle(
                &b_root,
                &binding,
                &key,
                &remote,
                "pi",
                &temp.path().join("empty"),
                Direction::Upload,
                || false
            )
            .unwrap()
            .published,
            0
        );
        fs::create_dir_all(temp.path().join("empty")).unwrap();
        fs::OpenOptions::new()
            .append(true)
            .open(&branch_file)
            .unwrap()
            .write_all(b"{\"type\":\"message\",\"message\":\"B turn\"}\n")
            .unwrap();
        cycle(
            &b_root,
            &binding,
            &key,
            &remote,
            "pi",
            &temp.path().join("empty"),
            Direction::Upload,
            || false,
        )
        .unwrap();
        let b_after = Replica::open(&b_root.join("replica"), "space")
            .unwrap()
            .transport_bundles()
            .unwrap();
        let b_head = b_after
            .values()
            .find(|b| {
                b.snapshot.parents == vec![base.id.clone()]
                    && b.snapshot.files.contains_key("session.json")
            })
            .unwrap()
            .clone();
        assert_eq!(b_head.snapshot.stream, base.snapshot.stream);
        assert_eq!(
            cycle(
                &b_root,
                &binding,
                &key,
                &remote,
                "pi",
                &temp.path().join("empty"),
                Direction::Upload,
                || false
            )
            .unwrap()
            .published,
            0
        );

        cycle(
            &a_root,
            &binding,
            &key,
            &remote,
            "pi",
            &a_home,
            Direction::Download,
            || false,
        )
        .unwrap();
        assert_eq!(
            fs::read(
                a_home.join("sessions/--project--/2026-019f0000-0000-7000-8000-000000000001.jsonl")
            )
            .unwrap(),
            original
        );
        let a_all = Replica::open(&a_root.join("replica"), "space")
            .unwrap()
            .transport_bundles()
            .unwrap();
        let a_b_head = a_all.get(&b_head.id).unwrap();
        let b_manifest = unpack(a_b_head, &a_all).unwrap();
        let a_profile = a_root.join("a-branch");
        restore_manifest(&b_manifest, &a_profile).unwrap();
        register_handoff(&a_root, a_b_head, &b_manifest, &a_profile, None, &[], None).unwrap();
        fs::OpenOptions::new()
            .append(true)
            .open(a_profile.join(main_file(&b_manifest).unwrap()))
            .unwrap()
            .write_all(b"{\"type\":\"message\",\"message\":\"A return turn\"}\n")
            .unwrap();
        cycle(
            &a_root,
            &binding,
            &key,
            &remote,
            "pi",
            &a_home,
            Direction::Upload,
            || false,
        )
        .unwrap();
        let a_after = Replica::open(&a_root.join("replica"), "space")
            .unwrap()
            .transport_bundles()
            .unwrap();
        let a_head = a_after
            .values()
            .find(|b| {
                b.snapshot.parents == vec![b_head.id.clone()]
                    && b.snapshot.files.contains_key("session.json")
            })
            .unwrap();
        let final_manifest = unpack(a_head, &a_after).unwrap();
        let final_bytes = STANDARD
            .decode(final_manifest.files.values().next().unwrap())
            .unwrap();
        let final_text = std::str::from_utf8(&final_bytes).unwrap();
        assert!(final_text.contains("B turn") && final_text.contains("A return turn"));
        assert_eq!(
            fs::read(
                a_home.join("sessions/--project--/2026-019f0000-0000-7000-8000-000000000001.jsonl")
            )
            .unwrap(),
            original
        );
        cycle(
            &b_root,
            &binding,
            &key,
            &remote,
            "pi",
            &b_home,
            Direction::Download,
            || false,
        )
        .unwrap();
        assert!(!fs::read(&branch_file)
            .unwrap()
            .windows(b"A return turn".len())
            .any(|v| v == b"A return turn"));
    }
    #[test]
    fn mapped_receive_prepares_new_profile_and_pristine_capture_does_not_loop() {
        use std::io::Write;
        let temp = tempfile::tempdir().unwrap();
        let a_home = temp.path().join("a-home");
        let b_home = temp.path().join("b-home");
        let a_root = temp.path().join("a-sync");
        let b_root = temp.path().join("b-sync");
        fixture("codex", &a_home);
        let target_project = temp.path().join("receiving-project");
        fs::create_dir_all(&target_project).unwrap();
        let mappings = [crate::project_mapping::Mapping {
            source: "/project".into(),
            target: target_project.to_string_lossy().into_owned(),
        }];
        let binding = Binding {
            folder: "folder".into(),
            space: "space".into(),
            proof: "proof".into(),
        };
        let key = SpaceKey::generate().unwrap();
        let remote = Remote(
            RefCell::new(BTreeMap::from([(
                "proof".into(),
                queue::proof_bundle("space").unwrap(),
            )])),
            Cell::new(0),
        );
        cycle(
            &a_root,
            &binding,
            &key,
            &remote,
            "codex",
            &a_home,
            Direction::Upload,
            || false,
        )
        .unwrap();
        let r = cycle_with_mappings(
            &b_root,
            &binding,
            &key,
            &remote,
            "codex",
            &b_home,
            Direction::Both,
            &mappings,
            || false,
        )
        .unwrap();
        assert_eq!(r.restored, 1, "{:?}", r.issues);
        assert_eq!(r.published, 0);
        assert!(!b_home.exists());
        assert!(!r.issues.contains_key("source_missing"));
        let registry = load_handoffs(&b_root).unwrap();
        assert_eq!(registry.entries.len(), 1);
        let handoff = &registry.entries[0];
        let file = handoff.path.join(&handoff.main_file);
        let before = fs::read(&file).unwrap();
        let header: serde_json::Value = serde_json::from_str(
            std::str::from_utf8(&before)
                .unwrap()
                .lines()
                .next()
                .unwrap(),
        )
        .unwrap();
        let restored_cwd = header["payload"]["cwd"].as_str().unwrap();
        assert_eq!(restored_cwd, handoff.cwd);
        assert_eq!(
            Path::new(restored_cwd).canonicalize().unwrap(),
            target_project.canonicalize().unwrap()
        );
        let r = cycle_with_mappings(
            &b_root,
            &binding,
            &key,
            &remote,
            "codex",
            &b_home,
            Direction::Both,
            &mappings,
            || false,
        )
        .unwrap();
        assert_eq!(r.published, 0);
        assert!(!r.issues.contains_key("source_missing"));
        fs::OpenOptions::new()
            .append(true)
            .open(&file)
            .unwrap()
            .write_all(b"{\"type\":\"response_item\",\"payload\":{\"text\":\"B continuation\"}}\n")
            .unwrap();
        let remote_before = remote.0.borrow().len();
        let stops = Cell::new(0usize);
        assert_eq!(
            cycle_with_mappings(
                &b_root,
                &binding,
                &key,
                &remote,
                "codex",
                &b_home,
                Direction::Both,
                &mappings,
                || {
                    stops.set(stops.get() + 1);
                    stops.get() == 2
                },
            )
            .err()
            .unwrap(),
            "sync_paused"
        );
        assert_eq!(remote.0.borrow().len(), remote_before);
        let r = cycle_with_mappings(
            &b_root,
            &binding,
            &key,
            &remote,
            "codex",
            &b_home,
            Direction::Both,
            &mappings,
            || false,
        )
        .unwrap();
        assert_eq!(r.published, 1, "{:?}", r.issues);
        let replica = Replica::open(&b_root.join("replica"), "space").unwrap();
        let all = replica.transport_bundles().unwrap();
        let child = all.values().find(|b| {
            b.snapshot.parents == vec![handoff.base.clone()]
                && b.snapshot.files.contains_key("session.json")
        });
        // The in-memory entry predates publication; the persisted registry advances.
        assert!(child.is_some());
        let registry = load_handoffs(&b_root).unwrap();
        assert_eq!(registry.entries[0].base, child.unwrap().id);
        assert_eq!(
            fs::read(&file).unwrap().len(),
            before.len()
                + b"{\"type\":\"response_item\",\"payload\":{\"text\":\"B continuation\"}}\n".len()
        );
    }
    #[test]
    fn concurrent_restored_edits_keep_two_causal_heads() {
        let t = tempfile::tempdir().unwrap();
        let agent = "codex";
        let session = "019f0000-0000-7000-8000-000000000001";
        let a_home = t.path().join("a-home");
        let a_project = t.path().join("a-project");
        fs::create_dir_all(&a_project).unwrap();
        let original_cwd = a_project.to_string_lossy().into_owned();
        cross_fixture(agent, &a_home, &original_cwd, session, "base turn");
        let original_file = a_home.join(cross_relative(agent, &original_cwd, session));
        let original = fs::read(&original_file).unwrap();
        let binding = Binding {
            folder: "folder".into(),
            space: "space".into(),
            proof: "proof".into(),
        };
        let key = SpaceKey::generate().unwrap();
        let remote = Remote(
            RefCell::new(BTreeMap::from([(
                "proof".into(),
                queue::proof_bundle("space").unwrap(),
            )])),
            Cell::new(0),
        );
        let a_root = t.path().join("a-sync");
        cycle(
            &a_root,
            &binding,
            &key,
            &remote,
            agent,
            &a_home,
            Direction::Upload,
            || false,
        )
        .unwrap();
        let a_all = Replica::open(&a_root.join("replica"), "space")
            .unwrap()
            .transport_bundles()
            .unwrap();
        let base = cross_head(&a_all, agent).id.clone();
        for branch in ["b", "c"] {
            let root = t.path().join(format!("{branch}-sync"));
            let missing_home = t.path().join(format!("{branch}-home"));
            let project = t.path().join(format!("{branch}-project"));
            fs::create_dir_all(&project).unwrap();
            let mappings = [crate::project_mapping::Mapping {
                source: original_cwd.clone(),
                target: project.to_string_lossy().into_owned(),
            }];
            let result = cycle_with_mappings(
                &root,
                &binding,
                &key,
                &remote,
                agent,
                &missing_home,
                Direction::Download,
                &mappings,
                || false,
            )
            .unwrap();
            assert_eq!(result.restored, 1, "{branch}: {:?}", result.issues);
        }
        for (branch, marker) in [("b", "B offline turn"), ("c", "C offline turn")] {
            let root = t.path().join(format!("{branch}-sync"));
            let missing_home = t.path().join(format!("{branch}-home"));
            let project = t.path().join(format!("{branch}-project"));
            let mappings = [crate::project_mapping::Mapping {
                source: original_cwd.clone(),
                target: project.to_string_lossy().into_owned(),
            }];
            let handoff = load_handoffs(&root).unwrap();
            append_cross_turn(&handoff.entries[0], marker);
            cycle_with_mappings(
                &root,
                &binding,
                &key,
                &remote,
                agent,
                &missing_home,
                Direction::Upload,
                &mappings,
                || false,
            )
            .unwrap();
        }
        let mappings = ["b", "c"].map(|branch| crate::project_mapping::Mapping {
            source: fs::canonicalize(t.path().join(format!("{branch}-project")))
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            target: a_project.to_string_lossy().into_owned(),
        });
        let result = cycle_with_mappings(
            &a_root,
            &binding,
            &key,
            &remote,
            agent,
            &a_home,
            Direction::Download,
            &mappings,
            || false,
        )
        .unwrap();
        assert_eq!(result.restored, 2, "{:?}", result.issues);
        assert_eq!(fs::read(&original_file).unwrap(), original);
        let all = Replica::open(&a_root.join("replica"), "space")
            .unwrap()
            .transport_bundles()
            .unwrap();
        let heads = all
            .values()
            .filter(|b| {
                b.snapshot.parents == vec![base.clone()]
                    && b.snapshot.files.contains_key("session.json")
            })
            .collect::<Vec<_>>();
        assert_eq!(heads.len(), 2);
        assert_eq!(heads[0].snapshot.stream, heads[1].snapshot.stream);
        assert_eq!(load_handoffs(&a_root).unwrap().entries.len(), 2);
    }
    #[test]
    fn segmented_history_waits_for_every_part_and_reuses_unchanged_parts() {
        let t = tempfile::tempdir().unwrap();
        let home = t.path().join("source");
        fixture("pi", &home);
        let mut paths = vec![];
        walk(&home.join("sessions"), 4, &mut paths).unwrap();
        let m = capture_file("pi", &home, &paths[0], t.path()).unwrap();
        let replica = Replica::open(&t.path().join("replica"), "space").unwrap();
        let mut journal = Journal {
            node: "node".into(),
            ..Default::default()
        };
        let mut batch = replica.export_batch().unwrap();
        let id = publish(&m, &mut batch, &mut journal, 64).unwrap();
        drop(batch);
        let all = replica.transport_bundles().unwrap();
        assert!(all.len() > 2);
        let root = all[&id].clone();
        let parts: Parts =
            serde_json::from_str(&root.snapshot.files["session.parts.json"].content).unwrap();
        let mut incomplete = all.clone();
        incomplete.remove(&parts.ids[0]);
        let target = t.path().join("restored");
        assert_eq!(
            restore(&root, &incomplete, &target).unwrap_err(),
            "session_parts_pending"
        );
        assert!(!target.exists());
        restore(&root, &all, &target).unwrap();
        for (path, bytes) in &m.files {
            assert_eq!(
                fs::read(target.join(path)).unwrap(),
                STANDARD.decode(bytes).unwrap()
            );
        }
        let mut batch = replica.export_batch().unwrap();
        assert_eq!(publish(&m, &mut batch, &mut journal, 64).unwrap(), id);
        drop(batch);
        assert_eq!(replica.transport_bundles().unwrap().len(), all.len());
        let key = SpaceKey::generate().unwrap();
        for b in all.values() {
            assert_eq!(key.open("space", &key.seal(b).unwrap()).unwrap(), *b);
        }
        let stream = root.snapshot.stream.clone();
        let batch = replica.export_batch().unwrap();
        assert!(cached_native_baseline_complete(&batch, &stream, &id));
        drop(batch);
        fs::remove_file(
            t.path()
                .join("replica/objects")
                .join(format!("{}.json", parts.ids[0])),
        )
        .unwrap();
        let mut batch = replica.export_batch().unwrap();
        assert!(!cached_native_baseline_complete(&batch, &stream, &id));
        assert_eq!(
            publish(&m, &mut batch, &mut journal, 64).unwrap_err(),
            "unknown_baseline"
        );
    }
    #[test]
    fn changing_and_malformed_sessions_report_partial_without_losing_good_ones() {
        let t = tempfile::tempdir().unwrap();
        let source = t.path().join("source");
        fixture("pi", &source);
        write(&source, "sessions/project/bad.jsonl", b"{incomplete");
        let binding = Binding {
            folder: "f".into(),
            space: "s".into(),
            proof: "proof".into(),
        };
        let key = SpaceKey::generate().unwrap();
        let remote = Remote(
            RefCell::new(BTreeMap::from([(
                "proof".into(),
                queue::proof_bundle("s").unwrap(),
            )])),
            Cell::new(0),
        );
        let r = cycle(
            &t.path().join("sync"),
            &binding,
            &key,
            &remote,
            "pi",
            &source,
            Direction::Upload,
            || false,
        )
        .unwrap();
        assert_eq!(r.state, "partial");
        assert_eq!(r.published, 1);
        assert_eq!(r.issues.get("session_invalid"), Some(&1));
    }
    #[test]
    fn sqlite_snapshot_includes_committed_wal_without_modifying_source() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("session.db");
        let c = rusqlite::Connection::open(&p).unwrap();
        c.execute_batch("PRAGMA journal_mode=WAL;CREATE TABLE trajectory_meta(id TEXT);CREATE TABLE steps(idx INTEGER);INSERT INTO steps VALUES(42);").unwrap();
        let bytes = sqlite_snapshot(&p, t.path()).unwrap();
        let dest = t.path().join("restored.db");
        fs::write(&dest, bytes).unwrap();
        let copy = rusqlite::Connection::open(dest).unwrap();
        assert_eq!(
            copy.query_row("SELECT idx FROM steps", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            42
        );
        assert_eq!(
            c.query_row("SELECT count(*) FROM steps", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    #[test]
    fn divergent_active_file_is_never_replaced_and_credentials_are_rejected() {
        let t = tempfile::tempdir().unwrap();
        let a_home = t.path().join("a-home");
        let b_home = t.path().join("b-home");
        fixture("codex", &a_home);
        fixture("codex", &b_home);
        let file =
            b_home.join("sessions/2026/09/05/rollout-019f0000-0000-7000-8000-000000000001.jsonl");
        let changed = [
            fs::read(&file).unwrap(),
            b"{\"type\":\"response_item\",\"payload\":{\"text\":\"local divergent turn\"}}\n"
                .to_vec(),
        ]
        .concat();
        fs::write(&file, &changed).unwrap();
        let binding = Binding {
            folder: "folder".into(),
            space: "space".into(),
            proof: "proof".into(),
        };
        let key = SpaceKey::generate().unwrap();
        let remote = Remote(
            RefCell::new(BTreeMap::from([(
                "proof".into(),
                queue::proof_bundle("space").unwrap(),
            )])),
            Cell::new(0),
        );
        cycle(
            &t.path().join("a-sync"),
            &binding,
            &key,
            &remote,
            "codex",
            &a_home,
            Direction::Upload,
            || false,
        )
        .unwrap();
        let project = t.path().join("project");
        fs::create_dir_all(&project).unwrap();
        let mappings = [crate::project_mapping::Mapping {
            source: "/project".into(),
            target: project.to_string_lossy().into_owned(),
        }];
        let result = cycle_with_mappings(
            &t.path().join("b-sync"),
            &binding,
            &key,
            &remote,
            "codex",
            &b_home,
            Direction::Download,
            &mappings,
            || false,
        )
        .unwrap();
        assert_eq!(result.restored, 1, "{:?}", result.issues);
        assert_eq!(fs::read(&file).unwrap(), changed);
        assert_eq!(
            load_handoffs(&t.path().join("b-sync"))
                .unwrap()
                .entries
                .len(),
            1
        );
        let mut paths = vec![];
        walk(&a_home.join("sessions"), 4, &mut paths).unwrap();
        let m = capture_file("codex", &a_home, &paths[0], t.path()).unwrap();
        let mut bad = m;
        bad.files = BTreeMap::from([("auth.json".into(), STANDARD.encode("credential"))]);
        assert!(encode(&bad).is_err());
        assert!(restore_manifest(&bad, &t.path().join("bad-profile")).is_err());
    }
    #[test]
    fn traversal_and_symlinks_cannot_enter_restored_profiles() {
        for p in [
            "../escape",
            "/absolute",
            "a/../escape",
            "C:/x",
            "a\\x",
            "a/./x",
        ] {
            assert!(!safe_relative(p), "{p}");
        }
        #[cfg(unix)]
        {
            let t = tempfile::tempdir().unwrap();
            let p = write(t.path(), "secret", b"secret");
            std::os::unix::fs::symlink(p, t.path().join("link")).unwrap();
            assert!(stable(&t.path().join("link")).is_err());
        }
    }
    fn cross_relative(agent: &str, cwd: &str, session: &str) -> String {
        let normalized = cwd.replace('\\', "/");
        match agent {
            "codex" => format!("sessions/2026/10/01/rollout-{session}.jsonl"),
            "claude-code" => {
                let group = cwd
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                    .collect::<String>();
                format!("projects/{group}/{session}.jsonl")
            }
            "pi" => {
                let group = normalized.trim_start_matches('/').replace(['/', ':'], "-");
                format!("sessions/--{group}--/2026-{session}.jsonl")
            }
            "grok" => {
                let group = crate::project_mapping::grok_group_for_cwd(cwd);
                format!("sessions/{group}/{session}/updates.jsonl")
            }
            "agy" => format!("conversations/{session}.db"),
            _ => unreachable!(),
        }
    }
    fn cross_fixture(agent: &str, profile: &Path, cwd: &str, session: &str, marker: &str) {
        let relative = cross_relative(agent, cwd, session);
        let timestamp = "2026-10-01T00:00:00Z";
        match agent {
            "codex" => {
                let lines = [
                    serde_json::json!({"timestamp":timestamp,"type":"session_meta","payload":{"id":session,"timestamp":timestamp,"cwd":cwd,"originator":"codex_cli_rs","cli_version":"0.120.0","source":"cli","model_provider":"openai","git":null}}),
                    serde_json::json!({"timestamp":timestamp,"type":"event_msg","payload":{"type":"user_message","message":marker,"images":[],"local_images":[],"text_elements":[]}}),
                    serde_json::json!({"timestamp":timestamp,"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":marker}]}}),
                ];
                write(
                    profile,
                    &relative,
                    format!(
                        "{}\n",
                        lines
                            .iter()
                            .map(serde_json::Value::to_string)
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                    .as_bytes(),
                );
            }
            "pi" => {
                let lines = [
                    serde_json::json!({"type":"session","version":3,"id":session,"cwd":cwd,"timestamp":timestamp}),
                    serde_json::json!({"type":"message","id":"m1","parentId":null,"timestamp":timestamp,"message":{"role":"user","content":[{"type":"text","text":marker}],"timestamp":1790812800000_i64}}),
                ];
                write(
                    profile,
                    &relative,
                    format!(
                        "{}\n",
                        lines
                            .iter()
                            .map(serde_json::Value::to_string)
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                    .as_bytes(),
                );
            }
            "claude-code" => {
                let line = serde_json::json!({"type":"user","sessionId":session,"cwd":cwd,"uuid":"019f0000-0000-7000-8000-000000000100","parentUuid":null,"isSidechain":false,"timestamp":timestamp,"message":{"role":"user","content":marker}});
                write(profile, &relative, format!("{line}\n").as_bytes());
            }
            "grok" => {
                let update = serde_json::json!({"method":"session/update","params":{"sessionId":session,"update":{"sessionUpdate":"user_message_chunk","content":{"type":"text","text":marker}}}});
                write(profile, &relative, format!("{update}\n").as_bytes());
                write(
                    profile,
                    &relative.replace("updates.jsonl", "summary.json"),
                    serde_json::json!({"info":{"id":session,"cwd":cwd},"created_at":timestamp,"updated_at":timestamp,"last_active_at":timestamp,"num_messages":1,"num_chat_messages":1,"next_trace_turn":1,"chat_format_version":1,"session_kind":"headless","reasoning_effort":"high","sandbox_profile":"off","session_summary":"fixture","current_model_id":"grok-4.6"})
                        .to_string()
                        .as_bytes(),
                );
                write(profile, &relative.replace("updates.jsonl", "chat_history.jsonl"), format!("{{\"type\":\"user\",\"content\":[{{\"type\":\"text\",\"text\":\"{marker}\"}}]}}\n").as_bytes());
                if crate::project_mapping::grok_long_cwd(cwd) {
                    write(
                        profile,
                        &relative.replace(&format!("/{session}/updates.jsonl"), "/.cwd"),
                        cwd.as_bytes(),
                    );
                }
            }
            "agy" => {
                let file = write(profile, &relative, b"");
                let db = rusqlite::Connection::open(file).unwrap();
                db.execute_batch("CREATE TABLE trajectory_meta(trajectory_id TEXT PRIMARY KEY); CREATE TABLE steps(idx INTEGER PRIMARY KEY, data BLOB);").unwrap();
                db.execute("INSERT INTO trajectory_meta VALUES (?1)", [session])
                    .unwrap();
                db.execute("INSERT INTO steps VALUES (1, ?1)", [marker.as_bytes()])
                    .unwrap();
            }
            _ => unreachable!(),
        }
    }
    fn cross_key() -> SpaceKey {
        SpaceKey::recover("bas1_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").unwrap()
    }
    fn cross_remote(input: Option<&Path>, agent: &str, key: &SpaceKey) -> Remote {
        let mut objects =
            BTreeMap::from([("proof".into(), queue::proof_bundle("cross-os").unwrap())]);
        if let Some(input) = input {
            for entry in fs::read_dir(input.join("objects").join(agent)).unwrap() {
                let path = entry.unwrap().path();
                let bundle = key.open("cross-os", &fs::read(path).unwrap()).unwrap();
                objects.insert(bundle.id.clone(), bundle);
            }
        }
        Remote(RefCell::new(objects), Cell::new(0))
    }
    fn cross_output(
        output: &Path,
        agent: &str,
        key: &SpaceKey,
        all: &BTreeMap<String, bundle::Bundle>,
    ) {
        let folder = output.join("objects").join(agent);
        fs::create_dir_all(&folder).unwrap();
        for bundle in all.values() {
            storage::immutable(
                &folder.join(format!("{}.sealed", bundle.id)),
                &key.seal(bundle).unwrap(),
            )
            .unwrap();
        }
    }
    fn cross_head<'a>(
        all: &'a BTreeMap<String, bundle::Bundle>,
        agent: &str,
    ) -> &'a bundle::Bundle {
        let parents = all
            .values()
            .flat_map(|b| b.snapshot.parents.iter().cloned())
            .collect::<BTreeSet<_>>();
        all.values()
            .filter(|b| {
                b.snapshot.stream.agent == agent
                    && b.snapshot.files.contains_key("session.json")
                    && !parents.contains(&b.id)
            })
            .max_by_key(|b| b.snapshot.parents.len())
            .unwrap()
    }
    /// Opt-in recovery of a real, isolated Agy conversation database through
    /// the production SQLite snapshot, encrypted bundle, and restore paths.
    #[test]
    #[ignore = "requires isolated BASTET_AGY_RECOVERY_SOURCE and BASTET_AGY_RECOVERY_TARGET"]
    fn live_agy_native_recovery() {
        let source = PathBuf::from(std::env::var_os("BASTET_AGY_RECOVERY_SOURCE").unwrap());
        let target = PathBuf::from(std::env::var_os("BASTET_AGY_RECOVERY_TARGET").unwrap());
        let temp_roots = [
            std::env::temp_dir(),
            #[cfg(unix)]
            PathBuf::from("/tmp"),
        ];
        let source = fs::canonicalize(source).unwrap();
        let parent = fs::canonicalize(target.parent().unwrap()).unwrap();
        assert!(temp_roots.iter().any(|root| {
            let root = fs::canonicalize(root).unwrap();
            source.starts_with(&root) && parent.starts_with(&root)
        }));
        assert!(!target.exists());
        let working = tempfile::tempdir().unwrap();
        let key = cross_key();
        let binding = Binding {
            folder: "folder".into(),
            space: "cross-os".into(),
            proof: "proof".into(),
        };
        let remote = cross_remote(None, "agy", &key);
        let result = cycle(
            working.path(),
            &binding,
            &key,
            &remote,
            "agy",
            &source,
            Direction::Upload,
            || false,
        )
        .unwrap();
        assert_eq!(result.captured, 1, "{:?}", result.issues);
        let all = Replica::open(&working.path().join("replica"), "cross-os")
            .unwrap()
            .transport_bundles()
            .unwrap();
        let reopened = all
            .iter()
            .map(|(id, bundle)| {
                let encrypted = key.seal(bundle).unwrap();
                (id.clone(), key.open("cross-os", &encrypted).unwrap())
            })
            .collect::<BTreeMap<_, _>>();
        let head = cross_head(&reopened, "agy");
        let manifest = restore(head, &reopened, &target).unwrap();
        let database = target.join(format!("conversations/{}.db", manifest.session));
        let connection = rusqlite::Connection::open_with_flags(
            database,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        assert_eq!(
            connection
                .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        let steps: i64 = connection
            .query_row("SELECT COUNT(*) FROM steps", [], |row| row.get(0))
            .unwrap();
        assert!(steps > 0);
        println!(
            "agy recovery PASS session={} steps={} restored={} head={}",
            manifest.session,
            steps,
            target.display(),
            head.id
        );
    }
    #[test]
    #[ignore = "requires explicit BASTET_HANDOFF_MODE and temporary artifact directories"]
    fn cross_os_handoff_exchange() {
        let mode = std::env::var("BASTET_HANDOFF_MODE").expect("produce, continue or verify mode");
        let output = std::env::var_os("BASTET_HANDOFF_OUTPUT").map(PathBuf::from);
        let input = std::env::var_os("BASTET_HANDOFF_INPUT").map(PathBuf::from);
        let key = cross_key();
        let binding = Binding {
            folder: "folder".into(),
            space: "cross-os".into(),
            proof: "proof".into(),
        };
        let session = "019f0000-0000-7000-8000-000000000001";
        let agents = ["codex", "claude-code", "pi", "grok", "agy"];
        let marker_a = "BASTET_CROSS_OS_A";
        let marker_b = "BASTET_CROSS_OS_B";
        match mode.as_str() {
            "produce" => {
                let output = output.expect("BASTET_HANDOFF_OUTPUT");
                fs::create_dir_all(&output).unwrap();
                let temp = tempfile::tempdir().unwrap();
                let mut cases = vec![];
                for agent in agents {
                    let profile_relative = format!("profiles/{agent}");
                    let profile = output.join(&profile_relative);
                    let project = output.join("projects").join(agent);
                    fs::create_dir_all(&project).unwrap();
                    let cwd = project.to_string_lossy().into_owned();
                    cross_fixture(agent, &profile, &cwd, session, marker_a);
                    let remote = cross_remote(None, agent, &key);
                    let root = temp.path().join(format!("sync-{agent}"));
                    let result = cycle(
                        &root,
                        &binding,
                        &key,
                        &remote,
                        agent,
                        &profile,
                        Direction::Upload,
                        || false,
                    )
                    .unwrap();
                    assert_eq!(result.captured, 1, "{agent}: {:?}", result.issues);
                    let all = Replica::open(&root.join("replica"), "cross-os")
                        .unwrap()
                        .transport_bundles()
                        .unwrap();
                    cross_output(&output, agent, &key, &all);
                    cases.push(ExchangeCase {
                        agent: agent.into(),
                        profile: profile_relative,
                        session: session.into(),
                        marker: marker_a.into(),
                        cwd,
                        head: cross_head(&all, agent).id.clone(),
                    });
                }
                fs::write(
                    output.join("manifest.json"),
                    json(&ExchangeManifest { cases }).unwrap(),
                )
                .unwrap();
            }
            "continue" | "verify" => {
                let input = input.expect("BASTET_HANDOFF_INPUT");
                let old: ExchangeManifest =
                    serde_json::from_slice(&fs::read(input.join("manifest.json")).unwrap())
                        .unwrap();
                let temp = tempfile::tempdir().unwrap();
                let mut cases = vec![];
                for prior in old.cases {
                    let agent = prior.agent.as_str();
                    let remote = cross_remote(Some(&input), agent, &key);
                    let root = temp.path().join(format!("sync-{agent}"));
                    let home = temp.path().join(format!("active-{agent}"));
                    fs::create_dir_all(&home).unwrap();
                    let active = home.join("active-sentinel");
                    fs::write(&active, b"untouched active store").unwrap();
                    let project = if mode == "continue" {
                        output
                            .as_ref()
                            .expect("BASTET_HANDOFF_OUTPUT")
                            .join("projects")
                            .join(agent)
                    } else {
                        temp.path().join("projects").join(agent)
                    };
                    fs::create_dir_all(&project).unwrap();
                    let cwd = project.to_string_lossy().into_owned();
                    let mappings = [crate::project_mapping::Mapping {
                        source: prior.cwd.clone(),
                        target: cwd.clone(),
                    }];
                    let result = cycle_with_mappings(
                        &root,
                        &binding,
                        &key,
                        &remote,
                        agent,
                        &home,
                        Direction::Download,
                        &mappings,
                        || false,
                    )
                    .unwrap();
                    assert_eq!(fs::read(&active).unwrap(), b"untouched active store");
                    let all = Replica::open(&root.join("replica"), "cross-os")
                        .unwrap()
                        .transport_bundles()
                        .unwrap();
                    let head = cross_head(&all, agent).clone();
                    if mode == "verify" {
                        if agent != "agy" {
                            assert_eq!(head.id, prior.head);
                            assert_eq!(head.snapshot.parents.len(), 1);
                            assert!(all.contains_key(&head.snapshot.parents[0]));
                            assert_eq!(result.restored, 1, "{agent}: {:?}", result.issues);
                            let handoff = load_handoffs(&root).unwrap();
                            let m =
                                capture_handoff(&handoff.entries[0], &root, true, None).unwrap();
                            let content = m
                                .files
                                .values()
                                .flat_map(|v| STANDARD.decode(v).unwrap())
                                .collect::<Vec<_>>();
                            assert!(content
                                .windows(marker_a.len())
                                .any(|v| v == marker_a.as_bytes()));
                            assert!(content
                                .windows(marker_b.len())
                                .any(|v| v == marker_b.as_bytes()));
                        } else {
                            assert_eq!(result.restored, 0);
                        }
                        continue;
                    }
                    let output = output.as_ref().unwrap();
                    fs::create_dir_all(output.join("profiles")).unwrap();
                    let profile_relative = format!("profiles/{agent}");
                    let profile = output.join(&profile_relative);
                    if agent == "agy" {
                        let m = unpack(&head, &all).unwrap();
                        restore_manifest(&m, &profile).unwrap();
                        cross_output(output, agent, &key, &all);
                        cases.push(ExchangeCase {
                            agent: prior.agent,
                            profile: profile_relative,
                            session: prior.session,
                            marker: marker_a.into(),
                            cwd: prior.cwd,
                            head: prior.head,
                        });
                        continue;
                    }
                    assert_eq!(result.restored, 1, "{agent}: {:?}", result.issues);
                    let handoff = load_handoffs(&root).unwrap();
                    assert_eq!(handoff.entries.len(), 1);
                    let h = &handoff.entries[0];
                    append_cross_turn(h, marker_b);
                    cycle_with_mappings(
                        &root,
                        &binding,
                        &key,
                        &remote,
                        agent,
                        &home,
                        Direction::Upload,
                        &mappings,
                        || false,
                    )
                    .unwrap();
                    let after = Replica::open(&root.join("replica"), "cross-os")
                        .unwrap()
                        .transport_bundles()
                        .unwrap();
                    let child = cross_head(&after, agent);
                    assert_eq!(child.snapshot.parents, vec![prior.head.clone()]);
                    let edited = capture_handoff(
                        &load_handoffs(&root).unwrap().entries[0],
                        &root,
                        true,
                        None,
                    )
                    .unwrap();
                    restore_manifest(&edited, &profile).unwrap();
                    cross_output(output, agent, &key, &after);
                    cases.push(ExchangeCase {
                        agent: prior.agent,
                        profile: profile_relative,
                        session: prior.session,
                        marker: marker_b.into(),
                        cwd,
                        head: child.id.clone(),
                    });
                }
                if mode == "continue" {
                    let output = output.unwrap();
                    fs::write(
                        output.join("manifest.json"),
                        json(&ExchangeManifest { cases }).unwrap(),
                    )
                    .unwrap();
                }
            }
            _ => panic!("invalid BASTET_HANDOFF_MODE"),
        }
    }

    /// Opt-in proof that a model's real turn in an isolated restored profile can
    /// return to a fresh replica as a causal child of the transferred head.
    #[test]
    #[ignore = "requires BASTET_LIVE_RETURN_INPUT with isolated live-model fixtures"]
    fn live_model_return_exchange() {
        let input =
            PathBuf::from(std::env::var_os("BASTET_LIVE_RETURN_INPUT").expect("fixture root"))
                .canonicalize()
                .unwrap();
        let temp_roots = [
            std::env::temp_dir(),
            #[cfg(unix)]
            PathBuf::from("/tmp"),
        ];
        assert!(temp_roots
            .iter()
            .filter_map(|root| root.canonicalize().ok())
            .any(|root| input.starts_with(root)));
        let mut live_cases = vec![
            ("codex", "BASTET_LIVE_CODEX_OK", input.clone()),
            ("claude-code", "BASTET_LIVE_CLAUDE_OK", input.clone()),
            ("pi", "BASTET_LIVE_PI_OK", input.clone()),
        ];
        if let Some(grok) = std::env::var_os("BASTET_LIVE_RETURN_GROK_INPUT") {
            live_cases.push((
                "grok",
                "BASTET_LIVE_GROK_OK",
                PathBuf::from(grok).canonicalize().unwrap(),
            ));
        }
        let key = cross_key();
        let binding = Binding {
            folder: "folder".into(),
            space: "cross-os".into(),
            proof: "proof".into(),
        };
        let temp = tempfile::tempdir().unwrap();
        for (agent, live_marker, input) in live_cases {
            assert!(temp_roots
                .iter()
                .filter_map(|root| root.canonicalize().ok())
                .any(|root| input.starts_with(root)));
            let manifest: ExchangeManifest =
                serde_json::from_slice(&fs::read(input.join("manifest.json")).unwrap()).unwrap();
            let prior = manifest.cases.iter().find(|c| c.agent == agent).unwrap();
            assert_eq!(prior.marker, "BASTET_CROSS_OS_B");
            let profile = input.join(&prior.profile).canonicalize().unwrap();
            assert!(profile.starts_with(&input));
            assert!(Path::new(&prior.cwd)
                .canonicalize()
                .unwrap()
                .starts_with(&input));
            let remote = cross_remote(Some(&input), agent, &key);
            let b_root = temp.path().join(format!("b-sync-{agent}"));
            let b_home = temp.path().join(format!("b-home-{agent}"));
            let restored = cycle_with_mappings(
                &b_root,
                &binding,
                &key,
                &remote,
                agent,
                &b_home,
                Direction::Download,
                &[],
                || false,
            )
            .unwrap();
            assert_eq!(restored.restored, 1, "{agent}: {:?}", restored.issues);
            let registry = load_handoffs(&b_root).unwrap();
            assert_eq!(registry.entries.len(), 1);
            let handoff = &registry.entries[0];
            assert_eq!(handoff.base, prior.head);
            let before = capture_handoff(handoff, &b_root, true, None).unwrap();
            let baseline = manifest_content_fingerprint(&before).unwrap();
            assert_eq!(before.session, prior.session);
            for relative in before.files.keys() {
                assert!(allowed(agent, relative));
                safe_profile_file(&profile, relative).unwrap();
                safe_profile_file(&handoff.path, relative).unwrap();
                fs::copy(profile.join(relative), handoff.path.join(relative)).unwrap();
            }
            let edited = capture_handoff(handoff, &b_root, true, None).unwrap();
            assert_ne!(manifest_content_fingerprint(&edited).unwrap(), baseline);
            assert_eq!(edited.session, prior.session);
            let live = STANDARD
                .decode(edited.files.get(&handoff.main_file).unwrap())
                .unwrap();
            for marker in ["BASTET_CROSS_OS_A", "BASTET_CROSS_OS_B", live_marker] {
                assert!(
                    live.windows(marker.len()).any(|w| w == marker.as_bytes()),
                    "{agent}: missing {marker}"
                );
            }
            assert!(
                live_turn_has_tool_result(agent, &live),
                "{agent}: missing tool result"
            );
            if agent == "grok" {
                let history = handoff
                    .main_file
                    .replace("updates.jsonl", "chat_history.jsonl");
                let bytes = STANDARD
                    .decode(edited.files.get(&history).unwrap())
                    .unwrap();
                assert!(std::str::from_utf8(&bytes).unwrap().lines().any(|line| {
                    serde_json::from_str::<serde_json::Value>(line)
                        .is_ok_and(|record| record["type"] == "tool_result")
                }));
            }

            let published = cycle_with_mappings(
                &b_root,
                &binding,
                &key,
                &remote,
                agent,
                &b_home,
                Direction::Upload,
                &[],
                || false,
            )
            .unwrap();
            assert_eq!(published.published, 1, "{agent}: {:?}", published.issues);
            let b_all = Replica::open(&b_root.join("replica"), "cross-os")
                .unwrap()
                .transport_bundles()
                .unwrap();
            let child = cross_head(&b_all, agent);
            assert_eq!(child.snapshot.parents, vec![prior.head.clone()]);

            let a_root = temp.path().join(format!("a-sync-{agent}"));
            let a_home = temp.path().join(format!("a-home-{agent}"));
            let a_project = temp.path().join(format!("a-project-{agent}"));
            fs::create_dir_all(&a_project).unwrap();
            let a_mappings = [crate::project_mapping::Mapping {
                source: prior.cwd.clone(),
                target: a_project.to_string_lossy().into_owned(),
            }];
            let received = cycle_with_mappings(
                &a_root,
                &binding,
                &key,
                &remote,
                agent,
                &a_home,
                Direction::Download,
                &a_mappings,
                || false,
            )
            .unwrap();
            assert_eq!(received.restored, 1, "{agent}: {:?}", received.issues);
            let a_all = Replica::open(&a_root.join("replica"), "cross-os")
                .unwrap()
                .transport_bundles()
                .unwrap();
            assert_eq!(cross_head(&a_all, agent).id, child.id);
            let a_registry = load_handoffs(&a_root).unwrap();
            let returned = capture_handoff(&a_registry.entries[0], &a_root, true, None).unwrap();
            assert_eq!(returned.session, prior.session);
            let returned_bytes = STANDARD
                .decode(
                    returned
                        .files
                        .get(&a_registry.entries[0].main_file)
                        .unwrap(),
                )
                .unwrap();
            for marker in ["BASTET_CROSS_OS_A", "BASTET_CROSS_OS_B", live_marker] {
                assert!(
                    returned_bytes
                        .windows(marker.len())
                        .any(|w| w == marker.as_bytes()),
                    "{agent}: missing return {marker}"
                );
            }
            assert!(live_turn_has_tool_result(agent, &returned_bytes));
            if agent == "grok" {
                let history = a_registry.entries[0]
                    .main_file
                    .replace("updates.jsonl", "chat_history.jsonl");
                let bytes = STANDARD
                    .decode(returned.files.get(&history).unwrap())
                    .unwrap();
                assert!(std::str::from_utf8(&bytes).unwrap().lines().any(|line| {
                    serde_json::from_str::<serde_json::Value>(line)
                        .is_ok_and(|record| record["type"] == "tool_result")
                }));
            }
            println!(
                "LIVE_RETURN_PASS {agent} session={} parent={} child={}",
                prior.session, prior.head, child.id
            );
        }
    }

    fn live_turn_has_tool_result(agent: &str, bytes: &[u8]) -> bool {
        std::str::from_utf8(bytes).unwrap().lines().any(|line| {
            let Ok(record) = serde_json::from_str::<serde_json::Value>(line) else {
                return false;
            };
            match agent {
                "codex" => {
                    record["type"] == "response_item"
                        && record["payload"]["type"] == "custom_tool_call_output"
                }
                "claude-code" => {
                    record["type"] == "user"
                        && record["message"]["content"]
                            .as_array()
                            .is_some_and(|parts| {
                                parts.iter().any(|part| part["type"] == "tool_result")
                            })
                }
                "pi" => record["type"] == "message" && record["message"]["role"] == "toolResult",
                "grok" => {
                    record["method"] == "session/update"
                        && record["params"]["update"]["sessionUpdate"] == "tool_call_update"
                        && !record["params"]["update"]["rawOutput"].is_null()
                }
                _ => false,
            }
        })
    }
    fn append_cross_turn(h: &Handoff, marker: &str) {
        let file = h.path.join(&h.main_file);
        if h.agent == "codex" {
            writeln!(
                fs::OpenOptions::new().append(true).open(&file).unwrap(),
                "{}",
                serde_json::json!({"timestamp":"2026-10-01T00:01:00Z","type":"event_msg","payload":{"type":"user_message","message":marker,"images":[],"local_images":[],"text_elements":[]}})
            ).unwrap();
        }
        let line = match h.agent.as_str() {
            "codex" => {
                serde_json::json!({"timestamp":"2026-10-01T00:01:00Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":marker}]}})
            }
            "pi" => {
                serde_json::json!({"type":"message","id":"m2","parentId":"m1","timestamp":"2026-10-01T00:01:00Z","message":{"role":"user","content":[{"type":"text","text":marker}],"timestamp":1790812860000_i64}})
            }
            "claude-code" => {
                serde_json::json!({"type":"user","sessionId":h.session,"cwd":h.cwd,"uuid":"019f0000-0000-7000-8000-000000000101","parentUuid":"019f0000-0000-7000-8000-000000000100","isSidechain":false,"timestamp":"2026-10-01T00:01:00Z","message":{"role":"user","content":marker}})
            }
            "grok" => {
                serde_json::json!({"method":"session/update","params":{"sessionId":h.session,"update":{"sessionUpdate":"user_message_chunk","content":{"type":"text","text":marker}}}})
            }
            _ => unreachable!(),
        };
        writeln!(
            fs::OpenOptions::new().append(true).open(&file).unwrap(),
            "{line}"
        )
        .unwrap();
        if h.agent == "grok" {
            let history = file.with_file_name("chat_history.jsonl");
            writeln!(
                fs::OpenOptions::new().append(true).open(history).unwrap(),
                "{}",
                serde_json::json!({"type":"user","content":[{"type":"text","text":marker}]})
            )
            .unwrap();
            let summary = file.with_file_name("summary.json");
            let mut value: serde_json::Value =
                serde_json::from_slice(&fs::read(&summary).unwrap()).unwrap();
            value["updated_at"] = "2026-10-01T00:01:00Z".into();
            value["last_active_at"] = "2026-10-01T00:01:00Z".into();
            value["num_messages"] = 2.into();
            value["num_chat_messages"] = 2.into();
            fs::write(summary, value.to_string()).unwrap();
        }
    }
}
