//! A real filesystem round trip using synthetic, isolated data. Never uses GUI profile paths.
use super::{
    bundle::{Result, Stream},
    capture, Direction, LocalTransport, Replica,
};
use serde::Serialize;
use std::{collections::BTreeMap, fs};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub verified: bool,
    pub transferred: usize,
    pub preserved_branches: usize,
    pub repeat_transfers: usize,
    pub recovered_objects: usize,
}
pub fn run() -> Result<Diagnostic> {
    static RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    use std::sync::atomic::Ordering;
    if RUNNING.swap(true, Ordering::AcqRel) {
        return Err("sync_busy".into());
    }
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            RUNNING.store(false, Ordering::Release);
        }
    }
    let _reset = Reset;
    let dir = tempfile::tempdir().map_err(|_| "store_unavailable")?;
    let source = dir.path().join("prepared");
    fs::create_dir(&source).map_err(|_| "store_unavailable")?;
    fs::write(
        source.join("conversation.jsonl"),
        "{\"role\":\"user\",\"content\":\"Synthetic test conversation\"}\n",
    )
    .map_err(|_| "store_unavailable")?;
    let stream = Stream {
        agent: "codex".into(),
        profile: "demo".into(),
        conversation: "synthetic".into(),
    };
    let cloud = LocalTransport::create(&dir.path().join("shared"))?;
    let a = Replica::open(&dir.path().join("device-a"), &cloud.space)?;
    let b = Replica::open(&dir.path().join("device-b"), &cloud.space)?;
    a.export(
        stream.clone(),
        capture(&source, &["conversation.jsonl".into()])?,
    )?;
    let sent = a.sync(&cloud, Direction::Upload)?;
    let received = b.sync(&cloud, Direction::Download)?;
    a.export(
        stream.clone(),
        BTreeMap::from([("conversation.jsonl".into(), "Device A branch\n".into())]),
    )?;
    b.export(
        stream.clone(),
        BTreeMap::from([("conversation.jsonl".into(), "Device B branch\n".into())]),
    )?;
    a.sync(&cloud, Direction::Both)?;
    b.sync(&cloud, Direction::Both)?;
    let conflict = a.sync(&cloud, Direction::Both)?;
    let repeated = a.sync(&cloud, Direction::Both)?;
    let journal = a.checkpoint()?;
    let branches = journal.streams.first().map(|s| s.ids.len()).unwrap_or(0);
    drop(a);
    let restarted = Replica::open(&dir.path().join("device-a"), &cloud.space)?;
    let recovered = restarted.checkpoint()?;
    // An explicit resolution references both observed heads; old objects remain immutable.
    let heads = recovered
        .streams
        .first()
        .ok_or("diagnostic_failed")?
        .ids
        .clone();
    restarted.resolve(
        stream,
        BTreeMap::from([(
            "conversation.jsonl".into(),
            "Explicitly reconciled sample\n".into(),
        )]),
        heads,
    )?;
    if sent.published != 1
        || received.received != 1
        || branches != 2
        || conflict.conflicts != 1
        || repeated.published + repeated.received != 0
        || recovered.objects.len() != 3
        || restarted.checkpoint()?.objects.len() != 4
    {
        return Err("diagnostic_failed".into());
    }
    Ok(Diagnostic {
        verified: true,
        transferred: sent.published + received.received,
        preserved_branches: branches,
        repeat_transfers: repeated.published + repeated.received,
        recovered_objects: recovered.objects.len(),
    })
}
