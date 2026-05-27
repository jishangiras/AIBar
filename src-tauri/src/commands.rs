use crate::services::{AiService, ServiceData};
use crate::state::{AppSettings, AppState};
use keyring::Entry;
use tauri::{Emitter, Manager, Runtime, State};

/// Tell the main window to switch to the settings view.
/// Also shows the window in case it was hidden.
#[tauri::command]
pub async fn open_settings_view(app_handle: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app_handle.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
    let _ = app_handle.emit("navigate", "settings");
    Ok(())
}

const KEYRING_SERVICE: &str = "aibar";

#[tauri::command]
pub async fn get_all_services(state: State<'_, AppState>) -> Result<Vec<ServiceData>, String> {
    let data = state.service_data.read().await;
    let mut services: Vec<ServiceData> = data.values().cloned().collect();
    services.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(services)
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.settings.read().await.clone())
}

#[tauri::command]
pub async fn save_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<(), String> {
    *state.settings.write().await = settings;
    Ok(())
}

#[tauri::command]
pub async fn save_api_key(service_id: String, api_key: String) -> Result<(), String> {
    Entry::new(KEYRING_SERVICE, &service_id)
        .map_err(|e| e.to_string())?
        .set_password(&api_key)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_api_key(service_id: String) -> Result<(), String> {
    Entry::new(KEYRING_SERVICE, &service_id)
        .map_err(|e| e.to_string())?
        .delete_credential()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_stored_service_ids() -> Result<Vec<String>, String> {
    let ids = ["claude", "openai", "gemini", "grok", "perplexity"];
    Ok(ids
        .iter()
        .filter(|&&id| {
            Entry::new(KEYRING_SERVICE, id)
                .ok()
                .and_then(|e| e.get_password().ok())
                .is_some()
        })
        .map(|s| s.to_string())
        .collect())
}

#[tauri::command]
pub async fn get_aggregate_status(state: State<'_, AppState>) -> Result<String, String> {
    use crate::services::ServiceStatus;
    let data = state.service_data.read().await;
    let worst = data.values().fold(ServiceStatus::Green, |acc, svc| {
        match (&acc, &svc.status) {
            (_, ServiceStatus::Unknown) | (ServiceStatus::Unknown, _) => ServiceStatus::Unknown,
            (_, ServiceStatus::Red) | (ServiceStatus::Red, _) => ServiceStatus::Red,
            (_, ServiceStatus::Yellow) => ServiceStatus::Yellow,
            _ => acc,
        }
    });
    Ok(serde_json::to_string(&worst).unwrap())
}

/// Refresh all enabled services concurrently. Called from polling loop and tray "Refresh All".
pub async fn refresh_all_services<R: Runtime>(
    state: &AppState,
    services: &[Box<dyn crate::services::AiService>],
    app_handle: &tauri::AppHandle<R>,
) where
    tauri::AppHandle<R>: Manager<R>,
{
    let settings = state.settings.read().await.clone();
    let mut handles = vec![];

    for svc in services {
        let cfg = settings
            .service_configs
            .get(svc.id())
            .cloned()
            .unwrap_or_default();

        if !cfg.enabled {
            continue;
        }

        let api_key = match Entry::new(KEYRING_SERVICE, svc.id())
            .ok()
            .and_then(|e| e.get_password().ok())
        {
            Some(k) if !k.is_empty() => k,
            _ => continue,
        };

        let thresholds = cfg.thresholds.clone();
        let service_id = svc.id().to_string();
        let svc_name = svc.name().to_string();
        let svc_icon = svc.icon().to_string();
        let dashboard = svc.dashboard_url().to_string();
        let data_store = state.service_data.clone();
        let app_handle = app_handle.clone();

        handles.push(tauri::async_runtime::spawn(async move {
            let fetched = fetch_by_id(
                &service_id,
                &svc_name,
                &svc_icon,
                &dashboard,
                &api_key,
                &thresholds,
            )
            .await;

            data_store.write().await.insert(service_id, fetched.clone());
            let _ = app_handle.emit("service-updated", &fetched);
        }));
    }

    for h in handles {
        let _ = h.await;
    }

    crate::icon::update_tray_icon(app_handle, &*state.service_data.read().await);
}

#[tauri::command]
pub async fn refresh_service(
    service_id: String,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<ServiceData, String> {
    use crate::services::build_error_data;

    let settings = state.settings.read().await.clone();
    let cfg = settings
        .service_configs
        .get(&service_id)
        .cloned()
        .unwrap_or_default();

    // Look up the static service metadata (name, icon, dashboard) by id
    let (svc_name, svc_icon, svc_dashboard) = match service_id.as_str() {
        "claude"     => ("Claude",      "claude",      "https://console.anthropic.com/settings/usage"),
        "openai"     => ("ChatGPT",     "openai",      "https://platform.openai.com/usage"),
        "gemini"     => ("Gemini",      "gemini",      "https://aistudio.google.com/app/apikey"),
        "grok"       => ("Grok",        "grok",        "https://console.x.ai/"),
        "perplexity" => ("Perplexity",  "perplexity",  "https://www.perplexity.ai/settings/api"),
        _ => return Err(format!("Unknown service: {service_id}")),
    };

    // Retrieve stored key — emit an error-state service-updated if missing
    let api_key = match Entry::new(KEYRING_SERVICE, &service_id)
        .ok()
        .and_then(|e| e.get_password().ok())
        .filter(|k| !k.is_empty())
    {
        Some(k) => k,
        None => {
            let err = build_error_data(
                &service_id, svc_name, svc_icon, svc_dashboard,
                "No API key stored — add one in Settings".to_string(),
            );
            state.service_data.write().await.insert(service_id, err.clone());
            let _ = app_handle.emit("service-updated", &err);
            crate::icon::update_tray_icon(&app_handle, &*state.service_data.read().await);
            return Ok(err);
        }
    };

    let data = fetch_by_id(
        &service_id, svc_name, svc_icon, svc_dashboard,
        &api_key, &cfg.thresholds,
    )
    .await;

    state.service_data.write().await.insert(service_id.clone(), data.clone());
    let _ = app_handle.emit("service-updated", &data);
    crate::icon::update_tray_icon(&app_handle, &*state.service_data.read().await);
    Ok(data)
}

/// Construct one ServiceData for a known service id without dyn dispatch overhead.
/// Re-instantiates the service inside the task so it's 'static + Send.
async fn fetch_by_id(
    id: &str,
    name: &str,
    icon: &str,
    dashboard: &str,
    api_key: &str,
    thresholds: &crate::services::StatusThresholds,
) -> ServiceData {
    use crate::services::build_error_data;

    match id {
        "claude" => {
            crate::services::claude::ClaudeService::new()
                .fetch(api_key, thresholds)
                .await
        }
        "openai" => {
            crate::services::openai::OpenAiService::new()
                .fetch(api_key, thresholds)
                .await
        }
        "gemini" => {
            crate::services::gemini::GeminiService::new()
                .fetch(api_key, thresholds)
                .await
        }
        "grok" => {
            crate::services::grok::GrokService::new()
                .fetch(api_key, thresholds)
                .await
        }
        "perplexity" => {
            crate::services::perplexity::PerplexityService::new()
                .fetch(api_key, thresholds)
                .await
        }
        _ => build_error_data(id, name, icon, dashboard, "Unknown service".to_string()),
    }
}
