//! A stroll along the bottom of the screen: the pet window moved a little at a
//! time, on a thread of its own, while the page plays the walk.
//!
//! The page decides when to go, how fast and how far (it knows what the pet is
//! doing); this side knows the screen, and moves the window. It walks only on
//! the bottom of the work area — standing on the Dock or the taskbar — so a
//! pet put down somewhere in the middle of the screen stays where it was put.
//! It turns round at the edges of the monitor it is on, stops the moment
//! anything else needs the pet, and saves where it ended up once, at the end.

use crate::hover::HoverState;
use crate::pet_window::{self, Rect, PET_LABEL};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Runtime, WebviewWindow};

/// How near the bottom of the work area its feet must be, in logical pixels.
const NEAR_BOTTOM: f64 = 24.0;
/// How often the window moves: about 30 times a second.
const STEP: Duration = Duration::from_millis(33);
/// The look in the direction of travel before setting off.
const GLANCE: Duration = Duration::from_millis(500);
/// Logical pixels a second, and logical pixels a stroll, that are allowed.
const SPEED: (f64, f64) = (30.0, 50.0);
const DISTANCE: (f64, f64) = (100.0, 500.0);

const RUNNING: u8 = 0;
const STOPPED: u8 = 1;
/// Stopped by something that moves the pet itself, and saves where it goes.
const MOVED: u8 = 2;

/// Why a walk is stopped from outside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// Anything that needs the pet where it is: input, or the page.
    Now,
    /// Something else is moving the pet — a drag, or the menu putting it back
    /// in its corner — and saves the position once it is done.
    Moved,
}

struct Walk {
    id: u32,
    stop: Arc<AtomicU8>,
}

#[derive(Default)]
pub struct WalkState {
    current: Mutex<Option<Walk>>,
    /// Whether a walk is under way, for the runtime to check with every input
    /// without taking the lock.
    walking: AtomicBool,
    next_id: AtomicU32,
}

/// What the page is told as a walk goes on: `glance`, then `walk` (again at
/// every turn), then `stop`.
#[derive(Clone, Debug, Serialize)]
pub struct WalkEvent {
    pub id: u32,
    pub phase: &'static str,
    pub dir: i8,
}

/// Whether feet at `feet_y` stand on the bottom of `work_area`.
pub fn near_bottom(feet_y: i32, work_area: Rect, scale_factor: f64) -> bool {
    let bottom = work_area.y + work_area.h;
    f64::from((feet_y - bottom).abs()) <= (NEAR_BOTTOM * scale_factor).round()
}

/// Where the window's left edge may go, so the whole window stays on the work
/// area (the same rule the position is clamped by when it is saved).
pub fn range_x(work_area: Rect, win_w: i32) -> (f64, f64) {
    let min = work_area.x;
    let max = (work_area.x + work_area.w - win_w).max(min);
    (f64::from(min), f64::from(max))
}

/// One step of `dx` in `dir`, turning round at either end.
pub fn step(x: f64, dir: i8, dx: f64, (min, max): (f64, f64)) -> (f64, i8) {
    let next = x + f64::from(dir) * dx;
    if next > max {
        ((max - (next - max)).max(min), -1)
    } else if next < min {
        ((min + (min - next)).min(max), 1)
    } else {
        (next, dir)
    }
}

/// The way it sets off: the one asked for, unless it is already at that end.
pub fn start_dir(wanted: i8, x: f64, (min, max): (f64, f64)) -> i8 {
    if wanted >= 0 {
        if x >= max {
            -1
        } else {
            1
        }
    } else if x <= min {
        1
    } else {
        -1
    }
}

/// The work area of the monitor the point is on.
fn work_area_at<R: Runtime>(window: &WebviewWindow<R>, (x, y): (i32, i32)) -> Option<Rect> {
    window.available_monitors().ok()?.iter().find_map(|m| {
        let (pos, size) = (m.position(), m.size());
        let on = x >= pos.x
            && x < pos.x + size.width as i32
            && y >= pos.y
            && y < pos.y + size.height as i32;
        on.then(|| {
            let wa = m.work_area();
            Rect {
                x: wa.position.x,
                y: wa.position.y,
                w: wa.size.width as i32,
                h: wa.size.height as i32,
            }
        })
    })
}

