//! Account deck library: shared starters and append-only saved versions.
use super::{Lobby, LobbyRequest, Screen};
use baylee_core::deckdigest::Leader;
use serde::Deserialize;
use std::collections::BTreeMap;

/// A shared deck, readable but never edited by an ordinary account.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct HouseDeck {
    /// Server identity.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Format, as published by the server.
    #[serde(default)]
    pub format: String,
    /// Optional editorial description.
    #[serde(default)]
    pub description: String,
    /// Current saved version.
    pub version: i32,
    /// Number of main-deck rows (not card copies).
    pub cards: usize,
    /// Number of sideboard rows.
    pub sideboard: usize,
    /// Commander names.
    #[serde(default)]
    pub commanders: Vec<String>,
    /// Cards counting copies, as `GET /decks` says it; zero from a gateway
    /// that does not say.
    #[serde(default)]
    pub copies: u32,
    /// The colour identity as `WUBRG` letters.
    #[serde(default)]
    pub identity: String,
    /// The commanders' printings, each with its artist.
    #[serde(default)]
    pub leaders: Vec<Leader>,
    /// A deck without commanders: its picture.
    #[serde(default)]
    pub signature: Option<Leader>,
    /// Main-deck copies this build cannot play.
    #[serde(default)]
    pub unplayable: u32,
}

impl HouseDeck {
    /// What pictures it: the first commander, else the signature.
    #[must_use]
    pub fn picture(&self) -> Option<&Leader> {
        self.leaders.first().or(self.signature.as_ref())
    }

    /// The art its tile shows; `None` without a credited artist.
    #[must_use]
    pub fn art(&self) -> Option<super::shelf::DeckArt> {
        self.picture()
            .and_then(|p| super::shelf::art_at(&crate::images::art_base(), p))
    }
}

/// What happens once a house deck is copied.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AfterCopy {
    /// Open the copy in the builder (the builder's own path).
    #[default]
    Edit,
    /// Keep it on the shelf and stay where the press was (**Add**).
    Keep,
    /// Keep it and make it the next game's deck (**Add and use**, and the
    /// first-run mini-tiles; S-3).
    Use,
}

/// A superseded save, newest first in the server response.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Revision {
    /// Saved version number.
    pub version: i32,
    /// Save description, when available.
    pub summary: Option<String>,
    /// Time this version was superseded, Unix seconds.
    pub superseded_at: i64,
    /// Main-deck rows.
    pub cards: usize,
    /// Sideboard rows.
    pub sideboard: usize,
}

/// The current head and all retained saves.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct History {
    /// Current saved version number.
    pub version: i32,
    /// Time the current version was saved, Unix seconds.
    pub updated_at: i64,
    /// Earlier versions.
    pub past: Vec<Revision>,
}

/// A read-only snapshot; selecting one never mutates the working deck.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Snapshot {
    /// Saved version number.
    pub version: i32,
    /// Main-deck rows, including printing and notes.
    pub cards: Vec<String>,
    /// Sideboard rows.
    pub sideboard: Vec<String>,
    /// Commander names.
    pub commanders: Vec<String>,
}

/// Which library page is visible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    /// Shared starting decks.
    House,
    /// History for a persisted account deck.
    History(String),
}

/// Read-only library state, separate from unsaved edits in the builder.
#[derive(Clone, Debug, Default)]
pub struct Library {
    pending: Option<Request>,
    /// Open page, if any.
    pub page: Option<Page>,
    /// Published house decks.
    pub house: Vec<HouseDeck>,
    /// History of the open account deck.
    pub history: Option<History>,
    /// Current saved version, retained as the comparison base while browsing older saves.
    pub current: Option<Snapshot>,
    /// Selected saved contents.
    pub preview: Option<(String, Snapshot)>,
    /// Whether an operation is in flight.
    pub loading: bool,
    /// Explicit second-click restore confirmation.
    pub confirm_restore: bool,
    /// The latest library failure, kept apart from background lobby polling.
    pub error: Option<String>,
    /// What the copy in flight is for.
    after_copy: AfterCopy,
    /// Whether the restore in flight came from the Decks screen's history
    /// sheet, which stays on the shelf, rather than from the builder.
    restore_stays: bool,
    /// The last restore made from the history sheet: the deck, and the
    /// version that was its head before, for the Undo toast.
    pub restored: Option<(String, i32)>,
}

/// Library API operations, carried intact through the HTTP response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// List shared decks.
    House,
    /// Read one account deck's save log.
    History(String),
    /// Read the full contents of one version.
    Version(String, i32),
    /// Create an account-owned copy.
    Copy(String),
    /// Restore a historical version as a new save.
    Restore(String, i32),
}

/// Responses carry their originating request so late reads can be ignored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    /// Shared deck catalog.
    House(Vec<HouseDeck>),
    /// Save log.
    History(String, History),
    /// Full snapshot.
    Version(String, Snapshot),
    /// A copy was saved under this new identity.
    Copied(String),
    /// Restore succeeded; reload the live deck.
    Restored(String),
    /// Failure without replacing the working deck.
    Failed(String),
}

