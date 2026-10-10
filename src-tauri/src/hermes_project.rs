//! Safe project working-state capture and isolated restore for cross-host
//! Hermes continuation.
//!
//! A capture is a self-describing base64 file map whose paths live in a single
//! reserved namespace:
//!
//! * `bastet/project-manifest.json` — a JSON sidecar describing the project
//!   (kind, sanitized git identity, per-file hashes/states/modes, explicit
//!   exclusions and machine-readable blockers).
//! * `bastet/project-files/<source>` — one base64 work file per entry, keyed by
//!   its project-relative `source` path.
//!
//! The capture never reads `.git`, credentials/`.env`, ignored or symlinked
//! paths, never records a credential-bearing remote URL and never mutates the
//! source tree. `git` is used with fixed, read-only, local subcommands only,
//! under a bounded wall-clock/output budget with system/global configuration and
//! every hook/fsmonitor/credential facility disabled; there is no shell and no
//! network access. Restoration writes into a NEW isolated directory and never
//! touches a receiver's existing workspace.
//!
//! ## Selected capture
//!
//! [`capture`] snapshots the full working state. [`capture_selected`] snapshots
//! only an explicit set of project-relative paths the user chose. An empty
//! selection is explicit, not "everything": it produces a valid manifest carrying
//! a `project_selected_empty` blocker and `baselineRequired`. A selected capture
//! is always marked `baselineRequired` because a user-chosen subset is never a
//! verified-complete working tree.
//!
//! ## Restore
//!
//! [`restore_to_new_directory`] atomically reserves a brand-new destination
//! directory with `create_dir` (which fails even against a concurrently created
//! *empty* directory), writes the captured files, and on failure removes only the
//! directory it created. It returns the manifest so a caller can read readiness
//! via [`read_manifest`]/[`ready`]; blocking snapshots are still restorable for
//! the files they do contain.
//!
//! ## Residual limitation (not solved here)
//!
//! Secret handling is filename-heuristic only (`is_secret_component`): it proves a
//! path is never captured, but it cannot redact a secret that a *non-secret-named*
//! file happens to contain. Content-level secret scanning remains a review
//! requirement and is intentionally not claimed here.
use crate::{
    portable_paths::{path_is_portable, paths_do_not_collide},
    sync::{
        bundle::{self, Result},
        storage,
    },
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// Namespace reserved for a project capture. Never collides with ordinary
/// project content because `validate` also proves portable, non-colliding keys.
pub(crate) const MANIFEST_PATH: &str = "bastet/project-manifest.json";
const SNAPSHOT_PREFIX: &str = "bastet/project-files/";
const VERSION: u32 = 1;

/// Bounds applied to a single capture. Exceeding any of them is reported as an
/// explicit `project_limit` blocker, never a silent drop.
pub(crate) const MAX_FILES: usize = 2048;
const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
const MAX_DEPTH: usize = 32;
const MAX_ENTRIES: usize = 20000;

/// Bounds applied to every `git` invocation: fixed wall-clock and output caps so
/// a hostile repository (huge untracked set, a blocking filter) cannot hang the
/// capture or exhaust memory.
const MAX_GIT_OUTPUT: u64 = 16 * 1024 * 1024;
const MAX_GIT_WAIT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct ProjectManifest {
    pub version: u32,
    /// `git` for a repository working tree, `files` for a plain directory.
    pub kind: String,
    /// Leaf project name only; no absolute host path is ever recorded.
    pub project: String,
    /// Present only for a git working tree.
    pub repo: Option<RepoIdentity>,
    /// True when the snapshot does not contain every required work file (a
    /// selected subset, an LFS pointer, an oversize or unreadable file, a
    /// submodule, ...) so the receiver must reconstruct the rest itself. An
    /// explicit working-tree deletion is *recorded*, not missing, so it does not
    /// set this.
    pub baseline_required: bool,
    pub files: Vec<FileEntry>,
    pub excluded: Vec<ExcludedEntry>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct RepoIdentity {
    /// Leaf name of the repository root.
    pub name: String,
    /// Remote URL with every credential, query and fragment removed, or absent
    /// when none is set.
    pub url: Option<String>,
    pub branch: Option<String>,
    /// 40-hex commit id, absent on an unborn branch.
    pub commit: Option<String>,
    /// 40-hex id of the tracked tree at `commit`.
    pub tree: Option<String>,
    pub detached: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct FileEntry {
    /// Project-relative path; the storage key is `SNAPSHOT_PREFIX + source`.
    pub source: String,
    pub sha256: String,
    pub bytes: u64,
    /// `tracked`, `dirty`, `untracked` (git) or `file` (plain directory).
    pub state: String,
    /// POSIX owner/group/other execute bit as a boolean. Always `false` on
    /// non-POSIX platforms (metadata is retained, not synthesized there).
    #[serde(default)]
    pub executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct ExcludedEntry {
    pub path: String,
    /// `secret`, `git`, `symlink`, `deleted`, `submodule`, `lfs`, `unreadable`,
    /// `unsafe`, `bound`.
    pub reason: String,
}

struct Work {
    source: String,
    bytes: Vec<u8>,
    state: String,
    executable: bool,
}

struct Capture {
    kind: String,
    project: String,
    repo: Option<RepoIdentity>,
    baseline_required: bool,
    work: Vec<Work>,
    excluded: Vec<ExcludedEntry>,
    blockers: Vec<String>,
}

/// Capture a project's full working state into a base64 file map.
///
/// A git working tree is captured as its full current working content (tracked
/// files as they exist on disk, plus bounded non-ignored untracked work files)
/// so a fresh folder is runnable. A tracked file deleted from the working tree
/// is recorded as an explicit `deleted` exclusion: the deletion *is* the
/// captured state, so a restore omits it and it never forces a baseline.
/// Unreadable, oversize and LFS-pointer tracked files, submodules and non-UTF8
/// paths are reported as exclusions and force `baselineRequired` because their
/// content genuinely cannot be captured. A plain directory is captured as
/// bounded ordinary files. `.git`, credentials/`.env`, ignored and symlinked
/// paths are never read.
pub(crate) fn capture(project: &Path) -> Result<BTreeMap<String, String>> {
    capture_with(project, OsStr::new("git"), None)
}

/// Capture only the explicitly selected project-relative `selected` sources.
///
/// An empty selection is explicit: it yields a valid manifest carrying a
/// `project_selected_empty` blocker and `baselineRequired`. Every selected
/// capture is marked `baselineRequired`.
pub(crate) fn capture_selected(
    project: &Path,
    selected: &[String],
) -> Result<BTreeMap<String, String>> {
    capture_with(project, OsStr::new("git"), Some(selected))
}

fn capture_with(
    project: &Path,
    program: &OsStr,
    selection: Option<&[String]>,
) -> Result<BTreeMap<String, String>> {
    let root = fs::canonicalize(project).map_err(|_| "project_unreadable".to_string())?;
    if !root.is_dir() {
        return Err("project_unreadable".into());
    }
    let probe = git_string(program, &root, &["rev-parse", "--is-inside-work-tree"]);
    let capture = if probe.as_deref() == Some("true") {
        capture_git(program, &root, selection)
    } else if !git_usable(program) && enclosing_git(&root) {
        // A `.git` exists in this tree but `git` could not run. Fail closed
        // rather than scanning the repository as a plain folder, which would
        // read ignored/secret paths.
        return Err("project_git_unavailable".into());
    } else {
        capture_files(&root, selection)
    };

    let mut capture = capture;
    capture.work.sort_by(|a, b| a.source.cmp(&b.source));
    capture.excluded.sort();
    capture.excluded.dedup();
    capture.blockers.sort();
    capture.blockers.dedup();

    let mut files = BTreeMap::new();
    let mut entries = Vec::with_capacity(capture.work.len());
    for work in &capture.work {
        let key = format!("{SNAPSHOT_PREFIX}{}", work.source);
        entries.push(FileEntry {
            source: work.source.clone(),
            sha256: bundle::hash(&work.bytes),
            bytes: work.bytes.len() as u64,
            state: work.state.clone(),
            executable: work.executable,
        });
        files.insert(key, STANDARD.encode(&work.bytes));
    }
    let manifest = ProjectManifest {
        version: VERSION,
        kind: capture.kind,
        project: capture.project,
        repo: capture.repo,
        baseline_required: capture.baseline_required,
        files: entries,
        excluded: capture.excluded,
        blockers: capture.blockers,
    };
    let json = serde_json::to_vec(&manifest).map_err(|_| "project_invalid".to_string())?;
    files.insert(MANIFEST_PATH.to_string(), STANDARD.encode(&json));

    // Self-check: a capture must always be a valid, restorable map.
    validate(&files)?;
    Ok(files)
}

/// Decode and return the manifest carried by a file map.
pub(crate) fn read_manifest(files: &BTreeMap<String, String>) -> Result<ProjectManifest> {
    let bytes = STANDARD
        .decode(files.get(MANIFEST_PATH).ok_or("project_invalid")?)
        .map_err(|_| "project_invalid".to_string())?;
    serde_json::from_slice(&bytes).map_err(|_| "project_invalid".to_string())
}

/// True when a manifest describes a snapshot that fully reconstructs the
/// captured work: no blockers and no required git baseline.
pub(crate) fn ready(manifest: &ProjectManifest) -> bool {
    !manifest.baseline_required && manifest.blockers.is_empty()
}

/// A storage path is admissible only inside the reserved namespace, with a
/// portable, credential-free, non-`.git` relative source.
pub(crate) fn allowed(path: &str) -> bool {
    if !path_is_portable(path) {
        return false;
    }
    if path == MANIFEST_PATH {
        return true;
    }
    match path.strip_prefix(SNAPSHOT_PREFIX) {
        Some(source) => safe_source(source),
        None => false,
    }
}

fn safe_source(source: &str) -> bool {
    !source.is_empty() && path_is_portable(source) && blocked_component(source).is_none()
}

/// Return the first exclusion reason for a relative path, if any.
fn blocked_component(path: &str) -> Option<&'static str> {
    for part in path.split('/') {
        if part == ".git" {
            return Some("git");
        }
        if is_secret_component(part) {
            return Some("secret");
        }
    }
    None
}

fn is_secret_component(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    const EXACT: &[&str] = &[
        ".env",
        ".envrc",
        ".netrc",
        ".npmrc",
        ".git-credentials",
        ".htpasswd",
        ".docker",
        ".kube",
        ".tfvars",
        ".tfstate",
        "credentials",
        "credentials.json",
        "secrets.json",
        "secrets.yaml",
        "secrets.yml",
        "secrets.toml",
        "auth.json",
        "id_rsa",
        "id_dsa",
        "id_ecdsa",
        "id_ed25519",
        "kubeconfig",
        ".kubeconfig",
        "terraform.tfstate",
        "terraform.tfstate.backup",
        "terraform.tfvars",
        "service-account.json",
        "serviceaccount.json",
        "service_account.json",
    ];
    if EXACT.contains(&n.as_str()) || n.starts_with(".env.") {
        return true;
    }
    // Terraform state/variable files and their backups carry live secrets.
    if n.ends_with(".tfstate") || n.contains(".tfstate.") || n.ends_with(".tfvars") {
        return true;
    }
    // SSH/FIDO private keys, including `id_ed25519_sk`, `id_ecdsa_sk`, ...
    if n.starts_with("id_") && n.ends_with("_sk") {
        return true;
    }
    // API-key material by common file naming convention.
    if n.contains("apikey") || n.contains("api-key") || n.contains("api_key") {
        return true;
    }
    if [".pem", ".key", ".p12", ".pfx", ".jks", ".keystore", ".ppk"]
        .iter()
        .any(|suffix| n.ends_with(suffix))
    {
        return true;
    }
    if n == "kubeconfig" || n.ends_with(".kubeconfig") {
        return true;
    }
    n.ends_with(".json")
        && (n.contains("credential")
            || n.contains("secret")
            || n.contains("service-account")
            || n.contains("service_account")
            || n.contains("serviceaccount"))
}

/// Fully validate a captured map: portable, non-colliding, reserved-namespace
/// paths, a well-formed manifest, a remote URL that is a fixed point of the
/// credential sanitizer, and content that matches every recorded hash.
pub(crate) fn validate(files: &BTreeMap<String, String>) -> Result<()> {
    if files.is_empty() || files.len() > MAX_FILES + 1 {
        return Err("project_invalid".into());
    }
    for key in files.keys() {
        if !allowed(key) {
            return Err("project_invalid".into());
        }
    }
    if !paths_do_not_collide(files.keys().map(String::as_str)) {
        return Err("project_invalid".into());
    }
    let manifest = read_manifest(files)?;
    if manifest.version != VERSION || !matches!(manifest.kind.as_str(), "git" | "files") {
        return Err("project_invalid".into());
    }
    if manifest.kind == "git" && manifest.repo.is_none() {
        return Err("project_invalid".into());
    }
    if let Some(repo) = &manifest.repo {
        for hash in [&repo.commit, &repo.tree].into_iter().flatten() {
            if !is_git_id(hash) {
                return Err("project_invalid".into());
            }
        }
        // A stored URL must already be fully sanitized: any surviving userinfo,
        // query or fragment (or a schemeless password form the sanitizer drops)
        // makes the record invalid.
        if let Some(url) = &repo.url {
            if sanitize_remote(url).as_deref() != Some(url.as_str()) {
                return Err("project_invalid".into());
            }
        }
    }
    if manifest.files.len() > MAX_FILES {
        return Err("project_limit".into());
    }
    let mut expected = BTreeSet::new();
    let mut total = 0u64;
    for entry in &manifest.files {
        if !safe_source(&entry.source) || !bundle::is_hash(&entry.sha256) {
            return Err("project_invalid".into());
        }
        let key = format!("{SNAPSHOT_PREFIX}{}", entry.source);
        if !expected.insert(key.clone()) {
            return Err("project_invalid".into());
        }
        let value = files.get(&key).ok_or("project_invalid")?;
        let bytes = STANDARD
            .decode(value)
            .map_err(|_| "project_invalid".to_string())?;
        if bytes.len() as u64 != entry.bytes {
            return Err("project_invalid".into());
        }
        if bundle::hash(&bytes) != entry.sha256 {
            return Err("project_hash_mismatch".into());
        }
        total = total.saturating_add(entry.bytes);
    }
    if total > MAX_TOTAL_BYTES {
        return Err("project_limit".into());
    }
    for key in files.keys() {
        if key != MANIFEST_PATH && !expected.contains(key) {
            return Err("project_invalid".into());
        }
    }
    Ok(())
}

/// Restore a captured map into a NEW isolated directory and return its manifest.
///
/// The destination is atomically reserved with `create_dir`; a concurrently
/// created directory — even an empty one — is never replaced. Content is written
/// directly into the reserved directory; if anything fails afterwards, exactly
/// that directory is removed and no foreign data is touched. Blocking snapshots
/// are still restorable for the files they contain; callers inspect
/// [`ready`]/[`read_manifest`] for readiness.
pub(crate) fn restore_to_new_directory(
    files: &BTreeMap<String, String>,
    destination: &Path,
) -> Result<ProjectManifest> {
    validate(files)?;
    if destination.as_os_str().is_empty() || !destination.is_absolute() {
        return Err("project_invalid".into());
    }
    let parent = destination.parent().ok_or("project_invalid")?;
    if !parent.is_dir() {
        return Err("project_invalid".into());
    }
    let manifest = read_manifest(files)?;
    // Atomic reservation: fails if the destination (or an empty decoy created by
    // a racing process) already exists, so it is never clobbered.
    match fs::create_dir(destination) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err("restore_destination_exists".into());
        }
        Err(_) => return Err("store_unavailable".into()),
    }
    if let Err(error) = populate(destination, files, &manifest) {
        // We created this directory; remove only it, never an adversary's.
        let _ = fs::remove_dir_all(destination);
        return Err(error);
    }
    Ok(manifest)
}

fn populate(
    destination: &Path,
    files: &BTreeMap<String, String>,
    manifest: &ProjectManifest,
) -> Result<()> {
    let mut identities = BTreeSet::new();
    for entry in &manifest.files {
        let key = format!("{SNAPSHOT_PREFIX}{}", entry.source);
        let bytes = STANDARD
            .decode(files.get(&key).ok_or("project_invalid")?)
            .map_err(|_| "project_invalid".to_string())?;
        let target = destination.join(&entry.source);
        fs::create_dir_all(target.parent().ok_or("project_invalid")?)
            .map_err(|_| "store_unavailable".to_string())?;
        storage::immutable(&target, &bytes)?;
        set_mode(&target, entry.executable)?;
        let canonical = fs::canonicalize(&target).map_err(|_| "project_invalid".to_string())?;
        if !identities.insert(canonical) {
            return Err("project_invalid".into());
        }
    }
    Ok(())
}

/// Apply the captured POSIX execute bit. On non-POSIX platforms the metadata is
/// retained (recorded in the manifest) but not synthesized onto disk.
fn set_mode(path: &Path, executable: bool) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if executable { 0o755 } else { 0o644 };
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
            .map_err(|_| "store_unavailable".to_string())?;
    }
    #[cfg(not(unix))]
    {
        let _ = (path, executable);
    }
    Ok(())
}

