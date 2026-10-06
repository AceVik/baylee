//! A language model's key in the OS credential store (macOS Keychain,
//! Windows Credential Manager, the Secret Service on Linux), for a player
//! who would rather not keep it in the environment
//! (`docs/llm-seat.md` §"Where a key is kept").
//!
//! A key is read from the environment variable a profile's `key_env` names
//! first, and only where that variable is unset from the store; it never
//! goes in the settings file or any other plain file, on a command line,
//! in a log, or to the gateway. Only the bridge (`baylee-seat`) opens the
//! store: the client hands it a key the player typed over a pipe
//! (`baylee-seat key set`) and asks it whether one is kept, never what.
//!
//! An entry is named by the key's variable *and* the host the key goes to
//! ([`KeyEntry`]), so a profile pointed at another address finds no key
//! there: a key the player gave for `api.deepseek.com` is never sent
//! anywhere else, whatever the file says later.
//!
//! This module is the store's shape and its stand-in ([`MemoryKeys`]); the
//! store itself is in `baylee-seat` (`keys::OsKeys`), the only program that
//! links it.

use super::{Profile, Provider, is_loopback};
use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

/// The service every entry is kept under.
pub const SERVICE: &str = "baylee-seat";

/// The environment variable that turns the store off for a bridge:
/// `off` reads keys from the environment only (and every test that starts
/// the real bridge sets it, so no test reaches the player's store).
pub const STORE_ENV: &str = "BAYLEE_KEY_STORE";

/// The longest key the store takes, in bytes; no provider's is near it.
pub const KEY_BYTES: usize = 512;

/// Where a key is kept: the variable it would otherwise be read from, and
/// the host (with its port) it is sent to.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct KeyEntry {
    key_env: String,
    host: String,
}

impl KeyEntry {
    /// The entry for the key in `key_env` that goes to `base`; `None` for
    /// an address with no host.
    #[must_use]
    pub fn new(key_env: &str, base: &str) -> Option<Self> {
        let host = authority(base)?;
        Some(Self {
            key_env: key_env.to_string(),
            host,
        })
    }

    /// The variable the key would otherwise be read from.
    #[must_use]
    pub fn key_env(&self) -> &str {
        &self.key_env
    }

    /// The host (and port) the key goes to.
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// The account the store keeps it under: `DEEPSEEK_API_KEY@api.deepseek.com`.
    #[must_use]
    pub fn account(&self) -> String {
        format!("{}@{}", self.key_env, self.host)
    }
}

/// The host and port of `base`, lower-cased: `https://API.deepseek.com/v1`
/// → `api.deepseek.com`, `http://127.0.0.1:1234/v1` → `127.0.0.1:1234`.
fn authority(base: &str) -> Option<String> {
    let rest = base.trim().split_once("://").map(|(_, rest)| rest)?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = host.rsplit_once('@').map_or(host, |(_, h)| h);
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

/// The address a key for `provider` goes to: the profile's `base_url`,
/// else the provider's variable (`env_base`), else the provider's own, as
/// the bridge resolves it. `None` for a CLI.
#[must_use]
pub fn address(
    provider: Provider,
    base_url: Option<&str>,
    env_base: Option<&str>,
) -> Option<String> {
    let tidy = |b: &str| b.trim().trim_end_matches('/').to_string();
    base_url
        .map(tidy)
        .or_else(|| env_base.map(tidy).filter(|b| !b.is_empty()))
        .or_else(|| provider.default_base().map(str::to_string))
}

/// The entry `profile`'s key is kept under, with the provider's address
/// variable read as `env_base`; `None` for a CLI, which reads no key.
#[must_use]
pub fn entry(profile: &Profile, env_base: Option<&str>) -> Option<KeyEntry> {
    let key_env = profile.key_env()?;
    let base = address(profile.provider, profile.base_url.as_deref(), env_base)?;
    KeyEntry::new(key_env, &base)
}

/// Whether a profile at `base` needs a key at all: a server on this machine
/// may go without (the bridge sends one if it finds one).
#[must_use]
pub fn needs_a_key(provider: Provider, base: &str) -> bool {
    !(provider == Provider::OpenAi && is_loopback(base))
}

/// What is wrong with `key` as typed, or `None`: one line of printable
/// characters with no space, at most [`KEY_BYTES`]. The sentence never
/// quotes it.
#[must_use]
pub fn key_fault(key: &str) -> Option<&'static str> {
    if key.is_empty() {
        Some("the key is empty")
    } else if key.len() > KEY_BYTES {
        Some("that is longer than any key")
    } else if key.chars().any(|c| c.is_whitespace() || c.is_control()) {
        Some("a key has no spaces and is one line")
    } else {
        None
    }
}

