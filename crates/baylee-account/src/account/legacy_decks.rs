//! Conversion of the authenticated gateway export kept in a migrated account's vault.
//! Old exports store only the card arrays for historical versions. Metadata belongs to the head.
use super::{Account, Error, LegacyData};
use crate::decks::{DeckDraft, DeckSource, ImportedDeck, ImportedVersion};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

#[derive(Deserialize)]
struct SourceLists {
    cards: Vec<String>,
    sideboard: Vec<String>,
    commanders: Vec<String>,
}

#[derive(Deserialize)]
struct Historical {
    deck_id: Uuid,
    version: u64,
    #[serde(flatten)]
    lists: SourceLists,
    summary: Option<String>,
    superseded_at: String,
}

#[derive(Deserialize)]
struct Head {
    id: Uuid,
    account_id: Uuid,
    kind: String,
    name: String,
    format: String,
    version: u64,
    description: Option<String>,
    copied_from: Option<Uuid>,
    copied_version: Option<u64>,
    sleeve: Option<String>,
    playmat: Option<String>,
    #[serde(flatten)]
    lists: SourceLists,
    updated_at: String,
    history: Vec<Historical>,
}

fn timestamp(value: &str) -> Result<i64, Error> {
    OffsetDateTime::parse(value, &Rfc3339)
        .map(OffsetDateTime::unix_timestamp)
        .map_err(|_| Error::Rejected)
}

fn source(origin: &str, account: Option<Uuid>, deck: Uuid, revision: u64) -> DeckSource {
    DeckSource {
        gateway: Some(origin.to_owned()),
        account: account.map(|id| id.to_string()),
        deck: deck.to_string(),
        revision,
    }
}

fn convert(origin: &str, account: Uuid, head: Head) -> Result<ImportedDeck, Error> {
    if head.account_id != account || head.kind != "account" || head.version == 0 {
        return Err(Error::Rejected);
    }
    let copied_from = match (head.copied_from, head.copied_version) {
        (None, None) => None,
        (Some(id), Some(rev)) if rev > 0 => Some(source(origin, None, id, rev)),
        _ => return Err(Error::Rejected),
    };
    let cosmetic_origin = (head.sleeve.is_some() || head.playmat.is_some()).then(|| origin.into());
    let snapshot = |revision, lists: SourceLists, summary| DeckDraft {
        name: head.name.clone(),
        format: head.format.clone(),
        description: head.description.clone().unwrap_or_default(),
        cards: lists.cards,
        sideboard: lists.sideboard,
        commanders: lists.commanders,
        summary,
        sleeve: head.sleeve.clone(),
        playmat: head.playmat.clone(),
        cosmetic_origin: cosmetic_origin.clone(),
        source: Some(source(origin, Some(account), head.id, revision)),
        copied_from: copied_from.clone(),
    };
    let mut versions = Vec::with_capacity(head.history.len() + 1);
    let mut last = 0;
    for old in head.history {
        // The gateway prunes the oldest saves, so the first available version can exceed one.
        if old.deck_id != head.id || old.version <= last || old.version >= head.version {
            return Err(Error::Rejected);
        }
        last = old.version;
        versions.push(ImportedVersion {
            revision: old.version,
            draft: snapshot(old.version, old.lists, old.summary),
            saved_at: timestamp(&old.superseded_at)?,
        });
    }
    versions.push(ImportedVersion {
        revision: head.version,
        draft: snapshot(head.version, head.lists, None),
        saved_at: timestamp(&head.updated_at)?,
    });
    Ok(ImportedDeck { versions })
}

pub(super) fn import_key(legacy: &LegacyData) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"baylee:legacy-deck-library:v1\0");
    hash.update(legacy.gateway.as_bytes());
    hash.update([0]);
    hash.update(legacy.account_id.as_bytes());
    hash.update([0]);
    hash.update(legacy.digest);
    hash.finalize().into()
}

