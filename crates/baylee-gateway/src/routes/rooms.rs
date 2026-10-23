//! The lobby's rooms: opening one, sitting down, getting ready, starting,
//! handing the host on, and leaving.

use crate::{
    Deserialize, ErrorBody, HeaderMap, Json, LobbyGame, LobbyState, MAX_SEATS, Path, Serialize,
    Shared, State, StatusCode, ai_preset, auth, authed, chair, clock, engine, err, err_saying,
    listing, lobby, own_deck, routes::LobbyListing, table_prints, try_start,
};

/// A chair handed to the caller: the game it is at, its number, and the
/// seat token that opens it.
///
/// @description The caller's chair: the game it is at, its seat, and the
/// seat token that proves it.
/// @openapi
#[derive(Serialize)]
pub(crate) struct SeatGrant {
    // In key order: the bytes the `json!` it replaced wrote (serde_json sorts keys).
    /// @description The game the chair is at (for a rematch, the new room's
    /// id, not the one in the path).
    pub(crate) game_id: String,
    /// @description The chair's seat number, counted from 0.
    pub(crate) seat: usize,
    /// @description The chair's secret, sent as `Authorization: Bearer` to
    /// buy a seat socket's ticket (`POST /ws-ticket`) and to read the
    /// table's cosmetics. Handed out once; a new one
    /// (`POST /lobby/games/{id}/seat`) ends the old.
    pub(crate) seat_token: String,
}

/// What `POST /lobby/games` opens.
///
/// @description A game against the house AI (`mode: ai`), or a room the
/// caller hosts.
/// @openapi
#[derive(Deserialize)]
pub(crate) struct CreateGameBody {
    /// @description One of the caller's decks (`GET /decks`). Required for
    /// `mode: ai`; a room may be opened without one, and its host picks one
    /// before the game starts.
    #[serde(default)]
    deck_id: String,
    /// @description `ai` is the one-tap game against the house AI, which
    /// starts at once; anything else (`open`) opens a room the host
    /// arranges.
    /// @internal
    /// `"ai"` is the one-tap game against the house AI and starts at once.
    /// Anything else opens a room the host configures.
    #[serde(default)]
    mode: String,
    /// @description How many chairs the room has, 2 to 8; 2 when absent.
    /// Not read for `mode: ai`.
    /// @internal
    /// How many chairs the table has. Two unless the host says otherwise.
    /// @openapi minimum: 2
    /// @openapi maximum: 8
    #[serde(default)]
    seats: Option<usize>,
    /// @description What the room is called in the list; the first 60
    /// characters are kept, and an empty name lists it under its host's.
    /// Not read for `mode: ai`.
    /// @internal
    /// What the table is called in the list.
    #[serde(default)]
    name: String,
    /// @description A password for the room; empty or absent leaves it
    /// open. Not read for `mode: ai`.
    /// @internal
    /// A password for the room. Empty or absent leaves it open.
    #[serde(default)]
    password: String,
    /// @description Which clock the table plays at, by name
    /// (`GET /auth/config` lists them); the first of them when absent.
    /// @internal
    /// Which clock this table plays at, by name. See `clock::PRESETS`.
    #[serde(default)]
    clock: Option<String>,
    /// @description Seconds a player has for each decision, overriding the
    /// clock's: 0 for no decision clock, else 10 to 3600.
    /// @internal
    /// Seconds to answer one question, overriding whatever `clock` gave.
    #[serde(default)]
    decision_timeout_secs: Option<u32>,
    /// @description Seconds a seat that lost its connection is held before
    /// the house answers for it, 10 to 3600; the gateway's own window
    /// (`BAYLEE_RECONNECT_SECS`, 180 by default) when absent.
    /// @internal
    /// Seconds a seat may be gone before the house answers for it, same.
    #[serde(default)]
    reconnect_window_secs: Option<u32>,
    /// @description The house AI's difficulty for `mode: ai`; `steady`
    /// when absent, and an unknown one is refused.
    /// @internal
    /// The house AI's difficulty for `mode: "ai"`, by its name
    /// (`AIProfile::NAMED`); `steady` when absent, refused when unknown.
    /// @openapi enum: [novice, casual, steady, sharp, expert, null]
    #[serde(default)]
    ai: Option<String>,
    /// @description Whether signed-in players who hold no chair may watch
    /// the game; on when absent.
    /// @internal
    /// Whether players who hold no chair may watch (`docs/protocol.md`
    /// §"Spectators"). On when absent.
    #[serde(default)]
    allow_spectators: Option<bool>,
}

