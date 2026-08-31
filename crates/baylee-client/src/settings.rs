//! Client-side settings, remembered across launches.
//!
//! Natively they persist as a small JSON file in the platform config dir
//! (`~/.config/baylee/`); in a browser the same JSON lives in `localStorage`,
//! scoped to the origin the client is served from. Both back ends are
//! best-effort by design: a corrupt, missing or unreadable store must never
//! stop the game from starting.

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
    /// store must never stop the game from starting).
    #[must_use]
    pub fn load() -> Self {
        store::read()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Persists the settings (best-effort; neither a read-only home dir nor a
    /// browser with site data blocked is worth a crash).
    pub fn save(&self) {
        if let Ok(text) = serde_json::to_string_pretty(self) {
            store::write(&text);
        }
    }
}

/// The native back end: a JSON file in the platform config dir.
#[cfg(not(target_arch = "wasm32"))]
mod store {
    /// Reads the settings JSON, or `None` if there is nothing to read.
    pub fn read() -> Option<String> {
        std::fs::read_to_string(path()?).ok()
    }

    /// Writes the settings JSON, creating the config dir if needed.
    pub fn write(text: &str) {
        let Some(path) = path() else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, text);
    }

    /// The settings file location.
    fn path() -> Option<std::path::PathBuf> {
        let base = std::env::var("XDG_CONFIG_HOME")
            .ok()
            .filter(|v| !v.is_empty())
            .map_or_else(
                || std::env::var("HOME").ok().map(|h| format!("{h}/.config")),
                Some,
            )?;
        Some(std::path::PathBuf::from(base).join("baylee/client-settings.json"))
    }
}

/// The browser back end: `localStorage`, which survives a reload and is scoped
/// to the origin the client is served from.
#[cfg(target_arch = "wasm32")]
mod store {
    /// The `localStorage` key holding the settings JSON. Namespaced because a
    /// browser origin is shared with whatever else is served from it.
    const KEY: &str = "baylee:client-settings";

    /// Reads the settings JSON, or `None` if there is nothing to read.
    pub fn read() -> Option<String> {
        storage()?.get_item(KEY).ok().flatten()
    }

    /// Writes the settings JSON; a full or disabled store is silently ignored.
    pub fn write(text: &str) {
        if let Some(storage) = storage() {
            let _ = storage.set_item(KEY, text);
        }
    }

    /// The origin's `localStorage`, or `None` when it is unavailable.
    ///
    /// Every step here is genuinely fallible: there is no window off the main
    /// thread, and a browser configured to block site data throws on the
    /// `local_storage` accessor itself rather than returning an empty store.
    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::ClientSettings;

    /// Both back ends hand `load` whatever a previous version wrote, so the
    /// decode has to survive a store that predates a field. `#[serde(default)]`
    /// is what buys that, and it is easy to drop by accident.
    #[test]
    fn a_store_missing_fields_decodes_to_defaults() {
        let settings: ClientSettings = serde_json::from_str("{}").expect("empty object decodes");
        assert!(
            (settings.preview_scale - ClientSettings::default().preview_scale).abs() < f32::EPSILON
        );
    }

    #[test]
    fn settings_round_trip_through_json() {
        let written = ClientSettings {
            preview_scale: 1.75,
        };
        let text = serde_json::to_string_pretty(&written).expect("serializes");
        let read: ClientSettings = serde_json::from_str(&text).expect("decodes");
        assert!((read.preview_scale - 1.75).abs() < f32::EPSILON);
    }
}
