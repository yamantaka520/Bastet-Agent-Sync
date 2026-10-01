//! Explicit, receiving-device project path mapping for native session snapshots.
//!
//! Only provider-owned location metadata is changed. Conversation messages,
//! tool inputs/results, summaries, and arbitrary strings are historical evidence.
use crate::{native_sessions::Manifest, sync::bundle::Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Mapping {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Absolute {
    root: String,
    components: Vec<String>,
    windows: bool,
}

struct Prepared {
    source: Absolute,
    target: String,
    target_absolute: Absolute,
}

fn absolute(input: &str) -> Result<Absolute> {
    if input.is_empty() || input.chars().any(char::is_control) {
        return Err("project_mapping_invalid".into());
    }
    let replaced = input.replace('\\', "/");
    let path = if let Some(rest) = replaced.strip_prefix("//?/") {
        if rest
            .get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("UNC/"))
        {
            format!("//{}", &rest[4..])
        } else {
            rest.to_owned()
        }
    } else {
        replaced
    };
    let (root, rest, windows) = if let Some(unc) = path.strip_prefix("//") {
        let mut parts = unc.splitn(3, '/');
        let server = parts.next().unwrap_or("");
        let share = parts.next().unwrap_or("");
        if server.is_empty()
            || share.is_empty()
            || matches!(server, "." | "..")
            || matches!(share, "." | "..")
        {
            return Err("project_mapping_invalid".into());
        }
        (
            format!("//{server}/{share}"),
            parts.next().unwrap_or(""),
            true,
        )
    } else if let Some(rest) = path.strip_prefix('/') {
        ("/".to_string(), rest, false)
    } else if path.len() >= 3
        && path.as_bytes()[0].is_ascii_alphabetic()
        && path.as_bytes()[1] == b':'
        && path.as_bytes()[2] == b'/'
    {
        (path[..2].to_ascii_uppercase(), &path[3..], true)
    } else {
        return Err("project_mapping_invalid".into());
    };
    let components = rest
        .split('/')
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if components
        .iter()
        .any(|part| part == "." || part == ".." || (windows && part.contains(':')))
    {
        return Err("project_mapping_invalid".into());
    }
    Ok(Absolute {
        root,
        components,
        windows,
    })
}

fn canonical_target(path: &Path) -> Result<String> {
    let canonical = fs::canonicalize(path).map_err(|_| "project_mapping_target_missing")?;
    let raw = canonical.to_str().ok_or("project_mapping_invalid")?;
    let normalized = absolute(raw)?;
    let mut result = if normalized.windows {
        normalized.root.replace('/', "\\")
    } else {
        normalized.root.clone()
    };
    let separator = if normalized.windows { '\\' } else { '/' };
    if normalized.windows && result.ends_with(':') {
        result.push(separator);
    }
    for part in normalized.components {
        if !result.ends_with(separator) {
            result.push(separator);
        }
        result.push_str(&part);
    }
    Ok(result)
}

fn starts_with(path: &Absolute, prefix: &Absolute) -> bool {
    if path.windows != prefix.windows || prefix.components.len() > path.components.len() {
        return false;
    }
    let same_root = if path.windows {
        path.root.eq_ignore_ascii_case(&prefix.root)
    } else {
        path.root == prefix.root
    };
    if !same_root {
        return false;
    }
    path.components
        .iter()
        .zip(&prefix.components)
        .all(|(a, b)| {
            if path.windows {
                a.eq_ignore_ascii_case(b)
            } else {
                a == b
            }
        })
}

fn same_path(a: &Absolute, b: &Absolute) -> bool {
    a.components.len() == b.components.len() && starts_with(a, b)
}

fn prepare(mappings: &[Mapping]) -> Result<Vec<Prepared>> {
    if mappings.len() > 64 {
        return Err("project_mapping_invalid".into());
    }
    let mut parsed = Vec::with_capacity(mappings.len());
    for mapping in mappings {
        let source = absolute(&mapping.source)?;
        let target_path = Path::new(&mapping.target);
        if !target_path.is_absolute() || !target_path.is_dir() {
            return Err("project_mapping_target_missing".into());
        }
        let target = canonical_target(target_path)?;
        let target_absolute = absolute(&target)?;
        if parsed.iter().any(|old: &Prepared| {
            same_path(&source, &old.source) && !same_path(&target_absolute, &old.target_absolute)
        }) {
            return Err("project_mapping_conflict".into());
        }
        parsed.push(Prepared {
            source,
            target,
            target_absolute,
        });
    }
    Ok(parsed)
}

/// Validate all mappings before a snapshot is touched. Targets must be local,
/// absolute, existing directories. A source cannot point to two destinations.
pub fn validate_mappings(mappings: &[Mapping]) -> Result<()> {
    prepare(mappings).map(|_| ())
}

/// Returns None for an unmapped path. Matching uses complete components, with
/// case-insensitive comparison only for Windows source paths.
pub fn remap_path(path: &str, mappings: &[Mapping]) -> Result<Option<String>> {
    remap_prepared(path, &prepare(mappings)?)
}

fn remap_prepared(path: &str, mappings: &[Prepared]) -> Result<Option<String>> {
    let parsed = absolute(path)?;
    let mut best: Option<(usize, &Prepared)> = None;
    for mapping in mappings {
        if starts_with(&parsed, &mapping.source)
            && best.is_none_or(|(n, _)| mapping.source.components.len() > n)
        {
            best = Some((mapping.source.components.len(), mapping));
        }
    }
    let Some((prefix_len, mapping)) = best else {
        return Ok(None);
    };
    let mut mapped = std::path::PathBuf::from(&mapping.target);
    for part in &parsed.components[prefix_len..] {
        mapped.push(part);
    }
    Ok(Some(
        mapped.to_str().ok_or("project_mapping_invalid")?.to_owned(),
    ))
}

fn mapped_field(object: &mut Value, key: &str, mappings: &[Prepared]) -> Result<()> {
    if let Some(value) = object.get(key).and_then(Value::as_str).map(str::to_owned) {
        if let Some(mapped) = remap_prepared(&value, mappings)? {
            object[key] = Value::String(mapped);
        }
    }
    Ok(())
}

fn rewrite_jsonl(text: &str, agent: &str, mappings: &[Prepared]) -> Result<(String, bool)> {
    let mut output = String::with_capacity(text.len());
    let mut changed = false;
    for segment in text.split_inclusive('\n') {
        let (line, ending) = segment
            .strip_suffix('\n')
            .map_or((segment, ""), |s| (s, "\n"));
        if line.trim().is_empty() {
            output.push_str(segment);
            continue;
        }
        let mut value: Value = serde_json::from_str(line).map_err(|_| "session_invalid")?;
        let before = value.clone();
        match agent {
            "codex" | "chatgpt-work"
                if value["type"] == "session_meta" || value["type"] == "turn_context" =>
            {
                if let Some(payload) = value.get_mut("payload") {
                    mapped_field(payload, "cwd", mappings)?;
                }
            }
            "claude" | "claude-code" => mapped_field(&mut value, "cwd", mappings)?,
            "pi" if value["type"] == "session" => mapped_field(&mut value, "cwd", mappings)?,
            _ => {}
        }
        if value == before {
            output.push_str(segment);
        } else {
            changed = true;
            output.push_str(&serde_json::to_string(&value).map_err(|_| "session_invalid")?);
            output.push_str(ending);
        }
    }
    Ok((output, changed))
}

// Installed Pi SessionManager getDefaultSessionDirPath (2026-10-01):
// `--${resolvedCwd.replace(/^[/\\]/, "").replace(/[/\\:]/g, "-")}--`.
fn pi_group(cwd: &str) -> String {
    let normalized = cwd.replace('\\', "/");
    let without_initial = normalized.strip_prefix('/').unwrap_or(&normalized);
    format!("--{}--", without_initial.replace(['/', ':'], "-"))
}

// Installed Claude Code 2.1.259 bundle: projects/Cm(cwd), where Cm uses
// sC(cwd). sC replaces non-ASCII-alphanumeric UTF-16 units with '-', then
// keeps 200 characters plus a base36 abs(JS 32-bit string hash) for long names.
fn claude_group(cwd: &str) -> Result<String> {
    let encoded = cwd
        .encode_utf16()
        .map(|unit| {
            if unit <= 127 && (unit as u8).is_ascii_alphanumeric() {
                char::from(unit as u8)
            } else {
                '-'
            }
        })
        .collect::<String>();
    if encoded.len() <= 200 {
        return Ok(encoded);
    }
    let hash = cwd.encode_utf16().fold(0_i32, |current, unit| {
        current.wrapping_mul(31).wrapping_add(i32::from(unit))
    });
    let mut magnitude = i64::from(hash).unsigned_abs();
    let mut digits = Vec::new();
    loop {
        let digit = (magnitude % 36) as u8;
        digits.push(if digit < 10 {
            b'0' + digit
        } else {
            b'a' + digit - 10
        });
        magnitude /= 36;
        if magnitude == 0 {
            break;
        }
    }
    digits.reverse();
    let suffix = String::from_utf8(digits).map_err(|_| "project_mapping_invalid")?;
    Ok(format!("{}-{suffix}", &encoded[..200]))
}

// xAI grok-build xai-grok-config/src/paths.rs encode_cwd_dirname: urlencoding
// for <=255 encoded bytes; otherwise lowercase slug of the final component
// (40 chars) plus the first 16 BLAKE3 hex digits. Long groups also need .cwd.
fn grok_percent(cwd: &str) -> String {
    let mut encoded = String::new();
    for b in cwd.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(*b));
        } else {
            encoded.push_str(&format!("%{b:02X}"));
        }
    }
    encoded
}

