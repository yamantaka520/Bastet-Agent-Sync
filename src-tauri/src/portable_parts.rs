//! Bounded multipart transport for a Hermes workspace that exceeds one
//! encrypted object's budget.
//!
//! The existing single-package path carries the whole workspace as one
//! `portable.json` inside one `Bundle`, so a home with more than
//! [`MAX_PART_FILES`] files, a file larger than [`MAX_CHUNK_BYTES`], or more than
//! [`MAX_PART_BYTES`] of payload cannot travel without dropping content. This
//! module keeps those *per-part* security budgets unchanged and instead splits
//! the workspace into several bounded parts plus one deterministic, complete
//! index. Every part is a plain [`Bundle`] and therefore rides the existing
//! encrypted, idempotent, retried, de-duplicated cloud queue unchanged — no new
//! transport, no new OAuth, no larger constant.
//!
//! Guarantees:
//! - The index is a *complete* manifest: every captured file appears with its
//!   total size and content hash, and its exact chunk layout. Content is never
//!   dropped silently; anything excluded is recorded with an explicit reason.
//! - Every chunk is bounded to [`MAX_CHUNK_BYTES`], every part to
//!   [`MAX_PART_FILES`] files, [`MAX_PART_CHUNKS`] chunks and [`MAX_PART_BYTES`]
//!   bytes — the per-part budgets are identical to the single-package budgets.
//! - A whole transfer is bounded by [`MAX_PARTS`], [`MAX_TOTAL_CHUNKS`],
//!   [`MAX_TOTAL_BYTES`] and [`MAX_FILES`] as resource-abuse protection, and an
//!   over-budget source fails closed rather than truncating.
//! - Reassembly requires *every* part the index names; a missing part, a
//!   tampered chunk, a chunk of the wrong length, a part bound to a different
//!   transfer, or an unknown extra part is refused. Listing reports a transfer
//!   as *complete* only when the whole set of parts is present and the index is
//!   structurally consistent — never for a partial transfer — and never claims
//!   byte-level integrity: that is verified by `reassemble` on compare/restore.
//! - Received content stays inert bytes; nothing here executes or writes a live
//!   home.
//!
//! Determinism: files are iterated in lexicographic path order and chunks are
//! packed greedily, so the same source yields byte-identical parts. The transfer
//! id is the SHA-256 of the serialized index, which binds every part to its
//! index.

use crate::sync::bundle::{self, Bundle, Result, MAX_OBJECTS};
#[cfg(test)]
use crate::sync::bundle::{Entry, Snapshot, Stream};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SCHEMA: u32 = 1;

/// Per-chunk payload ceiling. Identical to the single-package per-file budget so
/// the *part* budget is unchanged, never raised.
pub(crate) const MAX_CHUNK_BYTES: usize = 1024 * 1024;
/// Per-part payload ceiling (single-package total budget, preserved).
pub(crate) const MAX_PART_BYTES: usize = 16 * 1024 * 1024;
/// Per-part chunk ceiling (single-package file-count budget, preserved).
pub(crate) const MAX_PART_CHUNKS: usize = 256;
/// Per-part distinct-file ceiling: a part holds fragments of at most this many
/// files, so a receiver never assembles an unbounded file set from one object.
pub(crate) const MAX_PART_FILES: usize = 256;

/// Transfer-wide abuse ceilings. A legitimate home is far below these; a
/// pathological or hostile source fails closed.
pub(crate) const MAX_PARTS: usize = MAX_OBJECTS;
pub(crate) const MAX_TOTAL_CHUNKS: usize = MAX_OBJECTS;
pub(crate) const MAX_TOTAL_BYTES: usize = 512 * 1024 * 1024;
pub(crate) const MAX_FILES: usize = 65_536;

pub(crate) const INDEX_FILE: &str = "bastet/portable-index.json";
pub(crate) const PART_PREFIX: &str = "bastet/portable-part-";
/// Legacy single-package payload name (0.8 and earlier).
pub(crate) const LEGACY_FILE: &str = "portable.json";

/// One contiguous byte range of a file, addressed by part + in-part index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ChunkRef {
    pub part: u32,
    pub index: u32,
    pub bytes: u32,
    pub sha256: String,
}

/// A captured file's complete layout: total size, content hash, and every chunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct FileEntry {
    pub path: String,
    pub kind: String,
    pub bytes: u64,
    pub sha256: String,
    pub chunks: Vec<ChunkRef>,
}

/// A part's declared footprint, so the index alone reveals what is missing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PartRef {
    pub part: u32,
    pub files: u32,
    pub chunks: u32,
    pub bytes: u64,
}

/// The deterministic, complete index. `transfer` is the SHA-256 of this document
/// with `transfer` cleared, so it is a stable content id that also binds parts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PortableIndex {
    pub version: u32,
    pub agent: String,
    pub transfer: String,
    pub files: Vec<FileEntry>,
    pub excluded: BTreeMap<String, String>,
    pub parts: Vec<PartRef>,
}

/// A single stored chunk: its in-file index, its hash, and base64 content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StoredChunk {
    pub index: u32,
    pub sha256: String,
    pub content: String,
}

