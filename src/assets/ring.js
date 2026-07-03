// Ring overlay logic: idle spin, drag-vs-click, key entry with live validation,
// and the launch sequence. All window geometry lives in Rust; this file drives
// the visuals and calls the backend commands.
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;
const appWindow = getCurrentWindow();

const root = document.documentElement;
const ring = document.getElementById("ring");
const entry = document.getElementById("entry");
const keyInput = document.getElementById("key-input");
const enterBtn = document.getElementById("enter-btn");
const hint = document.getElementById("entry-hint");

let mode = "idle"; // idle | entry | launch
let knownKeys = new Set();
let ringPx = 100;

// Mirrors overlay.rs RING_MARGIN — the transparent margin around the ring in the
// idle window. The ring sits at the window centre, so its offset is half of the
// window (ring + margin) / 2.
const RING_MARGIN = 48;
function idlePad() {
  return (ringPx + RING_MARGIN) / 2;
}

function setRingPos(x, y) {
  root.style.setProperty("--rx", x + "px");
  root.style.setProperty("--ry", y + "px");
}

function applyConfig(cfg) {
  if (!cfg) return;
  ringPx = Math.min(200, Math.max(40, cfg.ring_size || 100));
  root.style.setProperty("--rs", ringPx + "px");
  root.style.setProperty("--hit", Math.max(24, ringPx + 12) + "px");
  knownKeys = new Set((cfg.shortcuts || []).map((s) => s.key));
  // The backend live-resizes the idle window on size change; recentre to match.
  if (mode === "idle") setRingPos(idlePad(), idlePad());
}

async function refreshConfig() {
  try { applyConfig(await invoke("get_config")); } catch (e) {}
}

// -------------------------------------------------------------- entry box ---

function placeEntry(layout) {
  const bottom = layout.corner[0] === "b";
  const clear = ringPx / 2 + 18; // clear the ring's edge, plus a gap
  entry.style.left = "16px";
  entry.style.right = "16px";
  if (bottom) {
    entry.style.bottom = layout.height - layout.ring_y + clear + "px";
    entry.style.top = "auto";
  } else {
    entry.style.top = layout.ring_y + clear + "px";
    entry.style.bottom = "auto";
  }
}

async function openEntry() {
  if (mode !== "idle") return;
  try {
    const layout = await invoke("open_entry");
    setRingPos(layout.ring_x, layout.ring_y);
    placeEntry(layout);
    mode = "entry";
    ring.classList.add("standby");
    entry.classList.add("show");
    keyInput.value = "";
    setValidity(null);
    setTimeout(() => keyInput.focus(), 30);
  } catch (e) {
    console.error(e);
  }
}

function resetVisuals() {
  entry.classList.remove("show");
  ring.classList.remove("standby", "launching");
  setRingPos(idlePad(), idlePad());
  mode = "idle";
}

async function closeEntry() {
  if (mode !== "entry") return;
  try { await invoke("close_entry"); } catch (e) {}
  resetVisuals();
}

// Rust closed us because the window lost focus (user clicked elsewhere).
listen("entry-dismissed", () => { if (mode === "entry") resetVisuals(); });
listen("config-changed", (e) => (e.payload ? applyConfig(e.payload) : refreshConfig()));

// ----------------------------------------------------------- drag vs click --

let pressing = false, moved = false, sx = 0, sy = 0;

ring.addEventListener("pointerdown", (e) => {
  if (e.button !== 0 || mode !== "idle") return;
  pressing = true;
  moved = false;
  sx = e.clientX;
  sy = e.clientY;
});
window.addEventListener("pointermove", (e) => {
  if (!pressing) return;
  if (!moved && (Math.abs(e.clientX - sx) > 4 || Math.abs(e.clientY - sy) > 4)) {
    moved = true;
    appWindow.startDragging();
  }
});
window.addEventListener("pointerup", () => {
  if (!pressing) return;
  pressing = false;
  if (!moved) openEntry();
});

// Right-click anywhere on the ring opens Settings (also available from the tray).
// The window is pre-created hidden at startup, so this just shows it.
window.addEventListener("contextmenu", (e) => {
  e.preventDefault();
  invoke("open_settings");
});

// ------------------------------------------------------------- key entry ----

function setValidity(state) {
  keyInput.classList.remove("valid", "invalid");
  hint.classList.remove("ok", "err");
  if (state === true) {
    keyInput.classList.add("valid");
    hint.textContent = "match found — press Enter";
    hint.classList.add("ok");
  } else if (state === false) {
    keyInput.classList.add("invalid");
    hint.textContent = "no shortcut for that key";
    hint.classList.add("err");
  } else {
    hint.textContent = "enter a 5-character key";
  }
}

function shake() {
  entry.classList.remove("shake");
  void entry.offsetWidth; // restart animation
  entry.classList.add("shake");
}

keyInput.addEventListener("input", () => {
  const v = keyInput.value.toLowerCase().replace(/[^a-z0-9]/g, "").slice(0, 5);
  keyInput.value = v;
  if (v.length < 5) setValidity(null);
  else setValidity(knownKeys.has(v));
});

keyInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter") { e.preventDefault(); submit(); }
  else if (e.key === "Escape") { e.preventDefault(); closeEntry(); }
});
enterBtn.addEventListener("click", submit);
document.getElementById("entry-close").addEventListener("click", closeEntry);

function submit() {
  const key = keyInput.value.trim().toLowerCase();
  if (key.length !== 5 || !knownKeys.has(key)) {
    setValidity(false);
    shake();
    return;
  }
  launch(key);
}

// ------------------------------------------------------------- launch -------

// Wait for the webview to report its fullscreen dimensions before drawing, so
// the effect is centred on the real (post-resize) window. Resolves once the
// width stops changing, or after a short safety timeout.
function waitForResize() {
  return new Promise((resolve) => {
    let last = -1, stable = 0;
    const t0 = performance.now();
    (function poll() {
      const w = window.innerWidth;
      if (w === last) stable += 1; else { stable = 0; last = w; }
      if (stable >= 3 || performance.now() - t0 > 800) resolve();
      else requestAnimationFrame(poll);
    })();
  });
}

async function launch(key) {
  mode = "launch";
  entry.classList.remove("show");
  ring.classList.remove("standby");
  ring.classList.add("launching");

  let layout;
  try {
    layout = await invoke("start_launch"); // fullscreen click-through overlay
  } catch (e) {
    console.error(e);
    ring.classList.remove("launching");
    mode = "idle";
    return;
  }

  // Where the gravity well forms within the overlay:
  //  * Windows/macOS: the ring's real screen position (`center_x/y`).
  //  * Linux/Wayland: the centre of the compositor-fullscreen window, computed
  //    here from innerWidth after the resize lands (positions aren't knowable).
  let cx, cy;
  if (layout.centered) {
    await waitForResize();
    cx = window.innerWidth / 2;
    cy = window.innerHeight / 2;
  } else {
    cx = layout.center_x;
    cy = layout.center_y;
  }
  const center = { x: cx, y: cy };
  setRingPos(cx, cy);

  await window.Vacuum.suck(center, ringPx);
  try { await invoke("open_shortcut", { key }); } catch (e) { console.error(e); }
  await window.Vacuum.spit(center);
  try { await invoke("finish_launch"); } catch (e) {}

  ring.classList.remove("launching");
  setRingPos(idlePad(), idlePad());
  keyInput.value = "";
  setValidity(null);
  mode = "idle";
}

// --------------------------------------------------------------- init -------

setRingPos(32, 32);
refreshConfig();
