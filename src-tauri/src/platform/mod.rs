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

/// Keeps the pet on every Space, out of Mission Control and the Cmd-` cycle.
/// With `full_screen`, full-screen Spaces too; without it, macOS leaves the pet
/// out of them.
/// Must run on the main thread.
#[cfg(target_os = "macos")]
pub fn pin_to_all_spaces<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>, full_screen: bool) {
    if let Ok(ns_window) = window.ns_window() {
        macos::pin_to_all_spaces(ns_window, full_screen);
    }
}

/// The physical layout of the keyboard: `"ansi"`, `"iso"`, or `"unknown"`
/// where the system does not say (everywhere but macOS).
pub fn keyboard_kind() -> &'static str {
    #[cfg(target_os = "macos")]
    return macos::keyboard_kind();
    #[allow(unreachable_code)]
    "unknown"
}

/// A rectangle as `(x, y, w, h)`.
pub type Bounds = (f64, f64, f64, f64);

/// Whether the rectangle holds the point.
pub fn contains(outer: Bounds, x: f64, y: f64) -> bool {
    x >= outer.0 && x < outer.0 + outer.2 && y >= outer.1 && y < outer.1 + outer.3
}

/// Whether a window covers all of a display: the mark of a full-screen one.
/// A pixel of slack allows for frames rounded differently on each side.
#[cfg_attr(target_os = "linux", allow(dead_code))]
pub fn covers(window: Bounds, display: Bounds) -> bool {
    const SLACK: f64 = 1.0;
    window.0 <= display.0 + SLACK
        && window.1 <= display.1 + SLACK
        && window.0 + window.2 >= display.0 + display.2 - SLACK
        && window.1 + window.3 >= display.1 + display.3 - SLACK
}

/// Whether another app shows a full-screen window (or a presentation) on the
/// display holding the point. The point is in the window API's physical
/// pixels; `scale` turns them into points where the platform measures so.
#[allow(unused_variables)]
pub fn fullscreen_covers(x: f64, y: f64, scale: f64) -> bool {
    #[cfg(target_os = "macos")]
    return macos::fullscreen_covers(x / scale, y / scale);
    #[cfg(windows)]
    return windows::fullscreen_covers(x, y);
    #[cfg(target_os = "linux")]
    return linux::fullscreen_covers(x, y);
    #[allow(unreachable_code)]
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Bounds = (0.0, 0.0, 1440.0, 900.0);

    #[test]
    fn a_window_as_large_as_the_display_covers_it() {
        assert!(covers(SCREEN, SCREEN));
        assert!(covers((-0.5, 0.0, 1441.0, 900.0), SCREEN));
    }

    #[test]
    fn a_maximised_window_leaves_the_menu_bar() {
        assert!(!covers((0.0, 25.0, 1440.0, 875.0), SCREEN));
        assert!(!covers((0.0, 0.0, 1440.0, 850.0), SCREEN));
    }

    #[test]
    fn a_full_screen_window_on_another_display_is_not_this_one() {
        let side = (1440.0, 0.0, 1920.0, 1080.0);
        assert!(!covers(side, SCREEN));
        assert!(contains(side, 1500.0, 10.0));
        assert!(!contains(side, 100.0, 10.0));
    }
}
