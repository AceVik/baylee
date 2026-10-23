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
/// @openapi
#[derive(Deserialize)]
pub(crate) struct ReplayPage {
    /// @description How many games to skip, from the newest; 0 when absent.
    #[serde(default)]
    offset: u64,
    /// @description How many games to list: 20 when absent, at most 50.
    limit: Option<u64>,
    /// @description `true` lists the games the account archived instead of
    /// the others.
    /// @internal
    /// The archived games instead of the others.
    #[serde(default)]
    archived: bool,
}

/// One seat of a listed game.
/// @openapi
#[derive(Serialize)]
struct ReplaySeat {
    /// @description The seat's number.
    seat: u8,
    /// @description Who sat there, to the asking account: `you`, `player`,
    /// `former` (a player whose account is gone), `house` (the house AI) or
    /// `model` (a seat bridge).
    /// @internal
    /// `you`, `player`, `former`, `house` or `model`.
    /// @openapi enum: [you, player, former, house, model]
    who: &'static str,
    /// @description What the seat is called: a player's handle (`Name#tag`),
    /// `House AI`, `Language model` or `Former player`.
    name: String,
    /// @description The deck's colour identity as `WUBRG` letters, for the
    /// asking account's own seat only.
    /// @internal
    /// The deck's colours, for the asking account's own seat only.
    #[serde(skip_serializing_if = "Option::is_none")]
    colours: Option<String>,
    /// @description The deck's commander as a `CardIndex`, `null` for none;
    /// public at the table.
    commander: Option<u32>,
    /// @description Whether the seat's hidden cards may be shown in this
    /// game's replays: its player shared them (or did not say, in a game
    /// recorded from the beta.7 deploy on), or it is an AI seat.
    /// @internal
    /// Whether the seat's hidden cards may be shown in this game's replays:
    /// its player shared them, or it is an AI seat.
    shared: bool,
    /// @description Whether the asking account played this seat.
    mine: bool,
}

/// One listed game.
/// @openapi
#[derive(Serialize)]
struct ReplayGame {
    /// @description The game's id.
    game: String,
    /// @description When it started (RFC 3339, UTC).
    /// @openapi format: date-time
    started_at: String,
    /// @description When its record ended (RFC 3339, UTC), `null` while it
    /// has not.
    /// @openapi format: date-time
    ended_at: Option<String>,
    /// @description Whether its record is complete.
    complete: bool,
    /// @description The seat the asking account played.
    seat: u8,
    /// @description How many turns it lasted, `null` when unknown.
    turn: Option<u32>,
    /// @description How it went for the asking account's seat: `won`,
    /// `lost`, `draw`, `unfinished` (the record has no end) or `unknown` (a
    /// game that ended before the gateway noted how).
    /// @internal
    /// `won`, `lost`, `draw`, `unfinished`, or `unknown` for a game that
    /// ended before the gateway noted how.
    /// @openapi enum: [won, lost, draw, unfinished, unknown]
    result: &'static str,
    /// @description Whether the account may ask for the public perspective;
    /// every seated account may.
    public: bool,
    /// @description Whether the asking account archived it.
    archived: bool,
    /// @description How far today's build is known to replay it as played:
    /// `exact` (the same build), `checked` (checked against what the table
    /// showed) or `unverified` (ask its tape with `verify: note`).
    /// @internal
    /// `exact`, `checked` or `unverified`.
    /// @openapi enum: [exact, checked, unverified]
    check: &'static str,
    /// @description The build its engine was, `null` when not recorded.
    build: Option<String>,
    /// @description Every seat at the table.
    seats: Vec<ReplaySeat>,
}

/// `GET /replays`'s answer.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ReplayListing {
    /// @description One page of the games, newest first.
    games: Vec<ReplayGame>,
    /// @description How many games the whole list holds.
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

