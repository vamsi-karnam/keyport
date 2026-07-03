//! The overlay window state machine.
//!
//! One transparent, frameless, always-on-top window hosts the ring. Rather than
//! keep a large click-through window (which needs constant cursor polling), we
//! resize the window to match its current state and keep the ring pinned to a
//! stable on-screen "anchor" point:
//!
//!   * Idle  -> a small window sized to the ring (ring + margin for glow).
//!   * Entry -> grows toward screen centre so the key box is fully on-screen.
//!   * Launch-> expands to the full monitor, but is made click-through so every
//!              other window underneath stays fully usable; the gravity effect
//!              is drawn only in a small radius around the ring.
//!
//! Because the ring diameter is user-configurable, the idle window size (and
//! therefore every derived dimension) is computed at runtime from the current
//! ring size held in [`Overlay`].

use serde::Serialize;
use std::sync::Mutex;
use tauri::{LogicalPosition, LogicalSize, WebviewWindow};

/// Transparent margin around the ring inside the idle window (room for the
/// spin/nebula glow, and a comfortable grab zone).
pub const RING_MARGIN: f64 = 48.0;
/// Gap from the screen edge when spawning / resetting to the default corner.
pub const SPAWN_MARGIN: f64 = 24.0;
/// Ring-size clamp (logical px diameter). Mirrors the settings slider.
pub const RING_MIN: f64 = 40.0;
pub const RING_MAX: f64 = 200.0;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Idle,
    Entry,
    Launch,
}

#[derive(Default)]
pub struct Overlay {
    pub mode: Mutex<Mode>,
    /// Ring-centre offset within the *current* window (logical px).
    pub ring_offset: Mutex<(f64, f64)>,
    /// Ring-centre screen position captured when a launch begins, used to
    /// restore the idle window afterwards.
    pub launch_anchor: Mutex<(f64, f64)>,
    /// Current visible ring diameter (logical px).
    pub ring_size: Mutex<f64>,
}

impl Overlay {
    pub fn new(ring_size: f64) -> Self {
        let ring_size = ring_size.clamp(RING_MIN, RING_MAX);
        let idle = ring_size + RING_MARGIN;
        Overlay {
            mode: Mutex::new(Mode::Idle),
            ring_offset: Mutex::new((idle / 2.0, idle / 2.0)),
            launch_anchor: Mutex::new((0.0, 0.0)),
            ring_size: Mutex::new(ring_size),
        }
    }
    pub fn set_mode(&self, m: Mode) {
        *self.mode.lock().unwrap() = m;
    }
    pub fn mode(&self) -> Mode {
        *self.mode.lock().unwrap()
    }

    /// (idle_size, pad, entry_w, entry_h) derived from the current ring size.
    pub fn dims(&self) -> (f64, f64, f64, f64) {
        let ring = *self.ring_size.lock().unwrap();
        let idle = ring + RING_MARGIN;
        let pad = idle / 2.0;
        let entry_w = (idle + 140.0).max(340.0);
        let entry_h = idle + 130.0;
        (idle, pad, entry_w, entry_h)
    }
}

#[derive(Serialize)]
pub struct EntryLayout {
    pub corner: String, // "br" | "bl" | "tr" | "tl"
    pub ring_x: f64,
    pub ring_y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Serialize)]
pub struct LaunchLayout {
    pub center_x: f64, // ring centre relative to the fullscreen overlay origin
    pub center_y: f64,
    pub width: f64,
    pub height: f64,
    /// When true (Linux/Wayland), the overlay was made fullscreen by the
    /// compositor and the effect must be drawn at the window centre computed in
    /// the webview (`center_x/y` are unused). When false (Windows/macOS), the
    /// overlay was pinned to the monitor origin and `center_x/y` give the ring's
    /// real position within it.
    pub centered: bool,
}

fn logical_pos(win: &WebviewWindow) -> Result<(f64, f64), String> {
    let scale = win.scale_factor().map_err(|e| e.to_string())?;
    let p = win
        .outer_position()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(scale);
    Ok((p.x, p.y))
}

fn logical_outer_size(win: &WebviewWindow) -> Result<(f64, f64), String> {
    let scale = win.scale_factor().map_err(|e| e.to_string())?;
    let s = win
        .outer_size()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(scale);
    Ok((s.width, s.height))
}

/// Returns the current monitor rect as logical (x, y, w, h), falling back to the
/// primary monitor if the window's current monitor can't be resolved.
fn monitor_rect(win: &WebviewWindow) -> Result<(f64, f64, f64, f64), String> {
    let scale = win.scale_factor().map_err(|e| e.to_string())?;
    let mon = win
        .current_monitor()
        .map_err(|e| e.to_string())?
        .or(win.primary_monitor().map_err(|e| e.to_string())?)
        .ok_or_else(|| "No monitor available".to_string())?;
    let p = mon.position().to_logical::<f64>(scale);
    let s = mon.size().to_logical::<f64>(scale);
    Ok((p.x, p.y, s.width, s.height))
}

