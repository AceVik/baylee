//! The lobby's rooms: opening one, sitting down, getting ready, starting,
//! handing the host on, and leaving.

use crate::{
    Deserialize, ErrorBody, HeaderMap, Json, LobbyGame, LobbyState, MAX_SEATS, Path, Shared, State,
    StatusCode, ai_preset, auth, authed, chair, clock, engine, err, err_saying, listing, lobby,
    own_deck, table_prints, try_start,
};

#[derive(Deserialize)]
pub(crate) struct CreateGameBody {
    #[serde(default)]
    deck_id: String,
    /// `"ai"` is the one-tap game against the house AI and starts at once.
    /// Anything else opens a room the host configures.
    #[serde(default)]
    mode: String,
    /// How many chairs the table has. Two unless the host says otherwise.
    #[serde(default)]
    seats: Option<usize>,
    /// What the table is called in the list.
    #[serde(default)]
    name: String,
    /// A password for the room. Empty or absent leaves it open.
    #[serde(default)]
    password: String,
    /// Which clock this table plays at, by name. See `clock::PRESETS`.
    #[serde(default)]
    clock: Option<String>,
    /// Seconds to answer one question, overriding whatever `clock` gave.
    #[serde(default)]
    decision_timeout_secs: Option<u32>,
    /// Seconds a seat may be gone before the house answers for it, same.
    #[serde(default)]
    reconnect_window_secs: Option<u32>,
    /// The house AI's difficulty for `mode: "ai"`, by its name
    /// (`AIProfile::NAMED`); `steady` when absent, refused when unknown.
    #[serde(default)]
    ai: Option<String>,
    /// Whether players who hold no chair may watch (`docs/protocol.md`
    /// §"Spectators"). On when absent.
    #[serde(default)]
    allow_spectators: Option<bool>,
}

pub(crate) async fn create_game(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<CreateGameBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let chosen = if body.deck_id.is_empty() {
        None
    } else {
        Some(own_deck(&state, &account_id, &body.deck_id).await?)
    };
    let game_id = auth::new_id();
    let seat_token = auth::new_token();
    // Before anything is built, because a room refused for its clock should
    // cost nothing and leave nothing behind.
    let house_rules = clock::resolve(
        body.clock.as_deref(),
        body.decision_timeout_secs,
        body.reconnect_window_secs,
    )
    .map_err(|reason| err_saying(StatusCode::BAD_REQUEST, reason))?;

    // The one-tap game against the house AI keeps its own path: it is a whole
    // table decided in one request, and nobody is going to configure it.
    if body.mode == "ai" {
        let (deck_name, deck) =
            chosen.ok_or_else(|| err(StatusCode::BAD_REQUEST, "pick a deck first"))?;
        // Which house: the one Play's difficulty caret named, else the one
        // every one-tap game has always been.
        let difficulty = body.ai.as_deref().unwrap_or("steady");
        let profile = baylee_core::preset::AIProfile::named(difficulty)
            .ok_or_else(|| err(StatusCode::BAD_REQUEST, "no such difficulty"))?;
        {
            let mut preset = ai_preset(&deck, auth::new_game_seed())?;
            preset.house_rules = house_rules.clone();
            preset.seats[0].controller = baylee_core::preset::SeatController::Open;
            preset.seats[1].controller = baylee_core::preset::SeatController::Ai(profile);
            state.art.warm(table_prints(&preset));
            let mut seats = vec![lobby::LobbySeat::open(0), lobby::LobbySeat::open(1)];
            seats[0].account_id = Some(account_id.clone());
            seats[0].seat_token_hash = Some(auth::token_hash(&seat_token));
            seats[0].deck_name = deck_name;
            seats[0].deck = Some(deck);
            seats[1].kind = lobby::SeatKind::Ai;
            seats[1].ai = Some(difficulty.to_string());
            seats[1].deck_name = "Victory".to_string();
            let mut game = LobbyGame::playing(game_id.clone(), seats, preset, auth::now_secs());
            game.house_rules = house_rules;
            game.allow_spectators = body.allow_spectators.unwrap_or(true);
            state.lobby.lock().games.insert(game_id.clone(), game);
        }
        if let Err(reason) = engine::start_engine(&state, &game_id) {
            state.lobby.lock().games.remove(&game_id);
            tracing::error!(game_id, reason, "could not start a game");
            return Err(err(StatusCode::SERVICE_UNAVAILABLE, "no engine available"));
        }
        state.lobby_moved();
        return Ok(Json(serde_json::json!({
            "game_id": game_id,
            "seat": 0,
            "seat_token": seat_token,
        })));
    }

    let chairs = body.seats.unwrap_or(2);
    if !(2..=MAX_SEATS).contains(&chairs) {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "a table seats between two and eight",
        ));
    }
    {
        let mut lobby = state.lobby.lock();
        let mut game = LobbyGame::blank(game_id.clone(), auth::now_secs());
        game.state = LobbyState::Waiting;
        game.host = Some(account_id.clone());
        game.name = body.name.chars().take(60).collect();
        game.seats = (0..chairs).map(lobby::LobbySeat::open).collect();
        let seq = game.claim_seq();
        game.seats[0].account_id = Some(account_id);
        game.seats[0].joined_seq = Some(seq);
        if let Some((name, deck)) = chosen {
            game.seats[0].deck_name = name;
            game.seats[0].deck = Some(deck);
        }
        game.seats[0].seat_token_hash = Some(auth::token_hash(&seat_token));
        game.house_rules = house_rules;
        game.allow_spectators = body.allow_spectators.unwrap_or(true);
        if !body.password.is_empty() {
            game.password_hash = Some(auth::token_hash(&body.password));
        }
        lobby.games.insert(game_id.clone(), game);
    }
    state.lobby_moved();
    Ok(Json(serde_json::json!({
        "game_id": game_id,
        "seat": 0,
        "seat_token": seat_token,
    })))
}

