//! JSON-file persistence for accounts and decks. Atomic writes
//! (write-temp-then-rename) so a crash mid-write can't corrupt the store.
//! Secrets at rest: Argon2id password hashes, SHA-256 token hashes —
//! never plaintext credentials.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A registered account. The username is the e-mail address; the
/// display name is shown to other players.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    /// Account id (`UUIDv7`).
    pub id: String,
    /// Login e-mail (lowercased, unique).
    pub email: String,
    /// Display name shown in the lobby (unique, case-insensitively).
    pub display_name: String,
    /// Argon2id PHC password hash.
    pub password_hash: String,
    /// Created at (unix seconds).
    pub created_at: u64,
}

/// A stored session token (only the SHA-256 hash is kept).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredToken {
    /// SHA-256 of the bearer token.
    pub token_hash: String,
    /// Owning account id.
    pub account_id: String,
    /// Expiry (unix seconds, sliding).
    pub expires_at: u64,
}

/// A player's deck (card names; resolved against the registry at use).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Deck {
    /// Deck id (`UUIDv7`).
    pub id: String,
    /// Owning account id.
    pub account_id: String,
    /// Deck name.
    pub name: String,
    /// Card lines (`"N Card Name"`).
    pub cards: Vec<String>,
    /// Commander card name, if any.
    pub commander: Option<String>,
    /// Last update (unix seconds).
    pub updated_at: u64,
}

/// The whole store.
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Store {
    /// Accounts by id.
    pub accounts: HashMap<String, Account>,
    /// Tokens by hash.
    pub tokens: HashMap<String, StoredToken>,
    /// Decks by id.
    pub decks: HashMap<String, Deck>,
}

impl Store {
    /// Load from disk (missing file = empty store).
    #[must_use]
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Persist atomically.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let tmp = PathBuf::from(format!("{}.tmp", path.display()));
        std::fs::write(
            &tmp,
            serde_json::to_string_pretty(self).expect("store serializes"),
        )?;
        std::fs::rename(&tmp, path)
    }

    /// Find an account by login e-mail (case-insensitive).
    #[must_use]
    pub fn account_by_email(&self, email: &str) -> Option<&Account> {
        self.accounts
            .values()
            .find(|a| a.email.eq_ignore_ascii_case(email))
    }

    /// Find an account by display name (case-insensitive).
    #[must_use]
    pub fn account_by_display_name(&self, display_name: &str) -> Option<&Account> {
        self.accounts
            .values()
            .find(|a| a.display_name.eq_ignore_ascii_case(display_name))
    }

    /// Resolve a bearer token to its account id when valid (sliding
    /// renewal bumps the expiry on use).
    pub fn resolve_token(&mut self, token: &str, now: u64) -> Option<String> {
        let hash = crate::auth::token_hash(token);
        let entry = self.tokens.get_mut(&hash)?;
        if entry.expires_at < now {
            self.tokens.remove(&hash);
            return None;
        }
        entry.expires_at = now + crate::auth::TOKEN_TTL.as_secs();
        Some(entry.account_id.clone())
    }
}
