//! The always-on-top transparent pet window: geometry, creation and placement.

use crate::settings::{PetSize, SettingsStore};
use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, Runtime, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

pub const PET_LABEL: &str = "pet";

/// Sprite grid in cells (see src/sprites/jokbet.ts).
const GRID_W: f64 = 24.0;
const GRID_H: f64 = 16.0;
/// Room above the sprite for the head counter and the hover bubble.
const TOP_PAD: f64 = 170.0;
const MIN_WIDTH: f64 = 220.0;
const EDGE_MARGIN: f64 = 16.0;

/// Logical window size for a pet size.
pub fn window_size(size: PetSize) -> (f64, f64) {
    let s = size.scale();
    ((GRID_W * s).max(MIN_WIDTH), GRID_H * s + TOP_PAD)
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

/// Bottom-right corner of a monitor's work area.
pub fn default_position(win: (i32, i32), work_area: Rect, margin: i32) -> (i32, i32) {
    (
        work_area.x + work_area.w - win.0 - margin,
        work_area.y + work_area.h - win.1,
    )
}

pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<WebviewWindow<R>> {
    let size = app.state::<SettingsStore>().get().pet_size;
    let (w, h) = window_size(size);
    let window = WebviewWindowBuilder::new(app, PET_LABEL, WebviewUrl::App("pet.html".into()))
        .title("jokerben-desktop-pet")
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
        .visible(false)
        .build()?;
    place(&window)?;
    window.show()?;
    Ok(window)
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
    let (w, h) = window_size(settings.get().pet_size);
    let win = ((w * sf).round() as i32, (h * sf).round() as i32);
    let top_pad = ((h - GRID_H * settings.get().pet_size.scale()) * sf).round() as i32;

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

/// Resizes the window for a new pet size, keeping the pet's feet in place.
pub fn apply_size<R: Runtime>(
    window: &WebviewWindow<R>,
    from: PetSize,
    to: PetSize,
) -> tauri::Result<()> {
    let sf = window.scale_factor()?;
    let pos = window.outer_position()?;
    let (w0, h0) = window_size(from);
    let (w1, h1) = window_size(to);
    let x = pos.x + ((w0 - w1) / 2.0 * sf).round() as i32;
    let y = pos.y + ((h0 - h1) * sf).round() as i32;
    window.set_size(LogicalSize::new(w1, h1))?;
    window.set_position(PhysicalPosition::new(x, y))?;
    save_position(window)
}

/// Clamps the window after a drag and remembers where it ended up.
pub fn save_position<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    let app = window.app_handle();
    let settings = app.state::<SettingsStore>();
    let pos = window.outer_position()?;
    let size = window.outer_size()?;
    let sf = window.scale_factor()?;
    let pet_size = settings.get().pet_size;
    let top_pad = ((window_size(pet_size).1 - GRID_H * pet_size.scale()) * sf).round() as i32;
    let win = (size.width as i32, size.height as i32);
    let clamped = clamp_position((pos.x, pos.y), win, top_pad, &monitor_rects(window)?);
    let (x, y) = match clamped {
        Some(p) => p,
        None => (pos.x, pos.y),
    };
    if (x, y) != (pos.x, pos.y) {
        window.set_position(PhysicalPosition::new(x, y))?;
    }
    settings
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
        assert_eq!(window_size(PetSize::Small), (220.0, 234.0));
        assert_eq!(window_size(PetSize::Medium), (220.0, 266.0));
        assert_eq!(window_size(PetSize::Large), (220.0, 298.0));
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
}
