//! Hides the real Windows cursor system-wide so the drawn hand is the only pointer.
//!
//! `SetSystemCursor` is global and outlives the process, so every path that can end
//! the app has to run `restore()`. See `main.rs` (RunEvent::Exit + panic hook).

use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::UI::WindowsAndMessaging::{
    CreateCursor, SetSystemCursor, SystemParametersInfoW, OCR_APPSTARTING, OCR_CROSS, OCR_HAND,
    OCR_HELP, OCR_IBEAM, OCR_NO, OCR_NORMAL, OCR_SIZEALL, OCR_SIZENESW, OCR_SIZENS, OCR_SIZENWSE,
    OCR_SIZEWE, OCR_UP, OCR_WAIT, SPI_SETCURSORS, SYSTEM_CURSOR_ID,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
};

static HIDDEN: AtomicBool = AtomicBool::new(false);

const REPLACED: [SYSTEM_CURSOR_ID; 14] = [
    OCR_NORMAL,
    OCR_IBEAM,
    OCR_WAIT,
    OCR_CROSS,
    OCR_UP,
    OCR_SIZENWSE,
    OCR_SIZENESW,
    OCR_SIZEWE,
    OCR_SIZENS,
    OCR_SIZEALL,
    OCR_NO,
    OCR_HAND,
    OCR_APPSTARTING,
    OCR_HELP,
];

const CURSOR_SIDE: i32 = 32;

/// A 32x32 cursor whose AND mask is all ones and XOR mask all zeros: fully see-through.
fn blank_cursor() -> Option<windows::Win32::UI::WindowsAndMessaging::HCURSOR> {
    let bytes = (CURSOR_SIDE * CURSOR_SIDE / 8) as usize;
    let and_mask = vec![0xFFu8; bytes];
    let xor_mask = vec![0x00u8; bytes];

    unsafe {
        CreateCursor(
            None,
            0,
            0,
            CURSOR_SIDE,
            CURSOR_SIDE,
            and_mask.as_ptr() as *const _,
            xor_mask.as_ptr() as *const _,
        )
        .ok()
    }
}

pub fn hide() {
    if HIDDEN.swap(true, Ordering::SeqCst) {
        return;
    }
    for id in REPLACED {
        // SetSystemCursor takes ownership of the handle, so each slot needs its own.
        if let Some(blank) = blank_cursor() {
            unsafe {
                let _ = SetSystemCursor(blank, id);
            }
        }
    }
}

/// Reloads every cursor from the user's registry scheme. Safe to call when not hidden.
pub fn restore() {
    if !HIDDEN.swap(false, Ordering::SeqCst) {
        return;
    }
    force_restore();
}

/// Runs at startup too: if a previous run was killed while hiding the cursor, this is
/// what gives it back.
pub fn force_restore() {
    unsafe {
        let _ = SystemParametersInfoW(
            SPI_SETCURSORS,
            0,
            None,
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
    }
}