/// Whether a key is kept for an entry, as the client shows it: never the
/// key itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyState {
    /// This machine has no store the bridge can open, and why: keys come
    /// from the environment only.
    Unavailable(String),
    /// None is kept.
    Absent,
    /// One is kept.
    Set,
}

impl KeyState {
    /// The one word `baylee-seat key status` prints for it, then the reason
    /// for [`KeyState::Unavailable`].
    #[must_use]
    pub fn line(&self) -> String {
        match self {
            Self::Unavailable(why) => format!("unavailable: {why}"),
            Self::Absent => "absent".into(),
            Self::Set => "set".into(),
        }
    }

    /// Reads [`KeyState::line`] back; anything else is unavailable.
    #[must_use]
    pub fn parse(line: &str) -> Self {
        match line.trim() {
            "set" => Self::Set,
            "absent" => Self::Absent,
            other => Self::Unavailable(
                other
                    .strip_prefix("unavailable: ")
                    .unwrap_or("the bridge did not say")
                    .chars()
                    .take(200)
                    .collect(),
            ),
        }
    }
}

/// A credential store, by entry. Every error is a sentence that names no
/// key.
pub trait KeyStore: Send + Sync {
    /// Whether the store can be opened here, and why not.
    ///
    /// # Errors
    /// A sentence: no store on this platform, or it is turned off.
    fn available(&self) -> Result<(), String>;

    /// Whether a key is kept for `entry`, read without the key itself
    /// where the store allows it.
    ///
    /// # Errors
    /// As [`KeyStore::available`], or the store failed.
    fn has(&self, entry: &KeyEntry) -> Result<bool, String>;

    /// The key kept for `entry`, `None` where there is none.
    ///
    /// # Errors
    /// As [`KeyStore::has`].
    fn get(&self, entry: &KeyEntry) -> Result<Option<String>, String>;

    /// Keeps `key` for `entry`, over any kept before.
    ///
    /// # Errors
    /// As [`KeyStore::has`].
    fn set(&self, entry: &KeyEntry, key: &str) -> Result<(), String>;

    /// Forgets the key kept for `entry`; none kept is no error.
    ///
    /// # Errors
    /// As [`KeyStore::has`].
    fn delete(&self, entry: &KeyEntry) -> Result<(), String>;
}

/// What the store says of `entry`.
#[must_use]
pub fn state(store: &dyn KeyStore, entry: &KeyEntry) -> KeyState {
    match store.available().and_then(|()| store.has(entry)) {
        Ok(true) => KeyState::Set,
        Ok(false) => KeyState::Absent,
        Err(why) => KeyState::Unavailable(why),
    }
}

/// A store in memory, for tests and for a platform with none
/// ([`MemoryKeys::unavailable`]): never the player's.
#[derive(Debug, Default)]
pub struct MemoryKeys {
    keys: Mutex<BTreeMap<String, String>>,
    off: Option<String>,
}

impl MemoryKeys {
    /// A store that cannot be opened, for `why`.
    #[must_use]
    pub fn unavailable(why: &str) -> Self {
        Self {
            keys: Mutex::default(),
            off: Some(why.to_string()),
        }
    }

    fn open(&self) -> Result<std::sync::MutexGuard<'_, BTreeMap<String, String>>, String> {
        match &self.off {
            Some(why) => Err(why.clone()),
            None => Ok(self.keys.lock().unwrap_or_else(PoisonError::into_inner)),
        }
    }
}

