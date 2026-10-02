//! Mac App Store access to folders chosen by the user. A path in settings is
//! never treated as a sandbox grant. The bookmark file lives in app-private
//! configuration and is deliberately separate from synced settings.
use std::path::{Path, PathBuf};

#[cfg(all(target_os = "macos", feature = "mac-app-store"))]
mod platform {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use objc2::rc::Retained;
    use objc2_foundation::{
        NSData, NSString, NSURLBookmarkCreationOptions, NSURLBookmarkResolutionOptions, NSURL,
    };
    use serde::{Deserialize, Serialize};
    use std::{
        collections::BTreeMap,
        fs,
        io::Write,
        sync::{Mutex, OnceLock},
    };

    const MAX_FILE: u64 = 1_048_576;
    const MAX_GRANTS: usize = 128;
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    #[derive(Default, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Bookmarks {
        version: u32,
        paths: BTreeMap<String, String>,
    }

    fn file(config: &Path) -> PathBuf {
        config.join("sandbox-bookmarks.json")
    }

    fn read(config: &Path) -> Result<Bookmarks, String> {
        let file = file(config);
        let bytes = match fs::read(&file) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Bookmarks {
                    version: 1,
                    ..Default::default()
                })
            }
            Err(_) => return Err("sandbox_grant_unavailable".into()),
        };
        if bytes.len() as u64 > MAX_FILE {
            return Err("sandbox_grant_unavailable".into());
        }
        let data: Bookmarks =
            serde_json::from_slice(&bytes).map_err(|_| "sandbox_grant_unavailable")?;
        if data.version != 1 || data.paths.len() > MAX_GRANTS {
            return Err("sandbox_grant_unavailable".into());
        }
        Ok(data)
    }

    fn write(config: &Path, data: &Bookmarks) -> Result<(), String> {
        fs::create_dir_all(config).map_err(|_| "sandbox_grant_unavailable")?;
        let bytes = serde_json::to_vec(data).map_err(|_| "sandbox_grant_unavailable")?;
        if bytes.len() as u64 > MAX_FILE {
            return Err("sandbox_grant_unavailable".into());
        }
        let mut temp =
            tempfile::NamedTempFile::new_in(config).map_err(|_| "sandbox_grant_unavailable")?;
        temp.write_all(&bytes)
            .map_err(|_| "sandbox_grant_unavailable")?;
        temp.as_file()
            .sync_all()
            .map_err(|_| "sandbox_grant_unavailable")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            temp.as_file()
                .set_permissions(fs::Permissions::from_mode(0o600))
                .map_err(|_| "sandbox_grant_unavailable")?;
        }
        temp.persist(file(config))
            .map_err(|_| "sandbox_grant_unavailable")?;
        Ok(())
    }

    fn canonical(path: &Path) -> Result<PathBuf, String> {
        let path = fs::canonicalize(path).map_err(|_| "sandbox_reauthorize")?;
        if !path.is_dir() {
            return Err("sandbox_reauthorize".into());
        }
        Ok(path)
    }

    pub(super) fn verify_resolved_path(
        stale: bool,
        selected: &Path,
        resolved: &Path,
    ) -> Result<(), String> {
        if stale || canonical(selected)? != canonical(resolved)? {
            return Err("sandbox_reauthorize".into());
        }
        Ok(())
    }

    pub fn grant_selected(config: &Path, picked: &Path) -> Result<PathBuf, String> {
        // Called immediately after NSOpenPanel/rfd returns, while its temporary
        // sandbox extension is still available to this process.
        let path = canonical(picked)?;
        let text = path.to_str().ok_or("sandbox_reauthorize")?;
        let url = NSURL::fileURLWithPath_isDirectory(&NSString::from_str(text), true);
        let data = url
            .bookmarkDataWithOptions_includingResourceValuesForKeys_relativeToURL_error(
                NSURLBookmarkCreationOptions::WithSecurityScope,
                None,
                None,
            )
            .map_err(|_| "sandbox_grant_unavailable")?;
        let lock = LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .map_err(|_| "sandbox_grant_unavailable")?;
        let mut bookmarks = read(config)?;
        if !bookmarks.paths.contains_key(text) && bookmarks.paths.len() >= MAX_GRANTS {
            return Err("sandbox_grant_unavailable".into());
        }
        bookmarks
            .paths
            .insert(text.to_owned(), STANDARD.encode(data.to_vec()));
        let result = write(config, &bookmarks);
        drop(lock);
        result.map(|_| path)
    }

    /// Guard must outlive every read/write made under the selected root.
    pub struct Scope(Retained<NSURL>);

    impl Drop for Scope {
        fn drop(&mut self) {
            // SAFETY: This URL successfully started access in `access`.
            unsafe { self.0.stopAccessingSecurityScopedResource() }
        }
    }

    pub fn access(config: &Path, selected: &Path) -> Result<Scope, String> {
        let key = selected.to_str().ok_or("sandbox_reauthorize")?;
        let encoded = {
            let _lock = LOCK
                .get_or_init(|| Mutex::new(()))
                .lock()
                .map_err(|_| "sandbox_grant_unavailable")?;
            read(config)?
                .paths
                .get(key)
                .cloned()
                .ok_or("sandbox_reauthorize")?
        };
        let bytes = STANDARD
            .decode(encoded)
            .map_err(|_| "sandbox_grant_unavailable")?;
        if bytes.is_empty() || bytes.len() > MAX_FILE as usize {
            return Err("sandbox_grant_unavailable".into());
        }
        let data = NSData::with_bytes(&bytes);
        let mut stale = objc2::runtime::Bool::NO;
        // SAFETY: `stale` is a valid writable pointer for the duration of the call.
        let url = unsafe {
            NSURL::URLByResolvingBookmarkData_options_relativeToURL_bookmarkDataIsStale_error(
                &data,
                NSURLBookmarkResolutionOptions::WithSecurityScope
                    | NSURLBookmarkResolutionOptions::WithoutUI,
                None,
                &mut stale,
            )
        }
        .map_err(|_| "sandbox_reauthorize")?;
        // SAFETY: Access is balanced by Scope::drop, including error returns below.
        if !unsafe { url.startAccessingSecurityScopedResource() } {
            return Err("sandbox_reauthorize".into());
        }
        let scope = Scope(url);
        let resolved = scope.0.path().ok_or("sandbox_reauthorize")?.to_string();
        // A moved folder has a different resolved URL. Require the user to
        // choose it again before using a path saved in settings.
        verify_resolved_path(stale.as_bool(), selected, Path::new(&resolved))?;
        Ok(scope)
    }
}

