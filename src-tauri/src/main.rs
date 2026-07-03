// Keyport — a tiny always-on-top portal that launches apps, folders, files, and
// websites from 5-character keys. Prevent a console window from appearing in Windows release
// builds (no-op on other platforms).
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod apps;
mod config;
mod launcher;
mod overlay;

use config::{Config, Shortcut};
use overlay::{EntryLayout, LaunchLayout, Mode, Overlay};

use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{
    AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};
use tauri_plugin_autostart::{ManagerExt, MacosLauncher};
use tauri_plugin_dialog::DialogExt;

// ---------------------------------------------------------------------------
// Config + shortcut commands
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_config(app: AppHandle) -> Config {
    config::load(&app)
}

#[tauri::command]
fn add_shortcut(
    app: AppHandle,
    key: String,
    kind: String,
    target: String,
    label: String,
) -> Result<Config, String> {
    let key = key.trim().to_lowercase();
    config::validate_key(&key)?;
    if kind != "folder" && kind != "app" && kind != "file" && kind != "web" {
        return Err("Shortcut kind must be 'folder', 'app', 'file', or 'web'.".into());
    }
    let target = target.trim().to_string();
    if target.is_empty() {
        return Err("Please choose a folder, app, or file, or enter a website URL first.".into());
    }

    let mut cfg = config::load(&app);
    if cfg.shortcuts.iter().any(|s| s.key == key) {
        return Err(format!("The key '{key}' is already in use."));
    }

    let label = if label.trim().is_empty() {
        // Fall back to the last path segment.
        std::path::Path::new(&target)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&target)
            .to_string()
    } else {
        label.trim().to_string()
    };

    cfg.shortcuts.push(Shortcut {
        key,
        kind,
        target,
        label,
    });
    config::save(&app, &cfg)?;
    let _ = app.emit("config-changed", &cfg);
    Ok(cfg)
}

#[tauri::command]
fn delete_shortcut(app: AppHandle, key: String) -> Result<Config, String> {
    let mut cfg = config::load(&app);
    cfg.shortcuts.retain(|s| s.key != key);
    config::save(&app, &cfg)?;
    let _ = app.emit("config-changed", &cfg);
    Ok(cfg)
}

#[tauri::command]
fn set_ring_size(app: AppHandle, size: f64) -> Result<Config, String> {
    let clamped = size.clamp(overlay::RING_MIN, overlay::RING_MAX);
    let mut cfg = config::load(&app);
    cfg.ring_size = clamped;
    config::save(&app, &cfg)?;
    // Live-resize the idle ring window so the change is visible immediately.
    if let Some(w) = app.get_webview_window("main") {
        let overlay = app.state::<Overlay>();
        let _ = overlay::apply_ring_size(&w, &overlay, clamped);
    }
    let _ = app.emit("config-changed", &cfg);
    Ok(cfg)
}

#[tauri::command]
fn open_shortcut(app: AppHandle, key: String) -> Result<(), String> {
    let cfg = config::load(&app);
    let sc = cfg
        .shortcuts
        .into_iter()
        .find(|s| s.key == key)
        .ok_or_else(|| format!("No shortcut for key '{key}'."))?;
    launcher::open_target(&sc.kind, &sc.target)
}

// ---------------------------------------------------------------------------
// App enumeration + folder picker
// ---------------------------------------------------------------------------

#[tauri::command]
fn list_installed_apps() -> Vec<apps::AppEntry> {
    apps::list_installed()
}

#[tauri::command]
fn pick_folder(app: AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .blocking_pick_folder()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
fn pick_file(app: AppHandle) -> Option<String> {
    // No type filter — a file shortcut may point at any document, media, or
    // other file; it opens with the OS default handler at launch time.
    app.dialog()
        .file()
        .blocking_pick_file()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().to_string())
}

