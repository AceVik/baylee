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
    /// One server-issued create anchor per outstanding builder visit.
    create: Option<(u64, Uuid)>,
}

pub(super) fn new_epoch() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1 << 63);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

impl Handle {
    pub(crate) fn draft_store(&self) -> Option<crate::buildui::draft::private::Private> {
        self.draft_store.clone()
    }
    pub(crate) fn library_epoch(&self) -> u64 {
        self.library_epoch
    }

    /// AppExit is the one blocking deck operation. A deletion whose undo
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
        match lang {
            Lang::De => {
                "Das Deck wurde inzwischen geändert. Lade die aktuelle Version vor dem Speichern."
                    .into()
            }
            Lang::En => {
                "This deck changed on another device. Load its current version before saving."
                    .into()
            }
        }
    } else {
        Phrase::LibraryReadFailed.text(lang).into()
    }
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
            })
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
                let saved = versions.save(account, deck_id.as_deref(), draft, true, None)?;
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
            })
        }
        LobbyRequest::Library(LibraryRequest::Copy(id)) => {
            let source = shared::published(state, &id);
            submit(state, mailbox, Target::Library, move |account, versions| {
                let draft = source?.read()?;
                let saved = account.save_deck(None, None, draft)?;
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
        visit: Option<u64>,
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
        let anchor = if let (None, Some(visit)) = (deck, visit) {
            if self.create.is_none_or(|(key, _)| key != visit) {
                self.create = Some((visit, account.reserve_deck_id()?));
            }
            self.create.map(|(_, id)| id)
        } else {
            deck
        };
        let saved = account.save_deck_visit(anchor, expected, draft, archive)?;
        self.edited.insert(saved.id, saved.revision);
        self.listed.insert(saved.id, saved.revision);
        if let Some(visit) = visit
            && self.create.is_some_and(|(key, _)| key == visit)
        {
            self.create = None;
        }
        Ok(saved)
    }
}

pub(in crate::lobby) fn autosave(state: &LobbyState, mailbox: &Mailbox, save: &DeckSave) {
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
            let saved = versions.save(
                account,
                existing.as_deref(),
                edits,
                save.versioned,
                Some(save.visit),
            )?;
            Ok(Reply::AutoSaved {
                deck_id: existing.is_none().then(|| saved.id.to_string()),
                archived: existing.is_some() && save.versioned,
            })
        },
    );
}