impl KeyStore for MemoryKeys {
    fn available(&self) -> Result<(), String> {
        self.open().map(drop)
    }

    fn has(&self, entry: &KeyEntry) -> Result<bool, String> {
        Ok(self.open()?.contains_key(&entry.account()))
    }

    fn get(&self, entry: &KeyEntry) -> Result<Option<String>, String> {
        Ok(self.open()?.get(&entry.account()).cloned())
    }

    fn set(&self, entry: &KeyEntry, key: &str) -> Result<(), String> {
        if let Some(why) = key_fault(key) {
            return Err(why.into());
        }
        self.open()?.insert(entry.account(), key.to_string());
        Ok(())
    }

    fn delete(&self, entry: &KeyEntry) -> Result<(), String> {
        self.open()?.remove(&entry.account());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::seating::Preset;
    use super::*;

    #[test]
    fn a_key_is_kept_under_its_variable_and_the_host_it_goes_to() {
        let deepseek = Preset::DeepSeek.profile();
        let over_anthropic = Preset::DeepSeekAnthropic.profile();
        let at = entry(&deepseek, None).unwrap();
        assert_eq!(at.account(), "DEEPSEEK_API_KEY@api.deepseek.com");
        // One host, one key, whichever protocol reaches it.
        assert_eq!(entry(&over_anthropic, None), Some(at.clone()));
        // A profile pointed elsewhere finds none of it.
        let moved = Profile {
            base_url: Some("https://llm.example.org/v1".into()),
            ..deepseek.clone()
        };
        assert_eq!(
            entry(&moved, None).unwrap().account(),
            "DEEPSEEK_API_KEY@llm.example.org"
        );
        // The provider's own address, or the variable's, where the profile
        // names none; a port is part of the host.
        let bare = Profile::new(Provider::Anthropic, "claude-opus-5-5");
        assert_eq!(
            entry(&bare, None).unwrap().account(),
            "ANTHROPIC_API_KEY@api.anthropic.com"
        );
        assert_eq!(
            entry(&bare, Some("http://127.0.0.1:4000/")).unwrap().host(),
            "127.0.0.1:4000"
        );
        assert_eq!(entry(&Preset::ClaudeCode.profile(), None), None, "a CLI");
        assert!(!needs_a_key(Provider::OpenAi, "http://localhost:1234/v1"));
        assert!(needs_a_key(Provider::Anthropic, "http://localhost:1234"));
    }

    #[test]
    fn the_store_says_whether_a_key_is_kept_and_never_what() {
        let store = MemoryKeys::default();
        let at = entry(&Preset::DeepSeek.profile(), None).unwrap();
        assert_eq!(state(&store, &at), KeyState::Absent);
        assert!(store.set(&at, "two words").is_err());
        assert!(store.set(&at, "").is_err());
        assert!(store.set(&at, &"k".repeat(KEY_BYTES + 1)).is_err());
        assert_eq!(state(&store, &at), KeyState::Absent, "refused, not kept");
        store.set(&at, "TEST-key-0123456789abcdef").unwrap();
        assert_eq!(state(&store, &at), KeyState::Set);
        assert_eq!(
            store.get(&at).unwrap().as_deref(),
            Some("TEST-key-0123456789abcdef")
        );
        store.delete(&at).unwrap();
        store.delete(&at).unwrap();
        assert_eq!(state(&store, &at), KeyState::Absent);
        let none = MemoryKeys::unavailable("this platform has no credential store");
        let said = state(&none, &at);
        assert_eq!(
            said,
            KeyState::Unavailable("this platform has no credential store".into())
        );
        // What `baylee-seat key status` prints reads back.
        for line in [KeyState::Set, KeyState::Absent, said] {
            assert_eq!(KeyState::parse(&line.line()), line);
        }
        assert!(matches!(
            KeyState::parse("garbled"),
            KeyState::Unavailable(_)
        ));
    }
}
