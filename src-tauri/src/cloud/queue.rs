//! One-shot encrypted exchange for explicit, fixture-exported replicas. No scheduler or native import.
use super::{crypto::SpaceKey, drive::Drive, Result};
use crate::sync::{
    bundle::{token, Bundle, Entry, Snapshot, Stream, MAX_OBJECTS, MAX_STORE},
    storage, Direction, Replica,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub folder: String,
    pub space: String,
    pub proof: String,
}
impl Binding {
    pub fn validate(&self) -> Result<()> {
        if [&self.folder, &self.space, &self.proof]
            .iter()
            .all(|s| token(s))
        {
            Ok(())
        } else {
            Err("invalid_cloud_binding".into())
        }
    }
}
pub trait Objects {
    fn ids(&self, folder: &str) -> Result<Vec<String>>;
    fn revisions(&self, folder: &str) -> Result<Vec<(String, Option<String>)>> {
        Ok(self.ids(folder)?.into_iter().map(|id| (id, None)).collect())
    }
    fn allocate(&self) -> Result<String>;
    fn put(&self, folder: &str, id: &str, key: &SpaceKey, bundle: &Bundle) -> Result<()>;
    fn get(&self, folder: &str, id: &str, space: &str, key: &SpaceKey) -> Result<Bundle>;
}
impl Objects for Drive {
    fn ids(&self, folder: &str) -> Result<Vec<String>> {
        Ok(self
            .list_objects(folder)?
            .into_iter()
            .map(|f| f.id)
            .collect())
    }
    fn revisions(&self, folder: &str) -> Result<Vec<(String, Option<String>)>> {
        Ok(self
            .list_objects(folder)?
            .into_iter()
            .map(|f| (f.id, f.version))
            .collect())
    }
    fn allocate(&self) -> Result<String> {
        self.allocate_id()
    }
    fn put(&self, folder: &str, id: &str, key: &SpaceKey, bundle: &Bundle) -> Result<()> {
        let file = self.upload(folder, id, key, bundle)?;
        if file.id != id {
            return Err("drive_invalid_response".into());
        }
        Ok(())
    }
    fn get(&self, folder: &str, id: &str, space: &str, key: &SpaceKey) -> Result<Bundle> {
        self.download(folder, id, space, key)
    }
}
/// The caller creates this key proof using a persisted allocated ID, then retains its Binding.
pub fn proof_bundle(space: &str) -> Result<Bundle> {
    Bundle::new(Snapshot {
        schema: 1,
        space: space.into(),
        device: "space-proof".into(),
        stream: Stream {
            agent: "codex".into(),
            profile: "bastet-protocol".into(),
            conversation: "space-proof-v1".into(),
        },
        parents: vec![],
        files: BTreeMap::from([(
            "proof.txt".into(),
            Entry::new("Bastet Agent Sync space key proof v1".into()),
        )]),
    })
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    binding: Binding,
    uploads: BTreeMap<String, String>,
}
#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Exchange {
    pub published: usize,
    pub received: usize,
    pub unchanged: usize,
    pub conflicts: usize,
    pub pending: usize,
}
/// Never treats a wrong key, missing proof or incomplete listing as an empty remote space.
pub fn exchange(
    root: &Path,
    replica: &Replica,
    binding: &Binding,
    key: &SpaceKey,
    remote: &impl Objects,
    direction: Direction,
) -> Result<Exchange> {
    exchange_filtered(root, replica, binding, key, remote, direction, None)
}
pub fn exchange_filtered(
    root: &Path,
    replica: &Replica,
    binding: &Binding,
    key: &SpaceKey,
    remote: &impl Objects,
    direction: Direction,
    agent: Option<&str>,
) -> Result<Exchange> {
    binding.validate()?;
    if replica.space_id() != binding.space {
        return Err("space_mismatch".into());
    }
    if remote.get(&binding.folder, &binding.proof, &binding.space, key)?
        != proof_bundle(&binding.space)?
    {
        return Err("invalid_space_proof".into());
    }
    storage::directory(root)?;
    let _lock = storage::lock(&root.join("exchange.lock"))?;
    let path = root.join("exchange.json");
    let mut journal = match std::fs::symlink_metadata(&path) {
        Ok(_) => serde_json::from_slice::<Journal>(&storage::read(&path, 1024 * 1024)?)
            .map_err(|_| "cloud_journal_invalid")?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Journal {
            binding: binding.clone(),
            uploads: BTreeMap::new(),
        },
        Err(_) => return Err("store_unavailable".into()),
    };
    if journal.binding != *binding {
        return Err("space_mismatch".into());
    }
    if journal.uploads.len() > MAX_OBJECTS {
        return Err("bundle_limit".into());
    }
    let ids = remote.ids(&binding.folder)?;
    if ids.len() > MAX_OBJECTS + 1 || ids.iter().any(|id| !token(id)) {
        return Err("bundle_limit".into());
    }
    let local = replica
        .transport_bundles()?
        .into_iter()
        .filter(|(_, b)| agent.is_none_or(|a| a == b.snapshot.stream.agent))
        .collect::<BTreeMap<_, _>>();
    let mut total = local
        .values()
        .try_fold(0usize, |n, b| b.bytes().map(|v| n + v.len()))?;
    if total > MAX_STORE || local.len() > MAX_OBJECTS {
        return Err("bundle_limit".into());
    }
    let mut other = BTreeMap::new();
    let mut remote_hashes = std::collections::BTreeSet::new();
    for id in ids {
        if id == binding.proof {
            continue;
        }
        let b = remote.get(&binding.folder, &id, &binding.space, key)?;
        b.validate()?;
        if b.snapshot.space != binding.space {
            return Err("space_mismatch".into());
        }
        // Other agents do not consume this adapter's union budget.
        if agent.is_some_and(|a| a != b.snapshot.stream.agent) {
            continue;
        }
        remote_hashes.insert(b.id.clone());
        if !local.contains_key(&b.id) && !other.contains_key(&b.id) {
            total = total.checked_add(b.bytes()?.len()).ok_or("bundle_limit")?;
            if total > MAX_STORE || local.len() + other.len() + 1 > MAX_OBJECTS {
                return Err("bundle_limit".into());
            }
            other.insert(b.id.clone(), b);
        }
    }
    let mut result = Exchange::default();
    if !matches!(direction, Direction::Download) {
        for (hash, b) in &local {
            if remote_hashes.contains(hash) {
                result.unchanged += 1;
                continue;
            }
            let id = match journal.uploads.get(hash) {
                Some(id) => id.clone(),
                None => {
                    if journal.uploads.len() >= MAX_OBJECTS {
                        return Err("bundle_limit".into());
                    }
                    let id = remote.allocate()?;
                    if !token(&id) {
                        return Err("drive_invalid_response".into());
                    }
                    journal.uploads.insert(hash.clone(), id.clone());
                    storage::replace(
                        &path,
                        &serde_json::to_vec(&journal).map_err(|_| "cloud_journal_invalid")?,
                    )?;
                    id
                }
            };
            match remote.put(&binding.folder, &id, key, b) {
                Ok(()) => {}
                Err(e) if e == "drive_id_exists" => {
                    if remote.get(&binding.folder, &id, &binding.space, key)? != *b {
                        return Err("immutable_collision".into());
                    }
                }
                Err(e) => return Err(e),
            }
            result.published += 1;
        }
    }
    drop(local);
    if !matches!(direction, Direction::Upload) {
        result.received = replica.receive_bundles(&other)?;
    }
    drop(other);
    let checkpoint = replica.checkpoint()?;
    result.conflicts = checkpoint
        .streams
        .iter()
        .filter(|s| s.ids.len() > 1)
        .count();
    result.pending = checkpoint.pending.len();
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    struct Remote {
        objects: RefCell<BTreeMap<String, Vec<u8>>>,
        next: Cell<usize>,
        fail_once: Cell<bool>,
        hide_once: Cell<bool>,
    }
    impl Objects for Remote {
        fn ids(&self, _: &str) -> Result<Vec<String>> {
            if self.hide_once.replace(false) {
                return Ok(vec!["proof".into()]);
            }
            Ok(self.objects.borrow().keys().cloned().collect())
        }
        fn allocate(&self) -> Result<String> {
            let n = self.next.get();
            self.next.set(n + 1);
            Ok(format!("object-{n}"))
        }
        fn put(&self, _: &str, id: &str, key: &SpaceKey, b: &Bundle) -> Result<()> {
            if self.objects.borrow().contains_key(id) {
                return Err("drive_id_exists".into());
            }
            self.objects.borrow_mut().insert(id.into(), key.seal(b)?);
            if self.fail_once.replace(false) {
                Err("network_unavailable".into())
            } else {
                Ok(())
            }
        }
        fn get(&self, _: &str, id: &str, space: &str, key: &SpaceKey) -> Result<Bundle> {
            key.open(
                space,
                self.objects.borrow().get(id).ok_or("drive_not_found")?,
            )
        }
    }
    fn setup() -> (
        tempfile::TempDir,
        Binding,
        SpaceKey,
        Remote,
        Replica,
        Replica,
    ) {
        let temp = tempfile::tempdir().unwrap();
        let key = SpaceKey::generate().unwrap();
        let binding = Binding {
            folder: "folder".into(),
            space: "space".into(),
            proof: "proof".into(),
        };
        let remote = Remote {
            objects: RefCell::new(BTreeMap::from([(
                "proof".into(),
                key.seal(&proof_bundle("space").unwrap()).unwrap(),
            )])),
            next: Cell::new(0),
            fail_once: Cell::new(false),
            hide_once: Cell::new(false),
        };
        let a = Replica::open(&temp.path().join("a"), "space").unwrap();
        let b = Replica::open(&temp.path().join("b"), "space").unwrap();
        (temp, binding, key, remote, a, b)
    }
    fn stream() -> Stream {
        Stream {
            agent: "codex".into(),
            profile: "p".into(),
            conversation: "c".into(),
        }
    }
    fn files(s: &str) -> BTreeMap<String, String> {
        BTreeMap::from([("sample.txt".into(), s.into())])
    }
    #[test]
    fn two_replicas_preserve_branches_and_do_not_repeat_encrypted_uploads() {
        let (temp, binding, key, remote, a, b) = setup();
        let qa = temp.path().join("qa");
        let qb = temp.path().join("qb");
        let base = a.export_from(stream(), files("original"), None).unwrap();
        assert_eq!(
            exchange(&qa, &a, &binding, &key, &remote, Direction::Both)
                .unwrap()
                .published,
            1
        );
        assert_eq!(
            exchange(&qb, &b, &binding, &key, &remote, Direction::Download)
                .unwrap()
                .received,
            1
        );
        a.export_from(stream(), files("A"), Some(&base)).unwrap();
        b.export_from(stream(), files("B"), Some(&base)).unwrap();
        exchange(&qa, &a, &binding, &key, &remote, Direction::Upload).unwrap();
        exchange(&qb, &b, &binding, &key, &remote, Direction::Both).unwrap();
        assert_eq!(
            exchange(&qa, &a, &binding, &key, &remote, Direction::Both)
                .unwrap()
                .conflicts,
            1
        );
        let repeated = exchange(&qa, &a, &binding, &key, &remote, Direction::Both).unwrap();
        assert_eq!(repeated.published + repeated.received, 0);
        assert_eq!(remote.objects.borrow().len(), 4);
    }
    #[test]
    fn selected_source_exchange_keeps_other_agents_out_of_the_union() {
        let (temp, binding, key, remote, a, b) = setup();
        a.export_from(stream(), files("codex"), None).unwrap();
        let mut pi = stream();
        pi.agent = "pi".into();
        a.export_from(pi, files("pi"), None).unwrap();
        let qa = temp.path().join("qa");
        let qb = temp.path().join("qb");
        assert_eq!(
            exchange_filtered(
                &qa,
                &a,
                &binding,
                &key,
                &remote,
                Direction::Both,
                Some("pi")
            )
            .unwrap()
            .published,
            1
        );
        assert_eq!(
            exchange_filtered(
                &qa,
                &a,
                &binding,
                &key,
                &remote,
                Direction::Both,
                Some("codex")
            )
            .unwrap()
            .published,
            1
        );
        let r = exchange_filtered(
            &qb,
            &b,
            &binding,
            &key,
            &remote,
            Direction::Download,
            Some("pi"),
        )
        .unwrap();
        assert_eq!(r.received, 1);
        assert!(b
            .transport_bundles()
            .unwrap()
            .values()
            .all(|b| b.snapshot.stream.agent == "pi"));
        assert_eq!(
            exchange_filtered(
                &qb,
                &b,
                &binding,
                &key,
                &remote,
                Direction::Both,
                Some("pi")
            )
            .unwrap()
            .published,
            0
        );
        assert_eq!(remote.objects.borrow().len(), 3);
    }
    #[test]
    fn ambiguous_upload_reuses_id_even_when_listing_lags() {
        let (temp, binding, key, remote, a, _) = setup();
        let q = temp.path().join("queue");
        a.export_from(stream(), files("original"), None).unwrap();
        remote.fail_once.set(true);
        assert_eq!(
            exchange(&q, &a, &binding, &key, &remote, Direction::Upload).unwrap_err(),
            "network_unavailable"
        );
        remote.hide_once.set(true);
        exchange(&q, &a, &binding, &key, &remote, Direction::Upload).unwrap();
        assert_eq!(remote.next.get(), 1);
        assert_eq!(remote.objects.borrow().len(), 2);
    }
    #[test]
    fn wrong_key_or_missing_proof_prevents_any_transfer() {
        let (temp, binding, key, remote, a, _) = setup();
        a.export_from(stream(), files("original"), None).unwrap();
        let q = temp.path().join("queue");
        assert!(exchange(
            &q,
            &a,
            &binding,
            &SpaceKey::generate().unwrap(),
            &remote,
            Direction::Both
        )
        .is_err());
        assert!(!q.exists());
        remote.objects.borrow_mut().remove("proof");
        assert!(exchange(&q, &a, &binding, &key, &remote, Direction::Both).is_err());
        assert_eq!(remote.next.get(), 0);
    }
}

