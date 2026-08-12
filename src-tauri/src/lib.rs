mod cursor;
mod fullscreen;
mod hook;

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, SM_CXVIRTUALSCREEN,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

const OVERLAY: &str = "overlay";
const PANEL: &str = "panel";

/// Pointer updates are coalesced to this interval; the hook itself can fire far faster.
const MOVE_INTERVAL: Duration = Duration::from_millis(6);

/// How often the full-screen guard re-checks the foreground window.
const GUARD_INTERVAL: Duration = Duration::from_millis(250);

static RUNNING: AtomicBool = AtomicBool::new(false);
/// True while a full-screen window forced the plain cursor back.
static SUSPENDED: AtomicBool = AtomicBool::new(false);
static ORIGIN_X: AtomicI32 = AtomicI32::new(0);
static ORIGIN_Y: AtomicI32 = AtomicI32::new(0);
static SCALE: AtomicU32 = AtomicU32::new(1.0f32.to_bits());

#[derive(Clone, serde::Serialize)]
struct Point {
    x: f64,
    y: f64,
}

fn virtual_screen() -> (i32, i32, i32, i32) {
    unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    }
}

/// Keeps the overlay out of the alt-tab list and stops it from ever taking focus.
fn make_passive(window: &WebviewWindow) {
    let Ok(raw) = window.hwnd() else { return };
    let hwnd = HWND(raw.0 as _);
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            style | WS_EX_NOACTIVATE.0 as isize | WS_EX_TOOLWINDOW.0 as isize,
        );
    }
}

fn cover_all_screens(overlay: &WebviewWindow) -> tauri::Result<()> {
    let (x, y, width, height) = virtual_screen();
    overlay.set_position(PhysicalPosition::new(x, y))?;
    overlay.set_size(PhysicalSize::new(width.max(1) as u32, height.max(1) as u32))?;

    ORIGIN_X.store(x, Ordering::Relaxed);
    ORIGIN_Y.store(y, Ordering::Relaxed);
    SCALE.store((overlay.scale_factor()? as f32).to_bits(), Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
fn start(app: AppHandle) -> Result<(), String> {
    let overlay = app
        .get_webview_window(OVERLAY)
        .ok_or_else(|| "overlay window is gone".to_string())?;

    cover_all_screens(&overlay).map_err(|e| e.to_string())?;
    overlay.show().map_err(|e| e.to_string())?;
    overlay.set_always_on_top(true).map_err(|e| e.to_string())?;
    cursor::hide();
    RUNNING.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
fn stop(app: AppHandle) {
    shut_down_effects();
    if let Some(overlay) = app.get_webview_window(OVERLAY) {
        let _ = overlay.hide();
    }
}

fn shut_down_effects() {
    RUNNING.store(false, Ordering::SeqCst);
    SUSPENDED.store(false, Ordering::SeqCst);
    cursor::restore();
}

/// Gives the plain cursor back while a full-screen window sits above the overlay,
/// otherwise there would be no visible pointer at all in games or the PrintScreen
/// snipping overlay.
fn guard_visibility(app: AppHandle) {
    loop {
        std::thread::sleep(GUARD_INTERVAL);

        if !RUNNING.load(Ordering::Relaxed) {
            SUSPENDED.store(false, Ordering::Relaxed);
            continue;
        }

        let covered = fullscreen::foreground_covers_monitor();
        if covered == SUSPENDED.load(Ordering::Relaxed) {
            continue;
        }
        SUSPENDED.store(covered, Ordering::Relaxed);

        let Some(overlay) = app.get_webview_window(OVERLAY) else {
            continue;
        };
        if covered {
            cursor::restore();
            let _ = overlay.hide();
        } else {
            let _ = overlay.show();
            cursor::hide();
        }
    }
}

fn pump_pointer(app: AppHandle, events: mpsc::Receiver<hook::Event>) {
    let mut pending: Option<(i32, i32)> = None;
    let mut last_move = Instant::now() - MOVE_INTERVAL;
    let mut ctrl_held = false;

    loop {
        match events.recv_timeout(MOVE_INTERVAL) {
            Ok(hook::Event::Move(x, y)) => pending = Some((x, y)),
            Ok(hook::Event::Down) => {
                if RUNNING.load(Ordering::Relaxed) {
                    let _ = app.emit_to(OVERLAY, "vc:down", ());
                }
            }
            Ok(hook::Event::CtrlDown) => {
                if RUNNING.load(Ordering::Relaxed) && !ctrl_held {
                    ctrl_held = true;
                    let _ = app.emit_to(OVERLAY, "vc:ctrl-down", ());
                }
            }
            Ok(hook::Event::CtrlUp) => {
                ctrl_held = false;
                if RUNNING.load(Ordering::Relaxed) {
                    let _ = app.emit_to(OVERLAY, "vc:ctrl-up", ());
                }
            }
            Ok(hook::Event::PanicStop) => {
                if RUNNING.load(Ordering::Relaxed) {
                    stop(app.clone());
                    let _ = app.emit_to(PANEL, "vc:stopped", ());
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }

        if pending.is_some() && last_move.elapsed() >= MOVE_INTERVAL {
            let (x, y) = pending.take().unwrap();
            if RUNNING.load(Ordering::Relaxed) {
                let scale = f32::from_bits(SCALE.load(Ordering::Relaxed)) as f64;
                let point = Point {
                    x: (x - ORIGIN_X.load(Ordering::Relaxed)) as f64 / scale,
                    y: (y - ORIGIN_Y.load(Ordering::Relaxed)) as f64 / scale,
                };
                let _ = app.emit_to(OVERLAY, "vc:move", point);
            }
            last_move = Instant::now();
        }
    }
}

pub fn run() {
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        shut_down_effects();
        previous_hook(info);
    }));

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![start, stop])
        .setup(|app| {
            cursor::force_restore();

            let overlay = WebviewWindowBuilder::new(
                app,
                OVERLAY,
                WebviewUrl::App("overlay.html".into()),
            )
            .title("ViralCursor Overlay")
            .position(0.0, 0.0)
            .inner_size(100.0, 100.0)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(false)
            .focused(false)
            .visible(false)
            .build()?;

            overlay.set_ignore_cursor_events(true)?;
            make_passive(&overlay);

            let (sender, receiver) = mpsc::channel();
            hook::spawn(sender);

            let handle = app.handle().clone();
            std::thread::spawn(move || pump_pointer(handle, receiver));

            let guard = app.handle().clone();
            std::thread::spawn(move || guard_visibility(guard));
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == PANEL {
                if let WindowEvent::CloseRequested { .. } = event {
                    shut_down_effects();
                    window.app_handle().exit(0);
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("failed to start ViralCursor")
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                shut_down_effects();
            }
        });
}
