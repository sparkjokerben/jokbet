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
    /// The hover card while it is up, in the same terms as the hit rect.
    pub bubble_rect: Mutex<Option<HitRect>>,
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

impl HitRect {
    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// The way up from the sprite to the card: as wide as the two of them, from
/// the card's bottom down to the sprite's top, over the counter or banner
/// between them.
fn bridge(rect: HitRect, card: HitRect) -> HitRect {
    let left = rect.x.min(card.x);
    let right = (rect.x + rect.w).max(card.x + card.w);
    let top = card.y + card.h;
    HitRect {
        x: left,
        y: top,
        w: right - left,
        h: (rect.y - top).max(0.0),
    }
}

/// `card` is the hover card while it is up: the cursor on it, or on the way up
/// to it, is still on the pet, so the card stays up to be read.
pub fn sample(
    cursor: (f64, f64),
    window_pos: (f64, f64),
    scale: f64,
    rect: HitRect,
    card: Option<HitRect>,
) -> CursorSample {
    let lx = (cursor.0 - window_pos.0) / scale;
    let ly = (cursor.1 - window_pos.1) / scale;
    let inside = rect.contains(lx, ly)
        || card.is_some_and(|card| card.contains(lx, ly) || bridge(rect, card).contains(lx, ly));
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
        let card = *app.state::<HoverState>().bubble_rect.lock().unwrap();
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

        let mut s = sample(
            (cursor.x, cursor.y),
            (pos.x as f64, pos.y as f64),
            scale,
            rect,
            card,
        );
        // The pet is in hand for the whole of a drag. The window trails the
        // cursor while it is carried, so a fast drag puts the cursor off the
        // sprite for a sample or two, which would take the bubble down and put
        // it back up again all the way.
        if state.dragging.load(Ordering::Acquire) {
            s.inside = true;
        }

        if ignoring != Some(!s.inside) && window.set_ignore_cursor_events(!s.inside).is_ok() {
            ignoring = Some(!s.inside);
        }
        if last.map(|l| l.inside) != Some(s.inside) {
            // The card goes as the cursor leaves; until the page says so, the
            // place it was is not to hold the hover.
            if !s.inside {
                *state.bubble_rect.lock().unwrap() = None;
            }
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
            None,
        );
        assert!(s.inside);
        assert_eq!(s.gaze, (0, 0));
    }

    #[test]
    fn transparent_area_is_outside() {
        let s = sample(
            (1000.0 + 20.0, 500.0 + 20.0),
            (1000.0, 500.0),
            1.0,
            RECT,
            None,
        );
        assert!(!s.inside);
        assert_eq!(s.gaze, (-1, -1));
    }

    #[test]
    fn gaze_follows_far_cursor() {
        let s = sample((5000.0, 500.0 + 210.0), (1000.0, 500.0), 1.0, RECT, None);
        assert_eq!(s.gaze, (1, 0));
        let s = sample((1000.0 + 110.0, 2000.0), (1000.0, 500.0), 1.0, RECT, None);
        assert_eq!(s.gaze, (0, 1));
    }

    /// A card wider than the pet, up above it with a gap between.
    const CARD: HitRect = HitRect {
        x: 35.0,
        y: 60.0,
        w: 150.0,
        h: 90.0,
    };

    #[test]
    fn card_keeps_the_hover() {
        let at = |x: f64, y: f64, card| sample((x, y), (0.0, 0.0), 1.0, RECT, card).inside;
        // On the card, and in the gap on the way up to it.
        assert!(at(40.0, 100.0, Some(CARD)));
        assert!(at(110.0, 165.0, Some(CARD)));
        // Neither is anything without a card up.
        assert!(!at(40.0, 100.0, None));
        assert!(!at(110.0, 165.0, None));
        // Beside the card and above it is off the pet.
        assert!(!at(20.0, 100.0, Some(CARD)));
        assert!(!at(110.0, 40.0, Some(CARD)));
    }
}
