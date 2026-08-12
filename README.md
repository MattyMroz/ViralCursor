# ViralCursor

Your Windows cursor becomes the giant poking hand from the
[pokey](reference/pokey_cursor_fast_hold_fx_shake.html) demo. Click and it pokes.
Hold the button and it turns into a machine gun while the whole desktop shakes.

![the hand](icon.png)

## What it does

- Replaces the system cursor everywhere with the animated hand — same SVG, same
  keyframes, same escalating poke as the web demo.
- Click bursts escalate: `poke-gentle` → `poke-medium` → `poke-strong` → `poke-jab`,
  each with its own impact flash.
- **Hold Ctrl** for 150 ms and it turns into a machine gun: ~11 pokes per second, rapid
  impact flashes, and the whole desktop shaking through the Windows full-screen
  magnification transform — every window on the primary monitor moves, not just an
  overlay.
- The art carries a white outline drawn under the fill, so the black-on-white hand and
  flashes stay readable on a black desktop too.

The mouse deliberately does nothing but poke. Ctrl is the trigger for everything
violent, because a shaking screen makes dragging and dropping impossible.

## Controls

| Setting | Range | Default |
|---|---|---|
| Click speed | 3.8 – 25 pokes/s | 10.9 /s |
| Arm length | 30 – 250 % | 100 % |
| Hand size | 50 – 220 % | 100 % |
| Opacity | 15 – 100 % | 100 % |
| Screen shake | off – 14 px | 2 px |
| Impact size | 50 – 200 % | 100 % |

**Ctrl + Alt + Q** stops everything and gives the normal cursor back. It works even if
the window is buried, which matters: hiding the cursor is a system-wide change.

## Build

Needs [Rust](https://rustup.rs) and Node.

```bash
npm install
npm run build     # -> src-tauri/target/release/ViralCursor.exe
npm run dev       # run without packaging
```

Regenerating the icon from the demo's SVG:

```bash
node tools/build-icon.mjs && npx tauri icon icon.png
```

## How it works

| Piece | Where |
|---|---|
| Hand + flash SVG, keyframes | `src/overlay.css`, `src/overlay.html` |
| Poke escalation, machine gun | `src/overlay.js` |
| Control panel | `src/index.html`, `src/panel.js` |
| Hiding the real cursor | `src-tauri/src/cursor.rs` (`SetSystemCursor`) |
| Mouse + Ctrl feed, panic hotkey | `src-tauri/src/hook.rs` (`WH_MOUSE_LL`, `WH_KEYBOARD_LL`) |
| Desktop shake | `src-tauri/src/shake.rs` (`MagSetFullscreenTransform`) |
| Overlay window, wiring | `src-tauri/src/lib.rs` |

`src/overlay.html` is generated — do not edit it by hand. It is assembled from
`reference/pokey_cursor_fast_hold_fx_shake.html` so the SVG art cannot drift from the
original:

```bash
python tools/build-overlay.py
```

The overlay is a transparent, click-through, always-on-top window spanning every
monitor. Because it is click-through it never receives mouse events itself, so the
pointer position comes from a global low-level mouse hook in Rust.

## Known limits

- Windows only. The whole effect is built on Win32 APIs.
- The desktop shake covers the **primary monitor**; the full-screen magnification
  transform has no multi-monitor variant.
- UAC prompts run on their own secure desktop, which neither the overlay nor the
  cursor replacement reaches: there you briefly get the plain system arrow back.
