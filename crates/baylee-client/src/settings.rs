//! Client-side settings, remembered across launches.
//!
//! Natively they persist as a small JSON file in the platform config dir
//! (`~/.config/baylee/`); in a browser the same JSON lives in `localStorage`,
//! scoped to the origin the client is served from. Both back ends are
//! best-effort by design: a corrupt, missing or unreadable store must never
//! stop the game from starting.

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

/// Whether this process may read and write the player's own files.
///
/// **Shut until [`open_store`] opens it, and only [`crate::standalone::run`]
/// does.** A unit test, an integration test under `tests/` and a bench never
/// go through `run`, so every one of them reads nothing and writes nothing,
/// whoever's machine it runs on. Both back ends ask here, below every caller.
///
/// It was `cfg!(test)` at two of the five callers before, and that missed
/// in both directions. `cfg(test)` is set only while this library is
/// compiled for its *own* unit tests, not as the dependency of one in
/// `tests/`. And `Prefs::local` had no guard at all: a settings test passed
/// on the owner's machine, whose `preferences.json` said
/// `skip_empty_blocks: false`, and failed on CI, where no file meant the
/// default. The same unguarded function also *saved*, so a test could have
/// overwritten the owner's preferences, and `offline-decks.json` holds decks
/// built by hand. One door at the store rather than a guard per caller,
/// because the next caller would not have brought one.
static OPEN: AtomicBool = AtomicBool::new(false);

/// Lets this process read and write the player's settings, decks and card
/// text cache. Called once, first thing, by [`crate::standalone::run`].
pub fn open_store() {
    OPEN.store(true, Ordering::Release);
}