fn dragging<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.state::<HoverState>().dragging.load(Ordering::Acquire)
}

/// Sets off on a stroll, if the pet is standing on the bottom of the screen
/// and nothing else has it. `speed` is in logical pixels a second, `distance`
/// in logical pixels, `dir` -1 for left and 1 for right. Returns the walk's id,
/// which every event about it carries.
pub fn start<R: Runtime>(app: &AppHandle<R>, speed: f64, distance: f64, dir: i8) -> Option<u32> {
    let state = app.state::<WalkState>();
    let mut current = state.current.lock().unwrap();
    if current.is_some() || !crate::visibility::shown(app) || dragging(app) {
        return None;
    }
    let window = app.get_webview_window(PET_LABEL)?;
    let sf = window.scale_factor().ok()?;
    let pos = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    let (win_w, win_h) = (size.width as i32, size.height as i32);
    let work_area = work_area_at(&window, (pos.x + win_w / 2, pos.y + win_h - 1))?;
    if !near_bottom(pos.y + win_h, work_area, sf) {
        return None;
    }
    let range = range_x(work_area, win_w);
    if range.0 >= range.1 {
        return None;
    }
    let id = state
        .next_id
        .fetch_add(1, Ordering::Relaxed)
        .wrapping_add(1);
    let stop = Arc::new(AtomicU8::new(RUNNING));
    *current = Some(Walk {
        id,
        stop: stop.clone(),
    });
    state.walking.store(true, Ordering::Release);
    drop(current);

    let walk = Stroll {
        id,
        x: f64::from(pos.x),
        // On the bottom edge exactly, wherever near it the pet was.
        y: work_area.y + work_area.h - win_h,
        dir: start_dir(dir, f64::from(pos.x), range),
        speed: speed.clamp(SPEED.0, SPEED.1) * sf,
        distance: distance.clamp(DISTANCE.0, DISTANCE.1) * sf,
        range,
    };
    let handle = app.clone();
    let spawned = std::thread::Builder::new()
        .name("walk".into())
        .spawn(move || walk.run(&handle, &window, &stop));
    if spawned.is_err() {
        finish(app, id);
        return None;
    }
    Some(id)
}

/// Stops the walk under way, if there is one.
pub fn stop<R: Runtime>(app: &AppHandle<R>, why: Stop) {
    let state = app.state::<WalkState>();
    if !state.walking.load(Ordering::Acquire) {
        return;
    }
    let code = match why {
        Stop::Now => STOPPED,
        Stop::Moved => MOVED,
    };
    let current = state.current.lock().unwrap();
    if let Some(walk) = current.as_ref() {
        let _ = walk
            .stop
            .compare_exchange(RUNNING, code, Ordering::AcqRel, Ordering::Acquire);
    }
}

fn finish<R: Runtime>(app: &AppHandle<R>, id: u32) {
    let state = app.state::<WalkState>();
    let mut current = state.current.lock().unwrap();
    if current.as_ref().is_some_and(|w| w.id == id) {
        *current = None;
        state.walking.store(false, Ordering::Release);
    }
}

/// One stroll, in physical pixels on one monitor.
struct Stroll {
    id: u32,
    x: f64,
    y: i32,
    dir: i8,
    speed: f64,
    distance: f64,
    range: (f64, f64),
}

