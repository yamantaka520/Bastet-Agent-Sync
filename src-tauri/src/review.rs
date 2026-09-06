//! Read-only comparison. Incoming bytes never overwrite a local agent file.
use crate::sync::{
    bundle::{self, Result},
    storage,
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::{Component, Path},
};
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub path: String,
    pub state: String,
    pub local_bytes: Option<u64>,
    pub incoming_bytes: u64,
    pub local_hash: Option<String>,
    pub incoming_hash: String,
    pub local_text: Option<String>,
    pub incoming_text: Option<String>,
    pub truncated: bool,
}
pub fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() < 2048
        && !path.contains(['\\', ':', '\0'])
        && path
            .split('/')
            .all(|c| !c.is_empty() && c != "." && c != "..")
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}
pub fn local_file(root: &Path, relative: &str, limit: u64) -> Result<Option<Vec<u8>>> {
    if !safe_path(relative) {
        return Err("unsafe_store".into());
    }
    let mut current = root.to_path_buf();
    for component in std::iter::once(None).chain(Path::new(relative).components().map(Some)) {
        if let Some(c) = component {
            current.push(c);
        }
        match std::fs::symlink_metadata(&current) {
            Ok(m) if m.file_type().is_symlink() => return Err("unsafe_store".into()),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err("store_unavailable".into()),
        }
    }
    let before = std::fs::metadata(&current).map_err(|_| "store_unavailable")?;
    let bytes = storage::read(&current, limit)?;
    let after = std::fs::metadata(&current).map_err(|_| "source_changing")?;
    if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
        return Err("source_changing".into());
    }
    Ok(Some(bytes))
}
pub fn diff(path: String, local: Option<Vec<u8>>, incoming: &[u8]) -> FileDiff {
    fn text(bytes: &[u8]) -> Option<String> {
        std::str::from_utf8(bytes)
            .ok()
            .map(|s| s.chars().take(32768).collect())
    }
    let incoming_hash = bundle::hash(incoming);
    let local_hash = local.as_ref().map(|v| bundle::hash(v));
    FileDiff {
        path,
        state: if local_hash.is_none() {
            "missing"
        } else if local_hash.as_ref() == Some(&incoming_hash) {
            "same"
        } else {
            "different"
        }
        .into(),
        local_bytes: local.as_ref().map(|v| v.len() as u64),
        incoming_bytes: incoming.len() as u64,
        local_hash,
        incoming_hash,
        local_text: local.as_deref().and_then(text),
        incoming_text: text(incoming),
        truncated: incoming.len() > 32768 || local.as_ref().is_some_and(|v| v.len() > 32768),
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    pub files: Vec<FileDiff>,
    pub fingerprint: String,
    pub reviewed: bool,
}
pub fn comparison(root: &Path, key: &str, files: Vec<FileDiff>) -> Result<Comparison> {
    let hashes: Vec<_> = files
        .iter()
        .map(|f| (&f.path, &f.local_hash, &f.incoming_hash))
        .collect();
    let fingerprint = bundle::hash(&serde_json::to_vec(&hashes).map_err(|_| "session_invalid")?);
    let marks = marks(root)?;
    Ok(Comparison {
        reviewed: marks.get(key) == Some(&fingerprint),
        files,
        fingerprint,
    })
}
fn marks(root: &Path) -> Result<BTreeMap<String, String>> {
    let path = root.join("reviewed-versions.json");
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    serde_json::from_slice(&storage::read(&path, 1024 * 1024)?)
        .map_err(|_| "local_store_damaged".into())
}
pub fn mark(root: &Path, key: String, fingerprint: &str) -> Result<()> {
    let mut marks = marks(root)?;
    if marks.len() >= 4096 && !marks.contains_key(&key) {
        return Err("review_limit".into());
    }
    marks.insert(key, fingerprint.into());
    storage::replace(
        &root.join("reviewed-versions.json"),
        &serde_json::to_vec(&marks).map_err(|_| "local_store_damaged")?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acknowledgement_expires_when_local_content_changes_and_binary_is_hash_only() {
        let root = tempfile::tempdir().unwrap();
        let compare = |bytes| {
            comparison(
                root.path(),
                "codex:version",
                vec![diff("session".into(), Some(bytes), b"incoming")],
            )
            .unwrap()
        };
        let a = compare(b"local".to_vec());
        assert!(!a.reviewed);
        assert_eq!(a.files[0].state, "different");
        mark(root.path(), "codex:version".into(), &a.fingerprint).unwrap();
        assert!(compare(b"local".to_vec()).reviewed);
        assert!(!compare(b"changed".to_vec()).reviewed);
        assert!(diff("binary".into(), None, &[255]).incoming_text.is_none());
        assert_eq!(diff("same".into(), Some(vec![1]), &[1]).state, "same");
    }
    #[test]
    fn local_preview_rejects_traversal_and_enforces_size() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("file"), b"four").unwrap();
        for path in ["../file", "/file", "dir/../file", "C:/file", "dir\\file"] {
            assert!(local_file(root.path(), path, 9).is_err());
        }
        assert!(local_file(root.path(), "file", 3).is_err());
        assert!(local_file(root.path(), "missing", 9).unwrap().is_none());
    }
    #[cfg(unix)]
    #[test]
    fn preview_never_follows_symlinks() {
        let root = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(root.path(), root.path().join("link")).unwrap();
        assert!(local_file(root.path(), "link/file", 9).is_err());
    }
}
