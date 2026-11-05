//! The deck builder's saves on their way to a gateway or to the device's own
//! store.
//!
//! Every edit saves the whole deck. A gateway's saves stand in one line
//! (`client_core::deckbuilder::saves`): one out at a time, later edits folded
//! into the next, so they land in the order they were made. The deck stays
//! dirty until an answer says the gateway holds what it holds now, and until
//! then its draft is the copy that counts (`crate::buildui::draft`).
//!
//! Every visit is one entry in the deck's history, written by the save that
//! first changes a list (`"versioned": true`); the saves after it update the
//! deck where it stands. Written first and not on the way out, so that
//! nothing has to go right later: quit, sign out or discard, and what the
//! deck held before this visit is already kept.

use super::http::{bearer, json_body, json_post};
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;
use crate::buildui::SaveState;
use baylee_client_core::deckbuilder::Draft;
use baylee_client_core::deckbuilder::saves::{Answered, DeckSave};

/// Gives a nameless deck that holds cards the default name: an auto-save
/// has nobody to ask.
fn name_a_nameless_deck(state: &mut LobbyState) {
    use baylee_client_core::deckbuilder::Zone;

    let deck = state.lobby.builder();
    if deck.name().trim().is_empty() && !deck.entries(Zone::Main).is_empty() {
        let default_name = Phrase::BuildUntitled.text(state.lobby.lang()).to_string();
        state.lobby.builder_mut().set_default_name(&default_name);
    }
}

/// Whether what the builder holds can be saved: somewhere to save it, and a
/// deck that may save.
pub(super) fn can_save(state: &mut LobbyState) -> bool {
    name_a_nameless_deck(state);
    state.saves_decks() && state.lobby.builder().saveable()
}

/// Whether leaving the builder saves what it holds: it can, and no failed
/// save stands unanswered. Where it does not, Back asks first.
pub(super) fn leaving_saves(state: &mut LobbyState) -> bool {
    can_save(state) && state.build.save != SaveState::Failed
}

/// What the player keeps per deck follows it to the id it was given.
pub(crate) fn carry_deck_prefs(prefs: &mut crate::prefs::Prefs, from: &str, to: &str) {
    if !prefs.all().deck_card_favourites_for(from).is_empty() {
        prefs.edit().migrate_deck_card_favourites(from, to);
    }
    if !prefs.all().variants_for_deck(from).is_empty() {
        prefs.edit().migrate_deck_variants(from, to);
    }
}

/// The HTTP call one save becomes.
fn save_request(base: &str, token: &str, save: &DeckSave) -> ehttp::Request {
    let base = base.trim_end_matches('/');
    let draft = &save.draft;
    let body = serde_json::json!({
        "name": draft.name.trim(),
        "cards": draft.main,
        "sideboard": draft.side,
        "commanders": draft.commanders,
        "format": save.format,
        "versioned": save.versioned,
    });
    let request = match &draft.editing {
        Some(id) => json_body(ehttp::Method::PUT, &format!("{base}/decks/{id}"), &body),
        None => json_post(&format!("{base}/decks"), &body),
    };
    bearer(request, Some(token))
}

/// What a finished save says, as replies: the save's own, and whatever else
/// its answer means.
fn read_save(lang: Lang, result: &ehttp::Result<ehttp::Response>) -> Vec<Reply> {
    /// `POST /decks` says `deck_id`, `PUT /decks/{id}` says `archived`, and
    /// a gateway from before that says nothing, which has to read as no: a
    /// save in place on a word that never came would lose a state.
    #[derive(Default, serde::Deserialize)]
    struct Saved {
        #[serde(default)]
        deck_id: Option<String>,
        #[serde(default)]
        archived: bool,
    }

    let failed = |why: Option<String>| Reply::AutoSaveFailed { why };
    let Ok(response) = result else {
        return vec![failed(Some(
            Phrase::GatewayUnreachable.text(lang).to_string(),
        ))];
    };
    if response.ok {
        let Saved { deck_id, archived } = response
            .text()
            .and_then(|text| serde_json::from_str(text).ok())
            .unwrap_or_default();
        return vec![Reply::AutoSaved { deck_id, archived }];
    }
    // The session's own endings are told by their own replies.
    if response.status == 401 {
        return vec![failed(None), Reply::Expired];
    }
    match client_core::lobby::account_refusal(response.status, &response.bytes) {
        Some(event) => vec![failed(None), Reply::Event(event)],
        None => vec![failed(Some(super::http::gateway_error(lang, response)))],
    }
}

/// Sends a save the line let out, its answer coming back through the
/// mailbox.
fn send_save(state: &LobbyState, mailbox: &Mailbox, save: &DeckSave) {
    let Some(token) = state.lobby.token() else {
        return;
    };
    let request = save_request(&state.gateway, token, save);
    let lang = state.lobby.lang();
    let epoch = state.gateway_epoch;
    let box_ = Arc::clone(&mailbox.0);
    crate::transport::fetch(request, move |result| {
        let replies = read_save(lang, &result);
        if let Ok(mut box_) = box_.lock() {
            for reply in replies {
                box_.push(Reply::Remote(epoch, Box::new(reply)));
            }
        }
    });
}

