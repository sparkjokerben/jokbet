use crate::engine::distance::Display;
use x11rb::connection::Connection;
use x11rb::protocol::randr::ConnectionExt as _;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _};
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

/// Asks the window manager (EWMH) whether the active window is fullscreen.
#[derive(Default)]
pub struct FullscreenX11 {
    conn: Option<(RustConnection, u32, [u32; 3])>,
}

impl FullscreenX11 {
    pub fn active(&mut self) -> bool {
        if self.conn.is_none() {
            self.conn = connect_ewmh();
        }
        let Some((conn, root, [active, state, fullscreen])) = &self.conn else {
            return false;
        };
        let window = conn
            .get_property(false, *root, *active, AtomEnum::WINDOW, 0, 1)
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|r| r.value32().and_then(|mut v| v.next()));
        let Some(window) = window.filter(|w| *w != 0) else {
            return false;
        };
        conn.get_property(false, window, *state, AtomEnum::ATOM, 0, 32)
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|r| r.value32().map(|mut v| v.any(|a| a == *fullscreen)))
            .unwrap_or(false)
    }
}

fn connect_ewmh() -> Option<(RustConnection, u32, [u32; 3])> {
    let (conn, screen) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots.get(screen)?.root;
    let atom = |name: &[u8]| {
        conn.intern_atom(false, name)
            .ok()?
            .reply()
            .ok()
            .map(|r| r.atom)
    };
    let atoms = [
        atom(b"_NET_ACTIVE_WINDOW")?,
        atom(b"_NET_WM_STATE")?,
        atom(b"_NET_WM_STATE_FULLSCREEN")?,
    ];
    Some((conn, root, atoms))
}
