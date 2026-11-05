//! The deck builder's saves on their way to a gateway or to the offline
//! store.

use super::http::{bearer, json_body, json_post};
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

/// Immediately saves the deck builder's current state without creating a new
/// version-history entry (`"versioned": false`).
#[allow(clippy::too_many_lines)]
pub(crate) fn auto_save_deck(state: &mut LobbyState, mailbox: &Mailbox) {
    use baylee_client_core::deckbuilder::Zone;

    let lang = state.lobby.lang();
    if state.lobby.builder().entries(Zone::Main).is_empty() {
        return;
    }
    if state.lobby.builder().name().trim().is_empty() {
        let default_name = Phrase::BuildUntitled.text(lang).to_string();
        state.lobby.builder_mut().set_default_name(&default_name);
    }
    let deck = state.lobby.builder();
    let name: String = deck.name().trim().chars().take(64).collect();
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
                .mark_auto_saved(deck_id.as_deref(), saved_changes);
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
            crate::buildui::draft::forget(&[editing.as_deref(), None]);
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
                .mark_auto_saved(deck_id.as_deref(), saved_changes);
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
            crate::buildui::draft::forget(&[editing.as_deref(), None]);
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
    if deck_id.is_some() {
        state
            .lobby
            .builder_mut()
            .mark_auto_saved(None, saved_changes);
    }
    let mut body = serde_json::json!({
        "name": name,
        "cards": cards,
        "sideboard": sideboard,
        "commanders": commanders,
        "versioned": false,
    });
    if let Some(format) = format {
        body["format"] = serde_json::Value::String(format);
    }
    let request = match deck_id {
        Some(id) => json_body(ehttp::Method::PUT, &format!("{base}/decks/{id}"), &body),
        None => json_post(&format!("{base}/decks"), &body),
    };
    let request = bearer(request, Some(&token));
    let epoch = state.gateway_epoch;
    let box_ = Arc::clone(&mailbox.0);
    crate::transport::fetch(request, move |result| {
        #[derive(serde::Deserialize)]
        struct SavedDeck {
            deck_id: String,
        }
        if let Ok(response) = result
            && response.ok
        {
            let deck_id = response
                .text()
                .and_then(|t| serde_json::from_str::<SavedDeck>(t).ok())
                .map(|d| d.deck_id);
            if let Ok(mut box_) = box_.lock() {
                box_.push(Reply::Remote(
                    epoch,
                    Box::new(Reply::AutoSaved {
                        deck_id,
                        saved_changes,
                    }),
                ));
            }
        }
    });
}

/// Commits all changes made during this deckbuilder session as a single
/// coalesced version-history entry (`"versioned": true`, `"superseded": baseline`)
/// when leaving the deckbuilder.
pub(crate) fn commit_session_version(state: &mut LobbyState, mailbox: &Mailbox) {
    use baylee_client_core::deckbuilder::Zone;

    let lang = state.lobby.lang();
    if state.offline.is_some() || state.central_without_gateway() {
        if state.lobby.builder().dirty() {
            auto_save_deck(state, mailbox);
        }
        let _ = state.lobby.builder_mut().take_session_baseline();
        return;
    }
    let Some(token) = state.lobby.token().map(str::to_owned) else {
        return;
    };
    let base = state.gateway.trim_end_matches('/').to_string();
    if base.is_empty() {
        return;
    }
    let is_new = state.lobby.builder().is_new_in_session();
    let changed = state.lobby.builder().session_changed();
    let baseline = state.lobby.builder_mut().take_session_baseline();
    if is_new || !changed {
        if state.lobby.builder().dirty() {
            auto_save_deck(state, mailbox);
        }
        return;
    }
    let Some(baseline) = baseline else {
        return;
    };
    let deck = state.lobby.builder();
    if deck.entries(Zone::Main).is_empty() {
        return;
    }
    let Some(id) = deck.editing().map(str::to_owned) else {
        return;
    };
    let name = if deck.name().trim().is_empty() {
        Phrase::BuildUntitled.text(lang).to_string()
    } else {
        deck.name().trim().chars().take(64).collect()
    };
    let cards = deck.rows(Zone::Main);
    let sideboard = deck.rows(Zone::Side);
    let commanders = deck.commander_names();
    let format = Some(deck.format().to_string());
    let saved_changes = deck.changes();
    state
        .lobby
        .builder_mut()
        .mark_auto_saved(None, saved_changes);
    let mut body = serde_json::json!({
        "name": name,
        "cards": cards,
        "sideboard": sideboard,
        "commanders": commanders,
        "versioned": true,
        "superseded": {
            "cards": baseline.cards,
            "sideboard": baseline.sideboard,
            "commanders": baseline.commanders,
        }
    });
    if let Some(format) = format {
        body["format"] = serde_json::Value::String(format);
    }
    let request = bearer(
        json_body(ehttp::Method::PUT, &format!("{base}/decks/{id}"), &body),
        Some(&token),
    );
    let epoch = state.gateway_epoch;
    let box_ = Arc::clone(&mailbox.0);
    crate::transport::fetch(request, move |result| {
        if let Ok(response) = result
            && response.ok
            && let Ok(mut box_) = box_.lock()
        {
            box_.push(Reply::Remote(
                epoch,
                Box::new(Reply::AutoSaved {
                    deck_id: None,
                    saved_changes,
                }),
            ));
        }
    });
}
