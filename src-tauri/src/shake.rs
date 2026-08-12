//! Shakes the whole desktop through the full-screen Magnification transform.
//!
//! `MagSetFullscreenTransform` can only offset and zoom, so the shake is a zoom just
//! large enough to hide the edges (`zoom_for`) plus a per-step offset jitter. All
//! Magnification calls live on one owned thread.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::Duration;

use windows::Win32::UI::Magnification::{MagInitialize, MagSetFullscreenTransform};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

/// One jitter step. Mirrors `fast-screen-shake 72ms steps(2, end)` from the web demo.
const STEP: Duration = Duration::from_millis(33);

struct Shared {
    held: bool,
    quit: bool,
    transformed: bool,
}

struct Engine {
    shared: Mutex<Shared>,
    wake: Condvar,
    idle: Condvar,
}

/// Amplitude in physical pixels, stored as f32 bits. 0 disables the shake entirely.
static AMPLITUDE: AtomicU32 = AtomicU32::new(0);
static ENGINE: OnceLock<&'static Engine> = OnceLock::new();

pub fn set_amplitude(px: f32) {
    AMPLITUDE.store(px.clamp(0.0, 40.0).to_bits(), Ordering::Relaxed);
}

fn amplitude() -> f32 {
    f32::from_bits(AMPLITUDE.load(Ordering::Relaxed))
}

fn engine() -> &'static Engine {
    ENGINE.get_or_init(|| {
        let engine: &'static Engine = Box::leak(Box::new(Engine {
            shared: Mutex::new(Shared {
                held: false,
                quit: false,
                transformed: false,
            }),
            wake: Condvar::new(),
            idle: Condvar::new(),
        }));
        std::thread::spawn(move || run(engine));
        engine
    })
}

pub fn hold(on: bool) {
    let engine = engine();
    let mut shared = engine.shared.lock().unwrap();
    shared.held = on;
    engine.wake.notify_all();
}

/// Drops the screen back to 1:1 and stops the thread. Called on app exit.
pub fn shutdown() {
    let Some(engine) = ENGINE.get() else {
        return;
    };
    let mut shared = engine.shared.lock().unwrap();
    shared.quit = true;
    shared.held = false;
    engine.wake.notify_all();

    let (shared, timeout) = engine
        .idle
        .wait_timeout_while(shared, Duration::from_millis(300), |s| s.transformed)
        .unwrap();
    drop(shared);

    if timeout.timed_out() {
        // The worker did not answer; reset from here rather than leave the desktop zoomed.
        unsafe {
            let _ = MagSetFullscreenTransform(1.0, 0, 0);
        }
    }
}

/// Smallest zoom that keeps a `±amp` offset inside the real screen on both axes.
fn zoom_for(amp: f32, width: f32, height: f32) -> f32 {
    let by_width = width / (width - 2.0 * amp).max(1.0);
    let by_height = height / (height - 2.0 * amp).max(1.0);
    by_width.max(by_height).max(1.0)
}

fn run(engine: &'static Engine) {
    unsafe {
        if !MagInitialize().as_bool() {
            return;
        }
    }

    let (width, height) = unsafe {
        (
            GetSystemMetrics(SM_CXSCREEN) as f32,
            GetSystemMetrics(SM_CYSCREEN) as f32,
        )
    };
    let mut seed: u32 = 0x9E37_79B9;

    loop {
        let mut shared = engine.shared.lock().unwrap();

        while !shared.quit && !shared.held {
            if shared.transformed {
                unsafe {
                    let _ = MagSetFullscreenTransform(1.0, 0, 0);
                }
                shared.transformed = false;
                engine.idle.notify_all();
            }
            shared = engine.wake.wait(shared).unwrap();
        }

        if shared.quit {
            if shared.transformed {
                unsafe {
                    let _ = MagSetFullscreenTransform(1.0, 0, 0);
                }
                shared.transformed = false;
            }
            engine.idle.notify_all();
            return;
        }
        drop(shared);

        let amp = amplitude();
        if amp < 0.5 {
            std::thread::sleep(STEP);
            continue;
        }

        let zoom = zoom_for(amp, width, height);
        let slack_x = (width - width / zoom) * 0.5;
        let slack_y = (height - height / zoom) * 0.5;

        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let jitter_x = ((seed & 0xFFFF) as f32 / 65535.0 * 2.0 - 1.0) * amp;
        let jitter_y = ((seed >> 16) as f32 / 65535.0 * 2.0 - 1.0) * amp;

        unsafe {
            let _ = MagSetFullscreenTransform(
                zoom,
                (slack_x + jitter_x) as i32,
                (slack_y + jitter_y) as i32,
            );
        }
        engine.shared.lock().unwrap().transformed = true;

        std::thread::sleep(STEP);
    }
}
