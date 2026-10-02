use serde::Serialize;
#[cfg(not(feature = "mac-app-store"))]
use std::sync::Mutex;
#[cfg(not(feature = "mac-app-store"))]
use tauri::{Manager, State};
#[cfg(not(feature = "mac-app-store"))]
use tauri_plugin_updater::{Update, UpdaterExt};
#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub phase: String,
    pub version: Option<String>,
    pub downloaded: u64,
    pub total: Option<u64>,
}
#[derive(Default)]
#[cfg(not(feature = "mac-app-store"))]
pub struct Updates(Mutex<(Status, Option<Update>)>);
#[cfg(not(feature = "mac-app-store"))]
fn busy(phase: &str) -> bool {
    matches!(phase, "checking" | "installing")
}
#[cfg(not(feature = "mac-app-store"))]
fn allowed(url: &url::Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("github.com")
        && url
            .path()
            .starts_with("/yamantaka520/Bastet-Agent-Sync/releases/download/")
        && url.username().is_empty()
        && url.password().is_none()
}
#[cfg(feature = "mac-app-store")]
#[tauri::command]
pub fn update_status() -> Status {
    Status {
        phase: "app_store".into(),
        ..Status::default()
    }
}
#[cfg(not(feature = "mac-app-store"))]
#[tauri::command]
pub fn update_status(state: State<Updates>) -> Result<Status, String> {
    Ok(state.0.lock().map_err(|_| "update_busy")?.0.clone())
}
#[cfg(not(feature = "mac-app-store"))]
#[tauri::command]
pub async fn check_update(
    app: tauri::AppHandle,
    state: State<'_, Updates>,
) -> Result<Status, String> {
    {
        let mut s = state.0.lock().map_err(|_| "update_busy")?;
        if busy(&s.0.phase) || s.0.phase == "installed" {
            return Err("update_busy".into());
        }
        *s = (
            Status {
                phase: "checking".into(),
                ..Status::default()
            },
            None,
        );
    }
    let result = async {
        app.updater_builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?
            .check()
            .await
    }
    .await;
    let mut s = state.0.lock().map_err(|_| "update_busy")?;
    s.0.phase = match result {
        Ok(Some(mut update)) if allowed(&update.download_url) => {
            update.timeout = Some(std::time::Duration::from_secs(300));
            s.0.version = Some(update.version.clone());
            s.1 = Some(update);
            "available"
        }
        Ok(Some(_)) => "failed",
        Ok(None) => "current",
        Err(tauri_plugin_updater::Error::ReleaseNotFound) => "unpublished",
        Err(tauri_plugin_updater::Error::TargetNotFound(_))
        | Err(tauri_plugin_updater::Error::TargetsNotFound(_)) => "unsupported",
        Err(_) => "failed",
    }
    .into();
    Ok(s.0.clone())
}
#[cfg(not(feature = "mac-app-store"))]
#[tauri::command]
pub async fn install_update(
    app: tauri::AppHandle,
    state: State<'_, Updates>,
) -> Result<Status, String> {
    if app.state::<crate::worker::Worker>().active() {
        return Err("sync_running".into());
    }
    let update = {
        let mut s = state.0.lock().map_err(|_| "update_busy")?;
        if s.0.phase != "available" {
            return Err("update_not_ready".into());
        }
        let u = s.1.take().ok_or("update_not_ready")?;
        s.0.phase = "installing".into();
        u
    };
    let progress_app = app.clone();
    let result = update
        .download_and_install(
            move |bytes, total| {
                if let Ok(mut s) = progress_app.state::<Updates>().0.lock() {
                    s.0.downloaded += bytes as u64;
                    s.0.total = total;
                }
            },
            || {},
        )
        .await;
    let mut s = state.0.lock().map_err(|_| "update_busy")?;
    s.0.phase = if result.is_ok() {
        "installed"
    } else {
        "failed"
    }
    .into();
    Ok(s.0.clone())
}
#[cfg(not(feature = "mac-app-store"))]
#[tauri::command]
pub fn restart_after_update(app: tauri::AppHandle, state: State<Updates>) -> Result<(), String> {
    if app.state::<crate::worker::Worker>().active() {
        return Err("sync_running".into());
    }
    if state.0.lock().map_err(|_| "update_busy")?.0.phase != "installed" {
        return Err("update_not_ready".into());
    }
    app.restart()
}
#[cfg(all(test, feature = "mac-app-store"))]
mod store_tests {
    #[test]
    fn store_channel_reports_store_updates_without_a_download_state() {
        let status = super::update_status();
        assert_eq!(status.phase, "app_store");
        assert!(status.version.is_none());
        assert_eq!(status.downloaded, 0);
    }
}
#[cfg(not(feature = "mac-app-store"))]
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn update_origin_is_restricted_to_this_repository() {
        for url in [
            "http://github.com/yamantaka520/Bastet-Agent-Sync/releases/download/v1/a",
            "https://evil.example/a",
            "https://github.com/other/repo/releases/download/v1/a",
        ] {
            assert!(!allowed(&url::Url::parse(url).unwrap()));
        }
        assert!(allowed(
            &url::Url::parse(
                "https://github.com/yamantaka520/Bastet-Agent-Sync/releases/download/v1/a"
            )
            .unwrap()
        ));
    }
}