/// Place the idle window at the bottom-right corner of the primary monitor.
pub fn spawn_default(win: &WebviewWindow, overlay: &Overlay) -> Result<(), String> {
    let (idle, pad, _, _) = overlay.dims();
    let scale = win.scale_factor().map_err(|e| e.to_string())?;
    let mon = win
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .or(win.current_monitor().map_err(|e| e.to_string())?)
        .ok_or_else(|| "No monitor available".to_string())?;
    let p = mon.position().to_logical::<f64>(scale);
    let s = mon.size().to_logical::<f64>(scale);
    let x = p.x + s.width - idle - SPAWN_MARGIN;
    let y = p.y + s.height - idle - SPAWN_MARGIN;
    win.set_size(LogicalSize::new(idle, idle))
        .map_err(|e| e.to_string())?;
    win.set_position(LogicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    *overlay.ring_offset.lock().unwrap() = (pad, pad);
    overlay.set_mode(Mode::Idle);
    Ok(())
}

/// Change the ring size and, if idle, resize the idle window around the ring's
/// current centre so it stays put.
pub fn apply_ring_size(win: &WebviewWindow, overlay: &Overlay, size: f64) -> Result<(), String> {
    let size = size.clamp(RING_MIN, RING_MAX);
    let idle_only = overlay.mode() == Mode::Idle;
    // Capture the current window centre *before* changing the size.
    let center = if idle_only {
        let (wx, wy) = logical_pos(win)?;
        let (cw, ch) = logical_outer_size(win)?;
        Some((wx + cw / 2.0, wy + ch / 2.0))
    } else {
        None
    };
    *overlay.ring_size.lock().unwrap() = size;
    if let Some((cx, cy)) = center {
        let (idle, pad, _, _) = overlay.dims();
        win.set_size(LogicalSize::new(idle, idle))
            .map_err(|e| e.to_string())?;
        win.set_position(LogicalPosition::new(cx - pad, cy - pad))
            .map_err(|e| e.to_string())?;
        *overlay.ring_offset.lock().unwrap() = (pad, pad);
    }
    Ok(())
}

/// Grow the idle window into the key-entry window, keeping the ring on its anchor.
///
/// Windows/macOS: the ring stays pinned to a fixed screen anchor and the window
/// grows toward the screen centre so the key box is fully visible.
///
/// Linux/Wayland: absolute window positioning is unavailable, so instead of
/// re-anchoring we keep the ring where it is (top-left of the grown window, at
/// its idle offset) and grow the window in place; the key box is placed just
/// below the ring. This keeps the ring from jumping and works identically on
/// single- and multi-monitor setups because it needs no screen coordinates.
pub fn open_entry(win: &WebviewWindow, overlay: &Overlay) -> Result<EntryLayout, String> {
    let (_idle, pad, entry_w, entry_h) = overlay.dims();

    if cfg!(target_os = "linux") {
        // Wayland: grow in place, ring stays put, key box drops just below it.
        win.set_size(LogicalSize::new(entry_w, entry_h))
            .map_err(|e| e.to_string())?;
        *overlay.ring_offset.lock().unwrap() = (pad, pad);
        overlay.set_mode(Mode::Entry);
        Ok(EntryLayout {
            corner: "tl".into(),
            ring_x: pad,
            ring_y: pad,
            width: entry_w,
            height: entry_h,
        })
    } else {
        let (wx, wy) = logical_pos(win)?;
        let anchor = (wx + pad, wy + pad); // ring centre while idle
        let (mx, my, mw, mh) = monitor_rect(win)?;
        let center_x = mx + mw / 2.0;
        let center_y = my + mh / 2.0;
        let right = anchor.0 >= center_x;
        let bottom = anchor.1 >= center_y;

        // Grow toward the screen centre so the key box is always fully visible.
        let (corner, ring_x, ring_y) = match (bottom, right) {
            (true, true) => ("br", entry_w - pad, entry_h - pad),
            (true, false) => ("bl", pad, entry_h - pad),
            (false, true) => ("tr", entry_w - pad, pad),
            (false, false) => ("tl", pad, pad),
        };

        let mut nx = anchor.0 - ring_x;
        let mut ny = anchor.1 - ring_y;
        nx = nx.clamp(mx, (mx + mw - entry_w).max(mx));
        ny = ny.clamp(my, (my + mh - entry_h).max(my));

        win.set_size(LogicalSize::new(entry_w, entry_h))
            .map_err(|e| e.to_string())?;
        win.set_position(LogicalPosition::new(nx, ny))
            .map_err(|e| e.to_string())?;

        *overlay.ring_offset.lock().unwrap() = (ring_x, ring_y);
        overlay.set_mode(Mode::Entry);

        Ok(EntryLayout {
            corner: corner.into(),
            ring_x,
            ring_y,
            width: entry_w,
            height: entry_h,
        })
    }
}

/// Collapse back to the idle window, keeping the ring on its anchor.
pub fn close_entry(win: &WebviewWindow, overlay: &Overlay) -> Result<(), String> {
    let (idle, pad, _, _) = overlay.dims();

    if cfg!(target_os = "linux") {
        // Shrink in place; the ring stays at its idle offset (no repositioning).
        win.set_size(LogicalSize::new(idle, idle))
            .map_err(|e| e.to_string())?;
    } else {
        let (wx, wy) = logical_pos(win)?;
        let (rx, ry) = *overlay.ring_offset.lock().unwrap();
        let anchor = (wx + rx, wy + ry);
        win.set_size(LogicalSize::new(idle, idle))
            .map_err(|e| e.to_string())?;
        win.set_position(LogicalPosition::new(anchor.0 - pad, anchor.1 - pad))
            .map_err(|e| e.to_string())?;
    }
    *overlay.ring_offset.lock().unwrap() = (pad, pad);
    overlay.set_mode(Mode::Idle);
    Ok(())
}

/// Expand to a fullscreen, click-through overlay for the launch animation.
///
/// Windows/macOS: resize to the monitor rect and pin to its origin, so the well
/// can be drawn at the ring's real position within the overlay.
///
/// Linux/Wayland: absolute positioning is unavailable and XWayland can't render
/// per-monitor HiDPI, so we ask the *compositor* for fullscreen (which it
/// honours, on whichever monitor the ring is on, rendered at that monitor's true
/// scale) and let the webview draw the well at the window centre. This is
/// monitor-agnostic: single-monitor, multi-monitor, and mixed-DPI all use it.
pub fn start_launch(win: &WebviewWindow, overlay: &Overlay) -> Result<LaunchLayout, String> {
    // Click-through first, so the moment we cover the screen nothing is blocked.
    win.set_ignore_cursor_events(true)
        .map_err(|e| e.to_string())?;
    overlay.set_mode(Mode::Launch);

    if cfg!(target_os = "linux") {
        win.set_fullscreen(true).map_err(|e| e.to_string())?;
        // center_x/y are computed in the webview from innerWidth; report the
        // centered flag so the frontend knows which path to take.
        Ok(LaunchLayout {
            center_x: 0.0,
            center_y: 0.0,
            width: 0.0,
            height: 0.0,
            centered: true,
        })
    } else {
        let (wx, wy) = logical_pos(win)?;
        let (rx, ry) = *overlay.ring_offset.lock().unwrap();
        let anchor = (wx + rx, wy + ry); // ring centre on screen right now
        *overlay.launch_anchor.lock().unwrap() = anchor;

        let (mx, my, mw, mh) = monitor_rect(win)?;
        win.set_size(LogicalSize::new(mw, mh))
            .map_err(|e| e.to_string())?;
        win.set_position(LogicalPosition::new(mx, my))
            .map_err(|e| e.to_string())?;

        Ok(LaunchLayout {
            center_x: anchor.0 - mx,
            center_y: anchor.1 - my,
            width: mw,
            height: mh,
            centered: false,
        })
    }
}

/// Tear down the fullscreen overlay and restore the idle ring in place.
pub fn finish_launch(win: &WebviewWindow, overlay: &Overlay) -> Result<(), String> {
    let (idle, pad, _, _) = overlay.dims();
    win.set_ignore_cursor_events(false)
        .map_err(|e| e.to_string())?;

    if cfg!(target_os = "linux") {
        // Leave fullscreen and return to the idle ring size; the compositor
        // restores placement (no client-side positioning available/needed).
        win.set_fullscreen(false).map_err(|e| e.to_string())?;
        win.set_size(LogicalSize::new(idle, idle))
            .map_err(|e| e.to_string())?;
    } else {
        let (ax, ay) = *overlay.launch_anchor.lock().unwrap();
        win.set_size(LogicalSize::new(idle, idle))
            .map_err(|e| e.to_string())?;
        win.set_position(LogicalPosition::new(ax - pad, ay - pad))
            .map_err(|e| e.to_string())?;
    }
    *overlay.ring_offset.lock().unwrap() = (pad, pad);
    overlay.set_mode(Mode::Idle);
    Ok(())
}
