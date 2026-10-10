//! Receiver-side requirement resolution for a restored Hermes workspace.
//!
//! [`super`] captures inert requirement *metadata*: environment variable
//! *names*, command names, a declared OS `platforms:` list, and opaque
//! app/credential/MCP/model markers. This module decides, on the *receiving*
//! host, whether each of those requirements is actually satisfied — without
//! executing received code and without ever inspecting a secret value:
//!
//! * `env` — the variable's **presence only** (`std::env::var_os`), never its
//!   value; a name bound to an empty string still counts as present.
//! * `runtime` — the command name is looked up in the *trusted, absolute* local
//!   `PATH` entries and must be a regular file with the executable bit set. The
//!   command is never run.
//! * `os` — the declared `platforms:` list re-evaluated against the receiver OS.
//! * `app` / `credential` / `mcp` / `model` / `memory` (and any unknown kind) —
//!   cannot be probed from a packet without reading credentials or host
//!   registries, so they stay **unverified** and need the recipient's explicit
//!   acknowledgment before a faithful resume is permitted. They are never
//!   reported as *verified*.
//!
//! A verifiable requirement that the probe rejects is reported as `missing` and
//! stays blocking — it is never downgraded to a warning. An opaque requirement
//! stays blocking until the recipient acknowledges it for the *exact* received
//! manifest; a changed manifest carries a different fingerprint and the old
//! acknowledgment no longer matches.
//!
//! Nothing here writes into the workspace bundle. Declared prerequisite names
//! are only ever read out of the restored `SKILL.md` for the recipient's own
//! review; the credential files themselves are never opened.

use super::{
    frontmatter, host_os, safe_text, ManifestDependency, WorkspaceManifest, MAX_WORKSPACE_FILE,
};
use crate::sync::bundle;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

/// Recipient-local store of explicit acknowledgments, keyed by the received
/// manifest's content hash. It lives directly in the managed home (a dotfile at
/// the root is never captured: the workspace walk only descends `skills/`), so
/// it can never travel back inside a bundle.
const ACK_STORE: &str = ".bastet-requirements.json";
const MAX_ACK_MANIFESTS: usize = 64;
const MAX_ACK_KEYS: usize = 256;
const MAX_ACK_BYTES: u64 = 1024 * 1024;

/// A resolved requirement's state on the receiving host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Satisfaction {
    /// A probe confirmed the requirement is met.
    Verified,
    /// A probe confirmed a *verifiable* requirement is unmet; it blocks.
    Missing,
    /// Opaque: cannot be probed without reading secrets or host registries.
    /// Blocks until the recipient explicitly acknowledges it.
    Unverified,
    /// Opaque and explicitly acknowledged for this manifest. Never "verified".
    Acknowledged,
}

impl Satisfaction {
    fn status(self) -> &'static str {
        match self {
            Satisfaction::Verified => "verified",
            Satisfaction::Missing => "missing",
            Satisfaction::Unverified => "unverified",
            Satisfaction::Acknowledged => "acknowledged",
        }
    }

    fn blocking(self) -> bool {
        matches!(self, Satisfaction::Missing | Satisfaction::Unverified)
    }
}

/// One requirement as resolved on the receiving host.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ResolvedRequirement {
    pub kind: String,
    pub name: String,
    pub detail: String,
    /// `"verified"`, `"missing"`, `"unverified"` or `"acknowledged"`.
    pub status: String,
    /// `true` when this requirement still blocks a faithful resume.
    pub blocking: bool,
    /// Prerequisite names recovered from the restored `SKILL.md` for the
    /// recipient's review. Never read from a credential file's contents.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recovered: Vec<String>,
}

/// The full receiver-side verdict for a workspace manifest's requirements.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RequirementsEvidence {
    pub requirements: Vec<ResolvedRequirement>,
    /// Blocking codes (`workspace_dependency_<kind>`): unmet verifiable
    /// requirements and opaque requirements not yet acknowledged.
    pub blockers: Vec<String>,
    /// Honest non-blocking codes (`workspace_dependency_<kind>_acknowledged`):
    /// opaque requirements the recipient acknowledged, still not verified.
    pub warnings: Vec<String>,
    /// Probe-verified codes (`workspace_dependency_<kind>_resolved`).
    pub verified: Vec<String>,
}