/// The gateway took the save that was out. An answer is about the deck it
/// was sent for: what the builder has opened since is not touched.
pub(super) fn save_landed(
    state: &mut LobbyState,
    prefs: &mut crate::prefs::Prefs,
    mailbox: &Mailbox,
    deck_id: Option<String>,
    archived: bool,
) {
    let Some(Answered {
        sent,
        next,
        thrown_away,
    }) = state.deck_saves.landed(deck_id.as_deref(), archived)
    else {
        return;
    };
    let mut asked = false;
    if thrown_away {
        if let Some(deck_id) = deck_id {
            dispatch(state, mailbox, Some(LobbyRequest::DeleteDeck { deck_id }));
        }
    } else {
        // What was marked on a deck before it had an id stands under "new".
        if let Some(id) = &deck_id {
            carry_deck_prefs(prefs, "new", id);
        }
        if state.lobby.builder().visit() == sent.visit {
            asked = landed_on_the_open_deck(state, &sent.draft, deck_id.as_deref(), archived);
        } else {
            state.drafts.forget_landed(&sent.draft);
        }
    }
    if let Some(next) = next {
        send_save(state, mailbox, &next);
    }
    if asked || !matches!(state.lobby.screen(), client_core::lobby::Screen::Build) {
        dispatch(state, mailbox, Some(LobbyRequest::ListDecks));
    }
}

/// [`save_landed`] for the deck the builder still holds, open or just left.
/// Answers whether the player had asked for this save.
fn landed_on_the_open_deck(
    state: &mut LobbyState,
    sent: &Draft,
    deck_id: Option<&str>,
    archived: bool,
) -> bool {
    let asked = state.build.save == SaveState::Saving;
    if let Some(id) = deck_id {
        state.lobby.remember_saved_deck(id);
        // The deck is drafted under its id from here on.
        state.drafts.forget(&[None]);
    }
    state
        .lobby
        .builder_mut()
        .mark_auto_saved(sent, deck_id, archived);
    let deck = state.lobby.builder();
    if deck.dirty() {
        return false;
    }
    let editing = deck.editing().map(str::to_owned);
    state.drafts.forget(&[editing.as_deref()]);
    // `follow_the_save` has the clock.
    state.build.save = SaveState::Saving;
    if asked {
        state.lobby.tell(Phrase::DeckSaved, &[]);
    }
    asked
}

/// The save that was out did not reach the gateway: the deck stays unsaved,
/// its draft stays on the device, and the player is told. `why` is `None`
/// where another reply says what happened.
pub(super) fn save_failed(state: &mut LobbyState, mailbox: &Mailbox, why: Option<String>) {
    let Some(Answered { sent, next, .. }) = state.deck_saves.failed() else {
        return;
    };
    if state.lobby.builder().visit() == sent.visit {
        state.build.save = SaveState::Failed;
    }
    if let Some(why) = why {
        let failed = Phrase::BuildSaveFailed.text(state.lobby.lang());
        state.lobby.say(format!("{failed}: {why}"));
    }
    if let Some(next) = next {
        send_save(state, mailbox, &next);
    }
}

/// The device's own store, where that is where decks are saved.
fn device_store(state: &mut LobbyState) -> Option<&mut offline::Offline> {
    if state.offline.is_some() {
        state.offline.as_mut()
    } else if state.central_without_gateway() {
        Some(state.local_decks.get_or_insert_with(offline::Offline::load))
    } else {
        None
    }
}

/// Saves into the device's store, in this call: there is nothing to wait for
/// and nothing that can land out of order.
fn save_on_the_device(state: &mut LobbyState, request: LobbyRequest) {
    let lang = state.lobby.lang();
    let Some(store) = device_store(state) else {
        return;
    };
    let LobbyEvent::DeckSaved { deck_id } = store.perform(request, lang) else {
        return;
    };
    let listed = store.perform(LobbyRequest::ListDecks, lang);
    let sent = state.lobby.builder().draft();
    state
        .lobby
        .builder_mut()
        .mark_auto_saved(&sent, deck_id.as_deref(), false);
    // `follow_the_save` has the clock.
    state.build.save = SaveState::Saving;
    if let LobbyEvent::Decks(decks) = listed {
        let select = state.lobby.builder().editing().map(str::to_owned);
        state.lobby.update_decks_in_place(decks, select.as_deref());
    }
    state.drafts.forget(&[sent.editing.as_deref()]);
}

/// Saves what the builder holds: at once on the device, through the line to
/// a gateway. A deck that may not save is not sent, since `rows` leaves out a
/// card the pool has not got.
pub(crate) fn auto_save_deck(state: &mut LobbyState, mailbox: &Mailbox) {
    name_a_nameless_deck(state);
    let deck = state.lobby.builder();
    let Some(request) = deck.save() else {
        return;
    };
    if state.offline.is_some() || state.central_without_gateway() {
        save_on_the_device(state, request);
        return;
    }
    if state.gateway.is_empty() || state.lobby.token().is_none() {
        return;
    }
    let save = DeckSave {
        visit: deck.visit(),
        draft: deck.draft(),
        format: deck.format().to_string(),
        versioned: !deck.saves_in_place(),
    };
    if let Some(now) = state.deck_saves.offer(save, state.gateway_epoch) {
        send_save(state, mailbox, &now);
    }
}