#[derive(Deserialize)]
pub(crate) struct JoinGameBody {
    #[serde(default)]
    deck_id: String,
    /// Which chair to take. The first free one when the body does not say.
    #[serde(default)]
    seat: Option<usize>,
    /// The room's password, for a room that has one.
    #[serde(default)]
    password: Option<String>,
}

pub(crate) async fn join_game(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<JoinGameBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let chosen = if body.deck_id.is_empty() {
        None
    } else {
        Some(own_deck(&state, &account_id, &body.deck_id).await?)
    };
    let seat_token = auth::new_token();
    let seat = {
        let mut lobby = state.lobby.lock();
        let game = lobby
            .games
            .get_mut(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        if game.state != LobbyState::Waiting {
            return Err(err(StatusCode::CONFLICT, "game already started"));
        }
        // Before anything else is checked, and answered the same way whether
        // the password was wrong or missing: a locked room should not tell a
        // stranger how full it is.
        if let Some(want) = &game.password_hash {
            let given = body.password.as_deref().unwrap_or_default();
            if &auth::token_hash(given) != want {
                return Err(err(StatusCode::FORBIDDEN, "wrong password"));
            }
        }
        if game
            .seats
            .iter()
            .any(|s| s.account_id.as_ref() == Some(&account_id))
        {
            return Err(err(StatusCode::CONFLICT, "you are already at this table"));
        }
        let seq = game.claim_seq();
        let free = |s: &&mut lobby::LobbySeat| s.kind == lobby::SeatKind::Human && !s.occupied();
        let chair = match body.seat {
            Some(at) => game
                .seats
                .iter_mut()
                .filter(free)
                .find(|s| s.seat == at)
                .ok_or_else(|| err(StatusCode::CONFLICT, "that seat is taken"))?,
            None => game
                .seats
                .iter_mut()
                .find(|s| free(s))
                .ok_or_else(|| err(StatusCode::CONFLICT, "the table is full"))?,
        };
        chair.account_id = Some(account_id);
        chair.seat_token_hash = Some(auth::token_hash(&seat_token));
        if let Some((name, deck)) = chosen {
            chair.deck_name = name;
            chair.deck = Some(deck);
        }
        chair.joined_seq = Some(seq);
        chair.seat
    };
    state.lobby_moved();
    Ok(Json(serde_json::json!({
        "game_id": id,
        "seat": seat,
        "seat_token": seat_token,
    })))
}

/// `POST /lobby/games/{id}/seat` — hand this account its chair back.
///
/// A seat token is issued once, at the join, and the gateway keeps only its
/// hash. A client that restarts has lost it for good, and every other route
/// answers the wrong question: `join` refuses ("you are already at this
/// table", and "game already started" once it is running), and the listing
/// shows the player their own table with no way into it. Everything else
/// about reconnecting is already built — the engine holds the chair open
/// (`Deadline::StandIn`, `Session::stand_in`, `SeatAttached`) and the client
/// knows how to re-dial (`reconnect.rs`, twelve attempts) — and all of it
/// hung on a secret only the dead process knew.
///
/// So this is deliberately *not* a join: no deck is chosen, no chair changes
/// hands, nothing another player can see moves. It asks one question — is
/// this account already sitting here — and a table that is **playing** is
/// exactly when the answer matters most.
///
/// Reissuing invalidates the old token, which is the right way round: the
/// chair's secret is whatever was handed out last, so a copy kept by some
/// older client cannot go on answering for a seat its owner has taken back.
pub(crate) async fn take_seat(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let seat_token = auth::new_token();
    let seat = {
        let mut lobby = state.lobby.lock();
        let game = lobby
            .games
            .get_mut(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        // A finished game has nothing left to answer for. The other two
        // states both do: before the start the seat screen waits for the
        // room, after it there is a game to catch up on.
        if game.state == LobbyState::Over {
            return Err(err(StatusCode::CONFLICT, "that game is over"));
        }
        let chair = game
            .seats
            .iter_mut()
            .find(|s| s.account_id.as_ref() == Some(&account_id))
            .ok_or_else(|| err(StatusCode::FORBIDDEN, "you are not at this table"))?;
        chair.seat_token_hash = Some(auth::token_hash(&seat_token));
        chair.seat
    };
    // No `lobby_moved()`: the arrangement of the table is exactly as it was.
    Ok(Json(serde_json::json!({
        "game_id": id,
        "seat": seat,
        "seat_token": seat_token,
    })))
}

#[derive(Deserialize)]
pub(crate) struct SeatBody {
    /// `"human"` or `"ai"`. Absent leaves the chair as it is.
    #[serde(default)]
    kind: Option<String>,
    /// Which difficulty an AI chair plays at.
    #[serde(default)]
    ai: Option<String>,
    /// The deck this chair plays.
    #[serde(default)]
    deck_id: Option<String>,
    /// Which team this chair plays for. Teams are numbered from 1, and `0`
    /// puts the chair back on its own side — a sentinel rather than a
    /// `null`, so "leave the team alone" stays the absent field it is for
    /// every other setting here.
    #[serde(default)]
    team: Option<u8>,
}

/// Configures one seat of a room.
///
/// Two authorities, deliberately narrow. The **host** arranges the table:
/// which chairs are people and which are the AI, how hard the AI plays, what
/// an AI chair brings, and which side each chair plays for. A **player**
/// changes exactly one thing, their own deck — including the host, whose own
/// chair is theirs as a player and not as the host.
///
/// Teams are the host's because they are the format, not a preference: a
/// player who could pick their own would pick the winning one, and a table
/// whose sides can change under the people at it is not the table they
/// agreed to sit at.
#[allow(clippy::too_many_lines)] // validate the complete edit before committing one chair
pub(crate) async fn set_seat(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path((id, seat)): Path<(String, usize)>,
    Json(body): Json<SeatBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    // Looked up before the lobby lock: the store has its own, and taking two
    // in one order here and the other order elsewhere is how deadlocks start.
    let chosen = match &body.deck_id {
        Some(deck_id) => Some(own_deck(&state, &account_id, deck_id).await?),
        None => None,
    };
    {
        let mut lobby = state.lobby.lock();
        let game = lobby
            .games
            .get_mut(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        if game.state != LobbyState::Waiting {
            return Err(err(StatusCode::CONFLICT, "game already started"));
        }
        let is_host = game.host.as_ref() == Some(&account_id);
        let mut chair = game
            .seats
            .get(seat)
            .cloned()
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such seat"))?;
        let is_mine = chair.account_id.as_ref() == Some(&account_id);
        if !is_mine && !is_host {
            return Err(err(StatusCode::FORBIDDEN, "not your seat"));
        }
        // The host arranges chairs, but never out from under someone else who
        // is sitting in one.
        if let Some(kind) = &body.kind {
            if !is_host {
                return Err(err(StatusCode::FORBIDDEN, "only the host arranges seats"));
            }
            if chair.account_id.is_some() && kind == "ai" {
                return Err(err(StatusCode::CONFLICT, "someone is sitting there"));
            }
            match kind.as_str() {
                "ai" => {
                    let deck = chair.deck.take();
                    let deck_name = std::mem::take(&mut chair.deck_name);
                    let profile = chair.ai.take();
                    chair.vacate();
                    chair.kind = lobby::SeatKind::Ai;
                    chair.ai = Some(profile.unwrap_or_else(|| "steady".to_string()));
                    chair.deck = deck;
                    chair.deck_name = if chair.deck.is_some() {
                        deck_name
                    } else {
                        "Victory".to_string()
                    };
                }
                "human" => {
                    // An AI's deck was the host's choice for a chair that is
                    // now waiting for a person, who brings their own.
                    if chair.account_id.is_none() {
                        chair.vacate();
                    } else {
                        chair.kind = lobby::SeatKind::Human;
                        chair.ai = None;
                    }
                }
                _ => return Err(err(StatusCode::BAD_REQUEST, "kind must be human or ai")),
            }
        }
        if let Some(profile) = &body.ai {
            if !is_host {
                return Err(err(StatusCode::FORBIDDEN, "only the host arranges seats"));
            }
            if baylee_core::preset::AIProfile::named(profile).is_none() {
                return Err(err(StatusCode::BAD_REQUEST, "no such AI"));
            }
            if chair.kind != lobby::SeatKind::Ai {
                return Err(err(StatusCode::CONFLICT, "that seat is not an AI"));
            }
            chair.ai = Some(profile.clone());
        }
        if let Some(team) = body.team {
            if !is_host {
                return Err(err(StatusCode::FORBIDDEN, "only the host arranges seats"));
            }
            if team > MAX_SEATS as u8 {
                return Err(err(StatusCode::BAD_REQUEST, "no such team"));
            }
            let moved = chair.team != (team > 0).then_some(team);
            chair.team = (team > 0).then_some(team);
            // Being moved to another side changes the game they said yes to,
            // exactly as a swapped deck does.
            if moved {
                chair.said_ready = false;
            }
        }
        if let Some((deck_name, deck)) = chosen {
            // A player sets their own deck; the host sets an AI's. Nobody
            // sets a deck for another person.
            let allowed = is_mine || (is_host && chair.kind == lobby::SeatKind::Ai);
            if !allowed {
                return Err(err(StatusCode::FORBIDDEN, "not your seat"));
            }
            chair.deck_name = deck_name;
            chair.deck = Some(deck);
            // A deck they have not seen yet is not a deck they said yes to.
            // Without this the host could swap an opponent into a game they
            // had already declared themselves ready for.
            chair.said_ready = false;
        }
        let old = &game.seats[seat];
        let changed = old.kind != chair.kind || old.ai != chair.ai || old.team != chair.team;
        game.seats[seat] = chair;
        if changed {
            for s in &mut game.seats {
                s.said_ready = false;
            }
        }
    }
    state.lobby_moved();
    Ok(Json(listing(&state, &account_id).await))
}

#[derive(Deserialize)]
pub(crate) struct ReadyBody {
    /// Absent means ready; a client withdraws by sending `false`.
    #[serde(default = "yes")]
    ready: bool,
}

/// `serde` default for [`ReadyBody::ready`].
pub(crate) const fn yes() -> bool {
    true
}

/// Says whether the caller is ready to play.
///
/// Only ever about the caller's own chair — the host arranges the table, but
/// nobody declares anyone else ready.
pub(crate) async fn set_ready(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<ReadyBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let rematch = {
        let mut lobby = state.lobby.lock();
        let game = lobby
            .games
            .get_mut(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        if game.state != LobbyState::Waiting {
            return Err(err(StatusCode::CONFLICT, "game already started"));
        }
        let is_rematch = game.parent.is_some();
        let chair = game
            .seats
            .iter_mut()
            .find(|s| s.account_id.as_ref() == Some(&account_id))
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "you are not at this table"))?;
        if body.ready && chair.deck.is_none() {
            return Err(err(StatusCode::CONFLICT, "pick a deck first"));
        }
        chair.said_ready = body.ready;
        is_rematch
    };
    // A rematch room starts itself, and the rule belongs to the *room* rather
    // than to the button that opened it. A chair there is only ready once its
    // player has claimed it through `/rematch`, so this route cannot be what
    // makes the table complete — but it can be what completes it *again*,
    // after someone withdrew and changed their mind, and then the host may
    // well be sitting on the waiting veil with no way to press start.
    //
    // Outside the lock, for the reason `start_room` gives, and unconditional
    // because `try_start` answers `Ok(false)` for a room that is not ready.
    if rematch {
        try_start(&state, &id)?;
    }
    state.lobby_moved();
    Ok(Json(listing(&state, &account_id).await))
}

/// Starts the room. The host's call, and only once every chair is ready.
pub(crate) async fn start_room(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    {
        let lobby = state.lobby.lock();
        let game = lobby
            .games
            .get(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        if !game.hosted_by(&account_id) {
            return Err(err(StatusCode::FORBIDDEN, "only the host starts the game"));
        }
        if game.state != LobbyState::Waiting {
            return Err(err(StatusCode::CONFLICT, "game already started"));
        }
        if !game.all_ready() {
            return Err(err(StatusCode::CONFLICT, "not everyone is ready"));
        }
    }
    // Outside the lock it took to check: `try_start` takes it again, and
    // ordering the engine must not happen underneath it.
    if !try_start(&state, &id)? {
        return Err(err(StatusCode::CONFLICT, "the room is no longer ready"));
    }
    state.lobby_moved();
    Ok(Json(listing(&state, &account_id).await))
}

#[derive(Deserialize)]
pub(crate) struct HostBody {
    /// The chair to hand the room to.
    seat: usize,
}

/// Hands the room to another player.
///
/// By seat rather than by name or account: the caller is looking at a
/// listing of chairs, and a seat index is the one handle in it that cannot
/// be ambiguous when two people share a display name.
pub(crate) async fn hand_over(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<HostBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    {
        let mut lobby = state.lobby.lock();
        let game = lobby
            .games
            .get_mut(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        if !game.hosted_by(&account_id) {
            return Err(err(StatusCode::FORBIDDEN, "not your room"));
        }
        if game.state != LobbyState::Waiting {
            return Err(err(StatusCode::CONFLICT, "game already started"));
        }
        let chair = game
            .seats
            .get(body.seat)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such seat"))?;
        let Some(new_host) = chair.account_id.clone() else {
            return Err(err(StatusCode::CONFLICT, "nobody is sitting there"));
        };
        game.host = Some(new_host);
    }
    // The chairs it handed out and nobody took yet were the host's to give.
    chair::revoke(&state, &id, &account_id);
    state.lobby_moved();
    Ok(Json(listing(&state, &account_id).await))
}

/// Gives up a seat, handing the room on if the host is the one leaving.
pub(crate) async fn leave_game(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let mut lobby = state.lobby.lock();
    let game = lobby
        .games
        .get_mut(&id)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
    if game.state != LobbyState::Waiting {
        return Err(err(StatusCode::CONFLICT, "game already started"));
    }
    let Some(chair) = game
        .seats
        .iter_mut()
        .find(|s| s.account_id.as_ref() == Some(&account_id))
    else {
        return Err(err(StatusCode::NOT_FOUND, "you are not at this table"));
    };
    chair.vacate();
    // A seat bridge it seated goes with it: nobody is left at the table to
    // answer for the chair (`chair.rs`).
    for seat in &mut game.seats {
        if seat.delegate.as_ref().is_some_and(|d| d.by == account_id) {
            seat.vacate();
        }
    }
    // A room outlives its host: it passes to whoever has been here longest,
    // and only a room with nobody left in it is closed. The earlier version
    // closed it the moment the host stood up, which threw everyone else out
    // of a table they were sitting at.
    if game.hosted_by(&account_id) && !game.hand_over_host() {
        game.finish(auth::now_secs());
    }
    drop(lobby);
    chair::revoke(&state, &id, &account_id);
    state.lobby_moved();
    Ok(StatusCode::NO_CONTENT)
}
