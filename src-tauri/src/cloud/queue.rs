//! One-shot encrypted exchange for explicit, fixture-exported replicas. No scheduler or native import.
use super::{crypto::SpaceKey, drive::Drive, Result};
use crate::sync::{
    bundle::{token, Bundle, Entry, Snapshot, Stream, MAX_OBJECTS, MAX_STORE},
    storage, Direction, Replica,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Condvar, Mutex},
};
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
    pub foreign_objects: usize,
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
    crate::progress::stage("list", None);
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
    let mut result = Exchange::default();
    let mut other = BTreeMap::new();
    let mut remote_hashes = std::collections::BTreeSet::new();
    crate::progress::stage(
        "download",
        Some(ids.iter().filter(|id| *id != &binding.proof).count()),
    );
    for id in ids {
        if id == binding.proof {
            continue;
        }
        let downloaded = remote.get(&binding.folder, &id, &binding.space, key);
        crate::progress::advance();
        let b = match downloaded {
            Ok(b) => b,
            // Only a positively classified foreign envelope can be skipped. The binding proof above
            // is always strict; unsupported formats, wrong keys and corrupted objects still fail.
            Err(e) if e == "foreign_space" => {
                result.foreign_objects += 1;
                continue;
            }
            Err(e) => return Err(e),
        };
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
    if !matches!(direction, Direction::Download) {
        crate::progress::stage(
            "upload",
            Some(
                local
                    .keys()
                    .filter(|id| !remote_hashes.contains(*id))
                    .count(),
            ),
        );
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
            crate::progress::advance();
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
    fn mixed_folder_preserves_own_downloads_and_reports_foreign_objects_without_importing_them() {
        let (temp, binding, key, remote, a, b) = setup();
        a.export_from(stream(), files("own-space"), None).unwrap();
        exchange(
            &temp.path().join("qa"),
            &a,
            &binding,
            &key,
            &remote,
            Direction::Upload,
        )
        .unwrap();
        let foreign_key = SpaceKey::generate().unwrap();
        let foreign = proof_bundle("another-space").unwrap();
        remote
            .objects
            .borrow_mut()
            .insert("foreign".into(), foreign_key.seal(&foreign).unwrap());
        let q = temp.path().join("qb");
        let r = exchange(&q, &b, &binding, &key, &remote, Direction::Download).unwrap();
        assert_eq!((r.received, r.foreign_objects), (1, 1));
        assert!(b
            .transport_bundles()
            .unwrap()
            .values()
            .all(|v| v.snapshot.space == binding.space));
        let r = exchange(&q, &b, &binding, &key, &remote, Direction::Download).unwrap();
        assert_eq!((r.received, r.published, r.foreign_objects), (0, 0, 1));
        // The selected proof is never optional, even if normal objects can be isolated.
        remote
            .objects
            .borrow_mut()
            .insert(binding.proof.clone(), foreign_key.seal(&foreign).unwrap());
        assert_eq!(
            exchange(&q, &b, &binding, &key, &remote, Direction::Download).unwrap_err(),
            "foreign_space"
        );
        assert_eq!(b.transport_bundles().unwrap().len(), 1);
    }
    #[test]
    fn same_space_wrong_keys_unknown_versions_and_malformed_objects_are_not_skipped() {
        let (temp, binding, key, remote, _, b) = setup();
        let own = proof_bundle(&binding.space).unwrap();
        let wrong_key = SpaceKey::generate().unwrap();
        remote
            .objects
            .borrow_mut()
            .insert("bad".into(), wrong_key.seal(&own).unwrap());
        let q = temp.path().join("queue");
        assert_eq!(
            exchange(&q, &b, &binding, &key, &remote, Direction::Download).unwrap_err(),
            "decrypt_failed"
        );
        let mut envelope: serde_json::Value =
            serde_json::from_slice(&key.seal(&own).unwrap()).unwrap();
        envelope["version"] = 2.into();
        remote
            .objects
            .borrow_mut()
            .insert("bad".into(), serde_json::to_vec(&envelope).unwrap());
        assert_eq!(
            exchange(&q, &b, &binding, &key, &remote, Direction::Download).unwrap_err(),
            "unsupported_encryption_version"
        );
        remote
            .objects
            .borrow_mut()
            .insert("bad".into(), b"not an envelope".to_vec());
        assert_eq!(
            exchange(&q, &b, &binding, &key, &remote, Direction::Download).unwrap_err(),
            "invalid_envelope"
        );
        assert!(b.transport_bundles().unwrap().is_empty());
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
    listings: Mutex<Listings>,
    downloads: [std::sync::Mutex<()>; 16],
    fresh: Option<String>,
}
#[derive(Default)]
struct Listings {
    revisions: BTreeMap<(String, String), Option<String>>,
    generations: BTreeMap<String, u64>,
    in_flight: BTreeMap<String, Arc<ListFlight>>,
}
struct ListFlight {
    generation: u64,
    result: Mutex<Option<Result<Vec<String>>>>,
    ready: Condvar,
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
            listings: Default::default(),
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
        loop {
            let (flight, leader) = {
                let mut listings = self.listings.lock().map_err(|_| "cloud_cache_busy")?;
                if let Some(flight) = listings.in_flight.get(folder) {
                    (Arc::clone(flight), false)
                } else {
                    let flight = Arc::new(ListFlight {
                        generation: *listings.generations.get(folder).unwrap_or(&0),
                        result: Mutex::new(None),
                        ready: Condvar::new(),
                    });
                    listings
                        .in_flight
                        .insert(folder.into(), Arc::clone(&flight));
                    (flight, true)
                }
            };
            if leader {
                // A remote panic must wake followers before it propagates to the worker.
                let (result, panic) =
                    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        self.remote.revisions(folder)
                    })) {
                        Ok(result) => (result, None),
                        Err(panic) => (Err("cloud_list_panicked".into()), Some(panic)),
                    };
                let mut listings = self.listings.lock().map_err(|_| "cloud_cache_busy")?;
                let current = *listings.generations.get(folder).unwrap_or(&0) == flight.generation;
                if current {
                    if let Ok(revisions) = &result {
                        listings.revisions.retain(|(f, _), _| f != folder);
                        listings
                            .revisions
                            .extend(revisions.iter().map(|(id, revision)| {
                                ((folder.to_string(), id.clone()), revision.clone())
                            }));
                    }
                }
                if listings
                    .in_flight
                    .get(folder)
                    .is_some_and(|f| Arc::ptr_eq(f, &flight))
                {
                    listings.in_flight.remove(folder);
                }
                let ids = result.map(|revisions| revisions.into_iter().map(|(id, _)| id).collect());
                *flight.result.lock().map_err(|_| "cloud_cache_busy")? = Some(ids.clone());
                flight.ready.notify_all();
                drop(listings);
                if let Some(panic) = panic {
                    std::panic::resume_unwind(panic);
                }
                if current {
                    return ids;
                }
            } else {
                let mut result = flight.result.lock().map_err(|_| "cloud_cache_busy")?;
                while result.is_none() {
                    result = flight.ready.wait(result).map_err(|_| "cloud_cache_busy")?;
                }
                let ids = result.as_ref().expect("checked above").clone();
                drop(result);
                if *self
                    .listings
                    .lock()
                    .map_err(|_| "cloud_cache_busy")?
                    .generations
                    .get(folder)
                    .unwrap_or(&0)
                    == flight.generation
                {
                    return ids;
                }
            }
        }
    }
    fn allocate(&self) -> Result<String> {
        self.remote.allocate()
    }
    fn put(&self, folder: &str, id: &str, key: &SpaceKey, bundle: &Bundle) -> Result<()> {
        self.remote.put(folder, id, key, bundle)?;
        let mut listings = self.listings.lock().map_err(|_| "cloud_cache_busy")?;
        let generation = listings.generations.entry(folder.into()).or_default();
        *generation = generation.wrapping_add(1);
        listings.revisions.remove(&(folder.into(), id.into()));
        listings.in_flight.remove(folder);
        Ok(())
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
            .listings
            .lock()
            .map_err(|_| "cloud_cache_busy")?
            .revisions
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

    #[test]
    fn overlapping_lists_share_one_request_but_later_list_refreshes_revision() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Barrier,
        };
        struct ListingRemote {
            calls: AtomicUsize,
            revision: AtomicUsize,
            gate: (Mutex<bool>, Condvar),
        }
        impl Objects for ListingRemote {
            fn ids(&self, _: &str) -> Result<Vec<String>> {
                unreachable!()
            }
            fn revisions(&self, _: &str) -> Result<Vec<(String, Option<String>)>> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                let mut open = self.gate.0.lock().unwrap();
                while !*open {
                    open = self.gate.1.wait(open).unwrap();
                }
                Ok(vec![(
                    "object".into(),
                    Some(self.revision.load(Ordering::SeqCst).to_string()),
                )])
            }
            fn allocate(&self) -> Result<String> {
                unreachable!()
            }
            fn put(&self, _: &str, _: &str, _: &SpaceKey, _: &Bundle) -> Result<()> {
                unreachable!()
            }
            fn get(&self, _: &str, _: &str, space: &str, _: &SpaceKey) -> Result<Bundle> {
                proof_bundle(space)
            }
        }
        let root = tempfile::tempdir().unwrap();
        let remote = ListingRemote {
            calls: AtomicUsize::new(0),
            revision: AtomicUsize::new(1),
            gate: (Mutex::new(false), Condvar::new()),
        };
        let cache = CachedObjects::new(&remote, root.path()).unwrap();
        let start = Barrier::new(9);
        std::thread::scope(|scope| {
            let threads: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        start.wait();
                        assert_eq!(cache.ids("folder").unwrap(), ["object"]);
                    })
                })
                .collect();
            start.wait();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            let overlapped = loop {
                let followers = cache
                    .listings
                    .lock()
                    .unwrap()
                    .in_flight
                    .get("folder")
                    .map(Arc::strong_count)
                    .unwrap_or(0);
                if followers == 9 {
                    break true;
                }
                if std::time::Instant::now() >= deadline {
                    break false;
                }
                std::thread::yield_now();
            };
            *remote.gate.0.lock().unwrap() = true;
            remote.gate.1.notify_all();
            for thread in threads {
                thread.join().unwrap();
            }
            assert!(overlapped, "list callers did not overlap");
        });
        assert_eq!(remote.calls.load(Ordering::SeqCst), 1);
        remote.revision.store(2, Ordering::SeqCst);
        cache.ids("folder").unwrap();
        assert_eq!(remote.calls.load(Ordering::SeqCst), 2);
        assert_eq!(
            cache
                .listings
                .lock()
                .unwrap()
                .revisions
                .get(&("folder".into(), "object".into())),
            Some(&Some("2".into()))
        );
    }

    #[test]
    fn own_write_invalidates_an_in_flight_listing() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct WritingRemote {
            calls: AtomicUsize,
            revision: AtomicUsize,
            gate: (Mutex<bool>, Condvar),
        }
        impl Objects for WritingRemote {
            fn ids(&self, _: &str) -> Result<Vec<String>> {
                unreachable!()
            }
            fn revisions(&self, _: &str) -> Result<Vec<(String, Option<String>)>> {
                let old = self.revision.load(Ordering::SeqCst);
                if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
                    let mut open = self.gate.0.lock().unwrap();
                    while !*open {
                        open = self.gate.1.wait(open).unwrap();
                    }
                }
                Ok(vec![("object".into(), Some(old.to_string()))])
            }
            fn allocate(&self) -> Result<String> {
                unreachable!()
            }
            fn put(&self, _: &str, _: &str, _: &SpaceKey, _: &Bundle) -> Result<()> {
                self.revision.store(2, Ordering::SeqCst);
                Ok(())
            }
            fn get(&self, _: &str, _: &str, space: &str, _: &SpaceKey) -> Result<Bundle> {
                proof_bundle(space)
            }
        }
        let root = tempfile::tempdir().unwrap();
        let remote = WritingRemote {
            calls: AtomicUsize::new(0),
            revision: AtomicUsize::new(1),
            gate: (Mutex::new(false), Condvar::new()),
        };
        let cache = CachedObjects::new(&remote, root.path()).unwrap();
        let key = SpaceKey::generate().unwrap();
        std::thread::scope(|scope| {
            let list = scope.spawn(|| cache.ids("folder").unwrap());
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while remote.calls.load(Ordering::SeqCst) == 0 {
                if std::time::Instant::now() >= deadline {
                    *remote.gate.0.lock().unwrap() = true;
                    remote.gate.1.notify_all();
                    panic!("first list did not start");
                }
                std::thread::yield_now();
            }
            cache
                .put("folder", "object", &key, &proof_bundle("space").unwrap())
                .unwrap();
            *remote.gate.0.lock().unwrap() = true;
            remote.gate.1.notify_all();
            assert_eq!(list.join().unwrap(), ["object"]);
        });
        assert_eq!(remote.calls.load(Ordering::SeqCst), 2);
        assert_eq!(
            cache
                .listings
                .lock()
                .unwrap()
                .revisions
                .get(&("folder".into(), "object".into())),
            Some(&Some("2".into()))
        );
    }

    #[test]
    fn own_write_preserves_unrelated_cached_download() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct TwoObjects(AtomicUsize);
        impl Objects for TwoObjects {
            fn ids(&self, _: &str) -> Result<Vec<String>> {
                unreachable!()
            }
            fn revisions(&self, _: &str) -> Result<Vec<(String, Option<String>)>> {
                Ok(vec![
                    ("old".into(), Some("1".into())),
                    ("new".into(), Some("1".into())),
                ])
            }
            fn allocate(&self) -> Result<String> {
                unreachable!()
            }
            fn put(&self, _: &str, _: &str, _: &SpaceKey, _: &Bundle) -> Result<()> {
                Ok(())
            }
            fn get(&self, _: &str, _: &str, space: &str, _: &SpaceKey) -> Result<Bundle> {
                self.0.fetch_add(1, Ordering::SeqCst);
                proof_bundle(space)
            }
        }
        let root = tempfile::tempdir().unwrap();
        let remote = TwoObjects(AtomicUsize::new(0));
        let key = SpaceKey::generate().unwrap();
        let cache = CachedObjects::new(&remote, root.path()).unwrap();
        cache.ids("folder").unwrap();
        cache.get("folder", "old", "space", &key).unwrap();
        cache
            .put("folder", "new", &key, &proof_bundle("space").unwrap())
            .unwrap();
        cache.get("folder", "old", "space", &key).unwrap();
        assert_eq!(remote.0.load(Ordering::SeqCst), 1);
        cache.get("folder", "new", "space", &key).unwrap();
        assert_eq!(remote.0.load(Ordering::SeqCst), 2);
    }
}
