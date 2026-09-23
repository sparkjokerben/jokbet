use crate::engine::distance::Display;
use windows::core::BOOL;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateRectRgn, DeleteObject, EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, HRGN,
    MONITORINFO,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_RAW_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_SWAPBUTTON};

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

/// Blurs the desktop behind the bubble.
///
/// Windows has no per-panel acrylic: the acrylic and mica backdrops cover the
/// whole window, which would put a frosted slab under the pet too. The one
/// material that can be clipped to a rectangle is the blur behind a region, so
/// that is what the bubble gets. `rect` is in physical pixels, client-area
/// coordinates; `None` clears it.
pub fn set_glass(hwnd: HWND, rect: Option<(i32, i32, i32, i32)>) {
    use windows::Win32::Graphics::Dwm::{
        DwmEnableBlurBehindWindow, DWM_BB_BLURREGION, DWM_BB_ENABLE, DWM_BLURBEHIND,
    };
    let (enabled, region) = match rect {
        Some((x, y, w, h)) => {
            let region = unsafe { CreateRectRgn(x, y, x + w, y + h) };
            (true, region)
        }
        None => (false, HRGN::default()),
    };
    let blur = DWM_BLURBEHIND {
        dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
        fEnable: enabled.into(),
        hRgnBlur: region,
        fTransitionOnMaximized: false.into(),
    };
    unsafe {
        let _ = DwmEnableBlurBehindWindow(hwnd, &blur);
        if !region.is_invalid() {
            let _ = DeleteObject(region);
        }
    }
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