/// One bounded part object. `chunks` maps a file path to its fragments present
/// in this part, ordered by ascending chunk index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PortablePart {
    pub version: u32,
    pub agent: String,
    pub transfer: String,
    pub part: u32,
    pub chunks: BTreeMap<String, Vec<StoredChunk>>,
}

/// Readiness of a transfer given only the part numbers a receiver currently has.
///
/// `complete` is presence + index consistency only — *not* byte-level integrity.
/// It says every declared part is present and the index is internally
/// consistent; the bytes themselves are verified later by [`PortableIndex::reassemble`]
/// (on compare/restore), so a complete-but-tampered set still reports `complete`
/// here and is refused there. It never overclaims "verified".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Readiness {
    pub complete: bool,
    pub present: usize,
    pub expected: usize,
    pub missing: Vec<u32>,
}

fn canonical(index: &PortableIndex) -> Result<Vec<u8>> {
    let mut clear = index.clone();
    clear.transfer = String::new();
    serde_json::to_vec(&clear).map_err(|_| "portable_format".into())
}

/// The transfer id of an index: a stable content hash that parts bind to.
pub(crate) fn transfer_id(index: &PortableIndex) -> Result<String> {
    Ok(bundle::hash(&canonical(index)?))
}

fn agent_ok(agent: &str) -> bool {
    !agent.is_empty() && agent.len() <= 64 && !agent.contains(['/', '\\', '\0', '\n', '\r'])
}

/// A part being filled during [`PortableIndex::split`].
struct OpenPart {
    number: u32,
    files: BTreeSet<String>,
    chunks: BTreeMap<String, Vec<StoredChunk>>,
    bytes: usize,
    chunk_count: usize,
}

fn split_chunks(bytes: &[u8]) -> Vec<&[u8]> {
    bytes.chunks(MAX_CHUNK_BYTES).collect()
}

impl PortableIndex {
    /// Split a workspace into one deterministic index plus bounded parts.
    ///
    /// `files` are `(path, kind, raw bytes)`, iterated in lexicographic order.
    /// Exceeding a transfer-wide abuse ceiling fails closed (`portable_limit`);
    /// it never truncates.
    pub(crate) fn split(
        agent: &str,
        files: BTreeMap<String, (String, Vec<u8>)>,
        excluded: BTreeMap<String, String>,
    ) -> Result<(PortableIndex, BTreeMap<u32, PortablePart>)> {
        if !agent_ok(agent) {
            return Err("portable_unsafe".into());
        }
        if files.len() > MAX_FILES || excluded.len() > MAX_FILES {
            return Err("portable_limit".into());
        }

        let mut collected: BTreeMap<u32, PortablePart> = BTreeMap::new();
        let mut refs: Vec<PartRef> = Vec::new();
        let mut open: Option<OpenPart> = None;
        let mut entries: Vec<FileEntry> = Vec::new();
        let mut total_bytes = 0usize;
        let mut total_chunks = 0usize;

        for (path, (kind, bytes)) in &files {
            if !bundle::portable_path(path) || !agent_ok(kind) {
                return Err("portable_unsafe".into());
            }
            let ranges = split_chunks(bytes);
            total_bytes = total_bytes
                .checked_add(bytes.len())
                .ok_or("portable_limit")?;
            total_chunks = total_chunks
                .checked_add(ranges.len())
                .ok_or("portable_limit")?;
            if total_bytes > MAX_TOTAL_BYTES || total_chunks > MAX_TOTAL_CHUNKS {
                return Err("portable_limit".into());
            }
            let mut stored_refs = Vec::with_capacity(ranges.len());
            for (index, range) in ranges.iter().enumerate() {
                let need_new = match &open {
                    Some(part) => {
                        part.chunk_count + 1 > MAX_PART_CHUNKS
                            || part.bytes + range.len() > MAX_PART_BYTES
                            || (!part.files.contains(path.as_str())
                                && part.files.len() >= MAX_PART_FILES)
                    }
                    None => true,
                };
                if need_new {
                    if let Some(part) = open.take() {
                        finish_part(agent, &mut collected, &mut refs, part);
                    }
                    if collected.len() >= MAX_PARTS {
                        return Err("portable_limit".into());
                    }
                    open = Some(OpenPart {
                        number: collected.len() as u32,
                        files: BTreeSet::new(),
                        chunks: BTreeMap::new(),
                        bytes: 0,
                        chunk_count: 0,
                    });
                }
                let part = open.as_mut().expect("opened above");
                part.files.insert(path.clone());
                part.chunks
                    .entry(path.clone())
                    .or_default()
                    .push(StoredChunk {
                        index: index as u32,
                        sha256: bundle::hash(range),
                        content: STANDARD.encode(range),
                    });
                part.bytes += range.len();
                part.chunk_count += 1;
                stored_refs.push(ChunkRef {
                    part: part.number,
                    index: index as u32,
                    bytes: range.len() as u32,
                    sha256: bundle::hash(range),
                });
            }
            entries.push(FileEntry {
                path: path.clone(),
                kind: kind.clone(),
                bytes: bytes.len() as u64,
                sha256: bundle::hash(bytes),
                chunks: stored_refs,
            });
        }
        if let Some(part) = open.take() {
            finish_part(agent, &mut collected, &mut refs, part);
        }

        let mut index = PortableIndex {
            version: SCHEMA,
            agent: agent.into(),
            transfer: String::new(),
            files: entries,
            excluded,
            parts: refs,
        };
        index.transfer = transfer_id(&index)?;
        for part in collected.values_mut() {
            part.transfer = index.transfer.clone();
        }
        index.validate()?;
        for part in collected.values() {
            part.validate(&index)?;
        }
        Ok((index, collected))
    }

