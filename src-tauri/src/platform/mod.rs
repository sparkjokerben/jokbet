//! Platform queries that are not input hooks.

use crate::engine::distance::Display;

#[cfg(target_os = "macos")]
mod macos;

/// Displays in the same coordinate space as `RawEvent::Move`, with physical size.
#[cfg(target_os = "macos")]
pub fn displays() -> Vec<Display> {
    macos::displays()
}

#[cfg(not(target_os = "macos"))]
pub fn displays() -> Vec<Display> {
    Vec::new()
}
