//! The OS credential store, where a player may keep a language model's key
//! instead of the environment (`docs/llm-seat.md` §"Where a key is kept").
//!
//! This program is the only one that opens it: the bridge reads a key from
//! it where the profile's variable is unset ([`stored_key`]), and the
//! client keeps, replaces and forgets keys through `baylee-seat key`, which
//! takes the key on stdin and answers only whether one is kept. A key never
//! lands in an environment, a command line, a file or a log on the way.
//!
//! Why the bridge reads the store itself rather than the client passing
//! the key in the bridge's environment: an environment is readable by every
//! process of the same user for the bridge's whole life (`/proc/<pid>/environ`)
//! and is inherited by whatever the bridge starts, while the store answers
//! only when asked, keeps its own access rules, and a key read from it sits
//! in the one process that sends it. It also lets a debug bridge that
//! changes its mind mid-game find the new profile's key, and a bridge
//! started from a terminal use the key the client kept.

use baylee_client_core::llmseat::keys::{KeyEntry, KeyStore, MemoryKeys, SERVICE, STORE_ENV};
use std::sync::Arc;

/// The store this machine has, unless `env` turns it off ([`STORE_ENV`]
/// `off`): the one the bridge and `baylee-seat key` open.
#[must_use]
pub fn store(env: &dyn Fn(&str) -> Option<String>) -> Arc<dyn KeyStore> {
    if env(STORE_ENV).is_some_and(|v| v.trim() == "off") {
        return Arc::new(MemoryKeys::unavailable(&format!(
            "{STORE_ENV}=off: keys come from the environment only"
        )));
    }
    os()
}

#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
fn os() -> Arc<dyn KeyStore> {
    Arc::new(OsKeys)
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn os() -> Arc<dyn KeyStore> {
    Arc::new(MemoryKeys::unavailable(
        "this platform has no credential store the bridge can open",
    ))
}

/// The platform's store: the Keychain, the Credential Manager, the Secret
/// Service. Never opened by a test.
#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
#[derive(Debug, Clone, Copy)]
pub struct OsKeys;

#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
impl OsKeys {
    fn entry(entry: &KeyEntry) -> Result<keyring::Entry, String> {
        keyring::Entry::new(SERVICE, &entry.account()).map_err(|e| said(&e))
    }
}

/// A store's error as a sentence. Its messages name the entry at most,
/// never a secret; a key-shaped run is blanked all the same.
#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
fn said(error: &keyring::Error) -> String {
    let text = match error {
        keyring::Error::NoDefaultStore => {
            "this machine's credential store could not be opened".to_string()
        }
        other => format!("the credential store refused: {other}"),
    };
    baylee_client_core::llmseat::blank_key_shapes(&text)
        .chars()
        .take(200)
        .collect()
}

#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
impl KeyStore for OsKeys {
    fn available(&self) -> Result<(), String> {
        match keyring::Entry::store_status() {
            Ok(()) => Ok(()),
            Err(e) => Err(said(e)),
        }
    }

    fn has(&self, entry: &KeyEntry) -> Result<bool, String> {
        // The credential itself, not its secret: the store need not hand
        // the key over to say one is kept.
        match Self::entry(entry)?.inner.get_credential() {
            Ok(_) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(said(&e)),
        }
    }

    fn get(&self, entry: &KeyEntry) -> Result<Option<String>, String> {
        match Self::entry(entry)?.get_password() {
            Ok(key) => Ok(Some(key)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(said(&e)),
        }
    }

    fn set(&self, entry: &KeyEntry, key: &str) -> Result<(), String> {
        if let Some(why) = baylee_client_core::llmseat::keys::key_fault(key) {
            return Err(why.into());
        }
        Self::entry(entry)?.set_password(key).map_err(|e| said(&e))
    }

    fn delete(&self, entry: &KeyEntry) -> Result<(), String> {
        match Self::entry(entry)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(said(&e)),
        }
    }
}

/// The key kept in `store` for the key in `key_env` sent to `base`, where
/// `env` has none in that variable and the store can be opened: `None`
/// otherwise. A store that cannot be opened is no error here (the missing
/// key is, later, in the sentence the player already knows).
#[must_use]
pub fn stored_key(
    store: &dyn KeyStore,
    key_env: &str,
    base: &str,
    env: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    if env(key_env).is_some_and(|k| !k.trim().is_empty()) {
        return None;
    }
    let entry = KeyEntry::new(key_env, base)?;
    store.available().ok()?;
    store.get(&entry).ok().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::llmseat::keys::KeyState;

    #[test]
    fn a_kept_key_is_read_only_where_the_environment_has_none_and_only_for_its_host() {
        let store = MemoryKeys::default();
        let deepseek = KeyEntry::new("DEEPSEEK_API_KEY", "https://api.deepseek.com/v1").unwrap();
        store.set(&deepseek, "TEST-kept-0123456789").unwrap();
        let empty = |_: &str| None;
        assert_eq!(
            stored_key(
                &store,
                "DEEPSEEK_API_KEY",
                "https://api.deepseek.com/anthropic",
                &empty
            )
            .as_deref(),
            Some("TEST-kept-0123456789"),
            "one host, either protocol"
        );
        assert_eq!(
            stored_key(
                &store,
                "DEEPSEEK_API_KEY",
                "https://llm.example.org/v1",
                &empty
            ),
            None,
            "never to another host"
        );
        let set = |name: &str| (name == "DEEPSEEK_API_KEY").then(|| "TEST-env-key".to_string());
        assert_eq!(
            stored_key(
                &store,
                "DEEPSEEK_API_KEY",
                "https://api.deepseek.com/v1",
                &set
            ),
            None,
            "the environment wins"
        );
    }

    #[test]
    fn the_store_is_off_when_the_environment_says_so() {
        let off = store(&|name| (name == STORE_ENV).then(|| "off".to_string()));
        let at = KeyEntry::new("ANTHROPIC_API_KEY", "https://api.anthropic.com").unwrap();
        assert!(matches!(
            baylee_client_core::llmseat::keys::state(&*off, &at),
            KeyState::Unavailable(why) if why.contains("off")
        ));
        assert_eq!(
            stored_key(
                &*off,
                "ANTHROPIC_API_KEY",
                "https://api.anthropic.com",
                &|_| None
            ),
            None
        );
    }
}
