// Settings window: add/remove shortcuts, pick folders, installed apps, or files,
// tune the ring size, and toggle start-on-login.
const { invoke } = window.__TAURI__.core;

const $ = (id) => document.getElementById(id);

let kind = "folder";
let selectedTarget = "";
let selectedLabel = "";
let apps = [];

// ---- window chrome -------------------------------------------------------
$("close-btn").addEventListener("click", () => invoke("close_settings"));
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") invoke("close_settings");
});

// ---- folder / app segment ------------------------------------------------
document.querySelectorAll(".seg-btn").forEach((b) => {
  b.addEventListener("click", () => {
    document.querySelectorAll(".seg-btn").forEach((x) => x.classList.remove("active"));
    b.classList.add("active");
    kind = b.dataset.kind;
    selectedTarget = "";
    selectedLabel = "";
    $("folder-pane").classList.toggle("hidden", kind !== "folder");
    $("app-pane").classList.toggle("hidden", kind !== "app");
    $("file-pane").classList.toggle("hidden", kind !== "file");
    $("web-pane").classList.toggle("hidden", kind !== "web");
    resetSelectionLabels();
    if (kind === "app" && apps.length === 0) loadApps();
  });
});

function resetSelectionLabels() {
  $("folder-path").textContent = "No folder selected";
  $("folder-path").classList.add("muted");
  $("app-selected").textContent = "No app selected";
  $("app-selected").classList.add("muted");
  $("file-path").textContent = "No file selected";
  $("file-path").classList.add("muted");
  $("web-url").value = "";
  document.querySelectorAll(".app-item").forEach((x) => x.classList.remove("sel"));
}

// ---- folder picker -------------------------------------------------------
$("browse-btn").addEventListener("click", async () => {
  const p = await invoke("pick_folder");
  if (p) {
    selectedTarget = p;
    selectedLabel = p.split(/[\\/]/).filter(Boolean).pop() || p;
    $("folder-path").textContent = p;
    $("folder-path").classList.remove("muted");
  }
});

// ---- file picker ---------------------------------------------------------
$("browse-file-btn").addEventListener("click", async () => {
  const p = await invoke("pick_file");
  if (p) {
    selectedTarget = p;
    // Keep the extension in the label so files read naturally (e.g. report.pdf).
    selectedLabel = p.split(/[\\/]/).filter(Boolean).pop() || p;
    $("file-path").textContent = p;
    $("file-path").classList.remove("muted");
  }
});

// ---- website URL ---------------------------------------------------------
$("web-url").addEventListener("input", () => {
  const raw = $("web-url").value.trim();
  if (!raw) { selectedTarget = ""; selectedLabel = ""; return; }
  // Default to https:// when no scheme is given, so it opens as a URL (not a search).
  const url = /:\/\//.test(raw) ? raw : "https://" + raw;
  selectedTarget = url;
  try { selectedLabel = new URL(url).hostname || url; } catch { selectedLabel = raw; }
});

// ---- installed apps ------------------------------------------------------
async function loadApps() {
  $("app-list").innerHTML = '<div class="loading">Scanning installed apps…</div>';
  try {
    apps = await invoke("list_installed_apps");
  } catch (e) {
    apps = [];
  }
  renderApps("");
}

function renderApps(filter) {
  const f = filter.toLowerCase();
  const items = apps.filter((a) => a.name.toLowerCase().includes(f)).slice(0, 300);
  const list = $("app-list");
  list.innerHTML = "";
  if (items.length === 0) {
    list.innerHTML = '<div class="loading">No apps found</div>';
    return;
  }
  for (const a of items) {
    const el = document.createElement("div");
    el.className = "app-item";
    el.textContent = a.name;
    el.title = a.path;
    el.addEventListener("click", () => {
      selectedTarget = a.path;
      selectedLabel = a.name;
      document.querySelectorAll(".app-item").forEach((x) => x.classList.remove("sel"));
      el.classList.add("sel");
      $("app-selected").textContent = a.name;
      $("app-selected").classList.remove("muted");
    });
    list.appendChild(el);
  }
}

