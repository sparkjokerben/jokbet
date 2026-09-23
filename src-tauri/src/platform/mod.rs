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

/// The system material the platform can put behind the hover bubble.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum Glass {
    /// macOS 26 and later: Liquid Glass.
    Liquid,
    /// Earlier macOS: a vibrancy material.
    Vibrancy,
    /// Windows: a blurred region behind the bubble.
    Blur,
}

impl Glass {
    /// The name the settings window shows; `"none"` means no support.
    pub fn name(self) -> &'static str {
        match self {
            Glass::Liquid => "liquidGlass",
            Glass::Vibrancy => "vibrancy",
            Glass::Blur => "blur",
        }
    }
}

/// What this system offers, if anything.
pub fn glass() -> Option<Glass> {
    #[cfg(target_os = "macos")]
    return macos::glass();
    #[cfg(windows)]
    return Some(Glass::Blur);
    #[cfg(target_os = "linux")]
    return None;
    #[allow(unreachable_code)]
    None
}

/// Puts the glass behind the bubble: `rect` is `(x, y, w, h)` in logical
/// pixels with the origin at the window's top-left, `None` removes it.
/// Must run on the main thread.
#[cfg(target_os = "macos")]
pub fn set_glass<R: tauri::Runtime>(
    window: &tauri::WebviewWindow<R>,
    rect: Option<(f64, f64, f64, f64)>,
    radius: f64,
) {
    if let Ok(ns_window) = window.ns_window() {
        macos::set_glass(ns_window, rect, radius);
    }
}

#[cfg(windows)]
pub fn set_glass<R: tauri::Runtime>(
    window: &tauri::WebviewWindow<R>,
    rect: Option<(f64, f64, f64, f64)>,
    radius: f64,
) {
    let _ = radius; // Windows has no corner radius for a blurred region.
    let Ok(hwnd) = window.hwnd() else { return };
    let scale = window.scale_factor().unwrap_or(1.0);
    let rect = rect.map(|(x, y, w, h)| {
        (
            (x * scale).round() as i32,
            (y * scale).round() as i32,
            (w * scale).round() as i32,
            (h * scale).round() as i32,
        )
    });
    windows::set_glass(hwnd, rect);
}

#[cfg(target_os = "linux")]
pub fn set_glass<R: tauri::Runtime>(
    _window: &tauri::WebviewWindow<R>,
    _rect: Option<(f64, f64, f64, f64)>,
    _radius: f64,
) {
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