    /// Validate the index structurally and against the transfer-wide ceilings.
    pub(crate) fn validate(&self) -> Result<()> {
        if self.version != SCHEMA {
            return Err("portable_version_unsupported".into());
        }
        if !agent_ok(&self.agent)
            || !bundle::is_hash(&self.transfer)
            || self.transfer_id()? != self.transfer
        {
            return Err("portable_tampered".into());
        }
        if self.files.len() > MAX_FILES
            || self.parts.len() > MAX_PARTS
            || self.excluded.len() > MAX_FILES
        {
            return Err("portable_limit".into());
        }
        let mut part_numbers: BTreeSet<u32> = BTreeSet::new();
        let mut chunk_total = 0usize;
        let mut byte_total = 0u64;
        for part in &self.parts {
            if !part_numbers.insert(part.part)
                || part.files as usize > MAX_PART_FILES
                || part.chunks as usize > MAX_PART_CHUNKS
                || part.bytes > MAX_PART_BYTES as u64
            {
                return Err("portable_limit".into());
            }
            chunk_total = chunk_total
                .checked_add(part.chunks as usize)
                .ok_or("portable_limit")?;
            byte_total = byte_total.checked_add(part.bytes).ok_or("portable_limit")?;
        }
        if chunk_total > MAX_TOTAL_CHUNKS || byte_total > MAX_TOTAL_BYTES as u64 {
            return Err("portable_limit".into());
        }
        // The *file* entries declare their own totals, and those declarations
        // bound every allocation `reassemble` performs. Bound them here, with
        // checked arithmetic, before any of those allocations: a crafted index
        // that passes the per-part footprint check but declares a file (or a
        // file set) far past the transfer-wide ceiling is refused here, so
        // `reassemble` never sizes a buffer from an unvalidated declaration.
        let mut file_bytes_total = 0u64;
        let mut file_chunk_total = 0usize;
        for file in &self.files {
            file_bytes_total = file_bytes_total
                .checked_add(file.bytes)
                .ok_or("portable_limit")?;
            file_chunk_total = file_chunk_total
                .checked_add(file.chunks.len())
                .ok_or("portable_limit")?;
            if file_bytes_total > MAX_TOTAL_BYTES as u64 || file_chunk_total > MAX_TOTAL_CHUNKS {
                return Err("portable_limit".into());
            }
        }
        // Parts must be numbered 0..n-1 with no gaps, so "all parts present" is
        // exactly "0..n present".
        if part_numbers != (0..self.parts.len() as u32).collect() {
            return Err("portable_format".into());
        }
        let mut seen_paths: BTreeSet<String> = BTreeSet::new();
        for file in &self.files {
            if !bundle::portable_path(&file.path)
                || !agent_ok(&file.kind)
                || !seen_paths.insert(file.path.clone())
                || !bundle::is_hash(&file.sha256)
            {
                return Err("portable_unsafe".into());
            }
            let mut sum = 0u64;
            for (expect_index, chunk) in file.chunks.iter().enumerate() {
                if chunk.index != expect_index as u32
                    || chunk.bytes == 0
                    || chunk.bytes as usize > MAX_CHUNK_BYTES
                    || !bundle::is_hash(&chunk.sha256)
                    || !part_numbers.contains(&chunk.part)
                {
                    return Err("portable_format".into());
                }
                sum = sum
                    .checked_add(chunk.bytes as u64)
                    .ok_or("portable_limit")?;
            }
            if (file.bytes == 0) != file.chunks.is_empty() {
                return Err("portable_format".into());
            }
            if sum != file.bytes {
                return Err("portable_format".into());
            }
        }
        // The index must name exactly the chunks its parts declare: every chunk
        // reference counts toward its part's declared chunk total and byte
        // total. This is checked from the index alone (no part content decoded),
        // so "all parts present" plus this consistency is real evidence of
        // completeness: an index that its parts cannot back is refused rather
        // than reported ready.
        let mut ref_chunks: BTreeMap<u32, usize> = BTreeMap::new();
        let mut ref_bytes: BTreeMap<u32, u64> = BTreeMap::new();
        for file in &self.files {
            for chunk in &file.chunks {
                *ref_chunks.entry(chunk.part).or_default() += 1;
                *ref_bytes.entry(chunk.part).or_default() += chunk.bytes as u64;
            }
        }
        for part in &self.parts {
            if ref_chunks.get(&part.part).copied().unwrap_or(0) != part.chunks as usize
                || ref_bytes.get(&part.part).copied().unwrap_or(0) != part.bytes
            {
                return Err("portable_format".into());
            }
        }
        Ok(())
    }

    fn transfer_id(&self) -> Result<String> {
        transfer_id(self)
    }

    /// Serialize the index payload for its [`Bundle`].
    pub(crate) fn to_json(&self) -> Result<String> {
        self.validate()?;
        serde_json::to_string(self).map_err(|_| "portable_format".into())
    }

