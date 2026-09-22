use crate::engine::distance::Display;
use x11rb::connection::Connection;
use x11rb::protocol::randr::ConnectionExt as _;

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
