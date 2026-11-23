//! Surface appearance travels outside the rules and view streams. A guest can
//! see their opponents' selected sleeves using the same seat as their game socket.
use crate::{
    ErrorBody, HeaderMap, Json, Path, Shared, State, StatusCode, bearer_token, err,
    lobby::{LobbyGame, LobbyState},
    seat_of_token,
    sessions::Live,
};
use central_api::{
    advent::{
        self,
        table::{HostRequest, HostResponse, Proof, Worn},
    },
    ranked::SeatProof,
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Weak},
    time::{Duration, Instant},
};
use uuid::Uuid;

type Failure = (StatusCode, Json<ErrorBody>);
pub(super) const PRIVATE: [(axum::http::HeaderName, &str); 1] =
    [(axum::http::header::CACHE_CONTROL, "private, no-store")];

pub(super) fn accepts_framing(headers: &HeaderMap) -> bool {
    let mut values = headers.get_all(advent::framing::CAPABILITY_HEADER).iter();
    matches!((values.next(), values.next()), (Some(value), None) if value.as_bytes() == b"1")
}
// Media traffic uses at most three of this host's sixteen Central exchange
// slots, leaving room for other operations and revocation synchronization.
static EXCHANGES: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(3);

#[derive(Default)]
pub(crate) struct Looks {
    seats: BTreeMap<usize, Presented>,
    sequence: u64,
    latest: BTreeMap<usize, u64>,
    closure: Option<(Weak<crate::central::Bridge>, advent::Id)>,
}

pub(super) struct Presented {
    pub(super) worn: Worn,
    proof: Proof,
    live: Weak<Live>,
    until: Instant,
}

pub(super) fn participant(game: &LobbyGame, seat: usize) -> bool {
    game.ranked.is_none()
        && game.challenge.is_none()
        && game.state == LobbyState::Playing
        && game
            .seats
            .get(seat)
            .is_some_and(|chair| chair.account_id.is_some() && !chair.participation_ended)
}

impl Looks {
    pub(crate) fn finish(&mut self) {
        self.seats.clear();
        self.latest.clear();
        let Some((bridge, game)) = self.closure.take() else {
            return;
        };
        if let (Some(bridge), Ok(runtime)) =
            (bridge.upgrade(), tokio::runtime::Handle::try_current())
        {
            runtime.spawn(async move {
                // A crash or unavailable Central is covered by the grant and receipt expiries.
                let _ = bridge.casual_appearance(HostRequest::Closed { game }).await;
            });
        }
    }
    pub(super) fn get(&self, game: &LobbyGame, seat: usize) -> Option<&Presented> {
        if !participant(game, seat) {
            return None;
        }
        let item = self.seats.get(&seat)?;
        let owner = item.live.upgrade()?;
        let chair = game.seats.get(seat)?;
        (Instant::now() < item.until
            && owner.stands(crate::sessions::now())
            && chair.central_session.as_deref() == Some(owner.key())
            && chair.account_id.as_deref() == Some(owner.account()))
        .then_some(item)
    }
}

impl Drop for Looks {
    fn drop(&mut self) {
        self.finish();
    }
}

fn denied() -> Failure {
    err(StatusCode::FORBIDDEN, "appearance unavailable at this seat")
}
fn capacity() -> Result<tokio::sync::SemaphorePermit<'static>, Failure> {
    EXCHANGES
        .try_acquire()
        .map_err(|_| err(StatusCode::TOO_MANY_REQUESTS, "appearance service busy"))
}
struct Viewer {
    seat: usize,
    live: Arc<Live>,
    hash: String,
}
impl Viewer {
    async fn read(state: &Shared, game: &str, headers: &HeaderMap) -> Result<Self, Failure> {
        if state
            .tower
            .as_ref()
            .is_some_and(|tower| tower.game(game).is_some())
        {
            return Err(denied());
        }
        let token = bearer_token(headers).ok_or_else(denied)?;
        let hash = crate::auth::token_hash(token);
        let seat = {
            let lobby = state.lobby.lock();
            let table = lobby.games.get(game).ok_or_else(denied)?;
            seat_of_token(table, token)
                .filter(|&n| participant(table, n))
                .ok_or_else(denied)?
        };
        let grant = super::seat_socket::SeatAuthorization::read(state, game, seat, &hash)
            .ok_or_else(denied)?;
        let live = grant.session(state, game, seat).await.ok_or_else(denied)?;
        Ok(Self { seat, live, hash })
    }
    fn seated(&self, game: &LobbyGame) -> bool {
        participant(game, self.seat)
            && self.live.stands(crate::sessions::now())
            && game.seats[self.seat].seat_token_hash.as_deref() == Some(self.hash.as_str())
            && game.seats[self.seat].account_id.as_deref() == Some(self.live.account())
    }
    fn sid(&self) -> Option<SeatProof> {
        let signed = self.live.signed()?;
        if signed.admin || signed.ranked_match.is_some() {
            return None;
        }
        let sid = std::str::from_utf8(self.live.key().strip_prefix(b"central:")?).ok()?;
        SeatProof::from_sid(sid).ok()
    }
}

/// @description A temporary table invitation returned to the wearer by Central.
/// @openapi
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PresentLook {
    /// @description Secret proof from the wearer's current native session, or null to remove the appearance.
    proof: Option<Proof>,
}

