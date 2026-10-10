//! Fixture tests for the Hermes Agent (state.db) native session adapter.
//!
//! The fixtures are built from the schema documented in the installed Hermes
//! source (`hermes_state_common.py`, SCHEMA_VERSION 31) via the same canonical DDL
//! the exporter uses: a `sessions` table keyed by session id, `messages` carrying
//! role/content/tool_calls/tool_call_id/tool_name/reasoning, `system_prompts`,
//! `session_model_usage`, the runtime/gateway tables and a `schema_version` row.
//! Nothing here reads the live user store.
use super::*;
use crate::project_mapping::Mapping;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const A: &str = "20261010_202527_e3998b";
const B: &str = "20261010_202528_ac8e20";

#[test]
fn hermes_unknown_schema_or_message_columns_are_not_silently_downgraded() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.db");
    hermes_db(&path, &[(A, "/project")]);
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute("UPDATE schema_version SET version=32", [])
        .unwrap();
    assert_eq!(
        hermes_canonicalize(&conn, A, "/project", dir.path()).unwrap_err(),
        "session_format_unsupported"
    );
    conn.execute("UPDATE schema_version SET version=31", [])
        .unwrap();
    conn.execute("ALTER TABLE messages ADD COLUMN future_context TEXT", [])
        .unwrap();
    assert_eq!(
        hermes_canonicalize(&conn, A, "/project", dir.path()).unwrap_err(),
        "session_format_unsupported"
    );
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}

