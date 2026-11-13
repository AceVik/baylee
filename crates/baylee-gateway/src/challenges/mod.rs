//! Official execution of Central's immutable global tasks. No game rules run here.
use crate::{
    ErrorBody, HeaderMap, Json, Path, Shared, State, StatusCode, auth, authed, engine, err, lobby,
};
use baylee_protocol::v1::challenge_result::Outcome;
use central_api::challenges::{AttemptState, Completion, HostRequest, HostResponse};
use uuid::Uuid;

mod delivery;
pub(crate) use delivery::start as start_delivery;

/// A slow record write is not a failed game. Keep reconciliation from voiding
/// it until the writer has either committed or explicitly failed.
pub(crate) fn finalizing(
    state: &Shared,
    game_id: &str,
    stored: tokio::sync::oneshot::Receiver<()>,
) {
    if let Some(game) = state.lobby.lock().games.get_mut(game_id) {
        game.challenge_finalizing = true;
    }
    let state = state.clone();
    let game_id = game_id.to_owned();
    tokio::spawn(async move {
        let _ = stored.await;
        if let Some(game) = state.lobby.lock().games.get_mut(&game_id) {
            game.challenge_finalizing = false;
        }
    });
}

/// @route POST /challenges/{attempt}/play
/// @scope public
/// @summary Play or resume a reserved global challenge
/// @description The official host claims the caller's Central attempt and supplies its frozen setup. Repeated requests resume the original game; no seed, deck, account ID or score is accepted from the player.
/// @tag Challenges
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param attempt: String "Central attempt `UUIDv7`"
/// @return 200: `$SeatGrant` "The player's official seat"
/// @return 400: `$ErrorBody` "Invalid attempt identity"
/// @return 403: `$ErrorBody` "Not the official host or no Central session"
/// @return 409: `$ErrorBody` "Attempt ended, another game is running, or the required executor is unavailable"
/// @return 429: `$ErrorBody` "Too many starts or another start is in progress"
/// @return 503: `$ErrorBody` "Central, storage or the executor is unavailable; the attempt is retained"
pub(crate) async fn play(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(attempt): Path<String>,
) -> Result<Json<crate::routes::SeatGrant>, (StatusCode, Json<ErrorBody>)> {
    let account = authed(&state, &headers).await?;
    let attempt = attempt_id(&attempt)?;
    if !state.game_limiter.allow(&account) {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many games"));
    }
    // A bounded admission lock serializes claim/launch/resume, without holding the lobby across I/O.
    let _start = state.challenge_starts.try_lock().map_err(|_| {
        err(
            StatusCode::TOO_MANY_REQUESTS,
            "another challenge is starting",
        )
    })?;
    let bridge = state
        .central
        .get()
        .filter(|bridge| bridge.official())
        .ok_or_else(|| err(StatusCode::FORBIDDEN, "challenges run on the official host"))?;
    require_owner(&state)?;
    let bearer = crate::bearer_token(&headers)
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "missing session"))?;
    let live = state
        .sessions
        .by_bearer(&auth::token_digest(bearer), crate::sessions::now())
        .ok_or_else(|| err(StatusCode::FORBIDDEN, "a Central session is required"))?;
    let signed = live
        .signed()
        .filter(|signed| !signed.admin)
        .ok_or_else(|| err(StatusCode::FORBIDDEN, "a Central session is required"))?;
    let local = Uuid::parse_str(&account)
        .map_err(|_| err(StatusCode::SERVICE_UNAVAILABLE, "account unavailable"))?;
    let proposed = baylee_db::challenges::reserve(
        &state.db,
        attempt,
        Uuid::now_v7(),
        local,
        Uuid::from_bytes(signed.central),
    )
    .await
    .map_err(|_| {
        err(
            StatusCode::SERVICE_UNAVAILABLE,
            "challenge storage unavailable",
        )
    })?;
    let game_id = proposed.to_string();
    {
        let lobby = state.lobby.lock();
        crate::routes::participation::account(&lobby, &account, Some(&game_id))?;
    }
    let claimed = bridge
        .challenge(HostRequest::Claim {
            attempt: *attempt.as_bytes(),
            account: signed.central,
            game: *proposed.as_bytes(),
            executor: baylee_challenges::executor().into(),
        })
        .await;
    let claimed = match claimed {
        Ok(claimed) => claimed,
        Err(status) => {
            if status.is_client_error() && status != StatusCode::TOO_MANY_REQUESTS {
                let _ = baylee_db::challenges::discard_unstarted(&state.db, attempt).await;
            }
            return Err(err(status, "challenge could not be claimed"));
        }
    };
    let HostResponse::Claimed {
        attempt: claimed,
        publication,
    } = claimed
    else {
        return Err(err(
            StatusCode::SERVICE_UNAVAILABLE,
            "unexpected challenge response",
        ));
    };
    if claimed.id != *attempt.as_bytes()
        || claimed.game != Some(*proposed.as_bytes())
        || claimed.state != AttemptState::Playing
    {
        return Err(err(StatusCode::CONFLICT, "challenge claim does not match"));
    }
    let already = state.lobby.lock().games.contains_key(&game_id);
    if already {
        return crate::routes::take_seat(State(state.clone()), headers, Path(game_id)).await;
    }
    launch(
        &state,
        &account,
        live.key(),
        attempt,
        proposed,
        &publication,
    )
    .await
}

