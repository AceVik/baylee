use super::{Error, Shared, Uuid, launch};
use crate::{ErrorBody, HeaderMap, Json, Path, State, StatusCode, auth, authed, err};
use serde::{Deserialize, Serialize};

type Refused = (StatusCode, Json<ErrorBody>);

/// @openapi
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Play {}

/// @openapi
#[derive(Serialize)]
pub(crate) struct Availability {
    pub host: Uuid,
    pub origin: String,
    pub engines: Vec<baylee_protocol::execution::Identity>,
}

pub(super) struct Player {
    pub local: String,
    pub central: Uuid,
    pub session: Vec<u8>,
}

async fn player(state: &Shared, headers: &HeaderMap) -> Result<Player, Refused> {
    let local = authed(state, headers).await?;
    if !state.game_limiter.allow(&local) {
        return Err(err(
            StatusCode::TOO_MANY_REQUESTS,
            "too many Tower requests",
        ));
    }
    let bearer = crate::bearer_token(headers)
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "missing session"))?;
    let live = state
        .sessions
        .by_bearer(&auth::token_digest(bearer), crate::sessions::now())
        .ok_or_else(|| err(StatusCode::FORBIDDEN, "a Central session is required"))?;
    let signed = live
        .signed()
        .filter(|signed| !signed.admin)
        .ok_or_else(|| err(StatusCode::FORBIDDEN, "a Central session is required"))?;
    Ok(Player {
        local,
        central: Uuid::from_bytes(signed.central),
        session: live.key().to_vec(),
    })
}

/// @route GET /tower
/// @scope public
/// @summary Discover this gateway's Tower execution capacity
/// @description Requires a live Central player session. Reports the authenticated
/// Tower host UUID, configured authority origin and currently available exact
/// Tower engine identities. No player, deck or outcome is accepted.
/// @tag Tower
/// @security bearer
/// @insert `SessionRefusals`
/// @return 200: `$Availability` "Host UUID, authority origin and exact engine identities"
/// @return 429: `$ErrorBody` "Request budget or host admission busy"
/// @return 503: `$ErrorBody` "Tower authority or compatible agent unavailable"
pub(crate) async fn available(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<Availability>, Refused> {
    player(&state, &headers).await?;
    let tower = state.tower.as_ref().ok_or_else(unavailable)?;
    let _start = tower.starts.try_lock().map_err(|_| failure(Error::Busy))?;
    tower.advertise(&state).await.map(Json).map_err(failure)
}

/// @route POST /tower/{run}/play
/// @scope public
/// @summary Attach the owner's reserved Tower duel
/// @operation-id `play_tower`
/// @description The body is an empty JSON object. Central identity comes from
/// the gateway session. The global authority supplies the immutable reservation;
/// only its exact engine can launch. Repeating while attached reclaims the human
/// seat. Progression and rewards require independent authority verification.
/// @tag Tower
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param run: String "The owner's UUIDv7 Tower run"
/// @return 200: `$SeatGrant` "Private human seat in the reserved duel"
/// @return 400: `$ErrorBody` "Malformed run UUID or nonempty request object"
/// @return 409: `$ErrorBody` "Reservation conflict or player already in another game"
/// @return 426: `$ErrorBody` "Incompatible Tower contract"
/// @return 429: `$ErrorBody` "Request budget or host admission busy"
/// @return 503: `$ErrorBody` "Tower authority or exact engine unavailable"
pub(crate) async fn play(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(run): Path<String>,
    Json(_choice): Json<Play>,
) -> Result<Json<crate::routes::SeatGrant>, Refused> {
    let player = player(&state, &headers).await?;
    let run = Uuid::parse_str(&run)
        .ok()
        .filter(|id| id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122)
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "invalid Tower run"))?;
    let tower = state.tower.as_ref().ok_or_else(unavailable)?;
    let _start = tower.starts.try_lock().map_err(|_| failure(Error::Busy))?;
    let existing = tower
        .games
        .lock()
        .iter()
        .find(|(_, game)| game.run == run && game.claimed.account == player.central)
        .map(|(id, _)| id.clone());
    if let Some(game) = existing {
        return crate::routes::take_seat(State(state.clone()), headers, Path(game)).await;
    }
    {
        let lobby = state.lobby.lock();
        crate::routes::participation::account(&lobby, &player.local, None)?;
    }
    if state.admission.held() {
        return Err(err(
            StatusCode::SERVICE_UNAVAILABLE,
            "gateway admission held",
        ));
    }
    tower.advertise(&state).await.map_err(failure)?;
    let claimed = tower.claim(player.central, run).await.map_err(failure)?;
    launch::start(&state, tower, run, player, claimed).map(Json)
}

pub(super) fn failure(error: Error) -> Refused {
    let status = match error {
        Error::Rejected | Error::TermsRequired => StatusCode::FORBIDDEN,
        Error::Conflict => StatusCode::CONFLICT,
        Error::Busy => StatusCode::TOO_MANY_REQUESTS,
        Error::UpdateRequired => StatusCode::UPGRADE_REQUIRED,
        Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    };
    err(status, "Tower operation unavailable")
}

pub(super) fn unavailable() -> Refused {
    failure(Error::Unavailable)
}