// ---------------------------------------------------------------------------
// Autostart
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_autostart(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let al = app.autolaunch();
    if enabled {
        al.enable().map_err(|e| e.to_string())
    } else {
        al.disable().map_err(|e| e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Window management
// ---------------------------------------------------------------------------

#[tauri::command]
fn open_entry(window: WebviewWindow, overlay: State<Overlay>) -> Result<EntryLayout, String> {
    let layout = overlay::open_entry(&window, &overlay)?;
    // Focus so the user can type immediately, and so a click elsewhere blurs
    // the window (which we use to dismiss the key box).
    let _ = window.set_focus();
    Ok(layout)
}

#[tauri::command]
fn close_entry(window: WebviewWindow, overlay: State<Overlay>) -> Result<(), String> {
    overlay::close_entry(&window, &overlay)
}

#[tauri::command]
fn start_launch(window: WebviewWindow, overlay: State<Overlay>) -> Result<LaunchLayout, String> {
    overlay::start_launch(&window, &overlay)
}

#[tauri::command]
fn finish_launch(window: WebviewWindow, overlay: State<Overlay>) -> Result<(), String> {
    overlay::finish_launch(&window, &overlay)
}

#[tauri::command]
fn reset_ring(app: AppHandle) -> Result<(), String> {
    let w = app
        .get_webview_window("main")
        .ok_or("Ring window not found.")?;
    overlay::spawn_default(&w, &app.state::<Overlay>())
}

#[tauri::command]
fn open_settings(app: AppHandle) {
    // Opening = just showing the pre-created (hidden) window. We never *create*
    // the webview in response to a command — runtime creation from a command
    // renders a blank window on Windows. Marshalled to the main thread for safety.
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || open_settings_window(&handle));
}

#[tauri::command]
fn close_settings(app: AppHandle) {
    // Hide, don't destroy — the same window is reused every time it's opened.
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.hide();
    }
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

/// Build the Settings window once, hidden, on the main thread (called from
/// `setup`). Frameless + opaque: its only close control is the custom top-right
/// × in settings.html (minimalist), opaque so it always renders solid. Because
/// it is pre-created and merely shown/hidden afterwards, opening it works from
/// any trigger (tray or an invoked command) — no blank-window bug.
fn create_settings_window(app: &AppHandle) {
    if app.get_webview_window("settings").is_some() {
        return;
    }
    let _ = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("Keyport Settings")
        .inner_size(560.0, 660.0)
        .min_inner_size(480.0, 560.0)
        .resizable(true)
        .decorations(false)
        .visible(false)
        .center()
        .build();
}

fn open_settings_window(app: &AppHandle) {
    create_settings_window(app); // no-op if it already exists
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn main() {
    tauri::Builder::default()
        // Single instance MUST be registered first. If Keyport is already
        // running, a second launch (e.g. clicking the icon again) fires this
        // callback in the ORIGINAL instance and then exits — so no duplicate
        // window. We use it to snap the ring back to its default corner.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = overlay::spawn_default(&w, &app.state::<Overlay>());
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(Overlay::new(100.0))
        .setup(|app| {
            // Load the saved ring size before sizing the window, so the idle
            // window matches the ring on first paint.
            let cfg = config::load(app.handle());
            {
                let overlay = app.state::<Overlay>();
                *overlay.ring_size.lock().unwrap() = cfg.ring_size;
            }

            // Position and reveal the ring on the primary monitor's corner.
            if let Some(main) = app.get_webview_window("main") {
                let overlay = app.state::<Overlay>();
                let _ = overlay::spawn_default(&main, &overlay);
                let _ = main.show();
            }

            // System tray — the only always-visible control surface, since the
            // ring has no taskbar entry.
            let settings_i = MenuItemBuilder::with_id("settings", "Settings\u{2026}").build(app)?;
            let reset_i =
                MenuItemBuilder::with_id("reset", "Reset ring position").build(app)?;
            let quit_i = MenuItemBuilder::with_id("quit", "Quit Keyport").build(app)?;
            let menu = MenuBuilder::new(app)
                .items(&[&settings_i, &reset_i, &quit_i])
                .build()?;

            let mut tray_builder = TrayIconBuilder::with_id("keyport-tray")
                .tooltip("Keyport")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "settings" => open_settings_window(app),
                    "reset" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = overlay::spawn_default(&w, &app.state::<Overlay>());
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }
            let _tray = tray_builder.build(app)?;

            // Pre-create the Settings window (hidden) on the main thread so it
            // renders correctly; opening it later is just show()/hide(), which
            // works from the tray OR an invoked command (right-click).
            create_settings_window(app.handle());

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            // Clicking away from the ring while the key box is open dismisses it.
            if let WindowEvent::Focused(false) = event {
                let overlay = window.state::<Overlay>();
                if overlay.mode() == Mode::Entry {
                    if let Some(w) = window.get_webview_window("main") {
                        let _ = overlay::close_entry(&w, &overlay);
                    }
                    let _ = window.emit("entry-dismissed", ());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            add_shortcut,
            delete_shortcut,
            set_ring_size,
            open_shortcut,
            list_installed_apps,
            pick_folder,
            pick_file,
            get_autostart,
            set_autostart,
            open_entry,
            close_entry,
            start_launch,
            finish_launch,
            reset_ring,
            open_settings,
            close_settings,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running Keyport");
}
