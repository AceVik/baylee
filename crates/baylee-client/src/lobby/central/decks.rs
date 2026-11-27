//! Private deck operations use the serial Central account worker. Only the
//! public starter catalogue belongs to the selected gateway.
use super::Handle;
use crate::lobby::{LobbyEvent, LobbyRequest, LobbyState, Mailbox, Reply};
use baylee_account::{
    Account, Error,
    decks::{DeckDraft, DeckSummary},
};
use baylee_client_core::{
    Phrase,
    deckbuilder::saves::DeckSave,
    i18n::Lang,
    lobby::library::{self, Request as LibraryRequest},
};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicU64, Ordering},
};
use uuid::Uuid;

pub(in crate::lobby) mod create;
mod history;
mod play;
pub(crate) use play::{play_commanders, play_rows};
mod shared;

#[derive(Default)]
pub(super) struct Versions {
    listed: BTreeMap<Uuid, u64>,
    /// Loaded or just committed in the editor. A background list cannot
    /// advance the base of a draft that was opened on another revision.
    edited: BTreeMap<Uuid, u64>,
    history: Option<(Uuid, u64)>,
}

pub(super) fn new_epoch() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1 << 63);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

impl Handle {
    /// Advance only a revision this worker had actually seen. A newer edit on
    /// another device must still conflict with a stale open builder.
    pub(in crate::lobby) fn appearance_saved(&self, id: Uuid, previous: u64, saved: u64) {
        if let Ok(mut versions) = self.library.lock() {
            if versions.edited.get(&id) == Some(&previous) {
                versions.edited.insert(id, saved);
            }
            versions
                .listed
                .entry(id)
                .and_modify(|r| *r = (*r).max(saved))
                .or_insert(saved);
        }
    }

    #[cfg(test)]
    pub(in crate::lobby) fn set_listed_revision_for_test(&self, id: Uuid, revision: u64) {
        self.library.lock().unwrap().listed.insert(id, revision);
    }

    pub(crate) fn listed_deck_revision(&self, id: Uuid) -> Result<u64, Error> {
        self.library
            .try_lock()
            .map_err(|_| Error::Busy)?
            .listed
            .get(&id)
            .copied()
            .ok_or(Error::Conflict)
    }

    /// A preview belongs to the concrete deck revision read. The render
    /// thread never waits behind an account-worker network operation.
    pub(crate) fn deck_revision_changed(&self, id: Uuid, revision: u64) -> bool {
        self.library.try_lock().is_ok_and(|versions| {
            versions
                .listed
                .get(&id)
                .is_none_or(|&latest| latest > revision)
        })
    }

    pub(crate) fn draft_store(&self) -> Option<crate::buildui::draft::private::Private> {
        self.draft_store.clone()
    }
    pub(crate) fn library_epoch(&self) -> u64 {
        self.library_epoch
    }

    /// `AppExit` is the one blocking deck operation. A deletion whose undo
    /// expired during shutdown still targets the authenticated library.
    pub(in crate::lobby) fn flush_deck_delete(&self, key: &str) -> Result<(), Error> {
        let id = id(key)?;
        let versions = self.library.clone();
        self.call(move |account| {
            let expected = versions
                .lock()
                .map_err(|_| Error::Unavailable)?
                .expected(id)?;
            account.delete_deck(id, expected)
        })
    }
}

fn id(value: &str) -> Result<Uuid, Error> {
    let id = Uuid::parse_str(value).map_err(|_| Error::Rejected)?;
    if id.get_version_num() != 7 || id.get_variant() != uuid::Variant::RFC4122 {
        return Err(Error::Rejected);
    }
    Ok(id)
}

fn failure(lang: Lang, error: Error) -> String {
    if error == Error::Conflict {
        Phrase::PrivateDeckChanged.text(lang).into()
    } else {
        Phrase::LibraryReadFailed.text(lang).into()
    }
}

