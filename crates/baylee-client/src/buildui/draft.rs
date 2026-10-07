//! The draft the builder keeps on the device while a deck is unsaved, and
//! the header's save state following the request it sent (§2.1, §2.5: "the
//! builder autosaves a local draft"; "Saving… → Saved ✓ · 2 min ago /
//! Couldn't save · Retry").
//!
//! The draft is the deck's rows in the stored form (`DeckBuilder::draft`),
//! one per deck being edited (`"new"` for a deck never saved), in one small
//! document beside the client's settings (`settings::store`; `localStorage`
//! in a browser). It is written a second after the last edit, forgotten
//! when the deck saves or the player discards it, and taken back when the
//! same deck is opened again with something the gateway never got.

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

/// Every kept draft, by the deck it is a draft of.
#[derive(Default, Serialize, Deserialize)]
struct Drafts {
    #[serde(default)]
    decks: BTreeMap<String, Draft>,
}

fn key(editing: Option<&str>) -> String {
    editing.map_or_else(|| "new".to_string(), str::to_owned)
}

fn read() -> Drafts {
    crate::settings::store::read_named(FILE)
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write(drafts: &Drafts) {
    if drafts.decks.is_empty() {
        crate::settings::store::remove_named(FILE);
    } else if let Ok(text) = serde_json::to_string(drafts) {
        crate::settings::store::write_named(FILE, &text);
    }
}

/// Forgets the drafts of `decks` (a saved deck, a discarded one).
pub(crate) fn forget(decks: &[Option<&str>]) {
    let mut drafts = read();
    let before = drafts.decks.len();
    for deck in decks {
        drafts.decks.remove(&key(*deck));
    }
    if drafts.decks.len() != before {
        write(&drafts);
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
}

fn now(time: Option<&Time<Real>>) -> f64 {
    time.map_or(0.0, Time::elapsed_secs_f64)
}

/// Keeps the draft: takes it back when its deck is opened again, writes it a
/// second after the last edit.
pub(crate) fn keep_the_draft(
    mut state: ResMut<LobbyState>,
    time: Option<Res<Time<Real>>>,
    mut kept: Local<Keeper>,
) {
    let on_screen = matches!(state.lobby.screen(), Screen::Build);
    if !on_screen {
        if kept.visiting {
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
        if state.lobby.builder().focus() == baylee_client_core::deckbuilder::BuildField::Name {
            state
                .lobby
                .builder_mut()
                .focus_on(baylee_client_core::deckbuilder::BuildField::Search);
        }
    }
    let deck = state.lobby.builder();
    if kept.restore && deck.loaded() && !state.lobby.busy() {
        kept.restore = false;
        let drafts = read();
        if let Some(draft) = drafts.decks.get(&key(deck.editing())).cloned()
            && !draft.is_empty()
            && deck.differs_from(&draft)
        {
            state.lobby.builder_mut().restore(&draft);
            state.lobby.tell_refusal(Phrase::BuildDraftRestored, &[]);
            kept.written = Some(draft);
        }
        return;
    }
    let deck = state.lobby.builder();
    if !deck.dirty() {
        kept.pending = None;
        return;
    }
    let at = now(time.as_deref());
    if state.is_changed() {
        let draft = deck.draft();
        if kept.written.as_ref() != Some(&draft)
            && kept.pending.as_ref().is_none_or(|(d, _)| *d != draft)
        {
            kept.pending = Some((draft, at));
        }
    }
    if let Some((draft, since)) = kept.pending.take() {
        if at - since >= SETTLE_SECS {
            let mut drafts = read();
            drafts
                .decks
                .insert(key(draft.editing.as_deref()), draft.clone());
            write(&drafts);
            kept.written = Some(draft);
        } else {
            kept.pending = Some((draft, since));
        }
    }
}

/// Follows the save the header's Save sent: Saved when the deck is no
/// longer dirty, Couldn't save when the lobby stopped waiting and it still
/// is; and the minutes since, once a minute.
pub(crate) fn follow_the_save(mut state: ResMut<LobbyState>, time: Option<Res<Time<Real>>>) {
    let at = now(time.as_deref());
    match state.build.save {
        SaveState::Saving => {
            let deck = state.lobby.builder();
            if !deck.dirty() {
                let editing = deck.editing().map(str::to_owned);
                state.build.save = SaveState::Saved { at, minutes: 0 };
                forget(&[editing.as_deref(), None]);
            } else if !state.lobby.busy() {
                state.build.save = SaveState::Failed;
            }
        }
        SaveState::Saved { at: since, minutes } => {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
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