    /// Parse and validate a received index.
    pub(crate) fn parse(text: &str) -> Result<Self> {
        let index: Self = serde_json::from_str(text).map_err(|_| "portable_format")?;
        index.validate()?;
        Ok(index)
    }

    /// Wrap the index in a transferable [`Bundle`].
    #[cfg(test)]
    pub(crate) fn bundle(&self, space: &str, device: &str, stream: Stream) -> Result<Bundle> {
        let json = self.to_json()?;
        Bundle::new(Snapshot {
            schema: 1,
            space: space.into(),
            device: device.into(),
            stream,
            parents: vec![],
            files: BTreeMap::from([(INDEX_FILE.into(), Entry::new(json))]),
        })
    }

    /// Read the index out of a received bundle, if it is an index bundle.
    pub(crate) fn from_bundle(b: &Bundle) -> Result<Option<Self>> {
        let Some(entry) = b.snapshot.files.get(INDEX_FILE) else {
            return Ok(None);
        };
        Ok(Some(Self::parse(&entry.content)?))
    }

    /// Whether a bundle is the legacy single-package payload.
    pub(crate) fn is_legacy(b: &Bundle) -> bool {
        b.snapshot.files.contains_key(LEGACY_FILE)
    }

    /// Which part number a bundle carries, if it is a part bundle.
    pub(crate) fn part_number(b: &Bundle) -> Option<u32> {
        b.snapshot.files.keys().find_map(|key| {
            key.strip_prefix(PART_PREFIX)?
                .strip_suffix(".json")?
                .parse()
                .ok()
        })
    }

    /// Current readiness given the part numbers a receiver has assembled.
    pub(crate) fn readiness(&self, present: &BTreeSet<u32>) -> Readiness {
        let expected = self.parts.len();
        let missing: Vec<u32> = self
            .parts
            .iter()
            .map(|p| p.part)
            .filter(|n| !present.contains(n))
            .collect();
        let valid = self.validate().is_ok();
        Readiness {
            complete: valid && missing.is_empty(),
            present: self.parts.len() - missing.len(),
            expected,
            missing,
        }
    }

    /// Reassemble the complete workspace from an index plus its parts.
    ///
    /// Requires every part the index names; refuses a missing part, a part bound
    /// to another transfer, a tampered chunk, a chunk of the wrong length, an
    /// unknown extra part, or a whole-file hash mismatch. Order-independent and
    /// idempotent: a repeated identical part reassembles to the same result.
    pub(crate) fn reassemble(
        &self,
        parts: &BTreeMap<u32, PortablePart>,
    ) -> Result<BTreeMap<String, (String, Vec<u8>)>> {
        self.validate()?;
        for number in parts.keys() {
            if self.parts.iter().all(|p| p.part != *number) {
                return Err("portable_foreign_part".into());
            }
        }
        for part in &self.parts {
            let stored = parts.get(&part.part).ok_or("portable_incomplete")?;
            stored.validate(self)?;
        }
        let mut out: BTreeMap<String, (String, Vec<u8>)> = BTreeMap::new();
        for file in &self.files {
            // `validate` above already bounds the declared total to
            // `MAX_TOTAL_BYTES`, so this reserve is both bounded and fallible:
            // a failed allocation is a clean `portable_limit`, never a panic or
            // an unbounded `with_capacity` sized from the index's own claim.
            let declared = usize::try_from(file.bytes).map_err(|_| "portable_limit")?;
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(declared)
                .map_err(|_| "portable_limit")?;
            for chunk in &file.chunks {
                let part = parts.get(&chunk.part).ok_or("portable_incomplete")?;
                let stored = part
                    .chunks
                    .get(&file.path)
                    .and_then(|chunks| chunks.iter().find(|c| c.index == chunk.index))
                    .ok_or("portable_incomplete")?;
                if stored.sha256 != chunk.sha256 {
                    return Err("portable_tampered".into());
                }
                let decoded = STANDARD
                    .decode(&stored.content)
                    .map_err(|_| "portable_format")?;
                if decoded.len() as u32 != chunk.bytes || bundle::hash(&decoded) != chunk.sha256 {
                    return Err("portable_tampered".into());
                }
                bytes.extend_from_slice(&decoded);
            }
            if bytes.len() as u64 != file.bytes || bundle::hash(&bytes) != file.sha256 {
                return Err("portable_tampered".into());
            }
            out.insert(file.path.clone(), (file.kind.clone(), bytes));
        }
        Ok(out)
    }
}

impl PortablePart {
    /// Wrap a part in a transferable [`Bundle`].
    #[cfg(test)]
    pub(crate) fn bundle(&self, space: &str, device: &str, stream: Stream) -> Result<Bundle> {
        self.validate_shape()?;
        let json = serde_json::to_string(self).map_err(|_| "portable_format")?;
        Bundle::new(Snapshot {
            schema: 1,
            space: space.into(),
            device: device.into(),
            stream,
            parents: vec![],
            files: BTreeMap::from([(format!("{PART_PREFIX}{}.json", self.part), Entry::new(json))]),
        })
    }

