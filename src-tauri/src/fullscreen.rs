//! Detects a foreground window that covers a whole monitor.
//!
//! The hand lives in an always-on-top overlay, but a full-screen window (a game, a
//! video, the Snipping Tool overlay that PrintScreen opens) draws above it. With the
//! real cursor hidden that would leave no pointer at all, so the app gives the plain
//! cursor back for as long as such a window is in front.

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetWindowRect,
};

/// The desktop itself always covers the monitor; it must not count as a cover-up.
const SHELL_CLASSES: [&str; 3] = ["Progman", "WorkerW", "Shell_TrayWnd"];

fn class_name(window: HWND) -> String {
    let mut buffer = [0u16; 64];
    let length = unsafe { GetClassNameW(window, &mut buffer) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

pub fn foreground_covers_monitor() -> bool {
    unsafe {
        let window = GetForegroundWindow();
        if window.is_invalid() {
            return false;
        }
        if SHELL_CLASSES.contains(&class_name(window).as_str()) {
            return false;
        }

        let mut bounds = RECT::default();
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