#[cfg(unix)]
fn is_executable(meta: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_meta: &fs::Metadata) -> bool {
    false
}

// ---------------------------------------------------------------------------
// Git working-tree capture (read-only, local commands only).
// ---------------------------------------------------------------------------

fn capture_git(program: &OsStr, root: &Path, selection: Option<&[String]>) -> Capture {
    let mut work = Vec::new();
    let mut excluded = Vec::new();
    let mut blockers = Vec::new();

    let toplevel = git_string(program, root, &["rev-parse", "--show-toplevel"])
        .unwrap_or_else(|| root.to_string_lossy().into_owned());
    let prefix = git_string(program, root, &["rev-parse", "--show-prefix"]).unwrap_or_default();
    let prefix = if prefix.is_empty() || prefix.ends_with('/') {
        prefix
    } else {
        format!("{prefix}/")
    };
    let commit = git_string(program, root, &["rev-parse", "--verify", "-q", "HEAD"]);
    if commit.is_none() {
        blockers.push("project_no_commit".into());
    }
    let tree = commit.as_ref().and_then(|_| {
        git_string(
            program,
            root,
            &["rev-parse", "--verify", "-q", "HEAD^{tree}"],
        )
    });
    let symbolic = git_string(program, root, &["symbolic-ref", "-q", "HEAD"]);
    let branch = symbolic
        .as_ref()
        .and_then(|s| s.strip_prefix("refs/heads/").map(str::to_owned));
    let detached = symbolic.is_none() && commit.is_some();
    let url = git_string(program, root, &["config", "--get", "remote.origin.url"])
        .and_then(|u| sanitize_remote(&u));
    let repo = Some(RepoIdentity {
        name: leaf(&toplevel),
        url,
        branch,
        commit,
        tree,
        detached,
    });

    let (statuses, status_bad) = git_status(program, root);
    let (tracked_entries, tracked_bad) =
        git_bytes(program, root, &["ls-files", "-s", "-z", "--full-name"])
            .map(split_stage)
            .unwrap_or_else(|| {
                blockers.push("project_git_unavailable".into());
                (Vec::new(), false)
            });
    let mut tracked_set: BTreeSet<String> = BTreeSet::new();
    let mut submodules: BTreeSet<String> = BTreeSet::new();
    for (mode, path) in &tracked_entries {
        tracked_set.insert(path.clone());
        if mode == "160000" {
            submodules.insert(path.clone());
        }
    }

    let mut budget = Budget::default();
    let mut incomplete = false;
    if status_bad || tracked_bad {
        // A non-UTF8 path cannot be represented or restored faithfully; report
        // it as a blocker instead of mangling it.
        blockers.push("project_non_utf8_path".into());
        incomplete = true;
    }

    match selection {
        None => {
            for path in &tracked_set {
                let Some(source) = relative_to(path, &prefix) else {
                    continue;
                };
                let xy = statuses.get(path.as_str());
                if let Some(reason) = blocked_component(&source) {
                    excluded.push(ExcludedEntry {
                        path: source,
                        reason: reason.into(),
                    });
                    continue;
                }
                if submodules.contains(path) {
                    excluded.push(ExcludedEntry {
                        path: source,
                        reason: "submodule".into(),
                    });
                    incomplete = true;
                    blockers.push("project_submodule".into());
                    continue;
                }
                if xy.is_some_and(|c| c.contains('D')) {
                    // The file is absent from the working tree, and that absence
                    // *is* the captured state: a restore simply omits it, and the
                    // `deleted` exclusion tells a receiver to drop it from any git
                    // baseline. It never forces a baseline, whatever the
                    // index/worktree combination (staged, unstaged, or both).
                    excluded.push(ExcludedEntry {
                        path: source,
                        reason: "deleted".into(),
                    });
                    continue;
                }
                let state = if xy.is_none() { "tracked" } else { "dirty" };
                if !capture_entry(
                    root,
                    &source,
                    state,
                    true,
                    &mut budget,
                    &mut work,
                    &mut excluded,
                    &mut blockers,
                    &mut incomplete,
                ) {
                    break;
                }
            }
            if !blockers.iter().any(|b| b == "project_limit") {
                for (path, xy) in &statuses {
                    if xy != "??" {
                        continue;
                    }
                    let Some(source) = relative_to(path, &prefix) else {
                        continue;
                    };
                    if !capture_entry(
                        root,
                        &source,
                        "untracked",
                        false,
                        &mut budget,
                        &mut work,
                        &mut excluded,
                        &mut blockers,
                        &mut incomplete,
                    ) {
                        break;
                    }
                }
            }
            // A deletion staged in the index (`git rm`) no longer appears in
            // `ls-files`; record it here. Like any working-tree deletion it is
            // captured state, so it never forces a baseline.
            for (path, xy) in &statuses {
                if !xy.contains('D') || tracked_set.contains(path) {
                    continue;
                }
                let Some(source) = relative_to(path, &prefix) else {
                    continue;
                };
                if let Some(reason) = blocked_component(&source) {
                    excluded.push(ExcludedEntry {
                        path: source,
                        reason: reason.into(),
                    });
                    continue;
                }
                excluded.push(ExcludedEntry {
                    path: source,
                    reason: "deleted".into(),
                });
            }
        }
        Some(list) => {
            if list.is_empty() {
                // Explicit: an empty selection is not "everything".
                blockers.push("project_selected_empty".into());
            }
            for source in list {
                let repo_rel = if prefix.is_empty() {
                    source.clone()
                } else {
                    format!("{prefix}{source}")
                };
                if let Some(reason) = blocked_component(source) {
                    excluded.push(ExcludedEntry {
                        path: source.clone(),
                        reason: reason.into(),
                    });
                    continue;
                }
                if !path_is_portable(source) {
                    excluded.push(ExcludedEntry {
                        path: source.clone(),
                        reason: "unsafe".into(),
                    });
                    continue;
                }
                let xy = statuses.get(&repo_rel);
                if submodules.contains(&repo_rel) {
                    excluded.push(ExcludedEntry {
                        path: source.clone(),
                        reason: "submodule".into(),
                    });
                    incomplete = true;
                    blockers.push("project_submodule".into());
                    continue;
                }
                if xy.is_some_and(|c| c.contains('D')) {
                    // A selected working-tree deletion is recorded, not missing.
                    excluded.push(ExcludedEntry {
                        path: source.clone(),
                        reason: "deleted".into(),
                    });
                    continue;
                }
                let tracked = tracked_set.contains(&repo_rel);
                let state = match (tracked, xy) {
                    (true, Some(code)) if code != "??" => "dirty",
                    (true, _) => "tracked",
                    (false, _) => "untracked",
                };
                if !capture_entry(
                    root,
                    source,
                    state,
                    tracked,
                    &mut budget,
                    &mut work,
                    &mut excluded,
                    &mut blockers,
                    &mut incomplete,
                ) {
                    break;
                }
            }
        }
    }

    if incomplete {
        blockers.push("project_missing_files".into());
    }
    let complete = !blockers
        .iter()
        .any(|b| b == "project_limit" || b == "project_missing_files");
    // A selected subset is never a verified-complete working tree.
    let baseline_required =
        selection.is_some() || (repo.as_ref().is_some_and(|r| r.commit.is_some()) && !complete);
    Capture {
        kind: "git".into(),
        project: leaf(&toplevel),
        repo,
        baseline_required,
        work,
        excluded,
        blockers,
    }
}

