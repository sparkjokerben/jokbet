//! Polls the cursor to drive click-through, hover, eye gaze and the end of
//! a drag.
//!
//! Polling `cursor_position` and the button state needs no Input Monitoring
//! permission, so the pet stays clickable and keeps looking at the cursor
//! even before it is granted.

use crate::pet_window::{self, PET_LABEL};
use crate::platform::PrimaryButton;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, Runtime};

const POLL: Duration = Duration::from_millis(33);
const IDLE_POLL: Duration = Duration::from_millis(250);
/// Logical-pixel distance from the pet's center before the eyes turn.
const GAZE_DEAD_ZONE: f64 = 40.0;
/// Consecutive polls with the button up before a drag counts as finished
/// (debounces a single missed sample).
const RELEASE_POLLS: u32 = 2;

/// The sprite's clickable area in logical pixels, relative to the window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HitRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Default)]
pub struct HoverState {
    pub hit_rect: Mutex<Option<HitRect>>,
    /// The pet is being dragged; ends when the primary button is released,
    /// however long the pointer rests on the way.
    pub dragging: AtomicBool,
}

/// Where the cursor is relative to the hit rect, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CursorSample {
    pub inside: bool,
    pub gaze: (i8, i8),
}

pub fn sample(
    cursor: (f64, f64),
    window_pos: (f64, f64),
    scale: f64,
    rect: HitRect,
) -> CursorSample {
    let lx = (cursor.0 - window_pos.0) / scale;
    let ly = (cursor.1 - window_pos.1) / scale;
    let inside = lx >= rect.x && lx < rect.x + rect.w && ly >= rect.y && ly < rect.y + rect.h;
    let dx = lx - (rect.x + rect.w / 2.0);
    let dy = ly - (rect.y + rect.h / 2.0);
    let axis = |d: f64| {
        if d < -GAZE_DEAD_ZONE {
            -1
        } else if d > GAZE_DEAD_ZONE {
            1
        } else {
            0
        }
    };
    CursorSample {
        inside,
        gaze: (axis(dx), axis(dy)),
    }
}

pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    std::thread::Builder::new()
        .name("hover".into())
        .spawn(move || run(app))
        .expect("spawn hover thread");
}

fn run<R: Runtime>(app: AppHandle<R>) {
    let mut ignoring: Option<bool> = None;
    let mut last: Option<CursorSample> = None;
    let mut button = PrimaryButton::new();
    let mut released_polls = 0;
    loop {
        let Some(window) = app.get_webview_window(PET_LABEL) else {
            std::thread::sleep(IDLE_POLL);
            continue;
        };
        let rect = *app.state::<HoverState>().hit_rect.lock().unwrap();
        let (Some(rect), Ok(true)) = (rect, window.is_visible()) else {
            // Away, or nothing to sample against: forget the last sample, so
            // that whatever is there when it comes back is reported again.
            last = None;
            std::thread::sleep(IDLE_POLL);
            continue;
        };
        let (Ok(cursor), Ok(pos), Ok(scale)) = (
            app.cursor_position(),
            window.outer_position(),
            window.scale_factor(),
        ) else {
            std::thread::sleep(IDLE_POLL);
            continue;
        };
        let state = app.state::<HoverState>();
        if state.dragging.load(Ordering::Acquire) {
            if button.pressed() {
                released_polls = 0;
            } else {
                released_polls += 1;
                if released_polls >= RELEASE_POLLS {
                    released_polls = 0;
                    state.dragging.store(false, Ordering::Release);
                    let _ = pet_window::save_position(&window);
                    let _ = app.emit_to(PET_LABEL, "pet://drag-end", ());
                }
            }
        }

        let s = sample(
            (cursor.x, cursor.y),
            (pos.x as f64, pos.y as f64),
            scale,
            rect,
        );

        if ignoring != Some(!s.inside) && window.set_ignore_cursor_events(!s.inside).is_ok() {
            ignoring = Some(!s.inside);
        }
        if last.map(|l| l.inside) != Some(s.inside) {
            // The material behind the bubble is native, and a menu can hold the
            // main thread: told here, it can follow the cursor even then.
            crate::platform::pet_hovered(s.inside);
            let _ = app.emit_to(PET_LABEL, "pet://hover", s.inside);
        }
        if last.map(|l| l.gaze) != Some(s.gaze) {
            let _ = app.emit_to(PET_LABEL, "pet://gaze", [s.gaze.0, s.gaze.1]);
        }
        // The material shows what is behind the pet through, and that changes as
        // the pet is carried about — a drag most of all, which is a move loop the
        // page never sees, so the card it draws is drawn for wherever the pet was
        // when the cursor arrived. Read here, where something is still running
        // through the drag; the reading is eased on the way (see `read_tone`), so
        // what the page gets is a drift it can draw as one.
        if s.inside {
            if let Some(tone) = crate::platform::read_tone() {
                let _ = app.emit_to(PET_LABEL, "pet://tone", tone);
            }
        }
        last = Some(s);
        std::thread::sleep(POLL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECT: HitRect = HitRect {
        x: 60.0,
        y: 180.0,
        w: 100.0,
        h: 60.0,
    };

    #[test]
    fn inside_uses_logical_coordinates() {
        // Window at physical (1000, 500) on a 2x display; cursor over the sprite center.
        let s = sample(
            (1000.0 + 110.0 * 2.0, 500.0 + 210.0 * 2.0),
            (1000.0, 500.0),
            2.0,
            RECT,
        );
        assert!(s.inside);
        assert_eq!(s.gaze, (0, 0));
    }

    #[test]
    fn transparent_area_is_outside() {
        let s = sample((1000.0 + 20.0, 500.0 + 20.0), (1000.0, 500.0), 1.0, RECT);
        assert!(!s.inside);
        assert_eq!(s.gaze, (-1, -1));
    }

    #[test]
    fn gaze_follows_far_cursor() {
        let s = sample((5000.0, 500.0 + 210.0), (1000.0, 500.0), 1.0, RECT);
        assert_eq!(s.gaze, (1, 0));
        let s = sample((1000.0 + 110.0, 2000.0), (1000.0, 500.0), 1.0, RECT);
        assert_eq!(s.gaze, (0, 1));
    }
}
