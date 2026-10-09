//! Account deck library: shared starters and append-only saved versions.
use super::{Lobby, LobbyRequest, Screen};
pub use baylee_core::deckdiff::{Change, Delta, ZoneDiff};
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
    /// What changed against the version before it (WG-6); `None` on the
    /// oldest, and from a gateway older than WG-6 (the row then shows its
    /// time only).
    #[serde(default)]
    pub delta: Option<Delta>,
    /// Cards counting copies (WG-6); `None` from an older gateway.
    #[serde(default)]
    pub card_count: Option<CardCount>,
}

/// A version's cards counting copies, as `GET /decks/{id}/history` says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct CardCount {
    /// Main-deck copies.
    pub main: u32,
    /// Sideboard copies.
    pub side: u32,
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
    /// The head's change against the newest past version (WG-6).
    #[serde(default)]
    pub delta: Option<Delta>,
    /// The head's cards counting copies (WG-6).
    #[serde(default)]
    pub card_count: Option<CardCount>,
}

impl History {
    /// Every version, the head first, then the past newest first.
    #[must_use]
    pub fn versions(&self) -> Vec<i32> {
        let mut all: Vec<i32> = std::iter::once(self.version)
            .chain(self.past.iter().map(|v| v.version))
            .collect();
        all.sort_unstable_by(|a, b| b.cmp(a));
        all.dedup();
        all
    }

    /// The version saved just before `version`, if the history keeps one.
    #[must_use]
    pub fn before(&self, version: i32) -> Option<i32> {
        self.versions().into_iter().find(|v| *v < version)
    }

    /// When `version` began: its predecessor's `superseded_at`; `None` for
    /// the oldest kept, whose start the store does not know (`DESIGN` §C.0).
    #[must_use]
    pub fn started(&self, version: i32) -> Option<i64> {
        let before = self.before(version)?;
        self.past
            .iter()
            .find(|v| v.version == before)
            .map(|v| v.superseded_at)
    }

    /// What `version` changed against the one before it, as the gateway
    /// counted it (`None` from a gateway older than WG-6).
    #[must_use]
    pub fn delta(&self, version: i32) -> Option<Delta> {
        if version == self.version {
            return self.delta;
        }
        self.past
            .iter()
            .find(|v| v.version == version)
            .and_then(|v| v.delta)
    }

    /// What made `version` (a revert, a precon's source build), as the
    /// store says it.
    ///
    /// The store keeps that sentence on the row the save *replaced*
    /// (`put_deck`: the old lists go to `deck_version` with the new save's
    /// summary), so `version`'s words are on its predecessor's row. Read off
    /// `version`'s own row, every restore was named one version early and
    /// the head never named.
    #[must_use]
    pub fn summary(&self, version: i32) -> Option<&str> {
        let before = self.before(version)?;
        self.past
            .iter()
            .find(|v| v.version == before)
            .and_then(|v| v.summary.as_deref())
    }
}

/// What the selected version is compared with (`DESIGN` §C.3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Compare {
    /// The version before it: what this save changed (the default; TOURS
    /// D14).
    #[default]
    Previous,
    /// The deck as it is now.
    Current,
    /// A second version picked from the list; `None` while the list waits
    /// for that pick.
    Pick(Option<i32>),
}