#[cfg(not(all(target_os = "macos", feature = "mac-app-store")))]
mod platform {
    use super::*;
    pub struct Scope;
    pub fn grant_selected(_: &Path, picked: &Path) -> Result<PathBuf, String> {
        Ok(picked.to_path_buf())
    }
    pub fn access(_: &Path, _: &Path) -> Result<Scope, String> {
        Ok(Scope)
    }
}

pub use platform::{access, grant_selected, Scope};

/// Normalize macOS's public system-directory symlinks for lexical overlap
/// checks when an unrelated root has no grant to resolve its full path.
#[cfg(feature = "mac-app-store")]
pub fn lexical_overlap_path(path: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        for prefix in ["/var", "/tmp", "/etc"] {
            if let Ok(suffix) = path.strip_prefix(prefix) {
                return Path::new("/private")
                    .join(prefix.trim_start_matches('/'))
                    .join(suffix);
            }
        }
    }
    path.to_path_buf()
}

/// Status-only probe for the UI. It never treats a saved path or environment
/// variable as a grant and never keeps a scope open after returning.
pub fn probe_agent_status(config: &Path, agents: &mut [crate::model::Agent]) {
    #[cfg(not(all(target_os = "macos", feature = "mac-app-store")))]
    {
        let _ = (config, agents);
    }
    #[cfg(all(target_os = "macos", feature = "mac-app-store"))]
    {
        for agent in agents {
            agent.detected = access(config, Path::new(&agent.path))
                .is_ok_and(|_scope| Path::new(&agent.path).is_dir());
        }
    }
}

#[cfg(all(target_os = "macos", feature = "mac-app-store"))]
fn effective_source_id(selected: &str) -> &str {
    match selected {
        "claude" => "claude-code",
        "chatgpt-work" => "codex",
        id => id,
    }
}

