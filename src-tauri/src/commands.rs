//! Tauri commands invoked from the webviews.

use crate::hover::{HitRect, HoverState};
use crate::menu::AppMenu;
use crate::pet_window;
use crate::settings::{Settings, SettingsStore};
use tauri::{State, WebviewWindow, Wry};

#[tauri::command]
pub fn get_settings(settings: State<'_, SettingsStore>) -> Settings {
    settings.get()
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
