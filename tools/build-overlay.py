"""Assembles src/overlay.html from the reference demo so the SVG art stays identical.

Usage: python tools/build-overlay.py
"""

from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "reference" / "pokey_cursor_fast_hold_fx_shake.html"
TARGET = ROOT / "src" / "overlay.html"

lines = SOURCE.read_text(encoding="utf-8").splitlines()

hero = next(line for line in lines if line.lstrip().startswith('<div id="hero-cursor"')).strip()
flashes = next(line for line in lines if line.lstrip().startswith('<div id="flash-1"')).strip()

# The arm is the only path filled with the body gradient; overlay.css stretches and
# outlines it through this id.
marker = 'fill="url(#cursor-body-grad)"'
assert hero.count(marker) == 1, f"expected exactly one arm path, found {hero.count(marker)}"
hero = hero.replace(marker, f'id="hand-arm" {marker}')

# A white twin of the arm gradient. Stroking the arm with it gives an outline that
# fades out exactly where the arm does, instead of ending in a hard white cap.
arm_outline = (
    '<linearGradient id="arm-outline-grad" x1="90.7442" y1="204" x2="90.7442" y2="1494"'
    ' gradientUnits="userSpaceOnUse">'
    '<stop offset="0" stop-color="#ffffff"></stop>'
    '<stop offset="1" stop-color="#ffffff" stop-opacity="0"></stop>'
    "</linearGradient>"
)
assert hero.count("</defs>") == 1, "expected a single defs block"
hero = hero.replace("</defs>", arm_outline + "</defs>")

assert hero.count("<svg") == 1, "hero cursor should carry exactly one svg"
assert flashes.count("<svg") == 5, f"expected 5 flash svgs, found {flashes.count('<svg')}"

TARGET.write_text(
    "\n".join(
        [
            "<!doctype html>",
            '<html lang="en">',
            "<head>",
            '<meta charset="utf-8">',
            "<title>ViralCursor Overlay</title>",
            '<link rel="stylesheet" href="overlay.css">',
            "</head>",
            "<body>",
            hero,
            flashes,
            '<script src="overlay.js"></script>',
            "</body>",
            "</html>",
            "",
        ]
    ),
    encoding="utf-8",
)

print(f"wrote {TARGET.relative_to(ROOT)} ({TARGET.stat().st_size} bytes)")