/// Whether [`open_store`] has been called in this process.
#[must_use]
pub fn store_is_open() -> bool {
    OPEN.load(Ordering::Acquire)
}

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
    /// Where the zone browser was left standing, and how big.
    ///
    /// Here and not in `Preferences`, which travels with the account over
    /// `/settings`: a sheet parked clear of the mat on a 1728-wide screen is
    /// a fact about *this screen*, and following the player onto a phone
    /// would be following them with the wrong answer. `None` means it has
    /// never been moved, which is not the same as the default rectangle —
    /// the default depends on the window, and this store does not know one.
    #[serde(default)]
    pub zone_browser: Option<baylee_client_core::browser::Placement>,
    /// Which shape the zone browser draws its list in.
    ///
    /// Beside the rectangle above rather than in `Preferences`, but for the
    /// opposite reason: the rectangle is a fact about *this screen*, and this
    /// is taste that simply has nowhere better to live — a view mode is not
    /// worth a round trip to the gateway and is wanted by a player who has
    /// never signed in. Its own `#[serde(default)]` is the one that matters if
    /// this enum ever grows or loses a variant: an unknown name would
    /// otherwise refuse the whole file and take the player's sheet, their
    /// language and their address with it.
    #[serde(default)]
    pub zone_view: baylee_client_core::browser::ViewMode,
    /// The address that last signed in here, to fill the sign-in box with.
    ///
    /// Here for the reason the zone browser above is, said from the other
    /// side: it has to be readable *before* anybody has signed in, so it
    /// cannot travel with the account over `/settings`.
    ///
    /// Written on a sign-in that worked and never on one that was refused —
    /// that is the one moment the client knows the address is a real one, and
    /// a remembered typo would be handed back on every launch. The password
    /// is not kept beside it and will not be: that belongs in the platform's
    /// keychain, not in a JSON file in a config directory.
    #[serde(default)]
    pub last_email: String,
    /// Locally saved gateway addresses; selection is explicit on every launch.
    pub gateways: Vec<String>,
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            preview_scale: 1.0,
            lang: "en".to_string(),
            prefer_text_view: false,
            zone_browser: None,
            zone_view: baylee_client_core::browser::ViewMode::default(),
            last_email: String::new(),
            gateways: Vec::new(),
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
        // A phone has neither. An app started by a tap on an icon inherits no
        // environment, and there is no working directory to put a `.env` in,
        // so on those two targets the address is baked in when the client is
        // compiled — `BAYLEE_GATEWAY=http://192.168.…:28766 cargo apk build`.
        // It sits *after* both runtime lookups on purpose: the iOS simulator
        // does pass an environment through (`SIMCTL_CHILD_BAYLEE_GATEWAY`),
        // and what was passed today must beat what was compiled last week.
        // `build.rs` is the other half — without it cargo would not rebuild
        // when the address changes, and the phone would keep dialling the old
        // one with nothing anywhere saying so.
        #[cfg(any(target_os = "android", target_os = "ios"))]
        if let Some(url) = option_env!("BAYLEE_GATEWAY").filter(|url| !url.is_empty()) {
            return trim_url(url);
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
    ///
    /// Defaults in a test, because a test never opens the store
    /// ([`store_is_open`]). That mattered here first: the developer set their
    /// own client to German to photograph it, and three lobby tests failed on
    /// the spot, because `LobbyState::new` reads this and they assert on the
    /// words on the screen. A test that passes or fails on whose machine it
    /// runs is worse than no test.
    #[must_use]
    pub fn load() -> Self {
        store::read()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Persists the settings (best-effort; neither a read-only home dir nor a
    /// browser with site data blocked is worth a crash).
    ///
    /// Nothing in a test, because a test never opens the store
    /// ([`store_is_open`]). This writes to the player's real config
    /// directory, and the zone browser's drag test, the first time it
    /// released the pointer, moved the sheet in the developer's own client by
    /// the delta the test had invented. The in-memory half is what a test has
    /// business asserting; `settings_round_trip_through_json` proves the
    /// encoding without a file.
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
    ///
    /// Into a temporary beside it and then renamed, which is what the
    /// gateway's own store already does and for the same reason:
    /// `fs::write` truncates first, so a crash between the truncate and the
    /// last byte leaves a file that parses as nothing. For the settings that
    /// costs a player their window placement; for `offline-decks.json` it
    /// costs them decks they built by hand and cannot regenerate, and that
    /// document goes through this same function.
    pub fn write_named(name: &str, text: &str) {
        let Some(path) = path(name) else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let temporary = path.with_extension("tmp");
        if std::fs::write(&temporary, text).is_ok() {
            let _ = std::fs::rename(&temporary, &path);
        }
    }

    /// Puts a document that could not be read out of the way, once.
    ///
    /// The moment a player's work is actually lost is not the crash — it is
    /// the *next save*, which overwrites a file nobody could parse with a
    /// fresh default. Every reader here answers a parse failure with
    /// defaults, so without this the unreadable bytes are gone within
    /// seconds, and with them any chance of getting the decks back by hand.
    ///
    /// Once, because a second failure would otherwise overwrite the first
    /// rescue with the very defaults that replaced it. The copy already set
    /// aside is the one worth keeping.
    pub fn set_aside(name: &str) {
        let Some(path) = path(name) else {
            return;
        };
        let broken = path.with_extension("broken");
        if broken.exists() {
            return;
        }
        let _ = std::fs::rename(&path, &broken);
    }

    /// A config-dir file location, and `None` in a process that has not
    /// opened the store ([`super::store_is_open`]): every function here goes
    /// through this one.
    fn path(name: &str) -> Option<std::path::PathBuf> {
        if !super::store_is_open() {
            return None;
        }
        resolve(name)
    }

    /// Where `name` lives in the platform config dir, door or no door.
    fn resolve(name: &str) -> Option<std::path::PathBuf> {
        let base = std::env::var("XDG_CONFIG_HOME")
            .ok()
            .filter(|v| !v.is_empty())
            .map_or_else(
                || std::env::var("HOME").ok().map(|h| format!("{h}/.config")),
                Some,
            )?;
        Some(std::path::PathBuf::from(base).join("baylee").join(name))
    }

    #[cfg(test)]
    mod tests {
        /// A test process reads and writes none of the player's files.
        ///
        /// `resolve` answers first, so the `None`s below are the door's and
        /// not a missing `HOME`. And the probe is written *before* it is read:
        /// with the door broken, the read would find the write.
        #[test]
        fn a_process_that_never_opened_the_store_reads_and_writes_nothing() {
            const PROBE: &str = "hermetic-probe.json";
            assert!(!super::super::store_is_open(), "nothing in a test opens it");
            let real = super::resolve(PROBE).expect("this machine has a config dir");

            super::write_named(PROBE, "{}");
            assert_eq!(super::read_named(PROBE), None);
            assert!(!real.exists(), "{} was written", real.display());
            assert_eq!(super::path(PROBE), None);
        }
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

    /// Puts a document that could not be read out of the way, once.
    ///
    /// The native back end's note explains why this exists. There is no
    /// half-written value to rescue here — `set_item` is atomic — but the
    /// *other* half of the hazard is the same in a browser as on a disk: a
    /// reader answers a parse failure with defaults and the next save
    /// overwrites the bytes nobody could read. So the key is moved rather
    /// than left to be replaced.
    pub fn set_aside(name: &str) {
        let key = format!("baylee:{name}");
        let broken = format!("{key}.broken");
        if read_key(&broken).is_some() {
            return;
        }
        if let Some(text) = read_key(&key) {
            write_key(&broken, &text);
            remove_key(&key);
        }
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
    ///
    /// And `None` in a process that has not opened the store
    /// ([`super::store_is_open`]): every function here goes through this one.
    fn storage() -> Option<web_sys::Storage> {
        if !super::store_is_open() {
            return None;
        }
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
            gateways: vec!["https://example.test".into()],
            preview_scale: 1.75,
            lang: "de".to_string(),
            prefer_text_view: true,
            zone_browser: Some(baylee_client_core::browser::Placement {
                left: 40.0,
                top: 24.0,
                width: 520.0,
                height: 380.0,
            }),
            zone_view: baylee_client_core::browser::ViewMode::Grid,
            last_email: "mail@acevik.de".to_string(),
        };
        let text = serde_json::to_string_pretty(&written).expect("serializes");
        let read: ClientSettings = serde_json::from_str(&text).expect("decodes");
        assert_eq!(read.gateways, written.gateways);
        assert!((read.preview_scale - 1.75).abs() < f32::EPSILON);
        assert_eq!(read.lang, "de");
        assert!(read.prefer_text_view);
        let place = read.zone_browser.expect("the sheet's place survived");
        assert!((place.left - 40.0).abs() < f32::EPSILON);
        assert!((place.width - 520.0).abs() < f32::EPSILON);
        assert_eq!(
            read.zone_view,
            baylee_client_core::browser::ViewMode::Grid,
            "the view the player chose has to come back, or the buttons are a setting that \
             resets every launch"
        );
        assert_eq!(read.last_email, "mail@acevik.de");
    }

    /// A settings file naming a view mode this build has never heard of loads
    /// as the rest of itself, with that one field defaulted.
    ///
    /// `load` is `from_str(…).ok().unwrap_or_default()`, so a field that
    /// refused would take the sheet's rectangle, the language and the
    /// remembered address down with it — which is exactly what one retired
    /// `Action` once did to every key a player had ever bound.
    #[test]
    fn a_retired_view_mode_does_not_take_the_whole_store_with_it() {
        let settings: ClientSettings =
            serde_json::from_str(r#"{"lang":"de","zone_view":"folders"}"#).expect("still decodes");
        assert_eq!(settings.lang, "de", "the rest of the file survived");
        assert_eq!(
            settings.zone_view,
            baylee_client_core::browser::ViewMode::default()
        );
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
