//! Detects a foreground window that the overlay cannot draw over.
//!
//! The overlay is always-on-top, so a window covering the whole monitor is not a
//! problem by itself: a browser at F11 is not topmost, and the hand still paints over
//! it. Only a window that is *also* topmost competes with us — the snipping overlay
//! PrintScreen opens, or a game on exclusive full screen. With the real cursor hidden
//! those would leave no pointer at all, so the app gives the plain cursor back for as
//! long as one of them is in front.

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect,
    GetWindowThreadProcessId, GWL_EXSTYLE, WS_EX_TOPMOST,
};

/// The desktop itself always covers the monitor; it must not count as a cover-up.
const SHELL_CLASSES: [&str; 3] = ["Progman", "WorkerW", "Shell_TrayWnd"];

fn class_name(window: HWND) -> String {
    let mut buffer = [0u16; 64];
    let length = unsafe { GetClassNameW(window, &mut buffer) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

fn is_ours(window: HWND) -> bool {
    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(window, Some(&mut pid));
        pid == GetCurrentProcessId()
    }
}

fn is_topmost(window: HWND) -> bool {
    let style = unsafe { GetWindowLongPtrW(window, GWL_EXSTYLE) };
    style & WS_EX_TOPMOST.0 as isize != 0
}

fn covers_its_monitor(window: HWND) -> bool {
    unsafe {
        let mut bounds = Default::default();
        if GetWindowRect(window, &mut bounds).is_err() {
            return false;
        }

        let monitor = MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return false;
        }

        let screen = info.rcMonitor;
        bounds.left <= screen.left
            && bounds.top <= screen.top
            && bounds.right >= screen.right
            && bounds.bottom >= screen.bottom
    }
}

pub fn foreground_blocks_overlay() -> bool {
    let window = unsafe { GetForegroundWindow() };
    if window.is_invalid() || is_ours(window) {
        return false;
    }
    if SHELL_CLASSES.contains(&class_name(window).as_str()) {
        return false;
    }
    is_topmost(window) && covers_its_monitor(window)
}
