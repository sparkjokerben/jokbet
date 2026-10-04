//! Windows raw input on a dedicated thread with its own message loop, through
//! a hidden window of its own. No permission is needed.
//!
//! The pet has to see input that is going somewhere else, a full-screen game
//! above all, and the low-level hooks this used to run cannot promise that.
//! The system silently removes a hook whose callback arrives late — the input
//! is passed on and the hook never runs again, with no way to ask — and a hook
//! is only called as one link in a chain, so a game's own hook that fails to
//! pass an event on hides it from every hook installed before it. Raw input
//! with `RIDEV_INPUTSINK` is the platform's own answer for watching input
//! while not in the foreground, and neither of those can touch it: the system
//! cannot turn the registration off, nothing else sits between it and the
//! device, and it keeps arriving while a game owns the display.
//!
//! What raw input does not carry is the hooks' "injected" flag, so input sent
//! by other software (automation, remote tools) is counted here, where the
//! hooks used to drop it.
//!
//! Keyboard reports carry the same scan code, E0 flag and virtual key the hook
//! structure did, so key names still come from `keymap::windows`. Mouse
//! reports carry clicks and wheel travel; cursor movement is read from the
//! cursor itself, which keeps mouse travel in the same pixels as before.

use super::{keymap, EventSink, InputHandle, MouseButton, Permission, RawEvent, LINES_PER_NOTCH};
use std::mem::size_of;
use std::sync::atomic::{AtomicPtr, AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread::JoinHandle;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::{
    GetRawInputData, RegisterRawInputDevices, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE,
    RAWINPUTDEVICE_FLAGS, RAWINPUTHEADER, RAWKEYBOARD, RAWMOUSE, RIDEV_INPUTSINK, RIDEV_REMOVE,
    RID_INPUT, RIM_TYPEKEYBOARD, RIM_TYPEMOUSE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW,
    PostThreadMessageW, RegisterClassW, SystemParametersInfoW, TranslateMessage, HWND_MESSAGE, MSG,
    SPI_GETWHEELSCROLLCHARS, SPI_GETWHEELSCROLLLINES, SYSTEM_PARAMETERS_INFO_ACTION,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WINDOW_EX_STYLE, WM_INPUT, WM_QUIT, WNDCLASSW, WS_POPUP,
};

/// Where hook-like callbacks are not plain functions here either: the window
/// procedure is one function, so the sink lives in a static. Set before the
/// window is created; never freed (it is tiny).
static SINK: AtomicPtr<EventSink> = AtomicPtr::new(std::ptr::null_mut());

fn sink() -> Option<&'static EventSink> {
    // SAFETY: the pointer is either null or a leaked Box that is never freed.
    unsafe { SINK.load(Ordering::Acquire).as_ref() }
}

/// One notch of a wheel's travel in `mouseData`.
const WHEEL_DELTA: f64 = 120.0;

/// How many lines a notch of the vertical wheel scrolls, and how many
/// characters one of the horizontal wheel does (as `f64` bits); a character
/// sideways counts as a line. Read from the system's settings as the window
/// goes up, so the message handler makes no calls for it.
static NOTCH_V: AtomicU64 = AtomicU64::new(0);
static NOTCH_H: AtomicU64 = AtomicU64::new(0);

