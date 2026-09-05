//! Invoke the installed AMOS CLI, never copy or overwrite its active database.
use crate::sync::{
    bundle::{hash, Result, MAX_FILE},
    storage,
};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub fn executable() -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "agent-memory.exe"
    } else {
        "agent-memory"
    };
    let mut candidates = vec![];
    if let Some(p) = std::env::var_os("BASTET_AGENT_MEMORY_CLI") {
        candidates.push(PathBuf::from(p));
    }
    if let Some(path) = std::env::var_os("PATH") {
        candidates.extend(
            std::env::split_paths(&path)
                .filter(|p| p.is_absolute())
                .map(|p| p.join(name)),
        );
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".local/bin").join(name));
        candidates.push(
            home.join(if cfg!(windows) {
                ".agent-memory/venv/Scripts"
            } else {
                ".agent-memory/venv/bin"
            })
            .join(name),
        );
        let venv = if cfg!(windows) {
            ".venv/Scripts"
        } else {
            ".venv/bin"
        };
        for parent in [
            home.join(".agent-memory"),
            home.join("Documents/GitHub/agent-memory-os"),
        ] {
            candidates.push(parent.join(venv).join(name));
        }
    }
    candidates
        .into_iter()
        .find(|p| p.is_absolute() && p.is_file())
}
pub fn configured(root: &Path) -> Result<PathBuf> {
    let file = root.join("memory-cli.json");
    if file.exists() {
        let path: PathBuf = serde_json::from_slice(&storage::read(&file, 16384)?)
            .map_err(|_| "memory_cli_missing")?;
        if path.is_absolute() && path.is_file() {
            return Ok(path);
        }
        return Err("memory_cli_missing".into());
    }
    executable().ok_or("memory_cli_missing".into())
}
#[tauri::command]
pub async fn choose_memory_cli(app: tauri::AppHandle) -> Result<bool> {
    use tauri::Manager;
    if app.state::<crate::worker::Worker>().active() {
        return Err("sync_running".into());
    }
    let selected = rfd::AsyncFileDialog::new()
        .set_title("Agent Memory OS CLI")
        .pick_file()
        .await;
    let Some(file) = selected else {
        return Ok(false);
    };
    if app.state::<crate::worker::Worker>().active() {
        return Err("sync_running".into());
    }
    let root = app
        .path()
        .app_config_dir()
        .map_err(|_| "store_unavailable")?;
    storage::replace(
        &root.join("memory-cli.json"),
        &serde_json::to_vec(file.path()).map_err(|_| "memory_cli_missing")?,
    )?;
    Ok(true)
}
fn run(cli: &Path, home: &Path, action: &str, target: &Path) -> Result<()> {
    let output = tempfile::NamedTempFile::new().map_err(|_| "memory_cli_failed")?;
    let mut cmd = Command::new(cli);
    // Source-checkout virtual environments may expose the entry point without an editable install.
    if let Some(repo) = cli.parent().and_then(Path::parent).and_then(Path::parent) {
        let source = repo.join("src");
        if source.join("agent_memory_os/cli.py").is_file() {
            cmd.env("PYTHONPATH", source);
        }
    }
    cmd.arg("--home").arg(home);
    if action == "backup" {
        cmd.arg("backup").arg(target);
    } else {
        cmd.arg("sync").arg(action).arg(target);
    }
    let mut child = cmd
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(output.reopen().map_err(|_| "memory_cli_failed")?)
        .spawn()
        .map_err(|_| "memory_cli_missing")?;
    let deadline = Instant::now() + Duration::from_secs(180);
    let status = loop {
        if let Some(s) = child.try_wait().map_err(|_| "memory_cli_failed")? {
            break s;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("memory_cli_timeout".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    if !status.success() {
        return Err("memory_cli_failed".into());
    }
    if action == "import" {
        let report: serde_json::Value =
            serde_json::from_slice(&storage::read(output.path(), 65536)?)
                .map_err(|_| "memory_import_failed")?;
        if !report["memories_added"].is_u64()
            || report.as_object().is_none_or(|r| {
                r.iter().any(|(k, v)| {
                    (k.contains("rejected") || k.contains("error")) && v.as_u64() != Some(0)
                })
            })
        {
            return Err("memory_import_failed".into());
        }
    }
    Ok(())
}
pub fn export(cli: &Path, home: &Path, staging: &Path) -> Result<String> {
    // Missing stores are errors, not empty exports (the CLI otherwise creates a new DB).
    if !home.join("memories.db").is_file() {
        return Err("memory_store_missing".into());
    }
    let temp = tempfile::tempdir_in(staging).map_err(|_| "store_unavailable")?;
    let path = temp.path().join("export.jsonl");
    run(cli, home, "export", &path)?;
    let text = String::from_utf8(storage::read(&path, MAX_FILE as u64)?)
        .map_err(|_| "memory_bundle_invalid")?;
    crate::memory_adapter::inspect(&text)?;
    Ok(text)
}
pub fn fingerprint(text: &str) -> Result<String> {
    crate::memory_adapter::inspect(text)?;
    // Header transport timestamps are not memory mutations. Preserve the actual export unchanged.
    let mut records = text
        .lines()
        .skip(1)
        .map(|l| {
            serde_json::from_str::<serde_json::Value>(l).and_then(|mut v| {
                // AMOS merge normalizes an unset link activation timestamp from null to empty text.
                if v["kind"] == "link" && v["last_activated_at"] == "" {
                    v["last_activated_at"] = serde_json::Value::Null;
                }
                serde_json::to_string(&v)
            })
        })
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| "memory_bundle_invalid")?;
    records.sort();
    Ok(hash(records.join("\n").as_bytes()))
}
pub fn apply(cli: &Path, home: &Path, staging: &Path, text: &str, id: &str) -> Result<()> {
    crate::memory_adapter::inspect(text)?;
    if !crate::sync::bundle::is_hash(id) || !home.join("memories.db").is_file() {
        return Err("memory_store_missing".into());
    }
    let backup = staging.join("backups");
    storage::directory(&backup)?;
    // A supported SQLite backup precedes every trusted, transactional AMOS merge.
    let dest = backup.join(format!("{id}.db"));
    if !dest.exists() {
        let backup_temp = tempfile::tempdir_in(&backup).map_err(|_| "store_unavailable")?;
        let pending = backup_temp.path().join("backup.db");
        run(cli, home, "backup", &pending)?;
        std::fs::rename(&pending, &dest).map_err(|_| "store_unavailable")?;
    }
    let temp = tempfile::tempdir_in(staging).map_err(|_| "store_unavailable")?;
    let path = temp.path().join("import.jsonl");
    storage::immutable(&path, text.as_bytes())?;
    run(cli, home, "import", &path)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires an installed AMOS CLI; runs only against isolated temporary homes"]
    fn installed_cli_exports_and_merges_without_manual_files() {
        let cli =
            PathBuf::from(std::env::var_os("BASTET_TEST_MEMORY_CLI").expect("test CLI required"));
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("a");
        let b = d.path().join("b");
        let fixture = d.path().join("fixture.jsonl");
        std::fs::write(&fixture, include_str!("../../tests/fixtures/amos-v3.jsonl")).unwrap();
        run(&cli, &a, "import", &fixture).unwrap();
        let empty = d.path().join("empty.jsonl");
        std::fs::write(&empty, "{\"kind\":\"bundle\",\"version\":3}\n").unwrap();
        run(&cli, &b, "import", &empty).unwrap();
        let text = export(&cli, &a, d.path()).unwrap();
        let id = hash(text.as_bytes());
        apply(&cli, &b, d.path(), &text, &id).unwrap();
        assert!(d.path().join("backups").join(format!("{id}.db")).is_file());
        let restored = export(&cli, &b, d.path()).unwrap();
        assert!(restored.contains("Legacy A") && restored.contains("Legacy B"));
        assert_eq!(fingerprint(&text).unwrap(), fingerprint(&restored).unwrap());
        apply(&cli, &b, d.path(), &text, &id).unwrap();
        assert_eq!(
            fingerprint(&restored).unwrap(),
            fingerprint(&export(&cli, &b, d.path()).unwrap()).unwrap()
        );
    }
    #[test]
    fn fingerprint_ignores_transport_timestamp_and_record_order() {
        let a = "{\"kind\":\"bundle\",\"version\":3,\"exported_at\":\"a\"}\n{\"kind\":\"memory\",\"id\":\"a\"}\n{\"kind\":\"memory\",\"id\":\"b\"}\n";
        let b = "{\"kind\":\"bundle\",\"version\":3,\"exported_at\":\"b\"}\n{\"id\":\"b\",\"kind\":\"memory\"}\n{\"id\":\"a\",\"kind\":\"memory\"}\n";
        assert_eq!(fingerprint(a).unwrap(), fingerprint(b).unwrap());
        assert_ne!(
            fingerprint(a).unwrap(),
            fingerprint(&b.replace("\"b\",\"kind\"", "\"c\",\"kind\"")).unwrap()
        );
    }
}
