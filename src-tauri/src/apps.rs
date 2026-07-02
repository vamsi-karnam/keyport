//! Enumerate installed applications, dependency-free:
//!
//! * Windows: read the shell **AppsFolder** (what the Start menu's "All apps"
//!   shows) via a hidden PowerShell + `Shell.Application` COM query. This covers
//!   BOTH classic Win32 apps AND Microsoft Store / UWP apps (which have no
//!   `.lnk` shortcut). Each item's `Path` is either a filesystem path (launch it
//!   directly) or an AppUserModelID (launch via `shell:AppsFolder\<AUMID>`).
//!   Falls back to a Start-Menu `.lnk` scan if the shell query yields nothing.
//! * Linux:   scan XDG `.desktop` entries (user + system + Flatpak).
//!
//! The Linux scanner (and the Windows `.lnk` fallback) are ordinary file I/O so
//! they compile and type-check on every platform; only the relevant one runs.

use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone)]
pub struct AppEntry {
    pub name: String,
    /// A launch target the OS launcher understands: a filesystem path, a
    /// `shell:AppsFolder\<AUMID>` string (Windows), or a `.desktop` path (Linux).
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
// Windows
// --------------------------------------------------------------------------

#[cfg(windows)]
fn list_windows() -> Vec<AppEntry> {
    let mut out = apps_folder();
    if out.is_empty() {
        // Shell query blocked/failed — fall back to Start-Menu shortcuts.
        out = start_menu_lnks();
    }
    out.retain(|a| is_real_app(&a.name));
    finalize(&mut out);
    out
}

/// Enumerate the shell AppsFolder (Win32 + Store apps) via PowerShell COM.
#[cfg(windows)]
fn apps_folder() -> Vec<AppEntry> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    // NOTE: the script deliberately contains NO double-quotes. Passing a script
    // with embedded `"` to powershell.exe via `-Command` mangles across the
    // Rust→CreateProcess→PowerShell boundary; single-quote + `+` concatenation
    // round-trips cleanly. Output is one `Path|Name` line per app.
    let script = concat!(
        "[Console]::OutputEncoding=[System.Text.Encoding]::UTF8;",
        "$ErrorActionPreference='SilentlyContinue';",
        "$a=(New-Object -ComObject Shell.Application).NameSpace('shell:AppsFolder');",
        "foreach($i in $a.Items()){$i.Path + '|' + $i.Name}"
    );

    let output = match std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let text = String::from_utf8_lossy(&output.stdout);
    let mut out: Vec<AppEntry> = Vec::new();
    for line in text.lines() {
        let line = line.trim_end_matches('\r').trim();
        if line.is_empty() {
            continue;
        }
        let (raw_path, name) = match line.split_once('|') {
            Some(v) => v,
            None => continue,
        };
        let raw_path = raw_path.trim();
        let name = name.trim();
        if raw_path.is_empty() || name.is_empty() {
            continue;
        }

        let target = if is_fs_path(raw_path) {
            // Filesystem target: skip documents/installers that aren't real apps.
            if has_any_ext(
                raw_path,
                &["html", "htm", "txt", "chm", "pdf", "md", "url", "msi", "ini"],
            ) {
                continue;
            }
            raw_path.to_string()
        } else {
            // AppUserModelID (Store or Win32) — launched through the shell.
            format!("shell:AppsFolder\\{raw_path}")
        };

        out.push(AppEntry {
            name: name.to_string(),
            path: target,
        });
    }
    out
}

/// Start-Menu `.lnk` scan (fallback for Windows; type-checked everywhere).
#[cfg_attr(not(windows), allow(dead_code))]
fn start_menu_lnks() -> Vec<AppEntry> {
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

/// Drop obvious non-app entries (docs, uninstallers, "… on the web", etc.).
#[cfg_attr(not(windows), allow(dead_code))]
fn is_real_app(name: &str) -> bool {
    let n = name.to_lowercase();
    !(n.contains("uninstall")
        || n.contains("readme")
        || n.starts_with("license")
        || n.contains("help")
        || n.contains("documentation")
        || n.contains("manual")
        || n.contains("release notes")
        || n.contains("on the web")
        || n.contains("website"))
}

#[cfg_attr(not(windows), allow(dead_code))]
fn is_fs_path(p: &str) -> bool {
    let b = p.as_bytes();
    (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/'))
        || p.starts_with("\\\\")
}

#[cfg_attr(not(windows), allow(dead_code))]
fn has_any_ext(path: &str, exts: &[&str]) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| exts.iter().any(|x| e.eq_ignore_ascii_case(x)))
        .unwrap_or(false)
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