    fn validate_shape(&self) -> Result<()> {
        if self.version != SCHEMA {
            return Err("portable_version_unsupported".into());
        }
        if !agent_ok(&self.agent) {
            return Err("portable_unsafe".into());
        }
        if self.chunks.len() > MAX_PART_FILES {
            return Err("portable_limit".into());
        }
        let mut chunks = 0usize;
        let mut bytes = 0usize;
        for (path, list) in &self.chunks {
            if !bundle::portable_path(path) {
                return Err("portable_unsafe".into());
            }
            // A file that spans several parts keeps its file-level chunk indices,
            // so a part's fragments are strictly ascending (not necessarily from
            // zero) and never repeat.
            let mut previous: Option<u32> = None;
            for stored in list {
                if !bundle::is_hash(&stored.sha256)
                    || previous.is_some_and(|prev| stored.index <= prev)
                {
                    return Err("portable_format".into());
                }
                previous = Some(stored.index);
                let decoded = STANDARD
                    .decode(&stored.content)
                    .map_err(|_| "portable_format")?;
                if decoded.is_empty()
                    || decoded.len() > MAX_CHUNK_BYTES
                    || bundle::hash(&decoded) != stored.sha256
                {
                    return Err("portable_tampered".into());
                }
                chunks += 1;
                bytes += decoded.len();
            }
        }
        if chunks > MAX_PART_CHUNKS || bytes > MAX_PART_BYTES {
            return Err("portable_limit".into());
        }
        Ok(())
    }

    /// Validate a part against its index: identity, binding, and footprint.
    pub(crate) fn validate(&self, index: &PortableIndex) -> Result<()> {
        self.validate_shape()?;
        if self.agent != index.agent || self.transfer != index.transfer {
            return Err("portable_tampered".into());
        }
        let expected = index
            .parts
            .iter()
            .find(|p| p.part == self.part)
            .ok_or("portable_foreign_part")?;
        let files = self.chunks.len() as u32;
        let chunks: u32 = self.chunks.values().map(|v| v.len() as u32).sum();
        let bytes: u64 = self
            .chunks
            .values()
            .flat_map(|v| v.iter())
            .map(|c| {
                STANDARD
                    .decode(&c.content)
                    .map(|d| d.len() as u64)
                    .unwrap_or(0)
            })
            .sum();
        if files != expected.files || chunks != expected.chunks || bytes != expected.bytes {
            return Err("portable_tampered".into());
        }
        Ok(())
    }
}

