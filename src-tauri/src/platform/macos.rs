use crate::engine::distance::Display;
use objc2_foundation::NSRect;

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGSize {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventSourceButtonState(state_id: i32, button: u32) -> bool;
    fn CGGetActiveDisplayList(max: u32, displays: *mut u32, count: *mut u32) -> i32;
    fn CGDisplayBounds(display: u32) -> CGRect;
    fn CGDisplayScreenSize(display: u32) -> CGSize;
    fn CGDisplayCreateUUIDFromDisplayID(display: u32) -> *mut std::ffi::c_void;
}

/// Active display ids with their bounds in global points.
fn display_bounds() -> Vec<(u32, CGRect)> {
    let mut ids = [0u32; 16];
    let mut count = 0u32;
    if unsafe { CGGetActiveDisplayList(ids.len() as u32, ids.as_mut_ptr(), &mut count) } != 0 {
        return Vec::new();
    }
    ids[..count as usize]
        .iter()
        .map(|&id| (id, unsafe { CGDisplayBounds(id) }))
        .collect()
}

/// Active displays in global points (the space CGEvent locations use).
pub fn displays() -> Vec<Display> {
    display_bounds()
        .into_iter()
        .map(|(id, b)| {
            let mm = unsafe { CGDisplayScreenSize(id) };
            Display::new(
                b.origin.x,
                b.origin.y,
                b.size.width,
                b.size.height,
                mm.width,
            )
        })
        .collect()
}

/// NSWindowCollectionBehavior bits.
const CAN_JOIN_ALL_SPACES: usize = 1 << 0;
const STATIONARY: usize = 1 << 4;
const IGNORES_CYCLE: usize = 1 << 6;
const FULL_SCREEN_AUXILIARY: usize = 1 << 8;

pub fn pin_to_all_spaces(ns_window: *mut std::ffi::c_void, full_screen: bool) {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    // SAFETY: `ns_window` is the live NSWindow of a Tauri window, used on the main thread.
    unsafe {
        let window = &*(ns_window as *const AnyObject);
        let behavior: usize = msg_send![window, collectionBehavior];
        let mut behavior = behavior | CAN_JOIN_ALL_SPACES | STATIONARY | IGNORES_CYCLE;
        if full_screen {
            behavior |= FULL_SCREEN_AUXILIARY;
        } else {
            behavior &= !FULL_SCREEN_AUXILIARY;
        }
        let _: () = msg_send![window, setCollectionBehavior: behavior];
    }
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> *mut std::ffi::c_void;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: *const std::ffi::c_void);
    fn CFUUIDCreateString(
        allocator: *const std::ffi::c_void,
        uuid: *const std::ffi::c_void,
    ) -> *mut std::ffi::c_void;
}

extern "C" {
    fn dlsym(
        handle: *mut std::ffi::c_void,
        symbol: *const std::ffi::c_char,
    ) -> *mut std::ffi::c_void;
}

/// `dlsym`'s handle for "wherever the symbol is loaded".
const RTLD_DEFAULT: *mut std::ffi::c_void = -2isize as *mut std::ffi::c_void;

const LIST_ON_SCREEN_ONLY: u32 = 1 << 0;
const LIST_EXCLUDE_DESKTOP: u32 = 1 << 4;
/// Ordinary app windows; menus, panels and the pet itself sit above it.
const NORMAL_LAYER: i64 = 0;

/// The type the window server gives a full-screen Space (an ordinary desktop
/// is 0).
const FULL_SCREEN_SPACE: i64 = 4;

/// The window server's calls for Spaces. They are private, so they are looked
/// up rather than linked: the app still starts if a macOS ever drops them.
struct SpaceCalls {
    main_connection: unsafe extern "C" fn() -> i32,
    copy_display_spaces: unsafe extern "C" fn(i32) -> *mut std::ffi::c_void,
}

fn space_calls() -> Option<&'static SpaceCalls> {
    static CALLS: std::sync::OnceLock<Option<SpaceCalls>> = std::sync::OnceLock::new();
    CALLS
        .get_or_init(|| {
            // SAFETY: the symbols are looked up by name and, when present, have
            // these signatures.
            unsafe {
                let main = dlsym(RTLD_DEFAULT, c"CGSMainConnectionID".as_ptr());
                let copy = dlsym(RTLD_DEFAULT, c"CGSCopyManagedDisplaySpaces".as_ptr());
                (!main.is_null() && !copy.is_null()).then(|| SpaceCalls {
                    main_connection: std::mem::transmute::<
                        *mut std::ffi::c_void,
                        unsafe extern "C" fn() -> i32,
                    >(main),
                    copy_display_spaces: std::mem::transmute::<
                        *mut std::ffi::c_void,
                        unsafe extern "C" fn(i32) -> *mut std::ffi::c_void,
                    >(copy),
                })
            }
        })
        .as_ref()
}

