use serde::Serialize;
use tauri::{Manager, State};
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preflight {
    pub reasons: Vec<String>,
    pub unsupported_agents: Vec<String>,
}
pub fn evaluate(settings: Option<&crate::model::Settings>, complete: bool) -> Preflight {
    let mut reasons = vec![];
    let selected = settings
        .map(|s| s.selected_agents.clone())
        .unwrap_or_default();
    if settings.is_none() {
        reasons.push("settings".into());
    }
    if selected.is_empty() {
        reasons.push("sources".into());
    }
    if !complete {
        reasons.push("drive".into());
    }
    // Unsupported sources are reported independently from the ready AMOS source.
    if !selected.is_empty() && !selected.iter().any(|s| s == "agent-memory-os") {
        reasons.push("adapters".into());
    }
    Preflight {
        reasons,
        unsupported_agents: selected
            .into_iter()
            .filter(|s| s != "agent-memory-os")
            .collect(),
    }
}
#[tauri::command]
pub async fn sync_preflight(
    app: tauri::AppHandle,
    cloud: State<'_, crate::cloud::desktop::CloudState>,
) -> Result<Preflight, String> {
    let shared = cloud.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = shared.0.try_lock().map_err(|_| "cloud_busy")?;
        let root = app
            .path()
            .app_config_dir()
            .map_err(|_| "store_unavailable")?;
        let settings = crate::model::load(&root.join("settings.json"))?;
        let wizard = crate::cloud::wizard::Transaction::open(&root)?;
        Ok(evaluate(settings.as_ref(), wizard.state.complete))
    })
    .await
    .map_err(|_| "cloud_failed".to_string())?
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completed_drive_never_bypasses_missing_adapter() {
        let s = crate::model::Settings {
            selected_agents: vec!["codex".into()],
            ..Default::default()
        };
        assert_eq!(evaluate(Some(&s), true).reasons, vec!["adapters"]);
        assert_eq!(
            evaluate(None, false).reasons,
            vec!["settings", "sources", "drive"]
        );
    }
}