fn grok_long(cwd: &str) -> bool {
    grok_percent(cwd).len() > 255
}

pub(crate) fn grok_long_cwd(cwd: &str) -> bool {
    grok_long(cwd)
}

fn grok_group(cwd: &str) -> String {
    let percent = grok_percent(cwd);
    if percent.len() <= 255 {
        return percent;
    }
    let normalized = cwd.replace('\\', "/");
    let leaf = normalized
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("");
    let leaf = if leaf.is_empty() { "workspace" } else { leaf };
    let mut slug = String::new();
    let mut previous_dash = false;
    for character in leaf.to_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }
    let trimmed = slug.trim_matches('-').chars().take(40).collect::<String>();
    let slug = if trimmed.is_empty() {
        "workspace"
    } else {
        &trimmed
    };
    let hash = blake3::hash(cwd.as_bytes()).to_hex();
    format!("{slug}-{}", &hash[..16])
}

pub(crate) fn grok_group_for_cwd(cwd: &str) -> String {
    grok_group(cwd)
}

/// Check provider lookup folders even when no mapping applies. An immutable
/// received bundle must not be reported ready if the installed provider would
/// search a different project group for its recorded working directory.
pub(crate) fn validate_provider_group(manifest: &Manifest) -> Result<()> {
    if manifest.cwd.is_empty() || manifest.files.is_empty() {
        return Err("project_mapping_format_unsupported".into());
    }
    let prefix = match manifest.agent.as_str() {
        "codex" | "chatgpt-work" => {
            // Codex groups by date, but its session_meta must still identify
            // the same conversation and cwd as the immutable manifest.
            return validate_primary_record(manifest);
        }
        "claude" | "claude-code" => format!("projects/{}/", claude_group(&manifest.cwd)?),
        "pi" => format!("sessions/{}/", pi_group(&manifest.cwd)),
        "grok" => format!("sessions/{}/", grok_group(&manifest.cwd)),
        _ => return Err("project_mapping_format_unsupported".into()),
    };
    if manifest.agent == "grok" {
        let group_prefix = format!("{prefix}{}/", manifest.session);
        let marker = format!("{prefix}.cwd");
        if manifest
            .files
            .keys()
            .any(|key| key != &marker && !key.starts_with(&group_prefix))
        {
            return Err("project_mapping_format_unsupported".into());
        }
        if !manifest
            .files
            .contains_key(&format!("{group_prefix}summary.json"))
            || !manifest
                .files
                .contains_key(&format!("{group_prefix}updates.jsonl"))
        {
            return Err("project_mapping_format_unsupported".into());
        }
        let marked = manifest.files.get(&marker);
        if grok_long(&manifest.cwd) {
            let recorded = marked.ok_or("project_mapping_format_unsupported")?;
            if STANDARD.decode(recorded).map_err(|_| "session_invalid")? != manifest.cwd.as_bytes()
            {
                return Err("project_mapping_format_unsupported".into());
            }
        } else if marked.is_some() {
            return Err("project_mapping_format_unsupported".into());
        }
        let summary = STANDARD
            .decode(
                manifest
                    .files
                    .get(&format!("{group_prefix}summary.json"))
                    .ok_or("session_invalid")?,
            )
            .map_err(|_| "session_invalid")?;
        let summary: Value = serde_json::from_slice(&summary).map_err(|_| "session_invalid")?;
        let recorded = summary["info"]["cwd"]
            .as_str()
            .or_else(|| summary["cwd"].as_str());
        if recorded != Some(manifest.cwd.as_str()) {
            return Err("project_mapping_format_unsupported".into());
        }
    } else if manifest.files.keys().any(|key| !key.starts_with(&prefix)) {
        return Err("project_mapping_format_unsupported".into());
    }
    if manifest.agent != "grok" {
        validate_primary_record(manifest)?;
    }
    Ok(())
}

