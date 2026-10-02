mod amos_runtime;
pub mod cloud;
pub mod memory_adapter;
mod model;
mod native_sessions;
mod operations;
mod portable;
mod portable_paths;
mod progress;
mod project_mapping;
mod resources;
mod review;
mod runtime_status;
mod sandbox_access;
pub mod sync;
mod updates;
mod worker;
use model::{Agent, Settings};
use serde::Serialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, State,
};

struct AppState {
    path: PathBuf,
    settings: Mutex<Option<Settings>>,
    tray_available: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Bootstrap {
    settings: Option<Settings>,
    agents: Vec<Agent>,
    memory_sync_available: bool,
    tray_available: bool,
    version: String,
    revision: String,
}
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").into()
}
fn detect(settings: Option<&Settings>) -> Vec<Agent> {
    let home = dirs::home_dir().unwrap_or_default();
    let config = dirs::config_dir().unwrap_or_else(|| home.clone());
    let mut env = HashMap::new();
    for key in [
        "CODEX_HOME",
        "CLAUDE_CONFIG_DIR",
        "GROK_HOME",
        "PI_CODING_AGENT_DIR",
        "AGENT_MEMORY_HOME",
    ] {
        if let Ok(v) = std::env::var(key) {
            if !v.is_empty() {
                env.insert(key.into(), v);
            }
        }
    }
    model::discover(
        &home,
        &config,
        &settings.map(|s| s.custom_paths.clone()).unwrap_or_default(),
        &env,
    )
    .into_iter()
    .filter(|agent| model::agent_available(&agent.id))
    .collect()
}
#[tauri::command]
fn bootstrap(app: tauri::AppHandle, state: State<AppState>) -> Result<Bootstrap, String> {
    let settings = model::load(&state.path)?;
    #[cfg(feature = "mac-app-store")]
    let settings = settings.map(|mut settings| {
        settings
            .selected_agents
            .retain(|id| id != "agent-memory-os");
        settings.custom_paths.remove("agent-memory-os");
        settings
    });
    let mut agents = detect(settings.as_ref());
    if let Ok(config) = app.path().app_config_dir() {
        sandbox_access::probe_agent_status(&config, &mut agents);
    }
    Ok(Bootstrap {
        settings,
        agents,
        memory_sync_available: !cfg!(feature = "mac-app-store"),
        tray_available: state.tray_available,
        version: app_version(),
        revision: env!("BASTET_BUILD_REVISION").into(),
    })
}
#[tauri::command]
fn scan_agents(app: tauri::AppHandle, settings: Settings) -> Vec<Agent> {
    let mut agents = detect(Some(&settings));
    if let Ok(config) = app.path().app_config_dir() {
        sandbox_access::probe_agent_status(&config, &mut agents);
    }
    agents
}
#[tauri::command]
async fn choose_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let selected = rfd::AsyncFileDialog::new().pick_folder().await;
    let Some(folder) = selected else {
        return Ok(None);
    };
    #[cfg(feature = "mac-app-store")]
    let path = sandbox_access::grant_selected(
        &app.path()
            .app_config_dir()
            .map_err(|_| "store_unavailable")?,
        folder.path(),
    )?;
    #[cfg(not(feature = "mac-app-store"))]
    let path = {
        let _ = app;
        folder.path().to_path_buf()
    };
    Ok(Some(path.to_string_lossy().into_owned()))
}
fn menu(app: &tauri::AppHandle, locale: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let (open, quit) = match locale {
        "zh-Hant" => ("開啟視窗", "結束程式"),
        "zh-Hans" => ("打开窗口", "退出程序"),
        "ja" => ("ウィンドウを開く", "終了"),
        "ko" => ("창 열기", "종료"),
        _ => ("Open window", "Quit"),
    };
    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "open", open, true, None::<&str>)?,
            &MenuItem::with_id(app, "quit", quit, true, None::<&str>)?,
        ],
    )
}
#[tauri::command]
fn save_settings(
    app: tauri::AppHandle,
    state: State<AppState>,
    settings: Settings,
) -> Result<(), String> {
    if app.state::<worker::Worker>().active() {
        return Err("sync_running".into());
    }
    let mut current = state.settings.lock().map_err(|_| "save_failed")?;
    // Do not overwrite a corrupted configuration through a stale UI.
    model::load(&state.path)?;
    let config = state.path.parent().ok_or("sandbox_grant_unavailable")?;
    let _scopes = sandbox_access::settings_scopes(config, &settings)?;
    let _legacy_folder_scope = if settings.folder.is_empty() {
        None
    } else {
        Some(sandbox_access::access(config, Path::new(&settings.folder))?)
    };
    model::validate(&settings)?;
    model::validate_overlap(&settings, &detect(Some(&settings)))?;
    if settings.close_to_tray && !state.tray_available {
        return Err("tray_unavailable".into());
    }
    model::save(&state.path, &settings)?;
    if let Some(tray) = app.tray_by_id("bastet") {
        let _ = tray.set_menu(Some(
            menu(&app, &settings.locale).map_err(|_| "tray_unavailable")?,
        ));
    }
    *current = Some(settings);
    Ok(())
}
#[tauri::command]
fn save_locale(
    app: tauri::AppHandle,
    state: State<AppState>,
    locale: String,
) -> Result<(), String> {
    let mut current = state.settings.lock().map_err(|_| "save_failed")?;
    let settings = model::save_locale(&state.path, &locale)?;
    *current = Some(settings);
    if let Some(tray) = app.tray_by_id("bastet") {
        if let Ok(next) = menu(&app, &locale) {
            let _ = tray.set_menu(Some(next));
        }
    }
    Ok(())
}