/// Whether the display is showing a full-screen Space; `None` where the window
/// server does not say.
///
/// This, not the shape of the frontmost window, is what full screen is: an app
/// with a toolbar (Mail, Edge) keeps it in a window of its own in front of the
/// full-screen one, and every app gets such a window while the title bar is
/// pulled down.
fn full_screen_space(display: u32) -> Option<bool> {
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject, Bool};
    use objc2_foundation::NSString;

    let calls = space_calls()?;
    let string_class = AnyClass::get(c"NSString")?;
    // SAFETY: the UUID, its string and the list are Core Foundation objects,
    // toll-free bridged to NSString, NSArray and NSDictionary, and released
    // once read.
    unsafe {
        let uuid = CGDisplayCreateUUIDFromDisplayID(display);
        if uuid.is_null() {
            return None;
        }
        let name = CFUUIDCreateString(std::ptr::null(), uuid);
        CFRelease(uuid);
        if name.is_null() {
            return None;
        }
        let this = (*(name as *const NSString)).to_string();
        CFRelease(name);

        let list = (calls.copy_display_spaces)((calls.main_connection)());
        if list.is_null() {
            return None;
        }
        let (id_key, current_key, type_key) = (
            NSString::from_str("Display Identifier"),
            NSString::from_str("Current Space"),
            NSString::from_str("type"),
        );
        let entries = list as *mut AnyObject;
        let count: usize = msg_send![entries, count];
        let mut spaces = Vec::with_capacity(count);
        for i in 0..count {
            let entry: *mut AnyObject = msg_send![entries, objectAtIndex: i];
            let id: *mut AnyObject = msg_send![entry, objectForKey: &*id_key];
            let current: *mut AnyObject = msg_send![entry, objectForKey: &*current_key];
            if id.is_null() || current.is_null() {
                continue;
            }
            let is_string: Bool = msg_send![id, isKindOfClass: string_class];
            let kind: *mut AnyObject = msg_send![current, objectForKey: &*type_key];
            if !is_string.as_bool() || kind.is_null() {
                continue;
            }
            let kind: i64 = msg_send![kind, longLongValue];
            spaces.push(((*(id as *const NSString)).to_string(), kind));
        }
        CFRelease(list);
        current_space_is_full_screen(&spaces, &this)
    }
}

/// Picks the display's entry out of the window server's list of displays and
/// their current Spaces. When displays share their Spaces there is one entry,
/// named "Main", and it speaks for all of them.
fn current_space_is_full_screen(spaces: &[(String, i64)], display: &str) -> Option<bool> {
    let (_, kind) = spaces
        .iter()
        .find(|(id, _)| id.eq_ignore_ascii_case(display))
        .or(match spaces {
            [only] => Some(only),
            _ => None,
        })?;
    Some(*kind == FULL_SCREEN_SPACE)
}

/// Whether the display holding the point (global points) shows another app
/// full screen: a full-screen Space, or else a frontmost ordinary window of
/// another app that covers the whole display (a borderless game or a
/// presentation, which need no Space of their own).
///
/// Only a window's layer, bounds, owner and alpha are read, which needs no
/// Screen Recording permission (window titles would).
pub fn fullscreen_covers(x: f64, y: f64) -> bool {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    use objc2_foundation::NSString;

    let Some((id, display)) = display_bounds()
        .into_iter()
        .map(|(id, b)| (id, (b.origin.x, b.origin.y, b.size.width, b.size.height)))
        .find(|&(_, d)| super::contains(d, x, y))
    else {
        return false;
    };
    if full_screen_space(id) == Some(true) {
        return true;
    }
    // SAFETY: the list is a CFArray of CFDictionary, toll-free bridged to
    // NSArray and NSDictionary; it is released once read.
    unsafe {
        let list = CGWindowListCopyWindowInfo(LIST_ON_SCREEN_ONLY | LIST_EXCLUDE_DESKTOP, 0);
        if list.is_null() {
            return false;
        }
        let key = |name: &str| NSString::from_str(name);
        let (layer_key, bounds_key, owner_key, alpha_key) = (
            key("kCGWindowLayer"),
            key("kCGWindowBounds"),
            key("kCGWindowOwnerPID"),
            key("kCGWindowAlpha"),
        );
        let number = |dict: *mut AnyObject, k: &NSString| -> Option<f64> {
            let n: *mut AnyObject = msg_send![dict, objectForKey: k];
            (!n.is_null()).then(|| msg_send![n, doubleValue])
        };
        let (x_key, y_key, w_key, h_key) = (key("X"), key("Y"), key("Width"), key("Height"));
        let windows = list as *mut AnyObject;
        let count: usize = msg_send![windows, count];
        let me = f64::from(std::process::id());
        let mut covered = false;
        // Front to back: the first ordinary window on this display decides.
        for i in 0..count {
            let info: *mut AnyObject = msg_send![windows, objectAtIndex: i];
            if number(info, &layer_key) != Some(NORMAL_LAYER as f64)
                || number(info, &alpha_key).is_some_and(|a| a <= 0.0)
            {
                continue;
            }
            let bounds: *mut AnyObject = msg_send![info, objectForKey: &*bounds_key];
            if bounds.is_null() {
                continue;
            }
            let (Some(wx), Some(wy), Some(ww), Some(wh)) = (
                number(bounds, &x_key),
                number(bounds, &y_key),
                number(bounds, &w_key),
                number(bounds, &h_key),
            ) else {
                continue;
            };
            let window = (wx, wy, ww, wh);
            let overlaps = wx < display.0 + display.2
                && display.0 < wx + ww
                && wy < display.1 + display.3
                && display.1 < wy + wh;
            if !overlaps {
                continue;
            }
            covered = number(info, &owner_key) != Some(me) && super::covers(window, display);
            break;
        }
        CFRelease(list);
        covered
    }
}