/// @route POST /lobby/games
/// @scope public
/// @summary Open a room, or play the house AI
/// @description With `mode: ai`, a game against the house AI that starts
/// at once: its engine is ordered before the answer, so the seat socket
/// can be opened right away. Otherwise a waiting room of `seats` chairs
/// with the caller as its host in the first one. Either way the answer is
/// the caller's chair.
/// @tag Lobby
/// @security bearer
/// @insert `SessionRefusals`
/// @return 400: `$ErrorBody` "No deck for `mode: ai` (`pick a deck first`), an unknown difficulty or clock, a clock out of bounds, a chair count outside 2 to 8, or a deck that does not load"
/// @return 403: `$ErrorBody` "The account is banned (`ban`) or must accept the terms first (`terms_required`), or the deck is someone else's (`not your deck`)"
/// @return 404: `$ErrorBody` "No such deck"
/// @return 409: `$ErrorBody` "The deck holds a card blocked on this gateway, or the caller is still playing a running game"
/// @return 500: `$ErrorBody` "The house deck is missing from this build"
/// @return 503: `$ErrorBody` "The account store or Baylee Central is unavailable, the gateway's database is, or no engine could be ordered for `mode: ai` (no agent, or a deploy holds new games)"
pub(crate) async fn create_game(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<CreateGameBody>,
) -> Result<Json<SeatGrant>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let central_session = crate::central::seat_binding(&state, &headers).await?;
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
        state.reconnect_secs,
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
            let mut preset = ai_preset(&state.blocks, &deck, auth::new_game_seed())?;
            preset.house_rules = house_rules.clone();
            preset.seats[0].controller = baylee_core::preset::SeatController::Open;
            preset.seats[1].controller = baylee_core::preset::SeatController::Ai(profile);
            state.art.warm(table_prints(&preset));
            let mut seats = vec![lobby::LobbySeat::open(0), lobby::LobbySeat::open(1)];
            seats[0].account_id = Some(account_id.clone());
            seats[0].seat_token_hash = Some(auth::token_hash(&seat_token));
            seats[0].central_session.clone_from(&central_session);
            seats[0].deck_name = deck_name;
            seats[0].deck = Some(deck);
            seats[1].kind = lobby::SeatKind::Ai;
            seats[1].ai = Some(difficulty.to_string());
            seats[1].deck_name = "Victory".to_string();
            let mut game = LobbyGame::playing(game_id.clone(), seats, preset, auth::now_secs());
            game.house_rules = house_rules;
            game.allow_spectators = body.allow_spectators.unwrap_or(true);
            let mut lobby = state.lobby.lock();
            super::participation::account(&lobby, &account_id, None)?;
            lobby.games.insert(game_id.clone(), game);
        }
        if let Err(reason) = engine::start_engine(&state, &game_id) {
            state.lobby.lock().games.remove(&game_id);
            tracing::error!(game_id, reason, "could not start a game");
            return Err(err(
                StatusCode::SERVICE_UNAVAILABLE,
                super::tables::unavailable(reason),
            ));
        }
        state.lobby_moved();
        return Ok(Json(SeatGrant {
            game_id,
            seat: 0,
            seat_token,
        }));
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
        super::participation::account(&lobby, &account_id, None)?;
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
        game.seats[0].central_session = central_session;
        game.house_rules = house_rules;
        game.allow_spectators = body.allow_spectators.unwrap_or(true);
        if !body.password.is_empty() {
            game.password_hash = Some(auth::token_hash(&body.password));
        }
        lobby.games.insert(game_id.clone(), game);
    }
    state.lobby_moved();
    Ok(Json(SeatGrant {
        game_id,
        seat: 0,
        seat_token,
    }))
}

