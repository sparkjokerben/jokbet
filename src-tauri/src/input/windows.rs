//! Windows low-level keyboard and mouse hooks on a dedicated thread with its
//! own message loop. No permission is needed.

use super::{keymap, EventSink, InputHandle, MouseButton, Permission, RawEvent};
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::mpsc;
use std::thread::JoinHandle;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, LLKHF_INJECTED, LLMHF_INJECTED, MSG,
    MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_QUIT,
    WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_XBUTTONDOWN, WM_XBUTTONUP,
};

/// Hook procedures are plain functions, so the sink lives in a static.
/// Set before the hooks are installed; never freed (it is tiny).
static SINK: AtomicPtr<EventSink> = AtomicPtr::new(std::ptr::null_mut());

fn sink() -> Option<&'static EventSink> {
    // SAFETY: the pointer is either null or a leaked Box that is never freed.
    unsafe { SINK.load(Ordering::Acquire).as_ref() }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let k = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        // Software-generated keystrokes (automation, remote tools) are not counted.
        if k.flags.0 & LLKHF_INJECTED.0 == 0 {
            let extended = k.flags.0 & LLKHF_EXTENDED.0 != 0;
            if let (Some(sink), Some(code)) =
                (sink(), keymap::windows(k.scanCode, extended, k.vkCode))
            {
                let msg = wparam.0 as u32;
                sink.send(RawEvent::Key {
                    code,
                    down: msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN,
                });
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let m = &*(lparam.0 as *const MSLLHOOKSTRUCT);
        if let (Some(sink), true) = (sink(), m.flags & LLMHF_INJECTED == 0) {
            let button = |button, down| RawEvent::Button { button, down };
            let ev = match wparam.0 as u32 {
                WM_MOUSEMOVE => Some(RawEvent::Move {
                    x: m.pt.x as f64,
                    y: m.pt.y as f64,
                }),
                WM_LBUTTONDOWN => Some(button(MouseButton::Left, true)),
                WM_LBUTTONUP => Some(button(MouseButton::Left, false)),
                WM_RBUTTONDOWN => Some(button(MouseButton::Right, true)),
                WM_RBUTTONUP => Some(button(MouseButton::Right, false)),
                WM_MBUTTONDOWN => Some(button(MouseButton::Middle, true)),
                WM_MBUTTONUP => Some(button(MouseButton::Middle, false)),
                WM_XBUTTONDOWN => Some(button(MouseButton::Other, true)),
                WM_XBUTTONUP => Some(button(MouseButton::Other, false)),
                WM_MOUSEWHEEL | WM_MOUSEHWHEEL => Some(RawEvent::Scroll { momentum: false }),
                _ => None,
            };
            if let Some(ev) = ev {
                sink.send(ev);
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

pub fn permission() -> Permission {
    Permission::NotRequired
}

pub fn request_permission() {}

struct WinHandle {
    thread_id: u32,
    thread: JoinHandle<()>,
}

impl InputHandle for WinHandle {
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

pub fn start(sink: EventSink) -> Result<Box<dyn InputHandle>, String> {
    SINK.store(Box::into_raw(Box::new(sink)), Ordering::Release);
    let (ready_tx, ready_rx) = mpsc::channel::<Result<u32, String>>();
    let thread = std::thread::Builder::new()
        .name("input-hook".into())
        .spawn(move || unsafe {
            let module = GetModuleHandleW(None).ok().map(|m| m.into());
            let keyboard = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0);
            let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module, 0);
            let (keyboard, mouse) = match (keyboard, mouse) {
                (Ok(k), Ok(m)) => (k, m),
                (k, m) => {
                    for h in [k, m].into_iter().flatten() {
                        let _ = UnhookWindowsHookEx(h);
                    }
                    let _ = ready_tx.send(Err("SetWindowsHookExW failed".into()));
                    return;
                }
            };
            let _ = ready_tx.send(Ok(GetCurrentThreadId()));

            // Low-level hooks are called on this thread while it pumps messages.
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {}

            let _ = UnhookWindowsHookEx(keyboard);
            let _ = UnhookWindowsHookEx(mouse);
        })
        .map_err(|e| e.to_string())?;
    let thread_id = ready_rx.recv().map_err(|e| e.to_string())??;
    Ok(Box::new(WinHandle { thread_id, thread }))
}