impl RequirementsEvidence {
    /// `true` when nothing blocks a faithful resume.
    pub(crate) fn ready(&self) -> bool {
        self.blockers.is_empty()
    }
}

/// The host a restored home will run on. `os` is the receiver OS identifier;
/// `path_dirs` are the trusted *absolute* `PATH` entries to scan; `env_names`
/// are the ambient variable *names* only (a value is never inspected); `home`
/// is the managed home that carries the acknowledgment store and the restored
/// `SKILL.md` files, if any.
pub(crate) struct Receiver {
    pub os: String,
    pub path_dirs: Vec<PathBuf>,
    pub env_names: BTreeSet<String>,
    pub home: Option<PathBuf>,
}

impl Receiver {
    /// The live local host. Environment *values* are never read: only the set of
    /// bound names is retained, so a required variable is judged by presence.
    pub(crate) fn local(home: Option<PathBuf>) -> Self {
        let env_names = std::env::vars_os()
            .map(|(name, _value)| name.to_string_lossy().into_owned())
            .collect();
        let path_dirs = std::env::var_os("PATH")
            .map(|value| {
                std::env::split_paths(&value)
                    .filter(|dir| dir.is_absolute())
                    .collect()
            })
            .unwrap_or_default();
        Self {
            os: host_os().to_string(),
            path_dirs,
            env_names,
            home,
        }
    }
}

fn requirement_key(kind: &str, name: &str) -> String {
    format!("{kind}:{name}")
}

/// Whether an opaque requirement of this kind can be acknowledged at all. A
/// plain PATH command (or the OS) is always verifiable, so it can never be
/// acknowledged — that is what keeps a missing verifiable dependency a blocker.
fn acknowledgeable(kind: &str, name: &str) -> bool {
    match kind {
        "env" | "os" => false,
        "runtime" => !command_name_ok(name),
        _ => true,
    }
}

fn ack_name_ok(name: &str) -> bool {
    !name.is_empty() && name.len() <= 200 && !name.contains(['\n', '\r', '\0'])
}

/// A plain command token that is safe to join under a trusted `PATH` directory.
/// Anything with a separator, a parent reference or an odd character is not a
/// PATH command and is treated as opaque instead of being probed.
fn command_name_ok(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-'))
}

