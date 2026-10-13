//! `GET /replays` and `POST /replays/{game}/tape` (`docs/protocol.md`
//! §"Replays"): the hosted games a signed-in account sat at, and one of them
//! played again for it as a tape.
//!
//! The seat a tape is drawn from is the one the table says the account
//! played (`records::seat_of`), never one the request names: the body has no
//! seat field and refuses fields it does not know. The public view is any
//! seated account's. The record stays on the gateway; the job that projects
//! it is in `replays.rs`.

use std::time::Duration;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use baylee_db::records::{ListedSeat, Seated, SeatedGame};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::replays::{self, JobError, Refused};
use crate::{ErrorBody, Shared, err};

/// The most games one page of `GET /replays` lists.
pub const MAX_PAGE: u64 = 50;

/// The page `GET /replays` lists when it is not asked for one.
pub const DEFAULT_PAGE: u64 = 20;

/// How long a game that just ended is waited for, for its record's last
/// piece to be stored, before its tape is projected from what is there.
const END_WAIT: Duration = Duration::from_secs(3);

/// A game that ended this recently may still have its last piece on the
/// way.
const JUST_ENDED_SECS: u64 = 10;

/// What a seat of a game in the list is called when it names no account.
const HOUSE: &str = "House AI";
/// What a seat bridge's seat is called.
const MODEL: &str = "Language model";
/// What a seat is called whose account link was cut (the account left).
const FORMER: &str = "Former player";

type Refusal = (StatusCode, Json<ErrorBody>);

/// `GET /replays`'s query.
#[derive(Deserialize)]
pub(crate) struct Page {
    #[serde(default)]
    offset: u64,
    limit: Option<u64>,
    /// The archived games instead of the others.
    #[serde(default)]
    archived: bool,
}

/// One seat of a listed game.
#[derive(Serialize)]
struct SeatOut {
    seat: u8,
    /// `you`, `player`, `former`, `house` or `model`.
    who: &'static str,
    name: String,
    /// The deck's colours, for the asking account's own seat only.
    #[serde(skip_serializing_if = "Option::is_none")]
    colours: Option<String>,
    commander: Option<u32>,
    /// Whether the seat's hidden cards may be shown in this game's replays:
    /// its player shared them, or it is an AI seat.
    shared: bool,
    /// Whether the asking account played this seat.
    mine: bool,
}

/// One listed game.
#[derive(Serialize)]
struct GameOut {
    game: String,
    started_at: String,
    ended_at: Option<String>,
    complete: bool,
    seat: u8,
    turn: Option<u32>,
    /// `won`, `lost`, `draw`, `unfinished`, or `unknown` for a game that
    /// ended before the gateway noted how.
    result: &'static str,
    public: bool,
    /// Whether the asking account archived it.
    archived: bool,
    /// `exact`, `checked` or `unverified`.
    check: &'static str,
    build: Option<String>,
    seats: Vec<SeatOut>,
}

/// `GET /replays`'s answer.
#[derive(Serialize)]
pub(crate) struct Listing {
    games: Vec<GameOut>,
    total: u64,
}

fn account_of(session: &crate::store::Session) -> Result<Uuid, Refusal> {
    Uuid::parse_str(&session.account_id)
        .map_err(|_| err(StatusCode::UNAUTHORIZED, "invalid or expired token"))
}

/// What a seat is, to the account asking.
fn who(seat: &ListedSeat, me: Uuid) -> &'static str {
    if seat.account == Some(me) {
        return "you";
    }
    // A record from before the column: what the links still say.
    match seat.seated {
        Seated::Person | Seated::Unknown if seat.account.is_some() => "player",
        Seated::Person => "former",
        Seated::Model => "model",
        Seated::Unknown if seat.delegated_by.is_some() => "model",
        Seated::House | Seated::Unknown => "house",
    }
}

/// Whether a seat's hidden cards may be shown in a replay at all: an AI seat
/// always, a person's as it said, else by default (yes for a game recorded
/// from the beta.7 deploy on, no before; `docs/protocol.md` §"Replays").
fn shared(seat: &ListedSeat, who: &str) -> bool {
    matches!(who, "house" | "model") || seat.shared
}

