//! The always-on-top transparent pet window: geometry, creation and placement.

use crate::settings::SettingsStore;
use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, Runtime, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

pub const PET_LABEL: &str = "pet";

/// Sprite canvas in cells (see src/sprites/jokbet.ts); the pet's own box is
/// 24x16 inside it, leaving room for the ball it juggles and its laptop.
const GRID_W: f64 = 40.0;
const GRID_H: f64 = 26.0;
/// The middle of the pet's box, in canvas columns (the box starts at column 6).
const PET_CENTER_X: f64 = 18.0;
/// Room above the canvas for the head counter and the hover bubble. The
/// bubble is a fixed-size card, so this covers all scales; the canvas above
/// the pet's head adds to it.
const TOP_PAD: f64 = 180.0;
const MIN_WIDTH: f64 = 220.0;
const EDGE_MARGIN: f64 = 16.0;
/// How near the bottom of the work area the feet must be, in logical pixels,
/// to count as standing on it. A stroll sets off from here, and a drag sticks
/// to the same line.
pub const GROUND_SNAP_LOGICAL: f64 = 24.0;

/// Logical window size for a scale (pixels per sprite cell).
pub fn window_size(scale: f64) -> (f64, f64) {
    ((GRID_W * scale).max(MIN_WIDTH), GRID_H * scale + TOP_PAD)
}

/// Logical x of the pet's middle in the window (the canvas is centered in it).
fn pet_center_x(scale: f64) -> f64 {
    window_size(scale).0 / 2.0 + (PET_CENTER_X - GRID_W / 2.0) * scale
}

/// Axis-aligned rectangle in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// Keeps the sprite part of the window on the monitor that holds the window's
/// bottom-center; the transparent area above it may hang off the top edge.
/// Returns `None` when that point is on no monitor (e.g. one was unplugged).
pub fn clamp_position(
    pos: (i32, i32),
    win: (i32, i32),
    top_pad: i32,
    monitors: &[Rect],
) -> Option<(i32, i32)> {
    let anchor = (pos.0 + win.0 / 2, pos.1 + win.1 - 1);
    let m = monitors.iter().find(|m| m.contains(anchor.0, anchor.1))?;
    let x = pos.0.clamp(m.x, (m.x + m.w - win.0).max(m.x));
    let y = pos
        .1
        .clamp(m.y - top_pad, (m.y + m.h - win.1).max(m.y - top_pad));
    Some((x, y))
}

/// Where a dragged window's top sits. `y` is where the cursor would put it.
/// Once the feet are within [`GROUND_SNAP_LOGICAL`] of the work-area bottom,
/// or lower — pushed down onto the Dock or the taskbar — they sit on that
/// bottom. Anything higher is left where the cursor put it, so pulling up
/// lets go at that same line. Compared against the cursor's `y`, not wherever
/// a previous sample stuck the window, so the two do not flicker.
pub fn snap_drag_y(y: i32, win_h: i32, work_area: Rect, scale_factor: f64) -> i32 {
    let ground = work_area.y + work_area.h - win_h;
    let threshold = (GROUND_SNAP_LOGICAL * scale_factor).round() as i32;
    if y >= ground - threshold {
        ground
    } else {
        y
    }
}

/// Cursor height for one drag sample. `snap` is false while Shift is held,
/// and then `y` is kept even on or below the ground.
pub fn placed_drag_y(y: i32, win_h: i32, work_area: Rect, scale_factor: f64, snap: bool) -> i32 {
    if snap {
        snap_drag_y(y, win_h, work_area, scale_factor)
    } else {
        y
    }
}

/// Bottom-right corner of a monitor's work area.
pub fn default_position(win: (i32, i32), work_area: Rect, margin: i32) -> (i32, i32) {
    (
        work_area.x + work_area.w - win.0 - margin,
        work_area.y + work_area.h - win.1,
    )
}

pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<WebviewWindow<R>> {
    let scale = app.state::<SettingsStore>().get().pet_scale;
    let (w, h) = window_size(scale);
    let mut builder = WebviewWindowBuilder::new(app, PET_LABEL, WebviewUrl::App("pet.html".into()))
        .title("Jokbet")
        .inner_size(w, h)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focusable(false)
        .accept_first_mouse(true)
        .visible_on_all_workspaces(true)
        .visible(false);
    if let Some(dir) = crate::portable::webview_dir() {
        builder = builder.data_directory(dir);
    }
    let window = builder.build()?;
    place(&window)?;
    window.show()?;
    Ok(window)
}

