//! macOS listen-only CGEventTap on its own thread and CFRunLoop.
//! Requires the Input Monitoring permission.

use super::{keymap, EventSink, InputHandle, MouseButton, Permission, RawEvent};
use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;

type CFTypeRef = *const c_void;
type CFMachPortRef = *mut c_void;
type CFRunLoopRef = *mut c_void;
type CFRunLoopSourceRef = *mut c_void;
type CGEventRef = *mut c_void;
type CGEventTapProxy = *mut c_void;
type TapCallback = extern "C" fn(CGEventTapProxy, u32, CGEventRef, *mut c_void) -> CGEventRef;

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: TapCallback,
        user_info: *mut c_void,
    ) -> CFMachPortRef;
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    fn CGEventTapIsEnabled(tap: CFMachPortRef) -> bool;
    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    fn CGEventGetFlags(event: CGEventRef) -> u64;
    fn CGEventGetLocation(event: CGEventRef) -> CGPoint;
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFRunLoopCommonModes: CFTypeRef;
    fn CFMachPortCreateRunLoopSource(
        allocator: CFTypeRef,
        port: CFMachPortRef,
        order: isize,
    ) -> CFRunLoopSourceRef;
    fn CFMachPortInvalidate(port: CFMachPortRef);
    fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFTypeRef);
    fn CFRunLoopRun();
    fn CFRunLoopStop(rl: CFRunLoopRef);
    fn CFRelease(cf: CFTypeRef);
}

const SESSION_EVENT_TAP: u32 = 1;
const HEAD_INSERT: u32 = 0;
const LISTEN_ONLY: u32 = 1;

// CGEventType values.
const LEFT_DOWN: u32 = 1;
const LEFT_UP: u32 = 2;
const RIGHT_DOWN: u32 = 3;
const RIGHT_UP: u32 = 4;
const MOUSE_MOVED: u32 = 5;
const LEFT_DRAGGED: u32 = 6;
const RIGHT_DRAGGED: u32 = 7;
const KEY_DOWN: u32 = 10;
const KEY_UP: u32 = 11;
const FLAGS_CHANGED: u32 = 12;
const SCROLL_WHEEL: u32 = 22;
const OTHER_DOWN: u32 = 25;
const OTHER_UP: u32 = 26;
const OTHER_DRAGGED: u32 = 27;
const TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFF_FFFE;
const TAP_DISABLED_BY_USER_INPUT: u32 = 0xFFFF_FFFF;

// CGEventField values.
const FIELD_BUTTON_NUMBER: u32 = 3;
const FIELD_AUTOREPEAT: u32 = 8;
const FIELD_KEYCODE: u32 = 9;
const FIELD_MOMENTUM_PHASE: u32 = 123;

const CAPS_LOCK: u16 = 0x39;

fn event_mask() -> u64 {
    [
        LEFT_DOWN,
        LEFT_UP,
        RIGHT_DOWN,
        RIGHT_UP,
        MOUSE_MOVED,
        LEFT_DRAGGED,
        RIGHT_DRAGGED,
        KEY_DOWN,
        KEY_UP,
        FLAGS_CHANGED,
        SCROLL_WHEEL,
        OTHER_DOWN,
        OTHER_UP,
        OTHER_DRAGGED,
    ]
    .iter()
    .fold(0, |mask, t| mask | (1u64 << t))
}

/// Device-dependent flag bit that is set while a modifier key is held.
fn modifier_mask(keycode: u16) -> Option<u64> {
    Some(match keycode {
        0x3B => 0x0000_0001, // ControlLeft
        0x38 => 0x0000_0002, // ShiftLeft
        0x3C => 0x0000_0004, // ShiftRight
        0x37 => 0x0000_0008, // MetaLeft
        0x36 => 0x0000_0010, // MetaRight
        0x3A => 0x0000_0020, // AltLeft
        0x3D => 0x0000_0040, // AltRight
        0x3E => 0x0000_2000, // ControlRight
        0x3F => 0x0080_0000, // Fn (secondary fn)
        _ => return None,
    })
}

struct TapContext {
    sink: EventSink,
    port: AtomicPtr<c_void>,
}