/// Revision-aware cache shared by selected adapters. Missing revisions always fetch.
/// Proof is deliberately fetched before listing; cached history never substitutes for key/account validation.
pub struct CachedObjects<'a, R> {
    pub remote: &'a R,
    pub root: &'a Path,
    revisions: std::sync::Mutex<BTreeMap<(String, String), Option<String>>>,
    downloads: [std::sync::Mutex<()>; 16],
    fresh: Option<String>,
}
#[derive(Serialize, Deserialize)]
struct Cached {
    revision: String,
    bundle: Bundle,
}
impl<'a, R: Objects> CachedObjects<'a, R> {
    pub fn new(remote: &'a R, root: &'a Path) -> Result<Self> {
        storage::directory(root)?;
        Ok(Self {
            remote,
            root,
            revisions: Default::default(),
            downloads: Default::default(),
            fresh: None,
        })
    }
}
impl<R> CachedObjects<'_, R> {
    /// Key proof must still reach Drive when other adapters have already listed objects.
    pub fn with_fresh_object(mut self, id: &str) -> Self {
        self.fresh = Some(id.into());
        self
    }
}
impl<R: Objects> Objects for CachedObjects<'_, R> {
    fn ids(&self, folder: &str) -> Result<Vec<String>> {
        let revisions = self.remote.revisions(folder)?;
        let mut known = self.revisions.lock().map_err(|_| "cloud_cache_busy")?;
        known.retain(|(f, _), _| f != folder);
        known.extend(
            revisions
                .iter()
                .map(|(id, revision)| ((folder.to_string(), id.clone()), revision.clone())),
        );
        Ok(revisions.into_iter().map(|(id, _)| id).collect())
    }
    fn allocate(&self) -> Result<String> {
        self.remote.allocate()
    }
    fn put(&self, folder: &str, id: &str, key: &SpaceKey, bundle: &Bundle) -> Result<()> {
        self.remote.put(folder, id, key, bundle)
    }
    fn get(&self, folder: &str, id: &str, space: &str, key: &SpaceKey) -> Result<Bundle> {
        if self.fresh.as_deref() == Some(id) {
            return self.remote.get(folder, id, space, key);
        }
        let hash = crate::sync::bundle::hash(format!("{folder}:{space}:{id}").as_bytes());
        // Fixed stripes bound lock memory and coalesce same-object downloads. Unrelated
        // objects can proceed concurrently; no global lock is held across network I/O.
        let stripe = usize::from_str_radix(&hash[..2], 16).map_err(|_| "invalid_bundle")?
            % self.downloads.len();
        let _download = self.downloads[stripe]
            .lock()
            .map_err(|_| "cloud_cache_busy")?;
        let rev = self
            .revisions
            .lock()
            .map_err(|_| "cloud_cache_busy")?
            .get(&(folder.to_string(), id.to_string()))
            .cloned()
            .flatten();
        let path = self.root.join(format!("{hash}.json"));
        if let Some(revision) = rev.as_ref() {
            if path.exists() {
                let cached: Cached = serde_json::from_slice(&storage::read(
                    &path,
                    crate::sync::bundle::MAX_WIRE + 4096,
                )?)
                .map_err(|_| "local_store_damaged")?;
                if &cached.revision == revision {
                    cached.bundle.validate()?;
                    if cached.bundle.snapshot.space != space {
                        return Err("space_mismatch".into());
                    }
                    return Ok(cached.bundle);
                }
            }
        }
        let bundle = self.remote.get(folder, id, space, key)?;
        if let Some(revision) = rev {
            storage::replace(
                &path,
                &serde_json::to_vec(&Cached {
                    revision,
                    bundle: bundle.clone(),
                })
                .map_err(|_| "invalid_bundle")?,
            )?;
        }
        Ok(bundle)
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    use std::cell::Cell;
    struct Remote {
        revision: Cell<u32>,
        gets: Cell<u32>,
    }
    impl Objects for Remote {
        fn ids(&self, _: &str) -> Result<Vec<String>> {
            Ok(vec!["object".into()])
        }
        fn revisions(&self, _: &str) -> Result<Vec<(String, Option<String>)>> {
            Ok(vec![(
                "object".into(),
                Some(self.revision.get().to_string()),
            )])
        }
        fn allocate(&self) -> Result<String> {
            unreachable!()
        }
        fn put(&self, _: &str, _: &str, _: &SpaceKey, _: &Bundle) -> Result<()> {
            unreachable!()
        }
        fn get(&self, _: &str, _: &str, space: &str, _: &SpaceKey) -> Result<Bundle> {
            self.gets.set(self.gets.get() + 1);
            proof_bundle(space)
        }
    }
    #[test]
    fn changed_drive_revision_refetches_and_cache_survives_restart() {
        let t = tempfile::tempdir().unwrap();
        let key = SpaceKey::generate().unwrap();
        let remote = Remote {
            revision: Cell::new(1),
            gets: Cell::new(0),
        };
        let cache = CachedObjects::new(&remote, t.path()).unwrap();
        cache.ids("folder").unwrap();
        cache.get("folder", "object", "space", &key).unwrap();
        cache.get("folder", "object", "space", &key).unwrap();
        assert_eq!(remote.gets.get(), 1);
        drop(cache);
        let cache = CachedObjects::new(&remote, t.path()).unwrap();
        cache.ids("folder").unwrap();
        cache.get("folder", "object", "space", &key).unwrap();
        assert_eq!(remote.gets.get(), 1);
        remote.revision.set(2);
        cache.ids("folder").unwrap();
        cache.get("folder", "object", "space", &key).unwrap();
        assert_eq!(remote.gets.get(), 2);
        // Before a new listing (e.g. space-key proof), fetch rather than trust the disk cache.
        drop(cache);
        let cache = CachedObjects::new(&remote, t.path()).unwrap();
        cache.get("folder", "object", "space", &key).unwrap();
        assert_eq!(remote.gets.get(), 3);
    }
    #[test]
    fn concurrent_reads_share_one_download_but_key_proof_always_fetches() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Barrier,
        };
        struct SharedRemote(AtomicUsize);
        impl Objects for SharedRemote {
            fn ids(&self, _: &str) -> Result<Vec<String>> {
                Ok(vec!["object".into()])
            }
            fn revisions(&self, _: &str) -> Result<Vec<(String, Option<String>)>> {
                Ok(vec![("object".into(), Some("1".into()))])
            }
            fn allocate(&self) -> Result<String> {
                unreachable!()
            }
            fn put(&self, _: &str, _: &str, _: &SpaceKey, _: &Bundle) -> Result<()> {
                unreachable!()
            }
            fn get(&self, _: &str, _: &str, space: &str, _: &SpaceKey) -> Result<Bundle> {
                self.0.fetch_add(1, Ordering::SeqCst);
                proof_bundle(space)
            }
        }
        let root = tempfile::tempdir().unwrap();
        let remote = SharedRemote(AtomicUsize::new(0));
        let key = SpaceKey::generate().unwrap();
        let cache = CachedObjects::new(&remote, root.path()).unwrap();
        cache.ids("folder").unwrap();
        let barrier = Barrier::new(8);
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    barrier.wait();
                    for _ in 0..5 {
                        cache.get("folder", "object", "space", &key).unwrap();
                    }
                });
            }
        });
        assert_eq!(remote.0.load(Ordering::SeqCst), 1);
        let cache = cache.with_fresh_object("object");
        cache.get("folder", "object", "space", &key).unwrap();
        cache.get("folder", "object", "space", &key).unwrap();
        assert_eq!(remote.0.load(Ordering::SeqCst), 3);
    }
}
