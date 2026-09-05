//! Narrow GUI commands. Secrets never cross the renderer boundary.
use super::{
    crypto::SpaceKey,
    drive::{Drive, File},
    oauth::{self, ClientConfig},
    vault::NativeStore,
    Result,
};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};
#[derive(Default, Clone)]
pub struct CloudState(pub Arc<Mutex<Option<Drive>>>);
fn config() -> Result<ClientConfig> {
    let config = ClientConfig {
        id: option_env!("BASTET_GOOGLE_CLIENT_ID").unwrap_or("").into(),
        secret: option_env!("BASTET_GOOGLE_CLIENT_SECRET")
            .map(|s| zeroize::Zeroizing::new(s.into())),
    };
    config.validate()?;
    Ok(config)
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    configured: bool,
    connected: bool,
}
#[tauri::command]
pub fn cloud_status(state: State<CloudState>) -> Result<Status> {
    Ok(Status {
        configured: config().is_ok(),
        connected: state.0.try_lock().map_err(|_| "cloud_busy")?.is_some(),
    })
}
#[tauri::command]
pub async fn connect_google(state: State<'_, CloudState>) -> Result<Vec<File>> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = shared.0.try_lock().map_err(|_| "cloud_busy")?;
        let config = config()?;
        let token = match oauth::reconnect(&config, &NativeStore) {
            Ok(t) => t,
            Err(e) if e == "reauth_required" => {
                let pending = oauth::Authorization::begin(&config)?;
                webbrowser::open(pending.url.as_str()).map_err(|_| "browser_open_failed")?;
                pending.finish(&config, &NativeStore)?
            }
            Err(e) => return Err(e),
        };
        let drive = Drive::new(token)?;
        let folders = drive.list_folders()?;
        *guard = Some(drive);
        Ok(folders)
    })
    .await
    .map_err(|_| "cloud_failed".to_string())?
}
#[tauri::command]
pub async fn create_google_folder(
    app: tauri::AppHandle,
    state: State<'_, CloudState>,
    name: String,
) -> Result<File> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let guard = shared.0.try_lock().map_err(|_| "cloud_busy")?;
        let drive = guard.as_ref().ok_or("reauth_required")?;
        let root = app
            .path()
            .app_config_dir()
            .map_err(|_| "store_unavailable")?;
        std::fs::create_dir_all(&root).map_err(|_| "store_unavailable")?;
        super::pending::create(
            &root,
            &crate::sync::bundle::hash(config()?.id.as_bytes()),
            &name,
            drive,
        )
    })
    .await
    .map_err(|_| "cloud_failed".to_string())?
}
#[tauri::command]
pub async fn disconnect_google(state: State<'_, CloudState>) -> Result<()> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = shared.0.try_lock().map_err(|_| "cloud_busy")?;
        *guard = None;
        oauth::forget_login(&config()?, &NativeStore)
    })
    .await
    .map_err(|_| "cloud_failed".to_string())?
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CryptoDiagnostic {
    verified: bool,
    recovery_verified: bool,
    tamper_rejected: bool,
}
#[tauri::command]
pub async fn run_crypto_diagnostic() -> Result<CryptoDiagnostic> {
    tauri::async_runtime::spawn_blocking(|| {
        use crate::sync::bundle::{Bundle, Entry, Snapshot, Stream};
        let bundle = Bundle::new(Snapshot {
            schema: 1,
            space: "isolated-crypto-check".into(),
            device: "synthetic".into(),
            stream: Stream {
                agent: "codex".into(),
                profile: "fixture".into(),
                conversation: "fixture".into(),
            },
            parents: vec![],
            files: std::collections::BTreeMap::from([(
                "sample.txt".into(),
                Entry::new("Synthetic encryption check / 三花貓".into()),
            )]),
        })?;
        let key = SpaceKey::generate()?;
        let bytes = key.seal(&bundle)?;
        let recovered = SpaceKey::recover(&key.recovery_code())?;
        let recovery_verified = recovered.open(&bundle.snapshot.space, &bytes)? == bundle;
        let mut changed: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| "crypto_check_failed")?;
        let encoded = changed["ciphertext"]
            .as_str()
            .ok_or("crypto_check_failed")?;
        let mut altered = encoded.to_owned();
        altered.replace_range(..1, if encoded.starts_with('A') { "B" } else { "A" });
        changed["ciphertext"] = serde_json::Value::String(altered);
        let tamper_rejected = recovered
            .open(
                &bundle.snapshot.space,
                &serde_json::to_vec(&changed).map_err(|_| "crypto_check_failed")?,
            )
            .is_err();
        if !recovery_verified || !tamper_rejected {
            return Err("crypto_check_failed".into());
        }
        Ok(CryptoDiagnostic {
            verified: true,
            recovery_verified,
            tamper_rejected,
        })
    })
    .await
    .map_err(|_| "crypto_check_failed".to_string())?
}