extern "C" fn on_event(
    _proxy: CGEventTapProxy,
    event_type: u32,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef {
    // SAFETY: user_info is the TapContext kept alive by the tap thread.
    let ctx = unsafe { &*(user_info as *const TapContext) };
    let field = |f| unsafe { CGEventGetIntegerValueField(event, f) };
    let sink = &ctx.sink;
    match event_type {
        TAP_DISABLED_BY_TIMEOUT | TAP_DISABLED_BY_USER_INPUT => {
            let port = ctx.port.load(Ordering::Acquire);
            if !port.is_null() {
                unsafe { CGEventTapEnable(port, true) };
            }
        }
        KEY_DOWN | KEY_UP => {
            if event_type == KEY_UP || field(FIELD_AUTOREPEAT) == 0 {
                let code = keymap::mac(field(FIELD_KEYCODE) as u16);
                sink.send(RawEvent::Key {
                    code,
                    down: event_type == KEY_DOWN,
                });
            }
        }
        FLAGS_CHANGED => {
            let keycode = field(FIELD_KEYCODE) as u16;
            let code = keymap::mac(keycode);
            if keycode == CAPS_LOCK {
                // Caps Lock reports a toggle, not press/release; count each as a press.
                sink.send(RawEvent::Key { code, down: true });
                sink.send(RawEvent::Key { code, down: false });
            } else if let Some(mask) = modifier_mask(keycode) {
                let down = unsafe { CGEventGetFlags(event) } & mask != 0;
                sink.send(RawEvent::Key { code, down });
            }
        }
        LEFT_DOWN | LEFT_UP => sink.send(RawEvent::Button {
            button: MouseButton::Left,
            down: event_type == LEFT_DOWN,
        }),
        RIGHT_DOWN | RIGHT_UP => sink.send(RawEvent::Button {
            button: MouseButton::Right,
            down: event_type == RIGHT_DOWN,
        }),
        OTHER_DOWN | OTHER_UP => sink.send(RawEvent::Button {
            button: if field(FIELD_BUTTON_NUMBER) == 2 {
                MouseButton::Middle
            } else {
                MouseButton::Other
            },
            down: event_type == OTHER_DOWN,
        }),
        SCROLL_WHEEL => sink.send(RawEvent::Scroll {
            momentum: field(FIELD_MOMENTUM_PHASE) != 0,
        }),
        MOUSE_MOVED | LEFT_DRAGGED | RIGHT_DRAGGED | OTHER_DRAGGED => {
            let p = unsafe { CGEventGetLocation(event) };
            sink.send(RawEvent::Move { x: p.x, y: p.y });
        }
        _ => {}
    }
    event
}

pub fn permission() -> Permission {
    if unsafe { CGPreflightListenEventAccess() } {
        Permission::Granted
    } else {
        Permission::Denied
    }
}

/// Shows the system prompt the first time; later calls do nothing visible.
pub fn request_permission() {
    unsafe { CGRequestListenEventAccess() };
}

struct RunLoop(CFRunLoopRef);
// SAFETY: CFRunLoopStop may be called from any thread.
unsafe impl Send for RunLoop {}

struct MacHandle {
    ctx: Arc<TapContext>,
    run_loop: RunLoop,
    thread: JoinHandle<()>,
}

impl InputHandle for MacHandle {
    fn check_health(&self) -> bool {
        let port = self.ctx.port.load(Ordering::Acquire);
        if port.is_null() {
            return false;
        }
        unsafe {
            if !CGEventTapIsEnabled(port) {
                CGEventTapEnable(port, true);
            }
            CGEventTapIsEnabled(port)
        }
    }

    fn stop(self: Box<Self>) {
        unsafe { CFRunLoopStop(self.run_loop.0) };
        let _ = self.thread.join();
    }
}

pub fn start(sink: EventSink) -> Result<Box<dyn InputHandle>, String> {
    let ctx = Arc::new(TapContext {
        sink,
        port: AtomicPtr::new(ptr::null_mut()),
    });
    let (ready_tx, ready_rx) = mpsc::channel::<Result<RunLoop, String>>();
    let thread_ctx = ctx.clone();
    let thread = std::thread::Builder::new()
        .name("input-tap".into())
        .spawn(move || unsafe {
            let user_info = Arc::as_ptr(&thread_ctx) as *mut c_void;
            let port = CGEventTapCreate(
                SESSION_EVENT_TAP,
                HEAD_INSERT,
                LISTEN_ONLY,
                event_mask(),
                on_event,
                user_info,
            );
            if port.is_null() {
                let _ = ready_tx.send(Err(
                    "CGEventTapCreate failed (Input Monitoring not granted?)".into(),
                ));
                return;
            }
            thread_ctx.port.store(port, Ordering::Release);
            let source = CFMachPortCreateRunLoopSource(ptr::null(), port, 0);
            let run_loop = CFRunLoopGetCurrent();
            CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);
            CGEventTapEnable(port, true);
            let _ = ready_tx.send(Ok(RunLoop(run_loop)));

            CFRunLoopRun();

            thread_ctx.port.store(ptr::null_mut(), Ordering::Release);
            CGEventTapEnable(port, false);
            CFMachPortInvalidate(port);
            CFRelease(source as CFTypeRef);
            CFRelease(port as CFTypeRef);
            // thread_ctx drops here, after the tap can no longer call back.
        })
        .map_err(|e| e.to_string())?;
    let run_loop = ready_rx.recv().map_err(|e| e.to_string())??;
    Ok(Box::new(MacHandle {
        ctx,
        run_loop,
        thread,
    }))
}
