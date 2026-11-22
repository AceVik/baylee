//! Private deck library. An immutable root contains complete heads; publishing
//! it through the account CAS commits every new snapshot and history together.
use super::{Account, Error};
use crate::decks::{
    Blob, DeckDraft, DeckSave, DeckSnapshot, DeckSource, DeckSummary, Head, Log, Revision, Root,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
mod import;
mod io;
mod write;
use io::{ROOT_LIMIT, SNAPSHOT_LIMIT, Staged};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    kind: String,
    snapshot: DeckSnapshot,
}
fn now() -> Result<i64, Error> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .ok_or(Error::Unavailable)
}
fn summarize(s: &DeckSnapshot, updated_at: i64) -> Result<DeckSummary, Error> {
    s.draft.validate()?;
    let copies = |rows: &[String]| {
        rows.iter().try_fold(0_u32, |total, row| {
            let count = baylee_core::deckrow::parse(row)
                .map_err(|_| Error::Rejected)?
                .count;
            total
                .checked_add(count)
                .filter(|n| *n <= 250)
                .ok_or(Error::Rejected)
        })
    };
    let d = &s.draft;
    Ok(DeckSummary {
        id: s.id,
        revision: s.revision,
        name: d.name.clone(),
        format: d.format.clone(),
        cards: d.cards.len(),
        sideboard: d.sideboard.len(),
        copies: copies(&d.cards)?,
        side_copies: copies(&d.sideboard)?,
        commanders: d.commanders.clone(),
        updated_at,
    })
}
enum Change<T> {
    Saved(T),
    Unchanged(T),
}
type SourceChecks = std::collections::BTreeMap<(String, u64), [u8; 32]>;

fn save_digest(draft: &DeckDraft, saved_at: i64) -> Result<[u8; 32], Error> {
    use sha2::{Digest, Sha256};
    serde_json::to_vec(&(draft, saved_at))
        .map(|bytes| Sha256::digest(bytes).into())
        .map_err(|_| Error::Rejected)
}
pub(super) fn source_checks(decks: &[crate::decks::ImportedDeck]) -> Result<SourceChecks, Error> {
    let mut checks = SourceChecks::new();
    for version in decks.iter().flat_map(|deck| &deck.versions) {
        let source = version.draft.source.as_ref().ok_or(Error::Rejected)?;
        if checks
            .insert(
                (source.deck.clone(), source.revision),
                save_digest(&version.draft, version.saved_at)?,
            )
            .is_some()
        {
            return Err(Error::Rejected);
        }
    }
    Ok(checks)
}

