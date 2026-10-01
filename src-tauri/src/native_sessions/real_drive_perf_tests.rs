//! Explicit real-Drive fixture test. Only a newly created test folder receives synthetic data.
use super::*;
use crate::cloud::{drive::Drive, oauth, vault::NativeStore, wizard::Wizard, wizard_desktop};
use std::{cell::Cell, io::Write, time::Instant};

const REAL_SESSIONS: usize = 3;
const REAL_SESSION_BYTES: usize = 64 * 1024;
const REAL_CWD: &str = "/bench/project";

#[derive(Clone, Copy, Default)]
struct DriveCounts {
    list_calls: u64,
    upload_objects: u64,
    encrypted_payload_bytes_estimate: u64,
    download_objects: u64,
    decoded_bundle_bytes: u64,
}
impl DriveCounts {
    fn delta(self, earlier: Self) -> Self {
        Self {
            list_calls: self.list_calls - earlier.list_calls,
            upload_objects: self.upload_objects - earlier.upload_objects,
            encrypted_payload_bytes_estimate: self.encrypted_payload_bytes_estimate
                - earlier.encrypted_payload_bytes_estimate,
            download_objects: self.download_objects - earlier.download_objects,
            decoded_bundle_bytes: self.decoded_bundle_bytes - earlier.decoded_bundle_bytes,
        }
    }
}
struct CountedDrive<'a> {
    drive: &'a Drive,
    counts: Cell<DriveCounts>,
}
impl<'a> CountedDrive<'a> {
    fn new(drive: &'a Drive) -> Self {
        Self {
            drive,
            counts: Cell::new(DriveCounts::default()),
        }
    }
}
impl Objects for CountedDrive<'_> {
    fn ids(&self, folder: &str) -> Result<Vec<String>> {
        let mut count = self.counts.get();
        count.list_calls += 1;
        self.counts.set(count);
        Objects::ids(self.drive, folder)
    }
    fn revisions(&self, folder: &str) -> Result<Vec<(String, Option<String>)>> {
        let mut count = self.counts.get();
        count.list_calls += 1;
        self.counts.set(count);
        Objects::revisions(self.drive, folder)
    }
    fn allocate(&self) -> Result<String> {
        self.drive.allocate_id()
    }
    fn put(&self, folder: &str, id: &str, key: &SpaceKey, bundle: &bundle::Bundle) -> Result<()> {
        // Envelope length is nonce-independent. It excludes HTTP multipart metadata/headers.
        let encrypted_len = key.seal(bundle)?.len() as u64;
        let uploaded = self.drive.upload(folder, id, key, bundle)?;
        if uploaded.id != id {
            return Err("drive_invalid_response".into());
        }
        let mut count = self.counts.get();
        count.upload_objects += 1;
        count.encrypted_payload_bytes_estimate += encrypted_len;
        self.counts.set(count);
        Ok(())
    }
    fn get(&self, folder: &str, id: &str, space: &str, key: &SpaceKey) -> Result<bundle::Bundle> {
        let bundle = self.drive.download(folder, id, space, key)?;
        let mut count = self.counts.get();
        count.download_objects += 1;
        count.decoded_bundle_bytes += bundle.bytes()?.len() as u64;
        self.counts.set(count);
        Ok(bundle)
    }
}

