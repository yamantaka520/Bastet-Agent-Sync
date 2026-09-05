use super::*;
fn stream() -> Stream {
    Stream {
        agent: "codex".into(),
        profile: "primary".into(),
        conversation: "test".into(),
    }
}
fn files(text: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("session.jsonl".into(), text.into())])
}
fn put(remote: &LocalTransport, b: &Bundle) {
    fs::write(
        remote.objects().join(format!("{}.json", b.id)),
        b.bytes().unwrap(),
    )
    .unwrap();
}
fn make(space: &str, parent: Vec<String>, text: &str) -> Bundle {
    Bundle::new(Snapshot {
        schema: 1,
        space: space.into(),
        device: "remote".into(),
        stream: stream(),
        parents: parent,
        files: files(text)
            .into_iter()
            .map(|(p, c)| (p, Entry::new(c)))
            .collect(),
    })
    .unwrap()
}

#[test]
fn isolated_end_to_end() {
    assert!(diagnostic::run().unwrap().verified);
}
#[test]
fn receive_then_export_unchanged_does_not_loop() {
    let t = tempfile::tempdir().unwrap();
    let remote = LocalTransport::create(&t.path().join("drive")).unwrap();
    let a = Replica::open(&t.path().join("a"), &remote.space).unwrap();
    let b = Replica::open(&t.path().join("b"), &remote.space).unwrap();
    let id = a.export(stream(), files("one")).unwrap();
    a.sync(&remote, Direction::Both).unwrap();
    b.sync(&remote, Direction::Both).unwrap();
    assert_eq!(b.export(stream(), files("one")).unwrap(), id);
    for _ in 0..3 {
        let r = b.sync(&remote, Direction::Both).unwrap();
        assert_eq!(r.published + r.received, 0);
        assert_eq!(b.checkpoint().unwrap().objects.len(), 1);
    }
}
#[test]
fn child_before_parent_is_pending_then_recovers() {
    let t = tempfile::tempdir().unwrap();
    let remote = LocalTransport::create(&t.path().join("drive")).unwrap();
    let b = Replica::open(&t.path().join("b"), &remote.space).unwrap();
    let parent = make(&remote.space, vec![], "base");
    let child = make(&remote.space, vec![parent.id.clone()], "next");
    put(&remote, &child);
    let r = b.sync(&remote, Direction::Download).unwrap();
    assert_eq!(r.pending, 1);
    assert!(b.checkpoint().unwrap().streams.is_empty());
    put(&remote, &parent);
    let r = b.sync(&remote, Direction::Download).unwrap();
    assert_eq!(r.pending, 0);
    assert_eq!(b.checkpoint().unwrap().streams[0].ids, vec![child.id]);
}
#[test]
fn incomplete_and_corrupt_remote_files_are_not_applied() {
    let t = tempfile::tempdir().unwrap();
    let remote = LocalTransport::create(&t.path().join("drive")).unwrap();
    let b = Replica::open(&t.path().join("b"), &remote.space).unwrap();
    let valid = make(&remote.space, vec![], "one");
    let path = remote.objects().join(format!("{}.json", valid.id));
    fs::write(&path, b"{\"id\":").unwrap();
    fs::write(remote.objects().join(".tmp-incomplete"), b"partial").unwrap();
    let r = b.sync(&remote, Direction::Download).unwrap();
    assert_eq!(r.received, 0);
    assert_eq!(r.issues.len(), 1);
    assert!(b.checkpoint().unwrap().objects.is_empty());
    fs::write(&path, valid.bytes().unwrap()).unwrap();
    assert_eq!(b.sync(&remote, Direction::Download).unwrap().received, 1);
}
#[test]
fn immutable_orphans_recover_after_journal_loss() {
    let t = tempfile::tempdir().unwrap();
    let remote = LocalTransport::create(&t.path().join("drive")).unwrap();
    let root = t.path().join("b");
    let b = Replica::open(&root, &remote.space).unwrap();
    let id = b.export(stream(), files("one")).unwrap();
    drop(b);
    fs::write(root.join("journal.json"), b"incomplete checkpoint").unwrap();
    let reopened = Replica::open(&root, &remote.space).unwrap();
    assert_eq!(reopened.checkpoint().unwrap().objects, vec![id]);
}
#[test]
fn concurrent_branches_require_explicit_fresh_resolution() {
    let t = tempfile::tempdir().unwrap();
    let remote = LocalTransport::create(&t.path().join("drive")).unwrap();
    let a = Replica::open(&t.path().join("a"), &remote.space).unwrap();
    let b = Replica::open(&t.path().join("b"), &remote.space).unwrap();
    a.export(stream(), files("A")).unwrap();
    b.export(stream(), files("B")).unwrap();
    a.sync(&remote, Direction::Both).unwrap();
    b.sync(&remote, Direction::Both).unwrap();
    a.sync(&remote, Direction::Both).unwrap();
    let heads = a.checkpoint().unwrap().streams[0].ids.clone();
    assert_eq!(
        a.export(stream(), files("C")).unwrap_err(),
        "conflict_requires_resolution"
    );
    assert_eq!(
        a.resolve(stream(), files("C"), vec![heads[0].clone()])
            .unwrap_err(),
        "stale_resolution"
    );
    a.resolve(stream(), files("C"), heads).unwrap();
    let j = a.checkpoint().unwrap();
    assert_eq!(j.objects.len(), 3);
    assert_eq!(j.streams[0].ids.len(), 1);
}
#[test]
fn different_space_rejected_and_no_destination_created() {
    let t = tempfile::tempdir().unwrap();
    let one = LocalTransport::create(&t.path().join("one")).unwrap();
    let two = LocalTransport::create(&t.path().join("two")).unwrap();
    let a = Replica::open(&t.path().join("a"), &one.space).unwrap();
    a.export(stream(), files("data")).unwrap();
    assert_eq!(a.sync(&two, Direction::Both).unwrap_err(), "space_mismatch");
    assert_eq!(fs::read_dir(two.objects()).unwrap().count(), 0);
    let absent = t.path().join("absent");
    assert!(LocalTransport::connect(&absent).is_err());
    assert!(!absent.exists());
}
#[test]
fn deleting_remote_objects_never_deletes_local_history() {
    let t = tempfile::tempdir().unwrap();
    let remote = LocalTransport::create(&t.path().join("drive")).unwrap();
    let a = Replica::open(&t.path().join("a"), &remote.space).unwrap();
    let id = a.export(stream(), files("one")).unwrap();
    a.sync(&remote, Direction::Both).unwrap();
    fs::remove_file(remote.objects().join(format!("{id}.json"))).unwrap();
    a.sync(&remote, Direction::Download).unwrap();
    assert_eq!(a.checkpoint().unwrap().objects, vec![id]);
}
#[test]
fn source_change_and_sensitive_filename_rejected() {
    let t = tempfile::tempdir().unwrap();
    fs::write(t.path().join("session.jsonl"), "before").unwrap();
    let paths = vec!["session.jsonl".into()];
    assert_eq!(
        capture_checked(t.path(), &paths, || {
            fs::write(t.path().join("session.jsonl"), "after").unwrap();
        })
        .unwrap_err(),
        "source_changed"
    );
    assert_eq!(
        capture(t.path(), &["auth.json".into()]).unwrap_err(),
        "excluded_file"
    );
    assert!(capture(t.path(), &["../secret.txt".into()]).is_err());
}
#[test]
fn malicious_paths_and_hashes_rejected() {
    let base = make("space", vec![], "content");
    for name in [
        "../escape",
        "/absolute",
        "C:/file",
        "a\\b",
        "a/../b",
        "CON.txt",
        "LPT1.md",
        "foo.",
        "foo//bar",
    ] {
        let mut s = base.snapshot.clone();
        s.files = BTreeMap::from([(name.into(), Entry::new("test".into()))]);
        assert!(Bundle::new(s).is_err(), "{name}");
    }
    let mut bad = base.clone();
    bad.snapshot.files.get_mut("session.jsonl").unwrap().content = "tamper".into();
    assert!(bad.validate().is_err());
    let mut s = base.snapshot.clone();
    s.files = BTreeMap::from([
        ("x.md".into(), Entry::new("x".into())),
        ("X.md".into(), Entry::new("y".into())),
    ]);
    assert!(Bundle::new(s).is_err());
    let mut s = base.snapshot.clone();
    s.files = BTreeMap::from([
        ("a".into(), Entry::new("x".into())),
        ("a/b".into(), Entry::new("y".into())),
    ]);
    assert!(Bundle::new(s).is_err());
    let mut s = base.snapshot.clone();
    s.schema = 999;
    assert!(Bundle::new(s).is_err());
    let mut s = base.snapshot;
    s.files.get_mut("session.jsonl").unwrap().content = "x".repeat(bundle::MAX_FILE + 1);
    assert!(Bundle::new(s).is_err());
}
#[test]
fn parent_cannot_cross_conversation() {
    let t = tempfile::tempdir().unwrap();
    let remote = LocalTransport::create(&t.path().join("drive")).unwrap();
    let b = Replica::open(&t.path().join("b"), &remote.space).unwrap();
    let parent = make(&remote.space, vec![], "parent");
    let mut child = make(&remote.space, vec![parent.id.clone()], "child").snapshot;
    child.stream.conversation = "other".into();
    let child = Bundle::new(child).unwrap();
    put(&remote, &parent);
    put(&remote, &child);
    let r = b.sync(&remote, Direction::Download).unwrap();
    assert_eq!(r.pending, 1);
    assert_eq!(r.issues[0].code, "invalid_parent");
    assert_eq!(b.checkpoint().unwrap().streams.len(), 1);
}
#[test]
fn exclusive_replica_lock_is_released_on_drop() {
    let t = tempfile::tempdir().unwrap();
    let a = Replica::open(t.path(), "space").unwrap();
    assert!(matches!(Replica::open(t.path(),"space"),Err(e) if e=="sync_busy"));
    drop(a);
    assert!(Replica::open(t.path(), "space").is_ok());
}
#[cfg(unix)]
#[test]
fn symlink_sources_and_remote_objects_rejected() {
    use std::os::unix::fs::symlink;
    let t = tempfile::tempdir().unwrap();
    let outside = t.path().join("outside.txt");
    fs::write(&outside, "private").unwrap();
    let source = t.path().join("source");
    fs::create_dir(&source).unwrap();
    symlink(&outside, source.join("session.txt")).unwrap();
    assert_eq!(
        capture(&source, &["session.txt".into()]).unwrap_err(),
        "unsafe_source"
    );
    let remote = LocalTransport::create(&t.path().join("drive")).unwrap();
    let b = Replica::open(&t.path().join("b"), &remote.space).unwrap();
    symlink(
        &outside,
        remote.objects().join(format!("{}.json", "a".repeat(64))),
    )
    .unwrap();
    let r = b.sync(&remote, Direction::Download).unwrap();
    assert_eq!(r.received, 0);
    assert_eq!(r.issues[0].code, "unsafe_store");
    assert_eq!(fs::read_to_string(outside).unwrap(), "private");
}