fn validate_primary_record(manifest: &Manifest) -> Result<()> {
    let main = manifest
        .files
        .iter()
        .filter(|(path, _)| {
            path.ends_with(".jsonl")
                && !(matches!(manifest.agent.as_str(), "claude" | "claude-code")
                    && path.contains("/subagents/"))
        })
        .collect::<Vec<_>>();
    if main.len() != 1 {
        return Err("project_mapping_format_unsupported".into());
    }
    let bytes = STANDARD.decode(main[0].1).map_err(|_| "session_invalid")?;
    let mut records = bytes.split(|b| *b == b'\n').filter(|line| !line.is_empty());
    match manifest.agent.as_str() {
        "codex" | "chatgpt-work" => {
            let first: Value = serde_json::from_slice(records.next().ok_or("session_invalid")?)
                .map_err(|_| "session_invalid")?;
            if first["type"] != "session_meta"
                || first["payload"]["id"].as_str() != Some(manifest.session.as_str())
                || first["payload"]["cwd"].as_str() != Some(manifest.cwd.as_str())
            {
                return Err("project_mapping_format_unsupported".into());
            }
        }
        "pi" => {
            let first: Value = serde_json::from_slice(records.next().ok_or("session_invalid")?)
                .map_err(|_| "session_invalid")?;
            if first["type"] != "session"
                || first["id"].as_str() != Some(manifest.session.as_str())
                || first["cwd"].as_str() != Some(manifest.cwd.as_str())
            {
                return Err("project_mapping_format_unsupported".into());
            }
        }
        "claude" | "claude-code" => {
            let mut found = false;
            for line in records {
                let record: Value = serde_json::from_slice(line).map_err(|_| "session_invalid")?;
                if record["sessionId"].as_str() == Some(manifest.session.as_str())
                    && record["cwd"].as_str() == Some(manifest.cwd.as_str())
                {
                    found = true;
                    break;
                }
            }
            if !found {
                return Err("project_mapping_format_unsupported".into());
            }
        }
        _ => return Err("project_mapping_format_unsupported".into()),
    }
    Ok(())
}

