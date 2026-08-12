/* Poke escalation, flash selection and fast-hold timing are ported straight from the
   pokey demo. The only structural change: the overlay is click-through, so pointer
   data arrives as Tauri events from the global mouse hook instead of DOM listeners. */

(() => {
  "use strict";

  const { listen } = window.__TAURI__.event;
  const { invoke } = window.__TAURI__.core;

  const root = document.documentElement;
  const cursor = document.getElementById("hero-cursor");
  const pointer = document.getElementById("hero-pointer");
  const arm = document.getElementById("hand-arm");

  /** Top of the arm path in SVG user units; the arm is stretched from this line down. */
  const ARM_ORIGIN = 204;

  let mouseX = innerWidth / 2;
  let mouseY = innerHeight / 2;
  let clickTimes = [];

  const levels = [
    { name: "poke-gentle", dur: "360ms", easing: "ease-out" },
    { name: "poke-medium", dur: "440ms", easing: "linear" },
    { name: "poke-strong", dur: "540ms", easing: "linear" },
    { name: "poke-jab", dur: "700ms", easing: "linear" }
  ];

  const HOLD_DELAY = 150;
  let holdSpeed = 92;

  let holdDelayTimer = null;
  let holdFxTimer = null;
  let holdActive = false;
  let fastFxIndex = 0;

  function moveCursor(x, y) {
    mouseX = x;
    mouseY = y;
    cursor.style.left = mouseX - 25 + "px";
    cursor.style.top = mouseY - 2 + "px";
    cursor.style.display = "block";
  }

  function flashForClick(count) {
    if (count < 2) return null;
    const t = count - 2;
    return t < 5 ? `flash-${t + 1}` : `flash-${((t - 5) % 3) + 3}`;
  }

  function flash(id, x, y) {
    const el = document.getElementById(id);
    if (!el) return;
    el.style.left = x + "px";
    el.style.top = y + "px";
    el.classList.remove("flash-active");
    el.getBoundingClientRect();
    el.classList.add("flash-active");
  }

  function smallFlash(id, x, y) {
    const el = document.getElementById(id);
    if (!el) return;
    el.classList.add("flash-small");
    flash(id, x, y);
    setTimeout(() => el.classList.remove("flash-small"), 320);
  }

  function triggerVisualHit() {
    // The machine gun owns the animation while it runs; a click must not cut into it.
    if (holdActive) return;

    const now = Date.now();
    const previous = clickTimes[clickTimes.length - 1];
    if (!previous || now - previous > 800) clickTimes = [];
    clickTimes.push(now);

    const count = clickTimes.length;
    const intensity = Math.min(count - 1, 3);

    pointer.style.animation = "none";
    pointer.getBoundingClientRect();

    if (count === 1) {
      pointer.style.animation = "poke-first-custom 340ms cubic-bezier(.18,.9,.22,1)";
      smallFlash("flash-1", mouseX - 6, mouseY + 2);
      return;
    }

    const { name, dur, easing } = levels[intensity];
    pointer.style.animation = `${name} ${dur} ${easing} forwards`;

    const primary = flashForClick(count);
    if (primary) flash(primary, mouseX, mouseY);

    if (intensity === 3) {
      const x = mouseX, y = mouseY;
      setTimeout(() => flash("flash-1", x - 8, y), 400);
    }
    if (intensity === 2) {
      const x = mouseX, y = mouseY;
      setTimeout(() => smallFlash("flash-1", x - 8, y), 460);
    }
  }

  function fastHoldImpactFx() {
    if (!holdActive) return;

    const ids = ["flash-1", "flash-2", "flash-3", "flash-4", "flash-5"];
    const id = ids[fastFxIndex++ % ids.length];
    const jitterX = (Math.random() - 0.5) * 8;
    const jitterY = (Math.random() - 0.5) * 7;

    flash(id, mouseX + jitterX, mouseY + jitterY);

    // Every few hits add a smaller secondary spark so it feels violent,
    // without spawning a huge amount of new DOM nodes every frame.
    if (fastFxIndex % 3 === 0) {
      smallFlash("flash-1", mouseX - 6 + jitterX * 0.4, mouseY + 2 + jitterY * 0.4);
    }
  }

  function startFastHold() {
    if (holdActive) return;
    holdActive = true;
    clickTimes = [];

    cursor.style.display = "block";
    pointer.style.animation = "none";
    pointer.getBoundingClientRect();
    pointer.style.animation = `poke-hold-fast ${holdSpeed}ms linear infinite`;

    invoke("set_hold", { on: true });

    fastHoldImpactFx();
    clearInterval(holdFxTimer);
    holdFxTimer = setInterval(fastHoldImpactFx, holdSpeed);
  }

  function stopFastHold() {
    clearTimeout(holdDelayTimer);
    holdDelayTimer = null;
    clearInterval(holdFxTimer);
    holdFxTimer = null;

    if (!holdActive) return;
    holdActive = false;
    invoke("set_hold", { on: false });
    pointer.style.animation = "none";
    pointer.getBoundingClientRect();
  }

  function applyConfig(cfg) {
    if (typeof cfg.speedMs === "number") holdSpeed = cfg.speedMs;
    if (typeof cfg.handScale === "number") root.style.setProperty("--hand-scale", cfg.handScale);
    if (typeof cfg.opacity === "number") root.style.setProperty("--hand-opacity", cfg.opacity);
    if (typeof cfg.flashScale === "number") root.style.setProperty("--flash-scale", cfg.flashScale);
    if (typeof cfg.armScale === "number" && arm) {
      arm.setAttribute(
        "transform",
        `translate(0 ${ARM_ORIGIN}) scale(1 ${cfg.armScale}) translate(0 ${-ARM_ORIGIN})`
      );
    }
  }

  listen("vc:move", (event) => moveCursor(event.payload.x, event.payload.y));

  // A click only pokes. Dragging has to stay usable, so nothing here starts the
  // machine gun or shakes the desktop.
  listen("vc:down", () => triggerVisualHit());

  // Ctrl is the machine-gun trigger. The 150 ms delay keeps ordinary shortcuts
  // (Ctrl+C, Ctrl+V) from setting it off.
  listen("vc:ctrl-down", () => {
    clearTimeout(holdDelayTimer);
    holdDelayTimer = setTimeout(startFastHold, HOLD_DELAY);
  });

  listen("vc:ctrl-up", () => stopFastHold());

  listen("vc:cfg", (event) => applyConfig(event.payload));
})();
