use super::{Account, Change, DeckSnapshot, Error, Head, Log, ROOT_LIMIT, io::Staged};
use crate::decks::{Blob, ImportedDeck, ImportedVersion, Root};
use std::collections::BTreeSet;

fn check_sources(decks: &[ImportedDeck]) -> Result<(), Error> {
    if decks.len() > 1000 {
        return Err(Error::Rejected);
    }
    let mut versions = 0_usize;
    let mut total = 0_usize;
    let mut sources = BTreeSet::new();
    for deck in decks {
        versions = versions
            .checked_add(deck.versions.len())
            .ok_or(Error::Rejected)?;
        if versions > 16_384 || deck.versions.len() > 8192 {
            return Err(Error::Rejected);
        }
        let source = deck
            .versions
            .first()
            .and_then(|v| v.draft.source.as_ref())
            .ok_or(Error::Rejected)?;
        if source.account.is_none() || !sources.insert(source.scope()) {
            return Err(Error::Rejected);
        }
        let mut last = 0;
        for version in &deck.versions {
            version.draft.validate()?;
            let member = version.draft.source.as_ref().ok_or(Error::Rejected)?;
            if version.revision <= last
                || member.revision != version.revision
                || member.scope() != source.scope()
            {
                return Err(Error::Rejected);
            }
            last = version.revision;
            total = total
                .checked_add(
                    serde_json::to_vec(&version.draft)
                        .map_err(|_| Error::Rejected)?
                        .len(),
                )
                .ok_or(Error::Rejected)?;
            if total > 256 * 1024 * 1024 {
                return Err(Error::Rejected);
            }
        }
    }
    Ok(())
}

impl Account {
    /// Check a source-generation marker in the currently authenticated library.
    ///
    /// # Errors
    /// Unavailable or damaged roots prevent an absence result.
    pub fn has_deck_import(&mut self, source_key: [u8; 32]) -> Result<bool, Error> {
        Ok(self.read_root()?.1.imports.contains(&source_key))
    }
    /// Import complete histories and their source-generation marker together.
    /// Matching scoped source rows are reused, and divergent local edits conflict.
    ///
    /// # Errors
    /// Invalid, stale, over-quota or interrupted imports leave the current root in place.
    pub fn import_decks(
        &mut self,
        source_key: [u8; 32],
        decks: Vec<ImportedDeck>,
    ) -> Result<(), Error> {
        self.import_decks_count(source_key, decks).map(|_| ())
    }
    /// Import a source generation and return the number of new or advanced deck heads.
    ///
    /// # Errors
    /// Invalid histories, conflicting edits or unavailable storage are refused.
    pub fn import_decks_count(
        &mut self,
        source_key: [u8; 32],
        decks: Vec<ImportedDeck>,
    ) -> Result<usize, Error> {
        check_sources(&decks)?;
        self.change_library(|account, root, stage, expired| {
            if root.imports.contains(&source_key) {
                return Ok(Change::Unchanged(0));
            }
            let mut changed = 0;
            for deck in decks {
                changed += usize::from(account.import_head(root, stage, expired, deck)?);
            }
            root.imports.insert(source_key);
            Ok(Change::Saved(changed))
        })
    }
    fn import_head(
        &self,
        root: &mut Root,
        stage: &mut Staged,
        expired: &mut Vec<Blob>,
        deck: ImportedDeck,
    ) -> Result<bool, Error> {
        let source = deck
            .versions
            .last()
            .and_then(|v| v.draft.source.as_ref())
            .ok_or(Error::Rejected)?
            .clone();
        if root
            .removed
            .values()
            .filter_map(|(_, source)| source.as_ref())
            .any(|s| s.scope() == source.scope())
        {
            return Ok(false);
        }
        let existing = root.heads.values().find(|h| {
            h.source
                .as_ref()
                .is_some_and(|s| s.scope() == source.scope())
        });
        let (id, mut log, prior, mut latest) = if let Some(head) = existing {
            let prior = head.source.as_ref().ok_or(Error::Rejected)?.revision;
            if source.revision > prior && head.summary.revision != prior {
                return Err(Error::Conflict);
            }
            (
                head.summary.id,
                self.deck_log(head)?,
                prior,
                Some(head.summary.clone()),
            )
        } else {
            if root.heads.len() >= 1000 {
                return Err(Error::Rejected);
            }
            let id = self.stage_deck_id(stage)?;
            (id, Log::empty(id), 0, None)
        };
        let mut changed = false;
        for version in deck.versions {
            let same_head = version.revision == source.revision && source.revision == prior;
            if let Some(stored) = log
                .history
                .iter()
                .find(|s| s.save.revision == version.revision)
            {
                let current = self.read_revision(id, stored)?;
                // A gateway can rename a live deck or change its cosmetics in
                // place. Its older rows carry only the card lists, while the
                // last row also proves the complete private metadata.
                if same_head
                    && (current.draft != version.draft
                        || (latest.as_ref().is_some_and(|h| h.revision == prior)
                            && stored.save.superseded_at != version.saved_at))
                {
                    return Err(Error::Conflict);
                }
                if current.draft.cards != version.draft.cards
                    || current.draft.sideboard != version.draft.sideboard
                    || current.draft.commanders != version.draft.commanders
                {
                    return Err(Error::Conflict);
                }
                continue;
            }
            // A coalesced or pruned local save below the last imported head must
            // not be resurrected by replaying an older gateway generation.
            if same_head {
                return Err(Error::Conflict);
            }
            if version.revision <= prior {
                continue;
            }
            let ImportedVersion {
                revision,
                draft,
                saved_at,
            } = version;
            let (summary, stored) = self.stage_revision(
                DeckSnapshot {
                    id,
                    revision,
                    draft,
                },
                saved_at,
                stage,
            )?;
            self.read_revision(id, &stored)?;
            log.history.push(stored);
            latest = Some(summary);
            changed = true;
        }
        if changed {
            if log.history.len() > 8192 {
                return Err(Error::Rejected);
            }
            let summary = latest.ok_or(Error::Rejected)?;
            let blob = self.write_deck_blob(&log, ROOT_LIMIT, stage)?;
            let head = Head {
                summary,
                source: Some(source),
                log: blob,
            };
            self.deck_log(&head)?;
            if let Some(old) = root.heads.insert(id, head) {
                expired.push(old.log);
            }
        }
        Ok(changed)
    }
}
