use crate::engine::distance::Display;
use std::sync::{Mutex, OnceLock};
use windows::core::{s, w, BOOL, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, EnumDisplayMonitors, GetDC, GetMonitorInfoW, GetPixel, ReleaseDC, CLR_INVALID,
    HDC, HMONITOR, MONITORINFO,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_RAW_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, GetSystemMetrics, GetWindowRect, IsWindowVisible, PostMessageW, SetWindowPos,
    ShowWindow, SM_SWAPBUTTON, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    SWP_SHOWWINDOW, SW_HIDE, WINDOWPOS, WM_APP, WM_ERASEBKGND, WM_WINDOWPOSCHANGED,
    WM_WINDOWPOSCHANGING,
};

unsafe extern "system" fn collect(monitor: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
    let out = &mut *(data.0 as *mut Vec<Display>);
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if GetMonitorInfoW(monitor, &mut info).as_bool() {
        let r = info.rcMonitor;
        let (w, h) = (f64::from(r.right - r.left), f64::from(r.bottom - r.top));
        let (mut dpi_x, mut dpi_y) = (0u32, 0u32);
        // Raw DPI comes from the monitor's EDID physical size.
        let width_mm = if GetDpiForMonitor(monitor, MDT_RAW_DPI, &mut dpi_x, &mut dpi_y).is_ok()
            && dpi_x > 0
        {
            w / f64::from(dpi_x) * 25.4
        } else {
            0.0
        };
        out.push(Display::new(
            f64::from(r.left),
            f64::from(r.top),
            w,
            h,
            width_mm,
        ));
    }
    true.into()
}

/// Puts the acrylic behind the bubble, or takes it away again (`None`).
/// `rect` is the bubble in physical pixels, client-area coordinates, and
/// `theme_dark` is whether the *system* is light or dark, which stands in for the
/// tone where the screen cannot be read at all.
///
/// Returns the tone: which way the panel leans, 0 for a card drawn with dark ink
/// over a pale panel and 1 for light ink over a dark one. The page draws the
/// card's ink, and how much of its own colour it carries, from that one number —
/// so nothing about the panel is a switch between two states that has to be
/// hidden when it throws.
///
/// Windows has no per-panel material: the acrylic covers a whole window, and
/// the pet's window is far larger than the card — the number above its head,
/// the ball it juggles and the width of the laptop all leave room that a slab
/// of acrylic under all of it would fill. So the material gets a window of its
/// own, cut to the card's shape and kept directly beneath the pet, which is
/// transparent and lets it show through.
pub fn set_glass(pet: HWND, rect: Option<(i32, i32, i32, i32)>, theme_dark: bool) -> f64 {
    *PET.lock().unwrap() = Some(pet.0 as isize);
    *BUBBLE.lock().unwrap() = rect.map(|rect| Bubble { rect, theme_dark });
    let tone = rect.map_or_else(
        || system_tone(theme_dark),
        |rect| {
            let mut tone = TONE.lock().unwrap();
            let next = shown_tone(*tone, || read_behind(pet, rect, theme_dark));
            *tone = Some(next);
            next
        },
    );
    show(pet, rect);
    tone
}

/// Reads behind the card again, and gives the tone it makes once that has moved
/// since the last reading — `None` while it has not, or while there is no card.
///
/// The page says where the card is once, when the cursor arrives on the pet; the
/// pet then moves without it — a drag runs in Windows' own move loop, which the
/// page never sees — so what is behind the material changes under a card the page
/// believes is still where it left it. Read as the pet is carried about, and
/// eased into the reading before it: a window edge passing under the pet is then
/// a drift, which the page draws as a drift.
///
/// Called from the hover thread, which is the thread still running while the pet
/// is dragged.
pub fn read_tone() -> Option<f64> {
    let (rect, theme_dark) = {
        let bubble = BUBBLE.lock().unwrap();
        let bubble = bubble.as_ref()?;
        (bubble.rect, bubble.theme_dark)
    };
    let pet = HWND((*PET.lock().unwrap())? as *mut _);
    let read = read_behind(pet, rect, theme_dark);
    let mut tone = TONE.lock().unwrap();
    let was = tone.unwrap_or(read);
    let next = eased(was, read);
    *tone = Some(next);
    ((next - was).abs() >= TONE_STEP).then_some(next)
}

