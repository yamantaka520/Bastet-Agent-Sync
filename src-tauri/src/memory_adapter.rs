//! Transport adapter for official AgentMemoryOS JSONL exports, never a database reader.
//! This is a bounded envelope preflight; AMOS remains responsible for semantic import validation.
use crate::sync::bundle::{Bundle, MAX_FILE};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
const FILE: &str = "agent-memory-os.jsonl";
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    pub version: u64,
    pub records: usize,
    pub kinds: BTreeMap<String, usize>,
    pub contains_authority_changes: bool,
}
pub fn inspect(text: &str) -> Result<Inspection, String> {
    if text.len() > MAX_FILE {
        return Err("memory_bundle_limit".into());
    }
    let mut lines = text.lines();
    let header: Value = serde_json::from_str(lines.next().ok_or("memory_bundle_invalid")?)
        .map_err(|_| "memory_bundle_invalid")?;
    let version = header["version"].as_u64().ok_or("memory_bundle_invalid")?;
    if header["kind"] != "bundle" || !(1..=3).contains(&version) {
        return Err("memory_bundle_version".into());
    }
    let mut result = Inspection {
        version,
        records: 0,
        kinds: BTreeMap::new(),
        contains_authority_changes: false,
    };
    for line in lines {
        let record: Value = serde_json::from_str(line).map_err(|_| "memory_bundle_invalid")?;
        let kind = record["kind"].as_str().ok_or("memory_bundle_invalid")?;
        if !matches!(
            kind,
            "memory" | "link" | "profile" | "tombstone" | "team" | "project" | "org_tombstone"
        ) || version < 2 && kind == "tombstone"
            || version < 3 && matches!(kind, "team" | "project" | "org_tombstone")
        {
            return Err("memory_bundle_record".into());
        }
        result.records += 1;
        if result.records > 10000 {
            return Err("memory_bundle_limit".into());
        }
        *result.kinds.entry(kind.into()).or_default() += 1;
        result.contains_authority_changes |=
            matches!(kind, "tombstone" | "team" | "project" | "org_tombstone");
    }
    Ok(result)
}
pub fn capture_export(text: String) -> Result<BTreeMap<String, String>, String> {
    inspect(&text)?;
    Ok(BTreeMap::from([(FILE.into(), text)]))
}
pub fn restore_export(bundle: &Bundle) -> Result<String, String> {
    bundle.validate()?;
    if bundle.snapshot.stream.agent != "agent-memory-os" || bundle.snapshot.files.len() != 1 {
        return Err("memory_bundle_invalid".into());
    }
    let text = &bundle
        .snapshot
        .files
        .get(FILE)
        .ok_or("memory_bundle_invalid")?
        .content;
    inspect(text)?;
    Ok(text.clone())
}
#[tauri::command]
pub async fn inspect_memory_export() -> Result<Option<Inspection>, String> {
    let selected = rfd::AsyncFileDialog::new()
        .add_filter("Agent Memory OS JSONL", &["jsonl"])
        .pick_file()
        .await;
    let Some(file) = selected else {
        return Ok(None);
    };
    let path = file.path().to_path_buf();
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = zeroize::Zeroizing::new(crate::sync::storage::read(&path, MAX_FILE as u64)?);
        let text = std::str::from_utf8(&bytes).map_err(|_| "memory_bundle_invalid")?;
        inspect(text).map(Some)
    })
    .await
    .map_err(|_| "memory_bundle_invalid".to_string())?
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::{bundle::Stream, Direction, LocalTransport, Replica};
    #[test]
    fn official_export_survives_replica_transfer_and_encryption_without_field_rewrites() {
        let text = include_str!("../../tests/fixtures/amos-v3.jsonl");
        let dir = tempfile::tempdir().unwrap();
        let remote = LocalTransport::create(&dir.path().join("transport")).unwrap();
        let a = Replica::open(&dir.path().join("a"), &remote.space).unwrap();
        let b = Replica::open(&dir.path().join("b"), &remote.space).unwrap();
        a.export_from(
            Stream {
                agent: "agent-memory-os".into(),
                profile: "fixture".into(),
                conversation: "memory-store".into(),
            },
            capture_export(text.into()).unwrap(),
            None,
        )
        .unwrap();
        a.sync(&remote, Direction::Upload).unwrap();
        b.sync(&remote, Direction::Download).unwrap();
        let bundles = b.transport_bundles().unwrap();
        assert_eq!(bundles.len(), 1);
        let key = crate::cloud::crypto::SpaceKey::generate().unwrap();
        let decoded = key
            .open(
                &remote.space,
                &key.seal(bundles.values().next().unwrap()).unwrap(),
            )
            .unwrap();
        assert_eq!(restore_export(&decoded).unwrap(), text);
        assert!(inspect(text).unwrap().contains_authority_changes);
    }
    #[test]
    fn preflight_rejects_database_unknown_version_and_records() {
        for text in [
            "SQLite format 3",
            "{\"kind\":\"bundle\",\"version\":4}\n",
            "{\"kind\":\"bundle\",\"version\":1}\n{\"kind\":\"team\"}\n",
            "{\"kind\":\"bundle\",\"version\":3}\n{\"kind\":\"future\"}\n",
            "{\"kind\":\"bundle\",\"version\":3}\n\n",
        ] {
            assert!(inspect(text).is_err());
        }
        assert!(inspect(&"x".repeat(MAX_FILE + 1)).is_err());
        for version in 1..=3 {
            assert_eq!(
                inspect(&format!("{{\"kind\":\"bundle\",\"version\":{version}}}\n"))
                    .unwrap()
                    .records,
                0
            );
        }
    }
}
