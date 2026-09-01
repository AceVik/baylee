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
    /// Sideboard lines, in the same form. Cards outside the game a seat may
    /// reach; never shuffled into the library.
    ///
    /// Defaulted rather than required, so a deck saved before there was a
    /// sideboard still loads as one without a sideboard.
    #[serde(default)]
    pub sideboard: Vec<String>,
    /// Commander card name, if any.
    pub commander: Option<String>,
    /// Last update (unix seconds).
    pub updated_at: u64,
}

/// A remembered answer to one optional ability, replayed into every game
/// the account sits down to.
///
/// The engine addresses standing answers by `AbilityRef { card, index }`,
/// a handle that says nothing about a particular game — which is exactly
/// what makes it storable here. "Always gain the life from Ondu Cleric's
/// rally trigger" is a preference about a *card*, so it belongs to the
/// account and not to the table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandingAnswer {
    /// Registry index of the card the ability is printed on.
    pub card: u32,
    /// Index into that card's ability list (`AbilityRef::index`); the
    /// reserved values above `AbilityRef::FIRST_RESERVED` name the
    /// abilities that are not listed on the card.
    pub ability: u32,
    /// What to answer without asking.
    pub yes: bool,
}

/// The whole store.
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Store {
    /// Accounts by id.
    pub accounts: HashMap<String, Account>,
    /// Tokens by hash.
    pub tokens: HashMap<String, StoredToken>,
    /// Decks by id.
    pub decks: HashMap<String, Deck>,
    /// Standing answers by account id. Defaulted so a store written
    /// before this existed still loads.
    #[serde(default)]
    pub automation: HashMap<String, Vec<StandingAnswer>>,
    /// Client preferences by account id — keymap, phase rail, and what the
    /// client may answer without asking.
    ///
    /// Deliberately an opaque JSON object rather than a typed struct. The
    /// gateway would have to link `baylee-client-core` to know what is in
    /// here, and it links neither the client's brain nor the engine on
    /// purpose; and a client that adds a preference should not need a
    /// gateway deploy before it can store it. What the gateway does enforce
    /// is that the value is an object and that it is small — see
    /// `MAX_SETTINGS_BYTES`.
    #[serde(default)]
    pub settings: HashMap<String, serde_json::Value>,
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
