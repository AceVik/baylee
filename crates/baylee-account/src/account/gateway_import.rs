//! Resumable, bounded gateway library import. The encrypted Central root is
//! changed only once a complete source deck has been read consistently.
use super::Account;
use crate::decks::{DeckSource, ImportedDeck, ImportedVersion, gateway_export::GatewayDeckVersion};
use crate::{Admission, Endpoint, Error, GatewayHttp};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Private cursor. Drop it on logout, cancellation or change of gateway.
/// Restarting skips already published source generations.
#[derive(Default)]
pub struct GatewayDeckImport {
    binding: Option<(Endpoint, Uuid, String)>,
    connection: Option<(GatewayHttp, Admission)>,
    local: Option<Uuid>,
    after: Option<Uuid>,
    current: Option<Reading>,
    scanned: usize,
    imported: usize,
    done: bool,
}

struct Reading {
    id: Uuid,
    generation: [u8; 32],
    head: u64,
    marker: [u8; 32],
    versions: Vec<ImportedVersion>,
}

impl GatewayDeckImport {
    /// Complete source decks inspected, including previously migrated ones.
    #[must_use]
    pub const fn scanned_decks(&self) -> usize {
        self.scanned
    }
    /// Newly processed source generations published in the Central library.
    #[must_use]
    pub const fn imported_decks(&self) -> usize {
        self.imported
    }
}

fn marker(origin: &str, page: &GatewayDeckVersion) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"baylee:private-gateway-deck:v1\0");
    hash.update(origin.as_bytes());
    hash.update([0]);
    hash.update(page.central.as_bytes());
    hash.update(page.owner.as_bytes());
    hash.update(page.deck.as_bytes());
    hash.update(page.generation);
    hash.finalize().into()
}

impl Account {
    /// Advance one migration page or publish one completed source deck. Returns
    /// true on reaching the end of the authenticated owner's shelf.
    ///
    /// # Errors
    /// An edited source or divergent local library returns a conflict. Transient
    /// failures leave the source intact and the cursor safe to retry.
    pub fn gateway_deck_import_step(
        &mut self,
        gateway: &str,
        progress: &mut GatewayDeckImport,
    ) -> Result<bool, Error> {
        let own = Uuid::from_bytes(self.session.identity().account);
        let binding = (self.http.endpoint().clone(), own, gateway.to_owned());
        if progress
            .binding
            .as_ref()
            .is_some_and(|before| before != &binding)
        {
            return Err(Error::Conflict);
        }
        if progress.done {
            return Ok(true);
        }
        if progress.connection.is_none() {
            let (source, token) = self.gateway_token(gateway)?;
            let admission = source.admit(&token).map_err(|why| why.error)?;
            if admission.terms_stale == Some(true) {
                return Err(Error::TermsRequired);
            }
            progress.connection = Some((source, admission));
            progress.binding = Some(binding);
        }
        if progress
            .current
            .as_ref()
            .is_some_and(|r| r.versions.last().is_some_and(|v| v.revision == r.head))
        {
            let reading = progress.current.take().ok_or(Error::Conflict)?;
            self.import_decks(
                reading.marker,
                vec![ImportedDeck {
                    versions: reading.versions,
                }],
            )?;
            progress.after = Some(reading.id);
            progress.scanned += 1;
            progress.imported += 1;
            return Ok(false);
        }
        let cursor = progress
            .current
            .as_ref()
            .and_then(|r| r.versions.last().map(|v| (r.id, v.revision)));
        let (source, admission) = progress.connection.as_ref().ok_or(Error::Conflict)?;
        let reply = source.private_deck_save(&admission.token, progress.after, cursor);
        let page = match reply {
            Ok(Some(page)) => page,
            Ok(None) if cursor.is_none() => {
                progress.done = true;
                progress.connection = None;
                return Ok(true);
            }
            Ok(None) | Err(Error::Conflict) => {
                progress.current = None;
                return Err(Error::Conflict);
            }
            Err(error) => {
                if matches!(error, Error::Rejected | Error::TermsRequired) {
                    progress.connection = None;
                }
                return Err(error);
            }
        };
        self.accept_gateway_page(progress, gateway, own, page)?;
        Ok(false)
    }

    fn accept_gateway_page(
        &mut self,
        progress: &mut GatewayDeckImport,
        gateway: &str,
        own: Uuid,
        page: GatewayDeckVersion,
    ) -> Result<(), Error> {
        if progress.scanned >= 1000
            || page.central != own
            || page.deck.get_version_num() != 7
            || page.owner.is_nil()
            || progress.local.is_some_and(|local| page.owner != local)
            || progress.after.is_some_and(|last| page.deck <= last)
            || page.draft.source.is_some()
            || page.draft.copied_from.is_some()
            || page.draft.cosmetic_origin.is_some()
            || page.revision == 0
            || page.revision > page.head_revision
            || page.head_revision > i32::MAX as u64
        {
            return Err(Error::Rejected);
        }
        progress.local = Some(page.owner);
        let stamp = marker(gateway, &page);
        if let Some(r) = progress.current.as_ref() {
            if r.generation != page.generation || r.id != page.deck || r.head != page.head_revision
            {
                progress.current = None;
                return Err(Error::Conflict);
            }
            if r.versions.len() >= 51
                || r.versions
                    .last()
                    .is_some_and(|v| page.revision <= v.revision)
            {
                return Err(Error::Rejected);
            }
        } else if self.has_deck_import(stamp)? {
            progress.scanned += 1;
            progress.after = Some(page.deck);
            return Ok(());
        }
        let revision = page.revision;
        let id = page.deck;
        let local = page.owner.to_string();
        let src = |deck: Uuid, rev: u64, account: Option<String>| DeckSource {
            gateway: Some(gateway.to_owned()),
            account,
            deck: deck.to_string(),
            revision: rev,
        };
        let mut draft = page.draft;
        draft.source = Some(src(id, revision, Some(local)));
        draft.copied_from = page.copied.map(|(deck, rev)| src(deck, rev, None));
        if draft.sleeve.is_some() || draft.playmat.is_some() {
            draft.cosmetic_origin = Some(gateway.to_owned());
        }
        draft.validate()?;
        progress
            .current
            .get_or_insert_with(|| Reading {
                id,
                generation: page.generation,
                head: page.head_revision,
                marker: stamp,
                versions: Vec::new(),
            })
            .versions
            .push(ImportedVersion {
                revision,
                draft,
                saved_at: page.saved_at,
            });
        Ok(())
    }
}