fn lines_per_notch(action: SYSTEM_PARAMETERS_INFO_ACTION) -> f64 {
    let mut lines: u32 = 0;
    let read = unsafe {
        SystemParametersInfoW(
            action,
            0,
            Some(&mut lines as *mut u32 as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    // A page per notch (WHEEL_PAGESCROLL) has no number of lines.
    if read.is_err() || lines == u32::MAX {
        return LINES_PER_NOTCH;
    }
    f64::from(lines)
}

/// The mouse and keyboard top-level collections, of the desktop usage page.
const USAGE_PAGE_DESKTOP: u16 = 0x01;
const USAGE_KEYBOARD: u16 = 0x06;
const USAGE_MOUSE: u16 = 0x02;

/// Raw keyboard report flags (`RI_KEY_*`): a release, and the E0 prefix that
/// tells an extended key from the plain one of the same scan code.
const RI_KEY_BREAK: u16 = 0x0001;
const RI_KEY_E0: u16 = 0x0002;

/// Raw mouse report button flags (`RI_MOUSE_*`).
const RI_MOUSE_LEFT_BUTTON_DOWN: u16 = 0x0001;
const RI_MOUSE_LEFT_BUTTON_UP: u16 = 0x0002;
const RI_MOUSE_RIGHT_BUTTON_DOWN: u16 = 0x0004;
const RI_MOUSE_RIGHT_BUTTON_UP: u16 = 0x0008;
const RI_MOUSE_MIDDLE_BUTTON_DOWN: u16 = 0x0010;
const RI_MOUSE_MIDDLE_BUTTON_UP: u16 = 0x0020;
const RI_MOUSE_BUTTON_4_DOWN: u16 = 0x0040;
const RI_MOUSE_BUTTON_4_UP: u16 = 0x0080;
const RI_MOUSE_BUTTON_5_DOWN: u16 = 0x0100;
const RI_MOUSE_BUTTON_5_UP: u16 = 0x0200;
const RI_MOUSE_WHEEL: u16 = 0x0400;
const RI_MOUSE_HWHEEL: u16 = 0x0800;

/// One raw keyboard report as a key event; `None` for the events the mapping
/// exists to drop (the fake shift that rides in front of navigation keys).
fn key_event(k: &RAWKEYBOARD) -> Option<RawEvent> {
    let down = k.Flags & RI_KEY_BREAK == 0;
    let extended = k.Flags & RI_KEY_E0 != 0;
    keymap::windows(u32::from(k.MakeCode), extended, u32::from(k.VKey))
        .map(|code| RawEvent::Key { code, down })
}

/// How many lines a wheel report moved: `data` is the signed travel in
/// `WHEEL_DELTA` units, and `notch` what one notch of the wheel is worth.
fn wheel_lines(data: u16, notch: f64) -> f64 {
    f64::from((data as i16).unsigned_abs()) / WHEEL_DELTA * notch
}

/// Sends what one raw mouse report carries: its buttons, its wheel travel,
/// and — where the report says the mouse moved — where the cursor is now,
/// which is the same measurement the hooks took.
fn send_mouse(sink: &EventSink, m: &RAWMOUSE, notch_v: f64, notch_h: f64) {
    // SAFETY: the button fields of the union are the ones every raw mouse
    // report fills (the other arm is the same bits as one number).
    let (buttons, data) = unsafe {
        (
            m.Anonymous.Anonymous.usButtonFlags,
            m.Anonymous.Anonymous.usButtonData,
        )
    };
    for (flag, button, down) in [
        (RI_MOUSE_LEFT_BUTTON_DOWN, MouseButton::Left, true),
        (RI_MOUSE_LEFT_BUTTON_UP, MouseButton::Left, false),
        (RI_MOUSE_RIGHT_BUTTON_DOWN, MouseButton::Right, true),
        (RI_MOUSE_RIGHT_BUTTON_UP, MouseButton::Right, false),
        (RI_MOUSE_MIDDLE_BUTTON_DOWN, MouseButton::Middle, true),
        (RI_MOUSE_MIDDLE_BUTTON_UP, MouseButton::Middle, false),
        (RI_MOUSE_BUTTON_4_DOWN, MouseButton::Other, true),
        (RI_MOUSE_BUTTON_4_UP, MouseButton::Other, false),
        (RI_MOUSE_BUTTON_5_DOWN, MouseButton::Other, true),
        (RI_MOUSE_BUTTON_5_UP, MouseButton::Other, false),
    ] {
        if buttons & flag != 0 {
            sink.send(RawEvent::Button { button, down });
        }
    }
    for (flag, notch) in [(RI_MOUSE_WHEEL, notch_v), (RI_MOUSE_HWHEEL, notch_h)] {
        if buttons & flag != 0 {
            sink.send(RawEvent::Scroll {
                lines: wheel_lines(data, notch),
                momentum: false,
            });
        }
    }
    if m.lLastX != 0 || m.lLastY != 0 {
        let mut point = POINT::default();
        if unsafe { GetCursorPos(&mut point) }.is_ok() {
            sink.send(RawEvent::Move {
                x: f64::from(point.x),
                y: f64::from(point.y),
            });
        }
    }
}

/// Reads one raw input message and hands what it carries to the sink.
fn receive(lparam: LPARAM) {
    let Some(sink) = sink() else { return };
    // Zeroed, so the whole structure is initialized even where the system
    // writes nothing: only the header and the report for the type in it are
    // ever read.
    let mut raw = RAWINPUT::default();
    let mut size = size_of::<RAWINPUT>() as u32;
    let read = unsafe {
        GetRawInputData(
            HRAWINPUT(lparam.0 as *mut _),
            RID_INPUT,
            Some(std::ptr::from_mut(&mut raw).cast()),
            &mut size,
            size_of::<RAWINPUTHEADER>() as u32,
        )
    };
    // Only the mouse and the keyboard are registered, so the buffer is always
    // the right size for what comes back.
    if read == 0 || read as usize > size_of::<RAWINPUT>() {
        return;
    }
    if raw.header.dwType == RIM_TYPEKEYBOARD.0 {
        // SAFETY: `dwType` says the union holds a keyboard report.
        if let Some(ev) = key_event(unsafe { &raw.data.keyboard }) {
            sink.send(ev);
        }
    } else if raw.header.dwType == RIM_TYPEMOUSE.0 {
        // SAFETY: `dwType` says the union holds a mouse report.
        send_mouse(
            sink,
            unsafe { &raw.data.mouse },
            f64::from_bits(NOTCH_V.load(Ordering::Relaxed)),
            f64::from_bits(NOTCH_H.load(Ordering::Relaxed)),
        );
    }
}

/// The class of the hidden window the raw input arrives at.
const INPUT_CLASS: PCWSTR = w!("JokbetRawInput");

unsafe extern "system" fn input_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_INPUT {
        receive(lparam);
    }
    // WM_INPUT still has to reach the default procedure, which is where the
    // system does its own cleanup for it.
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

/// The keyboard and the mouse registered (or, for `RIDEV_REMOVE`, released)
/// for raw input aimed at `window`.
fn register(window: HWND, flags: RAWINPUTDEVICE_FLAGS) -> Result<(), String> {
    let devices = [
        RAWINPUTDEVICE {
            usUsagePage: USAGE_PAGE_DESKTOP,
            usUsage: USAGE_KEYBOARD,
            dwFlags: flags,
            hwndTarget: window,
        },
        RAWINPUTDEVICE {
            usUsagePage: USAGE_PAGE_DESKTOP,
            usUsage: USAGE_MOUSE,
            dwFlags: flags,
            hwndTarget: window,
        },
    ];
    unsafe { RegisterRawInputDevices(&devices, size_of::<RAWINPUTDEVICE>() as u32) }
        .map_err(|e| format!("RegisterRawInputDevices failed: {e}"))
}

struct RawHandle {
    thread_id: u32,
    thread: JoinHandle<()>,
}

impl InputHandle for RawHandle {
    fn check_health(&self) -> bool {
        !self.thread.is_finished()
    }

    fn stop(self: Box<Self>) {
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        let _ = self.thread.join();
    }
}

pub fn permission() -> Permission {
    Permission::NotRequired
}

pub fn request_permission() {}

pub fn start(sink: EventSink) -> Result<Box<dyn InputHandle>, String> {
    SINK.store(Box::into_raw(Box::new(sink)), Ordering::Release);
    NOTCH_V.store(
        lines_per_notch(SPI_GETWHEELSCROLLLINES).to_bits(),
        Ordering::Relaxed,
    );
    NOTCH_H.store(
        lines_per_notch(SPI_GETWHEELSCROLLCHARS).to_bits(),
        Ordering::Relaxed,
    );
    let (ready_tx, ready_rx) = mpsc::channel::<Result<u32, String>>();
    let thread = std::thread::Builder::new()
        .name("input-raw".into())
        .spawn(move || {
            let instance = unsafe { GetModuleHandleW(None) }.ok().map(Into::into);
            // The class is made once; a second start only hears that it is
            // already registered, which is what is wanted.
            let _ = unsafe {
                RegisterClassW(&WNDCLASSW {
                    lpfnWndProc: Some(input_proc),
                    hInstance: instance.unwrap_or_default(),
                    lpszClassName: INPUT_CLASS,
                    ..Default::default()
                })
            };
            // A window of the message-only kind: it is never shown, never in
            // the taskbar, and nowhere near anything the user sees.
            let window = match unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    INPUT_CLASS,
                    PCWSTR::null(),
                    WS_POPUP,
                    0,
                    0,
                    0,
                    0,
                    Some(HWND_MESSAGE),
                    None,
                    instance,
                    None,
                )
            } {
                Ok(window) => window,
                Err(e) => {
                    let _ = ready_tx.send(Err(format!("CreateWindowExW failed: {e}")));
                    return;
                }
            };
            if let Err(e) = register(window, RIDEV_INPUTSINK) {
                unsafe {
                    let _ = DestroyWindow(window);
                };
                let _ = ready_tx.send(Err(e));
                return;
            }
            let _ = ready_tx.send(Ok(unsafe { GetCurrentThreadId() }));

            // Raw input is posted here, and the loop is what lets it arrive.
            let mut msg = MSG::default();
            while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
                unsafe {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            // Let go of the pair with the window, so a restart starts from
            // nothing rather than from a registration pointing nowhere.
            let _ = register(HWND(std::ptr::null_mut()), RIDEV_REMOVE);
            unsafe {
                let _ = DestroyWindow(window);
            };
        })
        .map_err(|e| e.to_string())?;
    let thread_id = ready_rx.recv().map_err(|e| e.to_string())??;
    Ok(Box::new(RawHandle { thread_id, thread }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::unbounded;

    fn keyboard(make: u16, flags: u16, vkey: u16) -> RAWKEYBOARD {
        RAWKEYBOARD {
            MakeCode: make,
            Flags: flags,
            VKey: vkey,
            ..Default::default()
        }
    }

    #[test]
    fn raw_keyboard_reports_map_like_the_hooks_did() {
        assert_eq!(
            key_event(&keyboard(0x2A, 0, 0xA0)),
            Some(RawEvent::Key {
                code: "ShiftLeft",
                down: true
            })
        );
        assert_eq!(
            key_event(&keyboard(0x2A, RI_KEY_BREAK, 0xA0)),
            Some(RawEvent::Key {
                code: "ShiftLeft",
                down: false
            })
        );
        // E0 is what makes the enter beside the numpad the extended one.
        assert_eq!(
            key_event(&keyboard(0x1C, RI_KEY_E0, 0x0D)),
            Some(RawEvent::Key {
                code: "NumpadEnter",
                down: true
            })
        );
        // And the fake shift in front of navigation keys is not a press.
        assert_eq!(key_event(&keyboard(0x2A, RI_KEY_E0, 0xA0)), None);
    }

    #[test]
    fn wheel_travel_is_lines_by_the_system_setting() {
        assert_eq!(wheel_lines(120, 3.0), 3.0);
        assert_eq!(wheel_lines((-120i16) as u16, 1.0), 1.0);
        assert_eq!(wheel_lines(240, 5.0), 10.0);
        assert_eq!(wheel_lines(0, 3.0), 0.0);
    }

    fn mouse() -> RAWMOUSE {
        RAWMOUSE {
            usFlags: windows::Win32::UI::Input::MOUSE_MOVE_RELATIVE,
            ..Default::default()
        }
    }

    #[test]
    fn one_mouse_report_sends_its_click_and_its_wheel() {
        let (tx, rx) = unbounded();
        let sink = EventSink::new(tx);
        let mut m = mouse();
        m.Anonymous.Anonymous.usButtonFlags = RI_MOUSE_LEFT_BUTTON_DOWN | RI_MOUSE_WHEEL;
        m.Anonymous.Anonymous.usButtonData = 120;
        send_mouse(&sink, &m, 3.0, 1.0);

        let events: Vec<_> = rx.try_iter().map(|e| e.ev).collect();
        assert_eq!(
            events,
            vec![
                RawEvent::Button {
                    button: MouseButton::Left,
                    down: true
                },
                RawEvent::Scroll {
                    lines: 3.0,
                    momentum: false
                },
            ]
        );
    }

    #[test]
    fn a_report_without_buttons_moves_only_the_cursor() {
        let (tx, rx) = unbounded();
        let sink = EventSink::new(tx);
        let mut m = mouse();
        m.lLastX = 4;
        m.lLastY = -2;
        send_mouse(&sink, &m, 3.0, 1.0);

        let events: Vec<_> = rx.try_iter().map(|e| e.ev).collect();
        match events.as_slice() {
            [RawEvent::Move { x, y }] => {
                // Whatever the cursor's position is here, it came as a point.
                assert!(x.is_finite() && y.is_finite());
            }
            other => panic!("expected one move, got {other:?}"),
        }
    }
}
