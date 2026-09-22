//! Platform queries that are not input hooks.

use crate::engine::distance::Display;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

/// Displays in the same coordinate space as `RawEvent::Move`, with physical size.
pub fn displays() -> Vec<Display> {
    #[cfg(target_os = "macos")]
    return macos::displays();
    #[cfg(windows)]
    return windows::displays();
    #[cfg(target_os = "linux")]
    return linux::displays();
    #[allow(unreachable_code)]
    Vec::new()
}

/// Whether keyboard input is currently hidden from listeners (macOS Secure Input).
/// Must be called on the main thread.
#[cfg(target_os = "macos")]
pub fn secure_input_enabled() -> bool {
    macos::secure_input_enabled()
}

#[cfg(not(target_os = "macos"))]
pub fn secure_input_enabled() -> bool {
    false
}

/// Whether the primary mouse button is held right now. Needs no permission;
/// used to tell when a window drag really ends.
pub struct PrimaryButton {
    #[cfg(target_os = "linux")]
    x11: linux::PointerX11,
}

impl PrimaryButton {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "linux")]
            x11: linux::PointerX11::default(),
        }
    }

    pub fn pressed(&mut self) -> bool {
        #[cfg(target_os = "macos")]
        return macos::primary_button_pressed();
        #[cfg(windows)]
        return windows::primary_button_pressed();
        #[cfg(target_os = "linux")]
        return self.x11.primary_pressed();
        #[allow(unreachable_code)]
        false
    }
}

/// Keeps the pet on every Space (fullscreen ones too), out of Mission Control
/// and the Cmd-` cycle.
/// Must run on the main thread.
#[cfg(target_os = "macos")]
pub fn pin_to_all_spaces<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) {
    if let Ok(ns_window) = window.ns_window() {
        macos::pin_to_all_spaces(ns_window);
    }
}
