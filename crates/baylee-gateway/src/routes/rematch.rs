//! Playing the same table again.

use crate::{
    Deck, ErrorBody, HeaderMap, Json, LobbyGame, LobbyState, Path, Shared, State, StatusCode, auth,
    authed, db_down, err, store, try_start,
};

/// Plays the same table again — or joins the room somebody already opened.
///
/// A rematch is not a new room the players have to find each other in a
/// second time. The first press builds one with the whole arrangement copied
/// — the chairs, the sides they play for, who was in them, which ones were
/// the AI and how hard, the name and the password — and every press after
/// that joins *that* room. Which is what the route being idempotent buys:
/// two people pressing at the same moment sit down at one table, not two.
///
/// Pressing it is also the ready statement. `said_ready` means "I want to
/// play", and there is nothing else a button that says *play again* could be
/// saying — a caller seated un-ready would be left on the waiting veil for a
/// game nobody had told the gateway to start.
///
/// So the room starts itself once every chair is ready. That is not the rule
/// taken out of `set_seat`: that one fired when a player picked a deck to
/// look at it, and this one fires when everyone at the table has asked for
/// another game. An AI chair is ready as soon as it is configured, so a solo
/// player is one press from the next game, which is the case that matters
/// most — the game they are asking to repeat was itself one tap.
pub(crate) async fn rematch(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let seat_token = auth::new_token();
    // Lobby, then store, then lobby again — never both at once, which is the
    // rule `seat_names` states and every path here keeps. Nothing in this
    // crate nests the two, so there is no order to nest them in, and the one
    // that wanted both is this: a rematch plays each deck as it stands *now*,
    // so the chairs' deck ids are read out of the finished table before the
    // store is asked what those decks say today. `None` where the caller
    // addressed the room instead: it exists already, and nothing about it is
    // being built from a deck.
    let played: Option<Vec<Option<String>>> = {
        let lobby = state.lobby.lock();
        let game = lobby
            .games
            .get(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        // Who sat there, not who knows the id: a game's id was in every
        // player's own listing while the table was waiting, so it is no
        // secret and cannot be what the check rests on.
        if !game
            .seats
            .iter()
            .any(|s| s.account_id.as_ref() == Some(&account_id))
        {
            return Err(err(StatusCode::FORBIDDEN, "you were not at that table"));
        }
        match game.state {
            LobbyState::Over => Some(
                game.seats
                    .iter()
                    .map(|s| s.deck.as_ref().map(|d| d.id.clone()))
                    .collect(),
            ),
            // The room itself. Both ends address the same rematch, because
            // the two people asking are looking at different things: the one
            // who just finished has the game's id on the screen in front of
            // them, and the one who went back to the lobby has the room's.
            LobbyState::Waiting if game.parent.is_some() => None,
            _ => return Err(err(StatusCode::CONFLICT, "that game is not over yet")),
        }
    };
    // One query for the whole table's decks rather than one per chair.
    let wanted: Vec<String> = played.iter().flatten().flatten().cloned().collect();
    let known = store::decks_by_id(&state.db, wanted)
        .await
        .map_err(|e| db_down(&e))?;
    let fresh: Vec<Option<Deck>> = played
        .iter()
        .flatten()
        .map(|deck| deck.as_deref().and_then(|d| known.get(d)).cloned())
        .collect();
    let (room_id, seat) = {
        let mut lobby = state.lobby.lock();
        // `played` is `None` when the caller addressed the rematch room
        // itself, which is then the room and there is nothing to open.
        let room_id = if played.is_none() {
            id.clone()
        } else {
            let opened = lobby
                .games
                .get(&id)
                .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?
                .rematch
                .clone();
            // A pointer whose room has been reaped is the same situation as
            // no pointer at all. Asked again here rather than above, because
            // two players pressing at once must not each open one.
            if let Some(open) = opened.filter(|open| lobby.games.contains_key(open)) {
                open
            } else {
                let room_id = auth::new_id();
                let mut room = {
                    let over = lobby
                        .games
                        .get(&id)
                        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
                    LobbyGame::rematch_of(over, room_id.clone(), auth::now_secs())
                };
                // Seat for seat with what was read above: a finished game's
                // chairs do not move, every route that rearranges them
                // wanting a room that is still waiting. A deck deleted in
                // between leaves its chair the copy the table played.
                for (chair, fresh) in room.seats.iter_mut().zip(&fresh) {
                    if let Some(deck) = fresh {
                        chair.deck_name.clone_from(&deck.name);
                        chair.deck = Some(deck.clone());
                    }
                }
                lobby.games.insert(room_id.clone(), room);
                if let Some(over) = lobby.games.get_mut(&id) {
                    over.rematch = Some(room_id.clone());
                }
                room_id
            }
        };
        let room = lobby
            .games
            .get_mut(&room_id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        if room.state != LobbyState::Waiting {
            return Err(err(StatusCode::CONFLICT, "the rematch has already started"));
        }
        let chair = room
            .seats
            .iter_mut()
            .find(|s| s.account_id.as_ref() == Some(&account_id))
            .ok_or_else(|| err(StatusCode::FORBIDDEN, "you were not at that table"))?;
        chair.seat_token_hash = Some(auth::token_hash(&seat_token));
        chair.said_ready = true;
        (room_id, chair.seat)
    };
    // Outside both locks, for the reason `start_room` gives: ordering an
    // engine must not happen underneath the lobby lock. A room that is not
    // ready yet stays waiting and the ticket is still good — whoever presses
    // last is the one who starts it.
    try_start(&state, &room_id)?;
    state.lobby_moved();
    Ok(Json(serde_json::json!({
        "game_id": room_id,
        "seat": seat,
        "seat_token": seat_token,
    })))
}
