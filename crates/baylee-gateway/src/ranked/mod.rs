//! Admission on the host Central selected. Waiting for the second player is private:
//! no mutable lobby room, chair ticket or client-supplied result exists for a reservation.
use crate::{ErrorBody, HeaderMap, Json, Path, Shared, State, StatusCode, auth, err};
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
mod replica_table;
pub(crate) mod replicate;
pub(crate) use delivery::worker;

type Failure = (StatusCode, Json<ErrorBody>);
type Table = tokio::sync::Mutex<[Option<Arrival>; 2]>;

pub(crate) struct Host {
    owner: AtomicBool,
    /// Only assignments confirmed by Central occupy memory. Reaping a pending
    /// entry loses no score and a player's next retry can supply the deck again.
    waiting: parking_lot::Mutex<HashMap<Uuid, (Instant, Arc<Table>)>>,
    limiter: auth::RateLimiter,
    /// Shared anonymous verification budget for the relay's ranked endpoint.
    pub(crate) login_attempts: auth::RateLimiter,
    pub(crate) login_slots: tokio::sync::Semaphore,
    pub(crate) seat_logins: auth::RateLimiter,
    admissions: tokio::sync::Semaphore,
    replicas: replicate::Registry,
}

impl Default for Host {
    fn default() -> Self {
        Self {
            owner: AtomicBool::new(false),
            waiting: parking_lot::Mutex::default(),
            limiter: auth::RateLimiter::new(Duration::from_secs(60), 60),
            login_attempts: auth::RateLimiter::new(Duration::from_secs(1), 64),
            login_slots: tokio::sync::Semaphore::new(8),
            seat_logins: auth::RateLimiter::new(Duration::from_secs(60), 12),
            admissions: tokio::sync::Semaphore::new(16),
            replicas: replicate::Registry::default(),
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

pub(crate) fn seat_names() -> Vec<String> {
    ["Seat A", "Seat B"].map(str::to_owned).into()
}

/// Frozen local evidence for reclaiming an already-running game without
/// uploading either deck a second time.
#[derive(Clone, Copy)]
pub(crate) struct Binding {
    pub(crate) id: Uuid,
    format: Format,
    decks: [[u8; 32]; 2],
    pub(crate) replica: Option<replica_table::Role>,
}

impl Binding {
    /// Single-host tables retain their normal admission. Replicas expose seats
    /// only while this primary's locally measured Central lease still stands.
    pub(crate) fn seats_live(self) -> bool {
        self.replica.is_none_or(replica_table::Role::serving)
    }
}

#[derive(serde::Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub(crate) enum RankedPlay {
    Submit(RankedDeck),
    Resume { game: Uuid },
}

/// @description The exact source deck version selected for this ranked entry.
/// @openapi
#[derive(Clone, serde::Deserialize, serde::Serialize)]
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
/// @path-param match_id: String "Version 7 UUID for the Central match"
/// @body `$RankedPlay` "Frozen registered deck or previously bound game"
/// @return 200: `$SeatGrant` "Assigned player's seat capability"
/// @return 202: "Waiting for the other player"
/// @return 400: `$ErrorBody` "Invalid deck, format or assignment ID"
/// @return 403: `$ErrorBody` "Wrong host or player"
/// @return 409: `$ErrorBody` "Assignment, commitment or existing participation conflicts"
/// @return 429: `$ErrorBody` "Ranked admission is busy"
/// @return 503: `$ErrorBody` "Central, storage, catalog or engine unavailable"
#[expect(
    clippy::doc_markdown,
    reason = "oas-forge reads the route and match_id parameter literally, including any Markdown backticks"
)]
pub(crate) async fn play(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(body): Json<RankedPlay>,
) -> Result<Response, Failure> {
    if id.get_version_num() != 7 || id.get_variant() != uuid::Variant::RFC4122 {
        return Err(err(StatusCode::BAD_REQUEST, "invalid ranked match ID"));
    }
    let bearer = crate::bearer_token(&headers)
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "missing session"))?;
    let live = state
        .sessions
        .scoped_bearer(
            &auth::token_digest(bearer),
            crate::sessions::now(),
            Some(*id.as_bytes()),
        )
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "invalid session"))?;
    let account = live
        .signed()
        .filter(|s| !s.admin)
        .map(|s| s.central)
        .ok_or_else(|| {
            err(
                StatusCode::FORBIDDEN,
                "a Central player session is required",
            )
        })?;
    if !state.ranked.limiter.allow(live.account()) {
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
    // Central checked the matched player's eligibility and terms before issuing
    // this pseudonymous admission; the host has no ordinary account to resolve.
    match body {
        RankedPlay::Submit(deck) => register(&state, &bridge, id, live, account, deck).await,
        RankedPlay::Resume { game } => resume(&state, &bridge, id, game, &live, account).await,
    }
}

