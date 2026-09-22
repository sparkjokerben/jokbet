//! Tauri commands invoked from the webviews.

use crate::engine::runtime::{Control, RuntimeHandle, Stats, Status};
use crate::hover::{HitRect, HoverState};
use crate::menu::AppMenu;
use crate::panels::Panel;
use crate::pet_window;
use crate::settings::{Settings, SettingsStore};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, Runtime, State, WebviewWindow, Wry};

/// Validates and saves a settings patch, then applies side effects everywhere.
pub fn apply_patch<R: Runtime>(
    app: &AppHandle<R>,
    patch: &serde_json::Value,
) -> Result<Settings, String> {
    let store = app.state::<SettingsStore>();
    let before = store.get();
    let next = store.patch(patch)?;
    app.state::<RuntimeHandle>()
        .send(Control::Settings(Box::new(next.clone())));
    if next.paused != before.paused {
        app.state::<AppMenu<R>>().sync_paused(next.paused);
    }
    if next.pet_size != before.pet_size {
        if let Some(window) = app.get_webview_window(pet_window::PET_LABEL) {
            pet_window::apply_size(&window, before.pet_size, next.pet_size)
                .map_err(|e| e.to_string())?;
        }
    }
    let _ = app.emit("settings://changed", &next);
    Ok(next)
}

#[tauri::command]
pub fn get_settings(settings: State<'_, SettingsStore>) -> Settings {
    settings.get()
}

#[tauri::command]
pub fn update_settings(app: AppHandle, patch: serde_json::Value) -> Result<Settings, String> {
    apply_patch(&app, &patch)
}

#[tauri::command]
pub fn set_hit_rect(rect: HitRect, hover: State<'_, HoverState>) {
    *hover.hit_rect.lock().unwrap() = Some(rect);
}

/// The pet starts a native window drag; the hover thread reports its end.
#[tauri::command]
pub fn pet_drag_start(hover: State<'_, HoverState>) {
    hover
        .dragging
        .store(true, std::sync::atomic::Ordering::Release);
}

#[tauri::command]
pub fn show_context_menu(
    window: WebviewWindow,
    menu: State<'_, AppMenu<Wry>>,
) -> Result<(), String> {
    window.popup_menu(&menu.menu).map_err(|e| e.to_string())
}

/// The pet has registered its listeners; resend status and counters.
#[tauri::command]
pub fn pet_ready(runtime: State<'_, RuntimeHandle>) {
    runtime.send(Control::PetReady);
}

/// Runs a blocking runtime query off the main thread.
async fn off_main<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_stats(days: u32, runtime: State<'_, RuntimeHandle>) -> Result<Stats, String> {
    let rt = runtime.inner().clone();
    off_main(move || rt.stats(days)).await
}

#[tauri::command]
pub async fn export_csv(dir: PathBuf, runtime: State<'_, RuntimeHandle>) -> Result<(), String> {
    let rt = runtime.inner().clone();
    off_main(move || rt.export_csv(dir)).await
}

#[tauri::command]
pub async fn clear_data(runtime: State<'_, RuntimeHandle>) -> Result<(), String> {
    let rt = runtime.inner().clone();
    off_main(move || rt.clear()).await
}

#[tauri::command]
pub fn open_panel(app: AppHandle, view: String) -> Result<(), String> {
    let panel = match view.as_str() {
        "stats" => Panel::Stats,
        "settings" => Panel::Settings,
        "onboarding" => Panel::Onboarding,
        other => return Err(format!("unknown panel {other}")),
    };
    crate::menu::open_panel(&app, panel);
    Ok(())
}

#[tauri::command]
pub async fn get_status(runtime: State<'_, RuntimeHandle>) -> Result<Status, String> {
    let rt = runtime.inner().clone();
    off_main(move || rt.status()).await
}

/// Registers the app for Input Monitoring and opens that settings pane.
#[tauri::command]
pub fn open_input_monitoring_settings() -> Result<(), String> {
    // The system prompt only appears once; requesting again still adds the
    // app to the list (switched off) so the user can find it.
    crate::input::request_permission();
    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Some macOS versions only honour a new Input Monitoring grant after a relaunch.
#[tauri::command]
pub fn restart_app(app: AppHandle) {
    app.restart();
}
