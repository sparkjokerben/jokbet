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

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn IsSecureEventInputEnabled() -> u8;
}

/// True while some app has Secure Event Input on (password fields, Terminal's
/// Secure Keyboard Entry): key events then reach no event tap.
/// Call on the main thread.
pub fn secure_input_enabled() -> bool {
    unsafe { IsSecureEventInputEnabled() != 0 }
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

/// Marks the material view so it can be found again in the content view.
const GLASS_TAG: isize = 0x4A6F_6B31;

const VIBRANCY_POPOVER: isize = 6;
const BLENDING_BEHIND_WINDOW: isize = 0;
const STATE_ACTIVE: isize = 1;

pub fn set_glass(ns_window: *mut std::ffi::c_void, rect: Option<(f64, f64, f64, f64)>, radius: f64) {
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject};
    // SAFETY: the NSWindow is alive and this runs on the main thread.
    unsafe {
        let window = &*(ns_window as *const AnyObject);
        let content: *mut AnyObject = msg_send![window, contentView];
        if content.is_null() {
            return;
        }
        let existing = find_glass(content);
        let Some((x, y, w, h)) = rect else {
            if let Some(view) = existing {
                let _: () = msg_send![view, setHidden: true];
            }
            return;
        };

        let view = match existing {
            Some(view) => view,
            None => {
                let Some(class) = view_class() else { return };
                let frame: NSRect = bounds(content);
                let alloc: *mut AnyObject = msg_send![class, alloc];
                let view: *mut AnyObject = msg_send![alloc, initWithFrame: frame];
                if view.is_null() {
                    return;
                }
                let _: () = msg_send![view, setTag: GLASS_TAG];
                let is_vibrancy: objc2::runtime::Bool = match AnyClass::get(c"NSVisualEffectView")
                {
                    Some(class) => msg_send![view, isKindOfClass: class],
                    None => false.into(),
                };
                if is_vibrancy.as_bool() {
                    let _: () = msg_send![view, setMaterial: VIBRANCY_POPOVER];
                    let _: () = msg_send![view, setBlendingMode: BLENDING_BEHIND_WINDOW];
                    let _: () = msg_send![view, setState: STATE_ACTIVE];
                }
                // Under everything else, so the webview draws on top of it.
                let _: () = msg_send![content, insertSubview: view, atIndex: 0usize];
                view
            }
        };

        let frame = frame_in_view(
            (x, y, w, h),
            bounds(content).size.height,
            msg_send![content, isFlipped],
        );
        let _: () = msg_send![view, setFrame: frame];
        let has_radius: objc2::runtime::Bool =
            msg_send![view, respondsToSelector: objc2::sel!(setCornerRadius:),];
        if has_radius.as_bool() {
            let _: () = msg_send![view, setCornerRadius: radius];
        }
        let _: () = msg_send![view, setHidden: false];
    }
}

/// Turns a rectangle the page reports (logical pixels from the window's top
/// left) into the view's own frame, which measures from the bottom left unless
/// the view is flipped.
fn frame_in_view(rect: (f64, f64, f64, f64), height: f64, flipped: objc2::runtime::Bool) -> NSRect {
    let (x, y, w, h) = rect;
    let top = if flipped.as_bool() { y } else { height - (y + h) };
    NSRect::new(
        objc2_foundation::NSPoint::new(x, top),
        objc2_foundation::NSSize::new(w, h),
    )
}

/// The class of the material view for this system.
fn view_class() -> Option<&'static objc2::runtime::AnyClass> {
    use objc2::runtime::AnyClass;
    AnyClass::get(c"NSGlassEffectView").or_else(|| AnyClass::get(c"NSVisualEffectView"))
}

unsafe fn bounds(view: *mut objc2::runtime::AnyObject) -> NSRect {
    objc2::msg_send![view, bounds]
}

unsafe fn find_glass(content: *mut objc2::runtime::AnyObject) -> Option<*mut objc2::runtime::AnyObject> {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    let subviews: *mut AnyObject = msg_send![content, subviews];
    if subviews.is_null() {
        return None;
    }
    let count: usize = msg_send![subviews, count];
    for i in 0..count {
        let view: *mut AnyObject = msg_send![subviews, objectAtIndex: i];
        let tag: isize = msg_send![view, tag];
        if tag == GLASS_TAG {
            return Some(view);
        }
    }
    None
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