/// The work area of the monitor the point is on.
pub(crate) fn work_area_at<R: Runtime>(
    window: &WebviewWindow<R>,
    (x, y): (i32, i32),
) -> Option<Rect> {
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

fn monitor_rects<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<Vec<Rect>> {
    Ok(window
        .available_monitors()?
        .iter()
        .map(|m| Rect {
            x: m.position().x,
            y: m.position().y,
            w: m.size().width as i32,
            h: m.size().height as i32,
        })
        .collect())
}

/// Moves the window to its saved position (clamped) or the default corner.
pub fn place<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    let app = window.app_handle();
    let settings = app.state::<SettingsStore>();
    let sf = window.scale_factor()?;
    let scale = settings.get().pet_scale;
    let (w, h) = window_size(scale);
    let win = ((w * sf).round() as i32, (h * sf).round() as i32);
    let top_pad = ((h - GRID_H * scale) * sf).round() as i32;

    let monitors = monitor_rects(window)?;
    let saved = settings
        .get()
        .pet_position
        .and_then(|[x, y]| clamp_position((x, y), win, top_pad, &monitors));
    let pos = match saved {
        Some(p) => p,
        None => {
            let Some(primary) = window.primary_monitor()? else {
                return Ok(());
            };
            let wa = primary.work_area();
            let work_area = Rect {
                x: wa.position.x,
                y: wa.position.y,
                w: wa.size.width as i32,
                h: wa.size.height as i32,
            };
            default_position(win, work_area, (EDGE_MARGIN * sf).round() as i32)
        }
    };
    window.set_position(PhysicalPosition::new(pos.0, pos.1))
}

/// Forgets where the pet was dragged to and puts it back in the default corner.
pub fn reset_position<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    window
        .app_handle()
        .state::<SettingsStore>()
        .update(|s| s.pet_position = None)
        .map_err(tauri::Error::Io)?;
    place(window)
}

/// Resizes the window for a new pet size, keeping the pet's feet in place.
pub fn apply_size<R: Runtime>(window: &WebviewWindow<R>, from: f64, to: f64) -> tauri::Result<()> {
    let sf = window.scale_factor()?;
    let pos = window.outer_position()?;
    let (w1, h1) = window_size(to);
    let x = pos.x + ((pet_center_x(from) - pet_center_x(to)) * sf).round() as i32;
    let y = pos.y + ((window_size(from).1 - h1) * sf).round() as i32;
    window.set_size(LogicalSize::new(w1, h1))?;
    window.set_position(PhysicalPosition::new(x, y))?;
    // The window may move asynchronously: remember where it is going, not
    // where it still is.
    remember_position(window, (x, y), to)
}

/// Follows the cursor while the pet is dragged, and sticks to the ground when
/// [`snap_drag_y`] says so. `grab` is where the cursor sat inside the window
/// when the drag began, in physical pixels, so the window does not jump under
/// the pointer. `snap` is false while Shift is held: the cursor's height is
/// kept, including down on the Dock or the taskbar. Nothing is saved here;
/// that waits until the button comes up.
pub fn drag_follow<R: Runtime>(
    window: &WebviewWindow<R>,
    grab: (i32, i32),
    snap: bool,
) -> tauri::Result<()> {
    let cursor = window.app_handle().cursor_position()?;
    let size = window.outer_size()?;
    let sf = window.scale_factor()?;
    let (win_w, win_h) = (size.width as i32, size.height as i32);
    let x = cursor.x.round() as i32 - grab.0;
    let mut y = cursor.y.round() as i32 - grab.1;
    if let Some(work) = work_area_at(window, (x + win_w / 2, y + win_h - 1)) {
        y = placed_drag_y(y, win_h, work, sf, snap);
    }
    let scale = window.app_handle().state::<SettingsStore>().get().pet_scale;
    let (w, h) = window_size(scale);
    let win = ((w * sf).round() as i32, (h * sf).round() as i32);
    let top_pad = ((h - GRID_H * scale) * sf).round() as i32;
    let (x, y) = clamp_position((x, y), win, top_pad, &monitor_rects(window)?).unwrap_or((x, y));
    let pos = window.outer_position()?;
    if (x, y) != (pos.x, pos.y) {
        window.set_position(PhysicalPosition::new(x, y))?;
    }
    Ok(())
}

/// Clamps the window after a drag and remembers where it ended up.
pub fn save_position<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    let pos = window.outer_position()?;
    let scale = window.app_handle().state::<SettingsStore>().get().pet_scale;
    remember_position(window, (pos.x, pos.y), scale)
}

