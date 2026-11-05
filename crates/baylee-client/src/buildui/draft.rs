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
    #[serde(default)]
    decks: BTreeMap<String, Draft>,
    /// Whether the document has been read.
    #[serde(skip)]
    read: bool,
}

fn key(editing: Option<&str>) -> String {
    editing.map_or_else(|| "new".to_string(), str::to_owned)
}

impl Drafts {
    fn decks(&mut self) -> &mut BTreeMap<String, Draft> {
        if !self.read {
            self.read = true;
            let kept: Option<Self> = crate::settings::store::read_named(FILE)
                .and_then(|text| serde_json::from_str(&text).ok());
            if let Some(kept) = kept {
                self.decks = kept.decks;
            }
        }
        &mut self.decks
    }

    fn write(&self) {
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

    /// Forgets the drafts of `decks` (a saved deck, a discarded one).
    pub(crate) fn forget(&mut self, decks: &[Option<&str>]) {
        let before = self.decks().len();
        for deck in decks {
            self.decks.remove(&key(*deck));
        }
        if self.decks.len() != before {
            self.write();
        }
    }
}

/// What [`keep_the_draft`] remembers between frames.
#[derive(Default)]
pub(crate) struct Keeper {
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
    let on_screen = matches!(state.lobby.screen(), Screen::Build);
    if !on_screen {
        if kept.visiting {
            // Whatever becomes of the save that goes out now, the device
            // holds the edit until the gateway has said it does.
            let deck = state.lobby.builder();
            if deck.dirty() {
                let draft = deck.draft();
                if !draft.is_empty() {
                    state.drafts.keep(&draft);
                }
            }
            if let Some(mailbox) = mailbox.as_deref()
                && state.lobby.builder().dirty()
            {
                crate::lobby::decksave::auto_save_deck(&mut state, mailbox);
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
        let editing = deck.editing().map(str::to_owned);
        let draft = state.drafts.of(editing.as_deref()).cloned();
        if let Some(draft) = draft
            && !draft.is_empty()
            && state.lobby.builder().differs_from(&draft)
        {
            state.lobby.builder_mut().restore(&draft);
            state.lobby.tell_refusal(Phrase::BuildDraftRestored, &[]);
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
        state.drafts.keep(draft);
        *written = Some(draft.clone());
    }
    if at - since < SAVE_SETTLE_SECS || offered.as_ref() == Some(draft) || !state.saves_decks() {
        return;
    }
    let Some(mailbox) = mailbox.as_deref() else {
        return;
    };
    let old_key = state.lobby.builder().editing().unwrap_or("new").to_owned();
    crate::lobby::decksave::auto_save_deck(&mut state, mailbox);
    // The device's store gives a deck its id in that call; a gateway's
    // answer brings it later.
    if let Some(prefs) = prefs.as_deref_mut()
        && let Some(new_id) = state.lobby.builder().editing()
        && new_id != old_key
    {
        crate::lobby::decksave::carry_deck_prefs(prefs, &old_key, new_id);
    }
    // Saving can name a nameless deck.
    let holds = state.lobby.builder().draft();
    *offered = Some(holds.clone());
    *pending = Some((holds, since));
}

/// Follows the last save: Saved once the deck is no longer dirty, back to
/// nothing when an edit overtook it (that has a save of its own coming, and a
/// save that failed says so itself); and the minutes since, once a minute.
pub(crate) fn follow_the_save(mut state: ResMut<LobbyState>, time: Option<Res<Time<Real>>>) {
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