/// The classified changes between two versions, by zone: main deck,
/// sideboard, commanders (`baylee_core::deckdiff`, the classifier the
/// gateway's WG-6 counts with).
#[must_use]
pub fn compare_snapshots(before: &Snapshot, after: &Snapshot) -> [ZoneDiff; 3] {
    [
        baylee_core::deckdiff::diff_zone(&before.cards, &after.cards),
        baylee_core::deckdiff::diff_zone(&before.sideboard, &after.sideboard),
        baylee_core::deckdiff::diff_zone(&before.commanders, &after.commanders),
    ]
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
    /// The versions of the open history read so far, by number.
    pub snapshots: BTreeMap<i32, Snapshot>,
    /// The version selected in the history's list.
    pub selected: Option<i32>,
    /// What the selected version is compared with.
    pub compare: Compare,
    /// A house deck's contents, for its preview sheet.
    pub preview: Option<(String, Snapshot)>,
    /// Whether an operation is in flight.
    pub loading: bool,
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
        // A house deck's history is read-only (Q-C4); its list is kept, so
        // the sheet knows it for one and closing returns to it.
        let house = self.library.house.iter().any(|deck| deck.id == id);
        if !house && !self.decks.iter().any(|deck| deck.id == id) {
            return None;
        }
        self.library = Library {
            page: Some(Page::History(id.clone())),
            loading: true,
            house: std::mem::take(&mut self.library.house),
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
        // A house deck's history was opened over the House tab's list:
        // closing it stands that list up again, as it was.
        let back_to_house =
            matches!(&self.library.page, Some(Page::History(id)) if self.house_deck(id));
        let house = std::mem::take(&mut self.library.house);
        self.library = Library::default();
        if back_to_house {
            self.library.page = Some(Page::House);
            self.library.house = house;
        }
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
    ///
    /// On a history this selects the version (or, while the compare bar
    /// waits for a pick, picks the other end) and reads whichever of the
    /// two compared versions is not read yet; the selection moves even
    /// while a read is out, and the read follows when it lands.
    pub fn preview_version(&mut self, id: &str, version: i32) -> Option<LobbyRequest> {
        if !self.has_a_performer() {
            return None;
        }
        match &self.library.page {
            Some(Page::House) => {
                if self.library.loading
                    || !self
                        .library
                        .house
                        .iter()
                        .any(|d| d.id == id && d.version == version)
                {
                    return None;
                }
                self.library.loading = true;
                self.library.error = None;
                Some(self.library_request(Request::Version(id.to_string(), version)))
            }
            Some(Page::History(deck)) => {
                let known = deck == id
                    && self
                        .library
                        .history
                        .as_ref()
                        .is_some_and(|h| h.versions().contains(&version));
                if !known {
                    return None;
                }
                if self.library.compare == Compare::Pick(None) {
                    self.library.compare = Compare::Pick(Some(version));
                } else {
                    self.library.selected = Some(version);
                }
                self.read_compared()
            }
            None => None,
        }
    }

    /// The selected version moved by `by` rows in the list (head first):
    /// `↑↓` on the versions.
    pub fn step_version(&mut self, by: i32) -> Option<LobbyRequest> {
        let Some(Page::History(id)) = self.library.page.clone() else {
            return None;
        };
        let versions = self.library.history.as_ref()?.versions();
        let at = self
            .library
            .selected
            .and_then(|v| versions.iter().position(|x| *x == v))
            .unwrap_or(0);
        let last = versions.len().checked_sub(1)?;
        let to = usize::try_from(i64::try_from(at).ok()? + i64::from(by))
            .unwrap_or(0)
            .min(last);
        let version = versions[to];
        if self.library.compare == Compare::Pick(None) {
            // The list's arrows move the selection, never the pick.
            self.library.selected = Some(version);
            return self.read_compared();
        }
        self.preview_version(&id, version)
    }

    /// Chooses what the selected version is compared with. A second
    /// **Pick…** while the list waits leaves the mode.
    pub fn compare_with(&mut self, compare: Compare) -> Option<LobbyRequest> {
        self.library.compare = match (self.library.compare, compare) {
            (Compare::Pick(None), Compare::Pick(_)) => Compare::Previous,
            (_, Compare::Pick(_)) => Compare::Pick(None),
            (_, other) => other,
        };
        self.read_compared()
    }

    /// `Esc` on the history: leaves the Pick… mode (back to Previous).
    /// `false` when there was no mode to leave, so the press closes the
    /// sheet instead (one thing per press).
    pub fn leave_pick(&mut self) -> bool {
        if matches!(self.library.compare, Compare::Pick(_)) {
            self.library.compare = Compare::Previous;
            return true;
        }
        false
    }

    /// The two compared versions, older end first: `(None, v)` when the
    /// selected version has nothing before it (the oldest kept).
    #[must_use]
    pub fn compared(&self) -> Option<(Option<i32>, i32)> {
        let lib = &self.library;
        let history = lib.history.as_ref()?;
        let selected = lib.selected?;
        Some(match lib.compare {
            Compare::Previous | Compare::Pick(None) => (history.before(selected), selected),
            Compare::Current => (Some(selected), history.version),
            Compare::Pick(Some(other)) => (Some(selected.min(other)), selected.max(other)),
        })
    }

    /// The classified changes between the two compared versions, once
    /// both are read; the oldest version against itself (no changes).
    #[must_use]
    pub fn compared_changes(&self) -> Option<[ZoneDiff; 3]> {
        let (from, to) = self.compared()?;
        let after = self.library.snapshots.get(&to)?;
        let before = match from {
            Some(from) => self.library.snapshots.get(&from)?,
            None => after,
        };
        Some(compare_snapshots(before, after))
    }

    /// Reads the first compared version not read yet, one at a time.
    fn read_compared(&mut self) -> Option<LobbyRequest> {
        if self.library.loading {
            return None;
        }
        let Some(Page::History(id)) = self.library.page.clone() else {
            return None;
        };
        let (from, to) = self.compared()?;
        let missing = [Some(to), from]
            .into_iter()
            .flatten()
            .find(|v| !self.library.snapshots.contains_key(v))?;
        self.library.loading = true;
        self.library.error = None;
        Some(self.library_request(Request::Version(id, missing)))
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

    /// Restores the selected version straight away: no second click,
    /// because the Undo toast that follows is the safety (S-10). From the
    /// Decks screen the sheet stays on the shelf; from the builder the
    /// restored deck is loaded into it (the builder asks its own "Discard
    /// changes?" first when it has unsaved edits).
    pub fn restore_version(&mut self) -> Option<LobbyRequest> {
        if self.library.loading || self.busy || self.token().is_none() {
            return None;
        }
        let Some(Page::History(id)) = &self.library.page else {
            return None;
        };
        let version = self.library.selected?;
        let head = self.library.history.as_ref()?.version;
        if head == version || self.house_deck(id) {
            return None;
        }
        let id = id.clone();
        self.library.loading = true;
        self.library.error = None;
        self.library.restore_stays = self.screen != Screen::Build;
        // The builder's restore loads the deck into it: its Undo is the
        // builder's own (the restored deck is a new version either way).
        if self.library.restore_stays {
            self.library.restored = Some((id.clone(), head));
        }
        Some(self.library_request(Request::Restore(id, version)))
    }

    /// Whether `id` is a house deck (its history is read-only, Q-C4).
    #[must_use]
    pub fn house_deck(&self, id: &str) -> bool {
        !self.decks.iter().any(|d| d.id == id) && self.library.house.iter().any(|d| d.id == id)
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
                self.library.selected = Some(version);
                return self.read_compared();
            }
            Reply::Version(id, snapshot) => {
                if matches!(self.library.page, Some(Page::History(_))) {
                    self.library.snapshots.insert(snapshot.version, snapshot);
                    // The other end, or a selection that moved while
                    // this read was out.
                    return self.read_compared();
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