/// Whether `dir/name` (or its `.exe`/`.cmd`/`.bat` variant on Windows) is a
/// regular, executable file. The candidate is stat'd, never executed.
fn executable_candidate(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Scan the trusted local `PATH` directories for `name`. No shell, no spawn.
fn command_on_path(name: &str, dirs: &[PathBuf]) -> bool {
    if !command_name_ok(name) {
        return false;
    }
    let candidates: Vec<String> = if cfg!(windows) {
        ["", ".exe", ".cmd", ".bat"]
            .iter()
            .map(|extension| format!("{name}{extension}"))
            .collect()
    } else {
        vec![name.to_string()]
    };
    dirs.iter().filter(|dir| dir.is_absolute()).any(|dir| {
        candidates
            .iter()
            .any(|candidate| executable_candidate(&dir.join(candidate)))
    })
}

/// Declared names from a string-or-array frontmatter field, filtered for safety.
fn declared_names(frontmatter: &serde_json::Value, key: &str) -> Vec<String> {
    let items = match frontmatter.get(key) {
        Some(serde_json::Value::String(value)) => vec![value.clone()],
        Some(serde_json::Value::Array(values)) => values
            .iter()
            .filter_map(|value| value.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    };
    let mut names: Vec<String> = items
        .into_iter()
        .map(|name| name.trim().to_string())
        .filter(|name| safe_text(name, 200))
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Declared credential-file labels (`{path|name}` maps or bare strings). Only
/// the declared *label* is read out of `SKILL.md`; the credential file itself is
/// never opened.
fn declared_credential_labels(frontmatter: &serde_json::Value) -> Vec<String> {
    let Some(declared) = frontmatter.get("required_credential_files") else {
        return Vec::new();
    };
    let mut labels: Vec<String> = match declared {
        serde_json::Value::String(_) => declared_names(frontmatter, "required_credential_files"),
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(|item| match item {
                serde_json::Value::String(value) => Some(value.clone()),
                serde_json::Value::Object(map) => ["path", "name"]
                    .iter()
                    .find_map(|key| map.get(*key).and_then(|value| value.as_str()))
                    .map(str::to_string),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    labels.retain(|label| safe_text(label, 200));
    labels.sort();
    labels.dedup();
    labels
}

/// Recover the declared prerequisite names for an opaque `app`/`credential`
/// requirement from the restored `SKILL.md`. This is *display metadata for the
/// recipient's review*: it never enters the bundle and never reads a credential
/// file. A missing or unsafe path yields nothing.
fn recover_declared(home: &Path, skill: &str, kind: &str) -> Vec<String> {
    if !matches!(kind, "app" | "credential") || !crate::portable_paths::path_is_portable(skill) {
        return Vec::new();
    }
    let relative = format!("skills/{skill}/SKILL.md");
    let Ok(Some(bytes)) = crate::review::local_file(home, &relative, MAX_WORKSPACE_FILE) else {
        return Vec::new();
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return Vec::new();
    };
    let Some(frontmatter) = frontmatter(text) else {
        return Vec::new();
    };
    if kind == "app" {
        declared_names(&frontmatter, "requires_apps")
    } else {
        declared_credential_labels(&frontmatter)
    }
}

fn resolve_dependency(
    dep: &ManifestDependency,
    receiver: &Receiver,
    acks: &BTreeSet<String>,
) -> ResolvedRequirement {
    let key = requirement_key(&dep.kind, &dep.name);
    let (satisfaction, recovered) = match dep.kind.as_str() {
        // An `os` requirement is re-evaluated on the receiver (see `blocks_on`),
        // never trusted from the sender's serialized verdict.
        "os" => (
            if dep.blocks_on(&receiver.os) {
                Satisfaction::Missing
            } else {
                Satisfaction::Verified
            },
            Vec::new(),
        ),
        "env" => (
            // Presence only: the value is never read or compared.
            if receiver.env_names.contains(&dep.name) {
                Satisfaction::Verified
            } else {
                Satisfaction::Missing
            },
            Vec::new(),
        ),
        "runtime" => {
            if !command_name_ok(&dep.name) {
                // e.g. a package spec (`pip:foo`): not a PATH command, so opaque.
                (
                    if acks.contains(&key) {
                        Satisfaction::Acknowledged
                    } else {
                        Satisfaction::Unverified
                    },
                    Vec::new(),
                )
            } else if command_on_path(&dep.name, &receiver.path_dirs) {
                (Satisfaction::Verified, Vec::new())
            } else {
                (Satisfaction::Missing, Vec::new())
            }
        }
        _ => {
            // Opaque kinds: app / credential / mcp / model / memory / unknown.
            let recovered = receiver
                .home
                .as_deref()
                .map(|home| recover_declared(home, &dep.name, &dep.kind))
                .unwrap_or_default();
            (
                if acks.contains(&key) {
                    Satisfaction::Acknowledged
                } else {
                    Satisfaction::Unverified
                },
                recovered,
            )
        }
    };
    ResolvedRequirement {
        kind: dep.kind.clone(),
        name: dep.name.clone(),
        detail: dep.detail.clone(),
        status: satisfaction.status().into(),
        blocking: satisfaction.blocking(),
        recovered,
    }
}

/// Resolve every dependency of a *validated* manifest for `receiver`, honoring
/// acknowledgments stored under `receiver.home` that match `fingerprint` — the
/// content hash of the exact received manifest text.
pub(crate) fn resolve_manifest(
    manifest: &WorkspaceManifest,
    receiver: &Receiver,
    fingerprint: &str,
) -> Result<RequirementsEvidence, String> {
    let acks = match receiver.home.as_deref() {
        Some(home) => acknowledgments_for(home, fingerprint)?,
        None => BTreeSet::new(),
    };
    let mut evidence = RequirementsEvidence::default();
    for dep in &manifest.dependencies {
        let resolved = resolve_dependency(dep, receiver, &acks);
        match resolved.status.as_str() {
            "verified" => evidence
                .verified
                .push(format!("workspace_dependency_{}_resolved", resolved.kind)),
            "acknowledged" => evidence.warnings.push(format!(
                "workspace_dependency_{}_acknowledged",
                resolved.kind
            )),
            _ => evidence
                .blockers
                .push(format!("workspace_dependency_{}", resolved.kind)),
        }
        evidence.requirements.push(resolved);
    }
    evidence.blockers.sort();
    evidence.blockers.dedup();
    evidence.warnings.sort();
    evidence.warnings.dedup();
    evidence.verified.sort();
    evidence.verified.dedup();
    Ok(evidence)
}

/// Parse and validate a received workspace manifest, then resolve it for the
/// live local host using `home` for the acknowledgment store and `SKILL.md`
/// recovery. Read-only: it never writes to the bundle or the home.
#[allow(dead_code)] // receive-side seam: wired by the readiness path after this module lands
pub(crate) fn receive_for_home(text: &str, home: &Path) -> Result<RequirementsEvidence, String> {
    let manifest: WorkspaceManifest =
        serde_json::from_str(text).map_err(|_| "workspace_format".to_string())?;
    manifest.validate()?;
    let receiver = Receiver::local(Some(home.to_path_buf()));
    resolve_manifest(&manifest, &receiver, &manifest_fingerprint(text))
}

/// The content hash that binds an acknowledgment to one exact manifest.
pub(crate) fn manifest_fingerprint(text: &str) -> String {
    bundle::hash(text.as_bytes())
}

fn ack_path(home: &Path) -> PathBuf {
    home.join(ACK_STORE)
}

fn read_acks(home: &Path) -> Result<BTreeMap<String, BTreeSet<String>>, String> {
    let path = ack_path(home);
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let bytes = crate::sync::storage::read(&path, MAX_ACK_BYTES)?;
    serde_json::from_slice(&bytes).map_err(|_| "local_store_damaged".to_string())
}

fn write_acks(home: &Path, store: &BTreeMap<String, BTreeSet<String>>) -> Result<(), String> {
    let bytes = serde_json::to_vec(store).map_err(|_| "local_store_damaged".to_string())?;
    crate::sync::storage::replace(&ack_path(home), &bytes)
}

/// Acknowledgment keys recorded for the manifest identified by `fingerprint`.
fn acknowledgments_for(home: &Path, fingerprint: &str) -> Result<BTreeSet<String>, String> {
    Ok(read_acks(home)?.remove(fingerprint).unwrap_or_default())
}

/// Acknowledgment keys recorded for the exact manifest `text`.
#[allow(dead_code)] // receive-side seam: wired by the readiness path after this module lands
pub(crate) fn acknowledgments(home: &Path, text: &str) -> Result<BTreeSet<String>, String> {
    acknowledgments_for(home, &manifest_fingerprint(text))
}

/// Record the recipient's explicit acknowledgment that an opaque requirement is
/// provisioned, bound to the exact received manifest. Idempotent. A verifiable
/// requirement (an OS, a PATH command or an environment variable) is refused:
/// acknowledgment can never turn an unmet probe into a pass.
#[allow(dead_code)] // receive-side seam: wired by the readiness path after this module lands
pub(crate) fn acknowledge(home: &Path, text: &str, kind: &str, name: &str) -> Result<(), String> {
    if !acknowledgeable(kind, name) {
        return Err("requirement_not_acknowledgeable".into());
    }
    if !ack_name_ok(name) || !safe_text(kind, 64) {
        return Err("workspace_unsafe".into());
    }
    let fingerprint = manifest_fingerprint(text);
    let key = requirement_key(kind, name);
    let mut store = read_acks(home)?;
    if !store.contains_key(&fingerprint) && store.len() >= MAX_ACK_MANIFESTS {
        return Err("workspace_limit".into());
    }
    let keys = store.entry(fingerprint).or_default();
    if keys.len() >= MAX_ACK_KEYS && !keys.contains(&key) {
        return Err("workspace_limit".into());
    }
    keys.insert(key);
    write_acks(home, &store)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(text: &str) -> WorkspaceManifest {
        let manifest: WorkspaceManifest = serde_json::from_str(text).unwrap();
        manifest.validate().unwrap();
        manifest
    }

    fn manifest_text(dependencies: Vec<serde_json::Value>) -> String {
        serde_json::to_string(&json!({
            "version": 1, "agent": "hermes", "files": [], "issues": [],
            "dependencies": dependencies,
        }))
        .unwrap()
    }

    fn receiver(os: &str, path_dirs: Vec<PathBuf>, env_names: &[&str]) -> Receiver {
        Receiver {
            os: os.into(),
            path_dirs,
            env_names: env_names.iter().map(|name| (*name).to_string()).collect(),
            home: None,
        }
    }

    fn resolve(text: &str, receiver: &Receiver) -> RequirementsEvidence {
        resolve_manifest(&parse(text), receiver, &manifest_fingerprint(text)).unwrap()
    }

    #[test]
    fn environment_requirement_resolves_on_presence_and_stays_blocking_when_absent() {
        let text = manifest_text(vec![json!({
            "kind": "env", "name": "NEEDED_KEY", "detail": "provide", "blocking": true,
        })]);

        // Absent: a verifiable miss is a blocker, never a warning.
        let absent = receiver("linux", Vec::new(), &[]);
        let evidence = resolve(&text, &absent);
        assert_eq!(evidence.requirements[0].status, "missing");
        assert!(evidence.requirements[0].blocking);
        assert!(evidence
            .blockers
            .contains(&"workspace_dependency_env".to_string()));
        assert!(evidence.warnings.is_empty());
        assert!(!evidence.ready());

        // Present: resolved. Only the name is checked — the value is never read.
        let present = receiver("linux", Vec::new(), &["NEEDED_KEY"]);
        let evidence = resolve(&text, &present);
        assert_eq!(evidence.requirements[0].status, "verified");
        assert!(!evidence.requirements[0].blocking);
        assert!(evidence
            .verified
            .contains(&"workspace_dependency_env_resolved".to_string()));
        assert!(evidence.ready());
    }

    #[cfg(unix)]
    #[test]
    fn command_requirement_scans_path_without_executing() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let sentinel = dir.path().join("executed-sentinel");
        let tool = bin.join("mytool");
        std::fs::write(&tool, format!("#!/bin/sh\ntouch {}\n", sentinel.display())).unwrap();

        let text = manifest_text(vec![json!({
            "kind": "runtime", "name": "mytool", "detail": "tool", "blocking": true,
        })]);
        let rec = receiver("linux", vec![bin.clone()], &[]);

        // Present as a file but not executable: still missing (permission).
        let evidence = resolve(&text, &rec);
        assert_eq!(evidence.requirements[0].status, "missing");
        assert!(evidence.requirements[0].blocking);

        // Same file, absent from PATH entirely: missing.
        let absent_dir = dir.path().join("elsewhere");
        std::fs::create_dir_all(&absent_dir).unwrap();
        let evidence = resolve(&text, &receiver("linux", vec![absent_dir], &[]));
        assert_eq!(evidence.requirements[0].status, "missing");

        // Executable bit set: resolved without ever running the command.
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        let evidence = resolve(&text, &rec);
        assert_eq!(evidence.requirements[0].status, "verified");
        assert!(!evidence.requirements[0].blocking);
        assert!(!sentinel.exists(), "the command must never be executed");
    }

    #[test]
    fn wrong_operating_system_still_blocks() {
        let text = manifest_text(vec![json!({
            "kind": "os", "name": "linuxonly",
            "detail": "skill declares platforms: linux",
            "blocking": true, "platforms": ["linux"],
        })]);
        let blocked = resolve(&text, &receiver("win32", Vec::new(), &[]));
        assert_eq!(blocked.requirements[0].status, "missing");
        assert!(blocked.requirements[0].blocking);

        let clear = resolve(&text, &receiver("linux", Vec::new(), &[]));
        assert_eq!(clear.requirements[0].status, "verified");
        assert!(!clear.requirements[0].blocking);
    }

    #[test]
    fn opaque_requirement_stays_pending_until_its_own_acknowledgment() {
        let home = tempfile::tempdir().unwrap();
        let text = manifest_text(vec![json!({
            "kind": "credential", "name": "vault", "detail": "provision", "blocking": true,
        })]);
        let with_home = |home: &Path| {
            let mut rec = receiver("linux", Vec::new(), &[]);
            rec.home = Some(home.to_path_buf());
            rec
        };

        // Not acknowledged: unverified and blocking.
        let evidence = resolve(&text, &with_home(home.path()));
        assert_eq!(evidence.requirements[0].status, "unverified");
        assert!(evidence.requirements[0].blocking);
        assert!(!evidence.ready());

        // A verifiable kind can never be acknowledged.
        assert_eq!(
            acknowledge(home.path(), &text, "env", "SOME_KEY").unwrap_err(),
            "requirement_not_acknowledgeable"
        );
        assert_eq!(
            acknowledge(home.path(), &text, "os", "linuxonly").unwrap_err(),
            "requirement_not_acknowledgeable"
        );

        // Explicit acknowledgment permits the resume but is not "verified".
        acknowledge(home.path(), &text, "credential", "vault").unwrap();
        let evidence = resolve(&text, &with_home(home.path()));
        assert_eq!(evidence.requirements[0].status, "acknowledged");
        assert!(!evidence.requirements[0].blocking);
        assert!(evidence.ready());
        assert!(evidence
            .warnings
            .contains(&"workspace_dependency_credential_acknowledged".to_string()));
        assert!(evidence.verified.is_empty());
    }

    #[test]
    fn acknowledgment_is_invalidated_when_the_manifest_changes() {
        let home = tempfile::tempdir().unwrap();
        let a = manifest_text(vec![json!({
            "kind": "mcp", "name": "demo", "detail": "install", "blocking": true,
        })]);
        acknowledge(home.path(), &a, "mcp", "demo").unwrap();
        assert!(acknowledgments(home.path(), &a)
            .unwrap()
            .contains("mcp:demo"));

        // A different manifest (its bytes changed) no longer matches the
        // recorded acknowledgment: it is unverified again.
        let b = manifest_text(vec![
            json!({"kind": "mcp", "name": "demo", "detail": "install", "blocking": true}),
            json!({"kind": "model", "name": "mycustom", "detail": "setup", "blocking": true}),
        ]);
        assert!(acknowledgments(home.path(), &b).unwrap().is_empty());
        let with_home = |text: &str| {
            let mut rec = receiver("linux", Vec::new(), &[]);
            rec.home = Some(home.path().to_path_buf());
            resolve(text, &rec)
        };
        assert_eq!(with_home(&a).requirements[0].status, "acknowledged");
        assert_eq!(with_home(&b).requirements[0].status, "unverified");
        assert!(with_home(&b).requirements[0].blocking);
    }

    #[test]
    fn declared_prerequisites_are_recovered_from_the_restored_skill_only() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join("skills/vault")).unwrap();
        std::fs::write(
            home.path().join("skills/vault/SKILL.md"),
            b"---\nname: vault\nrequires_apps: [thing-mcp]\nrequired_credential_files:\n  - path: creds/service.pem\n---\n# Vault\n",
        )
        .unwrap();
        // A credential file exists with different content; it must never be read
        // (only the declared label comes from SKILL.md).
        std::fs::create_dir_all(home.path().join("creds")).unwrap();
        std::fs::write(
            home.path().join("creds/service.pem"),
            b"CREDENTIAL-SENTINEL",
        )
        .unwrap();

        let mut rec = receiver("linux", Vec::new(), &[]);
        rec.home = Some(home.path().to_path_buf());

        let apps = resolve(
            &manifest_text(vec![json!({
                "kind": "app", "name": "vault", "detail": "enable app", "blocking": true,
            })]),
            &rec,
        );
        assert_eq!(
            apps.requirements[0].recovered,
            vec!["thing-mcp".to_string()]
        );

        let creds = resolve(
            &manifest_text(vec![json!({
                "kind": "credential", "name": "vault", "detail": "provision", "blocking": true,
            })]),
            &rec,
        );
        assert_eq!(
            creds.requirements[0].recovered,
            vec!["creds/service.pem".to_string()]
        );
        // The declared label is not the credential's contents.
        assert!(!creds.requirements[0]
            .recovered
            .iter()
            .any(|label| label.contains("SENTINEL")));

        // A traversal-shaped skill name recovers nothing (never walks out).
        let escaped = resolve(
            &manifest_text(vec![json!({
                "kind": "credential", "name": "../escape", "detail": "provision", "blocking": true,
            })]),
            &rec,
        );
        assert!(escaped.requirements[0].recovered.is_empty());
    }
}