impl Lobby {
    /// Read-only library model.
    #[must_use]
    pub const fn library(&self) -> &Library {
        &self.library
    }

    /// Browse the published starting decks.
    pub fn browse_house(&mut self) -> Option<LobbyRequest> {
        if !self.has_a_performer() || self.library.loading {
            return None;
        }
        self.library = Library {
            page: Some(Page::House),
            loading: true,
            ..Library::default()
        };
        Some(self.library_request(Request::House))
    }

    /// History belongs only to this account's saved decks.
    pub fn browse_history(&mut self) -> Option<LobbyRequest> {
        let id = self.builder.editing()?.to_string();
        self.browse_deck_history(&id)
    }

    /// Open history directly from an account deck in the collection.
    pub fn browse_deck_history(&mut self, id: &str) -> Option<LobbyRequest> {
        if self.token().is_none() || self.library.loading || self.busy {
            return None;
        }
        let id = id.to_string();
        if !self.decks.iter().any(|deck| deck.id == id) {
            return None;
        }
        self.library = Library {
            page: Some(Page::History(id.clone())),
            loading: true,
            ..Library::default()
        };
        Some(self.library_request(Request::History(id)))
    }

    /// The Undo of the last restore has run out: nothing is offered any more.
    pub fn forget_restore(&mut self) {
        self.library.restored = None;
    }

    /// Close the read-only page without touching the working deck.
    pub fn close_library(&mut self) {
        self.library = Library::default();
    }

    /// Closes whatever page the Decks screen left open as the builder
    /// opens on a deck: the house list (its House tab, or the first-run
    /// ask with no decks) or a deck's history sheet. The shell draws the
    /// builder's own history page over `Screen::Build` whenever a page is
    /// open, so a page left standing was the "old page" the owner saw on
    /// the way from the deck list into a deck (beta.6 review). The Undo of
    /// a restore made from the sheet survives it.
    pub(super) fn close_library_for_the_builder(&mut self) {
        if self.library.page.is_none() {
            return;
        }
        let restored = self.library.restored.take();
        self.close_library();
        self.library.restored = restored;
    }

    /// Inspect a house deck or a historical version without editing it.
    pub fn preview_version(&mut self, id: &str, version: i32) -> Option<LobbyRequest> {
        if self.library.loading || !self.has_a_performer() {
            return None;
        }
        let allowed = match &self.library.page {
            Some(Page::House) => self
                .library
                .house
                .iter()
                .any(|d| d.id == id && d.version == version),
            Some(Page::History(deck)) => {
                deck == id
                    && self.library.history.as_ref().is_some_and(|h| {
                        h.version == version || h.past.iter().any(|v| v.version == version)
                    })
            }
            None => false,
        };
        if !allowed {
            return None;
        }
        self.library.loading = true;
        self.library.error = None;
        self.library.confirm_restore = false;
        Some(self.library_request(Request::Version(id.to_string(), version)))
    }

    /// Make a private copy; shared originals have no edit/delete path.
    pub fn copy_house(&mut self, index: usize) -> Option<LobbyRequest> {
        self.copy_house_then(index, AfterCopy::Edit)
    }

    /// The same, with what follows the copy: the builder, nothing, or the
    /// next game (**Add**, **Add and use**, and the first-run mini-tiles
    /// that reuse them).
    pub fn copy_house_then(&mut self, index: usize, after: AfterCopy) -> Option<LobbyRequest> {
        if self.library.loading || !self.has_a_performer() || self.library.page != Some(Page::House)
        {
            return None;
        }
        let id = self.library.house.get(index)?.id.clone();
        self.library.loading = true;
        self.library.error = None;
        self.library.after_copy = after;
        Some(self.library_request(Request::Copy(id)))
    }

    /// Duplicates one of the account's own decks (the tile's `⋯`): the same
    /// `POST /decks/{id}/copy` a house deck takes, kept on the shelf.
    pub fn duplicate_deck(&mut self, index: usize) -> Option<LobbyRequest> {
        if self.library.loading || self.busy || self.token().is_none() {
            return None;
        }
        let id = self.decks.get(index)?.id.clone();
        self.library.loading = true;
        self.library.error = None;
        self.library.after_copy = AfterCopy::Keep;
        Some(self.library_request(Request::Copy(id)))
    }

    /// Restores the previewed version straight away, from the Decks
    /// screen's history sheet: no second click, because the Undo toast that
    /// follows is the safety (S-10), and the sheet stays on the shelf.
    pub fn restore_version(&mut self) -> Option<LobbyRequest> {
        if self.library.loading || self.busy || self.token().is_none() {
            return None;
        }
        let Some(Page::History(id)) = &self.library.page else {
            return None;
        };
        let (preview_id, snapshot) = self.library.preview.as_ref()?;
        let head = self.library.history.as_ref()?.version;
        if preview_id != id || head == snapshot.version {
            return None;
        }
        let (id, version) = (id.clone(), snapshot.version);
        self.library.loading = true;
        self.library.error = None;
        self.library.restore_stays = true;
        self.library.restored = Some((id.clone(), head));
        Some(self.library_request(Request::Restore(id, version)))
    }

