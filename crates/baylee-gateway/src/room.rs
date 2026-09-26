//! Host-only, atomic edits of a waiting room.
use crate::{ErrorBody, Shared, auth, authed, err, err_saying, listing, lobby};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use baylee_core::preset::RoomUpdate;

pub(crate) async fn configure(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<RoomUpdate>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account = authed(&state, &headers).await?;
    body.setup
        .validate(body.chairs)
        .map_err(|why| err_saying(StatusCode::BAD_REQUEST, why))?;
    if body.name.chars().count() > 60
        || body.name.chars().any(char::is_control)
        || body.password.as_ref().is_some_and(|p| p.len() > 128)
    {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "room name or password is too long",
        ));
    }
    {
        let mut lobby = state.lobby.lock();
        let game = lobby
            .games
            .get_mut(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        if !game.hosted_by(&account) {
            return Err(err(StatusCode::FORBIDDEN, "only the host arranges seats"));
        }
        if game.state != lobby::LobbyState::Waiting {
            return Err(err(StatusCode::CONFLICT, "game already started"));
        }
        if game
            .seats
            .iter()
            .skip(body.chairs)
            .any(|s| s.account_id.is_some())
        {
            return Err(err(StatusCode::CONFLICT, "someone is sitting there"));
        }
        // Resolve all cards on a bounded scratch preset before mutating the room.
        let house = crate::house_deck()?;
        let decks = vec![&house; body.chairs];
        let mut probe = baylee_cards::decks::preset_for_all(0, &decks);
        baylee_cards::decks::apply_room_setup(&mut probe, &body.setup)
            .map_err(|why| err_saying(StatusCode::BAD_REQUEST, why))?;
        let rules_changed = game.setup != body.setup || game.seats.len() != body.chairs;
        game.seats.truncate(body.chairs);
        while game.seats.len() < body.chairs {
            game.seats.push(lobby::LobbySeat::open(game.seats.len()));
        }
        game.name = body.name;
        game.setup = body.setup;
        if let Some(password) = body.password {
            game.password_hash = (!password.is_empty()).then(|| auth::token_hash(&password));
        }
        if rules_changed {
            for seat in &mut game.seats {
                seat.said_ready = false;
            }
        }
    }
    state.lobby_moved();
    Ok(Json(listing(&state, &account).await))
}