/// The material behind the hover bubble.
///
/// The bubble is drawn by the page, so the material is a native view *behind*
/// the webview: an `NSGlassEffectView` on macOS 26 and later, an
/// `NSVisualEffectView` before that. It is clipped to the rectangle the page
/// reports, and hidden whenever the bubble is away.
pub fn glass() -> Option<crate::platform::Glass> {
    use objc2::runtime::AnyClass;
    let liquid = AnyClass::get(c"NSGlassEffectView").is_some();
    Some(if liquid {
        crate::platform::Glass::Liquid
    } else {
        crate::platform::Glass::Vibrancy
    })
}

const VIBRANCY_POPOVER: isize = 6;
const BLENDING_BEHIND_WINDOW: isize = 0;
const STATE_ACTIVE: isize = 1;
const WINDOW_BELOW: isize = -1;

pub fn set_glass(
    ns_window: *mut std::ffi::c_void,
    rect: Option<(f64, f64, f64, f64)>,
    radius: f64,
) {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    // SAFETY: the NSWindow is alive and this runs on the main thread.
    unsafe {
        let window = &*(ns_window as *const AnyObject);
        let content: *mut AnyObject = msg_send![window, contentView];
        if content.is_null() {
            return;
        }
        let Some(class) = view_class() else { return };
        let view = match find_glass(content, class) {
            Some(view) => view,
            None => match make_glass(content, class) {
                Some(view) => view,
                None => return,
            },
        };
        match rect {
            Some(rect) => {
                let frame = frame_in_view(rect, bounds(content).size.height, is_flipped(content));
                let _: () = msg_send![view, setFrame: frame];
                let has_radius: objc2::runtime::Bool =
                    msg_send![view, respondsToSelector: objc2::sel!(setCornerRadius:),];
                if has_radius.as_bool() {
                    let _: () = msg_send![view, setCornerRadius: radius];
                }
                let _: () = msg_send![view, setHidden: false];
            }
            // Away with the bubble it was behind.
            None => {
                let _: () = msg_send![view, setHidden: true];
            }
        }
    }
}

/// Builds the material view and puts it behind the page. `None` if the view
/// could not be made.
unsafe fn make_glass(
    content: *mut objc2::runtime::AnyObject,
    class: &objc2::runtime::AnyClass,
) -> Option<*mut objc2::runtime::AnyObject> {
    use objc2::msg_send;
    use objc2::runtime::{AnyObject, Bool};
    let view: *mut AnyObject = msg_send![class, alloc];
    let view: *mut AnyObject = msg_send![view, initWithFrame: bounds(content)];
    if view.is_null() {
        return None;
    }
    // A vibrancy view needs telling what to be; a glass view is already glass.
    let is_vibrancy: Bool = match objc2::runtime::AnyClass::get(c"NSVisualEffectView") {
        Some(veil) => msg_send![view, isKindOfClass: veil],
        None => false.into(),
    };
    if is_vibrancy.as_bool() {
        let _: () = msg_send![view, setMaterial: VIBRANCY_POPOVER];
        let _: () = msg_send![view, setBlendingMode: BLENDING_BEHIND_WINDOW];
        let _: () = msg_send![view, setState: STATE_ACTIVE];
    }
    // Behind whatever is already there (the page), so it draws on top of it.
    let subviews: *mut AnyObject = msg_send![content, subviews];
    let subview_count: usize = if subviews.is_null() {
        0
    } else {
        msg_send![subviews, count]
    };
    if subview_count == 0 {
        let _: () = msg_send![content, addSubview: view];
    } else {
        let page: *mut AnyObject = msg_send![subviews, objectAtIndex: 0usize];
        let _: () =
            msg_send![content, addSubview: view, positioned: WINDOW_BELOW, relativeTo: page];
    }
    Some(view)
}

