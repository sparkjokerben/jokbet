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
}

/// Active displays in global points (the space CGEvent locations use).
pub fn displays() -> Vec<Display> {
    let mut ids = [0u32; 16];
    let mut count = 0u32;
    if unsafe { CGGetActiveDisplayList(ids.len() as u32, ids.as_mut_ptr(), &mut count) } != 0 {
        return Vec::new();
    }
    ids[..count as usize]
        .iter()
        .map(|&id| {
            let b = unsafe { CGDisplayBounds(id) };
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

pub fn pin_to_all_spaces(ns_window: *mut std::ffi::c_void) {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    // SAFETY: `ns_window` is the live NSWindow of a Tauri window, used on the main thread.
    unsafe {
        let window = &*(ns_window as *const AnyObject);
        let behavior: usize = msg_send![window, collectionBehavior];
        let _: () = msg_send![
            window,
            setCollectionBehavior: behavior
                | CAN_JOIN_ALL_SPACES
                | STATIONARY
                | IGNORES_CYCLE
                | FULL_SCREEN_AUXILIARY
        ];
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
}
