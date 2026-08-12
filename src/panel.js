(() => {
  "use strict";

  const { invoke } = window.__TAURI__.core;
  const { emitTo, listen } = window.__TAURI__.event;

  const STORAGE_KEY = "viralcursor.settings";

  const sliders = {
    speed: { el: document.getElementById("speed"), out: document.getElementById("speed-value") },
    arm: { el: document.getElementById("arm"), out: document.getElementById("arm-value") },
    size: { el: document.getElementById("size"), out: document.getElementById("size-value") },
    opacity: { el: document.getElementById("opacity"), out: document.getElementById("opacity-value") },
    shake: { el: document.getElementById("shake"), out: document.getElementById("shake-value") },
    flash: { el: document.getElementById("flash"), out: document.getElementById("flash-value") }
  };

  const DEFAULTS = { speed: 92, arm: 100, size: 100, opacity: 100, shake: 2, flash: 100 };

  const toggle = document.getElementById("toggle");
  const reset = document.getElementById("reset");
  let running = false;

  function read() {
    return {
      speed: Number(sliders.speed.el.value),
      arm: Number(sliders.arm.el.value),
      size: Number(sliders.size.el.value),
      opacity: Number(sliders.opacity.el.value),
      shake: Number(sliders.shake.el.value),
      flash: Number(sliders.flash.el.value)
    };
  }

  function paintLabels(values) {
    sliders.speed.out.textContent = (1000 / values.speed).toFixed(1) + " / s";
    sliders.arm.out.textContent = values.arm + "%";
    sliders.size.out.textContent = values.size + "%";
    sliders.opacity.out.textContent = values.opacity + "%";
    sliders.shake.out.textContent = values.shake === 0 ? "off" : values.shake + " px";
    sliders.flash.out.textContent = values.flash + "%";
  }

  function push() {
    const values = read();
    paintLabels(values);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(values));

    emitTo("overlay", "vc:cfg", {
      speedMs: values.speed,
      armScale: values.arm / 100,
      handScale: values.size / 100,
      opacity: values.opacity / 100,
      flashScale: values.flash / 100
    });
    invoke("set_shake", { pixels: values.shake });
  }

  function restore() {
    let saved;
    try {
      saved = JSON.parse(localStorage.getItem(STORAGE_KEY) || "null");
    } catch {
      saved = null;
    }
    if (saved) {
      for (const [key, slider] of Object.entries(sliders)) {
        if (typeof saved[key] === "number") slider.el.value = String(saved[key]);
      }
    }
    push();
  }

  function setRunning(next) {
    running = next;
    toggle.textContent = running ? "END" : "START";
  }

  toggle.addEventListener("click", async () => {
    if (running) {
      await invoke("stop");
      setRunning(false);
      return;
    }
    await invoke("start");
    setRunning(true);
    // The overlay webview is created fresh on first run; resend once it can listen.
    setTimeout(push, 120);
  });

  reset.addEventListener("click", () => {
    for (const [key, value] of Object.entries(DEFAULTS)) {
      sliders[key].el.value = String(value);
    }
    push();
  });

  for (const slider of Object.values(sliders)) {
    slider.el.addEventListener("input", push);
  }

  listen("vc:stopped", () => setRunning(false));

  restore();
})();