/// Maximum deck name length accepted by `DeckDraft::validate`.
const MAX_DECK_NAME_BYTES: usize = 64;

fn fit_name(text: &str, max_bytes: usize) -> &str {
    let mut end = text.len().min(max_bytes);
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].trim_end()
}

/// A distinct name for a duplicated personal deck, matching the offline
/// library (`Phrase::ListCopyName`) and staying within Central's 64-byte cap.
fn copy_name(original: &str, existing: &[String], lang: Lang) -> String {
    let raw = Phrase::ListCopyName.fill(lang, &[original]);
    let base = fit_name(&raw, MAX_DECK_NAME_BYTES).to_string();
    if !existing.iter().any(|n| n.eq_ignore_ascii_case(&base)) {
        return base;
    }
    for n in 2usize.. {
        let suffix = format!(" ({n})");
        let prefix = fit_name(&base, MAX_DECK_NAME_BYTES.saturating_sub(suffix.len()));
        let candidate = format!("{prefix}{suffix}");
        if !existing
            .iter()
            .any(|name| name.eq_ignore_ascii_case(&candidate))
        {
            return candidate;
        }
    }
    base
}

enum CopySource {
    Own {
        id: Uuid,
        existing: Vec<String>,
        lang: Lang,
    },
    Published(shared::Source),
}

fn copy_source(state: &mut LobbyState, key: &str) -> Result<CopySource, Error> {
    if state.lobby.decks().iter().any(|deck| deck.id == key) {
        let existing = state
            .lobby
            .decks()
            .iter()
            .map(|deck| deck.name.clone())
            .collect();
        return Ok(CopySource::Own {
            id: id(key)?,
            existing,
            lang: state.lobby.lang(),
        });
    }
    Ok(CopySource::Published(shared::published(state, key)?))
}

#[derive(Clone, Copy)]
enum Target {
    Event,
    Library,
    Autosave,
}

impl Target {
    fn refused(self, lang: Lang, error: Error) -> Reply {
        let why = failure(lang, error);
        match self {
            Self::Event => Reply::Event(LobbyEvent::Failed(why)),
            Self::Library => Reply::Event(LobbyEvent::Library(library::Reply::Failed(why))),
            Self::Autosave => Reply::AutoSaveFailed { why: Some(why) },
        }
    }
}

fn submit(
    state: &LobbyState,
    mailbox: &Mailbox,
    target: Target,
    work: impl FnOnce(&mut Account, &mut Versions) -> Result<Reply, Error> + Send + 'static,
) {
    let Some(handle) = state.central_handle() else {
        return;
    };
    let session = handle.clone();
    let versions = handle.library.clone();
    let lang = state.lobby.lang();
    let writer = mailbox.0.clone();
    let accepted = handle.submit(move |account| {
        let reply = versions
            .lock()
            .map_err(|_| Error::Unavailable)
            .and_then(|mut versions| work(account, &mut versions))
            .unwrap_or_else(|error| target.refused(lang, error));
        if let Ok(mut replies) = writer.lock() {
            replies.push(Reply::LibrarySession(session, Box::new(reply)));
        }
    });
    if !accepted && let Ok(mut replies) = mailbox.0.lock() {
        replies.push(Reply::LibrarySession(
            handle,
            Box::new(target.refused(lang, Error::Busy)),
        ));
    }
}