fn game_out(game: SeatedGame, me: Uuid, archived: bool) -> ReplayGame {
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
            ReplaySeat {
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
    ReplayGame {
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
///
/// @route GET /replays
/// @scope public
/// @summary The hosted games the account sat at
/// @description The games on this gateway the signed-in account played a
/// seat of (guests too), newest first, a page at a time: the ones it has
/// not archived, or with `archived=true` only the ones it has. Each says
/// how it went, who sat where, and how far this build can replay it.
/// @tag Replays
/// @security bearer
/// @insert `SessionRefusals`
/// @query-params `$ReplayPage`
/// @return 400: String text/plain "A query value of the wrong type (axum's rejection)"
pub(crate) async fn list_replays(
    State(state): State<Shared>,
    headers: HeaderMap,
    Query(page): Query<ReplayPage>,
) -> Result<Json<ReplayListing>, Refusal> {
    let session = crate::authed_session(&state, &headers).await?;
    let me = account_of(&session)?;
    let limit = page.limit.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE);
    let (games, total) =
        baylee_db::records::seated_games(&state.db, me, page.archived, page.offset, limit)
            .await
            .map_err(|e| crate::db_down(&anyhow::Error::from(e)))?;
    Ok(Json(ReplayListing {
        games: games
            .into_iter()
            .map(|g| game_out(g, me, page.archived))
            .collect(),
        total,
    }))
}

/// Whose view a tape is asked for.
///
/// @description Whose view the tape shows: `seat` (what a seat was sent),
/// `public` (what spectators were sent) or `all_hands` (a seat's view and
/// every shown seat's hand).
/// @openapi
#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum TapePerspective {
    Seat,
    Public,
    AllHands,
}

/// What a check that fails does.
///
/// @description What a replay that departs from the record does: `stop`
/// there, or `note` it and go on unverified.
/// @openapi
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
enum TapeVerify {
    #[default]
    Stop,
    Note,
}

/// `POST /replays/{game}/tape`'s body. `seat` is the seat the table is
/// drawn from (default: the asking account's own); whose hidden cards the
/// tape may hold the gateway decides from consent, never the request.
///
/// @description What tape to make. Whose hidden cards it may hold the
/// gateway decides from each seat's consent, never the request; any other
/// field is refused.
/// @openapi
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TapeAsk {
    /// @description Whose view the tape shows.
    perspective: TapePerspective,
    /// @description The seat the table is drawn from, the asking account's
    /// own when absent; ignored for `public`.
    #[serde(default)]
    seat: Option<u8>,
    /// @description What a departure from the record does; `stop` when
    /// absent.
    #[serde(default)]
    verify: TapeVerify,
}

/// What a tape shows, decided here from the table and each seat's consent,
/// never from the request (`docs/protocol.md` §"Replays").
struct Lens {
    seat: u8,
    perspective: baylee_protocol::v1::project_record::Perspective,
    own: bool,
    hands: Vec<u8>,
    others: Vec<u8>,
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
            TapePerspective::Seat => Perspective::Seat,
            TapePerspective::Public => Perspective::Public,
            TapePerspective::AllHands => Perspective::AllHands,
        };
        if perspective == Perspective::Public {
            return Ok(Self {
                seat: 0,
                perspective,
                own: false,
                hands: Vec::new(),
                others: Vec::new(),
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
                shown.iter().copied().filter(|&s| s != seat).collect()
            } else {
                Vec::new()
            },
            // My own seat is shown whole, what it was shown of others
            // included; another seat's view keeps only what it was shown of
            // seats in S.
            others: if seat == mine {
                seats
                    .iter()
                    .map(|s| s.seat)
                    .filter(|&s| s != seat)
                    .collect()
            } else {
                shown.into_iter().filter(|&s| s != seat).collect()
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
///
/// @route POST /replays/{game}/tape
/// @scope public
/// @summary Play a game again as a tape
/// @description An engine plays the game again on this build from its
/// record and answers what the asked perspective was sent, as gzip JSON
/// Lines (`baylee_protocol::tape`): a header, the frames, and a closing
/// `stop` line saying how far it is the recorded game. A tape that stops
/// early is still `200`. For an account that sat at the game; a client
/// retries a `502` once.
/// @tag Replays
/// @security bearer
/// @insert `SessionRefusals`
/// @no-infer
/// @path-param game: String "The game's id"
/// @body `$TapeAsk`
/// @return 200: { type: string, format: binary } application/gzip
/// @return 400: `$ErrorBody` "The body is not a tape request, or names a seat the table lacks"
/// @return 403: `$ErrorBody` "The account sat at no seat of that game; or it is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No record of that game"
/// @return 409: `$ErrorBody` "The game is still being played"
/// @return 413: `$ErrorBody` "The game's record is over 8 mebibytes compressed"
/// @return 429: `$ErrorBody` "More than 6 tapes a minute, or 2 at once, for the account"
/// @return 502: `$ErrorBody` "The tape passed 32 mebibytes compressed, or its engine failed or went away"
/// @return 503: `$ErrorBody` "Replays are off (`BAYLEE_REPLAY_JOBS=0`), as many are being made as the gateway allows, no agent has room, admission is held, or the account store or Baylee Central is unavailable"
/// @return 504: `$ErrorBody` "No tape within the job's time (`BAYLEE_REPLAY_JOB_SECS`, 60 s by default)"
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
            others: lens.others,
            note: ask.verify == TapeVerify::Note,
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
/// @openapi
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReplayShareAsk {
    /// @description Whether the other players may see this seat's hidden
    /// cards in the game's replays.
    shared: bool,
}

/// `PUT /replays/{game}/share`'s answer.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ReplayShare {
    /// @description The asking account's seat at the game.
    seat: u8,
    /// @description Whether its hidden cards are shown in the game's
    /// replays now.
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
///
/// @route PUT /replays/{game}/share
/// @scope public
/// @summary Share a seat's hidden cards in replays
/// @description Sets whether the other players may see the asking
/// account's own seat's hidden cards in that game's replays. The seat is
/// the one the table says the account played. Any time, also while the
/// game is played.
/// @tag Replays
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param game: String "The game's id"
/// @body `$ReplayShareAsk`
/// @return 400: `$ErrorBody` "The body is not a sharing choice"
/// @return 403: `$ErrorBody` "The account sat at no seat of that game; or it is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No record of that game"
pub(crate) async fn share_replay(
    State(state): State<Shared>,
    Path(game_id): Path<String>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<ReplayShare>, Refusal> {
    let session = crate::authed_session(&state, &headers).await?;
    let me = account_of(&session)?;
    let ask: ReplayShareAsk = serde_json::from_slice(&body)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "the body is not a sharing choice"))?;
    let seat = own_seat(&state, &game_id, me).await?;
    baylee_db::records::set_share(&state.db, &game_id, seat, ask.shared)
        .await
        .map_err(|e| crate::db_down(&anyhow::Error::from(e)))?;
    Ok(Json(ReplayShare {
        seat,
        shared: ask.shared,
    }))
}

/// `PUT /replays/{game}/archived`'s body and answer.
/// @openapi
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReplayArchived {
    /// @description Whether the game is on the account's archived list.
    archived: bool,
}

/// `PUT /replays/{game}/archived` `{"archived": bool}`: moves the game to
/// the asking account's archived list (`GET /replays?archived=true`), or
/// back. The record is never deleted (it trains the house AI); nothing else
/// changes, and an archived game's tapes are answered as before.
///
/// @route PUT /replays/{game}/archived
/// @scope public
/// @summary Archive a game, or bring it back
/// @description Moves the game to the asking account's archived list
/// (`GET /replays?archived=true`) or back, and nothing else: the record is
/// kept, other players' lists stay as they are, and its tapes are answered
/// as before. Answers the choice, also when it already was so.
/// @tag Replays
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param game: String "The game's id"
/// @body `$ReplayArchived`
/// @return 400: `$ErrorBody` "The body is not an archive choice"
/// @return 403: `$ErrorBody` "The account sat at no seat of that game; or it is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No record of that game"
pub(crate) async fn archive_replay(
    State(state): State<Shared>,
    Path(game_id): Path<String>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<ReplayArchived>, Refusal> {
    let session = crate::authed_session(&state, &headers).await?;
    let me = account_of(&session)?;
    let ask: ReplayArchived = serde_json::from_slice(&body)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "the body is not an archive choice"))?;
    own_seat(&state, &game_id, me).await?;
    baylee_db::records::set_archived(&state.db, me, &game_id, ask.archived)
        .await
        .map_err(|e| crate::db_down(&anyhow::Error::from(e)))?;
    Ok(Json(ask))
}
