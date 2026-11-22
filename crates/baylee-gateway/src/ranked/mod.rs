//! Admission on the host Central selected. Waiting for the second player is private:
//! no mutable lobby room, chair ticket or client-supplied result exists for a reservation.
use crate::{ErrorBody, HeaderMap, Json, Path, Shared, State, StatusCode, auth, authed, err};
use axum::response::{IntoResponse, Response};
use baylee_cards::decks::LoadedDeck;
use central_api::ranked::{Format, HostAssignment, HostRequest, HostResponse, Id, PoolEntry};
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use uuid::Uuid;

mod deck;
mod delivery;
pub(crate) use delivery::worker;

type Failure = (StatusCode, Json<ErrorBody>);
type Table = tokio::sync::Mutex<[Option<Arrival>; 2]>;

pub(crate) struct Host {
    owner: AtomicBool,
    /// Only assignments confirmed by Central occupy memory. Reaping a pending
    /// entry loses no score and a player's next retry can supply the deck again.
    waiting: parking_lot::Mutex<HashMap<Uuid, (Instant, Arc<Table>)>>,
    limiter: auth::RateLimiter,
    admissions: tokio::sync::Semaphore,
}

impl Default for Host {
    fn default() -> Self {
        Self {
            owner: AtomicBool::new(false),
            waiting: parking_lot::Mutex::default(),
            limiter: auth::RateLimiter::new(Duration::from_secs(60), 60),
            admissions: tokio::sync::Semaphore::new(16),
        }
    }
}

impl Host {
    fn reap(&self) {
        self.waiting.lock().retain(|_, (created, table)| {
            created.elapsed() < Duration::from_secs(150) || Arc::strong_count(table) > 1
        });
    }
    fn table(&self, id: Uuid) -> Result<Arc<Table>, Failure> {
        let mut waiting = self.waiting.lock();
        if waiting.len() >= 128 && !waiting.contains_key(&id) {
            return Err(err(
                StatusCode::TOO_MANY_REQUESTS,
                "ranked admission is busy",
            ));
        }
        Ok(waiting
            .entry(id)
            .or_insert_with(|| (Instant::now(), Arc::new(Table::default())))
            .1
            .clone())
    }
    fn require_owner(&self) -> Result<(), Failure> {
        if self.owner.load(Ordering::Acquire) {
            Ok(())
        } else {
            Err(err(
                StatusCode::SERVICE_UNAVAILABLE,
                "ranked host is not ready",
            ))
        }
    }
}

struct Arrival {
    live: Arc<crate::sessions::Live>,
    digest: [u8; 32],
    deck: LoadedDeck,
    pool: Vec<PoolEntry>,
}

/// @description The exact source deck version selected for this ranked entry.
/// @openapi
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RankedDeck {
    /// @description Ranked format (standard, highlander or commander).
    format: String,
    /// @description Main deck rows, including commanders as on the player's deck shelf.
    cards: Vec<String>,
    /// @description Sideboard rows.
    sideboard: Vec<String>,
    /// @description Commanders marked in the main deck.
    commanders: Vec<String>,
}

impl RankedDeck {
    fn format(&self) -> Result<Format, Failure> {
        match self.format.as_str() {
            "standard" => Ok(Format::Standard),
            "highlander" => Ok(Format::Highlander),
            "commander" => Ok(Format::Commander),
            _ => Err(err(StatusCode::BAD_REQUEST, "invalid ranked format")),
        }
    }
}

/// @route POST /ranked/{match_id}/play
/// @scope public
/// @operation-id `playRanked`
/// @summary Claim or resume your ranked duel on its assigned host
/// @description Each participant presents their own authenticated host session and frozen deck. The host waits for both legal registrations before ordering one engine.
/// @tag Ranked
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param match_id: String "Central match UUIDv7"
/// @body `$RankedDeck` "Frozen registered deck"
/// @return 200: `$SeatGrant` "Assigned player's seat capability"
/// @return 202: "Waiting for the other player"
/// @return 400: `$ErrorBody` "Invalid deck, format or assignment ID"
/// @return 403: `$ErrorBody` "Wrong host or player"
/// @return 409: `$ErrorBody` "Assignment, commitment or existing participation conflicts"
/// @return 429: `$ErrorBody` "Ranked admission is busy"
/// @return 503: `$ErrorBody` "Central, storage, catalog or engine unavailable"
pub(crate) async fn play(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(body): Json<RankedDeck>,
) -> Result<Response, Failure> {
    let local = authed(&state, &headers).await?;
    if id.get_version_num() != 7 || id.get_variant() != uuid::Variant::RFC4122 {
        return Err(err(StatusCode::BAD_REQUEST, "invalid ranked match ID"));
    }
    let bearer = crate::bearer_token(&headers)
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "missing session"))?;
    let live = state
        .sessions
        .by_bearer(&auth::token_digest(bearer), crate::sessions::now())
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "invalid session"))?;
    let account = live
        .signed()
        .filter(|s| !s.admin && live.account() == local)
        .map(|s| s.central)
        .ok_or_else(|| {
            err(
                StatusCode::FORBIDDEN,
                "a Central player session is required",
            )
        })?;
    if !state.ranked.limiter.allow(&local) {
        return Err(err(
            StatusCode::TOO_MANY_REQUESTS,
            "too many ranked seat requests",
        ));
    }
    state.ranked.require_owner()?;
    let _permit = state
        .ranked
        .admissions
        .try_acquire()
        .map_err(|_| err(StatusCode::TOO_MANY_REQUESTS, "ranked admission is busy"))?;
    let bridge = state
        .central
        .get()
        .ok_or_else(|| err(StatusCode::SERVICE_UNAVAILABLE, "Central unavailable"))?;
    let format = body.format()?;
    let (deck, digest, pool) = deck::validate(
        &state,
        format,
        &body.cards,
        &body.sideboard,
        &body.commanders,
    )
    .await?;
    let request = HostRequest::Claim {
        match_id: *id.as_bytes(),
        account,
        deck: digest,
    };
    let HostResponse::Assignment { assignment } = bridge
        .ranked(request)
        .await
        .map_err(|code| err(code, "ranked seat claim refused"))?
    else {
        return Err(err(
            StatusCode::SERVICE_UNAVAILABLE,
            "unexpected ranked assignment",
        ));
    };
    let seat = assigned_seat(&assignment, id, format, account, digest)?;
    let table = state.ranked.table(id)?;
    let mut arrivals = table.lock().await;
    // A peer can finish launching after this claim reached Central. Resolve the
    // game after taking the per-assignment lock so concurrent retries resume it.
    let running = state
        .lobby
        .lock()
        .running()
        .find(|g| g.ranked == Some(id))
        .map(|g| g.id.clone());
    if let Some(game) = running {
        return crate::routes::take_seat(State(state.clone()), headers, Path(game))
            .await
            .map(IntoResponse::into_response);
    }
    arrivals[seat] = Some(Arrival {
        live,
        digest,
        deck,
        pool,
    });
    let [Some(a), Some(b)] = &*arrivals else {
        return Ok(StatusCode::ACCEPTED.into_response());
    };
    let players = [a, b];
    for (idx, player) in players.iter().enumerate() {
        if player.digest != assignment.decks[idx]
            || player.live.signed().map(|s| s.central) != Some(assignment.accounts[idx])
            || !player.live.stands(crate::sessions::now())
        {
            return Err(err(StatusCode::CONFLICT, "ranked registration expired"));
        }
    }
    let game = launch(&state, &bridge, &assignment, id, players).await?;
    *arrivals = [None, None];
    state.ranked.waiting.lock().remove(&id);
    crate::routes::take_seat(State(state.clone()), headers, Path(game))
        .await
        .map(IntoResponse::into_response)
}