/// What a seat is called, in the list and in a tape's opening payload.
fn name_of(seat: &ListedSeat, who: &str) -> String {
    match (who, &seat.name) {
        ("you" | "player", Some((name, tag))) => crate::handle::handle(name, *tag),
        ("model", _) => MODEL.to_owned(),
        ("house", _) => HOUSE.to_owned(),
        _ => FORMER.to_owned(),
    }
}

/// How far today's build is known to play the game as it was played.
fn check_of(game: &SeatedGame) -> &'static str {
    if game.build.as_deref() == Some(baylee_build::short()) {
        "exact"
    } else if game.checked {
        "checked"
    } else {
        "unverified"
    }
}

fn result_of(game: &SeatedGame, seat: u8) -> &'static str {
    match &game.winners {
        _ if !game.complete => "unfinished",
        None => "unknown",
        Some(winners) if winners.is_empty() => "draw",
        Some(winners) if winners.contains(&seat) => "won",
        Some(_) => "lost",
    }
}

fn game_out(game: SeatedGame, me: Uuid, archived: bool) -> GameOut {
    let seat = game
        .seats
        .iter()
        .find(|s| s.account == Some(me))
        .map_or(0, |s| s.seat);
    let seats = game
        .seats
        .iter()
        .map(|s| {
            let who = who(s, me);
            SeatOut {
                seat: s.seat,
                who,
                name: name_of(s, who),
                colours: (who == "you").then(|| s.colours.clone()).flatten(),
                commander: s.commander,
                shared: shared(s, who),
                mine: who == "you",
            }
        })
        .collect();
    GameOut {
        result: result_of(&game, seat),
        check: check_of(&game),
        game: game.game_id,
        started_at: game.started_at,
        ended_at: game.ended_at,
        complete: game.complete,
        seat,
        turn: game.turn,
        public: true,
        archived,
        build: game.build,
        seats,
    }
}

/// `GET /replays?offset=0&limit=20`: the games the account sat at, newest
/// first. Guests too.
pub(crate) async fn list_replays(
    State(state): State<Shared>,
    headers: HeaderMap,
    Query(page): Query<Page>,
) -> Result<Json<Listing>, Refusal> {
    let session = crate::authed_session(&state, &headers).await?;
    let me = account_of(&session)?;
    let limit = page.limit.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE);
    let (games, total) =
        baylee_db::records::seated_games(&state.db, me, page.archived, page.offset, limit)
            .await
            .map_err(|e| crate::db_down(&anyhow::Error::from(e)))?;
    Ok(Json(Listing {
        games: games
            .into_iter()
            .map(|g| game_out(g, me, page.archived))
            .collect(),
        total,
    }))
}

/// Whose view a tape is asked for.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum PerspectiveArg {
    Seat,
    Public,
    AllHands,
}

/// What a check that fails does.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
enum VerifyArg {
    #[default]
    Stop,
    Note,
}

/// `POST /replays/{game}/tape`'s body. `seat` is the seat the table is
/// drawn from (default: the asking account's own); whose hidden cards the
/// tape may hold the gateway decides from consent, never the request.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TapeAsk {
    perspective: PerspectiveArg,
    #[serde(default)]
    seat: Option<u8>,
    #[serde(default)]
    verify: VerifyArg,
}

/// What a tape shows, decided here from the table and each seat's consent,
/// never from the request (`docs/protocol.md` §"Replays").
struct Lens {
    seat: u8,
    perspective: baylee_protocol::v1::project_record::Perspective,
    own: bool,
    hands: Vec<u8>,
    names: Vec<String>,
}

impl Lens {
    /// The lens `ask` asks of a game with `seats`, for `me` at seat `mine`.
    /// A seat's hidden cards are shown only where it is mine, an AI seat, or
    /// shared (the set S).
    fn of(seats: &[ListedSeat], me: Uuid, ask: &TapeAsk, mine: u8) -> Result<Self, Refusal> {
        use baylee_protocol::v1::project_record::Perspective;
        let names = seats.iter().map(|s| name_of(s, who(s, me))).collect();
        let shown: Vec<u8> = seats
            .iter()
            .filter(|s| {
                let who = who(s, me);
                who == "you" || shared(s, who)
            })
            .map(|s| s.seat)
            .collect();
        let perspective = match ask.perspective {
            PerspectiveArg::Seat => Perspective::Seat,
            PerspectiveArg::Public => Perspective::Public,
            PerspectiveArg::AllHands => Perspective::AllHands,
        };
        if perspective == Perspective::Public {
            return Ok(Self {
                seat: 0,
                perspective,
                own: false,
                hands: Vec::new(),
                names,
            });
        }
        let seat = ask.seat.unwrap_or(mine);
        if usize::from(seat) >= seats.len() {
            return Err(err(StatusCode::BAD_REQUEST, "that game has no such seat"));
        }
        Ok(Self {
            seat,
            perspective,
            own: shown.contains(&seat),
            hands: if perspective == Perspective::AllHands {
                shown.into_iter().filter(|&s| s != seat).collect()
            } else {
                Vec::new()
            },
            names,
        })
    }
}

