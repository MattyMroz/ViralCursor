//! Global low-level mouse and keyboard hooks.
//!
//! The overlay is click-through, so it never sees an input event of its own. These
//! hooks are the only source of pointer and Ctrl state. They are installed once and
//! left in place; the consumer decides whether the app is currently running.
//!
//! Low-level hook callbacks run on the thread that installed them, which is why that
//! thread owns a message loop. The same loop carries the Ctrl+Alt+Q panic stop.

use std::sync::mpsc::Sender;
use std::sync::OnceLock;

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, VIRTUAL_KEY, VK_CONTROL,
    VK_LCONTROL, VK_RCONTROL,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, HC_ACTION,
    KBDLLHOOKSTRUCT, MSG, MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_HOTKEY, WM_KEYDOWN,
    WM_KEYUP, WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

#[derive(Debug, Clone, Copy)]
pub enum Event {
    /// Physical screen coordinates.
    Move(i32, i32),
    Down,
    /// Ctrl is the trigger for the machine-gun mode, so the mouse stays usable.
    CtrlDown,
    CtrlUp,
    /// Ctrl+Alt+Q: kill switch that must work even when the panel is unreachable.
    PanicStop,
}

const HOTKEY_ID: i32 = 0xC0DE;

static SINK: OnceLock<Sender<Event>> = OnceLock::new();

fn emit(event: Event) {
    if let Some(sink) = SINK.get() {
        let _ = sink.send(event);
    }
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
        match wparam.0 as u32 {
            WM_MOUSEMOVE => emit(Event::Move(info.pt.x, info.pt.y)),
            WM_LBUTTONDOWN => emit(Event::Down),
            _ => {}
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

unsafe extern "system" fn key_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let key = VIRTUAL_KEY(info.vkCode as u16);
        if key == VK_LCONTROL || key == VK_RCONTROL || key == VK_CONTROL {
            match wparam.0 as u32 {
                // Auto-repeat fires KEYDOWN over and over; the consumer de-duplicates.
                WM_KEYDOWN | WM_SYSKEYDOWN => emit(Event::CtrlDown),
                WM_KEYUP | WM_SYSKEYUP => emit(Event::CtrlUp),
                _ => {}
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// Installs the hooks on a dedicated thread. Safe to call once; later calls are ignored.
pub fn spawn(sink: Sender<Event>) {
    if SINK.set(sink).is_err() {
        return;
    }

    std::thread::spawn(|| unsafe {
        let hook = match SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), None, 0) {
            Ok(hook) => hook,
            Err(_) => return,
        };
        let keys = SetWindowsHookExW(WH_KEYBOARD_LL, Some(key_proc), None, 0).ok();

        let _ = RegisterHotKey(
            None,
            HOTKEY_ID,
            HOT_KEY_MODIFIERS(MOD_CONTROL.0 | MOD_ALT.0 | MOD_NOREPEAT.0),
            b'Q' as u32,
        );

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            if msg.message == WM_HOTKEY && msg.wParam.0 as i32 == HOTKEY_ID {
                emit(Event::PanicStop);
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let _ = windows::Win32::UI::WindowsAndMessaging::UnhookWindowsHookEx(hook);
        if let Some(keys) = keys {
            let _ = windows::Win32::UI::WindowsAndMessaging::UnhookWindowsHookEx(keys);
        }
    });
}