fn attempt_id(value: &str) -> Result<Uuid, (StatusCode, Json<ErrorBody>)> {
    Uuid::parse_str(value)
        .ok()
        .filter(|id| id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122)
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "invalid challenge attempt"))
}

fn require_owner(state: &Shared) -> Result<(), (StatusCode, Json<ErrorBody>)> {
    if state
        .challenge_owner
        .load(std::sync::atomic::Ordering::Acquire)
    {
        Ok(())
    } else {
        Err(err(
            StatusCode::SERVICE_UNAVAILABLE,
            "official challenge host is not ready",
        ))
    }
}

async fn launch(
    state: &Shared,
    account: &str,
    session: &[u8],
    attempt: Uuid,
    proposed: Uuid,
    publication: &central_api::challenges::Publication,
) -> Result<Json<crate::routes::SeatGrant>, (StatusCode, Json<ErrorBody>)> {
    let game_id = proposed.to_string();
    let package = baylee_challenges::Package::from_publication(publication)
        .map_err(|_| err(StatusCode::CONFLICT, "challenge executor unavailable"))?;
    {
        let lobby = state.lobby.lock();
        crate::routes::participation::account(&lobby, account, None)?;
    }
    if !baylee_db::challenges::launch_once(&state.db, attempt, proposed)
        .await
        .map_err(|_| {
            err(
                StatusCode::SERVICE_UNAVAILABLE,
                "challenge storage unavailable",
            )
        })?
    {
        return Err(err(StatusCode::CONFLICT, "challenge recovery is pending"));
    }
    let seat_token = auth::new_token();
    let mut seats = vec![lobby::LobbySeat::open(0), lobby::LobbySeat::open(1)];
    seats[0].account_id = Some(account.to_owned());
    seats[0].seat_token_hash = Some(auth::token_hash(&seat_token));
    seats[0].central_session = Some(session.to_vec());
    seats[0]
        .deck_name
        .clone_from(&publication.description.title.en);
    seats[1].kind = lobby::SeatKind::Ai;
    let rules = package.preset.house_rules.clone();
    let mut game =
        lobby::LobbyGame::playing(game_id.clone(), seats, package.preset, auth::now_secs());
    game.challenge = Some(*attempt.as_bytes());
    game.house_rules = rules;
    game.allow_spectators = false;
    {
        let mut lobby = state.lobby.lock();
        crate::routes::participation::account(&lobby, account, None)?;
        lobby.games.insert(game_id.clone(), game);
    }
    if engine::start_engine(state, &game_id).is_err() {
        engine::end_game(state, &game_id);
        return Err(err(
            StatusCode::SERVICE_UNAVAILABLE,
            "challenge engine unavailable",
        ));
    }
    state.lobby_moved();
    Ok(Json(crate::routes::SeatGrant {
        game_id,
        seat: 0,
        seat_token,
    }))
}

/// Translate only authenticated engine evidence for this exact bound game.
pub(crate) fn completion(
    state: &Shared,
    game_id: &str,
    ended: &baylee_protocol::v1::GameEnded,
) -> Option<Completion> {
    let attempt = state.lobby.lock().games.get(game_id)?.challenge?;
    if ended.game_id != game_id {
        return None;
    }
    let result = ended.challenge.as_ref()?;
    let outcome = match Outcome::try_from(result.outcome).ok()? {
        Outcome::Void => AttemptState::Void,
        Outcome::Won if ended.winners == [0] && ended.turn > 0 => AttemptState::Won,
        Outcome::Lost if !ended.winners.contains(&0) && ended.turn > 0 => AttemptState::Lost,
        Outcome::Abandoned => AttemptState::Abandoned,
        _ => return None,
    };
    let completion = Completion {
        attempt,
        game: *Uuid::parse_str(game_id).ok()?.as_bytes(),
        state: outcome,
        turn: ended.turn,
        finished_at: i64::try_from(result.finished_at_unix_ms / 1000).ok()?,
        milliseconds: result.human_ms,
        record_digest: result.record_sha256.as_slice().try_into().ok()?,
    };
    completion.validate().ok()?;
    Some(completion)
}
