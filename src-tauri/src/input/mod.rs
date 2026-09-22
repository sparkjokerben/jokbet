//! Listen-only global keyboard and mouse hooks behind one interface.
//!
//! Hook callbacks only normalize the event and `try_send` it: they never block,
//! lock, allocate or touch keyboard-layout APIs. Counting happens elsewhere.

pub mod keymap;
#[cfg(target_os = "macos")]
mod macos;

use crossbeam_channel::Sender;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RawEvent {
    /// `code` is a W3C `KeyboardEvent.code`; auto-repeats are never sent.
    Key {
        code: &'static str,
        down: bool,
    },
    Button {
        button: MouseButton,
        down: bool,
    },
    /// `momentum` marks macOS inertial scrolling after the fingers lift.
    Scroll {
        momentum: bool,
    },
    /// Cursor position in the platform's global coordinates
    /// (points on macOS, physical pixels on Windows and X11).
    Move {
        x: f64,
        y: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimedEvent {
    /// Monotonic milliseconds since app start (see [`now_ms`]).
    pub t_ms: u64,
    pub ev: RawEvent,
}

static EPOCH: OnceLock<Instant> = OnceLock::new();

pub fn now_ms() -> u64 {
    EPOCH.get_or_init(Instant::now).elapsed().as_millis() as u64
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    Granted,
    Denied,
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    NotRequired,
}

/// Where hooks deliver events; counts what had to be dropped when full.
#[derive(Clone)]
pub struct EventSink {
    tx: Sender<TimedEvent>,
    dropped: Arc<AtomicU64>,
}

impl EventSink {
    pub fn new(tx: Sender<TimedEvent>) -> Self {
        Self {
            tx,
            dropped: Arc::new(AtomicU64::new(0)),
        }
    }

    #[inline]
    pub fn send(&self, ev: RawEvent) {
        if self.tx.try_send(TimedEvent { t_ms: now_ms(), ev }).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}

pub trait InputHandle: Send {
    /// True while events are being delivered; re-enables a hook the OS turned off.
    fn check_health(&self) -> bool;
    fn stop(self: Box<Self>);
}

#[cfg(target_os = "macos")]
pub use macos::{permission, request_permission, start};

#[cfg(not(target_os = "macos"))]
pub use null::{permission, request_permission, start};

#[cfg(not(target_os = "macos"))]
mod null {
    //! Placeholder until the Windows and X11 hooks land.
    use super::{EventSink, InputHandle, Permission};

    pub fn permission() -> Permission {
        Permission::NotRequired
    }

    pub fn request_permission() {}

    struct NullHandle;

    impl InputHandle for NullHandle {
        fn check_health(&self) -> bool {
            true
        }
        fn stop(self: Box<Self>) {}
    }

    pub fn start(_sink: EventSink) -> Result<Box<dyn InputHandle>, String> {
        Ok(Box::new(NullHandle))
    }
}
