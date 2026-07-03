<div align="center">

# 🌀 Keyport

**A tiny portal that lives on your desktop and opens your favourite folders, apps, files, and websites from a 5-character key.**

*A fun, lightweight little companion that sits quietly in the corner of your screen ready to teleport you to your next adventure.*

</div>

---

## What is Keyport?

Somewhere along the way, our desktops got boring.
We started focusing way too much on efficiency, and way less on having fun.
Remember the Windows XP days? Apps with personality, tiny widgets, weird one-purpose tools someone built just because they could, little pets that wandered across your desktop. 
Purely local, purely offline.

Keyport is a keyboard-speed launcher with a bit of theatre. That's the whole idea.
Give any folder, app, file, or website a short, memorable 5-character key (like a4g56, docs1, or spot5). Then, whenever you want that thing, launch Keyport and teleport to your next application adventure.

Keyport runs on **Windows** and **Linux**.

<div align="center">

![Keyport in action — click the ring, type a key like web01 or vcode, and your app launches out of the portal](media/keyport-demo.gif)

</div>

---

## Using Keyport

*(The controls are the same on every OS — only the small platform details in
"Platform notes" below differ.)*

### The ring

When Keyport starts, the ring appears in the **bottom-right corner** of your
primary screen and stays **on top of everything**.

| Action | What it does |
| --- | --- |
| **Click** the ring | Opens the key box — a small frosted panel to type a key |
| **Click-and-drag** the ring | Moves the portal anywhere, across any monitor |
| **Right-click** the ring | Opens **Settings** (also in the system tray) |
| Click **away** from the box | Dismisses it — the ring goes back to idle |

### Launching something

1. **Click** the ring. The void lights up with a blue nebula glow.
2. **Type** your 5-character key. The box turns **green** when it matches a
   shortcut, **red** when it doesn't.
3. Press **Enter** (or click **Enter**). The portal winds up into a vacuum for a
   few seconds, then launches your folder, app, file, or website.

If a key doesn't match anything, the box gives a little shake — nothing opens.

### Setting up your keys

**Right-click the ring** (or use the **system-tray icon** → *Settings…*) to open
Settings. In the settings window you can:

- **Add a shortcut** — pick a 5-character key, then choose one of:
  - **Folder** — browse to any folder on your PC,
  - **App** — search your installed apps and pick one,
  - **File** — browse to any file (a document, PDF, image, video…); it opens in
    whatever app your system uses for that file type, or
  - **Web** — enter a website URL; it opens in your default browser.
- **See and remove** your existing shortcuts.
- **Resize the ring** with the slider, if you'd like it bigger or smaller.
- **Start on login** — toggle whether Keyport launches when you log in.

**Key rules:** exactly **5 characters**, only **lowercase letters (a–z)** and
**digits (0–9)** — no spaces or symbols. Each key must be unique. The *order*
matters, so `a4g56`, `4ag56`, and `56g4a` are all different keys.

### The tray icon

Keyport has no taskbar button (it's just a floating ring), so it lives in your
**system tray**. Right-click (or click) it for:

- **Settings…**
- **Reset ring position** — snaps the portal back to the bottom-right corner if
  it ever wanders off-screen.
- **Quit Keyport**

---

## Running Keyport

### Windows

- Run the installer (`Keyport_…_x64-setup.exe`) or the standalone `keyport.exe` —
  no admin needed.
- Works on Windows 10/11 (the WebView2 runtime it uses is already installed;
  the installer fetches it if not).

### Linux

- Install the **`.deb`** with `sudo apt install ./keyport_*_amd64.deb` (the
  leading `./` matters — it lets apt pull the WebKitGTK/`xdg-utils` dependencies
  for you). If you install with `dpkg -i` and it complains about missing
  dependencies, run `sudo apt --fix-broken install` to pull them in.
- Alternatively run the **`.AppImage`**, or just run the built `keyport` binary.
- You need a desktop with **WebKitGTK** (`libwebkit2gtk-4.1`) and **`xdg-utils`**
  (both are standard on mainstream desktops). Runs on both **X11** and
  **Wayland** — see Platform notes for the Wayland details.

---

## Platform notes

### Windows

- **Installed-apps list** comes from your **Start-Menu shortcuts**.
- **Always on top:** the ring floats above normal and full-screen-*windowed*
  apps. A game in *true exclusive full-screen* may cover it — that's an OS
  limitation, not a bug.

### Linux

- **Installed-apps list** comes from your **`.desktop` entries** (system, user,
  and Flatpak apps).
- **Session (X11 & Wayland):** Keyport runs natively on both, and renders
  crisply on **HiDPI and mixed-DPI multi-monitor** setups (each monitor at its
  own scale). Wayland forbids apps from setting their own window position, so
  Keyport is written to not depend on it: the launch gravity effect uses
  **compositor fullscreen** (reliable on any monitor) with the well drawn at
  screen centre, and the key box grows in place — nothing jumps off-screen.
  - **One Wayland-only difference (by design):** because the compositor won't
    reveal the ring's on-screen position, the launch effect **centres on screen**
    (the ring glides to the middle for the animation) rather than forming at the
    ring's corner as it does on Windows/X11. The app still opens exactly the same.
  - **Window placement** (initial corner, "Reset ring position") is
    compositor-controlled on Wayland — if the ring doesn't start where you want,
    just **drag it** (dragging works). On an **X11/Xorg** session it spawns in the
    corner as on Windows.
- **Transparency** needs a running compositor (most desktops have one); without
  one the ring's background may look dark instead of see-through.
- **System tray:** some desktops (notably **GNOME**) don't show tray icons
  without an extension such as *"AppIndicator and KStatusNotifierItem Support"*.
  That's fine — you can always **right-click the ring** to open Settings (which
  has its own *Quit* and *Reset position* buttons). Install the extension only if
  you'd also like the tray icon.

*(On both systems, **your shortcuts are private** and stored only on your PC.
The launch animation briefly draws a gravity effect around the ring, but
everything else on screen stays fully clickable the whole time.)*

---

## Ideas for keys

- `spot5` → Spotify · `code1` → your editor · `mail0` → your mail app
- `dwnld` → your Downloads folder · `proj7` → a project folder · `scrns` → Screenshots
- `cv001` → your résumé PDF · `notes` → a notes doc · `song1` → a favourite track (files)
- `gh001` → a GitHub repo · `maps1` → Google Maps · `yt001` → YouTube (websites)

Have fun. 🌀
