//! Immutable text snapshots in an isolated inbox. This module never writes agent profiles.
pub mod bundle;
pub mod diagnostic;
mod storage;
#[cfg(test)]
mod tests;
use bundle::{Bundle, Entry, Result, Snapshot, Stream, MAX_OBJECTS, MAX_WIRE};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    schema: u32,
    space: String,
    device: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Space {
    schema: u32,
    id: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Journal {
    pub schema: u32,
    pub objects: Vec<String>,
    pub pending: Vec<String>,
    pub streams: Vec<Heads>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Heads {
    pub stream: Stream,
    pub ids: Vec<String>,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Issue {
    pub object: String,
    pub code: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub published: usize,
    pub received: usize,
    pub unchanged: usize,
    pub pending: usize,
    pub conflicts: usize,
    pub issues: Vec<Issue>,
}
#[derive(Clone, Copy)]
pub enum Direction {
    Both,
    Upload,
    Download,
}

pub struct LocalTransport {
    root: PathBuf,
    pub space: String,
}
impl LocalTransport {
    pub fn create(root: &Path) -> Result<Self> {
        storage::directory(root)?;
        let marker = root.join("space.json");
        if !marker.exists() {
            let space = Space {
                schema: 1,
                id: uuid::Uuid::new_v4().to_string(),
            };
            // Concurrent local initialization adopts the winner; never overwrite its identity.
            match storage::immutable(
                &marker,
                &serde_json::to_vec(&space).map_err(|_| "invalid_bundle")?,
            ) {
                Ok(_) => {}
                Err(e) if e == "immutable_collision" => {}
                Err(e) => return Err(e),
            }
        }
        storage::directory(&root.join("objects"))?;
        Self::connect(root)
    }
    pub fn connect(root: &Path) -> Result<Self> {
        let space: Space = serde_json::from_slice(&storage::read(&root.join("space.json"), 4096)?)
            .map_err(|_| "invalid_space")?;
        if space.schema != 1 || !bundle::token(&space.id) {
            return Err("invalid_space".into());
        }
        let m = fs::symlink_metadata(root.join("objects")).map_err(|_| "store_unavailable")?;
        if !m.is_dir() || m.file_type().is_symlink() {
            return Err("unsafe_store".into());
        }
        Ok(Self {
            root: root.canonicalize().map_err(|_| "store_unavailable")?,
            space: space.id,
        })
    }
    fn objects(&self) -> PathBuf {
        self.root.join("objects")
    }
    fn verify(&self) -> Result<()> {
        // A disconnected mount or folder replacement must not silently create a new space.
        let s: Space = serde_json::from_slice(&storage::read(&self.root.join("space.json"), 4096)?)
            .map_err(|_| "invalid_space")?;
        if s.schema != 1 || s.id != self.space {
            return Err("space_mismatch".into());
        }
        let m = fs::symlink_metadata(self.objects()).map_err(|_| "store_unavailable")?;
        if !m.is_dir() || m.file_type().is_symlink() {
            return Err("unsafe_store".into());
        }
        Ok(())
    }
}

pub struct Replica {
    root: PathBuf,
    identity: Identity,
    _lock: File,
}
impl Replica {
    pub fn open(root: &Path, space: &str) -> Result<Self> {
        if !bundle::token(space) {
            return Err("invalid_space".into());
        }
        storage::directory(root)?;
        let lock = storage::lock(&root.join("replica.lock"))?;
        storage::directory(&root.join("objects"))?;
        let path = root.join("identity.json");
        if !path.exists() {
            let id = Identity {
                schema: 1,
                space: space.into(),
                device: uuid::Uuid::new_v4().to_string(),
            };
            storage::immutable(
                &path,
                &serde_json::to_vec(&id).map_err(|_| "invalid_space")?,
            )?;
        }
        let identity: Identity =
            serde_json::from_slice(&storage::read(&path, 4096)?).map_err(|_| "invalid_space")?;
        if identity.schema != 1 || identity.space != space || !bundle::token(&identity.device) {
            return Err("space_mismatch".into());
        }
        let r = Self {
            root: root.canonicalize().map_err(|_| "store_unavailable")?,
            identity,
            _lock: lock,
        };
        r.checkpoint()?;
        Ok(r)
    }
    fn objects(&self) -> PathBuf {
        self.root.join("objects")
    }
    fn read_all(&self) -> Result<(BTreeMap<String, Bundle>, Vec<Issue>)> {
        collect(&self.objects(), &self.identity.space)
    }
    pub fn checkpoint(&self) -> Result<Journal> {
        let (objects, _) = self.read_all()?;
        let (journal, _) = graph(&objects);
        // Immutable objects are authoritative. Rebuild the checkpoint after an interrupted write.
        storage::replace(
            &self.root.join("journal.json"),
            &serde_json::to_vec_pretty(&journal).map_err(|_| "invalid_bundle")?,
        )?;
        Ok(journal)
    }
    pub fn export(&self, stream: Stream, files: BTreeMap<String, String>) -> Result<String> {
        let entries = files
            .into_iter()
            .map(|(p, c)| (p, Entry::new(c)))
            .collect::<BTreeMap<_, _>>();
        let (objects, issues) = self.read_all()?;
        if !issues.is_empty() {
            return Err("local_store_damaged".into());
        }
        let (journal, invalid) = graph(&objects);
        if !invalid.is_empty() || !journal.pending.is_empty() {
            return Err("history_pending".into());
        }
        let heads = journal
            .streams
            .iter()
            .find(|s| s.stream == stream)
            .map(|s| s.ids.clone())
            .unwrap_or_default();
        if heads.len() > 1 {
            return Err("conflict_requires_resolution".into());
        }
        if let Some(head) = heads.first() {
            if objects[head].snapshot.files == entries {
                return Ok(head.clone());
            }
        }
        self.commit(stream, entries, heads)
    }
    pub fn resolve(
        &self,
        stream: Stream,
        files: BTreeMap<String, String>,
        mut expected_heads: Vec<String>,
    ) -> Result<String> {
        let (objects, issues) = self.read_all()?;
        let (journal, invalid) = graph(&objects);
        if !issues.is_empty() || !invalid.is_empty() || !journal.pending.is_empty() {
            return Err("history_pending".into());
        }
        expected_heads.sort();
        let actual = journal
            .streams
            .iter()
            .find(|s| s.stream == stream)
            .map(|s| s.ids.clone())
            .unwrap_or_default();
        if actual.len() < 2 || actual != expected_heads {
            return Err("stale_resolution".into());
        }
        self.commit(
            stream,
            files.into_iter().map(|(p, c)| (p, Entry::new(c))).collect(),
            actual,
        )
    }
    fn commit(
        &self,
        stream: Stream,
        files: BTreeMap<String, Entry>,
        parents: Vec<String>,
    ) -> Result<String> {
        let b = Bundle::new(Snapshot {
            schema: 1,
            space: self.identity.space.clone(),
            device: self.identity.device.clone(),
            stream,
            parents,
            files,
        })?;
        let (mut all, _) = self.read_all()?;
        all.insert(b.id.clone(), b.clone());
        capacity(&all)?;
        storage::immutable(&self.objects().join(format!("{}.json", b.id)), &b.bytes()?)?;
        self.checkpoint()?;
        Ok(b.id)
    }
    pub fn sync(&self, remote: &LocalTransport, direction: Direction) -> Result<Report> {
        remote.verify()?;
        if remote.space != self.identity.space {
            return Err("space_mismatch".into());
        }
        if remote.root.starts_with(&self.root) || self.root.starts_with(&remote.root) {
            return Err("unsafe_store".into());
        }
        let (local, mut issues) = self.read_all()?;
        let (other, remote_issues) = collect(&remote.objects(), &remote.space)?;
        issues.extend(remote_issues);
        let union: BTreeMap<String, Bundle> = local
            .iter()
            .chain(other.iter())
            .map(|(id, b)| (id.clone(), b.clone()))
            .collect();
        capacity(&union)?;
        let mut report = Report {
            published: 0,
            received: 0,
            unchanged: 0,
            pending: 0,
            conflicts: 0,
            issues: vec![],
        };
        let (local_graph, _) = graph(&local);
        if !matches!(direction, Direction::Download) {
            for (id, b) in &local {
                if local_graph.pending.contains(id) {
                    continue;
                }
                if other.contains_key(id) {
                    report.unchanged += 1;
                    continue;
                }
                if other.len() + report.published >= MAX_OBJECTS {
                    return Err("bundle_limit".into());
                }
                match storage::immutable(&remote.objects().join(format!("{id}.json")), &b.bytes()?)
                {
                    Ok(true) => report.published += 1,
                    Ok(false) => report.unchanged += 1,
                    Err(code) => issues.push(Issue {
                        object: id.clone(),
                        code,
                    }),
                }
            }
        }
        if !matches!(direction, Direction::Upload) {
            for (id, b) in &other {
                if local.contains_key(id) {
                    continue;
                }
                if local.len() + report.received >= MAX_OBJECTS {
                    return Err("bundle_limit".into());
                }
                match storage::immutable(&self.objects().join(format!("{id}.json")), &b.bytes()?) {
                    Ok(true) => report.received += 1,
                    Ok(false) => {}
                    Err(code) => issues.push(Issue {
                        object: id.clone(),
                        code,
                    }),
                }
            }
        }
        let (all, _) = self.read_all()?;
        let (journal, invalid) = graph(&all);
        issues.extend(invalid);
        report.pending = journal.pending.len();
        report.conflicts = journal.streams.iter().filter(|s| s.ids.len() > 1).count();
        report.issues = issues;
        self.checkpoint()?;
        Ok(report)
    }
}

fn collect(root: &Path, space: &str) -> Result<(BTreeMap<String, Bundle>, Vec<Issue>)> {
    let m = fs::symlink_metadata(root).map_err(|_| "store_unavailable")?;
    if !m.is_dir() || m.file_type().is_symlink() {
        return Err("unsafe_store".into());
    }
    let mut objects = BTreeMap::new();
    let mut issues = vec![];
    let mut count = 0;
    let mut total_bytes = 0u64;
    for entry in fs::read_dir(root).map_err(|_| "store_unavailable")? {
        count += 1;
        if count > MAX_OBJECTS + 256 {
            return Err("bundle_limit".into());
        }
        let entry = entry.map_err(|_| "store_unavailable")?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(".tmp") {
            continue;
        }
        let id = name.strip_suffix(".json").unwrap_or("");
        if !bundle::is_hash(id) {
            issues.push(Issue {
                object: String::new(),
                code: "unexpected_file".into(),
            });
            continue;
        }
        let read = storage::read(&entry.path(), MAX_WIRE)
            .and_then(|bytes| {
                total_bytes += bytes.len() as u64;
                if total_bytes > bundle::MAX_STORE as u64 {
                    return Err("bundle_limit".into());
                }
                Bundle::parse(&bytes)
            })
            .and_then(|b| {
                if b.id != id {
                    return Err("hash_mismatch".into());
                }
                if b.snapshot.space != space {
                    return Err("space_mismatch".into());
                }
                Ok(b)
            });
        if total_bytes > bundle::MAX_STORE as u64 {
            return Err("bundle_limit".into());
        }
        match read {
            Ok(b) => {
                objects.insert(id.into(), b);
            }
            Err(code) => issues.push(Issue {
                object: id.into(),
                code,
            }),
        }
    }
    if objects.len() > MAX_OBJECTS {
        return Err("bundle_limit".into());
    }
    Ok((objects, issues))
}
fn graph(objects: &BTreeMap<String, Bundle>) -> (Journal, Vec<Issue>) {
    let mut ready = BTreeSet::new();
    let mut invalid = BTreeSet::new();
    for (id, b) in objects {
        if b.snapshot.parents.iter().any(|p| {
            objects
                .get(p)
                .is_some_and(|parent| parent.snapshot.stream != b.snapshot.stream)
        }) {
            invalid.insert(id.clone());
        }
    }
    loop {
        let before = ready.len();
        for (id, b) in objects {
            if !invalid.contains(id) && b.snapshot.parents.iter().all(|p| ready.contains(p)) {
                ready.insert(id.clone());
            }
        }
        if ready.len() == before {
            break;
        }
    }
    let mut streams: BTreeMap<Stream, BTreeSet<String>> = BTreeMap::new();
    for id in &ready {
        streams
            .entry(objects[id].snapshot.stream.clone())
            .or_default()
            .insert(id.clone());
    }
    for id in &ready {
        for p in &objects[id].snapshot.parents {
            if let Some(heads) = streams.get_mut(&objects[id].snapshot.stream) {
                heads.remove(p);
            }
        }
    }
    let pending = objects
        .keys()
        .filter(|id| !ready.contains(*id))
        .cloned()
        .collect();
    (
        Journal {
            schema: 1,
            objects: objects.keys().cloned().collect(),
            pending,
            streams: streams
                .into_iter()
                .map(|(stream, ids)| Heads {
                    stream,
                    ids: ids.into_iter().collect(),
                })
                .collect(),
        },
        invalid
            .into_iter()
            .map(|object| Issue {
                object,
                code: "invalid_parent".into(),
            })
            .collect(),
    )
}

/// Export only an explicit list from a quiescent staging directory, never a live agent profile.
/// Two identical full reads detect common writes during capture; they are not an OS snapshot.
pub fn capture(root: &Path, paths: &[String]) -> Result<BTreeMap<String, String>> {
    capture_checked(root, paths, || {})
}
fn capture_checked(
    root: &Path,
    paths: &[String],
    between: impl FnOnce(),
) -> Result<BTreeMap<String, String>> {
    if paths.is_empty() || paths.len() > 256 {
        return Err("bundle_limit".into());
    }
    let m = fs::symlink_metadata(root).map_err(|_| "source_unavailable")?;
    if !m.is_dir() || m.file_type().is_symlink() {
        return Err("unsafe_source".into());
    }
    let pass = || -> Result<BTreeMap<String, String>> {
        let mut out = BTreeMap::new();
        let mut total = 0;
        for path in paths {
            if !bundle::portable_path(path) {
                return Err("invalid_path".into());
            }
            // Only prepared text artifacts are eligible; no automatic home-directory scans.
            let lower = path.to_ascii_lowercase();
            if ![".md", ".jsonl", ".txt"]
                .iter()
                .any(|ext| lower.ends_with(ext))
                || lower.split('/').any(|p| {
                    p.starts_with('.')
                        || p.contains("auth")
                        || p.contains("credential")
                        || p.contains("secret")
                        || p == "node_modules"
                })
            {
                return Err("excluded_file".into());
            }
            let mut current = root.to_path_buf();
            for part in path.split('/') {
                current.push(part);
                if fs::symlink_metadata(&current)
                    .map_err(|_| "source_unavailable")?
                    .file_type()
                    .is_symlink()
                {
                    return Err("unsafe_source".into());
                }
            }
            let bytes = storage::read(&current, bundle::MAX_FILE as u64)?;
            total += bytes.len();
            if total > bundle::MAX_CONTENT {
                return Err("bundle_limit".into());
            }
            let content = String::from_utf8(bytes).map_err(|_| "invalid_text")?;
            if out.insert(path.clone(), content).is_some() {
                return Err("invalid_path".into());
            }
        }
        Ok(out)
    };
    let first = pass()?;
    between();
    let second = pass()?;
    if first != second {
        return Err("source_changed".into());
    }
    Ok(second)
}

fn capacity(objects: &BTreeMap<String, Bundle>) -> Result<()> {
    if objects.len() > MAX_OBJECTS {
        return Err("bundle_limit".into());
    }
    let mut size = 0usize;
    for b in objects.values() {
        size = size.checked_add(b.bytes()?.len()).ok_or("bundle_limit")?;
        if size > bundle::MAX_STORE {
            return Err("bundle_limit".into());
        }
    }
    Ok(())
}