// ---------------------------------------------------------------------------
// Plain directory capture.
// ---------------------------------------------------------------------------

fn capture_files(root: &Path, selection: Option<&[String]>) -> Capture {
    let mut capture = Capture {
        kind: "files".into(),
        project: leaf(&root.to_string_lossy()),
        repo: None,
        baseline_required: false,
        work: Vec::new(),
        excluded: Vec::new(),
        blockers: Vec::new(),
    };
    let mut budget = Budget::default();
    match selection {
        Some(list) => {
            if list.is_empty() {
                capture.blockers.push("project_selected_empty".into());
            }
            for source in list {
                if let Some(reason) = blocked_component(source) {
                    capture.excluded.push(ExcludedEntry {
                        path: source.clone(),
                        reason: reason.into(),
                    });
                    continue;
                }
                if !path_is_portable(source) {
                    capture.excluded.push(ExcludedEntry {
                        path: source.clone(),
                        reason: "unsafe".into(),
                    });
                    continue;
                }
                let mut incomplete = false;
                capture_entry(
                    root,
                    source,
                    "file",
                    false,
                    &mut budget,
                    &mut capture.work,
                    &mut capture.excluded,
                    &mut capture.blockers,
                    &mut incomplete,
                );
            }
            capture.baseline_required = true;
        }
        None => {
            let mut visited = 0usize;
            let mut stack = vec![(root.to_path_buf(), String::new(), 0usize)];
            while let Some((dir, rel, depth)) = stack.pop() {
                if depth > MAX_DEPTH {
                    capture.blockers.push("project_limit".into());
                    continue;
                }
                let mut entries = match fs::read_dir(&dir) {
                    Ok(entries) => entries
                        .filter_map(|e| e.ok().map(|e| e.path()))
                        .collect::<Vec<_>>(),
                    Err(_) => continue,
                };
                entries.sort();
                for path in entries {
                    visited += 1;
                    if visited > MAX_ENTRIES {
                        capture.blockers.push("project_limit".into());
                        break;
                    }
                    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                        capture.blockers.push("project_non_utf8_path".into());
                        continue;
                    };
                    let source = if rel.is_empty() {
                        name.to_owned()
                    } else {
                        format!("{rel}/{name}")
                    };
                    if let Some(reason) = blocked_component(&source) {
                        capture.excluded.push(ExcludedEntry {
                            path: source,
                            reason: reason.into(),
                        });
                        continue;
                    }
                    let meta = match fs::symlink_metadata(&path) {
                        Ok(meta) => meta,
                        Err(_) => {
                            capture.excluded.push(ExcludedEntry {
                                path: source,
                                reason: "unreadable".into(),
                            });
                            continue;
                        }
                    };
                    if meta.file_type().is_symlink() {
                        capture.excluded.push(ExcludedEntry {
                            path: source,
                            reason: "symlink".into(),
                        });
                        continue;
                    }
                    if meta.is_dir() {
                        stack.push((path, source, depth + 1));
                        continue;
                    }
                    let mut incomplete = false;
                    if !capture_entry(
                        root,
                        &source,
                        "file",
                        false,
                        &mut budget,
                        &mut capture.work,
                        &mut capture.excluded,
                        &mut capture.blockers,
                        &mut incomplete,
                    ) {
                        break;
                    }
                }
                if capture.blockers.iter().any(|b| b == "project_limit") {
                    break;
                }
            }
        }
    }
    capture
}

