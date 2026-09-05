mod model;
pub mod sync;
use model::{Agent, Settings};
use serde::Serialize;
use std::{collections::HashMap, path::PathBuf, sync::Mutex};
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
    tray_available: bool,
}
fn detect(settings: Option<&Settings>) -> Vec<Agent> {
    let home = dirs::home_dir().unwrap_or_default();
    let config = dirs::config_dir().unwrap_or_else(|| home.clone());
    let mut env = HashMap::new();
    for key in ["CODEX_HOME", "PI_CODING_AGENT_DIR"] {
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
}
#[tauri::command]
fn bootstrap(state: State<AppState>) -> Result<Bootstrap, String> {
    let settings = model::load(&state.path)?;
    let agents = detect(settings.as_ref());
    Ok(Bootstrap {
        settings,
        agents,
        tray_available: state.tray_available,
    })
}
#[tauri::command]
fn scan_agents(settings: Settings) -> Vec<Agent> {
    detect(Some(&settings))
}
#[tauri::command]
async fn choose_folder() -> Option<String> {
    rfd::AsyncFileDialog::new()
        .pick_folder()
        .await
        .map(|f| f.path().to_string_lossy().into_owned())
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
    let mut current = state.settings.lock().map_err(|_| "save_failed")?;
    // Do not overwrite a corrupted configuration through a stale UI.
    model::load(&state.path)?;
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
async fn run_sync_diagnostic() -> Result<sync::diagnostic::Diagnostic, String> {
    tauri::async_runtime::spawn_blocking(sync::diagnostic::run)
        .await
        .map_err(|_| "diagnostic_failed".to_string())?
}

pub fn run() {
    tauri::Builder::default()
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
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            scan_agents,
            choose_folder,
            save_settings,
            run_sync_diagnostic
        ])
        .run(tauri::generate_context!())
        .expect("desktop runtime failed");
}
