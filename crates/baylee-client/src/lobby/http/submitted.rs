//! Resolve an account deck on Central's serial worker. The gateway receives only
//! the play fields of the selected revision after its lobby is current again.
use super::{LobbyEvent, LobbyRequest, LobbyState, Mailbox, Phrase, Reply, build_with_deck, fetch};
use crate::lobby::central;
use baylee_account::decks::DeckDraft;
use baylee_client_core::lobby::gateway_list;
use uuid::Uuid;
pub(in crate::lobby) mod worn;

/// Work returned by Central and consumed on the lobby thread, before any
/// network request is signed or any card data leaves for a gateway.
pub(in crate::lobby) struct Prepared {
    epoch: u64,
    request: LobbyRequest,
    deck: Result<(serde_json::Value, baylee_account::advent::Equipment), Phrase>,
}

fn selected(request: &LobbyRequest) -> Option<&str> {
    let id = match request {
        LobbyRequest::CreateGame { deck_id, .. } | LobbyRequest::JoinGame { deck_id, .. } => {
            deck_id
        }
        LobbyRequest::SetSeat {
            deck_id: Some(deck_id),
            ..
        } => deck_id,
        _ => return None,
    };
    (!id.is_empty()).then_some(id)
}

fn play_fields(
    draft: DeckDraft,
    destination: &str,
) -> Result<serde_json::Value, baylee_account::Error> {
    let mut body = serde_json::json!({
        "name": draft.name,
        "cards": central::decks::play_rows(&draft.cards)?,
        "sideboard": central::decks::play_rows(&draft.sideboard)?,
        "commanders": central::decks::play_commanders(&draft.commanders)?,
    });
    if !draft.format.is_empty() {
        body["format"] = draft.format.into();
    }
    if draft
        .cosmetic_origin
        .as_deref()
        .is_some_and(|origin| gateway_list::same(origin, destination))
    {
        if let Some(sleeve) = draft.sleeve {
            body["sleeve"] = sleeve.into();
        }
        if let Some(playmat) = draft.playmat {
            body["playmat"] = playmat.into();
        }
    }
    Ok(body)
}

fn fail(state: &mut LobbyState, phrase: Phrase) {
    let message = phrase.text(state.lobby.lang()).to_owned();
    state.lobby.apply(LobbyEvent::Failed(message));
}

/// Return ordinary requests to the existing gateway performer.
pub(super) fn resolve(
    state: &mut LobbyState,
    mailbox: &Mailbox,
    request: LobbyRequest,
) -> Option<LobbyRequest> {
    let Some(id) = selected(&request).filter(|_| state.has_central_account()) else {
        return Some(request);
    };
    if state.submitted_decks_epoch != Some(state.gateway_epoch) || state.lobby.token().is_none() {
        fail(state, Phrase::GatewayDeckSubmissionUnavailable);
        return None;
    }
    let Some(id) = Uuid::try_parse(id)
        .ok()
        .filter(|id| id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122)
    else {
        fail(state, Phrase::LibraryReadFailed);
        return None;
    };
    let Some(handle) = state.central_handle() else {
        fail(state, Phrase::LibraryReadFailed);
        return None;
    };
    let Ok(revision) = handle.listed_deck_revision(id) else {
        fail(state, Phrase::LibraryReadFailed);
        return None;
    };
    let session = handle.clone();
    let mailbox = mailbox.clone();
    let epoch = state.gateway_epoch;
    let destination = state.gateway.clone();
    if !handle.submit(move |account| {
        let deck = account
            .deck_version(id, revision)
            .and_then(|snapshot| {
                if snapshot.id != id || snapshot.revision != revision {
                    return Err(baylee_account::Error::Rejected);
                }
                let look = snapshot.draft.look;
                play_fields(snapshot.draft, &destination).map(|deck| (deck, look))
            })
            .map_err(|_| Phrase::LibraryReadFailed);
        let prepared = Prepared {
            epoch,
            request,
            deck,
        };
        if let Ok(mut queue) = mailbox.0.lock() {
            queue.push(Reply::LibrarySession(
                session,
                Box::new(Reply::PlayDeck(Box::new(prepared))),
            ));
        }
    }) {
        fail(state, Phrase::LibraryReadFailed);
    }
    None
}

