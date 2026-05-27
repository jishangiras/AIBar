use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime,
};

static POSITIONED: AtomicBool = AtomicBool::new(false);

pub fn setup<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()>
where
    AppHandle<R>: Manager<R>,
{
    let quit = MenuItem::with_id(app, "quit", "Quit AIBar", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "Refresh All", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(app, &[&refresh, &sep, &settings, &sep, &quit])?;

    // Use the app's bundled window icon as the tray icon.
    // For macOS: replace src-tauri/icons/tray-icon.png with a monochrome template PNG
    // (32×32 or 64×64 @2x) for correct dark/light adaptation.
    let icon = app
        .default_window_icon()
        .cloned()
        .expect("no app icon found — run `npm run tauri icon <your-1024px-source.png>` first");

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .icon_as_template(false)
        .tooltip("AIBar – AI Service Monitor")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            "settings" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
                let _ = app.emit("navigate", "settings");
            }
            "refresh" => {
                let app_handle = app.clone();
                let state = app.state::<crate::state::AppState>().inner().clone();
                tauri::async_runtime::spawn(async move {
                    crate::commands::refresh_all_services(
                        &state,
                        &crate::all_services(),
                        &app_handle,
                    )
                    .await;
                });
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                position,
                ..
            } = event
            {
                toggle_main_window(tray.app_handle(), position);
            }
        })
        .build(app)?;

    Ok(())
}

fn toggle_main_window<R: Runtime>(app: &AppHandle<R>, cursor: tauri::PhysicalPosition<f64>)
where
    AppHandle<R>: Manager<R>,
{
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
        } else {
            if !POSITIONED.swap(true, Ordering::Relaxed) {
                position_popup(&win, cursor);
            }
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

fn position_popup<R: Runtime>(win: &tauri::WebviewWindow<R>, cursor: tauri::PhysicalPosition<f64>) {
    let Ok(size) = win.outer_size() else { return };

    let monitor = win
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| win.primary_monitor().ok().flatten());

    let (screen_w, screen_h) = monitor
        .map(|m| (m.size().width as f64, m.size().height as f64))
        .unwrap_or((1440.0, 900.0));

    let win_w = size.width as f64;
    let win_h = size.height as f64;

    let x = (cursor.x - win_w / 2.0).clamp(8.0, screen_w - win_w - 8.0);
    let y = (screen_h - win_h - 48.0).max(8.0);

    let _ = win.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
}

