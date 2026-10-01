//! Explicit, ignored synthetic workload. It never opens default agent homes or Drive.
use super::*;
use std::{
    cell::{Cell, RefCell},
    io::Write,
    time::Instant,
};

const SESSIONS: usize = 200;
const ORDINARY_BYTES: usize = 64 * 1024;
const LARGE_BYTES: usize = 8 * 1024 * 1024;
const CWD: &str = "/bench/project";

#[derive(Clone, Copy, Default)]
struct Counters {
    put_count: u64,
    put_bytes: u64,
    get_count: u64,
    get_bytes: u64,
}
impl Counters {
    fn delta(self, old: Self) -> Self {
        Self {
            put_count: self.put_count - old.put_count,
            put_bytes: self.put_bytes - old.put_bytes,
            get_count: self.get_count - old.get_count,
            get_bytes: self.get_bytes - old.get_bytes,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fault {
    None,
    BeforePut,
    AfterPut,
}
struct SyntheticRemote {
    objects: RefCell<BTreeMap<String, Vec<u8>>>,
    next: Cell<usize>,
    fault: Cell<Fault>,
    counters: Cell<Counters>,
}
impl SyntheticRemote {
    fn new(key: &SpaceKey) -> Self {
        Self {
            objects: RefCell::new(BTreeMap::from([(
                "proof".into(),
                key.seal(&queue::proof_bundle("bench").unwrap()).unwrap(),
            )])),
            next: Cell::new(0),
            fault: Cell::new(Fault::None),
            counters: Cell::new(Counters::default()),
        }
    }
    fn sample(&self) -> Counters {
        self.counters.get()
    }
}
impl Objects for SyntheticRemote {
    fn ids(&self, _: &str) -> Result<Vec<String>> {
        Ok(self.objects.borrow().keys().cloned().collect())
    }
    fn allocate(&self) -> Result<String> {
        let n = self.next.get();
        self.next.set(n + 1);
        Ok(format!("bench-{n}"))
    }
    fn put(&self, _: &str, id: &str, key: &SpaceKey, bundle: &bundle::Bundle) -> Result<()> {
        let fault = self.fault.replace(Fault::None);
        if fault == Fault::BeforePut {
            return Err("network_unavailable".into());
        }
        if self.objects.borrow().contains_key(id) {
            return Err("drive_id_exists".into());
        }
        let wire = key.seal(bundle)?;
        let mut counters = self.counters.get();
        counters.put_count += 1;
        counters.put_bytes += wire.len() as u64;
        self.counters.set(counters);
        self.objects.borrow_mut().insert(id.into(), wire);
        if fault == Fault::AfterPut {
            Err("network_unavailable".into())
        } else {
            Ok(())
        }
    }
    fn get(&self, _: &str, id: &str, space: &str, key: &SpaceKey) -> Result<bundle::Bundle> {
        let objects = self.objects.borrow();
        let wire = objects.get(id).ok_or("drive_not_found")?;
        let mut counters = self.counters.get();
        counters.get_count += 1;
        counters.get_bytes += wire.len() as u64;
        self.counters.set(counters);
        key.open(space, wire)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Phase {
    name: String,
    elapsed_ms: u128,
    put_count: u64,
    put_bytes: u64,
    get_count: u64,
    get_bytes: u64,
    published: usize,
    received: usize,
    restored: usize,
    remote_objects: usize,
    outcome: String,
}
fn measure(
    name: &str,
    remote: &SyntheticRemote,
    f: impl FnOnce() -> Result<SourceStatus>,
) -> (Phase, Result<SourceStatus>) {
    let before = remote.sample();
    let start = Instant::now();
    let result = f();
    let elapsed_ms = start.elapsed().as_millis();
    let delta = remote.sample().delta(before);
    let phase = Phase {
        name: name.into(),
        elapsed_ms,
        put_count: delta.put_count,
        put_bytes: delta.put_bytes,
        get_count: delta.get_count,
        get_bytes: delta.get_bytes,
        published: result.as_ref().map_or(0, |s| s.published),
        received: result.as_ref().map_or(0, |s| s.received),
        restored: result.as_ref().map_or(0, |s| s.restored),
        remote_objects: remote.objects.borrow().len() - 1,
        outcome: result.as_ref().map_or_else(|e| e.clone(), |_| "ok".into()),
    };
    (phase, result)
}
fn pseudo_text(len: usize, seed: u64) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut state = seed | 1;
    let mut text = String::with_capacity(len);
    for _ in 0..len {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        text.push(ALPHABET[(state & 63) as usize] as char);
    }
    text
}
fn session_id(index: usize) -> String {
    format!("bench-{index:04}")
}
fn source_file(home: &Path, index: usize) -> PathBuf {
    home.join("projects/-bench-project")
        .join(format!("{}.jsonl", session_id(index)))
}
fn create_source(home: &Path) -> u64 {
    fs::create_dir_all(home.join("projects/-bench-project")).unwrap();
    let mut bytes = 0;
    for index in 0..SESSIONS {
        let path = source_file(home, index);
        let id = session_id(index);
        let content = pseudo_text(
            if index == 0 {
                LARGE_BYTES
            } else {
                ORDINARY_BYTES
            },
            index as u64 + 1,
        );
        let line = serde_json::json!({"type":"user","sessionId":id,"cwd":CWD,"message":{"role":"user","content":content}});
        fs::write(&path, format!("{line}\n")).unwrap();
        bytes += fs::metadata(path).unwrap().len();
    }
    bytes
}
fn append_turn(home: &Path, index: usize, marker: &str) {
    let id = session_id(index);
    let line = serde_json::json!({"type":"user","sessionId":id,"cwd":CWD,"message":{"role":"user","content":marker}});
    writeln!(
        fs::OpenOptions::new()
            .append(true)
            .open(source_file(home, index))
            .unwrap(),
        "{line}"
    )
    .unwrap();
}
fn base_for(root: &Path, index: usize) -> String {
    let journal: Journal =
        serde_json::from_slice(&fs::read(root.join("native-journal.json")).unwrap()).unwrap();
    journal.bases[&bundle::hash(session_id(index).as_bytes())].clone()
}
fn assert_child(root: &Path, index: usize, parent: &str) {
    let id = base_for(root, index);
    let all = Replica::open(&root.join("replica"), "bench")
        .unwrap()
        .transport_bundles()
        .unwrap();
    assert_eq!(all[&id].snapshot.parents, vec![parent]);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Evidence {
    kind: &'static str,
    platform: &'static str,
    architecture: &'static str,
    sessions: usize,
    ordinary_session_bytes: usize,
    large_session_bytes: usize,
    source_bytes: u64,
    phases: Vec<Phase>,
}

#[test]
#[ignore = "explicit synthetic volume workload; set BASTET_PERF_OUTPUT to a temporary JSON path"]
fn synthetic_volume_initial_warm_append_offline_retry_and_receive() {
    let output = PathBuf::from(std::env::var_os("BASTET_PERF_OUTPUT").expect("BASTET_PERF_OUTPUT"));
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("source");
    let source_bytes = create_source(&home);
    let root = temp.path().join("sync-a");
    let receiver_root = temp.path().join("sync-b");
    let receiver_home = temp.path().join("empty-receiver-home");
    let receiver_project = temp.path().join("receiver-project");
    fs::create_dir_all(&receiver_project).unwrap();
    let binding = Binding {
        folder: "bench-folder".into(),
        space: "bench".into(),
        proof: "proof".into(),
    };
    let key = SpaceKey::generate().unwrap();
    let remote = SyntheticRemote::new(&key);
    let mut phases = vec![];
    let (phase, result) = measure("initial_upload", &remote, || {
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
    });
    let result = result.unwrap();
    assert_eq!(result.published, SESSIONS);
    assert_eq!(phase.put_count, SESSIONS as u64);
    phases.push(phase);
    let base = base_for(&root, 0);

    let (phase, result) = measure("warm_unchanged", &remote, || {
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
    });
    assert_eq!(result.unwrap().published, 0);
    assert_eq!(phase.put_count, 0);
    phases.push(phase);

    append_turn(&home, 0, "BASTET_VOLUME_APPEND");
    let (phase, result) = measure("append_upload", &remote, || {
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
    });
    assert_eq!(result.unwrap().published, 1);
    assert_eq!(phase.put_count, 1);
    assert_child(&root, 0, &base);
    phases.push(phase);

    append_turn(&home, 1, "BASTET_VOLUME_OFFLINE");
    remote.fault.set(Fault::BeforePut);
    let (phase, result) = measure("offline_before_put", &remote, || {
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
    });
    assert_eq!(result.err().unwrap(), "network_unavailable");
    assert_eq!(phase.put_count, 0);
    phases.push(phase);
    let (phase, result) = measure("restart_retry", &remote, || {
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
    });
    assert_eq!(result.unwrap().published, 1);
    assert_eq!(phase.put_count, 1);
    phases.push(phase);

    append_turn(&home, 2, "BASTET_VOLUME_AMBIGUOUS");
    remote.fault.set(Fault::AfterPut);
    let (phase, result) = measure("ambiguous_commit", &remote, || {
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
    });
    assert_eq!(result.err().unwrap(), "network_unavailable");
    assert_eq!(phase.put_count, 1);
    phases.push(phase);
    let (phase, result) = measure("restart_after_ambiguous_commit", &remote, || {
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
    });
    assert_eq!(result.unwrap().published, 0);
    assert_eq!(phase.put_count, 0);
    phases.push(phase);

    let mapping = [crate::project_mapping::Mapping {
        source: CWD.into(),
        target: receiver_project.to_string_lossy().into_owned(),
    }];
    let (phase, result) = measure("receiver_download_restore", &remote, || {
        cycle_with_mappings(
            &receiver_root,
            &binding,
            &key,
            &remote,
            "claude-code",
            &receiver_home,
            Direction::Download,
            &mapping,
            || false,
        )
    });
    let result = result.unwrap();
    assert_eq!(result.restored, SESSIONS, "{:?}", result.issues);
    assert_eq!(
        load_handoffs(&receiver_root).unwrap().entries.len(),
        SESSIONS
    );
    let restored = load_handoffs(&receiver_root)
        .unwrap()
        .entries
        .into_iter()
        .find(|h| h.session == session_id(0))
        .unwrap();
    let restored_file = restored.path.join(&restored.main_file);
    assert!(fs::read_to_string(restored_file)
        .unwrap()
        .contains("BASTET_VOLUME_APPEND"));
    phases.push(phase);

    let evidence = Evidence {
        kind: "isolated_synthetic_native_sync_core",
        platform: std::env::consts::OS,
        architecture: std::env::consts::ARCH,
        sessions: SESSIONS,
        ordinary_session_bytes: ORDINARY_BYTES,
        large_session_bytes: LARGE_BYTES,
        source_bytes,
        phases,
    };
    fs::write(&output, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    eprintln!("synthetic volume evidence: {}", output.display());
}