/// The tone of what is behind the card, with the system's own light or dark
/// standing in where the screen cannot be read at all.
fn read_behind(pet: HWND, rect: (i32, i32, i32, i32), theme_dark: bool) -> f64 {
    behind_luma(pet, rect).map_or_else(|| system_tone(theme_dark), tone_of)
}

/// The tone the system's own light or dark asks for, where the screen says
/// nothing.
fn system_tone(theme_dark: bool) -> f64 {
    if theme_dark {
        1.0
    } else {
        0.0
    }
}

/// The tone a backdrop of luma `behind` makes: the whole of it across the band
/// where the two inks come to much the same contrast, and none of it outside,
/// where the panel is plainly one way or the other already.
fn tone_of(behind: f64) -> f64 {
    ((LEAN_LIGHT_AT - behind) / (LEAN_LIGHT_AT - LEAN_DARK_AT)).clamp(0.0, 1.0)
}

/// One reading eased into the one before it.
fn eased(was: f64, read: f64) -> f64 {
    was + (read - was) * TONE_EASE
}

/// What the panel wears as it comes up: what it wore when it last went away.
///
/// Coming back is then not a change of colour — the card appears as it left,
/// and where the desktop underneath has moved on in the meantime the reading
/// drifts to it from there, rather than the card arriving in a colour of its own
/// and then correcting itself in front of whoever is looking at it. Only the
/// first panel of a run has nothing to wear, and that one is read.
fn shown_tone(kept: Option<f64>, read: impl FnOnce() -> f64) -> f64 {
    kept.unwrap_or_else(read)
}