fn relocate(relative: &str, agent: &str, old_cwd: &str, new_cwd: &str) -> Result<String> {
    let (prefix, old, new) = match agent {
        "claude" | "claude-code" => ("projects", claude_group(old_cwd)?, claude_group(new_cwd)?),
        "pi" => ("sessions", pi_group(old_cwd), pi_group(new_cwd)),
        "grok" => ("sessions", grok_group(old_cwd), grok_group(new_cwd)),
        "codex" | "chatgpt-work" => return Ok(relative.to_owned()),
        _ => return Err("project_mapping_format_unsupported".into()),
    };
    if old.len() > 255 || new.len() > 255 {
        return Err("project_mapping_format_unsupported".into());
    }
    let rest = relative
        .strip_prefix(prefix)
        .and_then(|s| s.strip_prefix('/'))
        .and_then(|s| s.strip_prefix(&old))
        .and_then(|s| s.strip_prefix('/'))
        .ok_or("project_mapping_format_unsupported")?;
    if rest.is_empty() {
        return Err("project_mapping_format_unsupported".into());
    }
    Ok(format!("{prefix}/{new}/{rest}"))
}

/// Return a new snapshot with verified location metadata and provider grouping
/// adapted to the receiving device. The original immutable snapshot is intact.
pub fn transform_manifest(manifest: &Manifest, mappings: &[Mapping]) -> Result<Manifest> {
    let prepared = prepare(mappings)?;
    if mappings.is_empty() || manifest.cwd.is_empty() {
        return Ok(manifest.clone());
    }
    let Some(new_cwd) = remap_prepared(&manifest.cwd, &prepared)? else {
        return Ok(manifest.clone());
    };
    if manifest.agent == "agy" {
        return Err("project_mapping_format_unsupported".into());
    }
    let mut files = BTreeMap::new();
    let mut metadata_changed = false;
    let grok_old_marker =
        (manifest.agent == "grok").then(|| format!("sessions/{}/.cwd", grok_group(&manifest.cwd)));
    if manifest.agent == "grok" && grok_long(&manifest.cwd) {
        let marker = grok_old_marker
            .as_ref()
            .ok_or("project_mapping_format_unsupported")?;
        let recorded = manifest
            .files
            .get(marker)
            .ok_or("project_mapping_format_unsupported")?;
        if STANDARD.decode(recorded).map_err(|_| "session_invalid")? != manifest.cwd.as_bytes() {
            return Err("project_mapping_format_unsupported".into());
        }
    }
    for (relative, encoded) in &manifest.files {
        if grok_old_marker.as_deref() == Some(relative.as_str()) {
            continue;
        }
        let new_relative = relocate(relative, &manifest.agent, &manifest.cwd, &new_cwd)?;
        let bytes = STANDARD.decode(encoded).map_err(|_| "session_invalid")?;
        let new_bytes = if manifest.agent == "grok" {
            if relative.ends_with("/summary.json") {
                let mut value: Value =
                    serde_json::from_slice(&bytes).map_err(|_| "session_invalid")?;
                let original = value.clone();
                mapped_field(&mut value, "cwd", &prepared)?;
                if let Some(info) = value.get_mut("info") {
                    mapped_field(info, "cwd", &prepared)?;
                }
                metadata_changed |= value != original;
                serde_json::to_vec(&value).map_err(|_| "session_invalid")?
            } else {
                bytes
            }
        } else {
            let text = std::str::from_utf8(&bytes).map_err(|_| "session_invalid")?;
            let (rewritten, changed) = rewrite_jsonl(text, &manifest.agent, &prepared)?;
            metadata_changed |= changed;
            rewritten.into_bytes()
        };
        if files
            .insert(new_relative, STANDARD.encode(new_bytes))
            .is_some()
        {
            return Err("project_mapping_conflict".into());
        }
    }
    if !metadata_changed {
        return Err("project_mapping_format_unsupported".into());
    }
    if manifest.agent == "grok" && grok_long(&new_cwd) {
        let marker = format!("sessions/{}/.cwd", grok_group(&new_cwd));
        if files.insert(marker, STANDARD.encode(&new_cwd)).is_some() {
            return Err("project_mapping_conflict".into());
        }
    }
    Ok(Manifest {
        cwd: new_cwd,
        files,
        ..manifest.clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn one(agent: &str, cwd: &str, relative: &str, contents: &str) -> Manifest {
        Manifest {
            version: 1,
            agent: agent.into(),
            session: "session-1".into(),
            cwd: cwd.into(),
            files: BTreeMap::from([(relative.into(), STANDARD.encode(contents))]),
        }
    }

    fn decoded(manifest: &Manifest) -> String {
        String::from_utf8(
            STANDARD
                .decode(manifest.files.values().next().unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    fn expected_target(path: &Path, suffix: &str) -> String {
        Path::new(&canonical_target(path).unwrap())
            .join(suffix)
            .to_string_lossy()
            .into()
    }

    #[test]
    fn cross_os_longest_component_boundary_and_conflicts() {
        let root = tempdir().unwrap();
        let general = root.path().join("general");
        let special = root.path().join("special");
        fs::create_dir_all(&general).unwrap();
        fs::create_dir_all(&special).unwrap();
        let mappings = [
            Mapping {
                source: r"C:\Users\Alice\work".into(),
                target: general.to_string_lossy().into(),
            },
            Mapping {
                source: "c:/users/alice/work/app".into(),
                target: special.to_string_lossy().into(),
            },
        ];
        assert_eq!(
            remap_path("C:/USERS/ALICE/work/app/src", &mappings).unwrap(),
            Some(expected_target(&special, "src"))
        );
        assert_eq!(
            remap_path("C:/users/alice/workbench", &mappings).unwrap(),
            None
        );
        assert_eq!(
            remap_path("C:/users/alice/work/other", &mappings).unwrap(),
            Some(expected_target(&general, "other"))
        );
        let unc = [Mapping {
            source: r"\\server\share\work".into(),
            target: general.to_string_lossy().into(),
        }];
        assert_eq!(
            remap_path("//SERVER/SHARE/work/app", &unc).unwrap(),
            Some(expected_target(&general, "app"))
        );
        assert_eq!(
            remap_path(r"\\?\UNC\server\share\work\app", &unc).unwrap(),
            Some(expected_target(&general, "app"))
        );
        assert!(remap_path("C:/users/alice/work/../secret", &mappings).is_err());
        assert!(validate_mappings(&[
            mappings[0].clone(),
            Mapping {
                source: "c:/USERS/alice/WORK".into(),
                target: special.to_string_lossy().into(),
            }
        ])
        .is_err());
        assert!(validate_mappings(&[
            mappings[0].clone(),
            Mapping {
                source: "/different".into(),
                target: general.to_string_lossy().into(),
            }
        ])
        .is_ok());
        assert!(validate_mappings(&[Mapping {
            source: "/source".into(),
            target: root.path().join("missing").to_string_lossy().into(),
        }])
        .is_err());
    }

    #[test]
    fn codex_rewrites_only_structural_cwd_and_preserves_history() {
        let target = tempdir().unwrap();
        let mapping = [Mapping {
            source: "/old".into(),
            target: target.path().to_string_lossy().into(),
        }];
        let history = "{\"type\":\"response_item\",\"payload\":{\"text\":\"/old/project\",\"command\":\"cd /old/project\"}}\n";
        let source = format!("{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"session-1\",\"cwd\":\"/old/project\"}}}}\n{{\"type\":\"turn_context\",\"payload\":{{\"cwd\":\"/old/project\"}}}}\n{history}");
        let manifest = one(
            "codex",
            "/old/project",
            "sessions/2026/01/01/rollout-session-1.jsonl",
            &source,
        );
        let changed = transform_manifest(&manifest, &mapping).unwrap();
        assert_eq!(changed.cwd, expected_target(target.path(), "project"));
        assert_eq!(changed.files.keys().next(), manifest.files.keys().next());
        let text = decoded(&changed);
        assert!(text.ends_with(history));
        let records: Vec<serde_json::Value> = text
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(records[0]["payload"]["cwd"], changed.cwd);
        assert_eq!(records[1]["payload"]["cwd"], changed.cwd);
        assert_eq!(decoded(&manifest), source);
    }

    #[test]
    fn claude_and_pi_move_verified_project_groups_only() {
        let target = tempdir().unwrap();
        let mapping = [Mapping {
            source: "/old".into(),
            target: target.path().to_string_lossy().into(),
        }];
        let claude = one("claude", "/old/project", "projects/-old-project/session-1.jsonl",
            "{\"type\":\"user\",\"sessionId\":\"session-1\",\"cwd\":\"/old/project\",\"message\":{\"content\":\"/old/project\"}}\n");
        let mapped = transform_manifest(&claude, &mapping).unwrap();
        assert!(validate_provider_group(&mapped).is_ok());
        assert_eq!(
            mapped.files.keys().next().unwrap(),
            &format!(
                "projects/{}/session-1.jsonl",
                claude_group(&mapped.cwd).unwrap()
            )
        );
        assert_eq!(
            serde_json::from_str::<Value>(decoded(&mapped).trim()).unwrap()["message"]["content"],
            "/old/project"
        );
        let pi = one("pi", r"C:\work\app", "sessions/--C--work-app--/2026_session-1.jsonl",
            "{\"type\":\"session\",\"version\":3,\"id\":\"session-1\",\"cwd\":\"C:\\\\work\\\\app\"}\n{\"type\":\"message\",\"message\":{\"content\":\"C:\\\\work\\\\app\"}}\n");
        let pi_mapping = [Mapping {
            source: r"C:\work".into(),
            target: target.path().to_string_lossy().into(),
        }];
        let changed = transform_manifest(&pi, &pi_mapping).unwrap();
        assert!(validate_provider_group(&changed).is_ok());
        assert_eq!(
            changed.files.keys().next().unwrap(),
            &format!("sessions/{}/2026_session-1.jsonl", pi_group(&changed.cwd))
        );
        assert!(decoded(&changed)
            .ends_with("{\"type\":\"message\",\"message\":{\"content\":\"C:\\\\work\\\\app\"}}\n"));
    }

    #[test]
    fn grok_changes_summary_and_group_but_not_update_stream() {
        let target = tempdir().unwrap();
        let mapping = [Mapping {
            source: "/old".into(),
            target: target.path().to_string_lossy().into(),
        }];
        let mut manifest = one(
            "grok",
            "/old/project",
            "sessions/%2Fold%2Fproject/session-1/summary.json",
            "{\"info\":{\"cwd\":\"/old/project\",\"title\":\"mention /old/project\"}}",
        );
        let history = "{\"sessionUpdate\":\"user_message_chunk\",\"text\":\"/old/project\"}\n";
        manifest.files.insert(
            "sessions/%2Fold%2Fproject/session-1/updates.jsonl".into(),
            STANDARD.encode(history),
        );
        let mapped = transform_manifest(&manifest, &mapping).unwrap();
        assert!(validate_provider_group(&mapped).is_ok());
        let group = grok_group(&mapped.cwd);
        let summary = mapped
            .files
            .get(&format!("sessions/{group}/session-1/summary.json"))
            .unwrap();
        let value: Value = serde_json::from_slice(&STANDARD.decode(summary).unwrap()).unwrap();
        assert_eq!(value["info"]["cwd"], mapped.cwd);
        assert_eq!(value["info"]["title"], "mention /old/project");
        assert_eq!(
            STANDARD
                .decode(
                    mapped
                        .files
                        .get(&format!("sessions/{group}/session-1/updates.jsonl"))
                        .unwrap()
                )
                .unwrap(),
            history.as_bytes()
        );
    }

    #[test]
    fn grok_long_groups_require_and_rewrite_cwd_marker() {
        let target = tempdir().unwrap();
        let old_cwd = format!("/old/{}", "中".repeat(30));
        assert!(grok_long(&old_cwd));
        let old_group = grok_group(&old_cwd);
        let mut manifest = one(
            "grok",
            &old_cwd,
            &format!("sessions/{old_group}/session-1/summary.json"),
            &serde_json::json!({"info":{"cwd":old_cwd}}).to_string(),
        );
        manifest.files.insert(
            format!("sessions/{old_group}/session-1/updates.jsonl"),
            STANDARD.encode("{\"method\":\"session/update\"}\n"),
        );
        let old_marker = format!("sessions/{old_group}/.cwd");
        manifest
            .files
            .insert(old_marker.clone(), STANDARD.encode(&old_cwd));
        let mapping = [Mapping {
            source: old_cwd.clone(),
            target: target.path().to_string_lossy().into(),
        }];
        let mapped = transform_manifest(&manifest, &mapping).unwrap();
        assert!(validate_provider_group(&mapped).is_ok());
        assert!(!mapped.files.contains_key(&old_marker));
        assert!(!mapped.files.keys().any(|key| key.ends_with("/.cwd")));
        let mut incomplete = manifest.clone();
        incomplete.files.remove(&old_marker);
        assert_eq!(
            transform_manifest(&incomplete, &mapping).unwrap_err(),
            "project_mapping_format_unsupported"
        );

        let deep = target
            .path()
            .join("測".repeat(12))
            .join("試".repeat(12))
            .join("案".repeat(12));
        fs::create_dir_all(&deep).unwrap();
        let mut short = one(
            "grok",
            "/old/project",
            "sessions/%2Fold%2Fproject/session-1/summary.json",
            "{\"info\":{\"cwd\":\"/old/project\"}}",
        );
        short.files.insert(
            "sessions/%2Fold%2Fproject/session-1/updates.jsonl".into(),
            STANDARD.encode("{\"method\":\"session/update\"}\n"),
        );
        let long_mapping = [Mapping {
            source: "/old/project".into(),
            target: deep.to_string_lossy().into(),
        }];
        let long = transform_manifest(&short, &long_mapping).unwrap();
        assert!(validate_provider_group(&long).is_ok());
        assert!(grok_long(&long.cwd));
        let marker = format!("sessions/{}/.cwd", grok_group(&long.cwd));
        assert_eq!(
            STANDARD.decode(long.files.get(&marker).unwrap()).unwrap(),
            long.cwd.as_bytes()
        );
    }

    #[test]
    fn unsupported_group_and_agy_are_explicit() {
        let target = tempdir().unwrap();
        let mapping = [Mapping {
            source: "/old".into(),
            target: target.path().to_string_lossy().into(),
        }];
        let bad = one(
            "pi",
            "/old/app",
            "sessions/wrong/session-1.jsonl",
            "{\"type\":\"session\",\"cwd\":\"/old/app\"}\n",
        );
        assert_eq!(
            transform_manifest(&bad, &mapping).unwrap_err(),
            "project_mapping_format_unsupported"
        );
        assert_eq!(
            validate_provider_group(&bad).unwrap_err(),
            "project_mapping_format_unsupported"
        );
        let agy = one("agy", "/old/app", "conversations/session-1.db", "data");
        assert_eq!(
            transform_manifest(&agy, &mapping).unwrap_err(),
            "project_mapping_format_unsupported"
        );
        assert_eq!(transform_manifest(&agy, &[]).unwrap().files, agy.files);
    }

    #[test]
    fn received_manifest_cannot_claim_a_different_cwd_than_native_header() {
        let codex = one(
            "codex",
            "/local",
            "sessions/2026/01/01/rollout-session-1.jsonl",
            "{\"type\":\"session_meta\",\"payload\":{\"id\":\"session-1\",\"cwd\":\"/foreign\"}}\n",
        );
        assert_eq!(
            validate_provider_group(&codex).unwrap_err(),
            "project_mapping_format_unsupported"
        );
        let pi = one(
            "pi",
            "/local",
            "sessions/--local--/2026_session-1.jsonl",
            "{\"type\":\"session\",\"version\":3,\"id\":\"session-1\",\"cwd\":\"/foreign\"}\n",
        );
        assert_eq!(
            validate_provider_group(&pi).unwrap_err(),
            "project_mapping_format_unsupported"
        );
    }

    #[test]
    fn claude_long_group_matches_installed_js_encoder() {
        let slug = claude_group(&format!("/{}", "a".repeat(210))).unwrap();
        assert_eq!(slug.len(), 207);
        assert!(slug.starts_with("-aaaa"));
        assert!(slug.ends_with("-djaaup"));
        assert_eq!(claude_group("/🦀").unwrap(), "---");
    }
}