/// Capture a single regular file into the accumulating capture. Returns `false`
/// when a hard budget bound was hit and the caller must stop.
///
/// `tracked` marks a HEAD-tracked git file: only those escalate a missing LFS
/// object or a read failure into a `baselineRequired`/blocker state.
#[allow(clippy::too_many_arguments)]
fn capture_entry(
    root: &Path,
    source: &str,
    state: &str,
    tracked: bool,
    budget: &mut Budget,
    work: &mut Vec<Work>,
    excluded: &mut Vec<ExcludedEntry>,
    blockers: &mut Vec<String>,
    incomplete: &mut bool,
) -> bool {
    if let Some(reason) = blocked_component(source) {
        excluded.push(ExcludedEntry {
            path: source.into(),
            reason: reason.into(),
        });
        return true;
    }
    if !allowed(&format!("{SNAPSHOT_PREFIX}{source}")) {
        excluded.push(ExcludedEntry {
            path: source.into(),
            reason: "unsafe".into(),
        });
        return true;
    }
    let full = root.join(source);
    if let Ok(meta) = fs::symlink_metadata(&full) {
        if meta.is_dir() && !meta.file_type().is_symlink() {
            // A tracked directory is a gitlink (submodule); a user-selected
            // directory is simply not a capturable file.
            excluded.push(ExcludedEntry {
                path: source.into(),
                reason: "unsafe".into(),
            });
            *incomplete = true;
            return true;
        }
    }
    match read_bounded(&full, MAX_FILE_BYTES) {
        Ok((bytes, executable)) => {
            if is_lfs_pointer(&bytes) {
                excluded.push(ExcludedEntry {
                    path: source.into(),
                    reason: "lfs".into(),
                });
                if tracked {
                    *incomplete = true;
                    blockers.push("project_missing_lfs".into());
                }
                return true;
            }
            if !budget.admit(bytes.len() as u64) {
                excluded.push(ExcludedEntry {
                    path: source.into(),
                    reason: "bound".into(),
                });
                blockers.push("project_limit".into());
                return false;
            }
            work.push(Work {
                source: source.into(),
                bytes,
                state: state.into(),
                executable,
            });
            true
        }
        Err(reason) => {
            if tracked || reason == "bound" {
                *incomplete = true;
            }
            if reason == "bound" {
                // An oversize tracked file is always incomplete + project_limit.
                blockers.push("project_limit".into());
            }
            excluded.push(ExcludedEntry {
                path: source.into(),
                reason,
            });
            true
        }
    }
}

// ---------------------------------------------------------------------------
// Shared helpers.
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Budget {
    count: usize,
    total: u64,
}

impl Budget {
    fn admit(&mut self, len: u64) -> bool {
        if self.count >= MAX_FILES || self.total.saturating_add(len) > MAX_TOTAL_BYTES {
            return false;
        }
        self.count += 1;
        self.total += len;
        true
    }
}

/// Read a regular file with a size bound, a stable double read (two full reads
/// must agree) and a before/after metadata check. Symlinks, non-files and
/// oversize/unstable files are refused with a reason.
fn read_bounded(path: &Path, max: u64) -> std::result::Result<(Vec<u8>, bool), String> {
    let before = fs::symlink_metadata(path).map_err(|_| "unreadable".to_string())?;
    if before.file_type().is_symlink() {
        return Err("symlink".into());
    }
    if !before.is_file() {
        return Err("unreadable".into());
    }
    if before.len() > max {
        return Err("bound".into());
    }
    let first = storage::read(path, max).map_err(map_read_error)?;
    let second = storage::read(path, max).map_err(map_read_error)?;
    if first != second {
        return Err("unreadable".into());
    }
    let after = fs::symlink_metadata(path).map_err(|_| "unreadable".to_string())?;
    if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
        return Err("unreadable".into());
    }
    Ok((first, is_executable(&before)))
}

fn map_read_error(error: String) -> String {
    if error == "bundle_limit" {
        "bound".to_string()
    } else {
        "unreadable".to_string()
    }
}

/// A git-lfs pointer left in the working tree because the real object was never
/// fetched. The pointer format is a fixed, safe signature.
fn is_lfs_pointer(bytes: &[u8]) -> bool {
    if bytes.len() > 1024 {
        return false;
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let mut lines = text.lines();
    match lines.next().map(str::trim_end) {
        Some("version https://git-lfs.github.com/spec/v1") => {}
        _ => return false,
    }
    let mut oid = false;
    let mut size = false;
    for line in lines.take(4) {
        let line = line.trim_end();
        if let Some(rest) = line.strip_prefix("oid sha256:") {
            oid = rest.len() == 64 && rest.bytes().all(|b| b.is_ascii_hexdigit());
        } else if let Some(rest) = line.strip_prefix("size ") {
            size = !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit());
        }
    }
    oid && size
}

fn relative_to(path: &str, prefix: &str) -> Option<String> {
    let source = if prefix.is_empty() {
        path.to_owned()
    } else {
        path.strip_prefix(prefix)?.to_owned()
    };
    (!source.is_empty()).then_some(source)
}

fn leaf(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty())
        .unwrap_or("project")
        .chars()
        .filter(|c| !c.is_control())
        .collect()
}

/// True when `root` or any ancestor holds a `.git` entry (dir, gitfile or
/// symlink). Used only to decide whether a missing `git` must fail closed.
fn enclosing_git(root: &Path) -> bool {
    let mut dir = Some(root);
    while let Some(current) = dir {
        if fs::symlink_metadata(current.join(".git")).is_ok() {
            return true;
        }
        dir = current.parent();
    }
    false
}

fn null_device() -> &'static str {
    if cfg!(windows) {
        "NUL"
    } else {
        "/dev/null"
    }
}

/// Whether the fixed program can be executed at all. Distinguishes "git is not
/// installed" from "git ran and reported something".
fn git_usable(program: &OsStr) -> bool {
    Command::new(program)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Run a fixed, read-only `git` command under a wall-clock and output bound.
/// Global/system config, hooks, fsmonitor, credential helpers and every
/// dangerous `GIT_*` override are disabled; stdin is closed and stderr discarded.
fn git_bytes(program: &OsStr, root: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let mut command = Command::new(program);
    command
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "core.quotePath=false",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.hooksPath=",
            "-c",
            "credential.helper=",
            "-c",
            "status.relativePaths=false",
            "-c",
            "advice.detachedHead=false",
        ])
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", null_device())
        .env("GIT_CONFIG_SYSTEM", null_device())
        .env("GIT_ATTR_NOSYSTEM", "1");
    for key in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_COMMON_DIR",
        "GIT_CEILING_DIRECTORIES",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_CONFIG",
        "GIT_CONFIG_COUNT",
        "GIT_CONFIG_PARAMETERS",
        "GIT_EXEC_PATH",
        "GIT_NAMESPACE",
        "GIT_PREFIX",
        "GIT_IMPLICIT_WORK_TREE",
        "GIT_QUARANTINE_PATH",
    ] {
        command.env_remove(key);
    }
    run_bounded(command)
}

fn run_bounded(mut command: Command) -> Option<Vec<u8>> {
    let mut child = command.spawn().ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stdout
            .by_ref()
            .take(MAX_GIT_OUTPUT + 1)
            .read_to_end(&mut buffer);
        buffer
    });
    let deadline = Instant::now() + MAX_GIT_WAIT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let buffer = reader.join().ok()?;
                if !status.success() || buffer.len() as u64 > MAX_GIT_OUTPUT {
                    return None;
                }
                return Some(buffer);
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = reader.join();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

fn git_string(program: &OsStr, root: &Path, args: &[&str]) -> Option<String> {
    let bytes = git_bytes(program, root, args)?;
    let text = String::from_utf8(bytes).ok()?;
    Some(text.trim_end_matches(['\n', '\r']).to_owned())
}

/// Parse `git status --porcelain=v1 -z` into repository-root-relative path to
/// two-character status code, plus whether any path was non-UTF8. Rename/copy
/// entries consume the following token.
fn git_status(program: &OsStr, root: &Path) -> (BTreeMap<String, String>, bool) {
    let mut map = BTreeMap::new();
    let Some(bytes) = git_bytes(
        program,
        root,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    ) else {
        return (map, false);
    };
    let mut non_utf8 = false;
    let mut tokens = bytes.split(|b| *b == 0);
    while let Some(token) = tokens.next() {
        if token.len() < 3 || token[2] != b' ' {
            continue;
        }
        let (x, y) = (token[0], token[1]);
        if !valid_code(x) || !valid_code(y) {
            continue;
        }
        let renamed = matches!(x, b'R' | b'C') || matches!(y, b'R' | b'C');
        match std::str::from_utf8(&token[3..]) {
            Ok(path) => {
                let code = String::from_utf8_lossy(&token[..2]).into_owned();
                map.insert(path.to_owned(), code);
            }
            Err(_) => non_utf8 = true,
        }
        if renamed {
            let _ = tokens.next();
        }
    }
    (map, non_utf8)
}