/// Sitting down at a waiting room.
///
/// @description Which chair to take, the deck to bring, and the room's
/// password or an invite to it.
/// @openapi
#[derive(Deserialize)]
pub(crate) struct JoinGameBody {
    /// @description One of the caller's decks (`GET /decks`); empty sits
    /// down without one, to be picked before saying ready.
    #[serde(default)]
    deck_id: String,
    /// @description Which chair to take; the first open person's chair
    /// when absent.
    /// @internal
    /// Which chair to take. The first free one when the body does not say.
    #[serde(default)]
    seat: Option<usize>,
    /// @description The room's password, for a room that has one.
    /// @internal
    /// The room's password, for a room that has one.
    #[serde(default)]
    password: Option<String>,
    /// @description A table invite (`POST /social/invites`) for the caller
    /// and this room: it stands in for the password and is spent by the
    /// join.
    /// @internal
    /// A table invite (`POST /social/invites`) for this caller and this
    /// room: it stands in for the password and is spent by the join.
    #[serde(default)]
    invite: Option<String>,
}

/// @route POST /lobby/games/{id}/join
/// @scope public
/// @summary Sit down at a waiting room
/// @description Takes a person's chair in a room that has not started: the
/// one named, or the first open one. A locked room wants its password (or
/// an invite) before its chairs are looked at, so a wrong password learns
/// nothing of how full it is. Answers the caller's chair.
/// @tag Lobby
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The room's game id"
/// @return 403: `$ErrorBody` "The account is banned (`ban`) or must accept the terms first (`terms_required`), the password is wrong or missing (`wrong password`), or the deck is someone else's (`not your deck`)"
/// @return 404: `$ErrorBody` "No such room (`no such game`) or deck (`no such deck`)"
/// @return 409: `$ErrorBody` "The game has started, the caller already sits there, the chair is taken, the table is full, the deck holds a card blocked here, or the caller is still playing a running game"
/// @return 503: `$ErrorBody` "The account store or Baylee Central is unavailable, or the gateway's database is"
pub(crate) async fn join_game(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<JoinGameBody>,
) -> Result<Json<SeatGrant>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let central_session = crate::central::seat_binding(&state, &headers).await?;
    let chosen = if body.deck_id.is_empty() {
        None
    } else {
        Some(own_deck(&state, &account_id, &body.deck_id).await?)
    };
    let seat_token = auth::new_token();
    let seat = {
        let mut lobby = state.lobby.lock();
        super::participation::account(&lobby, &account_id, None)?;
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
        let invited = body.invite.as_deref().is_some_and(|invite| {
            state
                .social
                .invites
                .lock()
                .fits(invite, &account_id, &id, std::time::Instant::now())
        });
        if let Some(want) = game.password_hash.as_ref().filter(|_| !invited) {
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
        chair.central_session = central_session;
        if let Some((name, deck)) = chosen {
            chair.deck_name = name;
            chair.deck = Some(deck);
        }
        chair.joined_seq = Some(seq);
        let seat = chair.seat;
        if invited && let Some(invite) = body.invite.as_deref() {
            let spent = state.social.invites.lock().spend(
                invite,
                chair.account_id.as_deref().unwrap_or_default(),
                &id,
                std::time::Instant::now(),
            );
            if let Some((_, tell)) = spent {
                for (to, frame) in tell {
                    state.social.push(&to, &frame);
                }
            }
        }
        seat
    };
    state.lobby_moved();
    Ok(Json(SeatGrant {
        game_id: id,
        seat,
        seat_token,
    }))
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
///
/// @route POST /lobby/games/{id}/seat
/// @scope public
/// @summary Take back the chair you are in
/// @description For a client that lost its seat token (a restart): hands
/// the caller a new token for the chair it already holds, in a waiting
/// room or a running game. Nothing else about the table changes, and the
/// old token stops working.
/// @tag Lobby
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The game's id"
/// @return 403: `$ErrorBody` "The account is banned (`ban`) or must accept the terms first (`terms_required`), or the caller has no chair there (`you are not at this table`)"
/// @return 404: `$ErrorBody` "No such game"
/// @return 409: `$ErrorBody` "The game is over, or the chair cannot be resumed (a rematch chair not yet claimed, or a participation that ended)"
pub(crate) async fn take_seat(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<SeatGrant>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let central_session = crate::central::seat_binding(&state, &headers).await?;
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
        if !game
            .seats
            .iter()
            .any(|seat| seat.account_id.as_ref() == Some(&account_id))
        {
            return Err(err(StatusCode::FORBIDDEN, "you are not at this table"));
        }
        let seat = game
            .resumable_seat(&account_id)
            .ok_or_else(|| err(StatusCode::CONFLICT, "that chair cannot be resumed"))?;
        let chair = game
            .seats
            .iter_mut()
            .find(|s| s.seat == seat)
            .ok_or_else(|| err(StatusCode::FORBIDDEN, "you are not at this table"))?;
        chair.seat_token_hash = Some(auth::token_hash(&seat_token));
        chair.central_session = central_session;
        chair.seat
    };
    // No `lobby_moved()`: the arrangement of the table is exactly as it was.
    Ok(Json(SeatGrant {
        game_id: id,
        seat,
        seat_token,
    }))
}

