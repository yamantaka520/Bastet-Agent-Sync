//! All wizard mutations run in Rust. File dialogs keep client secrets/recovery keys out of IPC.
use super::{
    desktop::CloudState,
    drive::File,
    oauth::{self, ClientConfig},
    vault::{NativeStore, SecretStore},
    wizard::{self, Transaction, Wizard},
    Result,
};
use crate::sync::{bundle::hash, storage};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{Manager, State};
use zeroize::Zeroizing;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub wizard: Wizard,
    pub build_configured: bool,
    pub connected: bool,
    pub folders: Vec<File>,
}
fn root(app: &tauri::AppHandle) -> Result<PathBuf> {
    let p = app
        .path()
        .app_config_dir()
        .map_err(|_| "store_unavailable")?;
    std::fs::create_dir_all(&p).map_err(|_| "store_unavailable")?;
    Ok(p)
}
fn build_config() -> Result<ClientConfig> {
    let c = ClientConfig {
        id: option_env!("BASTET_GOOGLE_CLIENT_ID").unwrap_or("").into(),
        secret: option_env!("BASTET_GOOGLE_CLIENT_SECRET").map(|s| Zeroizing::new(s.into())),
    };
    c.validate()?;
    Ok(c)
}
#[derive(Deserialize)]
struct DesktopJson {
    installed: Installed,
}
#[derive(Deserialize)]
struct Installed {
    client_id: String,
    client_secret: Option<String>,
}
impl Drop for Installed {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.client_secret.zeroize();
    }
}
fn parse_client(bytes: &[u8]) -> Result<ClientConfig> {
    if bytes.len() > 65536 {
        return Err("invalid_oauth_client".into());
    }
    let document: DesktopJson =
        serde_json::from_slice(bytes).map_err(|_| "invalid_oauth_client")?;
    let config = ClientConfig {
        id: document.installed.client_id.clone(),
        secret: document
            .installed
            .client_secret
            .as_ref()
            .map(|s| Zeroizing::new(s.clone())),
    };
    config.validate().map_err(|_| "invalid_oauth_client")?;
    Ok(config)
}
fn config(state: &Wizard) -> Result<ClientConfig> {
    let id = state.client_id.as_ref().ok_or("oauth_not_configured")?;
    let c = match state.client_source.as_deref() {
        Some("build") => build_config()?,
        Some("imported") => {
            let value = NativeStore
                .read(&format!("wizard-client:{}", hash(id.as_bytes())))?
                .ok_or("oauth_not_configured")?;
            parse_client(value.as_bytes())?
        }
        _ => return Err("oauth_not_configured".into()),
    };
    if c.id != *id {
        return Err("oauth_not_configured".into());
    }
    Ok(c)
}
fn view(wizard: Wizard, connected: bool, folders: Vec<File>) -> View {
    View {
        wizard,
        build_configured: build_config().is_ok(),
        connected,
        folders,
    }
}
#[tauri::command]
pub async fn wizard_get(app: tauri::AppHandle, state: State<'_, CloudState>) -> Result<View> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let guard = shared.0.try_lock().map_err(|_| "cloud_busy")?;
        let t = Transaction::open(&root(&app)?)?;
        Ok(view(
            t.save()?,
            guard.as_ref().is_some_and(|d| d.is_connected()),
            vec![],
        ))
    })
    .await
    .map_err(|_| "cloud_failed".to_string())?
}
#[tauri::command]
pub async fn wizard_navigate(
    app: tauri::AppHandle,
    state: State<'_, CloudState>,
    mode: String,
    page: usize,
) -> Result<View> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let guard = shared.0.try_lock().map_err(|_| "cloud_busy")?;
        let mut t = Transaction::open(&root(&app)?)?;
        Ok(view(
            t.navigate(&mode, page)?,
            guard.as_ref().is_some_and(|d| d.is_connected()),
            vec![],
        ))
    })
    .await
    .map_err(|_| "cloud_failed".to_string())?
}
#[tauri::command]
pub async fn wizard_restart(app: tauri::AppHandle, state: State<'_, CloudState>) -> Result<View> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = shared.0.try_lock().map_err(|_| "cloud_busy")?;
        let wizard = wizard::restart(&root(&app)?)?;
        *guard = None;
        Ok(view(wizard, false, vec![]))
    })
    .await
    .map_err(|_| "cloud_failed".to_string())?
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    ForgetLogin,
    OpenHelp,
    UseBuild,
    ImportClient,
    Connect,
    ListFolders,
    CreateFolder,
    SelectFolder,
    PrepareKey,
    ExportRecovery,
    PublishProof,
    ImportRecovery,
    Finish,
}
fn title<'a>(locale: &str, values: [&'a str; 5]) -> &'a str {
    values[match locale {
        "zh-Hant" => 0,
        "zh-Hans" => 1,
        "ja" => 3,
        "ko" => 4,
        _ => 2,
    }]
}
fn parse_folder(input: &str) -> Result<String> {
    let value = input.trim();
    if crate::sync::bundle::token(value) {
        return Ok(value.into());
    }
    let url = url::Url::parse(value).map_err(|_| "invalid_drive_id")?;
    if url.scheme() != "https" || url.host_str() != Some("drive.google.com") {
        return Err("invalid_drive_id".into());
    }
    let parts: Vec<_> = url.path_segments().ok_or("invalid_drive_id")?.collect();
    let i = parts
        .iter()
        .position(|p| *p == "folders")
        .ok_or("invalid_drive_id")?;
    let id = parts
        .get(i + 1)
        .filter(|s| crate::sync::bundle::token(s))
        .ok_or("invalid_drive_id")?;
    if parts.len() != i + 2 {
        return Err("invalid_drive_id".into());
    }
    Ok((*id).into())
}
#[tauri::command]
pub async fn wizard_execute(
    app: tauri::AppHandle,
    state: State<'_, CloudState>,
    action: Action,
    input: String,
    locale: String,
) -> Result<View> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        if input.len() > 2048 {
            return Err("invalid_setup_input".into());
        }
        let mut guard = shared.0.try_lock().map_err(|_| "cloud_busy")?;
        let mut t = Transaction::open(&root(&app)?)?;
        let mut folders = vec![];
        match action {
            Action::ForgetLogin => {
                oauth::forget_login(&config(&t.state)?, &NativeStore)?;
                *guard = None;
            }
            Action::OpenHelp => {
                webbrowser::open(
                    "https://developers.google.com/workspace/drive/api/quickstart/python",
                )
                .map_err(|_| "browser_open_failed")?;
            }
            Action::UseBuild => {
                let c = build_config()?;
                t.client(c.id, "build")?;
                *guard = None;
            }
            Action::ImportClient => {
                let chosen = rfd::FileDialog::new()
                    .set_title(title(
                        &locale,
                        [
                            "匯入 Google 桌面 OAuth 設定",
                            "导入 Google 桌面 OAuth 设置",
                            "Import Google Desktop OAuth configuration",
                            "Google デスクトップ OAuth 設定をインポート",
                            "Google 데스크톱 OAuth 설정 가져오기",
                        ],
                    ))
                    .add_filter("JSON", &["json"])
                    .pick_file();
                if let Some(path) = chosen {
                    let bytes = Zeroizing::new(storage::read(&path, 65536)?);
                    let c = parse_client(&bytes)?;
                    let raw = std::str::from_utf8(&bytes).map_err(|_| "invalid_oauth_client")?;
                    NativeStore.write(&format!("wizard-client:{}", hash(c.id.as_bytes())), raw)?;
                    t.client(c.id, "imported")?;
                    *guard = None;
                }
            }
            Action::Connect => {
                let c = config(&t.state)?;
                *guard = None;
                let token = match oauth::reconnect(&c, &NativeStore) {
                    Ok(token) => token,
                    Err(e) if e == "reauth_required" => {
                        let pending = oauth::Authorization::begin(&c)?;
                        webbrowser::open(pending.url.as_str())
                            .map_err(|_| "browser_open_failed")?;
                        pending.finish(&c, &NativeStore)?
                    }
                    Err(e) => return Err(e),
                };
                let drive = super::drive::Drive::new(token)?;
                folders = drive.list_folders()?;
                *guard = Some(drive);
                t.authorized()?;
            }
            Action::ExportRecovery => {
                let bytes = t.recovery(&NativeStore)?;
                let chosen = rfd::FileDialog::new()
                    .set_title(title(
                        &locale,
                        [
                            "保存恢復檔：請存於共用 Drive 資料夾之外",
                            "保存恢复文件：请存于共享 Drive 文件夹之外",
                            "Save recovery kit outside the shared Drive folder",
                            "共有 Drive フォルダーの外に復元ファイルを保存",
                            "공유 Drive 폴더 외부에 복구 파일 저장",
                        ],
                    ))
                    .set_file_name("bastet-recovery.json")
                    .add_filter("JSON", &["json"])
                    .save_file();
                if let Some(path) = chosen {
                    storage::immutable(&path, &bytes)?;
                    let verified = Zeroizing::new(storage::read(&path, 16384)?);
                    wizard::RecoveryKit::parse(&verified)?;
                    if *verified != *bytes {
                        return Err("recovery_export_failed".into());
                    }
                    t.recovery_exported()?;
                }
            }
            other => {
                if !t.state.authorized {
                    return Err("wizard_step_required".into());
                }
                if !guard.as_ref().is_some_and(|d| d.is_connected()) {
                    let token = oauth::reconnect(&config(&t.state)?, &NativeStore)?;
                    *guard = Some(super::drive::Drive::new(token)?);
                }
                let drive = guard.as_ref().ok_or("reauth_required")?;
                match other {
                    Action::ListFolders => {
                        folders = drive.list_folders()?;
                    }
                    Action::CreateFolder => {
                        if t.state.binding.is_some() {
                            return Err("wizard_restart_required".into());
                        }
                        let f = super::pending::create(
                            &t.operation_root()?,
                            &hash(config(&t.state)?.id.as_bytes()),
                            &input,
                            drive,
                        )?;
                        t.folder(f.id, f.name)?;
                    }
                    Action::SelectFolder => {
                        let f = drive.verify_folder(&parse_folder(&input)?)?;
                        t.folder(f.id, f.name)?;
                    }
                    Action::PrepareKey => {
                        drive.verify_folder(
                            t.state.folder_id.as_ref().ok_or("wizard_step_required")?,
                        )?;
                        t.prepare(drive, &NativeStore)?;
                    }
                    Action::PublishProof => {
                        t.publish(drive, &NativeStore)?;
                    }
                    Action::ImportRecovery => {
                        let chosen = rfd::FileDialog::new()
                            .set_title(title(
                                &locale,
                                [
                                    "匯入另一台電腦的恢復檔",
                                    "导入另一台电脑的恢复文件",
                                    "Import recovery kit from another computer",
                                    "別のコンピューターの復元ファイルをインポート",
                                    "다른 컴퓨터의 복구 파일 가져오기",
                                ],
                            ))
                            .add_filter("JSON", &["json"])
                            .pick_file();
                        if let Some(path) = chosen {
                            let bytes = Zeroizing::new(storage::read(&path, 16384)?);
                            t.import(&bytes, drive, &NativeStore)?;
                        }
                    }
                    Action::Finish => {
                        drive.verify_folder(
                            t.state.folder_id.as_ref().ok_or("wizard_step_required")?,
                        )?;
                        t.finish(drive, &NativeStore)?;
                    }
                    _ => return Err("invalid_setup_input".into()),
                }
            }
        }
        Ok(view(
            t.save()?,
            guard.as_ref().is_some_and(|d| d.is_connected()),
            folders,
        ))
    })
    .await
    .map_err(|_| "cloud_failed".to_string())?
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manual_folder_input_only_accepts_id_or_google_folder_link() {
        assert_eq!(parse_folder(" folder-id ").unwrap(), "folder-id");
        assert_eq!(
            parse_folder("https://drive.google.com/drive/u/0/folders/folder-id?usp=sharing")
                .unwrap(),
            "folder-id"
        );
        for s in [
            "https://evil.example/folders/x",
            "https://drive.google.com.evil/folders/x",
            "http://drive.google.com/folders/x",
            "../x",
            "https://drive.google.com/folders/x/extra",
        ] {
            assert!(parse_folder(s).is_err());
        }
    }
    #[test]
    fn imported_client_must_be_a_valid_desktop_document() {
        assert!(parse_client(br#"{"web":{"client_id":"x.apps.googleusercontent.com"}}"#).is_err());
        let c=parse_client(br#"{"installed":{"client_id":"x.apps.googleusercontent.com","client_secret":"fixture-only"}}"#).unwrap();
        assert_eq!(c.id, "x.apps.googleusercontent.com");
    }
}
