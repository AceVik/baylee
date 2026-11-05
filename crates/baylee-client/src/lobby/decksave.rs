//! The deck builder's saves on their way to a gateway or to the offline
//! store.

use super::http::{bearer, json_body, json_post};
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

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

/// Whether leaving the builder saves what it holds: somewhere to save it,
/// a deck that may save, and no failed save the player has not answered.
/// Where it does not, Back asks first.
pub(super) fn leaving_saves(state: &mut LobbyState) -> bool {
    name_a_nameless_deck(state);
    state.saves_decks()
        && state.lobby.builder().saveable()
        && state.build.save != crate::buildui::SaveState::Failed
}

/// What a finished save says, as replies: the save's own, and whatever else
/// its answer means. Separate from the callback so it can be tested.
fn read_save(
    lang: Lang,
    saved_changes: u32,
    result: &ehttp::Result<ehttp::Response>,
) -> Vec<Reply> {
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
    let response = match result {
        Ok(response) => response,
        Err(_) => {
            return vec![failed(Some(
                Phrase::GatewayUnreachable.text(lang).to_string(),
            ))];
        }
    };
    if response.ok {
        let Saved { deck_id, archived } = response
            .text()
            .and_then(|text| serde_json::from_str(text).ok())
            .unwrap_or_default();
        return vec![Reply::AutoSaved {
            deck_id,
            saved_changes,
            archived,
        }];
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

/// Sends one save, its answer coming back through the mailbox.
fn send_save(state: &LobbyState, mailbox: &Mailbox, request: ehttp::Request, saved_changes: u32) {
    let lang = state.lobby.lang();
    let epoch = state.gateway_epoch;
    let box_ = Arc::clone(&mailbox.0);
    crate::transport::fetch(request, move |result| {
        let replies = read_save(lang, saved_changes, &result);
        if let Ok(mut box_) = box_.lock() {
            for reply in replies {
                box_.push(Reply::Remote(epoch, Box::new(reply)));
            }
        }
    });
}

/// A save did not reach the gateway: the deck stays unsaved, its draft stays
/// on the device, and the player is told. `why` is `None` where another
/// reply says what happened.
pub(super) fn save_failed(state: &mut LobbyState, why: Option<String>) {
    state.build.save = crate::buildui::SaveState::Failed;
    if let Some(why) = why {
        let failed = Phrase::BuildSaveFailed.text(state.lobby.lang());
        state.lobby.say(format!("{failed}: {why}"));
    }
}

/// Immediately saves the deck builder's current state. The deck stays dirty
/// until the gateway has answered; a deck that may not save is not sent,
/// since `rows` leaves out a card the pool has not got.
///
/// Every visit is one entry in the deck's history, and the entry is written
/// by the save that first changes a list (`"versioned": true`), the ones
/// after it updating the deck where it stands. Written first and not on the
/// way out, so that nothing has to go right later: quit, sign out or discard
/// and what the deck held before this visit is already kept.
#[allow(clippy::too_many_lines)]
pub(crate) fn auto_save_deck(state: &mut LobbyState, mailbox: &Mailbox) {
    use baylee_client_core::deckbuilder::Zone;

    let lang = state.lobby.lang();
    name_a_nameless_deck(state);
    if !state.lobby.builder().saveable() {
        return;
    }
    let deck = state.lobby.builder();
    let name = deck.name().trim().to_string();
    let cards = deck.rows(Zone::Main);
    let sideboard = deck.rows(Zone::Side);
    let commanders = deck.commander_names();
    let format = Some(deck.format().to_string());
    let deck_id = deck.editing().map(str::to_owned);
    let saved_changes = deck.changes();

    if let Some(offline) = state.offline.as_mut() {
        let event = offline.perform(
            LobbyRequest::SaveDeck {
                deck_id,
                name,
                cards,
                sideboard,
                commanders,
                format,
            },
            lang,
        );
        if let LobbyEvent::DeckSaved { deck_id } = event {
            let editing = state.lobby.builder().editing().map(str::to_owned);
            state
                .lobby
                .builder_mut()
                .mark_auto_saved(deck_id.as_deref(), saved_changes, false);
            state.build.save = crate::buildui::SaveState::Saved {
                at: 0.0,
                minutes: 0,
            };
            let select_id = state.lobby.builder().editing().map(str::to_owned);
            if let Some(offline) = state.offline.as_mut()
                && let LobbyEvent::Decks(decks) = offline.perform(LobbyRequest::ListDecks, lang)
            {
                state
                    .lobby
                    .update_decks_in_place(decks, select_id.as_deref());
            }
            state.drafts.forget(&[editing.as_deref(), None]);
        }
        return;
    }
    if state.central_without_gateway() {
        let local = state.local_decks.get_or_insert_with(offline::Offline::load);
        let event = local.perform(
            LobbyRequest::SaveDeck {
                deck_id,
                name,
                cards,
                sideboard,
                commanders,
                format,
            },
            lang,
        );
        if let LobbyEvent::DeckSaved { deck_id } = event {
            let editing = state.lobby.builder().editing().map(str::to_owned);
            state
                .lobby
                .builder_mut()
                .mark_auto_saved(deck_id.as_deref(), saved_changes, false);
            state.build.save = crate::buildui::SaveState::Saved {
                at: 0.0,
                minutes: 0,
            };
            let select_id = state.lobby.builder().editing().map(str::to_owned);
            if let Some(local) = state.local_decks.as_mut()
                && let LobbyEvent::Decks(decks) = local.perform(LobbyRequest::ListDecks, lang)
            {
                state
                    .lobby
                    .update_decks_in_place(decks, select_id.as_deref());
            }
            state.drafts.forget(&[editing.as_deref(), None]);
        }
        return;
    }
    let Some(token) = state.lobby.token().map(str::to_owned) else {
        return;
    };
    let base = state.gateway.trim_end_matches('/').to_string();
    if base.is_empty() {
        return;
    }
    let mut body = serde_json::json!({
        "name": name,
        "cards": cards,
        "sideboard": sideboard,
        "commanders": commanders,
        "versioned": !state.lobby.builder().saves_in_place(),
    });
    if let Some(format) = format {
        body["format"] = serde_json::Value::String(format);
    }
    let request = match deck_id {
        Some(id) => json_body(ehttp::Method::PUT, &format!("{base}/decks/{id}"), &body),
        None => json_post(&format!("{base}/decks"), &body),
    };
    let request = bearer(request, Some(&token));
    send_save(state, mailbox, request, saved_changes);
}
