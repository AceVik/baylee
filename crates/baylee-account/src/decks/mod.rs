//! Private, account-encrypted deck library. Card rows retain their original
//! spelling, printing choices and notes across edits, imports and devices.
use crate::Error;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod gateway_export;
mod manifest;
pub use manifest::LibraryRef;
#[cfg(feature = "client")]
pub(crate) use manifest::{Blob, Chunk, Head, Log, Revision, Root};

/// Identity of a source deck, scoped to its originating gateway or Central account.
/// @openapi
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeckSource {
    /// Gateway origin for a gateway source; absent for a Central private copy.
    pub gateway: Option<String>,
    /// Source account, when known.
    pub account: Option<String>,
    /// Original deck identifier at that origin.
    pub deck: String,
    /// Original logical save number.
    pub revision: u64,
}

/// Complete editable content of a private deck.
/// @openapi
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeckDraft {
    /// Display name.
    pub name: String,
    /// Format key.
    pub format: String,
    /// Main-deck rows, including group/printing/notes.
    pub cards: Vec<String>,
    /// Sideboard rows in the same representation.
    pub sideboard: Vec<String>,
    /// Commander names, in presentation order.
    pub commanders: Vec<String>,
    /// Description supplied by the owner.
    pub description: String,
    /// Optional message describing this edit.
    pub summary: Option<String>,
    /// Gateway-owned sleeve identifier.
    pub sleeve: Option<String>,
    /// Gateway-owned playmat identifier.
    pub playmat: Option<String>,
    /// Gateway origin that owns both cosmetic references.
    pub cosmetic_origin: Option<String>,
    /// Original private row on a migrated gateway.
    pub source: Option<DeckSource>,
    /// Source of an explicit deck copy.
    pub copied_from: Option<DeckSource>,
}

/// Plaintext snapshot returned only to the authenticated caller.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeckSnapshot {
    /// Central-issued deck identity.
    pub id: Uuid,
    /// Logical save number.
    pub revision: u64,
    /// Editable content at that exact revision.
    pub draft: DeckDraft,
}

/// Metadata for one current deck, without loading its card list.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeckSummary {
    /// Central-issued deck identity.
    pub id: Uuid,
    /// Current logical save number for conditional writes.
    pub revision: u64,
    /// Display name.
    pub name: String,
    /// Format key.
    pub format: String,
    /// Main-deck row count.
    pub cards: usize,
    /// Sideboard row count.
    pub sideboard: usize,
    /// Main-deck card copies.
    pub copies: u32,
    /// Sideboard card copies.
    pub side_copies: u32,
    /// Commander names in presentation order.
    pub commanders: Vec<String>,
    /// UTC Unix seconds at the most recent save.
    pub updated_at: i64,
}

/// Metadata for a superseded save, newest first in history reads.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeckSave {
    /// Logical revision of the superseded snapshot.
    pub revision: u64,
    /// Optional description of the change.
    pub summary: Option<String>,
    /// Number of main-deck rows.
    pub cards: usize,
    /// Number of sideboard rows.
    pub sideboard: usize,
    /// UTC Unix seconds at supersession.
    pub superseded_at: i64,
}

/// One source save submitted during an all-or-nothing migration.
pub struct ImportedVersion {
    /// Original positive version number.
    pub revision: u64,
    /// Content, including original owner/source identifiers.
    pub draft: DeckDraft,
    /// Source timestamp in UTC Unix seconds.
    pub saved_at: i64,
}

/// Full chronological history of one original gateway deck.
pub struct ImportedDeck {
    /// Strictly increasing source versions, latest last.
    pub versions: Vec<ImportedVersion>,
}

#[cfg(test)]
mod tests;

fn valid_origin(text: &str) -> bool {
    central_api::gateway::validate_origin(text, text.starts_with("http://")).is_ok()
}

impl DeckSource {
    fn valid(&self) -> bool {
        !self.deck.is_empty()
            && self.deck.len() <= 128
            && self.revision > 0
            && self
                .account
                .as_ref()
                .is_none_or(|s| !s.is_empty() && s.len() <= 128)
            && self.gateway.as_deref().is_none_or(valid_origin)
    }
    #[cfg(feature = "client")]
    pub(crate) fn scope(&self) -> (Option<&str>, Option<&str>, &str) {
        (self.gateway.as_deref(), self.account.as_deref(), &self.deck)
    }
}

impl DeckDraft {
    /// Validate bounded storage data before encrypting and again after restoring.
    ///
    /// # Errors
    /// Invalid sizes, scoped references or display names are rejected.
    pub fn validate(&self) -> Result<(), Error> {
        if self.name.trim().is_empty()
            || self.name.len() > 64
            || !self.name.chars().all(central_api::gateway::shown_char)
            || self.format.len() > 64
            || self.description.len() > 16 * 1024
            || self.summary.as_ref().is_some_and(|s| s.len() > 1024)
            || self.cards.len() > 250
            || self.sideboard.len() > 250
            || self.commanders.len() > 2
            || self
                .commanders
                .iter()
                .any(|s| s.is_empty() || s.len() > 512)
            || self
                .cosmetic_origin
                .as_deref()
                .is_some_and(|s| !valid_origin(s))
            || [&self.sleeve, &self.playmat]
                .into_iter()
                .flatten()
                .any(|s| s.is_empty() || s.len() > 128)
        {
            return Err(Error::Rejected);
        }
        let mut bytes = 0;
        for line in self.cards.iter().chain(&self.sideboard) {
            if line.trim().is_empty() || line.len() > 4096 {
                return Err(Error::Rejected);
            }
            bytes += line.len();
        }
        if bytes > 128 * 1024 {
            return Err(Error::Rejected);
        }
        for src in [&self.source, &self.copied_from].into_iter().flatten() {
            if !src.valid() {
                return Err(Error::Rejected);
            }
        }
        Ok(())
    }
}
