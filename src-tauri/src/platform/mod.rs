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