/// The material view, if one has been made already. It cannot be tagged:
/// `NSGlassEffectView` has no `setTag:`, so it is known by its class.
unsafe fn find_glass(
    content: *mut objc2::runtime::AnyObject,
    class: &objc2::runtime::AnyClass,
) -> Option<*mut objc2::runtime::AnyObject> {
    use objc2::msg_send;
    use objc2::runtime::{AnyObject, Bool};
    let subviews: *mut AnyObject = msg_send![content, subviews];
    if subviews.is_null() {
        return None;
    }
    let count: usize = msg_send![subviews, count];
    for i in 0..count {
        let view: *mut AnyObject = msg_send![subviews, objectAtIndex: i];
        let is_glass: Bool = msg_send![view, isKindOfClass: class];
        if is_glass.as_bool() {
            return Some(view);
        }
    }
    None
}

/// The class of the material view for this system.
fn view_class() -> Option<&'static objc2::runtime::AnyClass> {
    use objc2::runtime::AnyClass;
    AnyClass::get(c"NSGlassEffectView").or_else(|| AnyClass::get(c"NSVisualEffectView"))
}

unsafe fn bounds(view: *mut objc2::runtime::AnyObject) -> NSRect {
    objc2::msg_send![view, bounds]
}

unsafe fn is_flipped(view: *mut objc2::runtime::AnyObject) -> objc2::runtime::Bool {
    objc2::msg_send![view, isFlipped]
}

/// Turns a rectangle the page reports (logical pixels from the window's top
/// left) into the view's own frame, which measures from the bottom left unless
/// the view is flipped.
fn frame_in_view(rect: (f64, f64, f64, f64), height: f64, flipped: objc2::runtime::Bool) -> NSRect {
    let (x, y, w, h) = rect;
    let top = if flipped.as_bool() {
        y
    } else {
        height - (y + h)
    };
    NSRect::new(
        objc2_foundation::NSPoint::new(x, top),
        objc2_foundation::NSSize::new(w, h),
    )
}

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn LMGetKbdType() -> u8;
    fn KBGetLayoutType(keyboard_type: i16) -> u32;
}

const KEYBOARD_ISO: u32 = u32::from_be_bytes(*b"ISO ");
const KEYBOARD_ANSI: u32 = u32::from_be_bytes(*b"ANSI");
const KEYBOARD_JIS: u32 = u32::from_be_bytes(*b"JIS ");

/// The physical layout of the keyboard last typed on. JIS is drawn as ANSI.
pub fn keyboard_kind() -> &'static str {
    // SAFETY: plain queries of the current keyboard type.
    match unsafe { KBGetLayoutType(i16::from(LMGetKbdType())) } {
        KEYBOARD_ISO => "iso",
        KEYBOARD_ANSI | KEYBOARD_JIS => "ansi",
        _ => "unknown",
    }
}

const COMBINED_SESSION_STATE: i32 = 0;
const LEFT_BUTTON: u32 = 0;

pub fn primary_button_pressed() -> bool {
    unsafe { CGEventSourceButtonState(COMBINED_SESSION_STATE, LEFT_BUTTON) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_rects_are_flipped_for_appkit_views() {
        // A bubble 20 px from the top of a 300 px tall window, 40 px tall.
        let frame = frame_in_view((10.0, 20.0, 100.0, 40.0), 300.0, false.into());
        assert_eq!(frame.origin.x, 10.0);
        assert_eq!(frame.origin.y, 300.0 - 60.0);
        assert_eq!(frame.size.width, 100.0);
        assert_eq!(frame.size.height, 40.0);
        // A flipped view measures from the top, like the page.
        let frame = frame_in_view((10.0, 20.0, 100.0, 40.0), 300.0, true.into());
        assert_eq!(frame.origin.y, 20.0);
    }

    fn spaces(entries: &[(&str, i64)]) -> Vec<(String, i64)> {
        entries
            .iter()
            .map(|&(id, kind)| (id.to_owned(), kind))
            .collect()
    }

    #[test]
    fn each_display_answers_for_its_own_space() {
        let list = spaces(&[("AAAA-1", 0), ("BBBB-2", FULL_SCREEN_SPACE)]);
        assert_eq!(current_space_is_full_screen(&list, "aaaa-1"), Some(false));
        assert_eq!(current_space_is_full_screen(&list, "BBBB-2"), Some(true));
        assert_eq!(current_space_is_full_screen(&list, "CCCC-3"), None);
    }

    #[test]
    fn shared_spaces_answer_for_every_display() {
        let list = spaces(&[("Main", FULL_SCREEN_SPACE)]);
        assert_eq!(current_space_is_full_screen(&list, "AAAA-1"), Some(true));
        assert_eq!(current_space_is_full_screen(&[], "AAAA-1"), None);
    }
}
