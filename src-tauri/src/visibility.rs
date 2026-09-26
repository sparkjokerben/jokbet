//! Whether the pet is on screen. Two things can take it off: the user, from the
//! menu or a shortcut, and another app going full screen, when the settings
//! ask for that. It shows only when neither holds.

use crate::pet_window::PET_LABEL;
use crate::settings::SettingsStore;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Manager, Runtime};

/// How often to look for a full-screen app. The pet is out of the way within
/// this long, and back as soon after.
const WATCH_EVERY: Duration = Duration::from_millis(500);

#[derive(Default)]
pub struct PetVisibility {
    user_hidden: AtomicBool,
    covered: AtomicBool,
}

/// Hides the pet if the user had it shown, and the other way round.
pub fn toggle<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<PetVisibility>();
    state.user_hidden.fetch_xor(true, Ordering::AcqRel);
    apply(app);
}

/// Brings the pet back if the user had hidden it.
pub fn show<R: Runtime>(app: &AppHandle<R>) {
    app.state::<PetVisibility>()
        .user_hidden
        .store(false, Ordering::Release);
    apply(app);
}

fn apply<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<PetVisibility>();
    let user_hidden = state.user_hidden.load(Ordering::Acquire);
    let covered = state.covered.load(Ordering::Acquire);
    if let Some(window) = app.get_webview_window(PET_LABEL) {
        let _ = if user_hidden || covered {
            // The material behind the bubble is a window of its own, so it
            // would otherwise be left hanging where the pet was.
            crate::commands::clear_glass(app);
            window.hide()
        } else {
            window.show()
        };
    }
    // The menu offers what the user can do: a pet hidden by a full-screen app
    // is still one the user has shown.
    crate::menu::on_main(app, move |menu| menu.sync_toggle(!user_hidden));
}

/// Watches for full-screen apps on the pet's display.
pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    std::thread::Builder::new()
        .name("fullscreen".into())
        .spawn(move || loop {
            let covered = app.state::<SettingsStore>().get().hide_in_fullscreen
                && pet_anchor(&app)
                    .is_some_and(|(x, y, scale)| crate::platform::fullscreen_covers(x, y, scale));
            let state = app.state::<PetVisibility>();
            if state.covered.swap(covered, Ordering::AcqRel) != covered {
                apply(&app);
            }
            std::thread::sleep(WATCH_EVERY);
        })
        .expect("spawn fullscreen thread");
}

/// The bottom middle of the pet window, where its feet are, in physical
/// pixels, with the window's scale factor.
fn pet_anchor<R: Runtime>(app: &AppHandle<R>) -> Option<(f64, f64, f64)> {
    let window = app.get_webview_window(PET_LABEL)?;
    let pos = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    let scale = window.scale_factor().ok()?;
    Some((
        f64::from(pos.x) + f64::from(size.width) / 2.0,
        f64::from(pos.y) + f64::from(size.height) - 1.0,
        scale,
    ))
}
