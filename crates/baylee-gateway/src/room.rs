//! Host-only, atomic edits of a waiting room.
use crate::{
    ErrorBody, Shared, auth, authed, err, err_saying, listing, lobby, routes::LobbyListing,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use baylee_core::preset::RoomUpdate;

/// The host's edit of a waiting room, checked whole before any of it is
/// made.
///
/// @route POST /lobby/games/{id}/configure
/// @scope public
/// @summary Change a waiting room's settings
/// @operation-id `configure_room`
/// @description The host sets the room's name, its number of chairs, its
/// password (`null` keeps it, empty removes it) and its shared rules
/// (starting life, free mulligans, each seat's own life and starting
/// permanents, the table format), all at once or not at all. A chair
/// someone sits in cannot be removed; a change to the rules or the chairs
/// takes back every chair's ready. Answers the whole listing as the caller
/// sees it.
/// @tag Lobby
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The room's game id"
/// @return 400: `$ErrorBody` "A setting out of bounds (life 1 to 999, free mulligans 0 to 7, 2 to 8 chairs, at most 32 starting permanents a seat), a name over 60 characters or with a control character, a password over 128 bytes, or a starting permanent that does not resolve"
/// @return 403: `$ErrorBody` "The account is banned (`ban`) or must accept the terms first (`terms_required`), or the caller is not the host (`only the host arranges seats`)"
/// @return 404: `$ErrorBody` "No such room"
/// @return 409: `$ErrorBody` "The game has started, or someone sits in a chair the edit removes"
/// @return 500: `$ErrorBody` "The house deck is missing from this build"
pub(crate) async fn configure(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<RoomUpdate>,
) -> Result<Json<LobbyListing>, (StatusCode, Json<ErrorBody>)> {
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