/// Whether the game is still being played here with `account` in a seat.
fn playing_at(state: &Shared, game_id: &str, account: &str) -> bool {
    state.lobby.lock().games.get(game_id).is_some_and(|g| {
        g.state == crate::lobby::LobbyState::Playing
            && g.seats
                .iter()
                .any(|seat| seat.account_id.as_deref() == Some(account))
    })
}

/// Whether the game ended here in the last [`JUST_ENDED_SECS`].
fn just_ended(state: &Shared, game_id: &str) -> bool {
    let now = crate::auth::now_secs();
    state
        .lobby
        .lock()
        .games
        .get(game_id)
        .and_then(|g| g.finished_at)
        .is_some_and(|at| now.saturating_sub(at) <= JUST_ENDED_SECS)
}

/// `POST /replays/{game}/tape`: the game played again for the asking
/// account, `200 application/gzip`, the tape. A tape that stops early
/// (diverged, refused) is still `200`: its `stop` line says why.
pub(crate) async fn replay_tape(
    State(state): State<Shared>,
    Path(game_id): Path<String>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Response, Refusal> {
    let session = crate::authed_session(&state, &headers).await?;
    let me = account_of(&session)?;
    let ask: TapeAsk = serde_json::from_slice(&body)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "the body is not a tape request"))?;
    if !state.replay_jobs.on() {
        return Err(err(
            StatusCode::SERVICE_UNAVAILABLE,
            "replays are off on this server",
        ));
    }
    let db_down = |e: sea_orm::DbErr| crate::db_down(&anyhow::Error::from(e));
    // A game that goes on is said so to a player who sits at it, before its
    // record is looked for: the record's row is written with its first
    // piece, which a short game has not sent yet. Anyone else is told what
    // the store says.
    if playing_at(&state, &game_id, &session.account_id) {
        return Err(err(StatusCode::CONFLICT, "the game is still being played"));
    }
    // A game that just ended may still have its last piece on the way (or,
    // a short one, its only piece), as a report's does.
    let mut size = baylee_db::records::stored_size(&state.db, &game_id)
        .await
        .map_err(db_down)?;
    if just_ended(&state, &game_id) {
        let deadline = tokio::time::Instant::now() + END_WAIT;
        while !size.is_some_and(|(_, complete)| complete) && tokio::time::Instant::now() < deadline
        {
            tokio::time::sleep(Duration::from_millis(100)).await;
            size = baylee_db::records::stored_size(&state.db, &game_id)
                .await
                .map_err(db_down)?;
        }
    }
    if size.is_none() {
        return Err(err(StatusCode::NOT_FOUND, "no such game"));
    }
    let Some(seat) = baylee_db::records::seat_of(&state.db, &game_id, me)
        .await
        .map_err(db_down)?
    else {
        return Err(err(StatusCode::FORBIDDEN, "you did not sit at that game"));
    };
    let Some((bytes, _)) = size else {
        return Err(err(StatusCode::NOT_FOUND, "no such game"));
    };
    if bytes > replays::MAX_RECORD_BYTES {
        return Err(err(
            StatusCode::PAYLOAD_TOO_LARGE,
            "that game's record is too large to replay",
        ));
    }
    let seats = baylee_db::records::seats_of(&state.db, &game_id)
        .await
        .map_err(db_down)?;
    let lens = Lens::of(&seats, me, &ask, seat)?;
    if state.admission.held() {
        return Err(err(StatusCode::SERVICE_UNAVAILABLE, crate::admission::HELD));
    }
    let _slot = replays::admit(&state.replay_jobs, &session.account_id).map_err(|r| match r {
        Refused::TooMany => err(StatusCode::TOO_MANY_REQUESTS, "too many replays asked for"),
        Refused::Busy => err(
            StatusCode::SERVICE_UNAVAILABLE,
            "too many replays are being made; try again in a moment",
        ),
    })?;
    let tape = replays::run(
        &state,
        replays::Ask {
            game_id,
            seat: lens.seat,
            perspective: lens.perspective,
            own: lens.own,
            hands: lens.hands,
            note: ask.verify == VerifyArg::Note,
            names: lens.names,
        },
    )
    .await
    .inspect_err(|e| {
        // An ask no engine took costs the account nothing of its budget.
        if matches!(e, JobError::Held | JobError::NoCapacity) {
            state.replay_jobs.give_back(&session.account_id);
        }
    })
    .map_err(|e| match e {
        JobError::Held => err(StatusCode::SERVICE_UNAVAILABLE, crate::admission::HELD),
        JobError::NoCapacity => err(StatusCode::SERVICE_UNAVAILABLE, "no engine available"),
        JobError::Timeout => err(
            StatusCode::GATEWAY_TIMEOUT,
            "the replay took too long to make",
        ),
        JobError::TooLarge => err(StatusCode::BAD_GATEWAY, "the replay is too large"),
        JobError::Failed => err(StatusCode::BAD_GATEWAY, "the replay could not be made"),
    })?;
    Ok(([(header::CONTENT_TYPE, "application/gzip")], tape).into_response())
}

