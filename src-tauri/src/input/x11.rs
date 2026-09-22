//! Linux X11 input via the RECORD extension: one connection controls the
//! recording context, a second one streams the recorded device events.
//! Wayland sessions do not allow global input listening at all.

use super::{keymap, EventSink, InputHandle, MouseButton, Permission, RawEvent};
use std::sync::mpsc;
use std::thread::JoinHandle;
use x11rb::connection::Connection;
use x11rb::protocol::record::{self, ConnectionExt as _, CS};
use x11rb::rust_connection::RustConnection;

const KEY_PRESS: u8 = 2;
const KEY_RELEASE: u8 = 3;
const BUTTON_PRESS: u8 = 4;
const BUTTON_RELEASE: u8 = 5;
const MOTION_NOTIFY: u8 = 6;
/// Recorded data "from server" (as opposed to start/end-of-data markers).
const FROM_SERVER: u8 = 0;

pub fn is_wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE").is_ok_and(|t| t.eq_ignore_ascii_case("wayland"))
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

pub fn permission() -> Permission {
    if is_wayland() {
        Permission::Unsupported
    } else {
        Permission::NotRequired
    }
}

pub fn request_permission() {}

/// Decodes one 32-byte core event. X11 auto-repeat arrives as a release
/// immediately followed by a press with the same timestamp; `held_release`
/// holds a release back until the next event shows whether it was one.
struct Decoder {
    held_release: Option<(u8, u32)>,
}

impl Decoder {
    fn feed(&mut self, ev: &[u8], out: &mut impl FnMut(RawEvent)) {
        if ev.len() < 32 {
            return;
        }
        let kind = ev[0] & 0x7f;
        let detail = ev[1];
        let time = u32::from_ne_bytes([ev[4], ev[5], ev[6], ev[7]]);

        if let Some((code, t)) = self.held_release.take() {
            if kind == KEY_PRESS && detail == code && time == t {
                return; // auto-repeat pair: drop both halves
            }
            out(key(code, false));
        }
        match kind {
            KEY_PRESS => out(key(detail, true)),
            KEY_RELEASE => self.held_release = Some((detail, time)),
            BUTTON_PRESS | BUTTON_RELEASE => {
                let down = kind == BUTTON_PRESS;
                match detail {
                    1 => out(RawEvent::Button {
                        button: MouseButton::Left,
                        down,
                    }),
                    2 => out(RawEvent::Button {
                        button: MouseButton::Middle,
                        down,
                    }),
                    3 => out(RawEvent::Button {
                        button: MouseButton::Right,
                        down,
                    }),
                    // Buttons 4-7 are wheel steps; each step sends press + release.
                    4..=7 if down => out(RawEvent::Scroll { momentum: false }),
                    4..=7 => {}
                    _ => out(RawEvent::Button {
                        button: MouseButton::Other,
                        down,
                    }),
                }
            }
            MOTION_NOTIFY => {
                let x = i16::from_ne_bytes([ev[20], ev[21]]);
                let y = i16::from_ne_bytes([ev[22], ev[23]]);
                out(RawEvent::Move {
                    x: x as f64,
                    y: y as f64,
                });
            }
            _ => {}
        }
    }
}

fn key(x11_keycode: u8, down: bool) -> RawEvent {
    RawEvent::Key {
        code: keymap::evdev(u32::from(x11_keycode).saturating_sub(8)),
        down,
    }
}

struct X11Handle {
    ctrl: RustConnection,
    context: record::Context,
    thread: JoinHandle<()>,
}

impl InputHandle for X11Handle {
    fn check_health(&self) -> bool {
        !self.thread.is_finished()
    }

    fn stop(self: Box<Self>) {
        let _ = self.ctrl.record_disable_context(self.context);
        let _ = self.ctrl.flush();
        let _ = self.thread.join();
        let _ = self.ctrl.record_free_context(self.context);
        let _ = self.ctrl.flush();
    }
}