impl Stroll {
    fn run<R: Runtime>(mut self, app: &AppHandle<R>, window: &WebviewWindow<R>, stop: &AtomicU8) {
        let emit = |phase: &'static str, dir: i8| {
            let _ = app.emit_to(
                PET_LABEL,
                "pet://walk",
                WalkEvent {
                    id: self.id,
                    phase,
                    dir,
                },
            );
        };
        let stopped = || {
            if dragging(app) {
                let _ = stop.compare_exchange(RUNNING, MOVED, Ordering::AcqRel, Ordering::Acquire);
            }
            stop.load(Ordering::Acquire) != RUNNING || !crate::visibility::shown(app)
        };
        let _ = window.set_position(PhysicalPosition::new(self.x.round() as i32, self.y));
        emit("glance", self.dir);
        let glance_ends = Instant::now() + GLANCE;
        while Instant::now() < glance_ends && !stopped() {
            std::thread::sleep(STEP);
        }
        if !stopped() {
            emit("walk", self.dir);
            let mut walked = 0.0;
            let mut last = Instant::now();
            while walked < self.distance && !stopped() {
                std::thread::sleep(STEP);
                let now = Instant::now();
                // A stall (a menu holding the main thread, a sleeping laptop)
                // is not made up for in one leap.
                let dt = (now - last).as_secs_f64().min(0.1);
                last = now;
                let dx = (self.speed * dt).min(self.distance - walked);
                let (x, dir) = step(self.x, self.dir, dx, self.range);
                walked += dx;
                self.x = x;
                if dir != self.dir {
                    self.dir = dir;
                    emit("walk", dir);
                }
                // Once more, right before the move: a drag or the menu may
                // have taken the pet in the meantime.
                if stopped() {
                    break;
                }
                let _ = window.set_position(PhysicalPosition::new(self.x.round() as i32, self.y));
            }
        }
        // A drag can begin just as the walk is stopped for something else.
        let moved = stop.load(Ordering::Acquire) == MOVED || dragging(app);
        finish(app, self.id);
        emit("stop", self.dir);
        // Where it stopped is where it stays, unless something else moved it.
        if !moved {
            if let Err(e) = pet_window::save_position(window) {
                log::warn!("walk: saving the position failed: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 1440x900 screen with a 70 px Dock at the bottom and the menu bar on top.
    const WORK: Rect = Rect {
        x: 0,
        y: 25,
        w: 1440,
        h: 805,
    };

    #[test]
    fn it_walks_only_on_the_bottom_of_the_work_area() {
        assert!(near_bottom(830, WORK, 1.0));
        assert!(near_bottom(830 + 24, WORK, 1.0), "on the Dock itself");
        assert!(near_bottom(830 - 24, WORK, 1.0));
        assert!(!near_bottom(830 - 25, WORK, 1.0), "held up in the air");
        assert!(!near_bottom(900, WORK, 1.0), "behind the Dock");
        assert!(
            near_bottom(830 - 48, WORK, 2.0),
            "the margin is in logical pixels"
        );
        assert!(!near_bottom(830 - 49, WORK, 2.0));
    }

    #[test]
    fn the_window_stays_on_the_work_area() {
        assert_eq!(range_x(WORK, 220), (0.0, 1220.0));
        let second = Rect {
            x: 1440,
            y: 0,
            w: 1920,
            h: 1040,
        };
        assert_eq!(range_x(second, 440), (1440.0, 2920.0));
        // A screen narrower than the window has nowhere to go.
        let narrow = Rect { w: 200, ..WORK };
        assert_eq!(range_x(narrow, 220), (0.0, 0.0));
    }

    #[test]
    fn it_turns_round_at_the_edges() {
        let range = (0.0, 100.0);
        assert_eq!(step(50.0, 1, 10.0, range), (60.0, 1));
        assert_eq!(step(50.0, -1, 10.0, range), (40.0, -1));
        assert_eq!(step(95.0, 1, 10.0, range), (95.0, -1));
        assert_eq!(step(3.0, -1, 10.0, range), (7.0, 1));
        // A step longer than the whole range still lands inside it.
        assert_eq!(step(50.0, 1, 500.0, range), (0.0, -1));
    }

    #[test]
    fn it_sets_off_the_way_there_is_room() {
        let range = (0.0, 100.0);
        assert_eq!(start_dir(1, 50.0, range), 1);
        assert_eq!(start_dir(-1, 50.0, range), -1);
        assert_eq!(start_dir(1, 100.0, range), -1);
        assert_eq!(start_dir(-1, 0.0, range), 1);
    }
}