/// Parse `git ls-files -s -z` into `(mode, path)` pairs, plus whether any path
/// was non-UTF8 (which can never be represented or restored faithfully).
fn split_stage(bytes: Vec<u8>) -> (Vec<(String, String)>, bool) {
    let mut pairs = Vec::new();
    let mut non_utf8 = false;
    for token in bytes.split(|b| *b == 0) {
        if token.is_empty() {
            continue;
        }
        let Some(tab) = token.iter().position(|b| *b == b'\t') else {
            if std::str::from_utf8(token).is_err() {
                non_utf8 = true;
            }
            continue;
        };
        let mode = std::str::from_utf8(&token[..tab])
            .ok()
            .and_then(|meta| meta.split(' ').next())
            .map(str::to_owned);
        match std::str::from_utf8(&token[tab + 1..]) {
            Ok(path) => {
                if let Some(mode) = mode {
                    pairs.push((mode, path.to_owned()));
                }
            }
            Err(_) => non_utf8 = true,
        }
    }
    (pairs, non_utf8)
}

fn valid_code(byte: u8) -> bool {
    matches!(
        byte,
        b' ' | b'M' | b'A' | b'D' | b'R' | b'C' | b'U' | b'?' | b'!' | b'T'
    )
}

/// A lowercase hex git object id (SHA-1 or SHA-256 repositories).
fn is_git_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Remove every credential, query and fragment from a remote URL, and refuse an
/// ambiguous schemeless form whose userinfo half may embed a password.
fn sanitize_remote(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains("://") {
        if let Ok(mut url) = url::Url::parse(trimmed) {
            let _ = url.set_password(None);
            let _ = url.set_username("");
            url.set_query(None);
            url.set_fragment(None);
            return Some(url.to_string());
        }
        // Looks scheme-like but unparseable; refuse if it may carry userinfo,
        // otherwise still drop any query/fragment.
        return (!trimmed.contains('@')).then(|| strip_query_fragment(trimmed).to_owned());
    }
    // Schemeless scp-like `[user@]host:path`. Query/fragment never survive, and
    // the userinfo half must carry no colon (an ambiguous embedded password).
    let stripped = strip_query_fragment(trimmed);
    match stripped.split_once('@') {
        Some((userinfo, _)) => (!userinfo.contains(':')).then(|| stripped.to_owned()),
        None => Some(stripped.to_owned()),
    }
}

