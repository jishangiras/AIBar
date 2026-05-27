pub mod commands;
pub mod icon;
pub mod polling;
pub mod services;
pub mod state;
pub mod tray;

use commands::{
    delete_api_key, get_aggregate_status, get_all_services, get_settings,
    get_stored_service_ids, open_settings_view, refresh_service, save_api_key, save_settings,
};
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "aibar=info".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::new())
        .setup(|app| {
            // Pure tray app — hide from macOS Dock
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            tray::setup(app.handle())?;

            // AppState::clone is cheap (Arc field bumps only)
            let handle = app.handle().clone();
            let state = app.state::<AppState>().inner().clone();

            tauri::async_runtime::spawn(async move {
                commands::refresh_all_services(&state, &all_services(), &handle).await;
            });

            polling::start(
                app.handle().clone(),
                app.state::<AppState>().inner().clone(),
            );

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_all_services,
            get_settings,
            save_settings,
            save_api_key,
            delete_api_key,
            get_stored_service_ids,
            get_aggregate_status,
            refresh_service,
            open_settings_view,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AIBar");
}

pub fn all_services() -> Vec<Box<dyn services::AiService>> {
    vec![
        Box::new(services::claude::ClaudeService::new()),
        Box::new(services::openai::OpenAiService::new()),
        Box::new(services::gemini::GeminiService::new()),
        Box::new(services::grok::GrokService::new()),
        Box::new(services::perplexity::PerplexityService::new()),
    ]
}
