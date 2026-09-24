//! Global shortcuts: one shows or hides the pet, one pauses counting. None is
//! set until the user records one in the settings.

use crate::settings::{parse_shortcut, SettingsStore, Shortcuts, SHORTCUT_TAKEN};
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state == ShortcutState::Pressed {
                run(app, shortcut);
            }
        })
        .build()
}

fn run<R: Runtime>(app: &AppHandle<R>, pressed: &Shortcut) {
    let set = app.state::<SettingsStore>().get().shortcuts;
    let is = |accelerator: &Option<String>| {
        accelerator
            .as_deref()
            .and_then(|a| parse_shortcut(a).ok())
            .is_some_and(|s| s.id() == pressed.id())
    };
    if is(&set.toggle_pet) {
        crate::visibility::toggle(app);
    } else if is(&set.pause) {
        crate::menu::toggle_pause(app);
    }
}

/// Makes `shortcuts` the registered set, and nothing else.
pub fn register<R: Runtime>(app: &AppHandle<R>, shortcuts: &Shortcuts) -> Result<(), String> {
    let global = app.global_shortcut();
    let _ = global.unregister_all();
    for accelerator in [&shortcuts.toggle_pet, &shortcuts.pause]
        .into_iter()
        .flatten()
    {
        global
            .register(parse_shortcut(accelerator)?)
            .map_err(|e| format!("{SHORTCUT_TAKEN}: {accelerator}: {e}"))?;
    }
    Ok(())
}

/// Registers `next`, or puts `previous` back if any of it cannot be had
/// (another app holds the combination).
pub fn replace<R: Runtime>(
    app: &AppHandle<R>,
    previous: &Shortcuts,
    next: &Shortcuts,
) -> Result<(), String> {
    register(app, next).inspect_err(|_| {
        let _ = register(app, previous);
    })
}

/// While the settings window records a combination, the current ones are let
/// go, so pressing one records it rather than runs it.
pub fn suspend<R: Runtime>(app: &AppHandle<R>, suspended: bool) {
    let result = if suspended {
        app.global_shortcut()
            .unregister_all()
            .map_err(|e| e.to_string())
    } else {
        register(app, &app.state::<SettingsStore>().get().shortcuts)
    };
    if let Err(e) = result {
        log::warn!("shortcuts: {e}");
    }
}