impl Prepared {
    /// The mailbox already checked the exact Central worker. Recheck the
    /// destination here before signing with its current bearer.
    pub(in crate::lobby) fn send(self, state: &mut LobbyState, mailbox: &Mailbox) {
        if self.epoch != state.gateway_epoch || !state.has_central_account() {
            return;
        }
        let Some(token) = state.lobby.token() else {
            return;
        };
        if state.submitted_decks_epoch != Some(self.epoch) {
            fail(state, Phrase::GatewayDeckSubmissionUnavailable);
            return;
        }
        let (deck, look) = match self.deck {
            Ok(deck) => deck,
            Err(why) => return fail(state, why),
        };
        let Some(account) = state.central_handle() else {
            return;
        };
        let selection = worn::Selection::new(account, look, &self.request);
        let (request, expect) = build_with_deck(
            &state.gateway,
            Some(token),
            &state.lang,
            self.request,
            Some(deck),
        );
        fetch(
            request,
            selection.map_or(expect, |look| super::Expect::Submitted(Box::new(look))),
            state.lobby.lang(),
            true,
            self.epoch,
            mailbox,
            None,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::lobby::GameMode;

    #[test]
    fn submitted_fields_are_scoped_to_the_selected_gateway() {
        let draft = DeckDraft {
            name: "Lantern".into(),
            format: "standard".into(),
            cards: vec!["1 Island # private plan".into()],
            sideboard: vec!["2 Mountain # secret boarding guide".into()],
            description: "private journal".into(),
            summary: Some("test build".into()),
            look: baylee_account::advent::Equipment {
                sleeve: Some(*Uuid::now_v7().as_bytes()),
                playmat: Some(*Uuid::now_v7().as_bytes()),
            },
            cosmetic_origin: Some("https://home.invalid".into()),
            sleeve: Some("home-only".into()),
            ..Default::default()
        };
        let fields = play_fields(draft.clone(), "https://elsewhere.invalid").unwrap();
        assert_eq!(
            fields,
            serde_json::json!({
                "name":"Lantern", "format":"standard", "cards":["1 Island"],
                "sideboard":["2 Mountain"], "commanders":[]
            })
        );
        let local = play_fields(draft, "https://home.invalid/").unwrap();
        assert_eq!(local["sleeve"], "home-only");
    }

    #[test]
    fn create_join_and_seating_replace_the_private_id_and_keep_room_fields() {
        let id = "central-private-deck";
        let create = LobbyRequest::CreateGame {
            deck_id: id.into(),
            mode: GameMode::Open,
            chairs: 2,
            name: String::new(),
            password: "room code".into(),
            clock: None,
            ai: None,
            spectators: false,
        };
        let join = LobbyRequest::JoinGame {
            game_id: "g1".into(),
            deck_id: id.into(),
            seat: Some(1),
            password: "room code".into(),
            invite: Some("invitation".into()),
        };
        let seat = LobbyRequest::SetSeat {
            game_id: "g1".into(),
            seat: 1,
            kind: None,
            ai: None,
            deck_id: Some(id.into()),
            team: Some(2),
        };
        let deck = serde_json::json!({"name":"Lantern","cards":["1 Island"]});
        for request in [create, join, seat] {
            assert_eq!(selected(&request), Some(id));
            let (wire, _) = build_with_deck(
                "https://home.invalid",
                None,
                "en",
                request.clone(),
                Some(deck.clone()),
            );
            let (legacy, _) = super::super::build("https://home.invalid", None, "en", request);
            let mut expected: serde_json::Value = serde_json::from_slice(&legacy.body).unwrap();
            assert_eq!(
                expected.as_object_mut().unwrap().remove("deck_id").unwrap(),
                id
            );
            expected["submitted_deck"] = deck.clone();
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&wire.body).unwrap(),
                expected
            );
            assert_eq!(wire.url, legacy.url);
        }
    }
}