#[allow(clippy::too_many_lines)]
pub(in crate::lobby) fn dispatch(
    state: &mut LobbyState,
    mailbox: &Mailbox,
    request: LobbyRequest,
) -> Option<LobbyRequest> {
    if !state.has_central_account() {
        return Some(request);
    }
    match request {
        LobbyRequest::ListDecks => submit(state, mailbox, Target::Event, |account, versions| {
            let decks = account.list_decks()?;
            versions.listed = decks.iter().map(|d| (d.id, d.revision)).collect();
            versions
                .edited
                .retain(|id, _| versions.listed.contains_key(id));
            Ok(Reply::Event(LobbyEvent::Decks(
                decks.into_iter().map(summary).collect(),
            )))
        }),
        LobbyRequest::LoadDeck { deck_id } => {
            submit(state, mailbox, Target::Event, move |account, versions| {
                let id = id(&deck_id)?;
                let deck = account.load_deck(id)?;
                versions.edited.insert(id, deck.revision);
                let d = deck.draft;
                Ok(Reply::Event(LobbyEvent::DeckLoaded {
                    id: deck_id,
                    name: d.name,
                    format: d.format,
                    cards: d.cards,
                    sideboard: d.sideboard,
                    commanders: d.commanders,
                }))
            });
        }
        LobbyRequest::SaveDeck {
            deck_id,
            name,
            cards,
            sideboard,
            commanders,
            format,
        } => {
            let draft = Updated {
                name,
                cards,
                sideboard,
                commanders,
                format,
            };
            submit(state, mailbox, Target::Event, move |account, versions| {
                let saved = versions.save(account, deck_id.as_deref(), draft, true)?;
                Ok(Reply::Event(LobbyEvent::DeckSaved {
                    deck_id: deck_id.is_none().then(|| saved.id.to_string()),
                }))
            });
        }
        LobbyRequest::DeleteDeck { deck_id } => {
            submit(state, mailbox, Target::Event, move |account, versions| {
                let id = id(&deck_id)?;
                account.delete_deck(id, versions.expected(id)?)?;
                versions.edited.remove(&id);
                versions.listed.remove(&id);
                Ok(Reply::Event(LobbyEvent::DeckDeleted))
            });
        }
        LobbyRequest::Library(LibraryRequest::Copy(key)) => {
            let source = copy_source(state, &key);
            submit(state, mailbox, Target::Library, move |account, versions| {
                let saved = match source? {
                    CopySource::Own { id, existing, lang } => {
                        let snapshot = account.load_deck(id)?;
                        let name = copy_name(&snapshot.draft.name, &existing, lang);
                        account.copy_deck(snapshot, name)?
                    }
                    CopySource::Published(published) => {
                        let draft = published.read()?;
                        account.save_deck(None, None, draft)?
                    }
                };
                versions.listed.insert(saved.id, saved.revision);
                Ok(Reply::Event(LobbyEvent::Library(library::Reply::Copied(
                    saved.id.to_string(),
                ))))
            });
        }
        LobbyRequest::Library(request) => {
            let shared = match &request {
                LibraryRequest::House => true,
                LibraryRequest::History(key) | LibraryRequest::Version(key, _) => {
                    state
                        .lobby
                        .library()
                        .house
                        .iter()
                        .any(|house| house.id == *key)
                        && !state.lobby.decks().iter().any(|deck| deck.id == *key)
                }
                _ => false,
            };
            if shared {
                return Some(LobbyRequest::Library(request));
            }
            submit(state, mailbox, Target::Library, move |account, versions| {
                Ok(Reply::Event(LobbyEvent::Library(history::apply(
                    account, versions, request,
                )?)))
            });
        }
        other => return Some(other),
    }
    None
}

fn summary(value: DeckSummary) -> baylee_client_core::lobby::DeckSummary {
    baylee_client_core::lobby::DeckSummary {
        id: value.id.to_string(),
        name: value.name,
        format: value.format,
        cards: value.cards,
        sideboard: value.sideboard,
        copies: value.copies,
        side_copies: value.side_copies,
        commanders: value.commanders,
        updated_at: u64::try_from(value.updated_at).unwrap_or_default(),
        ..Default::default()
    }
}

struct Updated {
    name: String,
    cards: Vec<String>,
    sideboard: Vec<String>,
    commanders: Vec<String>,
    format: Option<String>,
}

impl Versions {
    fn expected(&self, id: Uuid) -> Result<u64, Error> {
        self.edited
            .get(&id)
            .or_else(|| self.listed.get(&id))
            .copied()
            .ok_or(Error::Conflict)
    }