#[tauri::command]
async fn run_sync_diagnostic() -> Result<sync::diagnostic::Diagnostic, String> {
    tauri::async_runtime::spawn_blocking(sync::diagnostic::run)
        .await
        .map_err(|_| "diagnostic_failed".to_string())?
}

macro_rules! handlers {
    ($($memory:path),* ; $($updater:path),* $(,)?) => {
        tauri::generate_handler![
            bootstrap,
            scan_agents,
            choose_folder,
            save_settings,
            save_locale,
            run_sync_diagnostic,
            cloud::wizard_desktop::wizard_get,
            cloud::wizard_desktop::wizard_navigate,
            cloud::wizard_desktop::wizard_restart,
            cloud::wizard_desktop::wizard_execute,
            cloud::desktop::wizard_cancel_login,
            runtime_status::sync_preflight,
            $($memory,)*
            worker::sync_start,
            worker::sync_status,
            worker::sync_pause,
            worker::sync_pause_for,
            portable::portable_preview,
            portable::portable_list,
            portable::portable_compare,
            portable::portable_restore,
            operations::operations_view,
            operations::storage_usage,
            operations::clear_download_cache,
            operations::cloud_storage_usage,
            worker::sync_now,
            native_sessions::list_received_sessions,
            native_sessions::restore_received_session,
            native_sessions::compare_received_session,
            native_sessions::review_received_session,
            updates::update_status,
            $($updater,)*
            cloud::desktop::run_crypto_diagnostic
        ]
    };
}

pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(not(feature = "mac-app-store"))]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(updates::Updates::default());
    let builder = builder
        .manage(worker::Worker::default())
        .manage(cloud::desktop::CloudState::default())
        .setup(|app| {
            let path = app.path().app_config_dir()?.join("settings.json");
            let settings = model::load(&path).ok().flatten();
            let locale = settings.as_ref().map(|s| s.locale.as_str()).unwrap_or("en");
            let tray_available = TrayIconBuilder::with_id("bastet")
                .icon(app.default_window_icon().expect("bundled icon").clone())
                .tooltip("Bastet Agent Sync")
                .menu(&menu(app.handle(), locale)?)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "open" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(app)
                .is_ok();
            app.manage(AppState {
                path,
                settings: Mutex::new(settings),
                tray_available,
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                if state.tray_available
                    && state
                        .settings
                        .lock()
                        .ok()
                        .and_then(|s| s.as_ref().map(|s| s.close_to_tray))
                        .unwrap_or(false)
                    && window.hide().is_ok()
                {
                    api.prevent_close();
                }
            }
        });
    #[cfg(feature = "mac-app-store")]
    let builder = builder.invoke_handler(handlers![;]);
    #[cfg(not(feature = "mac-app-store"))]
    let builder = builder.invoke_handler(handlers![
        memory_adapter::inspect_memory_export,
        amos_runtime::choose_memory_cli;
        updates::check_update,
        updates::install_update,
        updates::restart_after_update
    ]);
    builder
        .build(tauri::generate_context!())
        .expect("desktop runtime failed")
        .run(|_, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                let _ = cloud::vault::NativeStore::clear_cache();
            }
        });
}
