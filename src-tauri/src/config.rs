//! Persistent configuration: the ring size and the user's key -> target shortcuts.
//! Stored as a single JSON file in the per-user app config directory
//! (e.g. %APPDATA%\com.keyport.app\config.json). No database, no runtime deps.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// A single shortcut: a 5-character key that opens a folder, an app, a file, or a website.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Shortcut {
    /// Exactly 5 chars, `[a-z0-9]`, unique across all shortcuts.
    pub key: String,
    /// `"folder"`, `"app"`, `"file"`, or `"web"`.
    pub kind: String,
    /// Absolute path to the folder, the app's `.lnk` / `.exe`, or a file — or a website URL.
    pub target: String,
    /// Friendly label shown in the settings list (folder name or app name).
    pub label: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    /// Visible ring diameter in logical pixels.
    #[serde(default = "default_ring_size")]
    pub ring_size: f64,
    #[serde(default)]
    pub shortcuts: Vec<Shortcut>,
}

fn default_ring_size() -> f64 {
    100.0
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ring_size: default_ring_size(),
            shortcuts: Vec::new(),
        }
    }
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("config.json"))
}

pub fn load(app: &AppHandle) -> Config {
    let path = match config_path(app) {
        Ok(p) => p,
        Err(_) => return Config::default(),
    };
    let mut cfg: Config = match fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Config::default(),
    };
    // Migrate/repair a ring size from an older build or a bad edit.
    if !(40.0..=200.0).contains(&cfg.ring_size) {
        cfg.ring_size = default_ring_size();
    }
    cfg
}

pub fn save(app: &AppHandle, cfg: &Config) -> Result<(), String> {
    let path = config_path(app)?;
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

/// Enforce the key format: exactly 5 characters, lowercase letters and digits only.
pub fn validate_key(key: &str) -> Result<(), String> {
    if key.chars().count() != 5 {
        return Err("Key must be exactly 5 characters.".into());
    }
    if !key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
        return Err("Key may only contain lowercase letters (a-z) and digits (0-9).".into());
    }
    Ok(())
}