/// Drop a trailing `?query`/`#fragment` from a non-URL remote form.
fn strip_query_fragment(value: &str) -> &str {
    value.split(['?', '#']).next().unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command as Proc;

    fn git(dir: &Path, args: &[&str]) {
        let status = Proc::new("git")
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

    fn git_out(dir: &Path, args: &[&str]) -> String {
        let output = Proc::new("git")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .expect("git runs");
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    fn init_repo(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        git(dir, &["init", "-q"]);
        git(dir, &["config", "user.email", "fixture@example.test"]);
        git(dir, &["config", "user.name", "Fixture"]);
        git(dir, &["config", "commit.gpgsign", "false"]);
    }

    fn commit(dir: &Path, message: &str) {
        git(dir, &["add", "-A", "-f"]);
        git(dir, &["commit", "-q", "-m", message]);
    }

    fn decode(files: &BTreeMap<String, String>, key: &str) -> Vec<u8> {
        STANDARD.decode(files.get(key).expect(key)).unwrap()
    }

    fn snapshot_key(source: &str) -> String {
        format!("{SNAPSHOT_PREFIX}{source}")
    }

    fn states_of(manifest: &ProjectManifest) -> BTreeMap<String, (String, bool)> {
        manifest
            .files
            .iter()
            .map(|e| (e.source.clone(), (e.state.clone(), e.executable)))
            .collect()
    }

    fn excluded_of(manifest: &ProjectManifest) -> BTreeMap<String, String> {
        manifest
            .excluded
            .iter()
            .map(|e| (e.path.clone(), e.reason.clone()))
            .collect()
    }

    #[test]
    fn allowed_accepts_only_the_reserved_portable_namespace() {
        assert!(allowed("bastet/project-manifest.json"));
        assert!(allowed("bastet/project-files/src/main.rs"));
        assert!(allowed("bastet/project-files/專案/對話.txt"));
        for bad in [
            "",
            "bastet",
            "bastet/project-files/",
            "bastet/project-files/.git/config",
            "bastet/project-files/a/.env",
            "bastet/project-files/a/credentials.json",
            "bastet/project-files/a/id_rsa",
            "bastet/project-files/a/service-account.json",
            "bastet/project-files/a/terraform.tfstate",
            "bastet/project-files/a/prod.tfvars",
            "bastet/project-files/a/kubeconfig",
            "bastet/project-files/a/.docker/config.json",
            "bastet/project-files/a/.kube/config",
            "bastet/project-files/a/apikey.txt",
            "bastet/project-files/a/id_ed25519_sk",
            "bastet/project-files/a/id_ecdsa_sk",
            "bastet/project-files/../escape",
            "bastet/../x",
            "/etc/passwd",
            "bastet/other.json",
            "bastet/project-files/a\\b",
            "bastet/project-files/a:b",
            "bastet/project-files/CON.txt",
            "bastet/project-files/LPT1",
            "bastet/project-files/trailing.",
            "bastet/project-files/trailing ",
        ] {
            assert!(!allowed(bad), "should reject {bad:?}");
        }
    }

    #[test]
    fn git_capture_round_trips_content_and_records_states() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("proj");
        init_repo(&root);
        fs::write(root.join("main.txt"), "line1\n").unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.txt"), "lib\n").unwrap();
        fs::write(root.join("gone.txt"), "gone\n").unwrap();
        fs::write(root.join("keep.txt"), "keep\n").unwrap();
        commit(&root, "init");

        fs::write(root.join("main.txt"), "line1\ndirty\n").unwrap();
        fs::write(root.join("new.txt"), "newfile\n").unwrap();
        fs::remove_file(root.join("gone.txt")).unwrap();
        fs::write(root.join(".env"), "SECRET=xyzzy\n").unwrap();
        git(
            &root,
            &[
                "remote",
                "add",
                "origin",
                "https://user:token-abc123@example.com/repo.git",
            ],
        );
        #[cfg(unix)]
        std::os::unix::fs::symlink(root.join("main.txt"), root.join("link.txt")).unwrap();

        let files = capture(&root).unwrap();
        let manifest = read_manifest(&files).unwrap();
        assert_eq!(manifest.kind, "git");
        let repo = manifest.repo.as_ref().unwrap();
        assert_eq!(repo.name, "proj");
        assert!(repo.branch.as_deref().is_some_and(|b| !b.is_empty()));
        assert!(!repo.detached);
        assert!(is_git_id(repo.commit.as_deref().unwrap()));
        assert!(is_git_id(repo.tree.as_deref().unwrap()));
        assert_eq!(repo.url.as_deref(), Some("https://example.com/repo.git"));
        // The working-tree deletion is captured state, not lost content.
        assert!(
            !manifest.baseline_required,
            "an unstaged worktree deletion is recorded, not missing"
        );
        assert!(
            !manifest
                .blockers
                .contains(&"project_missing_files".to_string()),
            "{:?}",
            manifest.blockers
        );
        assert!(ready(&manifest));

        // No captured byte or identifier may leak the credential.
        for value in files.values() {
            let bytes = STANDARD.decode(value).unwrap();
            assert!(!bytes.windows(13).any(|w| w == b"token-abc123"));
        }

        let states = states_of(&manifest);
        assert_eq!(states.get("main.txt"), Some(&("dirty".to_string(), false)));
        assert_eq!(
            states.get("new.txt"),
            Some(&("untracked".to_string(), false))
        );
        assert_eq!(
            states.get("keep.txt"),
            Some(&("tracked".to_string(), false))
        );
        assert_eq!(
            states.get("src/lib.txt"),
            Some(&("tracked".to_string(), false))
        );
        let excluded = excluded_of(&manifest);
        assert_eq!(
            excluded.get("gone.txt").map(String::as_str),
            Some("deleted")
        );
        assert_eq!(excluded.get(".env").map(String::as_str), Some("secret"));
        #[cfg(unix)]
        assert_eq!(
            excluded.get("link.txt").map(String::as_str),
            Some("symlink")
        );

        // Restore into a brand new isolated directory and prove parity.
        let destination = temp.path().join("copy");
        let restored = restore_to_new_directory(&files, &destination).unwrap();
        assert_eq!(restored, manifest);
        assert_eq!(
            fs::read(destination.join("main.txt")).unwrap(),
            b"line1\ndirty\n"
        );
        assert_eq!(fs::read(destination.join("new.txt")).unwrap(), b"newfile\n");
        assert_eq!(fs::read(destination.join("keep.txt")).unwrap(), b"keep\n");
        assert_eq!(fs::read(destination.join("src/lib.txt")).unwrap(), b"lib\n");
        assert!(!destination.join("gone.txt").exists());
        assert!(!destination.join(".env").exists());
        #[cfg(unix)]
        assert!(!destination.join("link.txt").exists());
        // The source tree is untouched.
        assert_eq!(fs::read(root.join("main.txt")).unwrap(), b"line1\ndirty\n");
    }

    #[test]
    fn clean_repository_needs_no_baseline_and_matches_head() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("clean");
        init_repo(&root);
        fs::write(root.join("a.txt"), "a\n").unwrap();
        commit(&root, "init");
        let files = capture(&root).unwrap();
        let manifest = read_manifest(&files).unwrap();
        assert!(
            !manifest.baseline_required,
            "a fully captured tree needs no baseline"
        );
        assert!(manifest.blockers.is_empty(), "{:?}", manifest.blockers);
        assert!(ready(&manifest));
        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].state, "tracked");
    }

    #[test]
    fn non_git_directory_capture_is_bounded_and_excludes_secrets_and_symlinks() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("plain");
        fs::create_dir_all(root.join("nested/deep")).unwrap();
        fs::write(root.join("readme.txt"), "hello\n").unwrap();
        fs::write(root.join("nested/deep/data.bin"), b"\x00\x01\x02").unwrap();
        fs::write(root.join(".env"), "SECRET=top\n").unwrap();
        fs::write(root.join("signing.pem"), "PRIVATE\n").unwrap();
        // A `.git` directory must never be opened.
        fs::create_dir_all(root.join(".git/objects")).unwrap();
        fs::write(root.join(".git/config"), "[core]\n").unwrap();
        // A symlink to an outside directory must not be followed.
        let outside = temp.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("marker.txt"), "OUTSIDE\n").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, root.join("escape")).unwrap();

        let files = capture(&root).unwrap();
        let manifest = read_manifest(&files).unwrap();
        assert_eq!(manifest.kind, "files");
        assert!(manifest.repo.is_none());
        assert!(!manifest.baseline_required);
        let sources: BTreeSet<_> = manifest.files.iter().map(|e| e.source.as_str()).collect();
        assert!(sources.contains("readme.txt"));
        assert!(sources.contains("nested/deep/data.bin"));
        let excluded = excluded_of(&manifest);
        assert_eq!(excluded.get(".env").map(String::as_str), Some("secret"));
        assert_eq!(
            excluded.get("signing.pem").map(String::as_str),
            Some("secret")
        );
        assert_eq!(excluded.get(".git").map(String::as_str), Some("git"));
        #[cfg(unix)]
        assert_eq!(excluded.get("escape").map(String::as_str), Some("symlink"));
        for value in files.values() {
            let bytes = STANDARD.decode(value).unwrap();
            assert!(!bytes.windows(8).any(|w| w == b"OUTSIDE\n"));
            assert!(!bytes.windows(7).any(|w| w == b"SECRET="));
        }

        let destination = temp.path().join("plain-copy");
        restore_to_new_directory(&files, &destination).unwrap();
        assert_eq!(
            fs::read(destination.join("readme.txt")).unwrap(),
            b"hello\n"
        );
        assert_eq!(
            fs::read(destination.join("nested/deep/data.bin")).unwrap(),
            b"\x00\x01\x02"
        );
        assert!(!destination.join(".env").exists());
        assert!(!destination.join(".git").exists());
    }

    #[test]
    fn capture_is_deterministic() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("det");
        init_repo(&root);
        fs::write(root.join("x.txt"), "x\n").unwrap();
        fs::write(root.join("y.txt"), "y\n").unwrap();
        commit(&root, "init");
        fs::write(root.join("x.txt"), "x changed\n").unwrap();
        fs::write(root.join("z.txt"), "z\n").unwrap();
        assert_eq!(capture(&root).unwrap(), capture(&root).unwrap());
    }

    #[test]
    fn validate_rejects_tampered_maps() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("valid");
        init_repo(&root);
        fs::write(root.join("file.txt"), "content\n").unwrap();
        commit(&root, "init");
        let files = capture(&root).unwrap();
        validate(&files).unwrap();

        // Wrong hash: swap a same-length payload for different bytes.
        let mut wrong = files.clone();
        let key = snapshot_key("file.txt");
        let mut bytes = decode(&files, &key);
        bytes[0] ^= 0xff;
        wrong.insert(key.clone(), STANDARD.encode(&bytes));
        assert_eq!(validate(&wrong).unwrap_err(), "project_hash_mismatch");

        // Wrong length.
        let mut short = files.clone();
        short.insert(key.clone(), STANDARD.encode(b"x"));
        assert_eq!(validate(&short).unwrap_err(), "project_invalid");

        // Missing manifest.
        let mut no_manifest = files.clone();
        no_manifest.remove(MANIFEST_PATH);
        assert_eq!(validate(&no_manifest).unwrap_err(), "project_invalid");

        // Undeclared extra snapshot file.
        let mut extra = files.clone();
        extra.insert(snapshot_key("smuggled.txt"), STANDARD.encode(b"x"));
        assert_eq!(validate(&extra).unwrap_err(), "project_invalid");

        // A secret path can never enter the namespace, even if declared.
        let mut secret = files.clone();
        secret.insert(snapshot_key(".env"), STANDARD.encode(b"y"));
        assert_eq!(validate(&secret).unwrap_err(), "project_invalid");

        // Empty map is not a valid capture.
        assert_eq!(validate(&BTreeMap::new()).unwrap_err(), "project_invalid");
    }

    #[test]
    fn restore_refuses_existing_or_invalid_destinations() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("src");
        init_repo(&root);
        fs::write(root.join("a.txt"), "a\n").unwrap();
        commit(&root, "init");
        let files = capture(&root).unwrap();

        let existing = temp.path().join("exists");
        fs::create_dir_all(&existing).unwrap();
        assert_eq!(
            restore_to_new_directory(&files, &existing).unwrap_err(),
            "restore_destination_exists"
        );

        let missing_parent = temp.path().join("no/such/child");
        assert_eq!(
            restore_to_new_directory(&files, &missing_parent).unwrap_err(),
            "project_invalid"
        );

        assert_eq!(
            restore_to_new_directory(&files, Path::new("relative/dir")).unwrap_err(),
            "project_invalid"
        );

        // A refused restore never writes the receiver's existing workspace.
        assert!(fs::read_dir(&existing).unwrap().next().is_none());
    }

    #[test]
    fn restore_never_replaces_even_an_empty_concurrent_destination() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("src2");
        init_repo(&root);
        fs::write(root.join("a.txt"), "a\n").unwrap();
        commit(&root, "init");
        let files = capture(&root).unwrap();

        // Simulate the lost race: the directory exists (empty) before the
        // reserve. create_dir still refuses it, so nothing is clobbered.
        let empty = temp.path().join("reserved");
        fs::create_dir(&empty).unwrap();
        assert_eq!(
            restore_to_new_directory(&files, &empty).unwrap_err(),
            "restore_destination_exists"
        );
        assert!(fs::read_dir(&empty).unwrap().next().is_none());
    }

    #[test]
    fn validate_rejects_case_colliding_sources() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("case");
        init_repo(&root);
        fs::write(root.join("a.txt"), "a\n").unwrap();
        commit(&root, "init");
        let files = capture(&root).unwrap();
        let mut manifest = read_manifest(&files).unwrap();
        let mut duplicate = manifest.files[0].clone();
        duplicate.source = "A.txt".to_string();
        manifest.files.push(duplicate);
        let json = serde_json::to_vec(&manifest).unwrap();
        let mut tampered = files.clone();
        tampered.insert(MANIFEST_PATH.into(), STANDARD.encode(&json));
        tampered.insert(snapshot_key("A.txt"), STANDARD.encode(b"a\n"));
        // Two sources that differ only by case can never coexist portably.
        assert_eq!(validate(&tampered).unwrap_err(), "project_invalid");
    }

    #[test]
    fn sanitize_remote_strips_all_credentials_query_and_fragment() {
        assert_eq!(
            sanitize_remote("https://user:tok@github.com/a/b.git").as_deref(),
            Some("https://github.com/a/b.git")
        );
        assert_eq!(
            sanitize_remote("https://***@github.com/a/b.git").as_deref(),
            Some("https://github.com/a/b.git")
        );
        // Query and fragment never survive.
        assert_eq!(
            sanitize_remote("https://user:tok@github.com/a/b.git?access_token=SECRET#frag")
                .as_deref(),
            Some("https://github.com/a/b.git")
        );
        assert_eq!(
            sanitize_remote("https://github.com/a/b.git?x=1#y").as_deref(),
            Some("https://github.com/a/b.git")
        );
        // A distinct scheme with a port keeps the host, drops the user.
        assert_eq!(
            sanitize_remote("ssh://git@host:22/a/b.git").as_deref(),
            Some("ssh://host:22/a/b.git")
        );
        assert_eq!(
            sanitize_remote("git@github.com:a/b.git").as_deref(),
            Some("git@github.com:a/b.git")
        );
        // A schemeless form drops a trailing query/fragment too.
        assert_eq!(
            sanitize_remote("git@github.com:a/b.git?ref=secret").as_deref(),
            Some("git@github.com:a/b.git")
        );
        assert_eq!(
            sanitize_remote("https://github.com/a/b.git").as_deref(),
            Some("https://github.com/a/b.git")
        );
        // An ambiguous schemeless password form is refused outright.
        assert_eq!(sanitize_remote("user:pass@host:path"), None);
        assert_eq!(sanitize_remote("  "), None);
    }

    #[test]
    fn validate_rejects_unsanitized_remote_urls() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("url");
        init_repo(&root);
        fs::write(root.join("a.txt"), "a\n").unwrap();
        commit(&root, "init");
        let files = capture(&root).unwrap();

        for malicious in [
            "https://user:token@github.com/a/b.git",
            "https://github.com/a/b.git?access_token=SECRET",
            "https://github.com/a/b.git#frag",
            "user:pass@host:path",
        ] {
            let mut manifest = read_manifest(&files).unwrap();
            manifest.repo.as_mut().unwrap().url = Some(malicious.to_string());
            let json = serde_json::to_vec(&manifest).unwrap();
            let mut tampered = files.clone();
            tampered.insert(MANIFEST_PATH.into(), STANDARD.encode(&json));
            assert_eq!(
                validate(&tampered).unwrap_err(),
                "project_invalid",
                "should reject {malicious:?}"
            );
        }
    }

    #[test]
    fn unrelated_sibling_project_files_are_not_captured() {
        // A subdirectory of a repository captures only its own subtree.
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("mono");
        init_repo(&root);
        fs::create_dir_all(root.join("pkg")).unwrap();
        fs::write(root.join("pkg/inside.txt"), "in\n").unwrap();
        fs::write(root.join("outside.txt"), "out\n").unwrap();
        commit(&root, "init");
        let files = capture(&root.join("pkg")).unwrap();
        let manifest = read_manifest(&files).unwrap();
        let sources: BTreeSet<_> = manifest.files.iter().map(|e| e.source.as_str()).collect();
        assert!(sources.contains("inside.txt"));
        assert!(!sources.contains("outside.txt"));
        let destination: PathBuf = temp.path().join("pkg-copy");
        restore_to_new_directory(&files, &destination).unwrap();
        assert_eq!(fs::read(destination.join("inside.txt")).unwrap(), b"in\n");
        assert!(!destination.join("outside.txt").exists());
    }

    #[test]
    fn unborn_repository_reports_no_commit_without_crashing() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("fresh");
        init_repo(&root);
        fs::write(root.join("wip.txt"), "wip\n").unwrap();
        let files = capture(&root).unwrap();
        let manifest = read_manifest(&files).unwrap();
        assert_eq!(manifest.kind, "git");
        assert!(manifest.repo.as_ref().unwrap().commit.is_none());
        assert!(manifest.blockers.contains(&"project_no_commit".to_string()));
        // The untracked working file is still captured for continuity.
        assert!(manifest.files.iter().any(|e| e.source == "wip.txt"));
    }

    // -----------------------------------------------------------------------
    // Review-fix regressions.
    // -----------------------------------------------------------------------

    #[test]
    fn staged_deletion_is_audited_but_does_not_force_a_baseline() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("staged");
        init_repo(&root);
        fs::write(root.join("a.txt"), "a\n").unwrap();
        fs::write(root.join("b.txt"), "b\n").unwrap();
        commit(&root, "init");
        // An intentional, staged deletion (`git rm`).
        git(&root, &["rm", "-q", "a.txt"]);

        let files = capture(&root).unwrap();
        let manifest = read_manifest(&files).unwrap();
        assert_eq!(
            excluded_of(&manifest).get("a.txt").map(String::as_str),
            Some("deleted")
        );
        assert!(
            !manifest.baseline_required,
            "an intentional staged deletion must not force a baseline"
        );
        assert!(
            !manifest
                .blockers
                .contains(&"project_missing_files".to_string()),
            "{:?}",
            manifest.blockers
        );
        assert!(ready(&manifest));
        assert!(!manifest.files.iter().any(|e| e.source == "a.txt"));
        assert!(manifest.files.iter().any(|e| e.source == "b.txt"));
        // The staged-deletion snapshot still restores the retained files.
        let destination = temp.path().join("staged-copy");
        restore_to_new_directory(&files, &destination).unwrap();
        assert_eq!(fs::read(destination.join("b.txt")).unwrap(), b"b\n");
        assert!(!destination.join("a.txt").exists());
    }

    #[test]
    fn unstaged_worktree_deletion_is_captured_as_an_explicit_deletion() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("lost");
        init_repo(&root);
        fs::write(root.join("c.txt"), "c\n").unwrap();
        fs::write(root.join("keep.txt"), "keep\n").unwrap();
        commit(&root, "init");
        fs::remove_file(root.join("c.txt")).unwrap();

        let files = capture(&root).unwrap();
        let manifest = read_manifest(&files).unwrap();
        assert_eq!(
            excluded_of(&manifest).get("c.txt").map(String::as_str),
            Some("deleted")
        );
        // The deletion is the working state: it is recorded, never "missing".
        assert!(!manifest.baseline_required);
        assert!(!manifest
            .blockers
            .contains(&"project_missing_files".to_string()));
        assert!(ready(&manifest));
        assert!(!manifest.files.iter().any(|e| e.source == "c.txt"));
        assert!(manifest.files.iter().any(|e| e.source == "keep.txt"));

        // A restore reproduces the deleted working tree: `c.txt` stays absent.
        let destination = temp.path().join("lost-copy");
        restore_to_new_directory(&files, &destination).unwrap();
        assert!(!destination.join("c.txt").exists());
        assert_eq!(fs::read(destination.join("keep.txt")).unwrap(), b"keep\n");
    }

    #[test]
    fn oversize_tracked_file_is_incomplete_with_project_limit() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("huge");
        init_repo(&root);
        fs::write(root.join("small.txt"), "ok\n").unwrap();
        commit(&root, "small");
        fs::write(root.join("big.bin"), vec![0u8; MAX_FILE_BYTES as usize + 1]).unwrap();
        commit(&root, "huge");

        let manifest = read_manifest(&capture(&root).unwrap()).unwrap();
        assert!(
            manifest.blockers.contains(&"project_limit".to_string()),
            "{:?}",
            manifest.blockers
        );
        assert!(
            manifest
                .blockers
                .contains(&"project_missing_files".to_string()),
            "{:?}",
            manifest.blockers
        );
        assert!(manifest.baseline_required);
        assert_eq!(
            excluded_of(&manifest).get("big.bin").map(String::as_str),
            Some("bound")
        );
        assert!(!manifest.files.iter().any(|e| e.source == "big.bin"));
        assert!(manifest.files.iter().any(|e| e.source == "small.txt"));
    }

    #[test]
    fn missing_lfs_content_is_a_blocker_not_false_ready() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("lfs");
        init_repo(&root);
        fs::write(
            root.join(".gitattributes"),
            "*.bin filter=lfs diff=lfs merge=lfs -text\n",
        )
        .unwrap();
        let pointer = b"version https://git-lfs.github.com/spec/v1\n\
oid sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393\n\
size 12345\n";
        fs::write(root.join("asset.bin"), pointer).unwrap();
        commit(&root, "lfs pointer");

        let manifest = read_manifest(&capture(&root).unwrap()).unwrap();
        assert!(
            manifest
                .blockers
                .contains(&"project_missing_lfs".to_string()),
            "{:?}",
            manifest.blockers
        );
        assert!(manifest.baseline_required);
        assert!(!ready(&manifest));
        assert_eq!(
            excluded_of(&manifest).get("asset.bin").map(String::as_str),
            Some("lfs")
        );
        // A normal text file that merely mentions the magic is not a pointer.
        assert!(!is_lfs_pointer(b"not a pointer, just text\n"));
        assert!(!is_lfs_pointer(
            b"version https://git-lfs.github.com/spec/v1\n"
        ));
    }

    #[test]
    fn submodule_is_an_explicit_blocker() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("super");
        init_repo(&root);
        fs::write(root.join("a.txt"), "a\n").unwrap();
        commit(&root, "init");
        let head = git_out(&root, &["rev-parse", "HEAD"]);
        // Record a gitlink (submodule) at `vendor/sub` and materialize its dir.
        fs::create_dir_all(root.join("vendor/sub")).unwrap();
        fs::write(root.join("vendor/sub/inner.txt"), "inner\n").unwrap();
        git(
            &root,
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("160000,{head},vendor/sub"),
            ],
        );

        let manifest = read_manifest(&capture(&root).unwrap()).unwrap();
        assert!(
            manifest.blockers.contains(&"project_submodule".to_string()),
            "{:?}",
            manifest.blockers
        );
        assert!(manifest.baseline_required);
        assert!(!ready(&manifest));
        assert_eq!(
            excluded_of(&manifest).get("vendor/sub").map(String::as_str),
            Some("submodule")
        );
    }

    #[test]
    fn git_absent_in_a_repository_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("repo-no-git");
        init_repo(&root);
        fs::write(root.join("a.txt"), "a\n").unwrap();
        // An ignored directory that must never be scanned as plain files.
        fs::create_dir_all(root.join("node_modules")).unwrap();
        fs::write(root.join("node_modules/dep.txt"), "dep\n").unwrap();
        commit(&root, "init");

        assert_eq!(
            capture_with(&root, OsStr::new("bastet-absent-git-program"), None).unwrap_err(),
            "project_git_unavailable"
        );
    }

    #[test]
    fn git_absent_in_a_subdirectory_of_a_repository_fails_closed() {
        // The project is a subdirectory; its `.git` lives in an ancestor. With
        // `git` unavailable the enclosing repository must be detected so the
        // capture fails closed instead of scanning ignored paths as plain files.
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("mono-no-git");
        init_repo(&root);
        fs::create_dir_all(root.join("pkg")).unwrap();
        fs::write(root.join("pkg/inside.txt"), "in\n").unwrap();
        fs::create_dir_all(root.join("node_modules")).unwrap();
        fs::write(root.join("node_modules/dep.txt"), "dep\n").unwrap();
        commit(&root, "init");

        assert_eq!(
            capture_with(
                &root.join("pkg"),
                OsStr::new("bastet-absent-git-program"),
                None
            )
            .unwrap_err(),
            "project_git_unavailable"
        );
    }

    #[test]
    fn git_absent_in_a_plain_directory_still_captures_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("plain-no-git");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("data.txt"), "data\n").unwrap();

        let files = capture_with(&root, OsStr::new("bastet-absent-git-program"), None).unwrap();
        let manifest = read_manifest(&files).unwrap();
        assert_eq!(manifest.kind, "files");
        assert!(manifest.files.iter().any(|e| e.source == "data.txt"));
    }

    #[test]
    fn expanded_secret_exclusions_are_never_captured() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("secrets");
        fs::create_dir_all(root.join(".docker")).unwrap();
        fs::create_dir_all(root.join(".kube")).unwrap();
        let pairs: &[(&str, &str)] = &[
            ("service-account.json", "sa\n"),
            ("terraform.tfstate", "state\n"),
            ("terraform.tfstate.backup", "backup\n"),
            ("prod.tfvars", "var\n"),
            ("kubeconfig", "kube\n"),
            (".docker/config.json", "docker\n"),
            (".kube/config", "kube2\n"),
            ("my-apikey.txt", "key\n"),
            ("api_key.json", "key2\n"),
            ("id_ed25519_sk", "sk\n"),
            ("id_ecdsa_sk", "sk2\n"),
        ];
        for (name, body) in pairs {
            fs::write(root.join(name), body).unwrap();
        }
        fs::write(root.join("ok.txt"), "ok\n").unwrap();

        let manifest = read_manifest(&capture(&root).unwrap()).unwrap();
        let excluded = excluded_of(&manifest);
        // Directory-based credential stores (`.docker/config.json`,
        // `.kube/config`) are excluded wholesale.
        assert_eq!(excluded.get(".docker").map(String::as_str), Some("secret"));
        assert_eq!(excluded.get(".kube").map(String::as_str), Some("secret"));
        for name in [
            "service-account.json",
            "terraform.tfstate",
            "terraform.tfstate.backup",
            "prod.tfvars",
            "kubeconfig",
            "my-apikey.txt",
            "api_key.json",
            "id_ed25519_sk",
            "id_ecdsa_sk",
        ] {
            assert_eq!(
                excluded.get(name).map(String::as_str),
                Some("secret"),
                "{name} must be excluded"
            );
        }
        for (name, _) in pairs {
            assert!(
                !manifest.files.iter().any(|e| e.source == *name),
                "{name} must never be captured"
            );
        }
        assert!(manifest.files.iter().any(|e| e.source == "ok.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn executable_bit_round_trips_on_posix() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("modes");
        init_repo(&root);
        fs::write(root.join("plain.txt"), "plain\n").unwrap();
        fs::write(root.join("run.sh"), "#!/bin/sh\necho run\n").unwrap();
        fs::set_permissions(root.join("run.sh"), fs::Permissions::from_mode(0o755)).unwrap();
        commit(&root, "init");

        let files = capture(&root).unwrap();
        let manifest = read_manifest(&files).unwrap();
        let states = states_of(&manifest);
        assert_eq!(states.get("run.sh"), Some(&("tracked".to_string(), true)));
        assert_eq!(
            states.get("plain.txt"),
            Some(&("tracked".to_string(), false))
        );

        let destination = temp.path().join("modes-copy");
        restore_to_new_directory(&files, &destination).unwrap();
        let run_mode = fs::metadata(destination.join("run.sh"))
            .unwrap()
            .permissions()
            .mode();
        let plain_mode = fs::metadata(destination.join("plain.txt"))
            .unwrap()
            .permissions()
            .mode();
        assert_ne!(run_mode & 0o111, 0, "restored script must be executable");
        assert_eq!(
            plain_mode & 0o111,
            0,
            "restored plain file must not be executable"
        );
    }

    #[test]
    fn non_utf8_paths_are_rejected_not_mangled() {
        // Pure parser path: a non-UTF8 tracked entry must raise the flag and
        // never be emitted as a lossily-mangled string. This is deterministic
        // and runs on every filesystem.
        let mut token = b"100644 0000000000000000000000000000000000000000 0\t".to_vec();
        token.extend_from_slice(b"bad\xff.txt");
        token.push(0);
        let (pairs, non_utf8) = split_stage(token);
        assert!(non_utf8, "a non-UTF8 path must be flagged");
        assert!(pairs.is_empty(), "a non-UTF8 path must not be emitted");

        // End-to-end when the filesystem actually permits a non-UTF8 name
        // (APFS/HFS reject them; ext4/btrfs do not).
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            let bad_name = std::ffi::OsStr::from_bytes(b"bad\xff.txt");
            let temp = tempfile::tempdir().unwrap();
            let plain = temp.path().join("plain-utf8");
            fs::create_dir_all(&plain).unwrap();
            if fs::write(plain.join(bad_name), b"x").is_ok() {
                fs::write(plain.join("ok.txt"), "y\n").unwrap();
                let manifest = read_manifest(&capture(&plain).unwrap()).unwrap();
                assert!(manifest
                    .blockers
                    .contains(&"project_non_utf8_path".to_string()));
                assert!(manifest.files.iter().any(|e| e.source == "ok.txt"));

                let repo = temp.path().join("repo-utf8");
                init_repo(&repo);
                fs::write(repo.join(bad_name), b"x").unwrap();
                fs::write(repo.join("ok.txt"), "y\n").unwrap();
                commit(&repo, "init");
                let manifest = read_manifest(&capture(&repo).unwrap()).unwrap();
                assert!(manifest
                    .blockers
                    .contains(&"project_non_utf8_path".to_string()));
            }
        }
    }

    #[test]
    fn selected_capture_empty_is_explicit() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("sel-empty");
        init_repo(&root);
        fs::write(root.join("a.txt"), "a\n").unwrap();
        commit(&root, "init");

        let files = capture_selected(&root, &[]).unwrap();
        let manifest = read_manifest(&files).unwrap();
        assert!(manifest
            .blockers
            .contains(&"project_selected_empty".to_string()));
        assert!(manifest.files.is_empty());
        assert!(manifest.baseline_required);
        assert!(!ready(&manifest));
        validate(&files).unwrap();
    }

    #[test]
    fn selected_capture_records_only_the_selected_work_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("sel");
        init_repo(&root);
        fs::write(root.join("keep.txt"), "keep\n").unwrap();
        fs::write(root.join("drop.txt"), "drop\n").unwrap();
        fs::write(root.join(".env"), "SECRET\n").unwrap();
        commit(&root, "init");
        fs::write(root.join("keep.txt"), "keep dirty\n").unwrap();

        let selected = vec![
            "keep.txt".to_string(),
            ".env".to_string(),
            "../escape".to_string(),
        ];
        let files = capture_selected(&root, &selected).unwrap();
        let manifest = read_manifest(&files).unwrap();
        let sources: BTreeSet<_> = manifest.files.iter().map(|e| e.source.as_str()).collect();
        assert!(sources.contains("keep.txt"));
        assert!(!sources.contains("drop.txt"));
        assert_eq!(
            states_of(&manifest).get("keep.txt"),
            Some(&("dirty".to_string(), false))
        );
        // A selected secret is still refused, and an escaping path is rejected.
        assert_eq!(
            excluded_of(&manifest).get(".env").map(String::as_str),
            Some("secret")
        );
        assert_eq!(
            excluded_of(&manifest).get("../escape").map(String::as_str),
            Some("unsafe")
        );
        assert!(manifest.baseline_required);
        validate(&files).unwrap();

        // A partially selected snapshot still restores what it holds.
        let destination = temp.path().join("sel-copy");
        restore_to_new_directory(&files, &destination).unwrap();
        assert_eq!(
            fs::read(destination.join("keep.txt")).unwrap(),
            b"keep dirty\n"
        );
        assert!(!destination.join("drop.txt").exists());
    }

    #[test]
    fn selected_capture_works_for_plain_directories() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("sel-plain");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("one.txt"), "one\n").unwrap();
        fs::write(root.join("two.txt"), "two\n").unwrap();
        let files = capture_selected(&root, &["one.txt".to_string()]).unwrap();
        let manifest = read_manifest(&files).unwrap();
        assert_eq!(manifest.kind, "files");
        assert!(manifest.files.iter().any(|e| e.source == "one.txt"));
        assert!(!manifest.files.iter().any(|e| e.source == "two.txt"));
        assert!(manifest.baseline_required);
    }

    #[test]
    fn restore_returns_readiness_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("rdy");
        init_repo(&root);
        fs::write(root.join("a.txt"), "a\n").unwrap();
        commit(&root, "init");
        let files = capture(&root).unwrap();

        let destination = temp.path().join("rdy-copy");
        let returned = restore_to_new_directory(&files, &destination).unwrap();
        assert_eq!(returned, read_manifest(&files).unwrap());
        assert!(ready(&returned));
    }
}
