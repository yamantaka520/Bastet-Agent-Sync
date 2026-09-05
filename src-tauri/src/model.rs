use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub const AGENTS: [&str; 8] = [
    "claude",
    "claude-code",
    "codex",
    "agy",
    "grok",
    "pi",
    "agent-memory-os",
    "chatgpt-work",
];
pub const LOCALES: [&str; 5] = ["en", "zh-Hant", "zh-Hans", "ja", "ko"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub schema: u32,
    pub locale: String,
    pub device_name: String,
    pub selected_agents: Vec<String>,
    pub custom_paths: HashMap<String, String>,
    pub folder: String,
    pub direction: String,
    pub schedule: String,
    pub interval_seconds: u32,
    pub close_to_tray: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema: 1,
            locale: "en".into(),
            device_name: String::new(),
            selected_agents: vec![],
            custom_paths: HashMap::new(),
            folder: String::new(),
            direction: "bidirectional".into(),
            schedule: "near-realtime".into(),
            interval_seconds: 60,
            close_to_tray: false,
        }
    }
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    pub id: String,
    pub path: String,
    pub detected: bool,
    pub custom: bool,
}

pub fn discover(
    home: &Path,
    config: &Path,
    overrides: &HashMap<String, String>,
    env: &HashMap<String, String>,
) -> Vec<Agent> {
    let desktop = if cfg!(target_os = "macos") {
        home.join("Library/Application Support/Claude")
    } else {
        config.join("Claude")
    };
    let defaults = [
        desktop,
        home.join(".claude"),
        env.get("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or(home.join(".codex")),
        home.join(".gemini/antigravity-cli"),
        home.join(".grok"),
        env.get("PI_CODING_AGENT_DIR")
            .map(PathBuf::from)
            .unwrap_or(home.join(".pi/agent")),
        env.get("AGENT_MEMORY_HOME")
            .map(PathBuf::from)
            .unwrap_or(home.join(".agent-memory")),
        env.get("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or(home.join(".codex")),
    ];
    AGENTS
        .iter()
        .zip(defaults)
        .map(|(id, default)| {
            let path = overrides.get(*id).map(PathBuf::from).unwrap_or(default);
            Agent {
                id: id.to_string(),
                path: path.to_string_lossy().into_owned(),
                detected: path.is_dir(),
                custom: overrides.contains_key(*id),
            }
        })
        .collect()
}

pub fn validate(settings: &Settings) -> Result<(), String> {
    if settings.schema != 1 || !LOCALES.contains(&settings.locale.as_str()) {
        return Err("invalid_settings".into());
    }
    if settings.device_name.trim().is_empty()
        || settings.device_name.chars().count() > 80
        || settings.device_name.chars().any(char::is_control)
    {
        return Err("invalid_device".into());
    }
    if !(15..=86400).contains(&settings.interval_seconds) {
        return Err("invalid_interval".into());
    }
    if !["bidirectional", "upload", "download"].contains(&settings.direction.as_str())
        || !["near-realtime", "interval", "manual"].contains(&settings.schedule.as_str())
    {
        return Err("invalid_settings".into());
    }
    let mut seen = std::collections::HashSet::new();
    if settings
        .selected_agents
        .iter()
        .any(|a| !AGENTS.contains(&a.as_str()) || !seen.insert(a))
    {
        return Err("invalid_settings".into());
    }
    for (id, path) in &settings.custom_paths {
        if !AGENTS.contains(&id.as_str())
            || !Path::new(path).is_absolute()
            || !Path::new(path).is_dir()
        {
            return Err("invalid_source".into());
        }
    }
    if !settings.folder.is_empty()
        && (!Path::new(&settings.folder).is_absolute() || !Path::new(&settings.folder).is_dir())
    {
        return Err("invalid_folder".into());
    }
    Ok(())
}

pub fn validate_overlap(settings: &Settings, agents: &[Agent]) -> Result<(), String> {
    if settings.folder.is_empty() {
        return Ok(());
    }
    let dest = fs::canonicalize(&settings.folder).map_err(|_| "invalid_folder")?;
    for agent in agents {
        if let Ok(source) = fs::canonicalize(&agent.path) {
            if source.starts_with(&dest) || dest.starts_with(&source) {
                return Err("overlapping_folder".into());
            }
        }
    }
    Ok(())
}

pub fn load(path: &Path) -> Result<Option<Settings>, String> {
    match fs::read(path) {
        Ok(bytes) => {
            if bytes.len() > 1_048_576 {
                return Err("settings_unreadable".into());
            }
            let s: Settings = serde_json::from_slice(&bytes).map_err(|_| "settings_unreadable")?;
            // Missing external folders must not prevent opening settings to repair them.
            if s.schema != 1 || !LOCALES.contains(&s.locale.as_str()) {
                return Err("settings_unreadable".into());
            }
            Ok(Some(s))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err("settings_unreadable".into()),
    }
}

pub fn save(path: &Path, settings: &Settings) -> Result<(), String> {
    validate(settings)?;
    let parent = path.parent().ok_or("save_failed")?;
    fs::create_dir_all(parent).map_err(|_| "save_failed")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
            .map_err(|_| "save_failed")?;
    }
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|_| "save_failed")?;
    let bytes = serde_json::to_vec_pretty(settings).map_err(|_| "save_failed")?;
    file.write_all(&bytes).map_err(|_| "save_failed")?;
    file.as_file().sync_all().map_err(|_| "save_failed")?;
    file.persist(path).map_err(|_| "save_failed")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn settings() -> Settings {
        Settings {
            device_name: "Test device".into(),
            ..Settings::default()
        }
    }
    #[test]
    fn save_reload_replace() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config/settings.json");
        assert!(load(&path).unwrap().is_none());
        let mut s = settings();
        save(&path, &s).unwrap();
        s.locale = "ja".into();
        save(&path, &s).unwrap();
        assert_eq!(load(&path).unwrap().unwrap().locale, "ja");
    }
    #[test]
    fn corrupt_data_is_not_silently_reset() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, b"broken").unwrap();
        assert_eq!(load(&path).unwrap_err(), "settings_unreadable");
        assert_eq!(fs::read(&path).unwrap(), b"broken");
    }
    #[test]
    fn invalid_settings_do_not_replace_saved_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let mut s = settings();
        save(&path, &s).unwrap();
        s.interval_seconds = 0;
        assert!(save(&path, &s).is_err());
        assert_eq!(load(&path).unwrap().unwrap().interval_seconds, 60);
        s.interval_seconds = 60;
        s.selected_agents = vec!["codex".into(), "codex".into()];
        assert!(validate(&s).is_err());
    }
    #[test]
    fn discovery_uses_override_and_does_not_read_files() {
        let dir = tempfile::tempdir().unwrap();
        let custom = dir.path().join("custom");
        fs::create_dir(&custom).unwrap();
        fs::write(custom.join("auth.json"), "never read").unwrap();
        let mut paths = HashMap::new();
        paths.insert("codex".into(), custom.to_string_lossy().into_owned());
        let agents = discover(dir.path(), dir.path(), &paths, &HashMap::new());
        assert_eq!(agents.len(), 8);
        assert!(agents[2].detected && agents[2].custom);
        assert!(!agents[1].detected);
    }
    #[test]
    fn ancestor_and_descendant_destinations_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("agent");
        let child = source.join("nested");
        fs::create_dir_all(&child).unwrap();
        let agents = vec![Agent {
            id: "codex".into(),
            path: source.to_string_lossy().into(),
            detected: true,
            custom: false,
        }];
        let mut s = settings();
        for path in [dir.path(), source.as_path(), child.as_path()] {
            s.folder = path.to_string_lossy().into();
            assert!(validate_overlap(&s, &agents).is_err());
        }
        let other = dir.path().join("other");
        fs::create_dir(&other).unwrap();
        s.folder = other.to_string_lossy().into();
        assert!(validate_overlap(&s, &agents).is_ok());
    }
    #[test]
    fn memory_os_home_is_discovered_and_selection_persists() {
        let d = tempfile::tempdir().unwrap();
        let env = HashMap::from([(
            "AGENT_MEMORY_HOME".into(),
            d.path().to_string_lossy().into_owned(),
        )]);
        assert!(discover(d.path(), d.path(), &HashMap::new(), &env)[6].detected);
        let mut s = settings();
        s.selected_agents = vec!["agent-memory-os".into()];
        save(&d.path().join("settings.json"), &s).unwrap();
        assert_eq!(
            load(&d.path().join("settings.json"))
                .unwrap()
                .unwrap()
                .selected_agents,
            s.selected_agents
        );
    }
    #[test]
    fn environment_profile_is_detected() {
        let dir = tempfile::tempdir().unwrap();
        let mut env = HashMap::new();
        env.insert(
            "PI_CODING_AGENT_DIR".into(),
            dir.path().to_string_lossy().into_owned(),
        );
        assert!(discover(dir.path(), dir.path(), &HashMap::new(), &env)[5].detected);
    }
}
