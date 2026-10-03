//! Allowlisted session snapshots. Receiving never writes into a live agent profile.
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
fn capture_handoff(h: &Handoff, staging: &Path) -> Result<Manifest> {
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
    } else {
        capture_file(&h.agent, &h.path, &file, staging)?
    };
    if m.session != h.session || m.agent != h.agent {
        return Err("session_conflict".into());
    }
    if m.cwd != h.cwd {
        return Err("project_mapping_required".into());
    }
    for relative in m.files.keys() {
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
            let manifest = capture_handoff(h, root)?;
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
            save_handoffs(root, &registry)?;
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
) -> Result<usize> {
    if bundle.snapshot.stream.agent == "agy" {
        // A copied database is recoverable data, not a verified standalone CLI profile.
        return Ok(0);
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
    let registry = load_handoffs(root)?;
    if let Some(h) = registry.entries.iter().find(|h| h.base == bundle.id) {
        capture_handoff(h, root)?;
        return Ok(0);
    }
    if let Some(h) = registry.entries.iter().find(|h| h.path == destination) {
        capture_handoff(h, root)?;
        return Ok(0);
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
    register_handoff(root, bundle, &manifest, &destination)?;
    Ok(1)
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
#[allow(clippy::too_many_arguments)]
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
        publish_handoffs(root, agent, &mut batch, &mut status, &stop)?;
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
                auto_prepare(root, b, &m, mappings)
            })();
            match result {
                Ok(n) => status.restored += n,
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
                                    .is_ok_and(|_scope| capture_handoff(h, &p).is_ok())
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
        let target = folder.join(format!("Bastet-{agent}-{}", uuid::Uuid::new_v4()));
        restore_manifest(&manifest, &target)?;
        if agent != "agy" {
            register_handoff(
                &root.join(format!("sessions-{agent}-{space}")),
                b,
                &manifest,
                &target,
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
    if manifest.agent != canonical || manifest.files.len() > 256 {
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
        };
        healthy.fingerprint =
            manifest_fingerprint(&capture_handoff(&healthy, &root).unwrap()).unwrap();
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
        register_handoff(&b, snapshot, &manifest, &target).unwrap();
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
        register_handoff(&b_root, &base, &base_manifest, &b_profile).unwrap();
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
        register_handoff(&a_root, a_b_head, &b_manifest, &a_profile).unwrap();
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
                            let m = capture_handoff(&handoff.entries[0], &root).unwrap();
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
                    let edited =
                        capture_handoff(&load_handoffs(&root).unwrap().entries[0], &root).unwrap();
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
            let before = capture_handoff(handoff, &b_root).unwrap();
            let baseline = manifest_content_fingerprint(&before).unwrap();
            assert_eq!(before.session, prior.session);
            for relative in before.files.keys() {
                assert!(allowed(agent, relative));
                safe_profile_file(&profile, relative).unwrap();
                safe_profile_file(&handoff.path, relative).unwrap();
                fs::copy(profile.join(relative), handoff.path.join(relative)).unwrap();
            }
            let edited = capture_handoff(handoff, &b_root).unwrap();
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
            let returned = capture_handoff(&a_registry.entries[0], &a_root).unwrap();
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
