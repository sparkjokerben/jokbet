use crate::engine::distance::Display;
use windows::core::BOOL;
use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
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
