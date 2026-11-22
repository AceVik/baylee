use super::{
    Account, Change, DeckDraft, DeckSnapshot, DeckSource, DeckSummary, Error, Head, Log,
    ROOT_LIMIT, Uuid, now,
};

impl Account {
    /// Save a conditional edit, retaining the prior snapshot in history.
    ///
    /// # Errors
    /// Invalid, stale or unavailable writes preserve the previous library.
    pub fn save_deck(
        &mut self,
        id: Option<Uuid>,
        expected: Option<u64>,
        draft: DeckDraft,
    ) -> Result<DeckSummary, Error> {
        self.save_deck_visit(id, expected, draft, true)
    }
    /// Save a whole builder draft, optionally coalescing the previous working
    /// state within the same visit. Logical revision increases on each change.
    ///
    /// # Errors
    /// Missing authorization, invalid drafts or stale revisions are refused.
    pub fn save_deck_visit(
        &mut self,
        id: Option<Uuid>,
        expected: Option<u64>,
        draft: DeckDraft,
        archive: bool,
    ) -> Result<DeckSummary, Error> {
        self.ensure_legacy_decks()?;
        draft.validate()?;
        if id.is_none() && expected.is_some() {
            return Err(Error::Rejected);
        }
        self.change_library(|account, root, stage, expired| {
            let id = match id {
                Some(id) => id,
                None => account.stage_deck_id(stage)?,
            };
            if root.removed.contains_key(&id) {
                return Err(Error::Conflict);
            }
            let mut log = if let Some(head) = root.heads.get(&id) {
                let log = account.deck_log(head)?;
                let cur = log.history.last().ok_or(Error::Rejected)?;
                if (Some(head.summary.revision) == expected
                    || expected.unwrap_or(0).checked_add(1) == Some(head.summary.revision))
                    && account.read_revision(id, cur)?.draft == draft
                {
                    return Ok(Change::Unchanged(head.summary.clone()));
                }
                if expected != Some(head.summary.revision) {
                    return Err(Error::Conflict);
                }
                if draft.source != head.source {
                    return Err(Error::Rejected);
                }
                expired.push(head.log.clone());
                log
            } else {
                if expected.is_some() || root.heads.len() >= 1000 {
                    return Err(Error::Conflict);
                }
                account.verify_reservation(id)?;
                Log::empty(id)
            };
            let revision = expected
                .unwrap_or(0)
                .checked_add(1)
                .ok_or(Error::Rejected)?;
            let date = now()?;
            let source = draft.source.clone();
            let summary_text = draft.summary.clone();
            let (summary, new) = account.stage_revision(
                DeckSnapshot {
                    id,
                    revision,
                    draft,
                },
                date,
                stage,
            )?;
            if archive {
                if let Some(old) = log.history.last_mut() {
                    old.save.superseded_at = date;
                    old.save.summary = summary_text;
                }
            } else if let Some(old) = log.history.pop() {
                expired.push(old.blob);
            }
            log.history.push(new);
            if log.history.len() > 8192 {
                return Err(Error::Rejected);
            }
            let saved = account.write_deck_blob(&log, ROOT_LIMIT, stage)?;
            root.heads.insert(
                id,
                Head {
                    summary: summary.clone(),
                    source,
                    log: saved,
                },
            );
            Ok(Change::Saved(summary))
        })
    }
    /// Copy an owned snapshot into a new deck with explicit provenance.
    ///
    /// # Errors
    /// Missing source revisions and invalid new names are refused.
    pub fn copy_deck(&mut self, source: DeckSnapshot, name: String) -> Result<DeckSummary, Error> {
        let original = self.deck_version(source.id, source.revision)?;
        if original != source {
            return Err(Error::Conflict);
        }
        let mut draft = original.draft;
        draft.copied_from = Some(DeckSource {
            gateway: None,
            account: None,
            deck: source.id.to_string(),
            revision: source.revision,
        });
        draft.source = None;
        draft.name = name;
        self.save_deck(None, None, draft)
    }
    /// Restore a historical snapshot as a new logical revision.
    ///
    /// # Errors
    /// A stale head, missing revision or unreachable storage prevents restoration.
    pub fn revert_deck(
        &mut self,
        id: Uuid,
        expected: u64,
        revision: u64,
    ) -> Result<DeckSummary, Error> {
        let mut draft = self.deck_version(id, revision)?.draft;
        draft.source = self.load_deck(id)?.draft.source;
        self.save_deck(Some(id), Some(expected), draft)
    }
    /// Publish a durable tombstone before removing unreferenced snapshots.
    ///
    /// # Errors
    /// A changed head cannot be deleted with an earlier expected revision.
    pub fn delete_deck(&mut self, id: Uuid, expected: u64) -> Result<(), Error> {
        self.ensure_legacy_decks()?;
        self.change_library(|account, root, _, expired| {
            if root.removed.contains_key(&id) {
                return Ok(Change::Unchanged(()));
            }
            let head = root.heads.remove(&id).ok_or(Error::Rejected)?;
            if head.summary.revision != expected {
                return Err(Error::Conflict);
            }
            expired.extend(account.deck_log(&head)?.history.into_iter().map(|r| r.blob));
            expired.push(head.log);
            root.removed.insert(id, head.source);
            Ok(Change::Saved(()))
        })?;
        let _ = self.session.delete_object(&self.http, *id.as_bytes(), 1);
        Ok(())
    }
}