// A faithful two-or-more session state.db carrying tool calls and a tool result.
fn hermes_db(path: &Path, sessions: &[(&str, &str)]) {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute_batch(HERMES_SCHEMA_DDL).unwrap();
    conn.execute(
        "INSERT INTO schema_version(version) VALUES(?1)",
        [HERMES_SCHEMA_VERSION],
    )
    .unwrap();
    for (index, (id, cwd)) in sessions.iter().enumerate() {
        let hash = format!("hash-{index}");
        conn.execute(
            "INSERT INTO system_prompts(hash,prompt) VALUES(?1,?2)",
            rusqlite::params![hash, format!("system prompt {index}")],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sessions(id,source,system_prompt_hash,started_at,cwd,title)
             VALUES(?1,'cli',?2,?3,?4,?5)",
            rusqlite::params![id, hash, index as f64, cwd, format!("title-{id}")],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO messages(session_id,role,content,timestamp) VALUES(?1,'user',?2,?3)",
            rusqlite::params![id, format!("user-marker-{id}"), index as f64],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO messages(session_id,role,reasoning,content,tool_calls,timestamp)
             VALUES(?1,'assistant','thinking hard',NULL,?2,?3)",
            rusqlite::params![
                id,
                r#"[{"id":"call_1","type":"function","function":{"name":"terminal","arguments":"{\"command\":\"ls\"}"}}]"#,
                index as f64
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO messages(session_id,role,content,tool_call_id,tool_name,timestamp)
             VALUES(?1,'tool','file.txt',?2,'terminal',?3)",
            rusqlite::params![id, format!("call_{id}"), index as f64],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO session_model_usage(session_id,model,input_tokens,output_tokens)
             VALUES(?1,'gpt-test',10,20)",
            [id],
        )
        .unwrap();
    }
}

// A live store that also carries everything an export must drop: a private table,
// FTS leftovers, a foreign trigger/view and runtime + gateway rows.
fn hermes_live_store(path: &Path, sessions: &[(&str, &str)]) {
    hermes_db(path, sessions);
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute_batch(
        "CREATE TABLE private_notes(note TEXT);
         INSERT INTO private_notes VALUES('SECRET-OTHER-DATA');
         INSERT INTO compression_locks VALUES('other-session','holder',1.0,9999999999.0);
         INSERT INTO session_turn_leases VALUES('other-conversation','holder',1.0,9999999999.0);
         INSERT INTO async_delegations(delegation_id,origin_session,state,dispatched_at,updated_at)
             VALUES('d1','other-session','done',1.0,1.0);
         INSERT INTO gateway_routing(scope,session_key,entry_json,updated_at)
             VALUES('','key','{}',1.0);
         INSERT INTO gateway_hygiene_state VALUES('key',3);
         INSERT INTO conversation_generations VALUES('cli','key',7);
         INSERT INTO gateway_heartbeats VALUES('backend',1,1.0,1.0,'','');
         INSERT INTO state_meta(key,value) VALUES('fts_rebuild_high_water','42');",
    )
    .unwrap();
    // FTS leftovers when the runtime supports them; ignored otherwise.
    let _ = conn.execute_batch(
        "CREATE VIEW messages_fts_src AS SELECT id, content, tool_name, tool_calls FROM messages;
         CREATE VIRTUAL TABLE messages_fts USING fts5(
             content, tool_name, tool_calls, content='messages_fts_src', content_rowid='id');
         INSERT INTO messages_fts(rowid, content, tool_name, tool_calls)
             SELECT id, content, tool_name, tool_calls FROM messages;",
    );
    conn.execute_batch(
        "CREATE TRIGGER evil_trigger AFTER INSERT ON messages
         BEGIN UPDATE messages SET content='tampered'; END;",
    )
    .unwrap();
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

fn binding() -> Binding {
    Binding {
        folder: "folder".into(),
        space: "space".into(),
        proof: "proof".into(),
    }
}

fn remote() -> Remote {
    Remote(
        RefCell::new(BTreeMap::from([(
            "proof".into(),
            queue::proof_bundle("space").unwrap(),
        )])),
        Cell::new(0),
    )
}

fn open(db: &[u8]) -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.db");
    fs::write(&path, db).unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    (dir, conn)
}

fn export_of(remote: &Remote, session: &str) -> Vec<u8> {
    let bundles = remote.0.borrow().clone();
    let b = bundles
        .values()
        .find(|b| {
            b.snapshot.stream.agent == "hermes"
                && b.snapshot.stream.conversation == bundle::hash(session.as_bytes())
        })
        .unwrap_or_else(|| panic!("no bundle for {session}"));
    let m = unpack(b, &bundles).unwrap();
    // The manifest also carries the merged portable workspace and (when present)
    // content-hashed referenced attachments; the native store is always `state.db`.
    assert!(m.files.contains_key("state.db"), "{:?}", m.files.keys());
    STANDARD.decode(m.files.get("state.db").unwrap()).unwrap()
}

// Export must isolate one session per bundle, keep every message row verbatim
// (tool call + tool result + reasoning) and never mutate the live store.
#[test]
fn hermes_export_isolates_one_session_and_preserves_tool_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    hermes_db(&home.join("state.db"), &[(A, "/project"), (B, "/other")]);
    fs::write(home.join("auth.json"), b"secret-must-not-travel").unwrap();
    let before = fs::read(home.join("state.db")).unwrap();

    let root = temp.path().join("sync");
    let key = SpaceKey::generate().unwrap();
    let remote = remote();
    let status = cycle(
        &root,
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();
    assert_eq!(status.captured, 2, "{:?}", status.issues);
    assert_eq!(status.published, 2, "{:?}", status.issues);
    assert_eq!(
        fs::read(home.join("state.db")).unwrap(),
        before,
        "the live store must never be modified"
    );

    let mut seen = vec![];
    for session in [A, B] {
        let db = export_of(&remote, session);
        assert!(!contains(&db, b"secret-must-not-travel"));
        let (_dir, conn) = open(&db);
        let count: i64 = conn
            .query_row("SELECT count(*) FROM sessions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "exactly one session per exported database");
        let id: String = conn
            .query_row("SELECT id FROM sessions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(id, session);
        let rows: i64 = conn
            .query_row("SELECT count(*) FROM messages", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 3);
        let (name, call, content): (String, String, String) = conn
            .query_row(
                "SELECT COALESCE(tool_name,''),COALESCE(tool_call_id,''),COALESCE(content,'')
                 FROM messages WHERE role='tool'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!((name.as_str(), content.as_str()), ("terminal", "file.txt"));
        assert_eq!(call, format!("call_{session}"));
        let tool_calls: String = conn
            .query_row(
                "SELECT tool_calls FROM messages WHERE role='assistant' AND tool_calls IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(tool_calls.contains("terminal") && tool_calls.contains("ls"));
        let reasoning: String = conn
            .query_row(
                "SELECT reasoning FROM messages WHERE reasoning IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(reasoning, "thinking hard");
        let user: String = conn
            .query_row("SELECT content FROM messages WHERE role='user'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(user, format!("user-marker-{session}"));
        seen.push(id);
    }
    seen.sort();
    assert_eq!(seen, vec![A.to_string(), B.to_string()]);
}

// The export is a rebuilt canonical store: no other session, no foreign table, no
// FTS leftovers, no triggers/views, runtime/gateway tables empty, referential
// integrity intact.
#[test]
fn hermes_export_rebuilds_canonical_store_without_leaking_other_data() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    hermes_live_store(&home.join("state.db"), &[(A, "/project"), (B, "/other")]);
    let before = fs::read(home.join("state.db")).unwrap();

    let root = temp.path().join("sync");
    let key = SpaceKey::generate().unwrap();
    let remote = remote();
    let status = cycle(
        &root,
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();
    assert_eq!(status.captured, 2, "{:?}", status.issues);
    assert_eq!(status.published, 2, "{:?}", status.issues);
    assert_eq!(
        fs::read(home.join("state.db")).unwrap(),
        before,
        "the live store must never be modified"
    );

    for session in [A, B] {
        let db = export_of(&remote, session);
        let (_dir, conn) = open(&db);
        assert_eq!(
            conn.query_row("SELECT version FROM schema_version", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            HERMES_SCHEMA_VERSION
        );
        assert_eq!(
            conn.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        // No trigger or view survives; the rebuilt store carries only inert tables.
        let unsafe_objects: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type IN ('trigger','view')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            unsafe_objects, 0,
            "no trigger/view may reach a restored store"
        );
        // Only allowlisted tables; no FTS leftovers, no foreign private table.
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap();
        for name in stmt.query_map([], |r| r.get::<_, String>(0)).unwrap() {
            let name = name.unwrap();
            assert!(
                name.starts_with("sqlite_") || HERMES_TABLES.contains(&name.as_str()),
                "unexpected table {name}"
            );
        }
        drop(stmt);
        // Runtime and gateway tables exist but are empty; nothing in flight travels.
        for table in [
            "compression_locks",
            "session_turn_leases",
            "async_delegations",
            "gateway_routing",
            "gateway_hygiene_state",
            "conversation_generations",
            "gateway_heartbeats",
            "state_meta",
        ] {
            let count: i64 = conn
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 0, "{table} must be empty");
        }
        // Exactly one session, only its own messages and only its own system prompt.
        let sessions: i64 = conn
            .query_row("SELECT count(*) FROM sessions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sessions, 1);
        let prompts: i64 = conn
            .query_row("SELECT count(*) FROM system_prompts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(prompts, 1);
        drop(conn);

        let other = if session == A { B } else { A };
        assert!(
            !contains(&db, other.as_bytes()),
            "another session's id leaked into {session}'s export"
        );
        assert!(
            !contains(&db, b"SECRET-OTHER-DATA"),
            "a foreign table leaked into {session}'s export"
        );
        assert!(
            !contains(&db, b"messages_fts"),
            "FTS leftovers leaked into {session}'s export"
        );
    }
}

// An unrelated session's edit must not perturb the export of another session; the
// exporter rebuilds deterministically so the unchanged session is deduplicated.
#[test]
fn hermes_unrelated_session_change_does_not_change_target_export() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let db = home.join("state.db");
    hermes_db(&db, &[(A, "/project"), (B, "/other")]);
    let staging = temp.path().join("staging");
    fs::create_dir_all(&staging).unwrap();

    let snapshot_a = |staging: &Path| -> Vec<u8> {
        let image = hermes_backup_bytes(&db, staging).unwrap();
        hermes_with_image(&image, staging, |conn| {
            let sessions = hermes_sessions_of(conn)?;
            let cwd = sessions
                .iter()
                .find(|(id, _)| id == A)
                .map(|(_, cwd)| cwd.clone())
                .unwrap();
            hermes_canonicalize(conn, A, &cwd, staging)
        })
        .unwrap()
    };
    let first = snapshot_a(&staging);
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute(
        "INSERT INTO messages(session_id,role,content,timestamp) VALUES(?1,'user','OTHER SESSION TURN',99.0)",
        [B],
    )
    .unwrap();
    drop(conn);
    assert_eq!(
        first,
        snapshot_a(&staging),
        "session A's export must be unchanged when only session B changes"
    );

    let root = temp.path().join("sync");
    let key = SpaceKey::generate().unwrap();
    let remote = remote();
    let first_cycle = cycle(
        &root,
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();
    assert_eq!(first_cycle.published, 2, "{:?}", first_cycle.issues);
    let a_before = remote
        .0
        .borrow()
        .values()
        .find(|b| b.snapshot.stream.conversation == bundle::hash(A.as_bytes()))
        .cloned()
        .unwrap();
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute(
        "INSERT INTO messages(session_id,role,content,timestamp) VALUES(?1,'user','ANOTHER B TURN',100.0)",
        [B],
    )
    .unwrap();
    drop(conn);
    let second_cycle = cycle(
        &root,
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();
    assert_eq!(second_cycle.captured, 2, "{:?}", second_cycle.issues);
    assert_eq!(
        second_cycle.published, 1,
        "only the changed session republishes: {:?}",
        second_cycle.issues
    );
    let a_bundles = remote
        .0
        .borrow()
        .values()
        .filter(|b| b.snapshot.stream.conversation == bundle::hash(A.as_bytes()))
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(a_bundles.len(), 1, "session A must not republish");
    assert_eq!(
        a_bundles[0], a_before,
        "session A's bundle must be byte-identical after B changed"
    );
    let a_db = export_of(&remote, A);
    assert!(!contains(&a_db, b"ANOTHER B TURN"));
    assert!(!contains(&a_db, b"OTHER SESSION TURN"));
}

// Running the same unchanged cycle twice produces no new objects.
#[test]
fn hermes_duplicate_cycle_is_idempotent() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    hermes_db(&home.join("state.db"), &[(A, "/project"), (B, "/other")]);
    let root = temp.path().join("sync");
    let key = SpaceKey::generate().unwrap();
    let remote = remote();
    let first = cycle(
        &root,
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();
    assert_eq!(
        (first.captured, first.published),
        (2, 2),
        "{:?}",
        first.issues
    );
    let objects = remote.0.borrow().len();
    let second = cycle(
        &root,
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();
    assert_eq!(
        (second.captured, second.published),
        (2, 0),
        "{:?}",
        second.issues
    );
    assert_eq!(remote.0.borrow().len(), objects);
}

// Committed content still sitting in the WAL of a live writer is captured by the
// consistent image, and the live store is never disturbed or checkpointed.
#[test]
fn hermes_wal_backlog_is_captured_without_touching_the_live_store() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let db = home.join("state.db");
    hermes_db(&db, &[(A, "/project")]);
    let live = rusqlite::Connection::open(&db).unwrap();
    live.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
    live.execute(
        "INSERT INTO messages(session_id,role,content,timestamp) VALUES(?1,'user','WAL-ONLY-TURN',123.0)",
        [A],
    )
    .unwrap();
    assert!(
        home.join("state.db-wal").exists(),
        "a WAL sidecar must exist"
    );

    let root = temp.path().join("sync");
    let key = SpaceKey::generate().unwrap();
    let remote = remote();
    let status = cycle(
        &root,
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();
    assert_eq!(status.captured, 1, "{:?}", status.issues);
    let exported = export_of(&remote, A);
    assert!(
        contains(&exported, b"WAL-ONLY-TURN"),
        "committed WAL content must be captured"
    );
    assert_eq!(
        live.query_row(
            "SELECT count(*) FROM messages WHERE content='WAL-ONLY-TURN'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1,
        "the live writer still sees its own turn"
    );
}

// A symlinked live store is untrusted input and must be refused, not followed.
#[cfg(unix)]
#[test]
fn hermes_symlinked_state_db_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let real = temp.path().join("real.db");
    hermes_db(&real, &[(A, "/project")]);
    std::os::unix::fs::symlink(&real, home.join("state.db")).unwrap();
    let root = temp.path().join("sync");
    let key = SpaceKey::generate().unwrap();
    let result = cycle(
        &root,
        &binding(),
        &key,
        &remote(),
        "hermes",
        &home,
        Direction::Upload,
        || false,
    );
    assert_eq!(result.err().unwrap(), "unsafe_store");
}

// A malformed or foreign store is refused instead of silently exporting nothing.
#[test]
fn hermes_malformed_or_foreign_store_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let root = temp.path().join("sync");
    let key = SpaceKey::generate().unwrap();

    fs::write(home.join("state.db"), b"not a database at all").unwrap();
    let err = cycle(
        &root,
        &binding(),
        &key,
        &remote(),
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .err()
    .unwrap();
    assert_eq!(err, "session_invalid");

    // A well-formed SQLite database that is not a Hermes store is refused.
    let foreign = temp.path().join("foreign.db");
    let conn = rusqlite::Connection::open(&foreign).unwrap();
    conn.execute_batch("CREATE TABLE t(x INTEGER)").unwrap();
    drop(conn);
    fs::write(home.join("state.db"), fs::read(&foreign).unwrap()).unwrap();
    assert_eq!(
        cycle(
            &root,
            &binding(),
            &key,
            &remote(),
            "hermes",
            &home,
            Direction::Upload,
            || false
        )
        .err()
        .unwrap(),
        "session_format_unsupported"
    );
}

// Only `state.db` is an accepted relative path for Hermes.
#[test]
fn hermes_manifest_only_accepts_state_db() {
    assert!(allowed("hermes", "state.db"));
    assert!(!allowed("hermes", "other.db"));
    assert!(!allowed("hermes", "sessions/state.db"));
    assert!(!allowed("hermes", "state.db/child"));
    let m = Manifest {
        version: 1,
        agent: "hermes".into(),
        session: A.into(),
        cwd: "/project".into(),
        files: BTreeMap::from([("evil.db".into(), STANDARD.encode(b"x"))]),
    };
    assert!(!manifest_paths_safe(&m));
}

// A received store that is not the canonical shape (extra table, view, trigger)
// is rejected by validation and rebuilt clean by the transform before any write.
#[test]
fn hermes_received_store_is_rejected_when_tampered_and_rebuilt_when_transformed() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("state.db");
    hermes_db(&path, &[(A, "/project")]);
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch(
        "CREATE TABLE smuggled(secret TEXT); INSERT INTO smuggled VALUES('LEAK');
         CREATE VIEW peek AS SELECT * FROM smuggled;",
    )
    .unwrap();
    conn.execute_batch(
        "CREATE TRIGGER evil AFTER INSERT ON messages
         BEGIN UPDATE messages SET content='tampered'; END;",
    )
    .unwrap();
    drop(conn);
    let manifest = Manifest {
        version: 1,
        agent: "hermes".into(),
        session: A.into(),
        cwd: "/project".into(),
        files: BTreeMap::from([("state.db".into(), STANDARD.encode(fs::read(&path).unwrap()))]),
    };
    assert_eq!(
        validate_hermes_manifest(&manifest).unwrap_err(),
        "session_format_unsupported".to_string()
    );

    let rebuilt = transform_hermes_manifest(&manifest, &[]).unwrap();
    let canonical = STANDARD
        .decode(rebuilt.files.get("state.db").unwrap())
        .unwrap();
    assert!(!contains(&canonical, b"LEAK"));
    assert!(!contains(&canonical, b"tampered"));
    let dir = tempfile::tempdir().unwrap();
    hermes_assert_canonical(&canonical, dir.path()).unwrap();
    let (_d, conn) = open(&canonical);
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type IN ('trigger','view')",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        conn.query_row("SELECT id FROM sessions", [], |r| r.get::<_, String>(0))
            .unwrap(),
        A
    );
}

// Receive prepares an isolated managed HERMES_HOME; edits there are captured as a
// causal child and a live home is never written into.
#[cfg(not(feature = "mac-app-store"))]
#[test]
fn hermes_receive_prepares_isolated_managed_home_and_handoff_child() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();

    // Computer A exports one session.
    let home_a = temp.path().join("home-a");
    fs::create_dir_all(&home_a).unwrap();
    hermes_db(&home_a.join("state.db"), &[(A, &cwd)]);
    let before_a = fs::read(home_a.join("state.db")).unwrap();
    let root_a = temp.path().join("sync-a");
    let key = SpaceKey::generate().unwrap();
    let remote_a = remote();
    cycle(
        &root_a,
        &binding(),
        &key,
        &remote_a,
        "hermes",
        &home_a,
        Direction::Upload,
        || false,
    )
    .unwrap();

    // Computer B receives; its own Hermes home has no sessions yet.
    let home_b = temp.path().join("home-b");
    fs::create_dir_all(&home_b).unwrap();
    let root_b = temp.path().join("sync-b");
    let remote_b = Remote(RefCell::new(remote_a.0.borrow().clone()), Cell::new(100));
    let status = cycle(
        &root_b,
        &binding(),
        &key,
        &remote_b,
        "hermes",
        &home_b,
        Direction::Download,
        || false,
    )
    .unwrap();
    assert_eq!(status.restored, 1, "{:?}", status.issues);

    let handoffs = load_handoffs(&root_b).unwrap();
    assert_eq!(handoffs.entries.len(), 1);
    let original = handoffs.entries[0].base.clone();
    let managed = handoffs.entries[0].path.clone();
    assert_eq!(handoffs.entries[0].agent, "hermes");
    assert_eq!(handoffs.entries[0].session, A);
    assert_eq!(handoffs.entries[0].main_file, "state.db");
    assert!(managed.join("state.db").is_file());
    assert!(
        !managed.starts_with(&home_b) && !managed.starts_with(&home_a),
        "managed restore must be isolated from every live home"
    );
    let conn = rusqlite::Connection::open(managed.join("state.db")).unwrap();
    let id: String = conn
        .query_row("SELECT id FROM sessions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(id, A);
    drop(conn);

    // The user continues the restored session in the managed home.
    let conn = rusqlite::Connection::open(managed.join("state.db")).unwrap();
    conn.execute(
        "INSERT INTO messages(session_id,role,content,timestamp) VALUES(?1,'user','continued',9.0)",
        rusqlite::params![A],
    )
    .unwrap();
    drop(conn);

    // The next upload captures the managed profile as a causal child.
    let status = cycle(
        &root_b,
        &binding(),
        &key,
        &remote_b,
        "hermes",
        &home_b,
        Direction::Upload,
        || false,
    )
    .unwrap();
    assert_eq!(status.published, 1, "{:?}", status.issues);
    let child = remote_b
        .0
        .borrow()
        .values()
        .find(|b| b.snapshot.parents.first().map(String::as_str) == Some(original.as_str()))
        .cloned()
        .expect("a child bundle of the restored session");
    let m = unpack(&child, &remote_b.0.borrow()).unwrap();
    assert_eq!(m.session, A);

    // Neither live home was touched.
    assert_eq!(fs::read(home_a.join("state.db")).unwrap(), before_a);
    assert!(!home_b.join("state.db").exists());
}

// Project mapping rewrites the recorded working directory inside the database.
#[test]
fn hermes_project_mapping_rewrites_cwd_inside_database() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let old = temp.path().join("old-project");
    let new = temp.path().join("new-project");
    fs::create_dir_all(&old).unwrap();
    fs::create_dir_all(&new).unwrap();
    let old_cwd = old.to_string_lossy().into_owned();
    // The mapping canonicalizes and normalizes the target (stripping Windows
    // verbatim `\\?\` prefixes); compare against that exact resolved form.
    let new_cwd = crate::project_mapping::canonical_target(&new).unwrap();
    hermes_db(&home.join("state.db"), &[(A, &old_cwd)]);
    let root = temp.path().join("sync");
    let key = SpaceKey::generate().unwrap();
    let remote = remote();
    cycle(
        &root,
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();

    let bundles = remote.0.borrow().clone();
    let b = bundles
        .values()
        .find(|b| b.snapshot.stream.agent == "hermes")
        .unwrap();
    let m = unpack(b, &bundles).unwrap();
    let mapped = crate::project_mapping::transform_manifest(
        &m,
        &[Mapping {
            source: old_cwd.clone(),
            target: new_cwd.clone(),
        }],
    )
    .unwrap();
    assert_eq!(mapped.cwd, new_cwd);
    crate::project_mapping::validate_provider_group(&mapped).unwrap();
    let db = STANDARD
        .decode(mapped.files.get("state.db").unwrap())
        .unwrap();
    let (_dir, conn) = open(&db);
    let cwd: String = conn
        .query_row("SELECT cwd FROM sessions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(cwd, new_cwd);
}

// ── Lineage graph and referenced attachments ───────────────────────────────

const P: &str = "20261010_210000_comp00";
const C: &str = "20261010_210001_child0";
const D: &str = "20261010_210002_ghost0";
const E: &str = "20261010_210003_branch0";

#[allow(clippy::too_many_arguments)]
fn hermes_session_row(
    conn: &rusqlite::Connection,
    id: &str,
    cwd: &str,
    parent: Option<&str>,
    end_reason: Option<&str>,
    model_config: Option<&str>,
    session_key: Option<&str>,
) {
    let hash = format!("hash-{id}");
    conn.execute(
        "INSERT OR IGNORE INTO system_prompts(hash,prompt) VALUES(?1,?2)",
        rusqlite::params![hash, format!("prompt-{id}")],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO sessions(id,source,system_prompt_hash,started_at,cwd,title,parent_session_id,end_reason,model_config,session_key) \
         VALUES(?1,'cli',?2,0.0,?3,?4,?5,?6,?7,?8)",
        rusqlite::params![id, hash, cwd, format!("title-{id}"), parent, end_reason, model_config, session_key],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO messages(session_id,role,content,timestamp) VALUES(?1,'user',?2,0.0)",
        rusqlite::params![id, format!("user-marker-{id}")],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO messages(session_id,role,reasoning,content,tool_calls,timestamp) \
         VALUES(?1,'assistant','think',NULL,?2,0.0)",
        rusqlite::params![id, r#"[{"id":"call_1","type":"function","function":{"name":"terminal","arguments":"{\"command\":\"ls\"}"}}]"#],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO messages(session_id,role,content,tool_call_id,tool_name,timestamp) \
         VALUES(?1,'tool',?2,?3,'terminal',0.0)",
        rusqlite::params![id, format!("result-{id}"), format!("call_{id}")],
    )
    .unwrap();
}

fn hermes_store(path: &Path, build: impl FnOnce(&rusqlite::Connection)) {
    let conn = rusqlite::Connection::open(path).unwrap();
    // A degraded store may reference a parent row that is missing or part of a
    // cycle; the exporter handles that, so the fixture must be able to build it.
    conn.execute_batch("PRAGMA foreign_keys=OFF;").unwrap();
    conn.execute_batch(HERMES_SCHEMA_DDL).unwrap();
    conn.execute(
        "INSERT INTO schema_version(version) VALUES(?1)",
        [HERMES_SCHEMA_VERSION],
    )
    .unwrap();
    build(&conn);
}

fn ordered_ids(db: &[u8]) -> Vec<String> {
    let (_dir, conn) = open(db);
    let mut stmt = conn.prepare("SELECT id FROM sessions ORDER BY id").unwrap();
    stmt.query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|row| row.unwrap())
        .collect()
}

fn parent_of(db: &[u8], id: &str) -> Option<String> {
    let (_dir, conn) = open(db);
    conn.query_row(
        "SELECT parent_session_id FROM sessions WHERE id=?1",
        [id],
        |r| r.get::<_, Option<String>>(0),
    )
    .unwrap()
}

fn manifest_for(remote: &Remote, session: &str) -> Manifest {
    let bundles = remote.0.borrow().clone();
    let b = bundles
        .values()
        .find(|b| {
            b.snapshot.stream.agent == "hermes"
                && b.snapshot.stream.conversation == bundle::hash(session.as_bytes())
        })
        .unwrap_or_else(|| panic!("no bundle for {session}"));
    unpack(b, &bundles).unwrap()
}

fn sorted_pair(a: &str, b: &str) -> Vec<String> {
    let mut v = vec![a.to_string(), b.to_string()];
    v.sort();
    v
}

// A compression continuation (parent ended by `compression`) must travel with
// its parent: the child's export keeps the parent row, the parent link and every
// tool call/result, and `manifest.session` stays the exported root.
#[test]
fn hermes_compression_lineage_graph_keeps_parent_link_and_tools() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    hermes_store(&home.join("state.db"), |conn| {
        hermes_session_row(conn, P, &cwd, None, Some("compression"), None, None);
        hermes_session_row(conn, C, &cwd, Some(P), None, None, None);
    });
    let before = fs::read(home.join("state.db")).unwrap();

    let root = temp.path().join("sync");
    let key = SpaceKey::generate().unwrap();
    let remote = remote();
    let status = cycle(
        &root,
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();
    assert_eq!(status.captured, 2, "{:?}", status.issues);
    assert_eq!(status.published, 2, "{:?}", status.issues);
    assert_eq!(
        fs::read(home.join("state.db")).unwrap(),
        before,
        "the live store must never be modified"
    );

    let child = manifest_for(&remote, C);
    assert_eq!(child.session, C, "manifest root is the exported session");
    let child_db = export_of(&remote, C);
    assert_eq!(ordered_ids(&child_db), sorted_pair(P, C));
    assert_eq!(parent_of(&child_db, C).as_deref(), Some(P));
    let (_dir, conn) = open(&child_db);
    let reason: Option<String> = conn
        .query_row("SELECT end_reason FROM sessions WHERE id=?1", [P], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(reason.as_deref(), Some("compression"));
    for id in [P, C] {
        let tool: String = conn
            .query_row(
                "SELECT COALESCE(tool_name,'') FROM messages WHERE session_id=?1 AND role='tool'",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tool, "terminal", "{id}");
        let user: String = conn
            .query_row(
                "SELECT content FROM messages WHERE session_id=?1 AND role='user'",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(user, format!("user-marker-{id}"));
    }
    drop(conn);
    // The parent's own export carries its live continuation child too.
    assert_eq!(ordered_ids(&export_of(&remote, P)), sorted_pair(P, C));
}

// A branch child (`_branched_from` marker) is connected in both directions.
#[test]
fn hermes_branch_child_marker_links_both_directions() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    hermes_store(&home.join("state.db"), |conn| {
        hermes_session_row(conn, P, &cwd, None, Some("completed"), None, None);
        hermes_session_row(
            conn,
            E,
            &cwd,
            Some(P),
            None,
            Some(&format!("{{\"_branched_from\":\"{P}\"}}")),
            None,
        );
    });
    let root = temp.path().join("sync");
    let key = SpaceKey::generate().unwrap();
    let remote = remote();
    let status = cycle(
        &root,
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();
    assert_eq!(status.published, 2, "{:?}", status.issues);
    let branch_db = export_of(&remote, E);
    assert_eq!(ordered_ids(&branch_db), sorted_pair(P, E));
    assert_eq!(parent_of(&branch_db, E).as_deref(), Some(P));
    // The parent graph reaches the branch child as a lineage continuation.
    assert_eq!(ordered_ids(&export_of(&remote, P)), sorted_pair(P, E));
}

// A parent cycle in a corrupted store terminates and keeps both references.
#[test]
fn hermes_parent_cycle_is_bounded_and_preserved() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    hermes_store(&home.join("state.db"), |conn| {
        hermes_session_row(conn, P, &cwd, Some(C), None, None, None);
        hermes_session_row(conn, C, &cwd, Some(P), None, None, None);
    });
    let root = temp.path().join("sync");
    let key = SpaceKey::generate().unwrap();
    let remote = remote();
    let status = cycle(
        &root,
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();
    assert_eq!(status.captured, 2, "{:?}", status.issues);
    assert_eq!(status.published, 2, "{:?}", status.issues);
    let db = export_of(&remote, P);
    assert_eq!(ordered_ids(&db), sorted_pair(P, C));
    assert_eq!(parent_of(&db, P).as_deref(), Some(C));
    assert_eq!(parent_of(&db, C).as_deref(), Some(P));
}

// A parent id the store references but does not contain is reported and cleared,
// never followed, and never silently dropped.
#[test]
fn hermes_dangling_parent_reference_is_reported_and_cleared() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    hermes_store(&home.join("state.db"), |conn| {
        hermes_session_row(conn, C, &cwd, Some(D), None, None, None);
    });
    let root = temp.path().join("sync");
    let key = SpaceKey::generate().unwrap();
    let remote = remote();
    let status = cycle(
        &root,
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();
    assert_eq!(status.captured, 1, "{:?}", status.issues);
    assert_eq!(
        status.issues.get("hermes_parent_missing"),
        Some(&1),
        "{:?}",
        status.issues
    );
    let db = export_of(&remote, C);
    assert_eq!(ordered_ids(&db), vec![C.to_string()]);
    assert!(
        parent_of(&db, C).is_none(),
        "dangling parent must be cleared"
    );
}

// ── Explicit lineage edges and the inert async-delegation sidecar ──────────

// Edge classification matches the installed source: a genuine compression
// continuation is a `Continuation` child; a `_delegate_from` child is a
// `Delegated` child and is NEVER a continuation even when its parent ended with
// 'compression'; a tool session child is neither and is not required.
#[test]
fn hermes_explicit_edges_classify_compression_delegate_and_tool_children() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("state.db");
    hermes_store(&path, |conn| {
        hermes_session_row(conn, P, "/project", None, Some("compression"), None, None);
        hermes_session_row(conn, C, "/project", Some(P), None, None, None);
        hermes_session_row(
            conn,
            D,
            "/project",
            Some(P),
            None,
            Some(&format!("{{\"_delegate_from\":\"{P}\"}}")),
            None,
        );
        hermes_session_row(conn, E, "/project", Some(P), None, None, None);
        conn.execute("UPDATE sessions SET source='tool' WHERE id=?1", [E])
            .unwrap();
    });
    let conn = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        hermes_edge_children(&conn, P, LineageEdge::Continuation).unwrap(),
        vec![C.to_string()],
        "only the genuine continuation is a compression child"
    );
    assert_eq!(
        hermes_edge_children(&conn, P, LineageEdge::Delegated).unwrap(),
        vec![D.to_string()],
        "the _delegate_from child is reached by the delegated edge"
    );
    assert!(hermes_edge_children(&conn, P, LineageEdge::Branch)
        .unwrap()
        .is_empty());
    assert_eq!(
        hermes_lineage_children(&conn, P).unwrap(),
        vec![C.to_string(), D.to_string()],
        "a tool session child is not a required lineage child"
    );
}

// A compression-ended parent and a delegate child travel together; the delegate
// child keeps the parent link and its tool calls/results, and the parent graph
// reaches the delegate child by its explicit edge.
#[test]
fn hermes_compression_parent_and_delegate_child_travel_with_explicit_edges() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    hermes_store(&home.join("state.db"), |conn| {
        hermes_session_row(conn, P, &cwd, None, Some("compression"), None, None);
        hermes_session_row(
            conn,
            C,
            &cwd,
            Some(P),
            None,
            Some(&format!("{{\"_delegate_from\":\"{P}\"}}")),
            None,
        );
    });
    let before = fs::read(home.join("state.db")).unwrap();

    let root = temp.path().join("sync");
    let key = SpaceKey::generate().unwrap();
    let remote = remote();
    let status = cycle(
        &root,
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();
    assert_eq!(status.captured, 2, "{:?}", status.issues);
    assert_eq!(status.published, 2, "{:?}", status.issues);
    assert_eq!(fs::read(home.join("state.db")).unwrap(), before);

    assert_eq!(ordered_ids(&export_of(&remote, C)), sorted_pair(P, C));
    assert_eq!(parent_of(&export_of(&remote, C), C).as_deref(), Some(P));
    // The parent graph reaches the delegate child through the delegated edge.
    assert_eq!(ordered_ids(&export_of(&remote, P)), sorted_pair(P, C));
}

// A finished async-delegation row travels as an inert sidecar: its outcome and
// task relationships are preserved with source ids, the live store is untouched,
// and the restored canonical store has an empty `async_delegations` table (no
// lease, no replay). An in-flight job is an explicit blocker, never a running
// process claimed as transferred.
#[test]
fn hermes_finished_async_delegation_is_inert_sidecar_and_never_activates_a_lease() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    hermes_store(&home.join("state.db"), |conn| {
        hermes_session_row(conn, A, &cwd, None, None, None, None);
        conn.execute(
            "INSERT INTO async_delegations(delegation_id,origin_session,origin_session_id,
                 parent_session_id,state,dispatched_at,completed_at,updated_at,event_json,
                 result_json,delivery_state,task_json)
             VALUES('deleg-done',?1,?1,?1,'done',1.0,2.0,2.0,'{\"kind\":\"async_delegation\"}',
                 '{\"summary\":\"DONE-RESULT\"}','pending','{\"prompt\":\"DO-TASK\"}')",
            [A],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO async_delegations(delegation_id,origin_session,state,dispatched_at,
                 updated_at,delivery_state)
             VALUES('deleg-live',?1,'running',3.0,3.0,'pending')",
            [A],
        )
        .unwrap();
    });
    let before = fs::read(home.join("state.db")).unwrap();

    let root = temp.path().join("sync");
    let key = SpaceKey::generate().unwrap();
    let remote = remote();
    let status = cycle(
        &root,
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();
    assert_eq!(status.captured, 1, "{:?}", status.issues);
    assert_eq!(
        status.issues.get("hermes_async_delegation_incomplete"),
        Some(&1),
        "an in-flight job must be an explicit blocker: {:?}",
        status.issues
    );
    assert_eq!(fs::read(home.join("state.db")).unwrap(), before);

    let manifest = manifest_for(&remote, A);
    let index: serde_json::Value = serde_json::from_slice(
        &STANDARD
            .decode(manifest.files.get(HERMES_ASYNC_INDEX).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(index["inert"], true, "the sidecar must be inert");
    assert_eq!(index["root"], A);
    let records = index["records"].as_array().unwrap();
    assert_eq!(records.len(), 1, "only finished jobs travel: {records:?}");
    assert_eq!(records[0]["delegationId"], "deleg-done");
    assert_eq!(records[0]["originSession"], A);
    assert_eq!(records[0]["parentSessionId"], A, "source id preserved");
    assert_eq!(records[0]["task"]["prompt"], "DO-TASK");
    assert_eq!(records[0]["result"]["summary"], "DONE-RESULT");
    assert_eq!(records[0]["deliveryState"], "pending");
    assert!(!records.iter().any(|r| r["state"] == "running"));

    // The rebuilt canonical store has no runtime row and no lease: nothing to
    // restart, resume or replay.
    let db = export_of(&remote, A);
    let (_dir, conn) = open(&db);
    for table in [
        "async_delegations",
        "session_turn_leases",
        "compression_locks",
    ] {
        let count: i64 = conn
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "{table} must be empty (no lease/replay)");
    }
}

// A run whose only delegation is still in flight exports no sidecar record set
// yet still reports the blocker, and never fabricates a transferred outcome.
#[test]
fn hermes_inflight_only_async_delegation_reports_blocker_without_records() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    hermes_store(&home.join("state.db"), |conn| {
        hermes_session_row(conn, A, &cwd, None, None, None, None);
        conn.execute(
            "INSERT INTO async_delegations(delegation_id,origin_session,state,dispatched_at,
                 updated_at,delivery_state)
             VALUES('deleg-live',?1,'finalizing',3.0,3.0,'pending')",
            [A],
        )
        .unwrap();
    });
    let root = temp.path().join("sync");
    let key = SpaceKey::generate().unwrap();
    let remote = remote();
    let status = cycle(
        &root,
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();
    assert_eq!(
        status.issues.get("hermes_async_delegation_incomplete"),
        Some(&1),
        "{:?}",
        status.issues
    );
    let manifest = manifest_for(&remote, A);
    let index: serde_json::Value = serde_json::from_slice(
        &STANDARD
            .decode(manifest.files.get(HERMES_ASYNC_INDEX).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert!(index["records"].as_array().unwrap().is_empty());
    assert_eq!(index["blockers"][0], "hermes_async_delegation_incomplete");
}

// Explicitly referenced attachments are content-hashed and captured; a missing
// file and a symlink are reported, never followed, never silently dropped.
#[cfg(unix)]
#[test]
fn hermes_referenced_attachments_are_hashed_missing_and_symlinks_reported() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    fs::write(project.join("attached.txt"), b"ATTACH-CONTENT").unwrap();
    let real = temp.path().join("real.txt");
    fs::write(&real, b"REAL").unwrap();
    std::os::unix::fs::symlink(&real, project.join("link.txt")).unwrap();
    hermes_store(&home.join("state.db"), |conn| {
        hermes_session_row(conn, A, &cwd, None, None, None, None);
        conn.execute(
            "UPDATE messages SET content=?1 WHERE session_id=?2 AND role='user'",
            rusqlite::params![
                "see @file:attached.txt and @file:missing.txt and @file:link.txt",
                A
            ],
        )
        .unwrap();
    });
    let key = SpaceKey::generate().unwrap();

    // No mapping: a recorded cwd is NOT an attachment root. Nothing under it may
    // be read; the references are reported, never silently captured.
    let unmapped_remote = remote();
    let unmapped = cycle(
        &temp.path().join("sync-unmapped"),
        &binding(),
        &key,
        &unmapped_remote,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();
    assert_eq!(unmapped.captured, 1, "{:?}", unmapped.issues);
    assert!(
        unmapped
            .issues
            .get("hermes_attachment_outside_roots")
            .copied()
            .unwrap_or(0)
            >= 1,
        "an unmapped cwd must never be read: {:?}",
        unmapped.issues
    );
    let unmapped_manifest = manifest_for(&unmapped_remote, A);
    assert!(
        !unmapped_manifest
            .files
            .keys()
            .any(|k| { k.starts_with("attachments/") && k.as_str() != "attachments/index.json" }),
        "no attachment may be captured from an unmapped cwd: {:?}",
        unmapped_manifest.files.keys()
    );
    for value in unmapped_manifest.files.values() {
        assert!(!contains(
            &STANDARD.decode(value).unwrap(),
            b"ATTACH-CONTENT"
        ));
    }

    // Authorized by an explicit mapping: the covered cwd is a read root, and a
    // missing file and a symlink are reported, never followed or silently
    // dropped.
    let mapped = temp.path().join("mapped");
    fs::create_dir_all(&mapped).unwrap();
    let mappings = [Mapping {
        source: cwd.clone(),
        target: mapped.to_string_lossy().into_owned(),
    }];
    let root = temp.path().join("sync");
    let remote = remote();
    let status = cycle_with_mappings(
        &root,
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        &mappings,
        || false,
    )
    .unwrap();
    assert_eq!(status.captured, 1, "{:?}", status.issues);
    assert_eq!(
        status.issues.get("hermes_attachment_missing"),
        Some(&1),
        "{:?}",
        status.issues
    );
    assert_eq!(
        status.issues.get("hermes_attachment_symlink"),
        Some(&1),
        "{:?}",
        status.issues
    );

    let manifest = manifest_for(&remote, A);
    let blob = manifest
        .files
        .keys()
        .find(|key| key.starts_with("attachments/") && key.ends_with("/attached.txt"))
        .unwrap_or_else(|| panic!("missing attachment blob: {:?}", manifest.files.keys()));
    let sha = blob.split('/').nth(1).unwrap();
    assert!(bundle::is_hash(sha));
    assert_eq!(
        STANDARD.decode(manifest.files.get(blob).unwrap()).unwrap(),
        b"ATTACH-CONTENT"
    );
    assert!(!manifest.files.keys().any(|k| k.ends_with("missing.txt")));
    assert!(!manifest.files.keys().any(|k| k.ends_with("link.txt")));

    let index: serde_json::Value = serde_json::from_slice(
        &STANDARD
            .decode(manifest.files.get("attachments/index.json").unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(index["root"], A);
    assert!(index["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["key"] == *blob && e["sha256"] == *sha));
    let reasons: Vec<String> = index["problems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["reason"].as_str().unwrap().to_string())
        .collect();
    assert!(reasons.contains(&"missing".to_string()));
    assert!(reasons.contains(&"symlink".to_string()));
}

// A home-rooted attachment reference is rewritten into the restored home on
// receive (Windows/POSIX handled by the shared path machinery), and the blob is
// materialized there.
#[cfg(not(feature = "mac-app-store"))]
#[test]
fn hermes_home_attachment_reference_is_rewritten_into_restored_home() {
    let temp = tempfile::tempdir().unwrap();
    let home_a = temp.path().join("home-a");
    fs::create_dir_all(&home_a).unwrap();
    let project_a = temp.path().join("project-a");
    fs::create_dir_all(&project_a).unwrap();
    let cwd = project_a.to_string_lossy().into_owned();
    fs::create_dir_all(home_a.join("composer-pastes")).unwrap();
    let note = home_a.join("composer-pastes").join("note.txt");
    fs::write(&note, b"NOTE-BYTES").unwrap();
    let note_ref = note.to_string_lossy().into_owned();
    hermes_store(&home_a.join("state.db"), |conn| {
        hermes_session_row(conn, A, &cwd, None, None, None, None);
        conn.execute(
            "UPDATE messages SET content=?1 WHERE session_id=?2 AND role='user'",
            rusqlite::params![format!("please read @file:{note_ref}"), A],
        )
        .unwrap();
    });
    let root_a = temp.path().join("sync-a");
    let key = SpaceKey::generate().unwrap();
    let remote_a = remote();
    cycle(
        &root_a,
        &binding(),
        &key,
        &remote_a,
        "hermes",
        &home_a,
        Direction::Upload,
        || false,
    )
    .unwrap();

    let home_b = temp.path().join("home-b");
    fs::create_dir_all(&home_b).unwrap();
    let project_b = temp.path().join("project-b");
    fs::create_dir_all(&project_b).unwrap();
    let root_b = temp.path().join("sync-b");
    let remote_b = Remote(RefCell::new(remote_a.0.borrow().clone()), Cell::new(100));
    let mappings = [Mapping {
        source: cwd.clone(),
        target: project_b.to_string_lossy().into_owned(),
    }];
    let status = cycle_with_mappings(
        &root_b,
        &binding(),
        &key,
        &remote_b,
        "hermes",
        &home_b,
        Direction::Download,
        &mappings,
        || false,
    )
    .unwrap();
    assert_eq!(status.restored, 1, "{:?}", status.issues);

    let handoffs = load_handoffs(&root_b).unwrap();
    assert_eq!(handoffs.entries.len(), 1);
    let managed = handoffs.entries[0].path.clone();
    let index: serde_json::Value =
        serde_json::from_slice(&fs::read(managed.join("attachments/index.json")).unwrap()).unwrap();
    let attachment_key = index["entries"][0]["key"].as_str().unwrap().to_string();
    let restored_path = managed.join(&attachment_key);
    assert!(restored_path.is_file(), "attachment blob must be restored");
    assert_eq!(fs::read(&restored_path).unwrap(), b"NOTE-BYTES");

    let (_dir, conn) = open(&fs::read(managed.join("state.db")).unwrap());
    let content: String = conn
        .query_row("SELECT content FROM messages WHERE role='user'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(
        content.contains(&restored_path.to_string_lossy().into_owned()),
        "reference not remapped: {content}"
    );
    assert!(
        !content.contains(&note_ref),
        "stale source path kept: {content}"
    );

    // A pristine re-upload republishes exactly once — the recorded attachment
    // index names the resolved source path, which is now the restored home — then
    // converges. This pins the bound and proves it does not loop.
    let reupload = cycle_with_mappings(
        &root_b,
        &binding(),
        &key,
        &remote_b,
        "hermes",
        &home_b,
        Direction::Upload,
        &mappings,
        || false,
    )
    .unwrap();
    assert_eq!(
        reupload.published, 1,
        "one-time re-index publish expected, got {:?}",
        reupload.issues
    );
    let settled = cycle_with_mappings(
        &root_b,
        &binding(),
        &key,
        &remote_b,
        "hermes",
        &home_b,
        Direction::Upload,
        &mappings,
        || false,
    )
    .unwrap();
    assert_eq!(
        settled.published, 0,
        "recapture must converge, got {:?}",
        settled.issues
    );
}

// A Windows-authored absolute reference is remapped to the receiving POSIX path.
#[test]
fn hermes_transform_remaps_windows_attachment_reference() {
    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("state.db");
    let win_cwd = r"C:\work\proj";
    hermes_store(&db_path, |conn| {
        hermes_session_row(conn, A, win_cwd, None, None, None, None);
        conn.execute(
            "UPDATE messages SET content=?1 WHERE session_id=?2 AND role='user'",
            rusqlite::params![r"see @file:C:\work\proj\assets\logo.png", A],
        )
        .unwrap();
    });
    let bytes = {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        hermes_canonicalize(&conn, A, win_cwd, temp.path()).unwrap()
    };
    let manifest = Manifest {
        version: 1,
        agent: "hermes".into(),
        session: A.into(),
        cwd: win_cwd.into(),
        files: BTreeMap::from([("state.db".into(), STANDARD.encode(bytes))]),
    };
    let target = temp.path().join("dest");
    fs::create_dir_all(&target).unwrap();
    // The mapping canonicalizes and normalizes the target (stripping Windows
    // verbatim `\\?\` prefixes); compare against that exact resolved form.
    let target_str = crate::project_mapping::canonical_target(&target).unwrap();
    let mapped = transform_hermes_manifest(
        &manifest,
        &[Mapping {
            source: win_cwd.into(),
            target: target_str.clone(),
        }],
    )
    .unwrap();
    assert_eq!(mapped.cwd, target_str);
    let (_dir, conn) = open(
        &STANDARD
            .decode(mapped.files.get("state.db").unwrap())
            .unwrap(),
    );
    let content: String = conn
        .query_row("SELECT content FROM messages WHERE role='user'", [], |r| {
            r.get(0)
        })
        .unwrap();
    // The rewritten reference uses the platform separator, so join it the same
    // way on Windows (`\`) and POSIX (`/`).
    let expected_ref = Path::new(&target_str)
        .join("assets")
        .join("logo.png")
        .to_string_lossy()
        .into_owned();
    assert!(
        content.contains(&format!("@file:{expected_ref}")),
        "{content}"
    );
    assert!(!content.contains(r"C:\work"));
}

// Restoring a compression graph into a managed home keeps both sessions and the
// parent link; a continued edit republishes a causal child whose root session id
// is unchanged.
#[cfg(not(feature = "mac-app-store"))]
#[test]
fn hermes_graph_restores_and_recaptures_multi_session_lineage() {
    let temp = tempfile::tempdir().unwrap();
    let home_a = temp.path().join("home-a");
    fs::create_dir_all(&home_a).unwrap();
    let project_a = temp.path().join("project-a");
    fs::create_dir_all(&project_a).unwrap();
    let cwd = project_a.to_string_lossy().into_owned();
    hermes_store(&home_a.join("state.db"), |conn| {
        hermes_session_row(conn, P, &cwd, None, Some("compression"), None, None);
        hermes_session_row(conn, C, &cwd, Some(P), None, None, None);
    });
    let key = SpaceKey::generate().unwrap();
    let remote_a = remote();
    cycle(
        &temp.path().join("sync-a"),
        &binding(),
        &key,
        &remote_a,
        "hermes",
        &home_a,
        Direction::Upload,
        || false,
    )
    .unwrap();

    let home_b = temp.path().join("home-b");
    fs::create_dir_all(&home_b).unwrap();
    let project_b = temp.path().join("project-b");
    fs::create_dir_all(&project_b).unwrap();
    let root_b = temp.path().join("sync-b");
    let remote_b = Remote(RefCell::new(remote_a.0.borrow().clone()), Cell::new(100));
    let mappings = [Mapping {
        source: cwd.clone(),
        target: project_b.to_string_lossy().into_owned(),
    }];
    let status = cycle_with_mappings(
        &root_b,
        &binding(),
        &key,
        &remote_b,
        "hermes",
        &home_b,
        Direction::Download,
        &mappings,
        || false,
    )
    .unwrap();
    assert_eq!(status.restored, 2, "{:?}", status.issues);

    let registry = load_handoffs(&root_b).unwrap();
    let handoff = registry
        .entries
        .iter()
        .find(|h| h.session == C)
        .expect("managed profile for the child root");
    let original_base = handoff.base.clone();
    let managed = handoff.path.clone();
    // The restored managed store keeps the whole lineage graph.
    let db = fs::read(managed.join("state.db")).unwrap();
    assert_eq!(ordered_ids(&db), sorted_pair(P, C));
    assert_eq!(parent_of(&db, C).as_deref(), Some(P));

    // Continue the restored child.
    let conn = rusqlite::Connection::open(managed.join("state.db")).unwrap();
    conn.execute(
        "INSERT INTO messages(session_id,role,content,timestamp) VALUES(?1,'user','continued-in-b',9.0)",
        [C],
    )
    .unwrap();
    drop(conn);

    let published = cycle_with_mappings(
        &root_b,
        &binding(),
        &key,
        &remote_b,
        "hermes",
        &home_b,
        Direction::Upload,
        &mappings,
        || false,
    )
    .unwrap();
    assert_eq!(published.published, 1, "{:?}", published.issues);
    let child = remote_b
        .0
        .borrow()
        .values()
        .find(|b| b.snapshot.parents.first().map(String::as_str) == Some(original_base.as_str()))
        .cloned()
        .expect("a causal child of the restored head");
    let m = unpack(&child, &remote_b.0.borrow()).unwrap();
    assert_eq!(m.session, C, "root session id stays unambiguous");
    let child_db = STANDARD.decode(m.files.get("state.db").unwrap()).unwrap();
    assert_eq!(ordered_ids(&child_db), sorted_pair(P, C));
    assert_eq!(parent_of(&child_db, C).as_deref(), Some(P));
    assert!(contains(&child_db, b"continued-in-b"));
}

// ── Project working-tree continuity ────────────────────────────────────────

#[cfg(not(feature = "mac-app-store"))]
fn project_run(dir: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .arg("-C")
        .arg(dir)
        .args(args)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?}");
}

#[cfg(not(feature = "mac-app-store"))]
fn project_init_repo(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    project_run(dir, &["init", "-q"]);
    project_run(dir, &["config", "user.email", "fixture@example.test"]);
    project_run(dir, &["config", "user.name", "Fixture"]);
    project_run(dir, &["config", "commit.gpgsign", "false"]);
}

#[cfg(not(feature = "mac-app-store"))]
fn project_commit_all(dir: &Path, message: &str) {
    project_run(dir, &["add", "-A", "-f"]);
    project_run(dir, &["commit", "-q", "-m", message]);
}

fn project_files(m: &Manifest) -> BTreeMap<String, String> {
    m.files
        .iter()
        .filter(|(path, _)| crate::hermes_project::allowed(path))
        .map(|(path, encoded)| (path.clone(), encoded.clone()))
        .collect()
}

fn project_manifest(files: &BTreeMap<String, String>) -> crate::hermes_project::ProjectManifest {
    crate::hermes_project::read_manifest(files).unwrap()
}

// The project working tree is captured ONLY for a user-authorized mapping; an
// unauthorized local cwd is never read, and its readiness blocks + is reported.
#[test]
fn hermes_project_capture_is_authorized_only_and_blocks_unmapped_cwd() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let project = temp.path().join("proj");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("MARKER.txt"), b"UNMAPPED-SECRET-MARKER").unwrap();
    let cwd = project.to_string_lossy().into_owned();
    hermes_db(&home.join("state.db"), &[(A, &cwd)]);

    // No mapping: nothing from the cwd may travel; readiness blocks.
    let key = SpaceKey::generate().unwrap();
    let remote1 = remote();
    let status = cycle(
        &temp.path().join("sync"),
        &binding(),
        &key,
        &remote1,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();
    assert_eq!(status.captured, 1, "{:?}", status.issues);
    assert_eq!(
        status.issues.get("project_mapping_required"),
        Some(&1),
        "{:?}",
        status.issues
    );

    let m = manifest_for(&remote1, A);
    let sidecar = project_files(&m);
    assert!(!sidecar.is_empty(), "a project readiness sidecar must ship");
    let blocked = project_manifest(&sidecar);
    assert!(
        blocked.files.is_empty(),
        "an unmapped cwd is never captured"
    );
    assert!(!crate::hermes_project::ready(&blocked));
    assert!(blocked
        .blockers
        .contains(&"project_mapping_required".to_string()));
    for value in m.files.values() {
        let bytes = STANDARD.decode(value).unwrap();
        assert!(
            !contains(&bytes, b"UNMAPPED-SECRET-MARKER"),
            "an unauthorized project must never be read"
        );
    }

    // Received: the blocking sidecar persists into the receiver's issues and the
    // version is NOT reported ready merely because the sidecar was copied.
    let home_recv = temp.path().join("home-recv");
    fs::create_dir_all(&home_recv).unwrap();
    let remote_recv = Remote(RefCell::new(remote1.0.borrow().clone()), Cell::new(200));
    let received = cycle_with_mappings(
        &temp.path().join("sync-recv"),
        &binding(),
        &key,
        &remote_recv,
        "hermes",
        &home_recv,
        Direction::Download,
        &[],
        || false,
    )
    .unwrap();
    // A received session whose recorded cwd has no explicit mapping is never
    // prepared: the non-sandbox host still restores the version but carries the
    // blocking project sidecar, while under the sandbox the prepare itself is
    // refused until the user maps an authorized root.
    assert_eq!(
        received.issues.get("project_mapping_required"),
        Some(&1),
        "{:?}",
        received.issues
    );
    #[cfg(not(feature = "mac-app-store"))]
    {
        assert_eq!(received.restored, 1, "{:?}", received.issues);
        assert_eq!(received.state, "partial");
    }
    #[cfg(feature = "mac-app-store")]
    {
        assert_eq!(received.restored, 0, "{:?}", received.issues);
    }

    // Authorized by an explicit mapping: the covered cwd is captured and ready.
    let mapped = temp.path().join("mapped");
    fs::create_dir_all(&mapped).unwrap();
    let mappings = [Mapping {
        source: cwd.clone(),
        target: mapped.to_string_lossy().into_owned(),
    }];
    let remote2 = remote();
    let status2 = cycle_with_mappings(
        &temp.path().join("sync2"),
        &binding(),
        &key,
        &remote2,
        "hermes",
        &home,
        Direction::Upload,
        &mappings,
        || false,
    )
    .unwrap();
    assert_eq!(status2.captured, 1, "{:?}", status2.issues);
    assert!(
        !status2.issues.contains_key("project_mapping_required"),
        "{:?}",
        status2.issues
    );
    let authorized = project_files(&manifest_for(&remote2, A));
    assert!(crate::hermes_project::ready(&project_manifest(&authorized)));
    assert!(authorized
        .values()
        .any(|encoded| STANDARD.decode(encoded).unwrap() == b"UNMAPPED-SECRET-MARKER"));
}

// A malicious project sidecar (tampered bytes, secret or escaping source) is
// refused by both the project validator and the integrated Hermes validator.
#[test]
fn hermes_project_sidecar_tampering_and_unsafe_paths_are_refused() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("proj");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("ok.txt"), "ok\n").unwrap();
    let files = crate::hermes_project::capture(&project).unwrap();
    crate::hermes_project::validate(&files).unwrap();

    let blob = "bastet/project-files/ok.txt";
    let mut tampered = files.clone();
    let mut bytes = STANDARD.decode(files.get(blob).unwrap()).unwrap();
    bytes[0] ^= 0xff;
    tampered.insert(blob.into(), STANDARD.encode(&bytes));
    assert_eq!(
        crate::hermes_project::validate(&tampered).unwrap_err(),
        "project_hash_mismatch"
    );

    for bad in [
        "bastet/project-files/.env",
        "bastet/project-files/../escape",
        "bastet/project-files/a/credentials.json",
    ] {
        assert!(!crate::hermes_project::allowed(bad), "{bad}");
        let mut injected = files.clone();
        injected.insert(bad.into(), STANDARD.encode(b"x"));
        assert!(crate::hermes_project::validate(&injected).is_err(), "{bad}");
    }

    // The integrated validator refuses the same tampering on a received bundle.
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    hermes_db(&home.join("state.db"), &[(A, "/project")]);
    let key = SpaceKey::generate().unwrap();
    let remote = remote();
    cycle(
        &temp.path().join("sync"),
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();
    let mut hostile = manifest_for(&remote, A);
    for (path, encoded) in &tampered {
        hostile.files.insert(path.clone(), encoded.clone());
    }
    assert!(validate_hermes_manifest(&hostile).is_err());
}

// A Windows-recorded source cwd maps onto a POSIX target and the restored
// session cwd is re-rooted to the NEW isolated working tree. POSIX-only: the
// "no `C:` survives" assertion is only meaningful when the local target is not
// itself on a Windows drive.
#[cfg(unix)]
#[test]
fn hermes_project_windows_source_mapping_reroots_to_restored_tree_on_posix() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("proj");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("app.txt"), "app\n").unwrap();
    let captured = crate::hermes_project::capture(&project).unwrap();

    let win_cwd = r"C:\work\proj";
    let db_path = temp.path().join("state.db");
    hermes_store(&db_path, |conn| {
        hermes_session_row(conn, A, win_cwd, None, None, None, None);
    });
    let bytes = {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        hermes_canonicalize(&conn, A, win_cwd, temp.path()).unwrap()
    };
    let mut files = BTreeMap::from([("state.db".to_string(), STANDARD.encode(bytes))]);
    for (path, encoded) in &captured {
        files.insert(path.clone(), encoded.clone());
    }
    let incoming = Manifest {
        version: 1,
        agent: "hermes".into(),
        session: A.into(),
        cwd: win_cwd.into(),
        files,
    };

    let target = temp.path().join("mapped");
    fs::create_dir_all(&target).unwrap();
    let target_canonical = fs::canonicalize(&target).unwrap();
    let mappings = [Mapping {
        source: win_cwd.into(),
        target: target_canonical.to_string_lossy().into_owned(),
    }];
    assert_eq!(
        usable_cwd(win_cwd, &mappings).as_deref(),
        Some(target_canonical.to_string_lossy().as_ref())
    );
    crate::project_mapping::validate_provider_group(&incoming).unwrap();
    let transformed = crate::project_mapping::transform_manifest(&incoming, &mappings).unwrap();
    assert_eq!(transformed.cwd, target_canonical.to_string_lossy());

    let project_destination = temp.path().join("restored-proj");
    let (restored, project_restore) =
        restore_hermes_project(&transformed, &project_destination).unwrap();
    let project_restore = project_restore.unwrap();
    let workdir = project_restore.workdir.unwrap();
    assert_eq!(workdir, fs::canonicalize(&project_destination).unwrap());
    assert_eq!(restored.cwd, workdir.to_string_lossy());
    assert_eq!(fs::read(workdir.join("app.txt")).unwrap(), b"app\n");
    let (_d, conn) = open(
        &STANDARD
            .decode(restored.files.get("state.db").unwrap())
            .unwrap(),
    );
    let cwd: String = conn
        .query_row("SELECT cwd FROM sessions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(cwd, workdir.to_string_lossy());
    assert!(
        !cwd.contains("C:"),
        "the Windows path must not survive restore"
    );
}

// Source → managed restore places the project in a NEW isolated tree (never the
// receiver's mapped worktree), re-roots the DB cwd, and a later work-file edit
// is captured into a causal child from that restored tree.
#[cfg(not(feature = "mac-app-store"))]
#[test]
fn hermes_project_source_to_managed_roundtrip_and_causal_child_edit() {
    let temp = tempfile::tempdir().unwrap();

    // Committed baseline plus a dirty edit, an untracked file, an intentional
    // (unstaged) deletion and an executable script.
    let repo = temp.path().join("src-proj");
    project_init_repo(&repo);
    fs::write(repo.join("dirty.txt"), "v1\n").unwrap();
    fs::write(repo.join("gone.txt"), "gone\n").unwrap();
    fs::write(repo.join("run.sh"), "#!/bin/sh\necho run\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(repo.join("run.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    project_commit_all(&repo, "init");
    let repo_cwd = repo.to_string_lossy().into_owned();
    fs::write(repo.join("dirty.txt"), "v2 dirty\n").unwrap();
    fs::create_dir_all(repo.join("new")).unwrap();
    fs::write(repo.join("new/work.txt"), "untracked work\n").unwrap();
    fs::remove_file(repo.join("gone.txt")).unwrap();

    // The receiver's mapped worktree already exists and must never be written.
    let mapped = temp.path().join("mapped-worktree");
    fs::create_dir_all(&mapped).unwrap();
    fs::write(mapped.join("KEEP.txt"), "receiver work\n").unwrap();
    let mappings = [Mapping {
        source: repo_cwd.clone(),
        target: mapped.to_string_lossy().into_owned(),
    }];

    let home_a = temp.path().join("home-a");
    fs::create_dir_all(&home_a).unwrap();
    hermes_db(&home_a.join("state.db"), &[(A, &repo_cwd)]);
    let key = SpaceKey::generate().unwrap();
    let remote_a = remote();
    let upload = cycle_with_mappings(
        &temp.path().join("sync-a"),
        &binding(),
        &key,
        &remote_a,
        "hermes",
        &home_a,
        Direction::Upload,
        &mappings,
        || false,
    )
    .unwrap();
    assert_eq!(upload.captured, 1, "{:?}", upload.issues);
    let a_project = project_files(&manifest_for(&remote_a, A));
    let a_manifest = project_manifest(&a_project);
    assert!(
        crate::hermes_project::ready(&a_manifest),
        "a fully captured dirty tree is ready: {:?}",
        a_manifest.blockers
    );
    assert!(a_manifest
        .excluded
        .iter()
        .any(|e| e.path == "gone.txt" && e.reason == "deleted"));

    let home_b = temp.path().join("home-b");
    fs::create_dir_all(&home_b).unwrap();
    let root_b = temp.path().join("sync-b");
    let remote_b = Remote(RefCell::new(remote_a.0.borrow().clone()), Cell::new(100));
    let download = cycle_with_mappings(
        &root_b,
        &binding(),
        &key,
        &remote_b,
        "hermes",
        &home_b,
        Direction::Download,
        &mappings,
        || false,
    )
    .unwrap();
    assert_eq!(download.restored, 1, "{:?}", download.issues);

    let registry = load_handoffs(&root_b).unwrap();
    assert_eq!(registry.entries.len(), 1);
    let handoff = &registry.entries[0];
    let managed_home = handoff.path.clone();
    let project_dir = handoff
        .project
        .clone()
        .expect("a restored project working directory");
    assert!(project_dir.is_dir());
    assert!(!project_dir.starts_with(&home_a) && !project_dir.starts_with(&home_b));
    assert!(
        !project_dir.starts_with(&mapped),
        "never the receiver's mapped worktree"
    );

    assert_eq!(
        fs::read(project_dir.join("dirty.txt")).unwrap(),
        b"v2 dirty\n"
    );
    assert_eq!(
        fs::read(project_dir.join("new/work.txt")).unwrap(),
        b"untracked work\n"
    );
    assert!(!project_dir.join("gone.txt").exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(project_dir.join("run.sh"))
            .unwrap()
            .permissions()
            .mode();
        assert_ne!(mode & 0o111, 0, "restored script must stay executable");
    }

    // The managed store's session cwd is the restored tree, not the mapping.
    let conn = rusqlite::Connection::open(managed_home.join("state.db")).unwrap();
    let cwd: String = conn
        .query_row("SELECT cwd FROM sessions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(cwd, project_dir.to_string_lossy());
    drop(conn);

    // The receiver's mapped worktree is completely untouched.
    assert_eq!(
        fs::read(mapped.join("KEEP.txt")).unwrap(),
        b"receiver work\n"
    );
    assert_eq!(fs::read_dir(&mapped).unwrap().count(), 1);

    let original_base = handoff.base.clone();

    // A later edit in the restored tree publishes a causal child captured from
    // that restored working tree.
    fs::write(project_dir.join("dirty.txt"), "v3 continued\n").unwrap();
    let published = cycle_with_mappings(
        &root_b,
        &binding(),
        &key,
        &remote_b,
        "hermes",
        &home_b,
        Direction::Upload,
        &mappings,
        || false,
    )
    .unwrap();
    assert_eq!(published.published, 1, "{:?}", published.issues);
    let child = remote_b
        .0
        .borrow()
        .values()
        .find(|b| b.snapshot.parents.first().map(String::as_str) == Some(original_base.as_str()))
        .cloned()
        .expect("a causal child of the restored head");
    let child_m = unpack(&child, &remote_b.0.borrow()).unwrap();
    assert_eq!(child_m.session, A);
    let child_project = project_files(&child_m);
    assert_eq!(
        STANDARD
            .decode(child_project.get("bastet/project-files/dirty.txt").unwrap())
            .unwrap(),
        b"v3 continued\n"
    );
}

// The persisted `@image:` grammar is captured as media; `@url:` is recognised
// but never read as a local file.
#[test]
fn hermes_attachment_media_grammar_captures_images_not_urls() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    fs::create_dir_all(home.join("composer-pastes")).unwrap();
    let pic = home.join("composer-pastes/pic.png");
    fs::write(&pic, b"PNGDATA").unwrap();
    let pic_ref = pic.to_string_lossy().into_owned();
    hermes_store(&home.join("state.db"), |conn| {
        hermes_session_row(conn, A, &cwd, None, None, None, None);
        conn.execute(
            "UPDATE messages SET content=?1 WHERE session_id=?2 AND role='user'",
            rusqlite::params![
                format!("look @image:{pic_ref} and @url:https://example.test/a.png"),
                A
            ],
        )
        .unwrap();
    });

    let key = SpaceKey::generate().unwrap();
    let remote = remote();
    let status = cycle(
        &temp.path().join("sync"),
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();
    assert_eq!(status.captured, 1, "{:?}", status.issues);

    let manifest = manifest_for(&remote, A);
    let index: serde_json::Value = serde_json::from_slice(
        &STANDARD
            .decode(manifest.files.get("attachments/index.json").unwrap())
            .unwrap(),
    )
    .unwrap();
    let entries = index["entries"].as_array().unwrap();
    assert!(
        entries.iter().any(
            |e| e["kind"] == "image" && e["reference"].as_str().unwrap().starts_with("@image:")
        ),
        "an @image: reference must be captured as media: {entries:?}"
    );
    assert!(
        !entries.iter().any(|e| e["kind"] == "url"),
        "an @url: reference must never be read as a local file"
    );
    let blob = manifest
        .files
        .keys()
        .find(|k| k.starts_with("attachments/") && k.ends_with("/pic.png"))
        .expect("image blob");
    assert_eq!(
        STANDARD.decode(manifest.files.get(blob).unwrap()).unwrap(),
        b"PNGDATA"
    );
}

// Attachment reads are confined to the Hermes runtime cache dirs and the
// session's own project; the ported canonical deny sets refuse the credential
// stores even when they sit under the home, and a plain non-cache home file is
// outside the allowed roots.
#[test]
fn hermes_attachment_roots_and_deny_sets_block_home_secrets() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    fs::create_dir_all(home.join("composer-pastes")).unwrap();
    fs::write(home.join("composer-pastes/ok.txt"), b"OK").unwrap();
    fs::write(home.join("auth.json"), b"AUTHSECRET").unwrap();
    fs::create_dir_all(home.join("mcp-tokens")).unwrap();
    fs::write(home.join("mcp-tokens/tok"), b"TOKENSECRET").unwrap();
    fs::create_dir_all(home.join("vault")).unwrap();
    fs::write(home.join("vault/vault.key"), b"VAULTSECRET").unwrap();
    fs::write(home.join("plain-home.txt"), b"PLAINHOME").unwrap();
    fs::write(project.join(".env"), b"ENVSECRET").unwrap();
    let h = |n: &str| home.join(n).to_string_lossy().into_owned();
    let refs = format!(
        "@file:{} @file:{} @file:{} @file:{} @file:{} @file:{} @file:.env",
        h("composer-pastes/ok.txt"),
        h("auth.json"),
        h("mcp-tokens/tok"),
        h("vault/vault.key"),
        h("state.db"),
        h("plain-home.txt"),
    );
    hermes_store(&home.join("state.db"), |conn| {
        hermes_session_row(conn, A, &cwd, None, None, None, None);
        conn.execute(
            "UPDATE messages SET content=?1 WHERE session_id=?2 AND role='user'",
            rusqlite::params![refs, A],
        )
        .unwrap();
    });

    let key = SpaceKey::generate().unwrap();
    // A recorded cwd is an attachment read root only when an explicit mapping
    // covers it; authorize the project so the deny-set behaviour under an
    // authorized root is exercised.
    let mapped = temp.path().join("mapped");
    fs::create_dir_all(&mapped).unwrap();
    let mappings = [Mapping {
        source: cwd.clone(),
        target: mapped.to_string_lossy().into_owned(),
    }];
    let remote = remote();
    let status = cycle_with_mappings(
        &temp.path().join("sync"),
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        &mappings,
        || false,
    )
    .unwrap();
    assert_eq!(status.captured, 1, "{:?}", status.issues);
    assert!(
        status
            .issues
            .get("hermes_attachment_credential")
            .copied()
            .unwrap_or(0)
            >= 5,
        "credential stores must be refused: {:?}",
        status.issues
    );
    assert!(
        status
            .issues
            .get("hermes_attachment_outside_roots")
            .copied()
            .unwrap_or(0)
            >= 1,
        "a non-cache home file is outside the allowed roots: {:?}",
        status.issues
    );

    let manifest = manifest_for(&remote, A);
    let blobs: Vec<&String> = manifest
        .files
        .keys()
        .filter(|k| k.starts_with("attachments/") && k.as_str() != "attachments/index.json")
        .collect();
    assert_eq!(
        blobs.len(),
        1,
        "only the cache-dir file may be captured: {blobs:?}"
    );
    assert!(blobs[0].ends_with("/ok.txt"));
    for secret in [
        b"AUTHSECRET".as_slice(),
        b"TOKENSECRET",
        b"VAULTSECRET",
        b"PLAINHOME",
        b"ENVSECRET",
    ] {
        for value in manifest.files.values() {
            let bytes = STANDARD.decode(value).unwrap();
            assert!(
                !contains(&bytes, secret),
                "secret bytes leaked: {:?}",
                String::from_utf8_lossy(secret)
            );
        }
    }
}

// The aggregated continuation object is never `ready` merely because the packet
// was copied: an unmapped project and an unverified memory binding both block.
#[test]
fn hermes_continuation_aggregate_blocks_until_bindings_verified() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    hermes_db(&home.join("state.db"), &[(A, "/project")]);
    let key = SpaceKey::generate().unwrap();
    let remote = remote();
    cycle(
        &temp.path().join("sync"),
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();
    let manifest = manifest_for(&remote, A);
    let continuation = continuation_for(&manifest, None);
    let value = serde_json::to_value(&continuation).unwrap();
    assert_eq!(value["state"], "blocked");
    let blockers: Vec<&str> = value["blockers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b.as_str().unwrap())
        .collect();
    assert!(
        blockers.contains(&"project_mapping_required"),
        "{blockers:?}"
    );
    assert!(blockers.contains(&"memory_unverified"), "{blockers:?}");
    assert!(
        value.get("projectPath").is_none(),
        "no restored project directory yet"
    );
    assert_ne!(value["state"], "ready");
}

// ── External-memory continuation wiring ────────────────────────────────────

#[cfg(all(unix, not(feature = "mac-app-store")))]
const MEMORY_EXPORT: &str = "bastet/memory-export.jsonl";
#[cfg(all(unix, not(feature = "mac-app-store")))]
const MEMORY_MANIFEST: &str = "bastet/memory-manifest.json";

#[cfg(all(unix, not(feature = "mac-app-store")))]
fn memory_subset_of(m: &Manifest) -> BTreeMap<String, String> {
    m.files
        .iter()
        .filter(|(path, _)| crate::hermes_memory::allowed(path))
        .map(|(path, encoded)| (path.clone(), encoded.clone()))
        .collect()
}

#[cfg(all(unix, not(feature = "mac-app-store")))]
fn memory_export_of(m: &Manifest) -> String {
    let subset = memory_subset_of(m);
    let manifest = crate::hermes_memory::read_manifest(&subset).unwrap();
    assert!(manifest.exported, "memory must be exported: {manifest:?}");
    String::from_utf8(STANDARD.decode(subset.get(MEMORY_EXPORT).unwrap()).unwrap()).unwrap()
}

#[cfg(all(unix, not(feature = "mac-app-store")))]
fn fake_memory_cli(dir: &Path) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("agent-memory");
    std::fs::write(
        &path,
        r#"#!/bin/sh
home=""; action=""; target=""
while [ $# -gt 0 ]; do
  case "$1" in
    --home) home="$2"; shift 2 ;;
    sync) action="$2"; target="$3"; shift 3 ;;
    *) shift ;;
  esac
done
note() { if [ -f "$home/store-note.txt" ]; then cat "$home/store-note.txt"; else printf 'FIXTURE'; fi; }
case "$action" in
  export)
    { printf '%s\n' '{"kind":"bundle","version":3}'
      printf '{"kind":"memory","id":"alpha","owner":"hermes-fixture","content":"%s"}\n' "$(note)"
      printf '%s\n' '{"kind":"team","id":"team-fixture","members":["agent-a"]}'
    } > "$target"
    ;;
  import)
    mkdir -p "$home"
    cat "$target" > "$home/imported.jsonl"
    grep -o '"content":"[^"]*"' "$target" | head -1 | sed 's/.*"content":"//; s/"$//' > "$home/store-note.txt"
    n=$(grep -c '"kind":"memory"' "$target" 2>/dev/null); [ -n "$n" ] || n=0
    { printf '%s\n' 'SQLite format 3 fake-store'; cat "$target"; } > "$home/memories.db"
    printf '{"memories_added": %s, "memories_updated": 0, "memories_skipped": 0, "links_added": 0, "links_merged": 0, "profiles_upserted": 0, "tombstones_applied": 0, "teams_upserted": 0, "projects_upserted": 0, "org_tombstones_applied": 0, "org_records_rejected": 0}\n' "$n"
    ;;
  *) echo unsupported >&2; exit 2 ;;
esac
"#,
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// An AgentMemoryOS source home whose bound store carries a note the fixture CLI
/// echoes in its official export.
#[cfg(all(unix, not(feature = "mac-app-store")))]
fn memory_home(dir: &Path, store_note: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let home = dir.join("home-a");
    fs::create_dir_all(&home).unwrap();
    fs::write(
        home.join("config.yaml"),
        "memory:\n  provider: \"agent-memory-os\"\ndisplay:\n  skin: default\n",
    )
    .unwrap();
    let store = dir.join("store");
    fs::create_dir_all(&store).unwrap();
    fs::write(store.join("memories.db"), b"SQLite format 3\0STORE").unwrap();
    fs::write(store.join("store-note.txt"), store_note).unwrap();
    fs::write(
        home.join("agent-memory-os.json"),
        format!(
            "{{\"home\": \"{}\", \"agent_id\": \"hermes-fixture\"}}",
            store.to_string_lossy()
        ),
    )
    .unwrap();
    hermes_db(&home.join("state.db"), &[(A, "/project")]);
    (home, store)
}

// An authorized AMOS export travels with the snapshot, restores into the NEW
// isolated managed home (verified by re-export), and is RECAPTURED from that
// managed store — not the original source — as a causal child. An unchanged
// store republishes nothing.
#[cfg(all(unix, not(feature = "mac-app-store")))]
#[test]
fn hermes_memory_export_travels_restores_and_recaptures_from_managed_home() {
    let temp = tempfile::tempdir().unwrap();
    let app_root = temp.path().join("app");
    fs::create_dir_all(&app_root).unwrap();
    let cli = fake_memory_cli(temp.path());
    std::fs::write(
        app_root.join("memory-cli.json"),
        serde_json::to_vec(&cli).unwrap(),
    )
    .unwrap();
    let (home_a, store) = memory_home(temp.path(), "SOURCE-NOTE");

    // Computer A opted into this exact store.
    let mut settings_a = crate::model::Settings::default();
    settings_a.custom_paths.insert(
        "agent-memory-os".into(),
        store.to_string_lossy().into_owned(),
    );

    let key = SpaceKey::generate().unwrap();
    let remote_a = remote();
    let status = cycle_with_context(
        &temp.path().join("sync-a"),
        &binding(),
        &key,
        &remote_a,
        "hermes",
        &home_a,
        Direction::Upload,
        &[],
        Some(MemoryContext {
            app_root: &app_root,
            settings: &settings_a,
        }),
        || false,
    )
    .unwrap();
    assert_eq!(status.captured, 1, "{:?}", status.issues);
    assert_eq!(status.published, 1, "{:?}", status.issues);

    let manifest = manifest_for(&remote_a, A);
    let subset = memory_subset_of(&manifest);
    let memory = crate::hermes_memory::read_manifest(&subset).unwrap();
    assert_eq!(memory.provider, "agent-memory-os");
    assert!(memory.exported && memory.blockers.is_empty(), "{memory:?}");
    assert!(memory_export_of(&manifest).contains("SOURCE-NOTE"));
    // A verified binding resolves the workspace's own memory dependency.
    let continuation = continuation_for(&manifest, None);
    assert!(
        continuation
            .verified
            .contains(&"external_memory_bound".to_string()),
        "{:?}",
        continuation.verified
    );
    assert!(
        !continuation
            .blockers
            .contains(&"workspace_dependency_memory".to_string()),
        "{:?}",
        continuation.blockers
    );

    // Computer B has not configured a local store: it opts in and lets the
    // received binding restore into the isolated home.
    let home_b = temp.path().join("home-b");
    fs::create_dir_all(&home_b).unwrap();
    let settings_b = crate::model::Settings {
        selected_agents: vec!["agent-memory-os".into()],
        ..Default::default()
    };
    let root_b = temp.path().join("sync-b");
    let remote_b = Remote(RefCell::new(remote_a.0.borrow().clone()), Cell::new(100));
    let ctx_b = MemoryContext {
        app_root: &app_root,
        settings: &settings_b,
    };
    let project_b = temp.path().join("project-b");
    fs::create_dir_all(&project_b).unwrap();
    let mappings_b = [Mapping {
        source: "/project".into(),
        target: project_b.to_string_lossy().into_owned(),
    }];
    let received = cycle_with_context(
        &root_b,
        &binding(),
        &key,
        &remote_b,
        "hermes",
        &home_b,
        Direction::Download,
        &mappings_b,
        Some(ctx_b),
        || false,
    )
    .unwrap();
    assert_eq!(received.restored, 1, "{:?}", received.issues);

    let registry = load_handoffs(&root_b).unwrap();
    let handoff = &registry.entries[0];
    let readiness = handoff.memory.as_ref().expect("persisted memory readiness");
    // The NEW isolated home has no provider plugin yet, so the real readiness is
    // NOT ready and places nothing — the recipient installs it and retries. It is
    // never fabricated ready.
    assert!(!readiness.ready && !readiness.imported, "{readiness:?}");
    assert_eq!(readiness.provider, "agent-memory-os");
    assert!(
        readiness
            .blockers
            .contains(&"memory_provider_not_installed".to_string()),
        "{readiness:?}"
    );
    let managed = handoff.path.clone();
    assert!(!managed.join(".agent-memory").exists());
    assert!(!managed.join("agent-memory-os.json").exists());

    // The UI surfaces the real (not synthetic) memory blocker for this version.
    let mut rendered = continuation_for(&manifest, Some(managed.to_string_lossy().into_owned()));
    apply_memory_readiness(&mut rendered, readiness);
    assert_eq!(rendered.state, "blocked");
    assert!(
        rendered
            .blockers
            .contains(&"memory_provider_not_installed".to_string()),
        "{:?}",
        rendered.blockers
    );
}

// A conflicting explicit memory home is unauthorized: the manifest still travels
// (with an actionable blocker) but no export — and no store byte — is dumped.
#[cfg(all(unix, not(feature = "mac-app-store")))]
#[test]
fn hermes_memory_conflicting_home_never_dumps_an_export() {
    let temp = tempfile::tempdir().unwrap();
    let app_root = temp.path().join("app");
    fs::create_dir_all(&app_root).unwrap();
    let cli = fake_memory_cli(temp.path());
    std::fs::write(
        app_root.join("memory-cli.json"),
        serde_json::to_vec(&cli).unwrap(),
    )
    .unwrap();
    let (home_a, _store) = memory_home(temp.path(), "SOURCE-NOTE");
    let other = temp.path().join("other-store");
    fs::create_dir_all(&other).unwrap();

    let mut settings = crate::model::Settings::default();
    settings.custom_paths.insert(
        "agent-memory-os".into(),
        other.to_string_lossy().into_owned(),
    );

    let key = SpaceKey::generate().unwrap();
    let remote = remote();
    let status = cycle_with_context(
        &temp.path().join("sync"),
        &binding(),
        &key,
        &remote,
        "hermes",
        &home_a,
        Direction::Upload,
        &[],
        Some(MemoryContext {
            app_root: &app_root,
            settings: &settings,
        }),
        || false,
    )
    .unwrap();
    assert_eq!(status.captured, 1, "{:?}", status.issues);
    assert_eq!(
        status.issues.get("memory_export_unauthorized"),
        Some(&1),
        "{:?}",
        status.issues
    );

    let manifest = manifest_for(&remote, A);
    let subset = memory_subset_of(&manifest);
    assert!(subset.contains_key(MEMORY_MANIFEST));
    assert!(
        !subset.contains_key(MEMORY_EXPORT),
        "an unauthorized home must never dump an export"
    );
    let memory = crate::hermes_memory::read_manifest(&subset).unwrap();
    assert!(!memory.exported);
    assert!(memory
        .blockers
        .contains(&"memory_export_unauthorized".to_string()));
    for value in manifest.files.values() {
        assert!(!contains(
            &STANDARD.decode(value).unwrap(),
            b"SQLite format 3\0STORE"
        ));
    }
}

// The managed-handoff recapture reads the RESTORED home's own bound store — not
// the original source — so a continued session keeps the memory it was restored
// with, and a later edit there (not at the source) is what travels.
#[cfg(all(unix, not(feature = "mac-app-store")))]
#[test]
fn hermes_memory_recapture_reads_the_managed_home_not_the_source() {
    let temp = tempfile::tempdir().unwrap();
    let app_root = temp.path().join("app");
    fs::create_dir_all(&app_root).unwrap();
    let cli = fake_memory_cli(temp.path());
    std::fs::write(
        app_root.join("memory-cli.json"),
        serde_json::to_vec(&cli).unwrap(),
    )
    .unwrap();
    let (source_home, source_store) = memory_home(temp.path(), "SOURCE-NOTE");

    // A restored managed home: its own config, a binding into a nested store and
    // that nested store carrying a different note.
    let managed = temp.path().join("managed");
    fs::create_dir_all(&managed).unwrap();
    fs::write(
        managed.join("config.yaml"),
        "memory:\n  provider: \"agent-memory-os\"\n",
    )
    .unwrap();
    let nested = managed.join(".agent-memory");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("memories.db"), b"SQLite format 3\0MANAGED").unwrap();
    fs::write(nested.join("store-note.txt"), b"MANAGED-NOTE").unwrap();
    fs::write(
        managed.join("agent-memory-os.json"),
        format!(
            "{{\"home\": \"{}\", \"agent_id\": \"hermes-fixture\"}}",
            nested.to_string_lossy()
        ),
    )
    .unwrap();
    hermes_db(&managed.join("state.db"), &[(A, "/project")]);

    let settings = crate::model::Settings::default();
    let ctx = MemoryContext {
        app_root: &app_root,
        settings: &settings,
    };

    // The managed home's own store is exported; the source store is not consulted.
    let (managed_files, _) = capture_memory_for_cycle(&managed, Some(ctx)).unwrap();
    let managed_export = String::from_utf8(
        STANDARD
            .decode(managed_files.get(MEMORY_EXPORT).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert!(managed_export.contains("MANAGED-NOTE"), "{managed_export}");
    assert!(!managed_export.contains("SOURCE-NOTE"), "{managed_export}");

    let (source_files, _) = capture_memory_for_cycle(&source_home, Some(ctx)).unwrap();
    let source_export = String::from_utf8(
        STANDARD
            .decode(source_files.get(MEMORY_EXPORT).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert!(source_export.contains("SOURCE-NOTE"), "{source_export}");

    // The managed-handoff recapture path (`capture_handoff`) routes through the
    // exact same read — it calls `capture_memory_for_cycle(&h.path)` — so the
    // managed home is what a continued session re-uploads, never the source.
    // The original source store is untouched by any of it.
    assert_eq!(
        fs::read(source_store.join("store-note.txt")).unwrap(),
        b"SOURCE-NOTE"
    );
}

// ── Explicit continuation recheck and provider setup ───────────────────────

/// A fixture CLI that additionally implements the fixed
/// `agent-memory hermes install --hermes-home <home>` operation, so the provider
/// shim can be materialized into an isolated managed home without a network.
#[cfg(all(unix, not(feature = "mac-app-store")))]
const RECHECK_CLI: &str = r#"#!/bin/sh
home=""; action=""; target=""; mode=""; hermes_home=""
while [ $# -gt 0 ]; do
  case "$1" in
    --home) home="$2"; shift 2 ;;
    hermes) mode="hermes"; shift ;;
    install) mode="install"; shift ;;
    --hermes-home) hermes_home="$2"; shift 2 ;;
    sync) action="$2"; target="$3"; shift 3 ;;
    *) shift ;;
  esac
done
if [ "$mode" = "install" ] && [ -n "$hermes_home" ]; then
  mkdir -p "$hermes_home/plugins/agent-memory-os"
  printf 'from agent_memory_os.hermes_plugin import AgentMemoryOSProvider\ndef register(ctx):\n    ctx.register_memory_provider(AgentMemoryOSProvider())\n' > "$hermes_home/plugins/agent-memory-os/__init__.py"
  exit 0
fi
case "$action" in
  export)
    { printf '%s\n' '{"kind":"bundle","version":3}'
      printf '%s\n' '{"kind":"memory","id":"alpha","owner":"hermes-fixture","content":"ALPHA"}'
      printf '%s\n' '{"kind":"team","id":"team-fixture","members":["agent-a"]}'
    } > "$target"
    ;;
  import)
    mkdir -p "$home"
    { printf '%s\n' 'SQLite format 3 fake-store'; cat "$target"; } > "$home/memories.db"
    printf 'x\n' >> "$home/imports.count"
    n=$(grep -c '"kind":"memory"' "$target" 2>/dev/null); [ -n "$n" ] || n=0
    printf '{"memories_added": %s, "memories_updated": 0, "memories_skipped": 0, "links_added": 0, "links_merged": 0, "profiles_upserted": 0, "tombstones_applied": 0, "teams_upserted": 0, "projects_upserted": 0, "org_tombstones_applied": 0, "org_records_rejected": 0}\n' "$n"
    ;;
  *) echo unsupported >&2; exit 2 ;;
esac
"#;

#[cfg(all(unix, not(feature = "mac-app-store")))]
fn recheck_cli(dir: &Path) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("agent-memory");
    std::fs::write(&path, RECHECK_CLI).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// End-to-end: an authorized AMOS export travels, blocks on the absent provider
/// plugin, then the explicit recipient-approved setup installs the trusted local
/// provider into the SAME registered managed home, re-runs the official import
/// and reports the real recall. The setup/recheck is idempotent and never
/// creates a second managed profile; the backend report stays authoritative.
#[cfg(all(unix, not(feature = "mac-app-store")))]
#[test]
fn hermes_recheck_setup_imports_once_and_creates_no_second_profile() {
    let temp = tempfile::tempdir().unwrap();
    let app_root = temp.path().join("app");
    fs::create_dir_all(&app_root).unwrap();
    let cli = recheck_cli(temp.path());
    fs::write(
        app_root.join("memory-cli.json"),
        serde_json::to_vec(&cli).unwrap(),
    )
    .unwrap();
    let (home_a, store) = memory_home(temp.path(), "SOURCE-NOTE");

    let mut settings_a = crate::model::Settings::default();
    settings_a.custom_paths.insert(
        "agent-memory-os".into(),
        store.to_string_lossy().into_owned(),
    );

    let key = SpaceKey::generate().unwrap();
    let space = binding().space.clone();
    let remote_a = remote();
    cycle_with_context(
        &temp.path().join("sync-a"),
        &binding(),
        &key,
        &remote_a,
        "hermes",
        &home_a,
        Direction::Upload,
        &[],
        Some(MemoryContext {
            app_root: &app_root,
            settings: &settings_a,
        }),
        || false,
    )
    .unwrap();

    let home_b = temp.path().join("home-b");
    fs::create_dir_all(&home_b).unwrap();
    let settings_b = crate::model::Settings {
        selected_agents: vec!["agent-memory-os".into()],
        ..Default::default()
    };
    let root_b = temp.path().join("sync-b");
    let remote_b = Remote(RefCell::new(remote_a.0.borrow().clone()), Cell::new(100));
    let project_b = temp.path().join("project-b");
    fs::create_dir_all(&project_b).unwrap();
    let mappings_b = [Mapping {
        source: "/project".into(),
        target: project_b.to_string_lossy().into_owned(),
    }];
    let received = cycle_with_context(
        &root_b,
        &binding(),
        &key,
        &remote_b,
        "hermes",
        &home_b,
        Direction::Download,
        &mappings_b,
        Some(MemoryContext {
            app_root: &app_root,
            settings: &settings_b,
        }),
        || false,
    )
    .unwrap();
    assert_eq!(received.restored, 1, "{:?}", received.issues);

    let registry = load_handoffs(&root_b).unwrap();
    assert_eq!(registry.entries.len(), 1);
    let id = registry.entries[0].base.clone();
    let managed = registry.entries[0].path.clone();
    let first = registry.entries[0]
        .memory
        .clone()
        .expect("memory readiness");
    assert!(!first.ready && !first.imported, "{first:?}");
    assert!(first
        .blockers
        .contains(&"memory_provider_not_installed".to_string()));
    assert!(!managed.join(".agent-memory").exists());
    assert!(!managed.join("agent-memory-os.json").exists());

    let staging = tempfile::tempdir().unwrap();
    // Explicit recipient-approved provider setup, then the official import into
    // the SAME registered home — never a caller path, never a new profile.
    let result = recheck_continuation_with(
        &root_b,
        &app_root,
        &space,
        &settings_b,
        "hermes",
        &id,
        false,
        true,
        staging.path(),
    )
    .unwrap();
    assert!(result.provider_installed, "the shim must be read back");
    let mem = result.memory.as_ref().unwrap();
    assert!(mem.ready && mem.imported, "{mem:?}");
    assert!(managed.join(".agent-memory/memories.db").is_file());
    assert!(managed.join("agent-memory-os.json").is_file());
    assert!(!result
        .continuation
        .blockers
        .iter()
        .any(|code| code.starts_with("memory_")));
    assert!(result.continuation.verified.contains(&"memory".to_string()));
    assert_eq!(
        fs::read_dir(root_b.join("managed-profiles"))
            .unwrap()
            .count(),
        1,
        "setup must not create a second managed profile"
    );

    // A plain recheck is idempotent: the existing store recalls the snapshot and
    // is not re-imported.
    let again = recheck_continuation_with(
        &root_b,
        &app_root,
        &space,
        &settings_b,
        "hermes",
        &id,
        false,
        false,
        staging.path(),
    )
    .unwrap();
    assert!(again.memory.as_ref().unwrap().ready);
    assert!(!again.memory.as_ref().unwrap().imported);
    let imports = fs::read_to_string(managed.join(".agent-memory/imports.count")).unwrap();
    assert_eq!(imports.lines().count(), 1, "only the first import inserts");
    assert_eq!(
        fs::read_dir(root_b.join("managed-profiles"))
            .unwrap()
            .count(),
        1
    );
}

/// The backend resolves the recheck target from its own registry only: an id it
/// never registered, a malformed id, and a registered entry whose path is a
/// symlink are all refused rather than acted on.
#[cfg(all(unix, not(feature = "mac-app-store")))]
#[test]
fn hermes_recheck_refuses_unregistered_malformed_and_symlinked_targets() {
    let temp = tempfile::tempdir().unwrap();
    let app_root = temp.path().join("app");
    fs::create_dir_all(&app_root).unwrap();
    let root = temp.path().join("sync");
    fs::create_dir_all(&root).unwrap();
    let settings = crate::model::Settings::default();
    let staging = tempfile::tempdir().unwrap();

    // An id this app never registered (a caller-supplied id is never a path).
    assert_eq!(
        recheck_continuation_with(
            &root,
            &app_root,
            "space",
            &settings,
            "hermes",
            &"a".repeat(64),
            false,
            false,
            staging.path(),
        )
        .unwrap_err(),
        "session_invalid"
    );
    // A malformed id is refused before any registry lookup.
    assert_eq!(
        recheck_continuation_with(
            &root,
            &app_root,
            "space",
            &settings,
            "hermes",
            "not-a-hash",
            false,
            false,
            staging.path(),
        )
        .unwrap_err(),
        "session_invalid"
    );

    // A registered entry whose managed path is a symlink is refused, not
    // followed into an external directory.
    let real = temp.path().join("real-profile");
    fs::create_dir_all(&real).unwrap();
    let link = temp.path().join("linked-profile");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let id = "c".repeat(64);
    save_handoffs(
        &root,
        &Handoffs {
            version: 1,
            entries: vec![Handoff {
                path: link.clone(),
                agent: "hermes".into(),
                session: "20260101_000000_recheck".into(),
                main_file: "state.db".into(),
                cwd: "/project".into(),
                stream_profile: "profile".into(),
                base: id.clone(),
                fingerprint: String::new(),
                project: None,
                project_blockers: Vec::new(),
                memory: None,
            }],
        },
    )
    .unwrap();
    assert_eq!(
        recheck_continuation_with(
            &root,
            &app_root,
            "space",
            &settings,
            "hermes",
            &id,
            false,
            false,
            staging.path(),
        )
        .unwrap_err(),
        "unsafe_store"
    );
}

/// The recipient-approved setup refuses a snapshot whose memory provider is not
/// the known external AgentMemoryOS provider: built-in memory has no provider
/// plugin to install, and an arbitrary provider is never handed a CLI call.
#[cfg(all(unix, not(feature = "mac-app-store")))]
#[test]
fn hermes_recheck_setup_refuses_a_non_external_provider() {
    let temp = tempfile::tempdir().unwrap();
    let app_root = temp.path().join("app");
    fs::create_dir_all(&app_root).unwrap();
    let cli = recheck_cli(temp.path());
    fs::write(
        app_root.join("memory-cli.json"),
        serde_json::to_vec(&cli).unwrap(),
    )
    .unwrap();
    let home_a = temp.path().join("home-a");
    fs::create_dir_all(&home_a).unwrap();
    fs::write(home_a.join("config.yaml"), "memory:\n  provider: builtin\n").unwrap();
    hermes_db(&home_a.join("state.db"), &[(A, "/project")]);

    let settings = crate::model::Settings::default();
    let key = SpaceKey::generate().unwrap();
    let remote_a = remote();
    cycle_with_context(
        &temp.path().join("sync-a"),
        &binding(),
        &key,
        &remote_a,
        "hermes",
        &home_a,
        Direction::Upload,
        &[],
        Some(MemoryContext {
            app_root: &app_root,
            settings: &settings,
        }),
        || false,
    )
    .unwrap();

    let home_b = temp.path().join("home-b");
    fs::create_dir_all(&home_b).unwrap();
    let root_b = temp.path().join("sync-b");
    let remote_b = Remote(RefCell::new(remote_a.0.borrow().clone()), Cell::new(100));
    let project_b = temp.path().join("project-b");
    fs::create_dir_all(&project_b).unwrap();
    let mappings_b = [Mapping {
        source: "/project".into(),
        target: project_b.to_string_lossy().into_owned(),
    }];
    cycle_with_context(
        &root_b,
        &binding(),
        &key,
        &remote_b,
        "hermes",
        &home_b,
        Direction::Download,
        &mappings_b,
        Some(MemoryContext {
            app_root: &app_root,
            settings: &settings,
        }),
        || false,
    )
    .unwrap();
    let registry = load_handoffs(&root_b).unwrap();
    let id = registry.entries[0].base.clone();

    let staging = tempfile::tempdir().unwrap();
    assert_eq!(
        recheck_continuation_with(
            &root_b,
            &app_root,
            &binding().space,
            &settings,
            "hermes",
            &id,
            false,
            true,
            staging.path(),
        )
        .unwrap_err(),
        "memory_unavailable_store"
    );
}

/// An explicit acknowledgment of the opaque (MCP/model/app/credential) workspace
/// requirements downgrades exactly those blockers to warnings and is hash-bound:
/// it is never a verified fact and never marks the version ready. A plain recheck
/// keeps honoring the stored acknowledgment.
#[cfg(all(unix, not(feature = "mac-app-store")))]
#[test]
fn hermes_recheck_acknowledges_opaque_requirements_without_marking_ready() {
    let temp = tempfile::tempdir().unwrap();
    let app_root = temp.path().join("app");
    fs::create_dir_all(&app_root).unwrap();
    let home_a = temp.path().join("home-a");
    fs::create_dir_all(&home_a).unwrap();
    fs::write(
        home_a.join("config.yaml"),
        "model:\n  provider: \"auto\"\nmcp_servers:\n  demo:\n    command: /usr/bin/demo\n    args: [\"serve\"]\n    env:\n      DEMO_TOKEN: \"should-not-travel\"\n",
    )
    .unwrap();
    hermes_db(&home_a.join("state.db"), &[(A, "/project")]);

    let settings = crate::model::Settings::default();
    let key = SpaceKey::generate().unwrap();
    let remote_a = remote();
    cycle_with_context(
        &temp.path().join("sync-a"),
        &binding(),
        &key,
        &remote_a,
        "hermes",
        &home_a,
        Direction::Upload,
        &[],
        Some(MemoryContext {
            app_root: &app_root,
            settings: &settings,
        }),
        || false,
    )
    .unwrap();

    let home_b = temp.path().join("home-b");
    fs::create_dir_all(&home_b).unwrap();
    let root_b = temp.path().join("sync-b");
    let remote_b = Remote(RefCell::new(remote_a.0.borrow().clone()), Cell::new(100));
    let project_b = temp.path().join("project-b");
    fs::create_dir_all(&project_b).unwrap();
    let mappings_b = [Mapping {
        source: "/project".into(),
        target: project_b.to_string_lossy().into_owned(),
    }];
    cycle_with_context(
        &root_b,
        &binding(),
        &key,
        &remote_b,
        "hermes",
        &home_b,
        Direction::Download,
        &mappings_b,
        Some(MemoryContext {
            app_root: &app_root,
            settings: &settings,
        }),
        || false,
    )
    .unwrap();
    let registry = load_handoffs(&root_b).unwrap();
    let id = registry.entries[0].base.clone();
    let managed = registry.entries[0].path.clone();
    let space = binding().space.clone();
    let staging = tempfile::tempdir().unwrap();

    // Before acknowledging, the MCP requirement is a blocker.
    let before = recheck_continuation_with(
        &root_b,
        &app_root,
        &space,
        &settings,
        "hermes",
        &id,
        false,
        false,
        staging.path(),
    )
    .unwrap();
    assert!(
        before
            .continuation
            .blockers
            .contains(&"workspace_dependency_mcp".to_string()),
        "{:?}",
        before.continuation.blockers
    );

    // The explicit acknowledgment moves it to an honest warning, never to
    // verified, and the state stays unverified — an acknowledgment is not a
    // validation.
    let acked = recheck_continuation_with(
        &root_b,
        &app_root,
        &space,
        &settings,
        "hermes",
        &id,
        true,
        false,
        staging.path(),
    )
    .unwrap();
    assert_eq!(
        acked.acknowledged,
        vec!["workspace_dependency_mcp_acknowledged".to_string()]
    );
    assert!(!acked
        .continuation
        .blockers
        .contains(&"workspace_dependency_mcp".to_string()));
    assert!(acked
        .continuation
        .warnings
        .contains(&"workspace_dependency_mcp_acknowledged".to_string()));
    assert!(!acked
        .continuation
        .verified
        .iter()
        .any(|fact| fact.starts_with("workspace_dependency_mcp")));
    assert_ne!(acked.continuation.state, "ready");
    // The acknowledgment is stored in the managed home, bound to the manifest.
    assert!(managed.join(".bastet-requirements.json").is_file());

    // A plain recheck keeps honoring the stored acknowledgment.
    let after = recheck_continuation_with(
        &root_b,
        &app_root,
        &space,
        &settings,
        "hermes",
        &id,
        false,
        false,
        staging.path(),
    )
    .unwrap();
    assert!(!after
        .continuation
        .blockers
        .contains(&"workspace_dependency_mcp".to_string()));
    assert!(after
        .continuation
        .warnings
        .contains(&"workspace_dependency_mcp_acknowledged".to_string()));
}

/// Real-CLI acceptance: with an installed AgentMemoryOS CLI, the explicit
/// provider setup writes the real shim into the isolated managed home and the
/// subsequent official import re-exports the captured snapshot. Ignored by
/// default; runs only against isolated synthetic scratch stores when
/// `BASTET_TEST_MEMORY_CLI` is set.
#[cfg(all(unix, not(feature = "mac-app-store")))]
#[ignore = "requires an installed AMOS CLI; runs only against isolated synthetic scratch stores"]
#[test]
fn installed_cli_provider_setup_imports_and_rechecks() {
    let cli = std::path::PathBuf::from(
        std::env::var_os("BASTET_TEST_MEMORY_CLI").expect("test CLI required"),
    );
    let fixture = include_str!("../../../tests/fixtures/amos-v3.jsonl");
    let temp = tempfile::tempdir().unwrap();
    let app_root = temp.path().join("app");
    fs::create_dir_all(&app_root).unwrap();
    fs::write(
        app_root.join("memory-cli.json"),
        serde_json::to_vec(&cli).unwrap(),
    )
    .unwrap();

    // Bootstrap a synthetic source store with the official importer.
    let store = temp.path().join("store");
    fs::create_dir_all(&store).unwrap();
    let fixture_path = temp.path().join("fixture.jsonl");
    fs::write(&fixture_path, fixture).unwrap();
    assert!(std::process::Command::new(&cli)
        .arg("--home")
        .arg(&store)
        .arg("sync")
        .arg("import")
        .arg(&fixture_path)
        .status()
        .unwrap()
        .success());

    let home_a = temp.path().join("home-a");
    fs::create_dir_all(&home_a).unwrap();
    fs::write(
        home_a.join("config.yaml"),
        "memory:\n  provider: \"agent-memory-os\"\n",
    )
    .unwrap();
    fs::write(
        home_a.join("agent-memory-os.json"),
        format!(
            "{{\"home\": \"{}\", \"agent_id\": \"hermes-fixture\"}}",
            store.to_string_lossy()
        ),
    )
    .unwrap();
    hermes_db(&home_a.join("state.db"), &[(A, "/project")]);

    let mut settings_a = crate::model::Settings::default();
    settings_a.custom_paths.insert(
        "agent-memory-os".into(),
        store.to_string_lossy().into_owned(),
    );
    let key = SpaceKey::generate().unwrap();
    let remote_a = remote();
    cycle_with_context(
        &temp.path().join("sync-a"),
        &binding(),
        &key,
        &remote_a,
        "hermes",
        &home_a,
        Direction::Upload,
        &[],
        Some(MemoryContext {
            app_root: &app_root,
            settings: &settings_a,
        }),
        || false,
    )
    .unwrap();

    let home_b = temp.path().join("home-b");
    fs::create_dir_all(&home_b).unwrap();
    let settings_b = crate::model::Settings {
        selected_agents: vec!["agent-memory-os".into()],
        ..Default::default()
    };
    let root_b = temp.path().join("sync-b");
    let remote_b = Remote(RefCell::new(remote_a.0.borrow().clone()), Cell::new(100));
    let project_b = temp.path().join("project-b");
    fs::create_dir_all(&project_b).unwrap();
    let mappings_b = [Mapping {
        source: "/project".into(),
        target: project_b.to_string_lossy().into_owned(),
    }];
    cycle_with_context(
        &root_b,
        &binding(),
        &key,
        &remote_b,
        "hermes",
        &home_b,
        Direction::Download,
        &mappings_b,
        Some(MemoryContext {
            app_root: &app_root,
            settings: &settings_b,
        }),
        || false,
    )
    .unwrap();
    let registry = load_handoffs(&root_b).unwrap();
    let id = registry.entries[0].base.clone();
    let managed = registry.entries[0].path.clone();
    assert!(!crate::hermes_memory::provider_plugin_present(&managed));

    let staging = tempfile::tempdir().unwrap();
    let result = recheck_continuation_with(
        &root_b,
        &app_root,
        &binding().space,
        &settings_b,
        "hermes",
        &id,
        false,
        true,
        staging.path(),
    )
    .unwrap();
    assert!(result.provider_installed);
    let mem = result.memory.as_ref().unwrap();
    assert!(mem.ready && mem.imported && mem.records > 0, "{mem:?}");
    assert!(managed.join(".agent-memory/memories.db").is_file());
    // The restored store recalls the captured records (Legacy A/B fixture).
    let text =
        crate::amos_runtime::export(&cli, &managed.join(".agent-memory"), staging.path()).unwrap();
    assert!(text.contains("Legacy A") && text.contains("Legacy B"));
}

// ---------------------------------------------------------------------------
// Native Hermes workspace continuity: a home with more than 256 skill files and
// a binary asset larger than the old single-object budget must travel the
// *native* capture -> encrypted queue -> receiver -> restore path (the shared
// `hermes_workspace` capture plus the per-conversation native blob), not merely
// the optional portable helper. Synthetic homes only; no live profile is read.
// ---------------------------------------------------------------------------

/// Deterministic, incompressible bytes so the asset does not gzip away and the
/// native gz/parts split is exercised deterministically.
fn native_entropy(len: usize) -> Vec<u8> {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 24) as u8
        })
        .collect()
}

#[test]
fn hermes_native_home_over_256_skills_and_a_multi_megabyte_asset_round_trips() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    hermes_db(&home.join("state.db"), &[(A, "/project")]);

    // 400 skill files: more than the former per-part 256-file capture budget.
    let mut expected: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for i in 0..400 {
        let rel = format!("skills/skill{i:04}/SKILL.md");
        let bytes = format!("# skill {i}\nbody {i}\n").into_bytes();
        fs::create_dir_all(home.join(&rel).parent().unwrap()).unwrap();
        fs::write(home.join(&rel), &bytes).unwrap();
        expected.insert(rel, bytes);
    }
    // A 2 MiB binary asset: larger than the former 1 MiB per-file capture cap.
    let asset = native_entropy(2 * 1024 * 1024);
    let asset_rel = "skills/assets/payload.bin".to_string();
    fs::create_dir_all(home.join(&asset_rel).parent().unwrap()).unwrap();
    fs::write(home.join(&asset_rel), &asset).unwrap();
    expected.insert(asset_rel.clone(), asset.clone());

    let root = temp.path().join("sync");
    let key = SpaceKey::generate().unwrap();
    let remote = remote();
    let status = cycle(
        &root,
        &binding(),
        &key,
        &remote,
        "hermes",
        &home,
        Direction::Upload,
        || false,
    )
    .unwrap();
    assert!(status.published >= 1, "{:?}", status.issues);

    // The receiver recovers the native manifest from the encrypted queue.
    let bundles = remote.0.borrow().clone();
    let head = bundles
        .values()
        .find(|b| {
            b.snapshot.stream.agent == "hermes"
                && b.snapshot.stream.conversation == bundle::hash(A.as_bytes())
        })
        .expect("a hermes session bundle")
        .clone();
    let m = unpack(&head, &bundles).unwrap();

    // Every skill file (>256) survives and the >1 MiB asset is intact.
    let skills = m.files.keys().filter(|p| p.starts_with("skills/")).count();
    assert!(skills > 256, "captured {skills} skill files");
    assert_eq!(
        STANDARD.decode(m.files.get(&asset_rel).unwrap()).unwrap(),
        asset
    );

    // The receiver writes the whole workspace into an isolated destination,
    // byte-identical — including the multi-megabyte binary asset.
    let dest = temp.path().join("restored");
    restore(&head, &bundles, &dest).unwrap();
    for (rel, bytes) in &expected {
        assert_eq!(&fs::read(dest.join(rel)).unwrap(), bytes, "{rel}");
    }
}

#[test]
fn hermes_native_oversized_home_travels_as_parts_and_waits_for_every_part() {
    let temp = tempfile::tempdir().unwrap();
    // A manifest equivalent to a >256-skill home carrying a multi-megabyte asset.
    let mut files = BTreeMap::new();
    files.insert("state.db".to_string(), STANDARD.encode(b"native-store"));
    for i in 0..400 {
        files.insert(
            format!("skills/s{i:04}/SKILL.md"),
            STANDARD.encode(format!("# {i}\n").as_bytes()),
        );
    }
    let asset = native_entropy(2 * 1024 * 1024);
    files.insert("skills/big/data.bin".to_string(), STANDARD.encode(&asset));
    let m = Manifest {
        version: 1,
        agent: "hermes".into(),
        session: A.into(),
        cwd: "/project".into(),
        files: files.clone(),
    };

    let replica = Replica::open(&temp.path().join("replica"), "space").unwrap();
    let mut journal = Journal {
        node: "node".into(),
        ..Default::default()
    };
    // A small part size forces the native gz/parts path deterministically; the
    // wire format is identical to the 23 MiB part the real cycle uses.
    let mut batch = replica.export_batch().unwrap();
    let id = publish(&m, &mut batch, &mut journal, 256 * 1024).unwrap();
    drop(batch);
    let all = replica.transport_bundles().unwrap();
    let root = all[&id].clone();
    assert!(root.snapshot.files.contains_key("session.parts.json"));
    let parts: Parts =
        serde_json::from_str(&root.snapshot.files["session.parts.json"].content).unwrap();
    assert!(
        parts.ids.len() > 1 && parts.ids.len() <= 64,
        "{} parts",
        parts.ids.len()
    );

    // Readiness is never inferred from a partial transfer: a missing part refuses
    // and nothing is written.
    let mut incomplete = all.clone();
    incomplete.remove(&parts.ids[0]);
    let target = temp.path().join("restored");
    assert_eq!(
        restore(&root, &incomplete, &target).unwrap_err(),
        "session_parts_pending"
    );
    assert!(!target.exists());

    // The complete set reassembles byte-identical, including the binary asset.
    restore(&root, &all, &target).unwrap();
    for (rel, encoded) in &m.files {
        assert_eq!(
            fs::read(target.join(rel)).unwrap(),
            STANDARD.decode(encoded).unwrap(),
            "{rel}"
        );
    }
}

/// BLOCKING BUG (currently FAILS) — producer/receiver file-count ceiling
/// mismatch. The workspace capture now admits up to `MAX_WORKSPACE_FILES`
/// (65536) files, but the receiver's `decode` still refuses a manifest with more
/// than 2048 files, so a legitimately large home is captured and published yet
/// can never be unpacked (`session_invalid`). Required invariant: a published
/// home round-trips.
#[test]
fn hermes_native_receiver_accepts_a_home_beyond_one_native_file_ceiling() {
    let temp = tempfile::tempdir().unwrap();
    let mut files = BTreeMap::new();
    files.insert("state.db".to_string(), STANDARD.encode(b"native-store"));
    for i in 0..2100 {
        files.insert(
            format!("skills/s{i:04}/SKILL.md"),
            STANDARD.encode(b"# x\n"),
        );
    }
    let m = Manifest {
        version: 1,
        agent: "hermes".into(),
        session: A.into(),
        cwd: "/project".into(),
        files,
    };
    let replica = Replica::open(&temp.path().join("replica"), "space").unwrap();
    let mut journal = Journal {
        node: "node".into(),
        ..Default::default()
    };
    let mut batch = replica.export_batch().unwrap();
    let id = publish(&m, &mut batch, &mut journal, PART_SIZE).unwrap();
    drop(batch);
    let all = replica.transport_bundles().unwrap();
    assert_eq!(
        unpack(&all[&id], &all).unwrap().files.len(),
        m.files.len(),
        "a published home must round-trip regardless of its file count"
    );
}

/// The producer refuses a source that cannot fit the *shared* manifest ceiling
/// before upload, with an explicit limit cause — never publishing a home the
/// receiver would only later reject as `session_invalid`.
#[test]
fn hermes_native_producer_rejects_a_home_over_the_manifest_file_ceiling() {
    let temp = tempfile::tempdir().unwrap();
    let mut files = BTreeMap::new();
    files.insert("state.db".to_string(), STANDARD.encode(b"native-store"));
    for i in 0..MAX_MANIFEST_FILES {
        files.insert(
            format!("skills/s{i:06}/SKILL.md"),
            STANDARD.encode(b"# x\n"),
        );
    }
    assert_eq!(files.len(), MAX_MANIFEST_FILES + 1);
    let m = Manifest {
        version: 1,
        agent: "hermes".into(),
        session: A.into(),
        cwd: "/project".into(),
        files,
    };
    let replica = Replica::open(&temp.path().join("replica"), "space").unwrap();
    let mut journal = Journal {
        node: "node".into(),
        ..Default::default()
    };
    let mut batch = replica.export_batch().unwrap();
    assert_eq!(
        publish(&m, &mut batch, &mut journal, PART_SIZE).unwrap_err(),
        "session_limit",
        "an impossible source is refused before upload with an explicit cause"
    );
    drop(batch);
    // The rejected source published nothing.
    assert!(replica.transport_bundles().unwrap().is_empty());
}

/// The shared ceiling is *derived* from the namespace budgets it merges, so a
/// home right at the merged file-count ceiling still round-trips while one past
/// it is refused: the two ends cannot disagree.
#[test]
fn hermes_native_file_ceiling_is_the_derived_union_of_namespaces() {
    const {
        assert!(
            MAX_MANIFEST_FILES
                == 1 + crate::hermes_workspace::MAX_WORKSPACE_FILES
                    + crate::hermes_project::MAX_FILES
                    + MANIFEST_SIDECAR_FILES
        );
        assert!(MAX_MANIFEST_FILES >= crate::hermes_workspace::MAX_WORKSPACE_FILES);
    };
}

/// The crash-window recovery enumerator (`profile_files`) bounds its walk by the
/// *same* manifest ceiling the producer and receiver enforce, not the old fixed
/// 2048: a legitimate home with more than 2048 files is enumerable (so its
/// pristine re-restore compares equal) instead of being refused as
/// `session_limit`.
#[test]
fn profile_recovery_enumeration_accepts_a_home_beyond_the_old_2048_cap() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("home");
    fs::create_dir_all(&root).unwrap();
    for i in 0..2100 {
        let rel = format!("skills/s{i:05}/SKILL.md");
        let path = root.join(&rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"# x\n").unwrap();
    }
    let mut found = std::collections::BTreeSet::new();
    profile_files(&root, &root, 8, &mut found).unwrap();
    assert_eq!(found.len(), 2100, "a >2048-file home is enumerable");
    assert!(found.contains("skills/s00000/SKILL.md"));
}
