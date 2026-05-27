use crate::state::AppState;
use rand::Rng;
use std::time::Duration;
use tauri::AppHandle;
use tauri::async_runtime;
use tokio::time;

// AppState is Clone (Arc fields) — move it directly; no Arc wrapper needed here.
pub fn start(app: AppHandle, state: AppState) {
    async_runtime::spawn(async move {
        loop {
            let interval_secs = state.settings.read().await.poll_interval_secs;

            // ±10% jitter prevents thundering herd on provider APIs
            let jitter = rand::thread_rng().gen_range(0..=(interval_secs / 10));
            time::sleep(Duration::from_secs(interval_secs + jitter)).await;

            crate::commands::refresh_all_services(&state, &crate::all_services(), &app).await;
        }
    });
}