/// `PUT /replays/{game}/share`'s body.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ShareAsk {
    shared: bool,
}

/// `PUT /replays/{game}/share`'s answer.
#[derive(Serialize)]
pub(crate) struct ShareOut {
    seat: u8,
    shared: bool,
}

/// The asking account's seat at `game_id`: `404` for a game with no
/// record, `403` for one it sat at no seat of.
async fn own_seat(state: &Shared, game_id: &str, me: Uuid) -> Result<u8, Refusal> {
    let db_down = |e: sea_orm::DbErr| crate::db_down(&anyhow::Error::from(e));
    if baylee_db::records::stored_size(&state.db, game_id)
        .await
        .map_err(db_down)?
        .is_none()
    {
        return Err(err(StatusCode::NOT_FOUND, "no such game"));
    }
    baylee_db::records::seat_of(&state.db, game_id, me)
        .await
        .map_err(db_down)?
        .ok_or_else(|| err(StatusCode::FORBIDDEN, "you did not sit at that game"))
}

/// `PUT /replays/{game}/share` `{"shared": bool}`: whether the asking
/// account's own seat shows its hidden cards in this game's replays. The
/// seat is the table's, never the request's. Any time, also while the game
/// is played (once its record has begun).
pub(crate) async fn share_replay(
    State(state): State<Shared>,
    Path(game_id): Path<String>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<ShareOut>, Refusal> {
    let session = crate::authed_session(&state, &headers).await?;
    let me = account_of(&session)?;
    let ask: ShareAsk = serde_json::from_slice(&body)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "the body is not a sharing choice"))?;
    let seat = own_seat(&state, &game_id, me).await?;
    baylee_db::records::set_share(&state.db, &game_id, seat, ask.shared)
        .await
        .map_err(|e| crate::db_down(&anyhow::Error::from(e)))?;
    Ok(Json(ShareOut {
        seat,
        shared: ask.shared,
    }))
}

/// `PUT /replays/{game}/archived`'s body and answer.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Archived {
    archived: bool,
}

/// `PUT /replays/{game}/archived` `{"archived": bool}`: moves the game to
/// the asking account's archived list (`GET /replays?archived=true`), or
/// back. The record is never deleted (it trains the house AI); nothing else
/// changes, and an archived game's tapes are answered as before.
pub(crate) async fn archive_replay(
    State(state): State<Shared>,
    Path(game_id): Path<String>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<Archived>, Refusal> {
    let session = crate::authed_session(&state, &headers).await?;
    let me = account_of(&session)?;
    let ask: Archived = serde_json::from_slice(&body)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "the body is not an archive choice"))?;
    own_seat(&state, &game_id, me).await?;
    baylee_db::records::set_archived(&state.db, me, &game_id, ask.archived)
        .await
        .map_err(|e| crate::db_down(&anyhow::Error::from(e)))?;
    Ok(Json(ask))
}
