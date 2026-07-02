<div align="center">

# 🌀 Keyport

**A tiny portal that lives on your desktop and opens your favourite folders and apps from a 5-character key.**

*A fun, lightweight little companion that sits quietly in the corner of your screen ready to teleport you to your next adventure.*

</div>

---

## What is Keyport?

Somewhere along the way, our desktops got boring.
We started focusing way too much on efficiency, and way less on having fun.
Remember the Windows XP days? Apps with personality, tiny widgets, weird one-purpose tools someone built just because they could, little pets that wandered across your desktop. 
Purely local, purely offline.

Keyport is a keyboard-speed launcher with a bit of theatre. That's the whole idea.
Give any folder or app a short, memorable 5-character key (like a4g56, docs1, or spot5). Then, whenever you want that thing, launch Keyport and teleport to your next application adventure.

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
   few seconds, then launches your folder or app.

If a key doesn't match anything, the box gives a little shake — nothing opens.

### Setting up your keys

**Right-click the ring** (or use the **system-tray icon** → *Settings…*) to open
Settings. In the settings window you can:

- **Add a shortcut** — pick a 5-character key, then choose either:
  - **Folder** — browse to any folder on your PC, or
  - **App** — search your installed apps and pick one.
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

### 🪟 Windows

- Run the installer (`Keyport_…_x64-setup.exe`) or the standalone `keyport.exe` —
  no admin needed.
- Works on Windows 10/11 (the WebView2 runtime it uses is already installed;
  the installer fetches it if not).

### 🐧 Linux

- Install the **`.deb`** (Debian/Ubuntu) or run the **`.AppImage`**, or just run
  the built `keyport` binary.
- You need a desktop with **WebKitGTK** (`libwebkit2gtk-4.1`) and **`xdg-utils`**
  (both are standard on mainstream desktops). Works best on an **X11 (Xorg)**
  session — see Platform notes.

---

## Platform notes

### 🪟 Windows

- **Installed-apps list** comes from your **Start-Menu shortcuts**.
- **Always on top:** the ring floats above normal and full-screen-*windowed*
  apps. A game in *true exclusive full-screen* may cover it — that's an OS
  limitation, not a bug.

### 🐧 Linux

- **Installed-apps list** comes from your **`.desktop` entries** (system, user,
  and Flatpak apps).
- **Session:** Keyport behaves best on **X11**. On **Wayland**, the compositor
  controls window placement, so the ring may not spawn in the corner, stay on
  top, or drag as expected — log into an **Xorg** session for the intended feel.
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

Have fun. 🌀
