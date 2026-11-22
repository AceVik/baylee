//! The draft the builder keeps on the device while a deck is unsaved, and
//! the header's save state following the request it sent (§2.1, §2.5: "the
//! builder autosaves a local draft"; "Saving… → Saved ✓ · 2 min ago /
//! Couldn't save · Retry").
//!
//! The draft is the deck's rows in the stored form (`DeckBuilder::draft`),
//! one per deck being edited (`"new"` for a deck never saved), in one small
//! document beside the client's settings (`settings::store`; `localStorage`
//! in a browser). It is written a second after the last edit and at once on
//! leaving, forgotten when a save of it lands or the player discards it, and
//! taken back when the same deck is opened again with something the gateway
//! never got.

use super::SaveState;
use crate::lobby::LobbyState;
use baylee_client_core::deckbuilder::Draft;
use baylee_client_core::i18n::Phrase;
use baylee_client_core::lobby::Screen;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod private;

/// The document the drafts live in.
const FILE: &str = "builder-drafts.json";

/// How long the deck has to stand still before its draft is written.
const SETTLE_SECS: f64 = 1.0;

/// How long it has to stand still before it is saved: a word typed into the
/// name, or a burst of cards added, is one save.
const SAVE_SETTLE_SECS: f64 = 0.4;

/// Every kept draft, by the deck it is a draft of: the document, read the
/// first time it is asked for and written through from then on.
#[derive(Default, Serialize, Deserialize)]
pub(crate) struct Drafts {
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    private: Option<private::Private>,
    #[serde(skip)]
    epoch: u64,
    #[serde(skip)]
    unreadable: bool,
    /// Fingerprint of the clean builder state on which a private draft was
    /// based. Stored beside that draft inside its encrypted envelope.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    bases: BTreeMap<String, [u8; 32]>,
    /// Source recovery records stay intact until their independent copy lands.
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    forks: BTreeMap<u64, (String, [u8; 32])>,
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    creates: BTreeMap<uuid::Uuid, private::ReservedCreate>,
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    new_anchor: Option<uuid::Uuid>,
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    create_visits: BTreeMap<u64, uuid::Uuid>,
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    new_visit: Option<u64>,
    #[serde(default)]
    decks: BTreeMap<String, Draft>,
    /// Whether the document has been read.
    #[serde(skip)]
    read: bool,
    /// Which visit already offered its final draft when Back was pressed.
    #[serde(skip)]
    offered_on_back: Option<u64>,
}

fn key(editing: Option<&str>) -> String {
    editing.map_or_else(|| "new".to_string(), str::to_owned)
}

impl Drafts {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn bind(&mut self, epoch: u64, private: Option<private::Private>) {
        if self.epoch != epoch {
            *self = Self {
                epoch,
                private,
                ..Self::default()
            };
        }
    }

    fn decks(&mut self) -> &mut BTreeMap<String, Draft> {
        if !self.read {
            self.read = true;
            #[cfg(not(target_arch = "wasm32"))]
            if self.epoch != 0 {
                if let Some(private) = &self.private {
                    match private.read() {
                        Ok(kept) => {
                            self.decks = kept.decks;
                            self.bases = kept.bases;
                            self.creates = kept.creates;
                            self.new_anchor = kept.new_anchor;
                        }
                        Err(()) => self.unreadable = true,
                    }
                }
                return &mut self.decks;
            }
            let kept: Option<Self> = crate::settings::store::read_named(FILE)
                .and_then(|text| serde_json::from_str(&text).ok());
            if let Some(kept) = kept {
                self.decks = kept.decks;
            }
        }
        &mut self.decks
    }

    fn write(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.epoch != 0 {
            if !self.unreadable
                && let Some(private) = &self.private
            {
                private.write(self);
            }
            return;
        }
        if self.decks.is_empty() {
            crate::settings::store::remove_named(FILE);
        } else if let Ok(text) = serde_json::to_string(self) {
            crate::settings::store::write_named(FILE, &text);
        }
    }

    /// The draft kept of a deck.
    pub(crate) fn of(&mut self, editing: Option<&str>) -> Option<&Draft> {
        self.decks().get(&key(editing))
    }

    /// Keeps `draft` as its deck's.
    pub(crate) fn keep(&mut self, draft: &Draft) {
        self.decks()
            .insert(key(draft.editing.as_deref()), draft.clone());
        self.write();
    }