$("app-search").addEventListener("input", (e) => renderApps(e.target.value));

// ---- key field live formatting -------------------------------------------
$("new-key").addEventListener("input", () => {
  const v = $("new-key").value.toLowerCase().replace(/[^a-z0-9]/g, "").slice(0, 5);
  $("new-key").value = v;
  const msg = $("key-msg");
  if (v.length === 0) {
    msg.textContent = "";
    msg.className = "msg";
  } else if (v.length < 5) {
    msg.textContent = `${5 - v.length} more character(s)`;
    msg.className = "msg muted";
  } else {
    msg.textContent = "looks good";
    msg.className = "msg ok";
  }
});

// ---- add shortcut --------------------------------------------------------
$("add-btn").addEventListener("click", async () => {
  const key = $("new-key").value.trim().toLowerCase();
  const msg = $("key-msg");
  if (key.length !== 5) {
    msg.textContent = "Key must be exactly 5 characters.";
    msg.className = "msg err";
    return;
  }
  if (!selectedTarget) {
    msg.textContent =
      { folder: "Choose a folder first.", app: "Choose an app first.", file: "Choose a file first.", web: "Enter a website URL first." }[kind] ||
      "Choose a target first.";
    msg.className = "msg err";
    return;
  }
  try {
    const cfg = await invoke("add_shortcut", {
      key,
      kind,
      target: selectedTarget,
      label: selectedLabel,
    });
    $("new-key").value = "";
    selectedTarget = "";
    selectedLabel = "";
    resetSelectionLabels();
    msg.textContent = "Shortcut added.";
    msg.className = "msg ok";
    renderShortcuts(cfg.shortcuts);
  } catch (e) {
    msg.textContent = String(e);
    msg.className = "msg err";
  }
});

// ---- shortcut list -------------------------------------------------------
function escapeHtml(s) {
  return s.replace(/[&<>"']/g, (c) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c])
  );
}

function renderShortcuts(list) {
  const wrap = $("shortcut-list");
  wrap.innerHTML = "";
  if (!list || list.length === 0) {
    wrap.innerHTML = '<div class="muted small">No shortcuts yet. Add one above.</div>';
    return;
  }
  for (const s of list) {
    const row = document.createElement("div");
    row.className = "shortcut-row";
    row.innerHTML =
      `<span class="key-chip mono">${escapeHtml(s.key)}</span>` +
      `<span class="sc-label" title="${escapeHtml(s.target)}">${escapeHtml(s.label)}</span>` +
      `<span class="sc-kind ${s.kind}">${s.kind}</span>`;
    const del = document.createElement("button");
    del.className = "icon-btn del";
    del.textContent = "✕";
    del.title = "Delete";
    del.addEventListener("click", async () => {
      const cfg = await invoke("delete_shortcut", { key: s.key });
      renderShortcuts(cfg.shortcuts);
    });
    row.appendChild(del);
    wrap.appendChild(row);
  }
}

// ---- appearance ----------------------------------------------------------
$("ring-size").addEventListener("input", (e) => {
  $("ring-size-val").textContent = e.target.value + "px";
});
$("ring-size").addEventListener("change", async (e) => {
  try { await invoke("set_ring_size", { size: parseFloat(e.target.value) }); } catch (err) {}
});

// ---- autostart -----------------------------------------------------------
$("autostart").addEventListener("change", async (e) => {
  try {
    await invoke("set_autostart", { enabled: e.target.checked });
  } catch (err) {
    e.target.checked = !e.target.checked; // revert on failure
  }
});

// ---- footer --------------------------------------------------------------
$("reset-btn").addEventListener("click", () => invoke("reset_ring"));
$("quit-btn").addEventListener("click", () => invoke("quit_app"));

// ---- init ----------------------------------------------------------------
(async () => {
  try {
    const cfg = await invoke("get_config");
    $("ring-size").value = cfg.ring_size;
    $("ring-size-val").textContent = Math.round(cfg.ring_size) + "px";
    renderShortcuts(cfg.shortcuts);
  } catch (e) {}
  try {
    $("autostart").checked = await invoke("get_autostart");
  } catch (e) {}
})();
