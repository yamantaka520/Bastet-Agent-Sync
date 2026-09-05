use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_FILE: usize = 32 * 1024 * 1024;
pub const MAX_CONTENT: usize = 32 * 1024 * 1024;
pub const MAX_WIRE: u64 = 64 * 1024 * 1024;
pub const MAX_OBJECTS: usize = 4096;
pub const MAX_STORE: usize = 1024 * 1024 * 1024;
pub type Result<T> = std::result::Result<T, String>;

pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn is_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

// Portable namespace, not an OS path. Reject Windows aliases on every platform.
pub fn portable_path(path: &str) -> bool {
    if path.len() > 240 || path.is_empty() {
        return false;
    }
    path.split('/').all(|part| {
        if part.is_empty() || part == "." || part == ".." || part.ends_with('.') || part.len() > 100
        {
            return false;
        }
        if !part
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        {
            return false;
        }
        let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str()) {
            return false;
        }
        if stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9')
        {
            return false;
        }
        true
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Stream {
    pub agent: String,
    pub profile: String,
    pub conversation: String,
}
impl Stream {
    pub fn validate(&self) -> Result<()> {
        if !crate::model::AGENTS.contains(&self.agent.as_str())
            || !token(&self.profile)
            || !token(&self.conversation)
        {
            return Err("invalid_bundle".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub sha256: String,
    pub content: String,
}
impl Entry {
    pub fn new(content: String) -> Self {
        Self {
            sha256: hash(content.as_bytes()),
            content,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema: u32,
    pub space: String,
    pub device: String,
    pub stream: Stream,
    pub parents: Vec<String>,
    pub files: BTreeMap<String, Entry>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub id: String,
    pub snapshot: Snapshot,
}
impl Bundle {
    pub fn new(snapshot: Snapshot) -> Result<Self> {
        let id = hash(&serde_json::to_vec(&snapshot).map_err(|_| "invalid_bundle")?);
        let b = Self { id, snapshot };
        b.validate()?;
        Ok(b)
    }
    pub fn validate(&self) -> Result<()> {
        let s = &self.snapshot;
        if s.schema != 1 || !token(&s.space) || !token(&s.device) {
            return Err("invalid_bundle".into());
        }
        s.stream.validate()?;
        if s.parents.len() > 16
            || s.parents.iter().any(|p| !is_hash(p) || p == &self.id)
            || s.parents.windows(2).any(|p| p[0] >= p[1])
        {
            return Err("invalid_bundle".into());
        }
        if s.files.is_empty() || s.files.len() > 256 {
            return Err("bundle_limit".into());
        }
        let mut total = 0usize;
        let mut paths = BTreeSet::new();
        for (path, file) in &s.files {
            if !portable_path(path) || !paths.insert(path.to_ascii_lowercase()) {
                return Err("invalid_path".into());
            }
            if file.content.len() > MAX_FILE {
                return Err("bundle_limit".into());
            }
            total = total
                .checked_add(file.content.len())
                .ok_or("bundle_limit")?;
            if total > MAX_CONTENT {
                return Err("bundle_limit".into());
            }
            if hash(file.content.as_bytes()) != file.sha256 {
                return Err("hash_mismatch".into());
            }
        }
        for path in &paths {
            let mut prefix = String::new();
            for part in path.split('/').take(path.split('/').count() - 1) {
                if !prefix.is_empty() {
                    prefix.push('/');
                }
                prefix.push_str(part);
                if paths.contains(&prefix) {
                    return Err("invalid_path".into());
                }
            }
        }
        if !is_hash(&self.id)
            || hash(&serde_json::to_vec(s).map_err(|_| "invalid_bundle")?) != self.id
        {
            return Err("hash_mismatch".into());
        }
        Ok(())
    }
    pub fn bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|_| "invalid_bundle")?;
        if bytes.len() as u64 > MAX_WIRE {
            return Err("bundle_limit".into());
        }
        Ok(bytes)
    }
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() as u64 > MAX_WIRE {
            return Err("bundle_limit".into());
        }
        let b: Self = serde_json::from_slice(bytes).map_err(|_| "invalid_bundle")?;
        b.validate()?;
        Ok(b)
    }
}
