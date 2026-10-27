//! Rebuildable, bounded Drive download cache. No lock is held during a remote request.
//! Bundle IDs address payloads; remote IDs only address small revision references.
use super::Result;
use crate::sync::{
    bundle::{hash, is_hash, Bundle, MAX_WIRE},
    storage,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) const MAX_BYTES: u64 = 256 * 1024 * 1024;
pub(crate) const MAX_REFS: usize = 4096;
const MAX_REF_BYTES: u64 = 4096;
const TTL: u64 = 7 * 24 * 60 * 60;
const LOCK: &str = "cache-v2.lock";
const STATE: &str = "cache-v2-state.json";
const DIRTY: &str = "cache-v2-dirty";
const GC_INTERVAL: u64 = 60 * 60;
// State (<=256 bytes), old+temporary state, and the durable dirty marker all fit
// in this reservation. Payload admission also reserves the new atomic ref copy.
const STATE_RESERVE: u64 = 1024;
#[cfg(test)]
thread_local! { static SCAN_VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    revision: String,
    blob: String,
    used: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    schema: u32,
    bytes: u64,
    refs: usize,
    collected: u64,
}
fn now() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "store_unavailable")?
        .as_secs())
}
pub(crate) fn remote_key(folder: &str, space: &str, id: &str) -> String {
    // A structured JSON tuple prevents separators in identifiers aliasing a key.
    hash(&serde_json::to_vec(&(folder, space, id)).expect("string tuple serialization"))
}
fn named(name: &str, prefix: &str) -> bool {
    name.strip_prefix(prefix)
        .and_then(|s| s.strip_suffix(".json"))
        .is_some_and(is_hash)
}
fn blob_path(root: &Path, id: &str) -> PathBuf {
    root.join(format!("blob-{id}.json"))
}
fn ref_path(root: &Path, id: &str) -> PathBuf {
    root.join(format!("ref-{id}.json"))
}
fn reference(path: &Path) -> Result<Reference> {
    let r: Reference = serde_json::from_slice(&storage::read(path, MAX_REF_BYTES)?)
        .map_err(|_| "local_store_damaged")?;
    if !is_hash(&r.blob) || r.revision.is_empty() || r.revision.len() > 1024 {
        return Err("local_store_damaged".into());
    }
    Ok(r)
}
fn remove(path: &Path) -> Result<()> {
    fs::remove_file(path).map_err(|_| "store_unavailable".into())
}
fn present(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err("store_unavailable".into()),
    }
}
fn locked(root: &Path) -> Result<Option<fs::File>> {
    storage::directory(root)?;
    match acquire(root) {
        Ok(lock) => Ok(Some(lock)),
        // Contention only disables this optional cache operation; validation still occurs.
        Err(e) if e == "sync_busy" => Ok(None),
        Err(e) => Err(e),
    }
}
fn acquire(root: &Path) -> Result<fs::File> {
    let path = root.join(LOCK);
    if let Ok(meta) = fs::symlink_metadata(&path) {
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err("unsafe_store".into());
        }
    }
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|_| "store_unavailable")?;
    fs2::FileExt::try_lock_exclusive(&file).map_err(|error| {
        if error.kind() == std::io::ErrorKind::WouldBlock
            || error.raw_os_error() == fs2::lock_contended_error().raw_os_error()
        {
            "sync_busy"
        } else {
            "store_unavailable"
        }
    })?;
    Ok(file)
}
/// TTL is per reference, independent of worker MIME listings. Native and portable
/// workers share this root and must not interpret each other's absent IDs as deletion.
fn collect(root: &Path, time: u64) -> Result<State> {
    let mut live = BTreeSet::new();
    let mut opaque = false;
    let mut state = State {
        schema: 1,
        bytes: 0,
        refs: 0,
        collected: time,
    };
    for item in fs::read_dir(root).map_err(|_| "store_unavailable")? {
        #[cfg(test)]
        SCAN_VISITS.with(|visits| visits.set(visits.get() + 1));
        let item = item.map_err(|_| "store_unavailable")?;
        let name = item.file_name();
        let name = name.to_string_lossy();
        if name.strip_suffix(".json").is_some_and(is_hash) || name.starts_with(".tmp") {
            // Invalidate v1 without reading payloads or following links.
            if !item.file_type().map_err(|_| "store_unavailable")?.is_dir() {
                remove(&item.path())?;
            }
        } else if named(&name, "ref-") {
            let meta = fs::symlink_metadata(item.path()).map_err(|_| "store_unavailable")?;
            let parsed = if state.refs < MAX_REFS {
                match reference(&item.path()) {
                    Ok(r) => Some(r),
                    Err(e)
                        if ["unsafe_store", "local_store_damaged", "bundle_limit"]
                            .contains(&e.as_str()) =>
                    {
                        None
                    }
                    Err(e) => return Err(e),
                }
            } else {
                None
            };
            if let Some(r) = parsed {
                if time.saturating_sub(r.used) >= TTL {
                    remove(&item.path())?;
                    continue;
                }
                // Missing payloads are an error only when their reference is read.
                live.insert(r.blob);
            } else {
                // An unreadable reference may name any blob. Keep all payloads and
                // charge their actual bytes, so unrelated valid IDs remain usable.
                opaque = true;
            }
            state.refs = state.refs.saturating_add(1);
            state.bytes = state.bytes.saturating_add(meta.len());
        }
    }
    for item in fs::read_dir(root).map_err(|_| "store_unavailable")? {
        #[cfg(test)]
        SCAN_VISITS.with(|visits| visits.set(visits.get() + 1));
        let item = item.map_err(|_| "store_unavailable")?;
        let name = item.file_name();
        let name = name.to_string_lossy();
        if named(&name, "blob-") {
            let meta = fs::symlink_metadata(item.path()).map_err(|_| "store_unavailable")?;
            let id = &name[5..69];
            if !opaque && !live.contains(id) && !meta.is_dir() {
                remove(&item.path())?;
            } else {
                state.bytes = state.bytes.saturating_add(meta.len());
            }
        }
    }
    // Out-of-band damage may already exceed the budget. Preserve readable IDs,
    // while retaining a bounded overflow sentinel which disables further writes.
    state.bytes = state.bytes.min(MAX_BYTES + 1);
    state.refs = state.refs.min(MAX_REFS + 1);
    Ok(state)
}
fn begin(root: &Path) -> Result<()> {
    storage::replace(&root.join(DIRTY), b"1")
}
fn commit(root: &Path, state: &State) -> Result<()> {
    let bytes = serde_json::to_vec(state).map_err(|_| "local_store_damaged")?;
    if bytes.len() > 256 {
        return Err("bundle_limit".into());
    }
    storage::replace(&root.join(STATE), &bytes)?;
    remove(&root.join(DIRTY))?;
    #[cfg(unix)]
    fs::File::open(root)
        .and_then(|file| file.sync_all())
        .map_err(|_| "store_unavailable")?;
    Ok(())
}
fn state(root: &Path, time: u64) -> Result<State> {
    let dirty = present(&root.join(DIRTY))?;
    let previous = if !dirty && present(&root.join(STATE))? {
        Some(
            serde_json::from_slice::<State>(&storage::read(&root.join(STATE), 256)?)
                .map_err(|_| "local_store_damaged")?,
        )
    } else {
        None
    };
    if !dirty {
        if let Some(previous) = previous {
            if previous.schema != 1
                || previous.bytes > MAX_BYTES + 1
                || previous.refs > MAX_REFS + 1
            {
                return Err("local_store_damaged".into());
            }
            if time.saturating_sub(previous.collected) < GC_INTERVAL {
                return Ok(previous);
            }
        }
    }
    // Persist before touching files. A crash leaves a marker forcing the next
    // process to rebuild accounting before admitting any more disk writes.
    begin(root)?;
    let state = collect(root, time)?;
    commit(root, &state)?;
    Ok(state)
}
pub(crate) fn initialize(root: &Path) -> Result<()> {
    if let Some(_lock) = locked(root)? {
        state(root, now()?)?;
    }
    Ok(())
}
pub(crate) fn get(root: &Path, key: &str, revision: &str, space: &str) -> Result<Option<Bundle>> {
    let Some(_lock) = locked(root)? else {
        return Ok(None);
    };
    let time = now()?;
    let mut state = state(root, time)?;
    let path = ref_path(root, key);
    if !present(&path)? {
        return Ok(None);
    }
    let mut r = reference(&path)?;
    if time.saturating_sub(r.used) >= TTL || r.revision != revision {
        return Ok(None);
    }
    let bundle = Bundle::parse(&storage::read(&blob_path(root, &r.blob), MAX_WIRE)?)?;
    if bundle.id != r.blob {
        return Err("hash_mismatch".into());
    }
    if bundle.snapshot.space != space {
        return Err("space_mismatch".into());
    }
    // Refresh at most hourly: frequent hits must not fsync three metadata files
    // each time. Expiration still checks the persisted timestamp on every read.
    if time.saturating_sub(r.used) < GC_INTERVAL {
        return Ok(Some(bundle));
    }
    r.used = time;
    let bytes = serde_json::to_vec(&r).map_err(|_| "local_store_damaged")?;
    if bytes.len() as u64 <= MAX_REF_BYTES
        && state.bytes.saturating_add(bytes.len() as u64) <= MAX_BYTES - STATE_RESERVE
    {
        let old = fs::symlink_metadata(&path)
            .map_err(|_| "store_unavailable")?
            .len();
        begin(root)?;
        storage::replace(&path, &bytes)?;
        state.bytes = state
            .bytes
            .saturating_sub(old)
            .saturating_add(bytes.len() as u64);
        commit(root, &state)?;
    }
    Ok(Some(bundle))
}
pub(crate) fn put(root: &Path, key: &str, revision: &str, bundle: &Bundle) -> Result<()> {
    put_with_limits(root, key, revision, bundle, MAX_BYTES, MAX_REFS)
}
fn put_with_limits(
    root: &Path,
    key: &str,
    revision: &str,
    bundle: &Bundle,
    max_bytes: u64,
    max_refs: usize,
) -> Result<()> {
    let bytes = bundle.bytes()?;
    if revision.is_empty() || revision.len() > 1024 {
        return Ok(());
    }
    let time = now()?;
    let metadata = serde_json::to_vec(&Reference {
        revision: revision.into(),
        blob: bundle.id.clone(),
        used: time,
    })
    .map_err(|_| "local_store_damaged")?;
    // JSON escaping can expand a short untrusted revision beyond the read limit.
    if metadata.len() as u64 > MAX_REF_BYTES {
        return Ok(());
    }
    let Some(_lock) = locked(root)? else {
        return Ok(());
    };
    let mut state = state(root, time)?;
    let path = ref_path(root, key);
    let old = if present(&path)? {
        reference(&path)?; // Never overwrite an unreadable accessed reference.
        Some(
            fs::symlink_metadata(&path)
                .map_err(|_| "store_unavailable")?
                .len(),
        )
    } else {
        None
    };
    let blob = blob_path(root, &bundle.id);
    let exists = present(&blob)?;
    if exists {
        let cached = Bundle::parse(&storage::read(&blob, MAX_WIRE)?)?;
        if cached != *bundle {
            return Err("hash_mismatch".into());
        }
    }
    let required = metadata.len() as u64 + if exists { 0 } else { bytes.len() as u64 };
    if (old.is_none() && state.refs >= max_refs)
        || state.bytes.saturating_add(required) > max_bytes.saturating_sub(STATE_RESERVE)
    {
        return Ok(());
    }
    begin(root)?;
    if !exists {
        storage::immutable(&blob, &bytes)?;
        state.bytes += bytes.len() as u64;
    }
    storage::replace(&path, &metadata)?;
    state.bytes = state
        .bytes
        .saturating_sub(old.unwrap_or(0))
        .saturating_add(metadata.len() as u64);
    if old.is_none() {
        state.refs += 1;
    }
    commit(root, &state)
}
/// The same root lock serializes manual cleanup with every cache read/write/GC.
pub(crate) fn clear(root: &Path) -> Result<usize> {
    if !present(root)? {
        return Ok(0);
    }
    storage::directory(root)?;
    let _lock = acquire(root)?;
    let mut count = 0;
    for item in fs::read_dir(root).map_err(|_| "store_unavailable")? {
        let item = item.map_err(|_| "store_unavailable")?;
        let name = item.file_name();
        let name = name.to_string_lossy();
        if (named(&name, "ref-")
            || named(&name, "blob-")
            || name.strip_suffix(".json").is_some_and(is_hash)
            || name.starts_with(".tmp")
            || name == STATE
            || name == DIRTY)
            && item.file_type().map_err(|_| "store_unavailable")?.is_file()
        {
            remove(&item.path())?;
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cloud::queue::proof_bundle, sync::bundle::Entry};
    fn bundle(n: usize) -> Bundle {
        let mut snapshot = proof_bundle("space").unwrap().snapshot;
        snapshot.files.insert(
            "payload.txt".into(),
            Entry::new(format!("{n}:{}", "x".repeat(2048))),
        );
        Bundle::new(snapshot).unwrap()
    }
    fn files(root: &Path, prefix: &str) -> usize {
        fs::read_dir(root)
            .unwrap()
            .filter(|e| named(&e.as_ref().unwrap().file_name().to_string_lossy(), prefix))
            .count()
    }
    fn size(root: &Path) -> u64 {
        fs::read_dir(root)
            .unwrap()
            .map(|e| e.unwrap().metadata().unwrap().len())
            .sum()
    }
    #[test]
    fn thousands_of_references_do_not_scan_the_directory_on_every_hit() {
        let root = tempfile::tempdir().unwrap();
        let bundle = bundle(0);
        fs::write(blob_path(root.path(), &bundle.id), bundle.bytes().unwrap()).unwrap();
        let metadata = serde_json::to_vec(&Reference {
            revision: "1".into(),
            blob: bundle.id.clone(),
            used: now().unwrap(),
        })
        .unwrap();
        for n in 0..4096 {
            fs::write(
                ref_path(root.path(), &hash(n.to_string().as_bytes())),
                &metadata,
            )
            .unwrap();
        }
        initialize(root.path()).unwrap();
        SCAN_VISITS.with(|visits| visits.set(0));
        let start = std::time::Instant::now();
        for n in 0..16 {
            assert_eq!(
                get(root.path(), &hash(n.to_string().as_bytes()), "1", "space").unwrap(),
                Some(bundle.clone())
            );
        }
        let visits = SCAN_VISITS.with(|visits| visits.get());
        eprintln!(
            "4096 refs, 16 cache hits: {:?}, {visits} directory entries visited",
            start.elapsed()
        );
        assert!(visits <= 2 * (4096 + 4), "per-hit full scans: {visits}");
    }
    #[test]
    fn distinct_bundles_and_references_respect_both_limits() {
        let root = tempfile::tempdir().unwrap();
        for n in 0..20 {
            put_with_limits(
                root.path(),
                &hash(n.to_string().as_bytes()),
                "1",
                &bundle(n),
                10_000,
                100,
            )
            .unwrap();
            assert!(size(root.path()) <= 10_000);
        }
        assert_eq!(files(root.path(), "blob-"), 3);
        assert_eq!(files(root.path(), "ref-"), 3);
        let root = tempfile::tempdir().unwrap();
        for n in 0..20 {
            put_with_limits(
                root.path(),
                &hash(n.to_string().as_bytes()),
                "1",
                &bundle(0),
                MAX_BYTES,
                4,
            )
            .unwrap();
        }
        assert_eq!(files(root.path(), "blob-"), 1);
        assert_eq!(files(root.path(), "ref-"), 4);
    }
    #[test]
    fn escaped_revision_cannot_create_an_unreadable_reference() {
        let root = tempfile::tempdir().unwrap();
        let key = hash(b"key");
        let revision = "\0".repeat(1024);
        put(root.path(), &key, &revision, &bundle(0)).unwrap();
        assert_eq!(files(root.path(), "ref-"), 0);
        assert_eq!(files(root.path(), "blob-"), 0);
        assert!(get(root.path(), &key, &revision, "space")
            .unwrap()
            .is_none());
        put(root.path(), &key, "1", &bundle(0)).unwrap();
        put(root.path(), &key, &revision, &bundle(1)).unwrap();
        assert_eq!(
            get(root.path(), &key, "1", "space").unwrap(),
            Some(bundle(0))
        );
    }
    #[test]
    fn damaged_reference_does_not_block_other_ids_after_restart_or_gc() {
        for damage in 0..3 {
            let root = tempfile::tempdir().unwrap();
            let a = hash(b"a");
            let b = hash(b"b");
            put(root.path(), &a, "1", &bundle(0)).unwrap();
            put(root.path(), &b, "1", &bundle(1)).unwrap();
            match damage {
                0 => fs::write(ref_path(root.path(), &a), b"broken").unwrap(),
                1 => fs::remove_file(blob_path(root.path(), &bundle(0).id)).unwrap(),
                _ => fs::File::create(ref_path(root.path(), &a))
                    .unwrap()
                    .set_len(MAX_REF_BYTES + 1)
                    .unwrap(),
            }
            // Exercise a real persisted recovery scan, not just a warm ledger.
            fs::write(root.path().join(DIRTY), b"1").unwrap();
            initialize(root.path()).unwrap();
            assert_eq!(get(root.path(), &b, "1", "space").unwrap(), Some(bundle(1)));
            assert!(get(root.path(), &a, "1", "space").is_err());
            let state = state(root.path(), now().unwrap()).unwrap();
            let actual: u64 = fs::read_dir(root.path())
                .unwrap()
                .map(|e| e.unwrap())
                .filter(|e| {
                    let name = e.file_name();
                    named(&name.to_string_lossy(), "ref-")
                        || named(&name.to_string_lossy(), "blob-")
                })
                .map(|e| e.metadata().unwrap().len())
                .sum();
            assert_eq!(state.bytes, actual);
        }
    }
    #[test]
    fn dirty_recovery_recounts_uncommitted_files_and_invalid_ledgers_fail_closed() {
        let root = tempfile::tempdir().unwrap();
        initialize(root.path()).unwrap();
        let key = hash(b"uncommitted");
        let content = bundle(0);
        begin(root.path()).unwrap();
        fs::write(
            blob_path(root.path(), &content.id),
            content.bytes().unwrap(),
        )
        .unwrap();
        fs::write(
            ref_path(root.path(), &key),
            serde_json::to_vec(&Reference {
                revision: "1".into(),
                blob: content.id.clone(),
                used: now().unwrap(),
            })
            .unwrap(),
        )
        .unwrap();
        initialize(root.path()).unwrap();
        assert!(!root.path().join(DIRTY).exists());
        let recovered = state(root.path(), now().unwrap()).unwrap();
        assert_eq!(recovered.refs, 1);
        assert!(recovered.bytes > content.bytes().unwrap().len() as u64);
        for (schema, bytes, refs) in [(0, 1, 1), (1, MAX_BYTES + 2, 1), (1, 1, MAX_REFS + 2)] {
            fs::write(
                root.path().join(STATE),
                serde_json::to_vec(&State {
                    schema,
                    bytes,
                    refs,
                    collected: now().unwrap(),
                })
                .unwrap(),
            )
            .unwrap();
            assert_eq!(initialize(root.path()).unwrap_err(), "local_store_damaged");
        }
        clear(root.path()).unwrap();
        assert!(!root.path().join(STATE).exists());
        assert!(!root.path().join(DIRTY).exists());
        initialize(root.path()).unwrap();
    }
    #[test]
    fn targeted_read_rejects_expired_reference_before_hourly_collection() {
        let root = tempfile::tempdir().unwrap();
        let key = hash(b"expired");
        put(root.path(), &key, "1", &bundle(0)).unwrap();
        let mut r = reference(&ref_path(root.path(), &key)).unwrap();
        r.used = now().unwrap() - TTL;
        fs::write(ref_path(root.path(), &key), serde_json::to_vec(&r).unwrap()).unwrap();
        // The ledger's GC timestamp is still fresh; only this read checks expiry.
        assert!(get(root.path(), &key, "1", "space").unwrap().is_none());
        assert!(ref_path(root.path(), &key).exists());
    }
    #[test]
    fn ttl_orphans_and_legacy_are_reclaimed_without_listing_namespaces() {
        let root = tempfile::tempdir().unwrap();
        let native = remote_key("folder", "space", "native");
        let portable = remote_key("folder", "space", "portable");
        put(root.path(), &native, "1", &bundle(0)).unwrap();
        put(root.path(), &portable, "1", &bundle(1)).unwrap();
        let mut old = reference(&ref_path(root.path(), &native)).unwrap();
        old.used = 0;
        fs::write(
            ref_path(root.path(), &native),
            serde_json::to_vec(&old).unwrap(),
        )
        .unwrap();
        let legacy = root.path().join(format!("{}.json", "a".repeat(64)));
        fs::File::create(&legacy)
            .unwrap()
            .set_len(MAX_BYTES + 1)
            .unwrap();
        let orphan = blob_path(root.path(), &hash(b"orphan"));
        fs::write(&orphan, b"not parsed").unwrap();
        fs::write(root.path().join(".tmp-interrupted"), b"partial").unwrap();
        fs::write(root.path().join(DIRTY), b"1").unwrap();
        initialize(root.path()).unwrap();
        assert!(!legacy.exists());
        assert!(!orphan.exists());
        assert!(!root.path().join(".tmp-interrupted").exists());
        assert_eq!(files(root.path(), "blob-"), 1);
        assert_eq!(files(root.path(), "ref-"), 1);
        assert_eq!(
            get(root.path(), &portable, "1", "space").unwrap(),
            Some(bundle(1))
        );
    }
    #[test]
    fn corrupt_reference_blob_hash_and_space_fail_closed() {
        let root = tempfile::tempdir().unwrap();
        let key = hash(b"key");
        put(root.path(), &key, "1", &bundle(0)).unwrap();
        fs::write(ref_path(root.path(), &key), b"broken").unwrap();
        assert_eq!(
            get(root.path(), &key, "1", "space").unwrap_err(),
            "local_store_damaged"
        );
        clear(root.path()).unwrap();
        put(root.path(), &key, "1", &bundle(0)).unwrap();
        fs::write(blob_path(root.path(), &bundle(0).id), b"broken").unwrap();
        assert!(get(root.path(), &key, "1", "space").is_err());
        clear(root.path()).unwrap();
        put(root.path(), &key, "1", &bundle(0)).unwrap();
        fs::write(
            blob_path(root.path(), &bundle(0).id),
            bundle(1).bytes().unwrap(),
        )
        .unwrap();
        assert_eq!(
            get(root.path(), &key, "1", "space").unwrap_err(),
            "hash_mismatch"
        );
        clear(root.path()).unwrap();
        put(root.path(), &key, "1", &bundle(0)).unwrap();
        assert_eq!(
            get(root.path(), &key, "1", "changed-space").unwrap_err(),
            "space_mismatch"
        );
        fs::File::create(ref_path(root.path(), &key))
            .unwrap()
            .set_len(MAX_REF_BYTES + 1)
            .unwrap();
        assert_eq!(
            get(root.path(), &key, "1", "space").unwrap_err(),
            "bundle_limit"
        );
    }
    #[test]
    fn independent_writers_share_one_budget_and_clear_uses_same_lock() {
        let root = tempfile::tempdir().unwrap();
        std::thread::scope(|scope| {
            for worker in 0..8 {
                let path = root.path();
                scope.spawn(move || {
                    for n in 0..16 {
                        put_with_limits(
                            path,
                            &hash(format!("{worker}:{n}").as_bytes()),
                            "1",
                            &bundle(worker * 16 + n),
                            16_000,
                            5,
                        )
                        .unwrap();
                    }
                });
            }
        });
        assert!(size(root.path()) <= 16_000);
        assert!(files(root.path(), "ref-") <= 5);
        assert_eq!(files(root.path(), "ref-"), files(root.path(), "blob-"));
        let lock = storage::lock(&root.path().join(LOCK)).unwrap();
        assert_eq!(clear(root.path()).unwrap_err(), "sync_busy");
        assert!(get(root.path(), &hash(b"missing"), "1", "space")
            .unwrap()
            .is_none());
        drop(lock);
        assert!(clear(root.path()).unwrap() > 0);
        assert_eq!(files(root.path(), "blob-"), 0);
        assert_eq!(files(root.path(), "ref-"), 0);
    }
    #[cfg(unix)]
    #[test]
    fn links_never_read_write_or_delete_their_targets() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("precious");
        fs::write(&target, b"preserve").unwrap();
        let cache = root.path().join("cache");
        fs::create_dir(&cache).unwrap();
        symlink(&target, cache.join(format!("{}.json", "a".repeat(64)))).unwrap();
        initialize(&cache).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"preserve");
        let key = hash(b"key");
        symlink(&target, ref_path(&cache, &key)).unwrap();
        assert_eq!(get(&cache, &key, "1", "space").unwrap_err(), "unsafe_store");
        fs::remove_file(ref_path(&cache, &key)).unwrap();
        symlink(&target, cache.join(LOCK)).unwrap_err(); // The real lock already exists.
        fs::remove_file(cache.join(LOCK)).unwrap();
        symlink(&target, cache.join(LOCK)).unwrap();
        assert_eq!(initialize(&cache).unwrap_err(), "unsafe_store");
        assert_eq!(fs::read(&target).unwrap(), b"preserve");
    }
}