#[test]
fn direction_modes_do_not_leak_opposite_traffic() {
    let t = tempfile::tempdir().unwrap();
    let remote = LocalTransport::create(&t.path().join("drive")).unwrap();
    let a = Replica::open(&t.path().join("a"), &remote.space).unwrap();
    let b = Replica::open(&t.path().join("b"), &remote.space).unwrap();
    let a_id = a.export(stream(), files("A")).unwrap();
    let b_id = b.export(stream(), files("B")).unwrap();
    a.sync(&remote, Direction::Upload).unwrap();
    let r = b.sync(&remote, Direction::Download).unwrap();
    assert_eq!(r.published, 0);
    assert_eq!(r.received, 1);
    assert!(!remote.objects().join(format!("{b_id}.json")).exists());
    b.sync(&remote, Direction::Upload).unwrap();
    assert_eq!(a.checkpoint().unwrap().objects, vec![a_id]);
}
#[test]
fn oversized_wire_and_file_lists_are_rejected() {
    let t = tempfile::tempdir().unwrap();
    let remote = LocalTransport::create(&t.path().join("drive")).unwrap();
    let a = Replica::open(&t.path().join("a"), &remote.space).unwrap();
    let f = File::create(remote.objects().join(format!("{}.json", "b".repeat(64)))).unwrap();
    f.set_len(MAX_WIRE + 1).unwrap();
    let r = a.sync(&remote, Direction::Download).unwrap();
    assert_eq!(r.received, 0);
    assert_eq!(r.issues[0].code, "bundle_limit");
    let many = (0..257)
        .map(|i| (format!("f{i}.txt"), String::new()))
        .collect();
    assert_eq!(a.export(stream(), many).unwrap_err(), "bundle_limit");
    assert!(a.checkpoint().unwrap().objects.is_empty());
}
#[test]
fn unicode_content_survives_the_round_trip() {
    let t = tempfile::tempdir().unwrap();
    let remote = LocalTransport::create(&t.path().join("drive")).unwrap();
    let a = Replica::open(&t.path().join("a"), &remote.space).unwrap();
    let b = Replica::open(&t.path().join("b"), &remote.space).unwrap();
    let text = "繁體中文 · 简体中文 · 日本語 · 한국어 · 🐈\n";
    let id = a.export(stream(), files(text)).unwrap();
    a.sync(&remote, Direction::Both).unwrap();
    b.sync(&remote, Direction::Both).unwrap();
    assert_eq!(
        b.read_all().unwrap().0[&id].snapshot.files["session.jsonl"].content,
        text
    );
}
#[test]
fn invalid_parent_is_not_republished_from_quarantine() {
    let t = tempfile::tempdir().unwrap();
    let remote = LocalTransport::create(&t.path().join("drive")).unwrap();
    let a = Replica::open(&t.path().join("a"), &remote.space).unwrap();
    let orphan = make(&remote.space, vec!["c".repeat(64)], "orphan");
    put(&remote, &orphan);
    a.sync(&remote, Direction::Download).unwrap();
    fs::remove_file(remote.objects().join(format!("{}.json", orphan.id))).unwrap();
    let r = a.sync(&remote, Direction::Upload).unwrap();
    assert_eq!(r.published, 0);
    assert_eq!(r.pending, 1);
}

