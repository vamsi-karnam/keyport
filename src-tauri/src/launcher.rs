//! Cross-platform target launcher.
//!
//! * Windows: `explorer.exe <target>` opens folders, launches `.lnk`/`.exe`,
//!   and opens files/URLs with their default handler — one code path.
//! * Linux:   folders/files/URLs open with `xdg-open`; an app shortcut is a
//!   `.desktop` file, whose `Exec=` line we run directly (no reliance on
//!   `gio`/`gtk-launch` being installed).
//!
//! The Linux helpers are ordinary Rust (file I/O + `std::process::Command`), so
//! they compile — and get type-checked — on every platform even though they are
//! only *called* on Linux.

use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

// Prevents a fleeting console window on Windows.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn open_target(kind: &str, target: &str) -> Result<(), String> {
    let _ = kind; // used on Linux; harmless elsewhere
    if target.trim().is_empty() {
        return Err("Shortcut has no target.".into());
    }

    #[cfg(windows)]
    {
        open_windows(target)
    }
    #[cfg(target_os = "linux")]
    {
        open_linux(kind, target)
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = target;
        Err("Keyport supports Windows and Linux only.".into())
    }
}

#[cfg(windows)]
fn open_windows(target: &str) -> Result<(), String> {
    Command::new("explorer.exe")
        .arg(target)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Failed to launch: {e}"))?;
    Ok(())
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn open_linux(kind: &str, target: &str) -> Result<(), String> {
    // An app shortcut is a .desktop file: run its Exec line so it launches the
    // same way the desktop menu would.
    if kind == "app" && target.ends_with(".desktop") {
        if let Some(exec) = desktop_exec(target) {
            return Command::new("sh")
                .arg("-c")
                .arg(&exec)
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("Failed to launch: {e}"));
        }
    }
    // Folders, files, URLs, or a plain executable path.
    Command::new("xdg-open")
        .arg(target)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to launch (is xdg-utils installed?): {e}"))
}

/// Extract the `Exec=` command from a `.desktop` file's `[Desktop Entry]`
/// group, stripping the `%`-field codes so it can be run by a shell.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn desktop_exec(path: &str) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut in_entry = false;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            if line == "[Desktop Entry]" {
                in_entry = true;
                continue;
            }
            if in_entry {
                break; // reached the next group
            }
            continue;
        }
        if in_entry {
            if let Some(rest) = line.strip_prefix("Exec=") {
                return Some(strip_field_codes(rest));
            }
        }
    }
    None
}

/// Remove `.desktop` `Exec` field codes (`%u %U %f %F %i %c %k` …). `%%` is a
/// literal percent.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn strip_field_codes(exec: &str) -> String {
    let mut out = String::new();
    let mut chars = exec.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            if let Some('%') = chars.next() {
                out.push('%');
            }
            // otherwise drop the field code character
        } else {
            out.push(c);
        }
    }
    out.trim().to_string()
}
