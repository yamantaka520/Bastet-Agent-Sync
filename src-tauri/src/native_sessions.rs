//! Allowlisted session snapshots. Receiving never writes into a live agent profile.
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
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};
use tauri::Manager;
const MAX_RAW: u64 = 512 * 1024 * 1024;
const MAX_PACKED: u64 = 384 * 1024 * 1024;
const PART_SIZE: usize = 23 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Manifest {
    pub version: u32,
    pub agent: String,
    pub session: String,
    pub cwd: String,
    pub files: BTreeMap<String, String>,
}
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceStatus {
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
}
fn json<T: Serialize>(v: &T) -> Result<String> {
    serde_json::to_string(v).map_err(|_| "session_invalid".into())
}
fn safe_relative(value: &str) -> bool {
    !value.is_empty()
        && value.len() < 2048
        && !value.contains(['\\', ':', '\0'])
        && Path::new(value)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        && value.split('/').all(|p| {
            !p.is_empty()
                && p != "."
                && p != ".."
                && !p.ends_with([' ', '.'])
                && ![
                    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6",
                    "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7",
                    "LPT8", "LPT9",
                ]
                .contains(
                    &p.split('.')
                        .next()
                        .unwrap_or("")
                        .to_ascii_uppercase()
                        .as_str(),
                )
        })
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
            parts.len() == 4
                && parts[0] == "sessions"
                && bundle::token(parts[2])
                && [
                    "summary.json",
                    "updates.jsonl",
                    "chat_history.jsonl",
                    "plan.json",
                    "signals.json",
                    "rewind_points.jsonl",
                ]
                .contains(&parts[3])
        }
        _ => false,
    }
}
fn ensure_directory(path: &Path) -> Result<()> {
    if path.is_dir() {
        return storage::directory(path);
    }
    if let Some(parent) = path.parent() {
        if parent != path {
            ensure_directory(parent)?;
        }
    }
    storage::directory(path)
}
/// Add missing sessions only. Different existing bytes are a conflict, never a replacement.
fn install_missing(m: &Manifest, home: &Path) -> Result<usize> {
    // The same session may already have moved to an archive or a mapped project.
    // Do not introduce a second native file carrying an active session's identity.
    let dirs = match m.agent.as_str() {
        "claude" | "claude-code" => vec!["projects"],
        "agy" => vec!["conversations"],
        "codex" | "chatgpt-work" => vec!["sessions", "archived_sessions"],
        _ => vec!["sessions"],
    };
    for dir in dirs {
        let root = home.join(dir);
        if !root.is_dir() {
            continue;
        }
        let mut existing = vec![];
        walk(&root, 8, &mut existing)?;
        for p in existing {
            let same_id = p
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.contains(&m.session))
                || (m.agent == "grok"
                    && p.components().any(|c| c.as_os_str() == m.session.as_str()));
            if same_id {
                let relative = p
                    .strip_prefix(home)
                    .map_err(|_| "unsafe_store")?
                    .to_str()
                    .ok_or("unsafe_store")?
                    .replace('\\', "/");
                if !m.files.contains_key(&relative) {
                    return Err("session_conflict".into());
                }
            }
        }
    }
    let mut files = Vec::new();
    for (relative, text) in &m.files {
        if !allowed(&m.agent, relative) {
            return Err("session_invalid".into());
        }
        let path = home.join(relative);
        let mut ancestor = path.parent();
        while let Some(p) = ancestor {
            if let Ok(meta) = fs::symlink_metadata(p) {
                if meta.file_type().is_symlink() || !meta.is_dir() {
                    return Err("unsafe_store".into());
                }
            }
            if p == home {
                break;
            }
            ancestor = p.parent();
        }
        let bytes = STANDARD.decode(text).map_err(|_| "session_invalid")?;
        if path.exists() {
            if storage::read(&path, MAX_RAW)? != bytes {
                return Err("session_conflict".into());
            }
        } else {
            files.push((path, bytes));
        }
    }
    for (path, bytes) in &files {
        ensure_directory(path.parent().ok_or("unsafe_store")?)?;
        storage::immutable(path, bytes)?;
    }
    Ok(usize::from(!files.is_empty()))
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
fn encode(manifest: &Manifest) -> Result<String> {
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
        || m.files.keys().any(|p| !allowed(&m.agent, p))
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
                cwd = v["cwd"].as_str().unwrap_or("").into();
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
    Ok(Manifest {
        version: 1,
        agent: "grok".into(),
        session,
        cwd,
        files,
    })
}
// Explicit dependencies keep the native cycle fixture-testable without a Tauri runtime.
#[allow(clippy::too_many_arguments)]
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
    storage::directory(root)?;
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
    if !matches!(direction, Direction::Download) {
        let mut batch = replica.export_batch()?;
        if !source.is_dir() {
            status.issues.insert("source_missing".into(), 1);
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
            for p in paths {
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
                    publish(&m, &mut batch, &mut journal, PART_SIZE)?;
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
    let all = replica.transport_bundles()?;
    let parents: std::collections::BTreeSet<_> = all
        .values()
        .flat_map(|b| b.snapshot.parents.iter().cloned())
        .collect();
    if !matches!(direction, Direction::Upload) {
        for b in all.values().filter(|b| {
            b.snapshot.stream.agent == agent
                && b.snapshot.stream.profile != journal.node
                && !parents.contains(&b.id)
                && b.snapshot.files.contains_key("session.json")
        }) {
            if stop() {
                return Err("sync_paused".into());
            }
            status.available += 1;
            let result = (|| {
                let m = unpack(b, &all)?;
                if m.agent != agent
                    || bundle::hash(m.session.as_bytes()) != b.snapshot.stream.conversation
                {
                    return Err("session_invalid".into());
                }
                install_missing(&m, source)
            })();
            match result {
                Ok(n) => status.restored += n,
                Err(e) => {
                    *status.issues.entry(e).or_default() += 1;
                }
            }
        }
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
}
fn native_root(app: &tauri::AppHandle) -> Result<(PathBuf, String)> {
    let root = app
        .path()
        .app_config_dir()
        .map_err(|_| "store_unavailable")?;
    let tx = crate::cloud::wizard::Transaction::open(&root)?;
    Ok((
        root,
        tx.state
            .binding
            .as_ref()
            .ok_or("wizard_step_required")?
            .space
            .clone(),
    ))
}
#[tauri::command]
pub async fn list_received_sessions(app: tauri::AppHandle) -> Result<Vec<Received>> {
    tauri::async_runtime::spawn_blocking(move || {
        let (root, space) = native_root(&app)?;
        let mut entries = vec![];
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
                    entries.push(Received {
                        id: b.id.clone(),
                        agent: agent.to_string(),
                        session: v["session"].as_str().unwrap_or("").into(),
                        cwd: v["cwd"].as_str().unwrap_or("").into(),
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
    // Unique child name and atomic rename keep partially restored profiles invisible.
    fs::rename(stage.path(), destination).map_err(|_| "restore_destination_exists")?;
    Ok(m)
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
    if !crate::model::AGENTS.contains(&agent.as_str()) || !bundle::is_hash(&id) {
        return Err("session_invalid".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let Some(folder) = rfd::FileDialog::new().pick_folder() else {
            return Ok(None);
        };
        let (root, space) = native_root(&app)?;
        let replica = Replica::open(
            &root
                .join(format!("sessions-{agent}-{space}"))
                .join("replica"),
            &space,
        )?;
        let all = replica.transport_bundles()?;
        let b = all.get(&id).ok_or("session_invalid")?;
        let target = folder.join(format!("Bastet-{agent}-{}", uuid::Uuid::new_v4()));
        restore(b, &all, &target)?;
        Ok(Some(target.to_string_lossy().into()))
    })
    .await
    .map_err(|_| "store_unavailable".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
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
                write(root,&format!("projects/project/{id}.jsonl"),format!("{{\"type\":\"user\",\"sessionId\":\"{id}\",\"cwd\":\"/project\",\"message\":{{\"role\":\"user\",\"content\":\"fixture\"}}}}\n").as_bytes());
            }
            "pi" => {
                write(root,&format!("sessions/project/2026-{id}.jsonl"),format!("{{\"type\":\"session\",\"version\":3,\"id\":\"{id}\",\"cwd\":\"/project\"}}\n").as_bytes());
            }
            "grok" => {
                write(
                    root,
                    &format!("sessions/project/{id}/updates.jsonl"),
                    b"{\"sessionUpdate\":\"user_message_chunk\"}\n",
                );
                write(
                    root,
                    &format!("sessions/project/{id}/summary.json"),
                    b"{\"cwd\":\"/project\"}",
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
            assert_eq!(r.restored, 1);
            assert_eq!(r.available, 1);
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
    }
    #[test]
    fn changing_and_malformed_sessions_report_partial_without_losing_good_ones() {
        let t = tempfile::tempdir().unwrap();
        fixture("pi", t.path());
        write(t.path(), "sessions/project/bad.jsonl", b"{incomplete");
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
            t.path(),
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
    fn conflicting_native_file_is_never_replaced_and_credentials_are_rejected() {
        let t = tempfile::tempdir().unwrap();
        fixture("pi", t.path());
        let mut paths = vec![];
        walk(&t.path().join("sessions"), 4, &mut paths).unwrap();
        let m = capture_file("pi", t.path(), &paths[0], t.path()).unwrap();
        assert_eq!(install_missing(&m, t.path()).unwrap(), 0);
        fs::write(&paths[0], b"active edits").unwrap();
        assert_eq!(
            install_missing(&m, t.path()).unwrap_err(),
            "session_conflict"
        );
        assert_eq!(fs::read(&paths[0]).unwrap(), b"active edits");
        let mut bad = m;
        bad.files = BTreeMap::from([("auth.json".into(), STANDARD.encode("credential"))]);
        assert!(decode(&encode(&bad).unwrap()).is_err());
        assert!(install_missing(&bad, t.path()).is_err());
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
}
