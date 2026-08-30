//! Client-side settings, remembered across launches.
//!
//! Natively they persist as a small JSON file in the platform config dir
//! (`~/.config/baylee/`). A browser has no filesystem; there the values
//! live for the session (localStorage is a deliberate later step — the
//! settings seam is the same).

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

/// Everything the client remembers.
#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ClientSettings {
    /// Scale factor for the card preview tooltip.
    pub preview_scale: f32,
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self { preview_scale: 1.0 }
    }
}

impl ClientSettings {
    /// Loads the settings (defaults on any problem — a corrupt or missing
    /// file must never stop the game from starting).
    #[must_use]
    pub fn load() -> Self {
        settings_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Persists the settings (best-effort; a read-only home dir is not
    /// worth a crash either).
    pub fn save(&self) {
        let Some(path) = settings_path() else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, text);
        }
    }
}

/// The settings file location (`None` in a browser).
fn settings_path() -> Option<std::path::PathBuf> {
    if cfg!(target_arch = "wasm32") {
        return None;
    }
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|v| !v.is_empty())
        .map_or_else(
            || std::env::var("HOME").ok().map(|h| format!("{h}/.config")),
            Some,
        )?;
    Some(std::path::PathBuf::from(base).join("baylee/client-settings.json"))
}