fn assigned_seat(
    a: &HostAssignment,
    id: Uuid,
    format: Format,
    account: Id,
    digest: [u8; 32],
) -> Result<usize, Failure> {
    let seat = a.accounts.iter().position(|p| *p == account);
    seat.filter(|&s| {
        a.match_id == *id.as_bytes()
            && a.format == format
            && a.accounts[0] != a.accounts[1]
            && a.decks[s] == digest
    })
    .ok_or_else(|| err(StatusCode::FORBIDDEN, "ranked assignment does not match"))
}

async fn launch(
    state: &Shared,
    bridge: &crate::central::Bridge,
    assignment: &HostAssignment,
    id: Uuid,
    players: [&Arrival; 2],
) -> Result<String, Failure> {
    state.ranked.require_owner()?;
    let preset = deck::preset(
        state,
        assignment.format,
        [&players[0].deck, &players[1].deck],
        auth::new_game_seed(),
    )
    .await?;
    let players_available = |lobby: &crate::lobby::Lobby| -> Result<(), Failure> {
        for player in players {
            if !player.live.stands(crate::sessions::now()) {
                return Err(err(StatusCode::FORBIDDEN, "ranked session expired"));
            }
            crate::routes::participation::account(lobby, player.live.account(), None)?;
        }
        Ok(())
    };
    players_available(&state.lobby.lock())?;
    let game = baylee_db::ranked::reserve(&state.db, id)
        .await
        .map_err(|e| crate::db_down(&e.into()))?;
    if assignment
        .game
        .is_some_and(|bound| bound != *game.as_bytes())
    {
        return Err(err(StatusCode::CONFLICT, "ranked game binding changed"));
    }
    match bridge
        .ranked(HostRequest::Start {
            match_id: *id.as_bytes(),
            game: *game.as_bytes(),
            pools: players.map(|p| p.pool.clone()),
        })
        .await
    {
        Ok(HostResponse::Done) => {}
        Err(code) => return Err(err(code, "ranked start refused")),
        Ok(_) => {
            return Err(err(
                StatusCode::SERVICE_UNAVAILABLE,
                "unexpected ranked start response",
            ));
        }
    }
    if !baylee_db::ranked::launch_once(&state.db, id, game)
        .await
        .map_err(|e| crate::db_down(&e.into()))?
    {
        return Err(err(StatusCode::CONFLICT, "ranked game recovery pending"));
    }
    let game_id = game.to_string();
    let seats = players
        .iter()
        .enumerate()
        .map(|(idx, p)| {
            let mut s = crate::lobby::LobbySeat::open(idx);
            s.account_id = Some(p.live.account().to_owned());
            // Mark the seat claimed. Only take_seat can issue a usable capability to its owner.
            s.seat_token_hash = Some(auth::token_hash(&auth::new_token()));
            s.central_session = Some(p.live.key().to_vec());
            s.deck_name.clone_from(&p.deck.name);
            s
        })
        .collect();
    let rules = preset.house_rules.clone();
    let mut game =
        crate::lobby::LobbyGame::playing(game_id.clone(), seats, preset, auth::now_secs());
    game.ranked = Some(id);
    game.house_rules = rules;
    game.allow_spectators = false;
    {
        let mut lobby = state.lobby.lock();
        state.ranked.require_owner()?;
        players_available(&lobby)?;
        lobby.games.insert(game_id.clone(), game);
    }
    if crate::engine::start_engine(state, &game_id).is_err() {
        crate::engine::end_game(state, &game_id);
        return Err(err(
            StatusCode::SERVICE_UNAVAILABLE,
            "ranked engine unavailable",
        ));
    }
    state.lobby_moved();
    Ok(game_id)
}
