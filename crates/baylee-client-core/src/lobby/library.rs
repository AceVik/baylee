//! Account deck library: shared starters and append-only saved versions.
use super::{Lobby, LobbyRequest, Screen};
use serde::Deserialize;
use std::collections::BTreeMap;

/// A shared deck, readable but never edited by an ordinary account.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
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
    /// Selected saved contents.
    pub preview: Option<(String, Snapshot)>,
    /// Whether an operation is in flight.
    pub loading: bool,
    /// Explicit second-click restore confirmation.
    pub confirm_restore: bool,
    /// The latest library failure, kept apart from background lobby polling.
    pub error: Option<String>,
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
        if self.token().is_none() || self.library.loading {
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
        if self.token().is_none() || self.library.loading || self.busy {
            return None;
        }
        let id = self.builder.editing()?.to_string();
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

    /// Close the read-only page without touching the working deck.
    pub fn close_library(&mut self) {
        self.library = Library::default();
    }

    /// Inspect a house deck or a historical version without editing it.
    pub fn preview_version(&mut self, id: &str, version: i32) -> Option<LobbyRequest> {
        if self.library.loading || self.token().is_none() {
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
        if self.library.loading || self.token().is_none() || self.library.page != Some(Page::House)
        {
            return None;
        }
        let id = self.library.house.get(index)?.id.clone();
        self.library.loading = true;
        self.library.error = None;
        Some(self.library_request(Request::Copy(id)))
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
        // A closed page or signed-out account cannot be reopened by a late reply.
        if self.library.page.is_none() || self.token().is_none() {
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
            Reply::Version(id, snapshot) => self.library.preview = Some((id, snapshot)),
            Reply::Copied(id) => {
                self.close_library();
                self.screen = Screen::Build;
                self.busy = true;
                // Refresh membership before loading the copy, so history is immediately available.
                self.copied_deck = Some(id);
                return Some(LobbyRequest::ListDecks);
            }
            Reply::Restored(id) if self.library.page == Some(Page::History(id.clone())) => {
                self.close_library();
                self.busy = true;
                return Some(LobbyRequest::LoadDeck { deck_id: id });
            }
            Reply::Failed(error) => self.library.error = Some(error),
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