pub(super) fn decode_heads(
    origin: &str,
    account_id: &str,
    rows: &[serde_json::Value],
) -> Result<Vec<ImportedDeck>, Error> {
    if rows.len() > 1000 {
        return Err(Error::Rejected);
    }
    let account = Uuid::parse_str(account_id).map_err(|_| Error::Rejected)?;
    let mut decks = Vec::with_capacity(rows.len());
    let mut ids = BTreeSet::new();
    for row in rows {
        let head: Head = Head::deserialize(row).map_err(|_| Error::Rejected)?;
        if !ids.insert(head.id) {
            return Err(Error::Rejected);
        }
        decks.push(convert(origin, account, head)?);
    }
    Ok(decks)
}

impl Account {
    /// Publish the full set once. Keep the original encrypted export on every failure.
    pub(super) fn ensure_legacy_decks(&mut self) -> Result<(), Error> {
        let Some(legacy) = self.data.legacy.as_ref() else {
            return Ok(());
        };
        let rows = legacy.decks.as_array().ok_or(Error::Rejected)?;
        if rows.is_empty() {
            return Ok(());
        }
        let key = import_key(legacy);
        if self.has_deck_import(key)? {
            return Ok(());
        }
        // Reading the current library can refresh the account document. Only import the
        // authenticated source whose migration marker we just checked.
        let legacy = self.data.legacy.as_ref().ok_or(Error::Conflict)?;
        if import_key(legacy) != key {
            return Err(Error::Conflict);
        }
        let decks = decode_heads(
            &legacy.gateway,
            &legacy.account_id,
            legacy.decks.as_array().ok_or(Error::Rejected)?,
        )?;
        self.import_decks(key, decks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn row(owner: Uuid, deck: Uuid) -> Value {
        json!({
            "id":deck,"account_id":owner,"kind":"account","format":"freeform",
            "name":"Rain","cards":["4 Island"],"sideboard":[],"commanders":[],
            "description":"old note","version":17,"updated_at":"2026-11-22T09:16:22.013+01:00",
            "sleeve":"gateway-image","history":[{
                "deck_id":deck,"version":16,"cards":["3 Island"],"sideboard":["1 Plains"],
                "commanders":[],"summary":"One land","superseded_at":"2026-11-21T11:15:00+00:00"
            }]
        })
    }

    #[test]
    fn imports_the_retained_history_and_scoped_cosmetics_without_renumbering() {
        let account = Uuid::now_v7();
        let deck = Uuid::now_v7();
        let origin = "https://home.example";
        let imported = convert(
            origin,
            account,
            Head::deserialize(&row(account, deck)).unwrap(),
        )
        .unwrap();
        let [old, current] = imported.versions.as_slice() else {
            panic!("two original versions")
        };
        assert_eq!((old.revision, current.revision), (16, 17));
        assert_eq!(old.draft.sideboard, ["1 Plains"]);
        assert_eq!(old.draft.summary.as_deref(), Some("One land"));
        assert_eq!(current.draft.cards, ["4 Island"]);
        assert_eq!(current.draft.cosmetic_origin.as_deref(), Some(origin));
        assert_eq!(
            current.draft.source.as_ref().unwrap().deck,
            deck.to_string()
        );
        assert!(current.saved_at > old.saved_at);
    }

    #[test]
    fn refuses_other_owners_foreign_versions_and_conflicting_copy_provenance() {
        let account = Uuid::now_v7();
        let deck = Uuid::now_v7();
        let assert_refused = |value| {
            let head = Head::deserialize(&value).unwrap();
            assert!(convert("https://home.example", account, head).is_err());
        };
        assert_refused(row(Uuid::now_v7(), deck));
        let mut foreign_version = row(account, deck);
        foreign_version["history"][0]["deck_id"] = json!(Uuid::now_v7());
        assert_refused(foreign_version);
        let mut duplicate = row(account, deck);
        duplicate["history"][0]["version"] = json!(17);
        assert_refused(duplicate);
        let mut missing_number = row(account, deck);
        missing_number["copied_from"] = json!(Uuid::now_v7());
        assert_refused(missing_number);
    }
}