fn finish_part(
    agent: &str,
    collected: &mut BTreeMap<u32, PortablePart>,
    refs: &mut Vec<PartRef>,
    part: OpenPart,
) {
    refs.push(PartRef {
        part: part.number,
        files: part.files.len() as u32,
        chunks: part.chunk_count as u32,
        bytes: part.bytes as u64,
    });
    collected.insert(
        part.number,
        PortablePart {
            version: SCHEMA,
            agent: agent.into(),
            transfer: String::new(),
            part: part.number,
            chunks: part.chunks,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::crypto::SpaceKey;
    use crate::cloud::queue::Objects;
    use std::cell::{Cell, RefCell};

    fn stream() -> Stream {
        Stream {
            agent: "hermes".into(),
            profile: "node".into(),
            conversation: "portable-preferences".into(),
        }
    }

    fn assets(n: usize, bytes_each: usize) -> BTreeMap<String, (String, Vec<u8>)> {
        (0..n)
            .map(|i| {
                (
                    format!("skills/skill{i:05}/SKILL.md"),
                    ("skill".to_string(), vec![b'x'; bytes_each]),
                )
            })
            .collect()
    }

    fn split(
        files: BTreeMap<String, (String, Vec<u8>)>,
    ) -> Result<(PortableIndex, BTreeMap<u32, PortablePart>)> {
        PortableIndex::split("hermes", files, BTreeMap::new())
    }

    fn every_part_within_budget(parts: &BTreeMap<u32, PortablePart>) {
        for part in parts.values() {
            assert!(part.chunks.len() <= MAX_PART_FILES, "part files");
            let chunks: usize = part.chunks.values().map(Vec::len).sum();
            assert!(chunks <= MAX_PART_CHUNKS, "part chunks");
            let bytes: usize = part
                .chunks
                .values()
                .flatten()
                .map(|c| STANDARD.decode(&c.content).unwrap().len())
                .sum();
            assert!(bytes <= MAX_PART_BYTES, "part bytes");
        }
    }

    #[test]
    fn over_256_legitimate_assets_round_trip_completely() {
        let source = assets(700, 40);
        let (index, parts) = split(source.clone()).unwrap();
        assert_eq!(index.files.len(), 700, "every file is in the manifest");
        assert!(
            parts.len() > 2,
            "expected several parts, got {}",
            parts.len()
        );
        every_part_within_budget(&parts);

        let out = index.reassemble(&parts).unwrap();
        assert_eq!(out.len(), 700);
        for (path, (kind, bytes)) in &source {
            assert_eq!(out[path].0, *kind);
            assert_eq!(&out[path].1, bytes, "{path}");
        }
        let present: BTreeSet<u32> = parts.keys().copied().collect();
        let readiness = index.readiness(&present);
        assert!(readiness.complete);
        assert_eq!(
            (readiness.present, readiness.expected),
            (parts.len(), parts.len())
        );
    }

    #[test]
    fn a_file_larger_than_the_part_byte_budget_is_streamed_across_parts_not_dropped() {
        // 17 MiB: one chunk over the 16 MiB per-part byte budget, so it must
        // span more than one part.
        let big = (MAX_PART_BYTES / MAX_CHUNK_BYTES + 1) * MAX_CHUNK_BYTES + 7;
        let mut source = assets(3, 10);
        source.insert(
            "skills/big/data.bin".into(),
            ("skill".into(), patterned(big)),
        );
        let (index, parts) = split(source.clone()).unwrap();
        every_part_within_budget(&parts);

        let entry = index
            .files
            .iter()
            .find(|f| f.path == "skills/big/data.bin")
            .unwrap();
        assert_eq!(entry.bytes as usize, big);
        assert!(entry.chunks.len() > 1);
        let touched: BTreeSet<u32> = entry.chunks.iter().map(|c| c.part).collect();
        assert!(touched.len() > 1, "the large file spans several parts");

        let out = index.reassemble(&parts).unwrap();
        assert_eq!(out["skills/big/data.bin"].1, patterned(big));
        assert_eq!(out.len(), source.len());
    }

    fn patterned(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i % 251) as u8).collect()
    }

    #[test]
    fn missing_or_tampered_parts_are_never_ready_or_imported() {
        let (index, parts) = split(assets(300, 12)).unwrap();
        assert!(parts.len() > 1);

        // Missing one part: not complete, and reassembly refuses.
        let mut missing: BTreeSet<u32> = parts.keys().copied().collect();
        let dropped = *missing.iter().next_back().unwrap();
        missing.remove(&dropped);
        let readiness = index.readiness(&missing);
        assert!(!readiness.complete);
        assert_eq!(readiness.missing, vec![dropped]);
        let mut partial = parts.clone();
        partial.remove(&dropped);
        assert_eq!(
            index.reassemble(&partial).unwrap_err(),
            "portable_incomplete"
        );

        // Tampered chunk content: hash mismatch is refused.
        let first = parts.keys().next().copied().unwrap();
        let mut tampered = parts.clone();
        let part = tampered.get_mut(&first).unwrap();
        let path = part.chunks.keys().next().unwrap().clone();
        let stored = &mut part.chunks.get_mut(&path).unwrap()[0];
        let original = STANDARD.decode(&stored.content).unwrap();
        let mut flipped = original.clone();
        flipped[0] ^= 0xff;
        stored.content = STANDARD.encode(&flipped);
        assert_eq!(
            index.reassemble(&tampered).unwrap_err(),
            "portable_tampered"
        );

        // Whole-file hash mismatch (a chunk replaced with same-length content).
        let mut swapped = parts.clone();
        let part = swapped.get_mut(&first).unwrap();
        let stored = &mut part.chunks.get_mut(&path).unwrap()[0];
        let mut other = original.clone();
        for byte in other.iter_mut() {
            *byte = byte.wrapping_add(1);
        }
        stored.sha256 = bundle::hash(&other);
        stored.content = STANDARD.encode(&other);
        assert!(index.reassemble(&swapped).is_err());
    }

    #[test]
    fn out_of_order_and_repeated_chunks_reassemble_identically() {
        let source = assets(600, 20);
        let (index, parts) = split(source.clone()).unwrap();
        let expected = index.reassemble(&parts).unwrap();

        // Reverse insertion order: a BTreeMap is order-independent by key, but
        // build it that way to prove nothing depends on arrival order.
        let mut shuffled: BTreeMap<u32, PortablePart> = BTreeMap::new();
        for (number, part) in parts.iter().rev() {
            shuffled.insert(*number, part.clone());
        }
        assert_eq!(index.reassemble(&shuffled).unwrap(), expected);

        // A repeated identical part is idempotent; a repeated *conflicting* part
        // is a tamper and is refused.
        let mut repeated = parts.clone();
        let first = parts.keys().next().copied().unwrap();
        repeated.insert(first, parts[&first].clone());
        assert_eq!(index.reassemble(&repeated).unwrap(), expected);

        let mut conflict = parts.clone();
        let mut bad = parts[&first].clone();
        bad.transfer = bundle::hash(b"other-transfer");
        conflict.insert(first, bad);
        assert_eq!(
            index.reassemble(&conflict).unwrap_err(),
            "portable_tampered"
        );

        // A part number the index never named is foreign.
        let mut foreign = parts.clone();
        foreign.insert(999, parts[&first].clone());
        assert_eq!(
            index.reassemble(&foreign).unwrap_err(),
            "portable_foreign_part"
        );
    }

    #[test]
    fn legacy_single_package_bundle_is_recognised_and_multipart_bundles_are_distinct() {
        // A 0.8 single-package bundle: one `portable.json` entry.
        let legacy = Bundle::new(Snapshot {
            schema: 1,
            space: "space".into(),
            device: "a".into(),
            stream: stream(),
            parents: vec![],
            files: BTreeMap::from([(LEGACY_FILE.into(), Entry::new("{}".into()))]),
        })
        .unwrap();
        assert!(PortableIndex::is_legacy(&legacy));
        assert!(PortableIndex::from_bundle(&legacy).unwrap().is_none());
        assert_eq!(PortableIndex::part_number(&legacy), None);

        let (index, parts) = split(assets(5, 8)).unwrap();
        let index_bundle = index.bundle("space", "a", stream()).unwrap();
        assert!(!PortableIndex::is_legacy(&index_bundle));
        assert_eq!(
            PortableIndex::from_bundle(&index_bundle).unwrap(),
            Some(index.clone())
        );
        for (number, part) in &parts {
            let bundle = part.bundle("space", "a", stream()).unwrap();
            assert_eq!(PortableIndex::part_number(&bundle), Some(*number));
            assert!(PortableIndex::from_bundle(&bundle).unwrap().is_none());
        }
    }

    #[test]
    fn unsupported_version_and_mismatched_binding_fail_closed() {
        let (mut index, parts) = split(assets(4, 8)).unwrap();

        let mut future = index.clone();
        future.version = 2;
        assert_eq!(
            future.validate().unwrap_err(),
            "portable_version_unsupported"
        );

        // An index whose transfer id does not match its content is tampering.
        index.transfer = bundle::hash(b"wrong");
        assert_eq!(index.validate().unwrap_err(), "portable_tampered");
        index.transfer = transfer_id(&index).unwrap();
        index.validate().unwrap();

        // A part bound to a different transfer is refused.
        let first = parts.keys().next().copied().unwrap();
        let mut foreign = parts[&first].clone();
        foreign.agent = "codex".into();
        assert_eq!(foreign.validate(&index).unwrap_err(), "portable_tampered");
    }

    #[test]
    fn transfer_wide_abuse_ceilings_fail_closed_rather_than_truncate() {
        // One more chunk than the whole transfer may carry.
        let too_many = assets(MAX_TOTAL_CHUNKS + 1, 1);
        assert_eq!(split(too_many).unwrap_err(), "portable_limit");

        // A source path that is not a portable logical path is refused.
        let mut unsafe_paths = BTreeMap::new();
        unsafe_paths.insert("../escape".to_string(), ("skill".to_string(), vec![1u8]));
        assert_eq!(split(unsafe_paths).unwrap_err(), "portable_unsafe");
    }

    #[test]
    fn excluded_entries_and_file_kinds_travel_in_the_index() {
        let mut excluded = BTreeMap::new();
        excluded.insert(
            "skills/x/secret.txt".to_string(),
            "suspected_credential".to_string(),
        );
        excluded.insert("skills/x/link".to_string(), "symlink".to_string());
        let mut source = assets(3, 16);
        source.insert(
            "config.yaml".into(),
            ("config".into(), b"model: {}\n".to_vec()),
        );
        let (index, parts) =
            PortableIndex::split("hermes", source.clone(), excluded.clone()).unwrap();
        assert_eq!(index.excluded, excluded);
        assert!(index
            .files
            .iter()
            .any(|f| f.path == "config.yaml" && f.kind == "config"));
        // Round-tripped kinds survive.
        let out = index.reassemble(&parts).unwrap();
        assert_eq!(out["config.yaml"].0, "config");
    }

    // ---- transport: the protocol over the existing `Objects` relay boundary ----

    #[derive(Clone, Copy)]
    enum Order {
        Sorted,
        Reversed,
        Duplicated,
    }

    /// An encrypted, id-addressed object store modelling the three cloud relays
    /// (Google Drive API, iCloud Drive folder, OneDrive folder) purely through
    /// the shared `Objects` boundary. The only differences are listing order and
    /// duplicate listing, which no correct transport may depend on.
    struct Relay {
        objects: RefCell<BTreeMap<String, Vec<u8>>>,
        next: Cell<usize>,
        order: Order,
        fail_once: Cell<bool>,
    }
    impl Relay {
        fn new(order: Order) -> Self {
            Self {
                objects: RefCell::new(BTreeMap::new()),
                next: Cell::new(0),
                order,
                fail_once: Cell::new(false),
            }
        }
    }
    impl Objects for Relay {
        fn ids(&self, _: &str) -> Result<Vec<String>> {
            let mut ids: Vec<String> = self.objects.borrow().keys().cloned().collect();
            match self.order {
                Order::Sorted => {}
                Order::Reversed => ids.reverse(),
                Order::Duplicated => {
                    let copy = ids.clone();
                    ids.extend(copy);
                }
            }
            Ok(ids)
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
                // The object is stored but the caller sees a transport error —
                // the ambiguous-upload case the queue journal exists for.
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

    fn send(
        relay: &Relay,
        key: &SpaceKey,
        bundles: &[Bundle],
        journal: &mut BTreeMap<String, String>,
    ) -> Result<()> {
        for bundle in bundles {
            let id = match journal.get(&bundle.id) {
                Some(id) => id.clone(),
                None => {
                    let id = relay.allocate()?;
                    journal.insert(bundle.id.clone(), id.clone());
                    id
                }
            };
            match relay.put("folder", &id, key, bundle) {
                Ok(()) => {}
                Err(e) if e == "drive_id_exists" => {
                    if relay.get("folder", &id, "space", key)? != *bundle {
                        return Err("immutable_collision".into());
                    }
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    fn receive(
        relay: &Relay,
        key: &SpaceKey,
    ) -> Result<(PortableIndex, BTreeMap<u32, PortablePart>)> {
        let mut index = None;
        let mut parts = BTreeMap::new();
        for id in relay.ids("folder")? {
            let bundle = relay.get("folder", &id, "space", key)?;
            if let Some(found) = PortableIndex::from_bundle(&bundle)? {
                index = Some(found);
            } else if let Some(number) = PortableIndex::part_number(&bundle) {
                let entry = bundle.snapshot.files.values().next().unwrap();
                let part: PortablePart =
                    serde_json::from_str(&entry.content).map_err(|_| "portable_format")?;
                parts.insert(number, part);
            }
        }
        index
            .map(|index| (index, parts))
            .ok_or("portable_missing".into())
    }

    fn all_bundles(index: &PortableIndex, parts: &BTreeMap<u32, PortablePart>) -> Vec<Bundle> {
        let mut bundles = vec![index.bundle("space", "device", stream()).unwrap()];
        bundles.extend(
            parts
                .values()
                .map(|part| part.bundle("space", "device", stream()).unwrap()),
        );
        bundles
    }

    #[test]
    fn three_relay_fixtures_deliver_and_reassemble_every_part_encrypted() {
        let source = assets(500, 30);
        let (index, parts) = split(source.clone()).unwrap();
        let bundles = all_bundles(&index, &parts);
        for order in [Order::Sorted, Order::Reversed, Order::Duplicated] {
            let relay = Relay::new(order);
            let key = SpaceKey::generate().unwrap();
            let mut journal = BTreeMap::new();
            send(&relay, &key, &bundles, &mut journal).unwrap();
            // Nothing crosses the wire in clear.
            assert!(relay
                .objects
                .borrow()
                .values()
                .all(|bytes| !String::from_utf8_lossy(bytes).contains("skills/")));

            let (received_index, received_parts) = receive(&relay, &key).unwrap();
            assert_eq!(received_index, index);
            let out = received_index.reassemble(&received_parts).unwrap();
            assert_eq!(out.len(), source.len());
            for (path, (_, bytes)) in &source {
                assert_eq!(&out[path].1, bytes);
            }
        }
    }

    #[test]
    fn interrupted_send_retries_without_duplicate_or_corrupt_content() {
        let source = assets(400, 24);
        let (index, parts) = split(source.clone()).unwrap();
        let bundles = all_bundles(&index, &parts);
        let relay = Relay::new(Order::Sorted);
        let key = SpaceKey::generate().unwrap();
        let mut journal = BTreeMap::new();

        relay.fail_once.set(true);
        assert_eq!(
            send(&relay, &key, &bundles, &mut journal).unwrap_err(),
            "network_unavailable"
        );
        let after_failure = relay.objects.borrow().len();

        // Retry with the same journal: the ambiguous object is reused, never
        // duplicated, and its bytes are unchanged.
        send(&relay, &key, &bundles, &mut journal).unwrap();
        assert_eq!(relay.objects.borrow().len(), bundles.len());
        assert!(after_failure <= relay.objects.borrow().len());

        let (received_index, received_parts) = receive(&relay, &key).unwrap();
        let out = received_index.reassemble(&received_parts).unwrap();
        assert_eq!(out.len(), source.len());
    }

    /// BLOCKING BUG (currently FAILS) — unbounded allocation from a crafted
    /// index. `reassemble` sizes `Vec::with_capacity(file.bytes)` from the
    /// index's *own* declaration, but `validate` never bounds the sum of a
    /// file's chunk bytes against the transfer-wide ceiling: only the per-part
    /// `PartRef` footprint is checked. A single part whose declared footprint is
    /// within `MAX_PART_BYTES` can back an arbitrarily many-chunk file entry
    /// (each chunk up to `MAX_CHUNK_BYTES`) summing far past `MAX_TOTAL_BYTES`:
    /// this ~50 KB index declares a 516 MiB file, and a full 32 MiB index
    /// (the bundle entry ceiling) declares hundreds of GiB, which `reassemble`
    /// then allocates before it ever validates a payload byte.
    /// Required invariant: `validate` refuses an over-ceiling file byte total.
    #[test]
    fn index_validate_bounds_total_file_bytes_before_reassembly() {
        let count = (MAX_TOTAL_BYTES / MAX_CHUNK_BYTES) as u32 + 4;
        let mut index = PortableIndex {
            version: SCHEMA,
            agent: "hermes".into(),
            transfer: String::new(),
            files: vec![FileEntry {
                path: "skills/x/big.bin".into(),
                kind: "skill".into(),
                bytes: count as u64 * MAX_CHUNK_BYTES as u64,
                sha256: bundle::hash(b"big"),
                chunks: (0..count)
                    .map(|i| ChunkRef {
                        part: 0,
                        index: i,
                        bytes: MAX_CHUNK_BYTES as u32,
                        sha256: bundle::hash(&[0u8]),
                    })
                    .collect(),
            }],
            excluded: BTreeMap::new(),
            // A perfectly legal single part footprint…
            parts: vec![PartRef {
                part: 0,
                files: 1,
                chunks: 1,
                bytes: MAX_CHUNK_BYTES as u64,
            }],
        };
        index.transfer = transfer_id(&index).unwrap();
        // …yet the file declares far more bytes than any part may carry.
        assert_eq!(index.validate().unwrap_err(), "portable_limit");
    }
}