/// Clamps a window position for a pet size, moves the window if that changed
/// it, and saves it.
fn remember_position<R: Runtime>(
    window: &WebviewWindow<R>,
    pos: (i32, i32),
    scale: f64,
) -> tauri::Result<()> {
    let sf = window.scale_factor()?;
    let (w, h) = window_size(scale);
    let win = ((w * sf).round() as i32, (h * sf).round() as i32);
    let top_pad = ((h - GRID_H * scale) * sf).round() as i32;
    let (x, y) = clamp_position(pos, win, top_pad, &monitor_rects(window)?).unwrap_or(pos);
    if (x, y) != pos {
        window.set_position(PhysicalPosition::new(x, y))?;
    }
    window
        .app_handle()
        .state::<SettingsStore>()
        .update(|s| s.pet_position = Some([x, y]))
        .map_err(tauri::Error::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN: Rect = Rect {
        x: 0,
        y: 0,
        w: 1920,
        h: 1080,
    };
    const SIDE: Rect = Rect {
        x: 1920,
        y: 0,
        w: 1280,
        h: 1024,
    };

    #[test]
    fn window_size_leaves_room_for_bubble() {
        // Small enough that the window keeps its minimum width.
        assert_eq!(window_size(4.0), (220.0, 284.0));
        assert_eq!(window_size(5.0), (220.0, 310.0));
        assert_eq!(window_size(7.0), (280.0, 362.0));
    }

    #[test]
    fn pet_sits_left_of_the_window_middle() {
        // The canvas is centered; the pet's middle is 2 cells left of its own.
        assert_eq!(pet_center_x(4.0), 110.0 - 8.0);
        assert_eq!(pet_center_x(5.0), 110.0 - 10.0);
        assert_eq!(pet_center_x(7.0), 140.0 - 14.0);
    }

    #[test]
    fn position_inside_monitor_is_unchanged() {
        assert_eq!(
            clamp_position((100, 200), (220, 246), 150, &[MAIN]),
            Some((100, 200))
        );
    }

    #[test]
    fn position_past_right_edge_is_pulled_back() {
        assert_eq!(
            clamp_position((1800, 200), (220, 246), 150, &[MAIN]),
            Some((1700, 200))
        );
    }

    #[test]
    fn transparent_top_may_hang_off_screen() {
        assert_eq!(
            clamp_position((100, -400), (220, 246), 150, &[MAIN, SIDE]),
            None
        );
        assert_eq!(
            clamp_position((100, -100), (220, 246), 150, &[MAIN]),
            Some((100, -100))
        );
        assert_eq!(
            clamp_position((100, -200), (220, 246), 150, &[MAIN]),
            Some((100, -150))
        );
    }

    #[test]
    fn uses_the_monitor_under_the_bottom_center() {
        assert_eq!(
            clamp_position((3000, 700), (220, 246), 150, &[MAIN, SIDE]),
            Some((2980, 700))
        );
    }

    #[test]
    fn unplugged_monitor_gives_none() {
        assert_eq!(clamp_position((4000, 100), (220, 246), 150, &[MAIN]), None);
    }

    #[test]
    fn default_is_bottom_right_of_work_area() {
        let wa = Rect {
            x: 0,
            y: 25,
            w: 1920,
            h: 1000,
        };
        assert_eq!(default_position((220, 246), wa, 16), (1684, 779));
    }

    /// A 1440x900 screen with a 70 px Dock and the menu bar, the pet's window
    /// 200 px tall, so the ground is y = 630.
    const WORK: Rect = Rect {
        x: 0,
        y: 25,
        w: 1440,
        h: 805,
    };
    const WIN_H: i32 = 200;
    const GROUND: i32 = 630;

    #[test]
    fn drag_above_the_line_stays_where_the_cursor_put_it() {
        // One physical pixel past the snap line: held up, as a stroll would see it.
        assert_eq!(snap_drag_y(GROUND - 25, WIN_H, WORK, 1.0), GROUND - 25);
        assert_eq!(snap_drag_y(100, WIN_H, WORK, 1.0), 100);
    }

    #[test]
    fn drag_within_the_line_sits_on_the_work_area_bottom() {
        assert_eq!(snap_drag_y(GROUND - 24, WIN_H, WORK, 1.0), GROUND);
        assert_eq!(snap_drag_y(GROUND, WIN_H, WORK, 1.0), GROUND);
    }

    #[test]
    fn drag_into_the_dock_stays_on_the_work_area_bottom() {
        // Feet would be at y = 900, behind the Dock. The ground is not the
        // monitor's bottom.
        assert_eq!(snap_drag_y(700, WIN_H, WORK, 1.0), GROUND);
    }

    #[test]
    fn drag_snap_line_is_in_logical_pixels() {
        assert_eq!(snap_drag_y(GROUND - 48, WIN_H, WORK, 2.0), GROUND);
        assert_eq!(snap_drag_y(GROUND - 49, WIN_H, WORK, 2.0), GROUND - 49);
    }

    #[test]
    fn shift_keeps_the_cursor_height() {
        // On the line, and down in the Dock: either would stick, and neither
        // does while Shift is held.
        assert_eq!(
            placed_drag_y(GROUND - 24, WIN_H, WORK, 1.0, false),
            GROUND - 24
        );
        assert_eq!(placed_drag_y(700, WIN_H, WORK, 1.0, false), 700);
        assert_eq!(placed_drag_y(GROUND - 24, WIN_H, WORK, 1.0, true), GROUND);
    }
}
