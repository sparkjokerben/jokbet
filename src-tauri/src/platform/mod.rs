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

/// Whether a fullscreen app (game, video, presentation) is in front, so the
/// pet should step aside. macOS needs no probe: native fullscreen happens in
/// its own Space, which the pet window does not join.
pub struct FullscreenProbe {
    #[cfg(target_os = "linux")]
    x11: linux::FullscreenX11,
}

impl FullscreenProbe {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "linux")]
            x11: linux::FullscreenX11::default(),
        }
    }

    pub fn active(&mut self) -> bool {
        #[cfg(windows)]
        return windows::fullscreen_active();
        #[cfg(target_os = "linux")]
        return self.x11.active();
        #[allow(unreachable_code)]
        false
    }
}

/// Keeps the pet on every Space, out of Mission Control and the Cmd-` cycle.
/// Must run on the main thread.
#[cfg(target_os = "macos")]
pub fn pin_to_all_spaces<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) {
    if let Ok(ns_window) = window.ns_window() {
        macos::pin_to_all_spaces(ns_window);
    }
}