pub fn start(sink: EventSink) -> Result<Box<dyn InputHandle>, String> {
    if is_wayland() {
        return Err("Wayland does not allow global input listening".into());
    }
    let err = |e: &dyn std::fmt::Display| e.to_string();
    let (ctrl, _) = x11rb::connect(None).map_err(|e| err(&e))?;
    let (data, _) = x11rb::connect(None).map_err(|e| err(&e))?;
    ctrl.record_query_version(1, 13)
        .map_err(|e| err(&e))?
        .reply()
        .map_err(|e| err(&e))?;

    let context = ctrl.generate_id().map_err(|e| err(&e))?;
    let range = record::Range {
        device_events: record::Range8 {
            first: KEY_PRESS,
            last: MOTION_NOTIFY,
        },
        ..Default::default()
    };
    ctrl.record_create_context(context, 0, &[CS::ALL_CLIENTS.into()], &[range])
        .map_err(|e| err(&e))?
        .check()
        .map_err(|e| err(&e))?;

    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
    let thread = std::thread::Builder::new()
        .name("input-record".into())
        .spawn(move || {
            let replies = match data.record_enable_context(context) {
                Ok(cookie) => cookie,
                Err(e) => {
                    let _ = ready_tx.send(Err(e.to_string()));
                    return;
                }
            };
            let _ = ready_tx.send(Ok(()));
            let mut decoder = Decoder { held_release: None };
            let mut emit = |ev| sink.send(ev);
            // Ends when the context is disabled from the control connection.
            for reply in replies {
                let Ok(reply) = reply else { break };
                if reply.category != FROM_SERVER {
                    continue;
                }
                for ev in reply.data.chunks_exact(32) {
                    decoder.feed(ev, &mut emit);
                }
            }
        })
        .map_err(|e| e.to_string())?;
    ready_rx.recv().map_err(|e| e.to_string())??;
    Ok(Box::new(X11Handle {
        ctrl,
        context,
        thread,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(kind: u8, detail: u8, time: u32) -> [u8; 32] {
        let mut e = [0u8; 32];
        e[0] = kind;
        e[1] = detail;
        e[4..8].copy_from_slice(&time.to_ne_bytes());
        e
    }

    fn decode(events: &[[u8; 32]]) -> Vec<RawEvent> {
        let mut d = Decoder { held_release: None };
        let mut out = Vec::new();
        for e in events {
            d.feed(e, &mut |ev| out.push(ev));
        }
        out
    }

    #[test]
    fn auto_repeat_pairs_are_dropped() {
        // 'a' is X keycode 38 (evdev 30).
        let out = decode(&[
            event(KEY_PRESS, 38, 100),
            event(KEY_RELEASE, 38, 600),
            event(KEY_PRESS, 38, 600),
            event(KEY_RELEASE, 38, 633),
            event(KEY_PRESS, 38, 633),
            event(KEY_RELEASE, 38, 700),
            event(MOTION_NOTIFY, 0, 800),
        ]);
        let keys: Vec<_> = out
            .iter()
            .filter_map(|e| match e {
                RawEvent::Key { code, down } => Some((*code, *down)),
                _ => None,
            })
            .collect();
        assert_eq!(keys, [("KeyA", true), ("KeyA", false)]);
    }

    #[test]
    fn real_second_press_is_kept() {
        let out = decode(&[
            event(KEY_PRESS, 38, 100),
            event(KEY_RELEASE, 38, 150),
            event(KEY_PRESS, 38, 300),
        ]);
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn wheel_steps_become_scroll_events() {
        let out = decode(&[
            event(BUTTON_PRESS, 4, 1),
            event(BUTTON_RELEASE, 4, 1),
            event(BUTTON_PRESS, 1, 2),
        ]);
        assert_eq!(
            out,
            [
                RawEvent::Scroll { momentum: false },
                RawEvent::Button {
                    button: MouseButton::Left,
                    down: true
                }
            ]
        );
    }

    #[test]
    fn motion_reads_root_coordinates() {
        let mut e = event(MOTION_NOTIFY, 0, 5);
        e[20..22].copy_from_slice(&1234i16.to_ne_bytes());
        e[22..24].copy_from_slice(&(-56i16).to_ne_bytes());
        assert_eq!(
            decode(&[e]),
            [RawEvent::Move {
                x: 1234.0,
                y: -56.0
            }]
        );
    }
}