/// Acquire only provider roots that the sync worker actually reads.
pub fn provider_scopes(
    config: &Path,
    settings: &crate::model::Settings,
) -> Result<Vec<Scope>, String> {
    #[cfg(not(all(target_os = "macos", feature = "mac-app-store")))]
    {
        let _ = (config, settings);
        Ok(Vec::new())
    }
    #[cfg(all(target_os = "macos", feature = "mac-app-store"))]
    {
        let agents = crate::detect(Some(settings));
        let mut paths = std::collections::BTreeSet::<PathBuf>::new();
        for selected in &settings.selected_agents {
            // The worker reads the canonical storage root for these aliases.
            let source = effective_source_id(selected);
            let agent = agents
                .iter()
                .find(|a| a.id == source)
                .ok_or("invalid_source")?;
            paths.insert(PathBuf::from(&agent.path));
        }
        paths.iter().map(|path| access(config, path)).collect()
    }
}

pub fn mapping_scopes(
    config: &Path,
    mappings: &[crate::project_mapping::Mapping],
) -> Result<Vec<Scope>, String> {
    #[cfg(not(all(target_os = "macos", feature = "mac-app-store")))]
    {
        let _ = (config, mappings);
        Ok(Vec::new())
    }
    #[cfg(all(target_os = "macos", feature = "mac-app-store"))]
    {
        let paths = mappings
            .iter()
            .map(|m| PathBuf::from(&m.target))
            .collect::<std::collections::BTreeSet<_>>();
        paths.iter().map(|path| access(config, path)).collect()
    }
}

/// Worker and settings validation use both selected providers and project
/// mappings. The optional legacy local diagnostic folder is not part of sync.
pub fn settings_scopes(
    config: &Path,
    settings: &crate::model::Settings,
) -> Result<Vec<Scope>, String> {
    let mut scopes = provider_scopes(config, settings)?;
    scopes.extend(mapping_scopes(config, &settings.project_mappings)?);
    Ok(scopes)
}

#[cfg(all(test, target_os = "macos", feature = "mac-app-store"))]
mod tests {
    use super::*;
    use std::{collections::HashMap, fs};

    #[test]
    fn a_saved_path_is_not_a_grant() {
        let config = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        let path = source.path().to_string_lossy().into_owned();
        let settings = crate::model::Settings {
            selected_agents: vec!["codex".into()],
            custom_paths: HashMap::from([("codex".into(), path)]),
            ..Default::default()
        };
        assert_eq!(
            settings_scopes(config.path(), &settings).err(),
            Some("sandbox_reauthorize".into())
        );
        fs::write(config.path().join("sandbox-bookmarks.json"), b"broken").unwrap();
        assert_eq!(
            settings_scopes(config.path(), &settings).err(),
            Some("sandbox_grant_unavailable".into())
        );
    }

    #[test]
    fn stale_moved_and_retargeted_folders_require_reselection() {
        let selected = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        assert!(platform::verify_resolved_path(false, selected.path(), selected.path()).is_ok());
        assert_eq!(
            platform::verify_resolved_path(true, selected.path(), selected.path()),
            Err("sandbox_reauthorize".into())
        );
        assert_eq!(
            platform::verify_resolved_path(false, selected.path(), other.path()),
            Err("sandbox_reauthorize".into())
        );
        let alias = selected.path().join("alias");
        std::os::unix::fs::symlink(other.path(), &alias).unwrap();
        assert_eq!(
            platform::verify_resolved_path(false, &alias, selected.path()),
            Err("sandbox_reauthorize".into())
        );
    }

    #[test]
    fn alias_sources_use_the_same_root_as_worker_planning() {
        assert_eq!(effective_source_id("claude"), "claude-code");
        assert_eq!(effective_source_id("chatgpt-work"), "codex");
        assert_eq!(effective_source_id("pi"), "pi");
    }
    #[test]
    fn lexical_overlap_recognizes_macos_system_aliases() {
        assert_eq!(
            lexical_overlap_path(Path::new("/var/folders/example")),
            PathBuf::from("/private/var/folders/example")
        );
        assert_eq!(
            lexical_overlap_path(Path::new("/various")),
            PathBuf::from("/various")
        );
    }
}