    pub(crate) fn keep_visit(&mut self, visit: u64, draft: &Draft) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.epoch != 0 && draft.editing.is_none() {
            self.decks();
            self.new_visit = Some(visit);
            self.new_anchor = self.create_visits.get(&visit).copied();
        }
        self.keep(draft);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn create_for(&mut self, visit: u64) -> Option<private::ReservedCreate> {
        self.decks();
        self.create_visits
            .get(&visit)
            .and_then(|id| self.creates.get(id))
            .cloned()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn reserve(
        &mut self,
        visit: u64,
        registration: private::ReservedCreate,
    ) -> Result<(), baylee_account::Error> {
        self.decks();
        if self.unreadable {
            return Err(baylee_account::Error::Unavailable);
        }
        self.record_create(visit, registration);
        self.persist_create()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn record_create(&mut self, visit: u64, registration: private::ReservedCreate) {
        let id = registration.id;
        self.creates.insert(id, registration);
        self.create_visits.insert(visit, id);
        if self.new_visit == Some(visit) {
            self.new_anchor = Some(id);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn persist_create(&self) -> Result<(), baylee_account::Error> {
        if self.unreadable {
            return Err(baylee_account::Error::Unavailable);
        }
        self.private
            .as_ref()
            .ok_or(baylee_account::Error::Unavailable)?
            .write_checked(self)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn resume_create(&mut self, visit: u64) {
        self.new_visit = Some(visit);
        if let Some(id) = self.new_anchor
            && self.creates.contains_key(&id)
        {
            self.create_visits.insert(visit, id);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn bind_private_owner(state: &mut LobbyState) {
        if let Some(handle) = state
            .central_handle()
            .filter(|_| state.has_central_account())
        {
            state
                .drafts
                .bind(handle.library_epoch(), handle.draft_store());
        }
    }

    /// A recovered edit of an older head opens as a separate draft, so it
    /// cannot overwrite a save made on another device while this one was down.
    fn recover(&mut self, current: &Draft) -> Option<(Draft, bool)> {
        #[cfg(not(target_arch = "wasm32"))]
        if self.epoch != 0 && current.editing.is_none() && !current.is_empty() {
            // A populated import or clone is a distinct creation.
            return None;
        }
        let desired = self.of(current.editing.as_deref()).cloned();
        #[cfg(not(target_arch = "wasm32"))]
        if self.epoch != 0
            && let Some(id) = current.editing.as_ref()
        {
            let base = private::fingerprint(current);
            let changed = self.bases.get(id) != Some(&base);
            if !changed || desired.as_ref().is_none_or(|draft| draft == current) {
                self.bases.insert(id.clone(), base);
            }
            return desired.map(|mut draft| {
                let fork = changed && draft != *current;
                if fork {
                    draft.editing = None;
                }
                (draft, fork)
            });
        }
        desired.map(|draft| (draft, false))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn committed(&mut self, visit: u64, sent: &Draft, created: Option<&str>) {
        if self.epoch != 0
            && let Some(id) = created.or(sent.editing.as_deref())
        {
            let mut current = sent.clone();
            current.editing = Some(id.to_owned());
            self.bases
                .insert(id.to_owned(), private::fingerprint(&current));
            if let Some(anchor) = self.create_visits.remove(&visit) {
                self.creates.remove(&anchor);
                if self.new_anchor == Some(anchor) {
                    self.new_anchor = None;
                    if let Some(latest) = self.decks.get("new").filter(|d| *d != sent) {
                        let mut latest = latest.clone();
                        latest.editing = Some(id.to_owned());
                        self.decks.insert(id.to_owned(), latest);
                    }
                    self.decks.remove("new");
                }
                self.write();
            }
            if created.is_some()
                && let Some((old, fingerprint)) = self.forks.remove(&visit)
                && self
                    .decks
                    .get(&old)
                    .is_some_and(|d| private::fingerprint(d) == fingerprint)
            {
                self.decks.remove(&old);
                self.bases.remove(&old);
                self.write();
            }
        }
    }

    /// Forgets the drafts of `decks` (a saved deck, a discarded one).
    pub(crate) fn forget(&mut self, decks: &[Option<&str>]) {
        let before = self.decks().len();
        for deck in decks {
            self.decks.remove(&key(*deck));
            #[cfg(not(target_arch = "wasm32"))]
            if deck.is_none() {
                if let Some(anchor) = self.new_anchor.take() {
                    self.creates.remove(&anchor);
                    self.create_visits.retain(|_, id| *id != anchor);
                }
                self.new_visit = None;
            }
        }
        if self.decks.len() != before {
            self.write();
        }
    }

    /// Forgets `sent` wherever it stands, for a save that landed after its
    /// deck left the builder, and nothing else: the deck's place may hold a
    /// newer edit by now, and "new" the next new deck. A deck created on its
    /// way out left its draft under "new" and landed under its id.
    pub(crate) fn forget_landed(&mut self, _visit: u64, sent: &Draft) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.epoch != 0 && sent.editing.is_none() && self.new_visit != Some(_visit) {
            return;
        }
        let same = |kept: &Draft| {
            Draft {
                editing: sent.editing.clone(),
                ..kept.clone()
            } == *sent
        };
        let places = [key(sent.editing.as_deref()), key(None)];
        let before = self.decks().len();
        self.decks
            .retain(|place, kept| !(places.contains(place) && same(kept)));
        if self.decks.len() != before {
            self.write();
        }
    }
}

/// What [`keep_the_draft`] remembers between frames.
#[derive(Default)]
pub(crate) struct Keeper {
    epoch: u64,
    /// Whether the builder was on screen last frame.
    visiting: bool,
    /// A draft may be waiting to be taken back for the deck just opened.
    restore: bool,
    /// The draft as it stands, and since when.
    pending: Option<(Draft, f64)>,
    /// The draft last written.
    written: Option<Draft>,
    /// The draft last offered to `auto_save_deck`. A save that failed is not
    /// sent again by itself: that would loop at the frame rate on a deck the
    /// gateway refuses. The next edit or Save sends it.
    offered: Option<Draft>,
}

fn now(time: Option<&Time<Real>>) -> f64 {
    time.map_or(0.0, Time::elapsed_secs_f64)
}

/// Saves what the builder holds. The device's store gives a new deck its id
/// in the call, and what the player keeps per deck goes with it; a gateway's
/// answer brings the id later (`decksave::save_landed`).
fn save(
    state: &mut LobbyState,
    prefs: Option<&mut crate::prefs::Prefs>,
    mailbox: &crate::lobby::Mailbox,
) {
    let old_key = state.lobby.builder().editing().unwrap_or("new").to_owned();
    crate::lobby::decksave::auto_save_deck(state, mailbox);
    if let Some(prefs) = prefs
        && let Some(new_id) = state.lobby.builder().editing()
        && new_id != old_key
    {
        crate::lobby::decksave::carry_deck_prefs(prefs, &old_key, new_id);
    }
}

fn finish_visit(
    state: &mut LobbyState,
    prefs: Option<&mut crate::prefs::Prefs>,
    mailbox: Option<&crate::lobby::Mailbox>,
) {
    #[cfg(not(target_arch = "wasm32"))]
    Drafts::bind_private_owner(state);
    let deck = state.lobby.builder();
    if deck.dirty() {
        let draft = deck.draft();
        if !draft.is_empty() {
            let visit = state.lobby.builder().visit();
            state.drafts.keep_visit(visit, &draft);
        }
        if let Some(mailbox) = mailbox {
            save(state, prefs, mailbox);
        }
    }
}

/// Keeps and offers the last edit as part of accepting Back, while any save
/// already out still occupies its place in the queue.
pub(crate) fn save_on_leaving(
    state: &mut LobbyState,
    prefs: &mut crate::prefs::Prefs,
    mailbox: &crate::lobby::Mailbox,
) {
    finish_visit(state, Some(prefs), Some(mailbox));
    state.drafts.offered_on_back = Some(state.lobby.builder().visit());
}

/// Keeps the draft: takes it back when its deck is opened again, saves every
/// change once it has stood [`SAVE_SETTLE_SECS`], and writes the draft a
/// second after the last edit for as long as no save has landed.
pub(crate) fn keep_the_draft(
    mut state: ResMut<LobbyState>,
    mut prefs: Option<ResMut<crate::prefs::Prefs>>,
    mailbox: Option<Res<crate::lobby::Mailbox>>,
    time: Option<Res<Time<Real>>>,
    mut kept: Local<Keeper>,
) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let scope = state
            .central_handle()
            .filter(|_| state.has_central_account());
        let epoch = scope
            .as_ref()
            .map_or(0, crate::lobby::central::Handle::library_epoch);
        if state.drafts.epoch != epoch {
            state
                .bypass_change_detection()
                .drafts
                .bind(epoch, scope.and_then(|s| s.draft_store()));
        }
        if kept.epoch != epoch {
            *kept = Keeper {
                epoch,
                ..Keeper::default()
            };
        }
    }
    let on_screen = matches!(state.lobby.screen(), Screen::Build);
    if state.lobby.scoped_builder() {
        if kept.visiting {
            *kept = Keeper::default();
        }
        return;
    }
    if !on_screen {
        if kept.visiting {
            let visit = state.lobby.builder().visit();
            let offered = state
                .bypass_change_detection()
                .drafts
                .offered_on_back
                .take();
            if offered != Some(visit) && state.lobby.builder().dirty() {
                finish_visit(&mut state, prefs.as_deref_mut(), mailbox.as_deref());
            }
            *kept = Keeper::default();
        }
        return;
    }
    if !kept.visiting {
        kept.visiting = true;
        kept.restore = true;
        // A builder opens with the caret in the pool's search (KEYBOARD W4:
        // "builder with an empty deck, focus in pool search"); a new deck is
        // named with F2 or the pen.
        if !state.build.naming_new
            && state.lobby.builder().focus() == baylee_client_core::deckbuilder::BuildField::Name
        {
            state
                .lobby
                .builder_mut()
                .focus_on(baylee_client_core::deckbuilder::BuildField::Search);
        }
        state.build.naming_new = false;
    }
    let deck = state.lobby.builder();
    if kept.restore && deck.loaded() && !state.lobby.busy() {
        kept.restore = false;
        let clean = deck.draft();
        let draft = state.drafts.recover(&clean);
        if let Some((draft, fork)) = draft
            && !draft.is_empty()
            && state.lobby.builder().differs_from(&draft)
        {
            state.lobby.builder_mut().restore(&draft);
            #[cfg(not(target_arch = "wasm32"))]
            if !fork && draft.editing.is_none() {
                let visit = state.lobby.builder().visit();
                state.drafts.resume_create(visit);
            }
            #[cfg(not(target_arch = "wasm32"))]
            if fork && let Some(old) = clean.editing.as_deref() {
                let visit = state.lobby.builder().visit();
                if let Some(kept) = state.drafts.of(Some(old)) {
                    let fingerprint = private::fingerprint(kept);
                    state
                        .drafts
                        .forks
                        .insert(visit, (old.to_owned(), fingerprint));
                }
            }
            state.lobby.tell_refusal(
                if fork {
                    Phrase::PrivateDraftRecoveredCopy
                } else {
                    Phrase::BuildDraftRestored
                },
                &[],
            );
            kept.written = Some(draft);
        }
        return;
    }
    // Until the deck just opened has arrived, the builder still holds the
    // one before it: a draft written now would be the old deck's, and could
    // bring back one the player has just discarded.
    if kept.restore {
        return;
    }
    let deck = state.lobby.builder();
    if !deck.dirty() {
        kept.pending = None;
        kept.written = None;
        return;
    }
    let at = now(time.as_deref());
    if state.is_changed() || kept.pending.is_none() {
        let draft = deck.draft();
        if kept.pending.as_ref().is_none_or(|(d, _)| *d != draft) {
            kept.pending = Some((draft, at));
        }
    }
    let Keeper {
        pending,
        written,
        offered,
        ..
    } = &mut *kept;
    let Some((draft, since)) = pending.as_ref() else {
        return;
    };
    let since = *since;
    if at - since >= SETTLE_SECS && written.as_ref() != Some(draft) {
        let visit = state.lobby.builder().visit();
        state.drafts.keep_visit(visit, draft);
        *written = Some(draft.clone());
    }
    if at - since < SAVE_SETTLE_SECS || offered.as_ref() == Some(draft) || !state.saves_decks() {
        return;
    }
    let Some(mailbox) = mailbox.as_deref() else {
        return;
    };
    save(&mut state, prefs.as_deref_mut(), mailbox);
    // Saving can name a nameless deck.
    let holds = state.lobby.builder().draft();
    *offered = Some(holds.clone());
    *pending = Some((holds, since));
}

/// Follows the last save: Saved once the deck is no longer dirty, back to
/// nothing when an edit overtook it (that has a save of its own coming, and a
/// save that failed says so itself); and the minutes since, once a minute.
pub(crate) fn follow_the_save(mut state: ResMut<LobbyState>, time: Option<Res<Time<Real>>>) {
    // A minigame acknowledgement, rather than the library's HTTP save queue,
    // owns this pending state. Its adapter marks the exact accepted composition.
    if state.lobby.scoped_builder() && state.build.save == SaveState::Saving {
        return;
    }
    let at = now(time.as_deref());
    match state.build.save {
        SaveState::Saving => {
            if !state.lobby.builder().dirty() {
                state.build.save = SaveState::Saved { at, minutes: 0 };
            } else if !state.deck_save_out() {
                state.build.save = SaveState::Idle;
            }
        }
        SaveState::Saved { at: since, minutes } => {
            // minutes of a session
            let now_minutes = ((at - since).max(0.0) / 60.0) as u32;
            if now_minutes != minutes {
                state.build.save = SaveState::Saved {
                    at: since,
                    minutes: now_minutes,
                };
            }
        }
        SaveState::Idle | SaveState::Failed => {}
    }
}