    /// Undoes the last restore made from the history sheet: the head before
    /// it is restored in turn — a new version again, as every restore is.
    pub fn undo_restore(&mut self) -> Option<LobbyRequest> {
        if self.token().is_none() || self.library.loading {
            return None;
        }
        // Taken: the restore this sends is not itself offered for Undo.
        let (id, version) = self.library.restored.take()?;
        self.library = Library {
            page: Some(Page::History(id.clone())),
            loading: true,
            restore_stays: true,
            ..Library::default()
        };
        Some(self.library_request(Request::Restore(id, version)))
    }

    /// The first click asks explicitly; the second restores as a new save.
    pub fn restore_preview(&mut self) -> Option<LobbyRequest> {
        if self.library.loading || self.busy || self.token().is_none() {
            return None;
        }
        let Some(Page::History(id)) = &self.library.page else {
            return None;
        };
        let (preview_id, snapshot) = self.library.preview.as_ref()?;
        if preview_id != id || self.library.history.as_ref()?.version == snapshot.version {
            return None;
        }
        if !self.library.confirm_restore {
            self.library.confirm_restore = true;
            return None;
        }
        self.library.loading = true;
        self.library.error = None;
        Some(self.library_request(Request::Restore(id.clone(), snapshot.version)))
    }

    fn library_request(&mut self, request: Request) -> LobbyRequest {
        self.library.pending = Some(request.clone());
        LobbyRequest::Library(request)
    }

    pub(super) fn library_reply(&mut self, reply: Reply) -> Option<LobbyRequest> {
        // A closed page or signed-out account cannot be reopened by a late
        // reply. A duplicate opens no page and is answered all the same.
        let duplicate =
            self.library.page.is_none() && matches!(self.library.pending, Some(Request::Copy(_)));
        if (self.library.page.is_none() && !duplicate) || !self.has_a_performer() {
            return None;
        }
        let matches_request = match (&self.library.pending, &reply) {
            (Some(Request::House), Reply::House(_))
            | (Some(Request::Copy(_)), Reply::Copied(_))
            | (Some(_), Reply::Failed(_)) => true,
            (Some(Request::History(expected)), Reply::History(id, _))
            | (Some(Request::Restore(expected, _)), Reply::Restored(id)) => expected == id,
            (Some(Request::Version(expected, version)), Reply::Version(id, snapshot)) => {
                expected == id && *version == snapshot.version
            }
            _ => false,
        };
        if !matches_request {
            return None;
        }
        self.library.pending = None;
        self.library.loading = false;
        match reply {
            Reply::House(decks) if self.library.page == Some(Page::House) => {
                self.library.house = decks;
            }
            Reply::History(id, history) if self.library.page == Some(Page::History(id.clone())) => {
                let version = history.version;
                self.library.history = Some(history);
                return self.preview_version(&id, version);
            }
            Reply::Version(id, snapshot) => {
                if self
                    .library
                    .history
                    .as_ref()
                    .is_some_and(|h| h.version == snapshot.version)
                {
                    self.library.current = Some(snapshot.clone());
                }
                self.library.preview = Some((id, snapshot));
            }
            Reply::Copied(id) => {
                let after = std::mem::take(&mut self.library.after_copy);
                self.busy = true;
                match after {
                    AfterCopy::Edit => {
                        self.close_library();
                        self.screen = Screen::Build;
                        // Refresh membership before loading the copy, so
                        // history is immediately available.
                        self.copied_deck = Some(id);
                    }
                    // The shelf stays as it is; the list is read again with
                    // the copy on it.
                    AfterCopy::Keep => {}
                    AfterCopy::Use => self.use_once_listed = Some(id),
                }
                return Some(LobbyRequest::ListDecks);
            }
            Reply::Restored(id)
                if self.library.restore_stays
                    && self.library.page == Some(Page::History(id.clone())) =>
            {
                let restored = self.library.restored.take();
                self.close_library();
                self.library.restored = restored;
                self.busy = true;
                return Some(LobbyRequest::ListDecks);
            }
            Reply::Restored(id) if self.library.page == Some(Page::History(id.clone())) => {
                self.close_library();
                self.screen = Screen::Build;
                self.busy = true;
                return Some(LobbyRequest::LoadDeck { deck_id: id });
            }
            Reply::Failed(error) => {
                self.library.restored = None;
                self.library.error = Some(error);
            }
            _ => {}
        }
        None
    }
}

/// Quantity changes, grouped by zone; printing/note changes remain visible.
#[must_use]
pub fn row_changes(before: &[String], after: &[String]) -> Vec<(String, i64)> {
    let mut counts = BTreeMap::<String, i64>::new();
    for (rows, sign) in [(before, -1), (after, 1)] {
        for row in rows {
            let (quantity, name) = row
                .split_once(' ')
                .and_then(|(q, name)| q.parse::<i64>().ok().map(|q| (q, name)))
                .unwrap_or((1, row));
            *counts.entry(name.to_string()).or_default() += sign * quantity;
        }
    }
    counts
        .into_iter()
        .filter(|(_, delta)| *delta != 0)
        .collect()
}