struct TestFolder<'a> {
    drive: &'a Drive,
    id: String,
    name: String,
    active: bool,
}
impl TestFolder<'_> {
    fn trash(&mut self) -> Result<()> {
        self.drive.trash_created_test_folder(&self.id, &self.name)?;
        self.active = false;
        Ok(())
    }
}
impl Drop for TestFolder<'_> {
    fn drop(&mut self) {
        if self.active {
            if let Err(code) = self.drive.trash_created_test_folder(&self.id, &self.name) {
                eprintln!("isolated_test_folder_cleanup_failed:{code}");
            }
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RealPhase {
    name: String,
    elapsed_ms: u128,
    list_calls: u64,
    upload_objects: u64,
    encrypted_payload_bytes_estimate: u64,
    download_objects: u64,
    decoded_bundle_bytes: u64,
    published: usize,
    received: usize,
    restored: usize,
}
fn measure_real(
    name: &str,
    remote: &CountedDrive<'_>,
    action: impl FnOnce() -> Result<SourceStatus>,
) -> (RealPhase, Result<SourceStatus>) {
    let before = remote.counts.get();
    let start = Instant::now();
    let result = action();
    let elapsed_ms = start.elapsed().as_millis();
    let delta = remote.counts.get().delta(before);
    let phase = RealPhase {
        name: name.into(),
        elapsed_ms,
        list_calls: delta.list_calls,
        upload_objects: delta.upload_objects,
        encrypted_payload_bytes_estimate: delta.encrypted_payload_bytes_estimate,
        download_objects: delta.download_objects,
        decoded_bundle_bytes: delta.decoded_bundle_bytes,
        published: result.as_ref().map_or(0, |s| s.published),
        received: result.as_ref().map_or(0, |s| s.received),
        restored: result.as_ref().map_or(0, |s| s.restored),
    };
    (phase, result)
}
fn small_payload(index: usize) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut state = index as u64 + 1;
    let mut text = String::with_capacity(REAL_SESSION_BYTES);
    for _ in 0..REAL_SESSION_BYTES {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        text.push(ALPHABET[(state & 63) as usize] as char);
    }
    text
}
fn session_path(home: &Path, index: usize) -> PathBuf {
    home.join("projects/-bench-project")
        .join(format!("cloud-bench-{index:03}.jsonl"))
}
fn make_fixture(home: &Path) -> u64 {
    fs::create_dir_all(home.join("projects/-bench-project")).unwrap();
    let mut total = 0;
    for index in 0..REAL_SESSIONS {
        let line = serde_json::json!({"type":"user","sessionId":format!("cloud-bench-{index:03}"),"cwd":REAL_CWD,"message":{"role":"user","content":small_payload(index)}});
        let path = session_path(home, index);
        fs::write(&path, format!("{line}\n")).unwrap();
        total += fs::metadata(path).unwrap().len();
    }
    total
}
#[allow(clippy::too_many_arguments)]
fn cloud_cycle(
    root: &Path,
    binding: &Binding,
    key: &SpaceKey,
    remote: &CountedDrive<'_>,
    cache: &Path,
    source: &Path,
    direction: Direction,
    mappings: &[crate::project_mapping::Mapping],
) -> Result<SourceStatus> {
    let cached = queue::CachedObjects::new(remote, cache)?.with_fresh_object(&binding.proof);
    cycle_with_mappings(
        root,
        binding,
        key,
        &cached,
        "claude-code",
        source,
        direction,
        mappings,
        || false,
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RealEvidence {
    kind: &'static str,
    build: &'static str,
    platform: &'static str,
    architecture: &'static str,
    sessions: usize,
    source_bytes: u64,
    setup_elapsed_ms: u128,
    cleanup_elapsed_ms: u128,
    folder_trashed: bool,
    phases: Vec<RealPhase>,
}

/// This is intentionally opt-in. The app's saved wizard is read only; no active binding is used.
#[test]
#[ignore = "requires explicit BASTET_REAL_DRIVE_TEST=1, wizard path, and temporary evidence paths"]
fn isolated_real_drive_small_synthetic_exchange() {
    assert_eq!(std::env::var("BASTET_REAL_DRIVE_TEST").as_deref(), Ok("1"));
    let wizard_path = PathBuf::from(
        std::env::var_os("BASTET_REAL_DRIVE_WIZARD_PATH").expect("BASTET_REAL_DRIVE_WIZARD_PATH"),
    );
    assert!(wizard_path.is_absolute());
    assert_eq!(wizard_path.file_name().unwrap(), "wizard.json");
    let evidence_path = PathBuf::from(
        std::env::var_os("BASTET_REAL_DRIVE_EVIDENCE").expect("BASTET_REAL_DRIVE_EVIDENCE"),
    );
    let receipt_path = PathBuf::from(
        std::env::var_os("BASTET_REAL_DRIVE_RECEIPT").expect("BASTET_REAL_DRIVE_RECEIPT"),
    );
    let wizard: Wizard = serde_json::from_slice(&fs::read(&wizard_path).unwrap()).unwrap();
    assert!(wizard.authorized && wizard.binding.is_some() && wizard.account.is_some());
    let active = wizard.binding.as_ref().unwrap();
    let start_setup = Instant::now();
    let config = wizard_desktop::config(&wizard).unwrap();
    let token = oauth::reconnect(&config, &NativeStore).unwrap();
    let drive = Drive::new(token).unwrap();
    let account = drive.account().unwrap();
    assert!(account.permission_id == wizard.account.as_ref().unwrap().permission_id);
    let id = drive.allocate_id().unwrap();
    assert!(id != active.folder);
    let name = format!("Bastet-test-{}", uuid::Uuid::new_v4());
    // Record the exact new ID before creation, covering even a terminated process.
    let receipt =
        serde_json::to_vec(&serde_json::json!({"folderId":id,"folderName":name,"trashed":false}))
            .unwrap();
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(&receipt_path)
        .unwrap()
        .write_all(&receipt)
        .unwrap();
    let created = match drive.create_folder(&id, &name) {
        Ok(created) => created,
        Err(code) => {
            // An uncertain API response may still have committed the folder.
            if drive.metadata(&id).is_ok_and(|f| f.name == name) {
                let _ = drive.trash_created_test_folder(&id, &name);
            }
            panic!("isolated_test_folder_create_failed:{code}");
        }
    };
    let mut owned = TestFolder {
        drive: &drive,
        id,
        name,
        active: true,
    };
    assert!(
        created.id == owned.id
            && created.name == owned.name
            && created.mime_type == "application/vnd.google-apps.folder"
    );
    let space = uuid::Uuid::new_v4().to_string();
    assert!(space != active.space);
    let key = SpaceKey::generate().unwrap();
    let proof = drive.allocate_id().unwrap();
    let uploaded = drive
        .upload(
            &owned.id,
            &proof,
            &key,
            &queue::proof_bundle(&space).unwrap(),
        )
        .unwrap();
    assert!(uploaded.id == proof);
    let binding = Binding {
        folder: owned.id.clone(),
        space,
        proof,
    };
    let setup_elapsed_ms = start_setup.elapsed().as_millis();

    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("synthetic-claude-source");
    let source_bytes = make_fixture(&source);
    let root_a = temp.path().join("sync-a");
    let root_b = temp.path().join("sync-b");
    let cache_a = temp.path().join("cache-a");
    let cache_b = temp.path().join("cache-b");
    let receiver_project = temp.path().join("receiver-project");
    fs::create_dir_all(&receiver_project).unwrap();
    let remote = CountedDrive::new(&drive);
    let mut phases = vec![];
    let (phase, result) = measure_real("initial_upload", &remote, || {
        cloud_cycle(
            &root_a,
            &binding,
            &key,
            &remote,
            &cache_a,
            &source,
            Direction::Upload,
            &[],
        )
    });
    assert_eq!(result.unwrap().published, REAL_SESSIONS);
    assert_eq!(phase.upload_objects, REAL_SESSIONS as u64);
    phases.push(phase);
    let (phase, result) = measure_real("cache_population", &remote, || {
        cloud_cycle(
            &root_a,
            &binding,
            &key,
            &remote,
            &cache_a,
            &source,
            Direction::Upload,
            &[],
        )
    });
    assert_eq!(result.unwrap().published, 0);
    phases.push(phase);
    let (phase, result) = measure_real("warm_unchanged", &remote, || {
        cloud_cycle(
            &root_a,
            &binding,
            &key,
            &remote,
            &cache_a,
            &source,
            Direction::Upload,
            &[],
        )
    });
    assert_eq!(result.unwrap().published, 0);
    assert_eq!(phase.upload_objects, 0);
    assert_eq!(phase.download_objects, 1, "proof must be fetched fresh");
    phases.push(phase);
    let base = {
        let journal: Journal =
            serde_json::from_slice(&fs::read(root_a.join("native-journal.json")).unwrap()).unwrap();
        journal.bases[&bundle::hash(b"cloud-bench-000")].clone()
    };
    let line = serde_json::json!({"type":"user","sessionId":"cloud-bench-000","cwd":REAL_CWD,"message":{"role":"user","content":"BASTET_REAL_DRIVE_SYNTHETIC_APPEND"}});
    writeln!(
        fs::OpenOptions::new()
            .append(true)
            .open(session_path(&source, 0))
            .unwrap(),
        "{line}"
    )
    .unwrap();
    let (phase, result) = measure_real("append_upload", &remote, || {
        cloud_cycle(
            &root_a,
            &binding,
            &key,
            &remote,
            &cache_a,
            &source,
            Direction::Upload,
            &[],
        )
    });
    assert_eq!(result.unwrap().published, 1);
    assert_eq!(phase.upload_objects, 1);
    let child_id = {
        let journal: Journal =
            serde_json::from_slice(&fs::read(root_a.join("native-journal.json")).unwrap()).unwrap();
        journal.bases[&bundle::hash(b"cloud-bench-000")].clone()
    };
    let all = Replica::open(&root_a.join("replica"), &binding.space)
        .unwrap()
        .transport_bundles()
        .unwrap();
    assert_eq!(all[&child_id].snapshot.parents, vec![base]);
    phases.push(phase);
    let mappings = [crate::project_mapping::Mapping {
        source: REAL_CWD.into(),
        target: receiver_project.to_string_lossy().into_owned(),
    }];
    let (phase, result) = measure_real("receiver_download_restore", &remote, || {
        cloud_cycle(
            &root_b,
            &binding,
            &key,
            &remote,
            &cache_b,
            &temp.path().join("absent-receiver-home"),
            Direction::Download,
            &mappings,
        )
    });
    assert_eq!(result.unwrap().restored, REAL_SESSIONS);
    assert_eq!(load_handoffs(&root_b).unwrap().entries.len(), REAL_SESSIONS);
    phases.push(phase);

    let start_cleanup = Instant::now();
    let cleanup = owned.trash();
    let cleanup_elapsed_ms = start_cleanup.elapsed().as_millis();
    let folder_trashed = cleanup.is_ok();
    fs::write(
        &receipt_path,
        serde_json::to_vec(&serde_json::json!({"folderId":owned.id,"folderName":owned.name,"trashed":folder_trashed})).unwrap(),
    )
    .unwrap();
    let evidence = RealEvidence {
        kind: "isolated_real_google_drive_synthetic_native_sync_core",
        build: "cargo_test_debug",
        platform: std::env::consts::OS,
        architecture: std::env::consts::ARCH,
        sessions: REAL_SESSIONS,
        source_bytes,
        setup_elapsed_ms,
        cleanup_elapsed_ms,
        folder_trashed,
        phases,
    };
    fs::write(
        &evidence_path,
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
    cleanup.unwrap();
    eprintln!("isolated real Drive evidence: {}", evidence_path.display());
}