impl Account {
    pub(super) fn verify_prelink_decks(
        &mut self,
        root_digest: [u8; 32],
        marker: [u8; 32],
        origin: &str,
        owner: &str,
        mut expected: Option<SourceChecks>,
    ) -> Result<(), Error> {
        let (root, book) = self.read_root()?;
        if root.as_ref().map(|r| r.blob.digest) != Some(root_digest)
            || !book.imports.contains(&marker)
        {
            return Err(Error::Conflict);
        }
        for head in book.heads.values().filter(|h| {
            h.source.as_ref().is_some_and(|s| {
                s.gateway.as_deref() == Some(origin) && s.account.as_deref() == Some(owner)
            })
        }) {
            for version in self.deck_log(head)?.history {
                let snapshot = self.read_revision(head.summary.id, &version)?;
                if let Some(checks) = expected.as_mut()
                    && let Some(source) = &snapshot.draft.source
                    && let Some(original) = checks.remove(&(source.deck.clone(), source.revision))
                    && original != save_digest(&snapshot.draft, version.save.superseded_at)?
                {
                    return Err(Error::Conflict);
                }
            }
        }
        if expected.is_some_and(|checks| !checks.is_empty()) {
            return Err(Error::Conflict);
        }
        Ok(())
    }
    fn change_library<T>(
        &mut self,
        run: impl FnOnce(&Self, &mut Root, &mut Staged, &mut Vec<Blob>) -> Result<Change<T>, Error>,
    ) -> Result<T, Error> {
        let (base, mut root) = self.read_root()?;
        let mut stage = Staged::new();
        let mut expired = Vec::new();
        let result =
            run(self, &mut root, &mut stage, &mut expired).and_then(|change| match change {
                Change::Unchanged(value) => Ok(value),
                Change::Saved(value) => {
                    self.publish_root(base.as_ref(), root, &mut stage)?;
                    if let Some(old) = base {
                        expired.push(old.blob);
                    }
                    for blob in expired {
                        self.forget_blob(blob);
                    }
                    Ok(value)
                }
            });
        if result.is_err() {
            stage.abort(self);
        }
        result
    }
    fn deck_log(&self, head: &Head) -> Result<Log, Error> {
        let log: Log = self.read_deck_blob(&head.log, ROOT_LIMIT)?;
        log.validate(head)?;
        Ok(log)
    }
    fn read_revision(&self, id: Uuid, entry: &Revision) -> Result<DeckSnapshot, Error> {
        let doc: Stored = self.read_deck_blob(&entry.blob, SNAPSHOT_LIMIT)?;
        let s = doc.snapshot;
        if doc.kind != "baylee/deck-snapshot/1"
            || s.id != id
            || s.revision != entry.save.revision
            || s.draft.cards.len() != entry.save.cards
            || s.draft.sideboard.len() != entry.save.sideboard
        {
            return Err(Error::Rejected);
        }
        s.draft.validate()?;
        Ok(s)
    }
    fn stage_revision(
        &self,
        s: DeckSnapshot,
        date: i64,
        stage: &mut Staged,
    ) -> Result<(DeckSummary, Revision), Error> {
        let summary = summarize(&s, date)?;
        let save = DeckSave {
            revision: s.revision,
            summary: s.draft.summary.clone(),
            cards: summary.cards,
            sideboard: summary.sideboard,
            superseded_at: date,
        };
        let blob = self.write_deck_blob(
            &Stored {
                kind: "baylee/deck-snapshot/1".into(),
                snapshot: s,
            },
            SNAPSHOT_LIMIT,
            stage,
        )?;
        Ok((summary, Revision { save, blob }))
    }
    /// Allocate a server-issued ID reusable after a lost create reply.
    ///
    /// # Errors
    /// Refuses unavailable or unauthorized storage.
    pub fn reserve_deck_id(&self) -> Result<Uuid, Error> {
        let mut stage = Staged::new();
        let result = self.stage_deck_id(&mut stage);
        if result.is_err() {
            stage.abort(self);
        }
        result
    }
    /// List private decks, newest first, after refreshing the encrypted account.
    ///
    /// # Errors
    /// Authentication failures or damaged manifests prevent partial listings.
    pub fn list_decks(&mut self) -> Result<Vec<DeckSummary>, Error> {
        self.ensure_legacy_decks()?;
        let (_, root) = self.read_root()?;
        let mut rows: Vec<_> = root.heads.into_values().map(|h| h.summary).collect();
        rows.sort_by_key(|s| (std::cmp::Reverse(s.updated_at), s.id));
        Ok(rows)
    }
    fn read_head<T>(
        &mut self,
        id: Uuid,
        read: impl Fn(&Self, &Head, &Log) -> Result<T, Error>,
    ) -> Result<T, Error> {
        self.ensure_legacy_decks()?;
        for _ in 0..3 {
            let (pointer, root) = self.read_root()?;
            let head = root.heads.get(&id).ok_or(Error::Rejected)?;
            match self.deck_log(head).and_then(|log| read(self, head, &log)) {
                Ok(value) => return Ok(value),
                Err(error) => {
                    // A concurrent replacement can reclaim the old log between
                    // the two reads. Retry only after authenticating a newer root.
                    if self.refresh_library()? == pointer {
                        return Err(error);
                    }
                }
            }
        }
        Err(Error::Conflict)
    }
    fn snapshot_at(&mut self, id: Uuid, requested: Option<u64>) -> Result<DeckSnapshot, Error> {
        self.read_head(id, |account, head, log| {
            let rev = requested.unwrap_or(head.summary.revision);
            let entry = log
                .history
                .iter()
                .find(|e| e.save.revision == rev)
                .ok_or(Error::Rejected)?;
            account.read_revision(id, entry)
        })
    }
    /// Restore the current private deck and its original printing choices.
    ///
    /// # Errors
    /// Unknown, tombstoned or damaged decks cannot be loaded.
    pub fn load_deck(&mut self, id: Uuid) -> Result<DeckSnapshot, Error> {
        self.snapshot_at(id, None)
    }
    /// Restore an exact logical save of a private deck.
    ///
    /// # Errors
    /// Unknown decks or saves, and damaged data, are refused.
    pub fn deck_version(&mut self, id: Uuid, revision: u64) -> Result<DeckSnapshot, Error> {
        self.snapshot_at(id, Some(revision))
    }
    /// Read the current head and its superseded saves from the same generation.
    ///
    /// # Errors
    /// Unavailable decks and damaged histories are refused.
    pub fn deck_history_page(&mut self, id: Uuid) -> Result<(DeckSummary, Vec<DeckSave>), Error> {
        self.read_head(id, |_, head, log| {
            let history = log
                .history
                .iter()
                .rev()
                .skip(1)
                .map(|r| r.save.clone())
                .collect();
            Ok((head.summary.clone(), history))
        })
    }
    /// Read superseded saves, newest first.
    ///
    /// # Errors
    /// Unavailable decks and damaged histories are refused.
    pub fn deck_history(&mut self, id: Uuid) -> Result<Vec<DeckSave>, Error> {
        Ok(self.deck_history_page(id)?.1)
    }
}
