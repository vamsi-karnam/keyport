# Building Keyport (Windows & Linux)

This is the developer/build guide. (End-user instructions live in
[README.md](README.md).) Keyport builds from the **same source** on both
platforms — only the toolchain prerequisites and the bundle format differ.

## The short version — what replaces PyInstaller + Inno Setup?

If you come from Python, your usual route is **PyInstaller** (freeze to an
executable) plus **Inno Setup** (wrap it in an installer). In Rust + Tauri, **one
command does both**:

```sh
cargo tauri build
```

That compiles a native, self-contained `keyport` binary **and** wraps it in a
platform installer — an **NSIS** setup `.exe` on Windows (NSIS is the
Inno-Setup equivalent), a **`.deb`** / **`.AppImage`** on Linux. No separate
freeze/packaging step, and no Python-style "hunt for missing modules" — Rust
statically links everything it needs at compile time.

The `bundle.targets` in `tauri.conf.json` is `"all"`, so each OS builds the
formats valid for it. To pick one explicitly, pass `--bundles` (recommended, see
each platform below).

---

## 1. Prerequisites (one-time)

> **Node.js is NOT required on either platform.** Keyport's UI is plain static
> HTML/CSS/JS served straight from the `src/` folder — no bundler, no `npm`.

Common to both: the **Rust toolchain** (<https://rustup.rs>) and the **Tauri
CLI** (`cargo install tauri-cli --version "^2"`).

### 🪟 Windows

| Tool | Why | Install |
| --- | --- | --- |
| **Rust** (stable, MSVC) | Compiler + Cargo | rustup — default `x86_64-pc-windows-msvc` |
| **MSVC C++ Build Tools** | Rust needs Microsoft's `link.exe` | [VS Build Tools](https://visualstudio.microsoft.com/visual-studio-build-tools/) → **“Desktop development with C++”** |
| **WebView2 Runtime** | The UI renders in Edge WebView2 | Pre-installed on Win 10/11; installer auto-fetches if missing |
| **Tauri CLI** | `cargo tauri …` | `cargo install tauri-cli --version "^2"` |

If `cargo build` fails with a **linker error** (`link.exe` not found), you're
missing the C++ Build Tools above — install them and reopen your terminal.

### 🐧 Linux

Rust needs a C toolchain plus the **WebKitGTK** dev libraries (the Linux webview)
and a few Tauri deps.

**Debian / Ubuntu:**

```sh
sudo apt update
sudo apt install \
  build-essential curl wget file \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libxdo-dev \
  xdg-utils
```

**Fedora:**

```sh
sudo dnf group install "Development Tools" "C Development Tools and Libraries"
sudo dnf install \
  webkit2gtk4.1-devel gtk3-devel \
  libappindicator-gtk3-devel librsvg2-devel \
  libxdo-devel xdg-utils
```

*(`xdg-utils` is a **runtime** dependency too — Keyport uses `xdg-open` to open
folders. `libayatana-appindicator3` provides the tray icon.)*

Verify (either OS):

```sh
rustc --version
cargo tauri --version
```

---

## 2. Run in development

From the project root, on either OS:

```sh
cargo tauri dev
```

First run is slow (it compiles the whole dependency tree once); later runs are
incremental and fast.

---

## 3. Build the release binary + installer

> **The installer comes from `cargo tauri build`, which requires the Tauri CLI.**
> Install it once if you haven't:
> ```sh
> cargo install tauri-cli --version "^2"
> ```
> ⚠️ Plain `cargo build --release` only produces the **standalone binary**
> (`target/release/keyport(.exe)`) — it does **not** create the `bundle/nsis/…`
> installer. You need `cargo tauri build` (Tauri CLI) for the installer/packages.

### 🪟 Windows

```powershell
cargo tauri build --bundles nsis
```

Outputs in `src-tauri/target/release/`:

| Artifact | What it is |
| --- | --- |
| `keyport.exe` | The standalone app (needs the WebView2 runtime present) |
| `bundle/nsis/Keyport_0.1.0_x64-setup.exe` | The **installer** to share |

The installer is **per-user** (no admin prompt) and uses the WebView2
**download-bootstrapper**, so it stays tiny (~a few MB) instead of embedding the
~150 MB runtime.

### 🐧 Linux

```sh
cargo tauri build --bundles deb       # Debian/Ubuntu package
# or
cargo tauri build --bundles appimage  # portable, distro-agnostic
```

Outputs in `src-tauri/target/release/`:

| Artifact | What it is |
| --- | --- |
| `keyport` | The standalone binary (needs WebKitGTK present) |
| `bundle/deb/keyport_0.1.0_amd64.deb` | Debian/Ubuntu installer |
| `bundle/appimage/keyport_0.1.0_amd64.AppImage` | Portable single-file app |

> Use `--bundles` to avoid pulling in packagers you don't have installed (e.g.
> `rpm` needs `rpmbuild`). Omit it to let the config's `"all"` build everything
> your machine can.