/// Shows the material for a bubble at `rect`, or takes it away (`None`).
fn show(pet: HWND, rect: Option<(i32, i32, i32, i32)>) {
    let Some((x, y, w, h)) = rect.and_then(|rect| on_screen(pet, rect)) else {
        if let Some(layer) = layer(pet) {
            unsafe {
                let _ = ShowWindow(layer, SW_HIDE);
            }
        }
        return;
    };
    let Some(layer) = layer(pet) else { return };
    unsafe {
        set_accent(layer, TINT);
        // How far the layer sits from the pet's own corner. The two are kept
        // together by this distance, so it is what the page's report really
        // means once the pet starts moving.
        let mut pet_rect = RECT::default();
        if GetWindowRect(pet, &mut pet_rect).is_ok() {
            *OFFSET.lock().unwrap() = Some((x - pet_rect.left, y - pet_rect.top));
        }
        // Directly under the pet, and out of the way of everything else.
        let _ = SetWindowPos(
            layer,
            Some(pet),
            x,
            y,
            w,
            h,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
    }
}

/// The cursor is on the pet or it is not, said by the thread that watches it.
///
/// The page says the same thing through the bubble it draws, but the page
/// cannot always be heard: the context menu holds the main thread for as long
/// as it is up, so everything the page asks for in that time waits. The hover
/// thread is not held up, and a posted message is one of the two things a menu
/// loop still dispatches — so the material follows the cursor through the menu
/// as well, rather than having to be taken away before it opens.
pub fn pet_hovered(on: bool) {
    let pet = *PET.lock().unwrap();
    if let Some(pet) = pet {
        unsafe {
            let _ = PostMessageW(
                Some(HWND(pet as *mut _)),
                WM_PET_HOVERED,
                WPARAM(usize::from(on)),
                LPARAM(0),
            );
        }
    }
}

/// Puts the material where the cursor says it belongs: up behind the bubble the
/// page last asked for while the cursor is on the pet, and away when it is not.
fn hover_changed(pet: HWND, on: bool) {
    match bubble_for(on, *BUBBLE.lock().unwrap()) {
        Some(rect) => show(pet, Some(rect)),
        None => show(pet, None),
    }
}

/// The bubble the material is to be shown behind, if any: the one the page last
/// asked for, and only while the cursor is on the pet.
fn bubble_for(on: bool, bubble: Option<Bubble>) -> Option<(i32, i32, i32, i32)> {
    match (on, bubble) {
        (true, Some(bubble)) => Some(bubble.rect),
        _ => None,
    }
}

/// The mean luma of what is on screen around the bubble: the desktop, or
/// whatever window is behind the pet there.
///
/// The sides the bubble itself covers are left out — the material is exactly its
/// rectangle, and the card's own colour says nothing about what is behind it.
/// What is sampled is above and beside the card, where the pet's window is bare.
fn behind_luma(pet: HWND, rect: (i32, i32, i32, i32)) -> Option<f64> {
    let (x, y, w, h) = on_screen(pet, rect)?;
    let screen = unsafe { GetDC(None) };
    if screen.is_invalid() {
        return None;
    }
    let points = [
        (x + w / 4, y - OUTSIDE),
        (x + w / 2, y - OUTSIDE),
        (x + 3 * w / 4, y - OUTSIDE),
        (x + w / 2, y - 2 * OUTSIDE),
        (x - OUTSIDE, y + 2 * h / 5),
        (x + w + OUTSIDE, y + 2 * h / 5),
        (x - OUTSIDE, y + 3 * h / 5),
        (x + w + OUTSIDE, y + 3 * h / 5),
    ];
    let (mut sum, mut taken) = (0.0, 0u32);
    for (px, py) in points {
        let colour = unsafe { GetPixel(screen, px, py) }.0;
        if colour == CLR_INVALID {
            continue;
        }
        let (r, g, b) = (
            (colour & 0xff) as f64,
            ((colour >> 8) & 0xff) as f64,
            ((colour >> 16) & 0xff) as f64,
        );
        sum += (r + g + b) / 3.0;
        taken += 1;
    }
    unsafe {
        ReleaseDC(None, screen);
    }
    (taken > 0).then(|| sum / f64::from(taken))
}

/// How far outside the card the desktop is sampled, in physical pixels.
const OUTSIDE: i32 = 12;

/// The bubble the page last asked for: where the card is, and what the system
/// said about its own light or dark, which is what reading the tone underneath
/// needs. Kept for the hover thread, which has no window API of its own to ask
/// and must not block on the one that is inside the menu.
#[derive(Clone, Copy)]
struct Bubble {
    rect: (i32, i32, i32, i32),
    theme_dark: bool,
}

/// What the material is showing, kept for the hover thread.
static BUBBLE: Mutex<Option<Bubble>> = Mutex::new(None);

/// The tone the material is wearing, which a panel coming up wears and the next
/// reading is eased into. It outlives the panel itself: taking the material away
/// leaves this where it was, so the next panel starts from it.
static TONE: Mutex<Option<f64>> = Mutex::new(None);

/// The pet's own window, kept for the hover thread, which has no window API of
/// its own to ask and must not block on the one that is inside the menu.
static PET: Mutex<Option<isize>> = Mutex::new(None);

/// Says the cursor has come onto the pet or gone off it.
const WM_PET_HOVERED: u32 = WM_APP + 1;

/// Where a client-area rectangle of the pet's window falls on the screen.
fn on_screen(pet: HWND, (x, y, w, h): (i32, i32, i32, i32)) -> Option<(i32, i32, i32, i32)> {
    let mut origin = POINT::default();
    unsafe { ClientToScreen(pet, &mut origin) }
        .as_bool()
        .then_some((origin.x + x, origin.y + y, w, h))
}

/// Keeps the material with the pet while it is dragged.
///
/// A drag runs inside Windows' own move loop, where nothing of ours runs: the
/// page cannot follow it, and a thread watching the window's position would at
/// best leave the material trailing behind the card. Every move is announced
/// first, though, and that is the chance to move the layer in the same breath,
/// so that the two never come apart.
unsafe extern "system" fn pet_proc(
    pet: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _subclass: usize,
    _data: usize,
) -> LRESULT {
    match message {
        // Where the pet is about to be. Only a message that moves it says
        // anything: one that does not has no position in it, and reads as
        // (0, 0) — taking that for a move would send the layer off to the
        // corner of the screen, which is what a panel that "flies about" is.
        WM_WINDOWPOSCHANGING if lparam.0 != 0 => {
            let pos = unsafe { &*(lparam.0 as *const WINDOWPOS) };
            if pos.flags.0 & SWP_NOMOVE.0 == 0 {
                if let Some(layer) = shown_layer(pet) {
                    unsafe { keep(layer, pos.x, pos.y) };
                }
            }
        }
        // And where it ended up. This is measured rather than guessed, so
        // whatever a move did to either window cannot leave the two apart.
        WM_WINDOWPOSCHANGED if lparam.0 != 0 => {
            if let Some(layer) = shown_layer(pet) {
                let mut now = RECT::default();
                if unsafe { GetWindowRect(pet, &mut now) }.is_ok() {
                    unsafe { keep(layer, now.left, now.top) };
                }
            }
        }
        // The cursor, from the thread that watches it rather than the page.
        WM_PET_HOVERED => hover_changed(pet, wparam.0 != 0),
        _ => {}
    }
    unsafe { DefSubclassProc(pet, message, wparam, lparam) }
}

/// Puts the layer where it belongs for a pet whose top-left is at `(x, y)`: as
/// far from that corner as the page last said the bubble was.
unsafe fn keep(layer: HWND, x: i32, y: i32) {
    let Some((dx, dy)) = *OFFSET.lock().unwrap() else {
        return;
    };
    let mut was = RECT::default();
    if GetWindowRect(layer, &mut was).is_err() || (was.left, was.top) == (x + dx, y + dy) {
        return;
    }
    let _ = SetWindowPos(
        layer,
        None,
        x + dx,
        y + dy,
        0,
        0,
        SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOZORDER,
    );
}

/// The acrylic material is the accent user32 keeps for a window. Nothing about
/// it is in the SDK — the call is not in any import library — so it is asked
/// for by name, and asking is also how this system is asked whether it has it.
type SetAccent = unsafe extern "system" fn(HWND, *mut AccentData) -> BOOL;

const WCA_ACCENT_POLICY: i32 = 19;
/// The acrylic of a flyout; Windows 10 1803 and later.
const ACCENT_ENABLE_ACRYLICBLURBEHIND: i32 = 4;

/// What the material is tinted by, as ABGR: a touch of neutral grey.
///
/// The acrylic has no colour of its own, so what shows through it is the desktop.
/// That is the material wanted — but a busy wallpaper under a card's small print
/// can leave it unreadable, so the veil is just enough to even one out: a neutral
/// grey lifts a dark desktop and settles a bright one, leaving the panel a pane
/// of glass rather than a slab of paint. Which way the panel then *leans* — how
/// much of its own colour it carries over that, and which ink is drawn on it —
/// follows the tone, and is the page's to draw.
const TINT: u32 = 0x33_80_80_80;

/// The backdrop luma the panel leans its darkest over, and the one it leans its
/// lightest over: the band the tone travels across. It is centred on the luma
/// where dark ink and light ink come to the same contrast, which is the point
/// the old single threshold sat at, so both inks read the same there.
const LEAN_DARK_AT: f64 = 93.0;
const LEAN_LIGHT_AT: f64 = 145.0;

/// How much of each reading the tone takes on: a slow hand, so that a window edge
/// passing under the pet is a drift rather than a step. At the hover thread's
/// poll this is about a sixth of a second.
const TONE_EASE: f64 = 0.2;

/// How far the tone has to move before the page is told. Far below what a card
/// can show, and well above the noise of a reading.
const TONE_STEP: f64 = 0.02;

#[repr(C)]
struct AccentPolicy {
    state: i32,
    flags: i32,
    gradient_color: u32,
    animation_id: i32,
}

#[repr(C)]
struct AccentData {
    attribute: i32,
    data: *mut std::ffi::c_void,
    size: usize,
}

fn accent() -> Option<SetAccent> {
    unsafe {
        let user32 = GetModuleHandleW(w!("user32.dll")).ok()?;
        let found = GetProcAddress(user32, s!("SetWindowCompositionAttribute"))?;
        Some(std::mem::transmute::<
            unsafe extern "system" fn() -> isize,
            SetAccent,
        >(found))
    }
}

/// Whether this system can put its acrylic behind the bubble.
pub fn acrylic() -> bool {
    accent().is_some()
}

/// Gives the layer its material.
///
/// Windows 11 has a documented backdrop for this, and it was tried first:
/// `DWMWA_SYSTEMBACKDROP_TYPE` with `DWMSBT_TRANSIENTWINDOW` (the acrylic of a
/// flyout). Measured over a black-and-white pattern, it comes out as an opaque
/// fill of its own that takes no colour at all from what is behind it — behind
/// a bubble that is a bright pane, not a material. The accent is the one that
/// shows the desktop through, so it is the one the bubble gets.
fn set_accent(layer: HWND, tint: u32) {
    let Some(set) = accent() else { return };
    let mut policy = AccentPolicy {
        state: ACCENT_ENABLE_ACRYLICBLURBEHIND,
        flags: 0,
        gradient_color: tint,
        animation_id: 0,
    };
    let mut data = AccentData {
        attribute: WCA_ACCENT_POLICY,
        data: std::ptr::from_mut(&mut policy).cast(),
        size: std::mem::size_of::<AccentPolicy>(),
    };
    unsafe {
        let _ = set(layer, &mut data);
    }
}

/// The window the acrylic is drawn in, made the first time a bubble asks for
/// it. `None` where the system has no accent to draw.
static LAYER: OnceLock<Option<isize>> = OnceLock::new();

/// How far the layer's top-left sits from the pet window's, in physical
/// pixels: the distance the page's report comes to once the pet is moved.
static OFFSET: Mutex<Option<(i32, i32)>> = Mutex::new(None);

/// The layer's own subclass id on the pet window's procedure.
const SUBCLASS: usize = 0x4a4b_0001;

fn layer(pet: HWND) -> Option<HWND> {
    LAYER
        .get_or_init(|| make_layer(pet).map(|layer| layer.0 as isize))
        .map(|layer| HWND(layer as *mut _))
}

/// The layer, if it is on screen at all: nothing else is worth moving.
fn shown_layer(pet: HWND) -> Option<HWND> {
    let layer = layer(pet)?;
    unsafe { IsWindowVisible(layer).as_bool().then_some(layer) }
}

/// The layer's window class: nothing is ever painted into it.
const LAYER_CLASS: PCWSTR = w!("JokbetGlassLayer");

fn make_layer(pet: HWND) -> Option<HWND> {
    use windows::Win32::Graphics::Dwm::{
        DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE,
        DWMWCP_ROUND,
    };
    use windows::Win32::UI::Controls::MARGINS;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, RegisterClassW, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
        WS_EX_TOPMOST, WS_POPUP,
    };
    if !acrylic() {
        return None;
    }
    let instance = unsafe { GetModuleHandleW(PCWSTR::null()) }
        .map(Into::into)
        .ok();
    unsafe {
        // This module loads once, so a second attempt at the class only says
        // it is already registered, which is what is wanted.
        let _ = RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(layer_proc),
            hInstance: instance.unwrap_or_default(),
            lpszClassName: LAYER_CLASS,
            ..Default::default()
        });
        // A tool window: out of the taskbar and Alt-Tab, and never focused.
        // Topmost keeps it in the pet's band, right under it.
        let layer = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
            LAYER_CLASS,
            PCWSTR::null(),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            instance,
            None,
        )
        .ok()?;
        // The whole window counts as frame, which leaves it see-through where
        // nothing is drawn into it. That is what the material shows through,
        // and it is also what a system without the accent falls back to: the
        // desktop, rather than an opaque pane.
        let margins = MARGINS {
            cxLeftWidth: -1,
            cxRightWidth: -1,
            cyTopHeight: -1,
            cyBottomHeight: -1,
        };
        let _ = DwmExtendFrameIntoClientArea(layer, &margins);
        // Round the corners as the card's own are round, so no square of
        // acrylic pokes out around it. This has to be DWM's doing: the accent
        // is drawn over the whole window and takes no notice of a window
        // region. A system without it gets the square corners its acrylic had
        // anyway.
        let round = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            layer,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            std::ptr::from_ref(&round).cast(),
            std::mem::size_of_val(&round) as u32,
        );
        // And a word with the pet before it moves, so the two stay together.
        let _ = SetWindowSubclass(pet, Some(pet_proc), SUBCLASS, 0);
        Some(layer)
    }
}

