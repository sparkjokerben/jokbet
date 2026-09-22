use crate::engine::distance::Display;

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
