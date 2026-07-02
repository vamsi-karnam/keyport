//! Enumerate installed applications, dependency-free:
//!
//! * Windows: scan Start-Menu `.lnk` shortcuts (current user + all users) and
//!   launch the shortcut directly.
//! * Linux:   scan XDG `.desktop` entries (user + system + Flatpak) and launch
//!   via the entry's `Exec` line (see `launcher.rs`).
//!
//! Both scanners are ordinary file I/O, so they compile and type-check on every
//! platform; only the relevant one is called.

use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone)]
pub struct AppEntry {
    pub name: String,
    pub path: String,
}

pub fn list_installed() -> Vec<AppEntry> {
    #[cfg(windows)]
    {
        list_windows()
    }
    #[cfg(target_os = "linux")]
    {
        list_linux()
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Vec::new()
    }
}

// --------------------------------------------------------------------------
// Windows — Start Menu .lnk shortcuts
// --------------------------------------------------------------------------

#[cfg_attr(not(windows), allow(dead_code))]
fn list_windows() -> Vec<AppEntry> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(program_data) = std::env::var("ProgramData") {
        roots.push(Path::new(&program_data).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Ok(app_data) = std::env::var("AppData") {
        roots.push(Path::new(&app_data).join(r"Microsoft\Windows\Start Menu\Programs"));
    }

    let mut out: Vec<AppEntry> = Vec::new();
    for root in roots {
        collect_lnks(&root, &mut out, 0);
    }

    out.retain(|a| {
        let n = a.name.to_lowercase();
        !(n.contains("uninstall")
            || n.contains("readme")
            || n.starts_with("license")
            || n.contains("help")
            || n.contains("documentation"))
    });
    finalize(&mut out);
    out
}

#[cfg_attr(not(windows), allow(dead_code))]
fn collect_lnks(dir: &Path, out: &mut Vec<AppEntry>, depth: usize) {
    if depth > 6 {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_lnks(&path, out, depth + 1);
        } else if has_ext(&path, "lnk") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                out.push(AppEntry {
                    name: stem.to_string(),
                    path: path.to_string_lossy().to_string(),
                });
            }
        }
    }
}

// --------------------------------------------------------------------------
// Linux — XDG .desktop entries
// --------------------------------------------------------------------------

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn list_linux() -> Vec<AppEntry> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(Path::new(&home).join(".local/share/applications"));
        dirs.push(Path::new(&home).join(".local/share/flatpak/exports/share/applications"));
    }
    dirs.push(PathBuf::from("/usr/share/applications"));
    dirs.push(PathBuf::from("/usr/local/share/applications"));
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
    if let Ok(xdg) = std::env::var("XDG_DATA_DIRS") {
        for d in xdg.split(':').filter(|s| !s.is_empty()) {
            dirs.push(Path::new(d).join("applications"));
        }
    }

    let mut out: Vec<AppEntry> = Vec::new();
    for dir in dirs {
        collect_desktops(&dir, &mut out, 0);
    }
    finalize(&mut out);
    out
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn collect_desktops(dir: &Path, out: &mut Vec<AppEntry>, depth: usize) {
    if depth > 4 {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_desktops(&path, out, depth + 1);
        } else if has_ext(&path, "desktop") {
            if let Some(name) = parse_desktop_name(&path) {
                out.push(AppEntry {
                    name,
                    path: path.to_string_lossy().to_string(),
                });
            }
        }
    }
}

/// Returns the display `Name` of a *visible* `Application` entry, else `None`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_desktop_name(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut in_entry = false;
    let mut name: Option<String> = None;
    let mut is_app = true; // assume Application when Type is absent
    let mut hidden = false;
    for line in content.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            if l == "[Desktop Entry]" {
                in_entry = true;
                continue;
            }
            if in_entry {
                break; // only the main group matters
            }
            continue;
        }
        if !in_entry {
            continue;
        }
        if let Some(v) = l.strip_prefix("Name=") {
            if name.is_none() {
                name = Some(v.trim().to_string());
            }
        } else if let Some(v) = l.strip_prefix("Type=") {
            is_app = v.trim().eq_ignore_ascii_case("Application");
        } else if let Some(v) = l.strip_prefix("NoDisplay=") {
            if v.trim().eq_ignore_ascii_case("true") {
                hidden = true;
            }
        } else if let Some(v) = l.strip_prefix("Hidden=") {
            if v.trim().eq_ignore_ascii_case("true") {
                hidden = true;
            }
        }
    }
    if hidden || !is_app {
        return None;
    }
    name
}

// --------------------------------------------------------------------------
// Shared helpers
// --------------------------------------------------------------------------

fn has_ext(path: &Path, ext: &str) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case(ext))
        .unwrap_or(false)
}

/// Sort by name and drop case-insensitive duplicates.
fn finalize(out: &mut Vec<AppEntry>) {
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
}