#[test]
fn receiving_remote_update_does_not_rebase_existing_local_edits() {
    let t = tempfile::tempdir().unwrap();
    let remote = LocalTransport::create(&t.path().join("drive")).unwrap();
    let a = Replica::open(&t.path().join("a"), &remote.space).unwrap();
    let b = Replica::open(&t.path().join("b"), &remote.space).unwrap();
    let base = a.export_from(stream(), files("base"), None).unwrap();
    a.sync(&remote, Direction::Both).unwrap();
    b.sync(&remote, Direction::Both).unwrap();
    let next = a
        .export_from(stream(), files("remote edit"), Some(&base))
        .unwrap();
    a.sync(&remote, Direction::Upload).unwrap();
    // B's staged edit still depends on base, even though the inbox now knows next.
    b.sync(&remote, Direction::Download).unwrap();
    let branch = b
        .export_from(stream(), files("offline local edit"), Some(&base))
        .unwrap();
    let j = b.checkpoint().unwrap();
    assert_eq!(j.streams[0].ids.len(), 2);
    assert!(j.streams[0].ids.contains(&next));
    assert!(j.streams[0].ids.contains(&branch));
    assert_eq!(
        b.read_all().unwrap().0[&branch].snapshot.parents,
        vec![base.clone()]
    );
    assert_eq!(
        b.export_from(stream(), files("base"), Some(&base)).unwrap(),
        base
    );
    assert_eq!(
        b.export_from(stream(), files("bad"), Some(&"a".repeat(64)))
            .unwrap_err(),
        "unknown_baseline"
    );
    assert_eq!(b.checkpoint().unwrap().objects.len(), 3);
}
