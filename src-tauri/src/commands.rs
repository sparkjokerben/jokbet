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
    // Shortcuts can be refused by the system, so they are taken before the
    // settings that name them are saved.
    let wanted = store.preview(patch)?.shortcuts;
    let shortcuts_change = wanted != before.shortcuts;
    if shortcuts_change {
        crate::shortcuts::replace(app, &before.shortcuts, &wanted)?;
    }
    let next = store.patch(patch).inspect_err(|_| {
        if shortcuts_change {
            let _ = crate::shortcuts::register(app, &before.shortcuts);
        }
    })?;
    app.state::<RuntimeHandle>()
        .send(Control::Settings(Box::new(next.clone())));
    if next.paused != before.paused {
        app.state::<AppMenu<R>>().sync_paused(next.paused);
    }
    if next.language != before.language {
        let lang = crate::i18n::Lang::resolve(next.language);
        app.state::<AppMenu<R>>().set_lang(lang);
        crate::menu::retitle_panels(app, lang);
    }
    if before.liquid_glass && !next.liquid_glass {
        // The bubble is told to take it away too, but it must not stay if the
        // setting flips while it is up.
        clear_glass(app);
    }
    #[cfg(target_os = "macos")]
    if next.hide_in_fullscreen != before.hide_in_fullscreen {
        let handle = app.clone();
        let full_screen = !next.hide_in_fullscreen;
        let _ = app.run_on_main_thread(move || {
            if let Some(window) = handle.get_webview_window(pet_window::PET_LABEL) {
                crate::platform::pin_to_all_spaces(&window, full_screen);
            }
        });
    }
    if next.pet_scale != before.pet_scale {
        if let Some(window) = app.get_webview_window(pet_window::PET_LABEL) {
            pet_window::apply_size(&window, before.pet_scale, next.pet_scale)
                .map_err(|e| e.to_string())?;
        }
    }
    let _ = app.emit("settings://changed", &next);
    Ok(next)
}

/// Takes the bubble's material away.
fn clear_glass<R: Runtime>(app: &AppHandle<R>) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = handle.get_webview_window(pet_window::PET_LABEL) {
            crate::platform::set_glass(&window, None, 0.0);
        }
    });
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

/// Which system glass the hover bubble can use, or `"none"` where there is none.
#[tauri::command]
pub fn glass_support() -> &'static str {
    crate::platform::glass().map_or("none", |glass| glass.name())
}

/// Where the hover bubble is, in logical pixels from the window's top left;
/// `None` takes the material away again. The material is a native view behind
/// the page, so the page has to say where to put it.
#[tauri::command]
pub fn set_glass_bubble(app: AppHandle, rect: Option<[f64; 4]>, radius: f64) {
    let wanted = app.state::<SettingsStore>().get().liquid_glass;
    let rect = if wanted {
        rect.map(|[x, y, w, h]| (x, y, w, h))
    } else {
        None
    };
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = handle.get_webview_window(pet_window::PET_LABEL) {
            crate::platform::set_glass(&window, rect, radius);
        }
    });
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
        "tour" => Panel::Tour,
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

/// Opens one of the app's own places outside it. The destinations are fixed
/// here, so the pages need no permission to open arbitrary URLs or paths.
#[tauri::command]
pub fn open_external(app: AppHandle, target: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let opener = app.opener();
    match target.as_str() {
        "site" => opener.open_url("https://jokbet.jokerben.top", None::<&str>),
        "github" => opener.open_url("https://github.com/sparkjokerben/jokbet", None::<&str>),
        "logs" => {
            let dir = app.path().app_log_dir().map_err(|e| e.to_string())?;
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            opener.open_path(dir.to_string_lossy(), None::<&str>)
        }
        other => return Err(format!("unknown place {other}")),
    }
    .map_err(|e| e.to_string())
}

/// Lets go of the global shortcuts while one is being recorded, and takes
/// them back after.
#[tauri::command]
pub fn suspend_shortcuts(app: AppHandle, suspended: bool) {
    crate::shortcuts::suspend(&app, suspended);
}

#[tauri::command]
pub fn update_status(app: AppHandle) -> crate::updater::UpdateStatus {
    crate::updater::status(&app)
}

/// Checks for an update now (and downloads it), rather than at the next
/// background check.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> crate::updater::UpdateStatus {
    crate::updater::check_now(&app).await
}

/// Installs the downloaded update and restarts into it.
#[tauri::command]
pub fn install_update(app: AppHandle) -> Result<(), String> {
    if crate::updater::install_pending(&app) {
        app.restart();
    }
    Err("no update could be installed".into())
}