async fn register(
    state: &Shared,
    bridge: &crate::central::Bridge,
    id: Uuid,
    live: Arc<crate::sessions::Live>,
    account: Id,
    body: RankedDeck,
) -> Result<Response, Failure> {
    let format = body.format()?;
    let (deck, digest, pool) = deck::validate(
        state,
        format,
        &body.cards,
        &body.sideboard,
        &body.commanders,
    )
    .await?;
    let assignment = claim(bridge, id, &live, account, digest).await?;
    let seat = assigned_seat(&assignment, id, format, account, digest)?;
    let table = state.ranked.table(id)?;
    let mut arrivals = table.lock().await;
    // A peer can finish launching after this claim reached Central. Resolve the
    // game after taking the per-assignment lock so concurrent retries resume it.
    let running = state
        .lobby
        .lock()
        .running()
        .find(|g| g.ranked.is_some_and(|b| b.id == id))
        .map(|g| g.id.clone());
    if let Some(game) = running {
        return grant(state, &live, id, game);
    }
    if live.signed().is_some_and(|s| s.ranked_generation.is_some()) && assignment.game.is_some() {
        // The committed setup can be restoring on this host. Never reopen the
        // deck-registration flow or reserve a new game for that assignment.
        return Ok(StatusCode::ACCEPTED.into_response());
    }
    arrivals[seat] = Some(Arrival {
        live: live.clone(),
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
    let game = launch(state, bridge, &assignment, id, players).await?;
    *arrivals = [None, None];
    state.ranked.waiting.lock().remove(&id);
    grant(state, &live, id, game)
}

async fn claim(
    bridge: &crate::central::Bridge,
    id: Uuid,
    live: &crate::sessions::Live,
    account: Id,
    digest: [u8; 32],
) -> Result<HostAssignment, Failure> {
    let request = HostRequest::Claim {
        match_id: *id.as_bytes(),
        account,
        proof: live
            .key()
            .strip_prefix(b"central:")
            .and_then(|sid| std::str::from_utf8(sid).ok())
            .and_then(|sid| central_api::ranked::SeatProof::from_sid(sid).ok())
            .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "missing ranked seat proof"))?,
        deck: digest,
    };
    match bridge.ranked(request).await {
        Ok(HostResponse::Assignment { assignment }) => Ok(assignment),
        Err(code) => Err(err(code, "ranked seat claim refused")),
        Ok(_) => Err(err(
            StatusCode::SERVICE_UNAVAILABLE,
            "unexpected ranked assignment",
        )),
    }
}

async fn resume(
    state: &Shared,
    bridge: &crate::central::Bridge,
    id: Uuid,
    game: Uuid,
    live: &crate::sessions::Live,
    account: Id,
) -> Result<Response, Failure> {
    let game_id = game.to_string();
    let (seat, binding) = if let Some(generation) = live.signed().and_then(|s| s.ranked_generation)
    {
        if id != game {
            return Err(err(StatusCode::FORBIDDEN, "wrong ranked game"));
        }
        let Some(assignment) = replica_table::assigned(state, id, generation, account)? else {
            return Ok(StatusCode::ACCEPTED.into_response());
        };
        assignment
    } else {
        let lobby = state.lobby.lock();
        lobby
            .games
            .get(&game_id)
            .and_then(|game| Some((game.resumable_seat(live.account())?, game.ranked?)))
            .filter(|(_, b)| b.id == id)
            .ok_or_else(|| err(StatusCode::FORBIDDEN, "no matching ranked seat"))?
    };
    let digest = binding.decks[seat];
    let a = claim(bridge, id, live, account, digest).await?;
    if assigned_seat(&a, id, binding.format, account, digest)? != seat
        || a.game != Some(*game.as_bytes())
    {
        return Err(err(StatusCode::FORBIDDEN, "ranked seat no longer assigned"));
    }
    grant(state, live, id, game_id)
}

fn grant(
    state: &Shared,
    live: &crate::sessions::Live,
    id: Uuid,
    game: String,
) -> Result<Response, Failure> {
    if !live.stands(crate::sessions::now()) {
        return Err(err(StatusCode::UNAUTHORIZED, "ranked session expired"));
    }
    if let Some(generation) = live.signed().and_then(|s| s.ranked_generation) {
        return replica_table::grant(state, live, id, &game, generation);
    }
    crate::routes::claim_seat(
        state,
        live.account(),
        Some(live.key().to_vec()),
        game,
        Some(id),
    )
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
    let generations = players.map(|p| p.live.signed().and_then(|s| s.ranked_generation));
    if generations[0] != generations[1] {
        return Err(err(StatusCode::CONFLICT, "ranked generation changed"));
    }
    if let Some(generation) = generations[0] {
        return replicate::freeze(state, bridge, assignment, players, generation).await;
    }
    let preset = deck::preset(
        state,
        assignment.format,
        [&players[0].deck, &players[1].deck],
        auth::new_game_seed(),
    )
    .await?;
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
            s
        })
        .collect();
    let rules = preset.house_rules.clone();
    let mut game =
        crate::lobby::LobbyGame::playing(game_id.clone(), seats, preset, auth::now_secs());
    game.ranked = Some(Binding {
        id,
        format: assignment.format,
        decks: assignment.decks,
        replica: None,
    });
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