/// What `POST /lobby/games/{id}/seats/{seat}` changes; an absent field
/// leaves that setting alone.
///
/// @description What to change about one chair; an absent field leaves
/// that setting as it is. `kind`, `ai` and `team` are the host's; `deck_id`
/// is the chair's own player's (or the host's, for a chair nobody sits in).
/// @openapi
#[derive(Deserialize)]
pub(crate) struct SeatBody {
    /// @description Whether the chair is for a person or the house AI. The
    /// host's; a chair someone sits in cannot be made the AI's.
    /// @internal
    /// `"human"` or `"ai"`. Absent leaves the chair as it is.
    /// @openapi enum: [human, ai, null]
    #[serde(default)]
    kind: Option<String>,
    /// @description The difficulty an AI chair plays at. The host's.
    /// @internal
    /// Which difficulty an AI chair plays at.
    /// @openapi enum: [novice, casual, steady, sharp, expert, null]
    #[serde(default)]
    ai: Option<String>,
    /// @description The deck the chair plays, one of the caller's own
    /// (`GET /decks`). Changing it takes back that chair's ready.
    /// @internal
    /// The deck this chair plays.
    #[serde(default)]
    deck_id: Option<String>,
    /// @description The side the chair plays for: teams are numbered from
    /// 1, and `0` puts the chair back on a side of its own. The host's.
    /// @internal
    /// Which team this chair plays for. Teams are numbered from 1, and `0`
    /// puts the chair back on its own side — a sentinel rather than a
    /// `null`, so "leave the team alone" stays the absent field it is for
    /// every other setting here.
    /// @openapi maximum: 8
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
///
/// @route POST /lobby/games/{id}/seats/{seat}
/// @scope public
/// @summary Arrange a chair of a waiting room
/// @description The host arranges the table (which chairs are people and
/// which the AI, how hard it plays, which side each chair is on); every
/// player sets the deck of their own chair. A change to a chair's kind,
/// difficulty or side takes back every chair's ready; a new deck or side
/// takes back that chair's. Answers the whole listing as the caller sees
/// it.
/// @tag Lobby
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The room's game id"
/// @path-param seat: usize "The chair, counted from 0"
/// @return 400: `$ErrorBody` "A `kind` other than `human` or `ai`, an unknown difficulty (`no such AI`), or a team above 8"
/// @return 403: `$ErrorBody` "The account is banned (`ban`) or must accept the terms first (`terms_required`); the chair is not the caller's (`not your seat`), a setting is the host's (`only the host arranges seats`), or the deck is someone else's (`not your deck`)"
/// @return 404: `$ErrorBody` "No such room (`no such game`), chair (`no such seat`) or deck (`no such deck`)"
/// @return 409: `$ErrorBody` "The game has started, someone sits in the chair made the AI's, `ai` for a chair that is not the AI's, or a deck (the chosen one, or the house deck an AI chair gets) holds a card blocked here"
/// @return 500: `$ErrorBody` "The house deck is missing from this build"
/// @return 503: `$ErrorBody` "The account store or Baylee Central is unavailable, or the gateway's database is"
#[allow(clippy::too_many_lines)] // validate the complete edit before committing one chair
pub(crate) async fn set_seat(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path((id, seat)): Path<(String, usize)>,
    Json(body): Json<SeatBody>,
) -> Result<Json<LobbyListing>, (StatusCode, Json<ErrorBody>)> {
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
                        // The house deck is not seated while it holds a
                        // card blocked here (`blocks.rs`).
                        state.blocks.check_loaded(
                            &super::tables::house_deck()?,
                            time::OffsetDateTime::now_utc(),
                        )?;
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
            // A player sets their own deck. The host chooses for the house,
            // a delegated/hosted model, or an open chair before handover.
            // An account-backed chair (including a reservation) stays its
            // player's own: hosting never grants control of their deck.
            let allowed = is_mine || (is_host && chair.account_id.is_none());
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
    crate::seathost::stop_unseated(&state);
    state.lobby_moved();
    Ok(Json(listing(&state, &account_id).await))
}

/// What a player says of their own chair.
///
/// @description Whether the caller is ready to play.
/// @openapi
#[derive(Deserialize)]
pub(crate) struct ReadyBody {
    /// @description `true` (also when absent) says ready, `false` takes it
    /// back.
    /// @internal
    /// Absent means ready; a client withdraws by sending `false`.
    /// @openapi default: true
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
///
/// @route POST /lobby/games/{id}/ready
/// @scope public
/// @summary Say you are ready, or take it back
/// @description Only ever about the caller's own chair, which needs a deck
/// first. A rematch room starts itself once every chair is ready, so this
/// may be what starts it. Answers the whole listing as the caller sees it.
/// @tag Lobby
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The room's game id"
/// @return 400: `$ErrorBody` "A rematch room that would start, but a deck at it does not load or a starting card does not resolve"
/// @return 404: `$ErrorBody` "No such room (`no such game`), or the caller has no chair there (`you are not at this table`)"
/// @return 409: `$ErrorBody` "The game has started, the chair has no deck (`pick a deck first`), the deck holds a card blocked here, or (a rematch room starting) every chair is on one team or a player is still in another running game"
/// @return 500: `$ErrorBody` "A rematch room that would start, but the house deck is missing from this build"
/// @return 503: `$ErrorBody` "The account store or Baylee Central is unavailable, the card catalog's database is, or no engine could be ordered for a rematch room that would start"
pub(crate) async fn set_ready(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<ReadyBody>,
) -> Result<Json<LobbyListing>, (StatusCode, Json<ErrorBody>)> {
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
        // A card blocked since the deck was chosen (`blocks.rs`).
        if body.ready
            && let Some(deck) = &chair.deck
        {
            state
                .blocks
                .check_stored(deck, time::OffsetDateTime::now_utc())?;
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
        try_start(&state, &id).await?;
    }
    state.lobby_moved();
    Ok(Json(listing(&state, &account_id).await))
}

/// Starts the room. The host's call, and only once every chair is ready.
///
/// @route POST /lobby/games/{id}/start
/// @scope public
/// @summary Start a waiting room
/// @description The host's go, once every chair is ready (the listing's
/// `startable`). Every deck at the table is checked again, the game's
/// engine is ordered, and the room turns `playing`. Answers the whole
/// listing as the caller sees it.
/// @tag Lobby
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The room's game id"
/// @return 400: `$ErrorBody` "A deck at the table does not load, or a starting card or printing does not resolve"
/// @return 403: `$ErrorBody` "The account is banned (`ban`) or must accept the terms first (`terms_required`), or the caller is not the host (`only the host starts the game`)"
/// @return 404: `$ErrorBody` "No such room"
/// @return 409: `$ErrorBody` "The game has started, not every chair is ready, the room changed meanwhile (`the room is no longer ready`), every chair is on one team, a deck holds a card blocked here, or a player is still in another running game"
/// @return 500: `$ErrorBody` "The house deck is missing from this build"
/// @return 503: `$ErrorBody` "The account store or Baylee Central is unavailable, the card catalog's database is, or no engine could be ordered (no agent, or a deploy holds new games)"
pub(crate) async fn start_room(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<LobbyListing>, (StatusCode, Json<ErrorBody>)> {
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
    if !try_start(&state, &id).await? {
        return Err(err(StatusCode::CONFLICT, "the room is no longer ready"));
    }
    state.lobby_moved();
    Ok(Json(listing(&state, &account_id).await))
}

/// Whom the room is handed to.
///
/// @description The chair whose player becomes the room's host.
/// @openapi
#[derive(Deserialize)]
pub(crate) struct HostBody {
    /// @description The chair to hand the room to, counted from 0.
    /// @internal
    /// The chair to hand the room to.
    seat: usize,
}

/// Hands the room to another player.
///
/// By seat rather than by name or account: the caller is looking at a
/// listing of chairs, and a seat index is the one handle in it that cannot
/// be ambiguous when two people share a display name.
///
/// @route POST /lobby/games/{id}/host
/// @scope public
/// @summary Hand the room to another player
/// @operation-id `hand_over_room`
/// @description The host makes whoever sits in that chair the host. The
/// chair tickets the old host handed out and nobody redeemed are dropped.
/// Answers the whole listing as the caller sees it.
/// @tag Lobby
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The room's game id"
/// @return 403: `$ErrorBody` "The account is banned (`ban`) or must accept the terms first (`terms_required`), or the caller is not the host (`not your room`)"
/// @return 404: `$ErrorBody` "No such room (`no such game`) or chair (`no such seat`)"
/// @return 409: `$ErrorBody` "The game has started, or nobody sits in that chair"
pub(crate) async fn hand_over(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<HostBody>,
) -> Result<Json<LobbyListing>, (StatusCode, Json<ErrorBody>)> {
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
///
/// @route POST /lobby/games/{id}/leave
/// @scope public
/// @summary Stand up from a waiting room
/// @description Frees the caller's chair in a room that has not started.
/// A seat bridge the caller seated goes with it; a room whose host leaves
/// passes to the player who joined earliest, and closes when nobody is
/// left.
/// @tag Lobby
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The room's game id"
/// @return 204: "The chair is free"
/// @return 404: `$ErrorBody` "No such room (`no such game`), or the caller has no chair there (`you are not at this table`)"
/// @return 409: `$ErrorBody` "The game has already started"
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
    if !game.leave(&account_id, auth::now_secs()) {
        return Err(err(StatusCode::NOT_FOUND, "you are not at this table"));
    }
    drop(lobby);
    chair::revoke(&state, &id, &account_id);
    crate::seathost::stop_unseated(&state);
    state.lobby_moved();
    Ok(StatusCode::NO_CONTENT)
}
