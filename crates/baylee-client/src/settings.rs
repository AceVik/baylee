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
    /// Language card text is requested in (`"en"`, `"de"`, …).
    ///
    /// The gateway falls back to English field by field, so an unavailable
    /// translation costs nothing but English text.
    pub lang: String,
    /// Show the constructed card face instead of the printed image.
    ///
    /// The modifier key (Cmd or Alt) toggles the face for as long as it is
    /// held; this is for players who want to read text all the time.
    pub prefer_text_view: bool,
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            preview_scale: 1.0,
            lang: "en".to_string(),
            prefer_text_view: false,
        }
    }
}

/// Where the client looks for the gateway.
///
/// The gateway is where the client signs in, plays, and fetches card text, so
/// the address has to be configurable without a rebuild. Natively:
/// `BAYLEE_GATEWAY` in the environment, then a `.env` file in the working
/// directory, then the development default.
///
/// In a browser there is neither, and the page's own origin is only the right
/// answer when the gateway is what served the page. A dev build from `trunk
/// serve` is not: it comes off `:8080` and the gateway is somewhere else. So a
/// `?gateway=…` on the page URL wins, and is **remembered** — a browser drops
/// the query string on the first internal navigation, and a client that
/// forgot where its table was would be worse than one that never knew.
#[must_use]
pub fn gateway_url() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(url) = std::env::var("BAYLEE_GATEWAY")
            && !url.is_empty()
        {
            return trim_url(&url);
        }
        if let Some(url) = dotenv_value("BAYLEE_GATEWAY") {
            return trim_url(&url);
        }
        "http://127.0.0.1:28766".to_string()
    }
    #[cfg(target_arch = "wasm32")]
    {
        /// `localStorage` key remembering the gateway a `?gateway=` named.
        const KEY: &str = "baylee:gateway";
        let query = web_sys::window()
            .and_then(|w| w.location().search().ok())
            .unwrap_or_default();
        if let Some(url) = crate::net::query_value(&query, "gateway").filter(|u| !u.is_empty()) {
            let url = trim_url(&url);
            store::write_key(KEY, &url);
            return url;
        }
        if let Some(url) = store::read_key(KEY).filter(|u| !u.is_empty()) {
            return trim_url(&url);
        }
        web_sys::window()
            .and_then(|w| w.location().origin().ok())
            .map(|origin| trim_url(&origin))
            .unwrap_or_else(|| "http://127.0.0.1:28766".to_string())
    }
}

/// A gateway base URL without its trailing slash — `{base}/decks`, never
/// `{base}//decks`, which is a 404 with no explanation.
fn trim_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

/// Forgets a remembered browser gateway, so the page origin decides again.
///
/// Only reachable from a browser: everywhere else the address comes from the
/// environment on every launch and there is nothing to forget.
#[cfg(target_arch = "wasm32")]
pub fn forget_gateway() {
    store::remove_key("baylee:gateway");
}

/// Reads one key out of a `.env` file in the working directory.
///
/// Deliberately tiny rather than a dependency: the file holds a handful of
/// deployment knobs, and the format that matters is `KEY=value` with `#`
/// comments. Quotes are stripped because writing `KEY="value"` is the first
/// thing everyone tries.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn dotenv_value(key: &str) -> Option<String> {
    let text = std::fs::read_to_string(".env").ok()?;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if name.trim() != key {
            continue;
        }
        let value = value.trim().trim_matches(['"', '\'']).to_string();
        if !value.is_empty() {
            return Some(value);
        }
    }
    None
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

/// The native back end: JSON files in the platform config dir.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod store {
    /// The file the client's own settings live in.
    const SETTINGS: &str = "client-settings.json";

    /// Reads the settings JSON, or `None` if there is nothing to read.
    pub fn read() -> Option<String> {
        read_named(SETTINGS)
    }

    /// Writes the settings JSON, creating the config dir if needed.
    pub fn write(text: &str) {
        write_named(SETTINGS, text);
    }

    /// Reads one named document out of the config dir.
    pub fn read_named(name: &str) -> Option<String> {
        std::fs::read_to_string(path(name)?).ok()
    }

    /// Writes one named document, creating the config dir if needed.
    pub fn write_named(name: &str, text: &str) {
        let Some(path) = path(name) else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, text);
    }

    /// A config-dir file location.
    fn path(name: &str) -> Option<std::path::PathBuf> {
        let base = std::env::var("XDG_CONFIG_HOME")
            .ok()
            .filter(|v| !v.is_empty())
            .map_or_else(
                || std::env::var("HOME").ok().map(|h| format!("{h}/.config")),
                Some,
            )?;
        Some(std::path::PathBuf::from(base).join("baylee").join(name))
    }
}

/// The browser back end: `localStorage`, which survives a reload and is scoped
/// to the origin the client is served from.
#[cfg(target_arch = "wasm32")]
pub(crate) mod store {
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

    /// Reads one named document, under the same namespace.
    pub fn read_named(name: &str) -> Option<String> {
        read_key(&format!("baylee:{name}"))
    }

    /// Writes one named document, under the same namespace.
    pub fn write_named(name: &str, text: &str) {
        write_key(&format!("baylee:{name}"), text);
    }

    /// Reads one namespaced key.
    pub fn read_key(key: &str) -> Option<String> {
        storage()?.get_item(key).ok().flatten()
    }

    /// Writes one namespaced key; a full or disabled store is ignored.
    pub fn write_key(key: &str, value: &str) {
        if let Some(storage) = storage() {
            let _ = storage.set_item(key, value);
        }
    }

    /// Forgets one namespaced key.
    pub fn remove_key(key: &str) {
        if let Some(storage) = storage() {
            let _ = storage.remove_item(key);
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
            lang: "de".to_string(),
            prefer_text_view: true,
        };
        let text = serde_json::to_string_pretty(&written).expect("serializes");
        let read: ClientSettings = serde_json::from_str(&text).expect("decodes");
        assert!((read.preview_scale - 1.75).abs() < f32::EPSILON);
        assert_eq!(read.lang, "de");
        assert!(read.prefer_text_view);
    }

    /// A store written before the language field existed must still load, and
    /// must land on English rather than on an empty language code that would
    /// make every catalog request fail.
    #[test]
    fn an_older_store_defaults_to_english() {
        let settings: ClientSettings =
            serde_json::from_str(r#"{"preview_scale":1.25}"#).expect("decodes");
        assert_eq!(settings.lang, "en");
        assert!(!settings.prefer_text_view);
    }

    /// The gateway address decides where card text comes from, so a trailing
    /// slash must not turn into a double slash in every request path.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn the_default_gateway_url_has_no_trailing_slash() {
        let url = super::gateway_url();
        assert!(!url.ends_with('/'), "{url}");
        assert!(url.starts_with("http"), "{url}");
    }
}
