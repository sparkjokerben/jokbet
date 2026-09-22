//! Tauri commands invoked from the webviews.

use crate::engine::runtime::{Control, RuntimeHandle};
use crate::hover::{HitRect, HoverState};
use crate::menu::AppMenu;
use crate::pet_window;
use crate::settings::{Settings, SettingsStore};
use tauri::{AppHandle, Emitter, Manager, Runtime, State, WebviewWindow, Wry};

/// Validates and saves a settings patch, then applies side effects everywhere.
pub fn apply_patch<R: Runtime>(
    app: &AppHandle<R>,
    patch: &serde_json::Value,
) -> Result<Settings, String> {
    let store = app.state::<SettingsStore>();
    let before = store.get();
    let next = store.patch(patch)?;
    if next.paused != before.paused {
        app.state::<RuntimeHandle>()
            .send(Control::SetPaused(next.paused));
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

#[tauri::command]
pub fn pet_drag_end(window: WebviewWindow) -> Result<(), String> {
    pet_window::save_position(&window).map_err(|e| e.to_string())
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
