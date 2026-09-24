use crate::engine::distance::Display;
use x11rb::connection::Connection;
use x11rb::protocol::randr::ConnectionExt as _;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, KeyButMask, Window};
use x11rb::rust_connection::RustConnection;

/// X11 outputs in root-window pixels, with RandR's physical size.
pub fn displays() -> Vec<Display> {
    query().unwrap_or_default()
}

fn query() -> Option<Vec<Display>> {
    let (conn, screen) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots.get(screen)?.root;
    let res = conn
        .randr_get_screen_resources_current(root)
        .ok()?
        .reply()
        .ok()?;
    let mut out = Vec::new();
    for crtc in res.crtcs {
        let Some(info) = conn
            .randr_get_crtc_info(crtc, res.config_timestamp)
            .ok()
            .and_then(|c| c.reply().ok())
        else {
            continue;
        };
        let Some(&output) = info.outputs.first() else {
            continue;
        };
        if info.mode == 0 {
            continue;
        }
        let mm = conn
            .randr_get_output_info(output, res.config_timestamp)
            .ok()
            .and_then(|c| c.reply().ok())
            .map_or(0.0, |o| f64::from(o.mm_width));
        out.push(Display::new(
            f64::from(info.x),
            f64::from(info.y),
            f64::from(info.width),
            f64::from(info.height),
            mm,
        ));
    }
    Some(out)
}

/// Reads the pointer's button mask from the X server.
#[derive(Default)]
pub struct PointerX11 {
    conn: Option<(RustConnection, u32)>,
}

impl PointerX11 {
    pub fn primary_pressed(&mut self) -> bool {
        if self.conn.is_none() {
            self.conn = x11rb::connect(None).ok().and_then(|(conn, screen)| {
                let root = conn.setup().roots.get(screen)?.root;
                Some((conn, root))
            });
        }
        let Some((conn, root)) = &self.conn else {
            return false;
        };
        match conn.query_pointer(*root).ok().and_then(|c| c.reply().ok()) {
            Some(r) => u16::from(r.mask) & u16::from(KeyButMask::BUTTON1) != 0,
            None => {
                self.conn = None; // reconnect next time
                false
            }
        }
    }
}

/// Whether the active window is full screen (`_NET_WM_STATE_FULLSCREEN`) on
/// the monitor holding the point, in root-window pixels.
pub fn fullscreen_covers(x: f64, y: f64) -> bool {
    active_fullscreen(x, y).unwrap_or(false)
}

fn active_fullscreen(x: f64, y: f64) -> Option<bool> {
    let (conn, screen) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots.get(screen)?.root;
    let atom = |name: &[u8]| -> Option<u32> {
        Some(conn.intern_atom(false, name).ok()?.reply().ok()?.atom)
    };
    let (active, state, fullscreen) = (
        atom(b"_NET_ACTIVE_WINDOW")?,
        atom(b"_NET_WM_STATE")?,
        atom(b"_NET_WM_STATE_FULLSCREEN")?,
    );
    let window: Window = conn
        .get_property(false, root, active, AtomEnum::WINDOW, 0, 1)
        .ok()?
        .reply()
        .ok()?
        .value32()?
        .next()?;
    if window == 0 {
        return Some(false);
    }
    let is_fullscreen = conn
        .get_property(false, window, state, AtomEnum::ATOM, 0, 64)
        .ok()?
        .reply()
        .ok()?
        .value32()?
        .any(|a| a == fullscreen);
    if !is_fullscreen {
        return Some(false);
    }
    // Only if it is on the pet's monitor: the window's box in root pixels.
    let geometry = conn.get_geometry(window).ok()?.reply().ok()?;
    let origin = conn
        .translate_coordinates(window, root, 0, 0)
        .ok()?
        .reply()
        .ok()?;
    let bounds = (
        f64::from(origin.dst_x),
        f64::from(origin.dst_y),
        f64::from(geometry.width),
        f64::from(geometry.height),
    );
    Some(super::contains(bounds, x, y))
}