    fn save(
        &mut self,
        account: &mut Account,
        identity: Option<&str>,
        edits: Updated,
        archive: bool,
    ) -> Result<DeckSummary, Error> {
        let deck = identity.map(id).transpose()?;
        let (expected, mut draft) = if let Some(id) = deck {
            let expected = self.expected(id)?;
            let source = account.load_deck(id)?;
            if source.revision != expected {
                return Err(Error::Conflict);
            }
            (Some(expected), source.draft)
        } else {
            (None, DeckDraft::default())
        };
        draft.name = edits.name;
        draft.cards = edits.cards;
        draft.sideboard = edits.sideboard;
        draft.commanders = edits.commanders;
        if let Some(format) = edits.format {
            draft.format = format;
        } else if draft.format.is_empty() {
            draft.format = if draft.commanders.is_empty() {
                "freeform"
            } else {
                "commander"
            }
            .into();
        }
        let saved = account.save_deck_visit(deck, expected, draft, archive)?;
        self.edited.insert(saved.id, saved.revision);
        self.listed.insert(saved.id, saved.revision);
        Ok(saved)
    }
}

pub(in crate::lobby) fn autosave(state: &mut LobbyState, mailbox: &Mailbox, save: &DeckSave) {
    if save.draft.editing.is_none() {
        create::start(state, mailbox, save);
        return;
    }
    let save = save.clone();
    submit(
        state,
        mailbox,
        Target::Autosave,
        move |account, versions| {
            let existing = save.draft.editing;
            let edits = Updated {
                name: save.draft.name.trim().into(),
                format: Some(save.format),
                cards: save.draft.main,
                sideboard: save.draft.side,
                commanders: save.draft.commanders,
            };
            let saved = versions.save(account, existing.as_deref(), edits, save.versioned)?;
            Ok(Reply::AutoSaved {
                deck_id: existing.is_none().then(|| saved.id.to_string()),
                archived: existing.is_some() && save.versioned,
            })
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_refresh_does_not_advance_a_private_editors_revision() {
        let id = Uuid::now_v7();
        let mut versions = Versions::default();
        assert_eq!(versions.expected(id), Err(Error::Conflict));
        versions.listed.insert(id, 1);
        assert_eq!(versions.expected(id), Ok(1));
        versions.edited.insert(id, 1);
        versions.listed.insert(id, 2);
        assert_eq!(versions.expected(id), Ok(1));
        assert!(super::id("system:starter").is_err());
        assert_eq!(super::id(&id.to_string()), Ok(id));
    }

    #[test]
    fn duplicating_an_own_central_deck_plans_an_account_copy_and_names_it() {
        let own = Uuid::now_v7();
        let mut state = LobbyState::default();
        state.lobby.apply(LobbyEvent::Decks(vec![
            baylee_client_core::lobby::DeckSummary {
                id: own.to_string(),
                name: "Burn".to_string(),
                ..Default::default()
            },
        ]));
        assert!(matches!(
            copy_source(&mut state, &own.to_string()),
            Ok(CopySource::Own { id, .. }) if id == own
        ));
        assert!(matches!(
            copy_source(&mut state, &Uuid::now_v7().to_string()),
            Err(Error::Conflict)
        ));

        let en_base = Phrase::ListCopyName.fill(Lang::En, &["Burn"]);
        assert_eq!(copy_name("Burn", &["Burn".to_string()], Lang::En), en_base);
        assert_eq!(
            copy_name("Burn", &["Burn".to_string(), en_base.clone()], Lang::En),
            format!("{en_base} (2)")
        );
        assert_eq!(
            copy_name("Burn", &[], Lang::De),
            Phrase::ListCopyName.fill(Lang::De, &["Burn"])
        );
        let long = "Ä".repeat(40);
        let fitted = copy_name(&long, &[], Lang::De);
        assert!(!fitted.is_empty() && fitted.len() <= MAX_DECK_NAME_BYTES);
    }
}