/// @route POST /games/{id}/event-look
/// @scope public
/// @summary Present equipped event rewards to this table
/// @description Confirm the wearer's short-lived invitation for the actual chair in a running casual game.
/// @tag Lobby
/// @security `seatToken`
/// @operation-id `present_event_look`
/// @path-param id: String "The game's id"
/// @body `$PresentLook`
/// @return 204: "The selected event surfaces are visible to the game's participants"
pub(crate) async fn present(
    State(state): State<Shared>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<PresentLook>,
) -> Result<StatusCode, Failure> {
    let viewer = Viewer::read(&state, &id, &headers).await?;
    let admission = viewer.sid().ok_or_else(denied)?;
    let game = Uuid::parse_str(&id).map_err(|_| denied())?.into_bytes();
    let bridge = state.central.get().ok_or_else(denied)?;
    let _slot = capacity()?;
    let started = Instant::now();
    let (sequence, proof) = {
        let mut lobby = state.lobby.lock();
        let table = lobby
            .games
            .get_mut(&id)
            .filter(|table| viewer.seated(table))
            .ok_or_else(denied)?;
        let looks = &mut table.event_looks;
        looks.sequence = looks.sequence.checked_add(1).ok_or_else(denied)?;
        looks.latest.insert(viewer.seat, looks.sequence);
        let Some(proof) = body.proof else {
            looks.seats.remove(&viewer.seat);
            return Ok(StatusCode::NO_CONTENT);
        };
        looks.closure = Some((Arc::downgrade(&bridge), game));
        (looks.sequence, proof)
    };
    let request = HostRequest::Confirm {
        game,
        seat: u8::try_from(viewer.seat).map_err(|_| denied())?,
        proof: proof.clone(),
        admission,
    };
    let HostResponse::Confirmed { worn, valid_for } = bridge
        .casual_appearance(request)
        .await
        .map_err(|code| err(code, "appearance unavailable"))?
    else {
        return Err(denied());
    };
    let until = started + Duration::from_secs(u64::from(valid_for.min(600)));
    let mut lobby = state.lobby.lock();
    let table = lobby
        .games
        .get_mut(&id)
        .filter(|table| viewer.seated(table))
        .ok_or_else(denied)?;
    if table.event_looks.latest.get(&viewer.seat) != Some(&sequence) || until <= Instant::now() {
        return Err(denied());
    }
    table.event_looks.seats.insert(
        viewer.seat,
        Presented {
            proof,
            worn,
            live: Arc::downgrade(&viewer.live),
            until,
        },
    );
    Ok(StatusCode::NO_CONTENT)
}

/// @description A released appearance on one of the participants' chairs.
/// @openapi
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EventArt {
    /// @description Wearer's seat index.
    seat: u8,
    /// @description Which equipped surface to read.
    kind: advent::Kind,
    /// @description Aligned offset in bytes.
    offset: u32,
    /// @description Optional digest of the previously authorized image at offset zero.
    cached: Option<advent::Digest>,
}

/// @route POST /games/{id}/event-art
/// @scope public
/// @summary Read a current participant's equipped event image
/// @description Return a bounded range or validate an exact cache digest after checking the viewer and wearer.
/// @tag Lobby
/// @security `seatToken`
/// @operation-id `read_event_art`
/// @path-param id: String "The game's id"
/// @header-param baylee-mat-framing: String optional `pattern=^1$` "Send 1 once to receive available playmat composition with image pages and cache hits"
/// @body `$EventArt`
/// @return 200: `$EventArtReply` "The authorized chunk or validated cache entry"
/// @return 403: `$ErrorBody` "There is no current appearance for this seat"
pub(crate) async fn art(
    State(state): State<Shared>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<EventArt>,
) -> Result<impl axum::response::IntoResponse, Failure> {
    let viewer = Viewer::read(&state, &id, &headers).await?;
    let worn_seat = usize::from(body.seat);
    let proof = {
        let lobby = state.lobby.lock();
        let table = lobby
            .games
            .get(&id)
            .filter(|g| viewer.seated(g))
            .ok_or_else(denied)?;
        table
            .event_looks
            .get(table, worn_seat)
            .ok_or_else(denied)?
            .proof
            .clone()
    };
    let digest = proof.digest();
    let game = Uuid::parse_str(&id).map_err(|_| denied())?.into_bytes();
    let _slot = capacity()?;
    let bridge = state.central.get().ok_or_else(denied)?;
    let mut response = match bridge
        .casual_appearance(HostRequest::Art {
            game,
            seat: body.seat,
            kind: body.kind,
            offset: body.offset,
            cached: body.cached,
            proof,
        })
        .await
        .map_err(|code| err(code, "appearance unavailable"))?
    {
        HostResponse::Asset { page } => advent::Response::Asset { page },
        HostResponse::Cached { asset } => advent::Response::Cached { asset },
        HostResponse::Confirmed { .. } | HostResponse::Closed => return Err(denied()),
    };
    let lobby = state.lobby.lock();
    let table = lobby
        .games
        .get(&id)
        .filter(|g| viewer.seated(g))
        .ok_or_else(denied)?;
    if table
        .event_looks
        .get(table, worn_seat)
        .is_none_or(|item| item.proof.digest() != digest)
    {
        return Err(denied());
    }
    if !accepts_framing(&headers) {
        response.omit_framing();
    }
    Ok((PRIVATE, Json(response)))
}