/// The layer draws nothing itself: the material is the window's accent, and
/// the card is painted by the pet's page over it.
unsafe extern "system" fn layer_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // Answered and left blank, or the window erases over the material.
    if message == WM_ERASEBKGND {
        return LRESULT(1);
    }
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

/// Monitors in physical pixels (the space low-level mouse hooks report).
pub fn displays() -> Vec<Display> {
    let mut out: Vec<Display> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(collect),
            LPARAM(&mut out as *mut _ as isize),
        );
    }
    out
}

/// GetAsyncKeyState reads physical buttons, so honour a left-handed swap.
pub fn primary_button_pressed() -> bool {
    unsafe {
        let vk = if GetSystemMetrics(SM_SWAPBUTTON) != 0 {
            VK_RBUTTON
        } else {
            VK_LBUTTON
        };
        GetAsyncKeyState(i32::from(vk.0)) as u16 & 0x8000 != 0
    }
}

/// Whether the foreground window belongs to a full-screen app (or a
/// presentation, or an exclusive-mode game) on the monitor holding the point,
/// in physical pixels.
pub fn fullscreen_covers(x: f64, y: f64) -> bool {
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONULL};
    use windows::Win32::UI::Shell::{
        SHQueryUserNotificationState, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetDesktopWindow, GetForegroundWindow, GetShellWindow, GetWindowRect,
    };
    unsafe {
        // Exclusive-mode games and presentation mode say so outright.
        if let Ok(state) = SHQueryUserNotificationState() {
            if state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE {
                return true;
            }
        }
        let window = GetForegroundWindow();
        if window.is_invalid() || window == GetShellWindow() || window == GetDesktopWindow() {
            return false;
        }
        // The desktop itself is a window as large as the screen.
        let mut class = [0u16; 16];
        let len = GetClassNameW(window, &mut class).max(0) as usize;
        let class = String::from_utf16_lossy(&class[..len]);
        if class == "Progman" || class == "WorkerW" {
            return false;
        }
        let monitor = MonitorFromWindow(window, MONITOR_DEFAULTTONULL);
        if monitor.is_invalid() {
            return false;
        }
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return false;
        }
        let bounds = |r: RECT| {
            (
                f64::from(r.left),
                f64::from(r.top),
                f64::from(r.right - r.left),
                f64::from(r.bottom - r.top),
            )
        };
        let display = bounds(info.rcMonitor);
        if !super::contains(display, x, y) {
            return false;
        }
        // The visible frame, without the invisible resize borders.
        let mut rect = RECT::default();
        let framed = DwmGetWindowAttribute(
            window,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut rect as *mut RECT as *mut _,
            std::mem::size_of::<RECT>() as u32,
        );
        if framed.is_err() && GetWindowRect(window, &mut rect).is_err() {
            return false;
        }
        super::covers(bounds(rect), display)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_material_is_shown_only_over_a_bubble_with_the_cursor_on_the_pet() {
        const BUBBLE: Bubble = Bubble {
            rect: (10, 20, 30, 40),
            theme_dark: false,
        };
        assert_eq!(bubble_for(true, Some(BUBBLE)), Some((10, 20, 30, 40)));
        // The cursor leaving takes the material away even while the menu holds
        // the thread the page would have said so on.
        assert_eq!(bubble_for(false, Some(BUBBLE)), None);
        // And nothing is shown where the page has asked for no bubble at all.
        assert_eq!(bubble_for(true, None), None);
        assert_eq!(bubble_for(false, None), None);
    }

    /// A desktop as dark as it gets, and one as pale, taken from a real screen.
    const BLACK_DESKTOP: f64 = 13.0;
    const WHITE_DESKTOP: f64 = 240.0;

    #[test]
    fn the_tone_is_what_is_behind_the_card() {
        // A desktop dark enough that the panel leans all the way over it: light
        // ink. This is the whole point of looking, and the case a card painted
        // out to the system's own colour cannot answer.
        assert_eq!(tone_of(BLACK_DESKTOP), 1.0);
        // And one pale enough to lean the other way.
        assert_eq!(tone_of(WHITE_DESKTOP), 0.0);
        // Past either end there is nothing more to say, and the tone stays put
        // rather than running off: the page's card is drawn from this number.
        assert_eq!(tone_of(0.0), 1.0);
        assert_eq!(tone_of(255.0), 0.0);
    }

    #[test]
    fn the_tone_drifts_across_the_middle_and_never_jumps() {
        // Every step of the way, the tone reads where the desktop actually is,
        // in one step of it — which is what lets the page draw a panel that
        // follows rather than one that changes hands. The middle is where the
        // two inks come to the same contrast, so neither is wrong there.
        let steps = [93.0, 106.0, 119.0, 132.0, 145.0].map(tone_of);
        assert_eq!(steps, [1.0, 0.75, 0.5, 0.25, 0.0]);
    }

    #[test]
    fn a_reading_is_drifted_into_and_not_dropped_on_the_page() {
        // The whole band at once, from the darkest reading to the palest: the
        // first poll moves a fifth of the way and each one after it moves a
        // fifth of what is left, so nothing arrives as a step.
        let mut tone = 1.0;
        let mut moves = Vec::new();
        for _ in 0..12 {
            let next = eased(tone, 0.0);
            moves.push(tone - next);
            tone = next;
        }
        assert!(moves[0] < 0.5, "not the whole step at once");
        assert!(
            moves.windows(2).all(|pair| pair[0] > pair[1]),
            "and quieter each time"
        );
        // Twelve polls is about a third of a second, which is as long as a hand
        // would take to notice it had not arrived.
        assert!(tone < 0.1, "arrived within a third of a second");
    }

    #[test]
    fn a_panel_comes_back_wearing_what_it_wore() {
        // Coming up is not a reading. A panel that is taken away and brought
        // back is one panel, and the desktop under it may be another by then:
        // read afresh, the card would arrive in a colour of its own and correct
        // itself in front of whoever is looking at it.
        let mut read = false;
        let kept = shown_tone(Some(0.7), || {
            read = true;
            0.0
        });
        assert_eq!(kept, 0.7);
        assert!(!read, "the desktop is not read again");
        // The first panel of a run has nothing to wear, so that one is read.
        assert_eq!(shown_tone(None, || 0.25), 0.25);
    }
}