---

## 3½. Releasing on GitHub (automated — no local build needed)

The repo ships a GitHub Actions workflow (`.github/workflows/release.yml`) that
builds Keyport for **Windows and Linux on GitHub's runners** and attaches the
installers to a GitHub Release — so users just download and run, and you don't
need both machines.

To cut a release, push a version tag:

```sh
git tag v0.1.0
git push origin v0.1.0
```

The workflow produces a **draft** release containing:

- **Windows:** `Keyport_<ver>_x64-setup.exe` (NSIS installer)
- **Linux:** `.deb` (Debian/Ubuntu), `.AppImage` (portable), `.rpm` (Fedora)

Review it under the repo's **Releases** tab and hit **Publish**. Nothing to
configure — it uses the built-in `GITHUB_TOKEN`. (It also runs on
*workflow_dispatch* for manual triggers.)

---

## 4. Keeping it lightweight (already configured — here's why)

**`src-tauri/Cargo.toml` — release profile:**

```toml
[profile.release]
opt-level = "z"     # optimise for size
lto = true          # link-time optimisation across crates
codegen-units = 1   # better optimisation (slower compile)
panic = "abort"     # drop unwinding machinery
strip = true        # strip debug symbols
```

**Minimal dependencies / features** — only what the app uses:

- `tauri` with just the **`tray-icon`** feature.
- Two small plugins: `tauri-plugin-dialog` (folder picker) and
  `tauri-plugin-autostart` (start-on-login).
- No frontend framework, no bundler, no `node_modules`.
- Launching + app discovery use the OS's own tools (`explorer.exe` / Start Menu
  on Windows, `xdg-open` / `.desktop` files on Linux) — no heavy crates.

### Tips to avoid accidental bloat

- **Don't add Tauri features you don't use** — each can pull in extra code.
- **Keep `panic = "abort"` and `strip = true`** unless you need backtraces.
- **Release builds already drop dev tooling** (devtools/inspector) automatically.
- Inspect binary size with `cargo install cargo-bloat` → `cargo bloat --release --crates`.

---

## 5. Regenerating the app icon (optional)

The icon set in `src-tauri/icons/` (a golden ring) is already committed. To
regenerate from a single square PNG:

```sh
cargo tauri icon src-tauri/icons/icon.png
```

That produces every size Tauri needs (`.png` set + `icon.ico`).

---

## 6. Project layout

```
keyport/
├─ src/                    # Frontend — static, no build step
│  ├─ index.html           #   the ring overlay window
│  ├─ settings.html        #   the settings window
│  └─ assets/              #   style.css, ring.js, vacuum.js, settings.js
├─ src-tauri/              # Rust backend
│  ├─ src/
│  │  ├─ main.rs           #   app setup, tray, autostart, command wiring
│  │  ├─ overlay.rs        #   the ring window state machine + geometry
│  │  ├─ config.rs         #   shortcut storage + key validation
│  │  ├─ launcher.rs       #   opens folders/apps (Windows + Linux)
│  │  └─ apps.rs           #   installed-app enumeration (.lnk / .desktop)
│  ├─ icons/               #   app + tray icons
│  ├─ capabilities/        #   Tauri permission set
│  ├─ Cargo.toml
│  ├─ build.rs
│  └─ tauri.conf.json
├─ .github/workflows/      # CI: build + publish GitHub Releases (release.yml)
├─ .gitignore
├─ README.md               # user guide
└─ compile.md              # this file
```

The platform-specific code lives in `launcher.rs` and `apps.rs`, gated by
`#[cfg(...)]`. The Linux branches are written as ordinary Rust so they still
**type-check when you build on Windows** (compiled as dead code) and vice versa —
handy for catching cross-platform mistakes without both machines.

---

## 7. Notes / gotchas

- **First compile is slow** (the Tauri dependency tree is large). Subsequent
  builds are incremental and quick.
- **Linux session:** the app targets **X11** for full behaviour. Under Wayland,
  window positioning/always-on-top is restricted by the compositor.
- **Linux runtime deps:** the `.deb` declares WebKitGTK; for a raw binary, ensure
  `libwebkit2gtk-4.1`, an appindicator library, and `xdg-utils` are present.
- **Antivirus / unsigned binaries:** freshly built, unsigned executables can get
  flagged. For distribution, consider code-signing (Windows) or providing
  checksums (Linux).
