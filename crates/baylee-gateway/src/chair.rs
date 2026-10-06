//! A host's chair for a seat bridge: a language model at its table
//! (`docs/protocol.md` §"A host's chair for a seat bridge").
//!
//! A seat bridge (`baylee-seat join`) used to sit down as a guest, so a
//! gateway that takes no guests (`BAYLEE_GUESTS=off`), one whose guest cap
//! is reached, and a closed beta whose guest door needs a key seated none.
//! Now the host's client asks for a **chair ticket** for one open chair of
//! the room it hosts, and hands it to the bridge it starts on the bridge's
//! stdin. The bridge redeems it and sits in that chair as the host's
//! delegate: no account is made, the room lists the chair as the host's
//! (`delegated_by`), the game's record names the host as the one who
//! answers for it (`game_record_seat.delegated_by`), and the chair goes when
//! the host leaves the room or its account is deleted.
//!
//! A chair ticket is a [`wsticket`] grant in a store of its own:
//!
//! - **asked for** by a host signed in to an account (a guest may not hand
//!   a chair on: a guest is what the guest switches exist to bound), for an
//!   open person's chair of a waiting room it hosts;
//! - **Off where the operator says** (`BAYLEE_CHAIR_TICKETS=off`): neither
//!   asked for nor redeemed, `403` [`SWITCHED_OFF`];
//! - **only a language model's chair**: the delegate's name must carry
//!   [`DELEGATE_PREFIX`], so a host cannot seat a person on its word;
//! - **random** (256 bits), kept only as its SHA-256 and looked up by that
//!   hash, so no comparison ever runs over the secret itself;
//! - **single use**, spent the moment it is presented whether or not the
//!   chair is still there to take;
//! - **bound** to its room, its chair and its host: a ticket for chair 2
//!   opens no chair 3, and one whose host has left the room, handed it on or
//!   been deleted opens nothing (checked at the redemption, and the host's
//!   unspent tickets are dropped as it goes);
//! - **short-lived**: [`DEFAULT_SECS`] unless `BAYLEE_CHAIR_TICKET_SECS`
//!   says otherwise (1..=600);
//! - **never logged and never in an address**: the host gets it in a JSON
//!   answer, the bridge shows it as `Authorization: Bearer`;
//! - **rate-limited**: [`LIMIT_TRIES`] tickets per host and [`LIMIT_TRIES`]
//!   redemptions per address in [`LIMIT_WINDOW`], and at most
//!   [`wsticket::PER_HOLDER`] unspent per host.
//!
//! The bridge then has a seat token and nothing else, so its other calls
//! take that: [`ready`] (its mind answered its check, so the chair may
//! play; the room does not start before), [`status`] (is the game on, at
//! what pace) and [`leave`] (give the chair back before the game). The host takes the chair back
//! the way it arranges any chair (`POST …/seats/{seat}`, `kind`), which
//! empties it and ends its seat token.

use crate::{
    DeckBody, ErrorBody, Shared, TICKET_REFUSED, auth, authed_session, bearer_token, err, lobby,
    rate_limit_ip, seat_of_token, store, validate_deck, wsticket,
};
use axum::{
    Json,
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// How long an unspent chair ticket lives when `BAYLEE_CHAIR_TICKET_SECS`
/// says nothing: long enough for a client to start a bridge and the bridge
/// to choose its mind, short enough that a copy left behind is dead.
pub const DEFAULT_SECS: u64 = 120;

/// The window [`LIMIT_TRIES`] counts in.
pub const LIMIT_WINDOW: Duration = Duration::from_secs(60);

/// Chair tickets one host may ask for, and redemptions one address may
/// try, in [`LIMIT_WINDOW`]. A host seating eight chairs and changing its
/// mind about each a few times stays far below it.
pub const LIMIT_TRIES: usize = 30;

/// Why nobody is handed a chair ticket where the operator switched them
/// off (`BAYLEE_CHAIR_TICKETS=off`).
pub const SWITCHED_OFF: &str = "this gateway hands no chair to a seat bridge";

/// What a delegate's name must begin with: a host's word seats a language
/// model and nothing else, and the table must see that no person sits there
/// (the seat bridge's `Disclosure::Llm`). A host who could seat any name
/// could seat a friend under a person's name with no account, key or guest
/// seat.
pub const DELEGATE_PREFIX: &str = "LLM-";

/// Whether `name` may be a delegate's: the display-name rule, and the
/// language model's prefix with something after it.
#[must_use]
pub fn delegate_name(name: &str) -> bool {
    auth::valid_display_name(name)
        && name
            .strip_prefix(DELEGATE_PREFIX)
            .is_some_and(|rest| !rest.is_empty())
}

/// Why a guest is not handed a chair ticket.
pub const GUEST_REFUSED: &str =
    "a guest cannot hand a chair to a seat bridge; sign in with an account";

/// Asking for a seat bridge's chair (`POST /lobby/games/{id}/chairs/{seat}/ticket`),
/// as the room's host: `{ticket, expires_in}`.
pub(crate) async fn mint(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path((id, seat)): Path<(String, usize)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    if !state.chair_tickets_enabled {
        return Err(err(StatusCode::FORBIDDEN, SWITCHED_OFF));
    }
    let session = authed_session(&state, &headers).await?;
    if session.guest {
        return Err(err(StatusCode::FORBIDDEN, GUEST_REFUSED));
    }
    let host = session.account_id;
    if !state.chair_limiter.allow(&format!("mint:{host}")) {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many attempts"));
    }
    let ticket = hand_over(
        &state.lobby.lock(),
        &state.chair_tickets,
        &id,
        seat,
        host,
        Instant::now(),
    )?;
    Ok(Json(serde_json::json!({
        "ticket": ticket,
        "expires_in": state.chair_tickets.ttl().as_secs(),
    })))
}

/// Issues a ticket for chair `seat` of room `id`, from `host`, while the
/// lobby is held: checked and issued under one lock, so a host leaving the
/// room (which revokes its tickets after the lobby moved) can never come
/// between the check and the ticket and leave a ticket its revocation
/// missed. The lock order is the lobby's first, then the store's, and
/// nothing takes them the other way round.
fn hand_over(
    lobby: &lobby::Lobby,
    tickets: &wsticket::Tickets,
    id: &str,
    seat: usize,
    host: String,
    now: Instant,
) -> Result<String, (StatusCode, Json<ErrorBody>)> {
    let game = lobby
        .games
        .get(id)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
    if !game.hosted_by(&host) {
        return Err(err(
            StatusCode::FORBIDDEN,
            "only the host hands a chair to a seat bridge",
        ));
    }
    open_chair(game, seat)?;
    let grant = wsticket::Grant::Chair {
        game_id: id.to_string(),
        seat,
        host,
    };
    tickets.issue(grant, now).map_err(|wsticket::Full| {
        tracing::warn!("the chair ticket store is full");
        err(
            StatusCode::SERVICE_UNAVAILABLE,
            "too many chairs are being handed over",
        )
    })
}

/// The chair `seat` of `game` when a seat bridge could sit there now: a
/// person's chair nobody sits in, at a room still being arranged.
fn open_chair(
    game: &lobby::LobbyGame,
    seat: usize,
) -> Result<&lobby::LobbySeat, (StatusCode, Json<ErrorBody>)> {
    if game.state != lobby::LobbyState::Waiting {
        return Err(err(StatusCode::CONFLICT, "game already started"));
    }
    let chair = game
        .seats
        .get(seat)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such seat"))?;
    if chair.kind != lobby::SeatKind::Human || chair.occupied() {
        return Err(err(StatusCode::CONFLICT, "that seat is not open"));
    }
    Ok(chair)
}

/// What a seat bridge sits down with.
#[derive(Deserialize)]
pub(crate) struct RedeemBody {
    /// The name the chair sits under (`LLM-…`), under the display-name rule.
    display_name: String,
    /// The deck it plays, as `POST /decks` takes one. Kept with the chair
    /// and nowhere else: the bridge has no account to keep it under.
    deck: DeckBody,
}

/// A seat bridge sitting down on its host's ticket
/// (`POST /lobby/games/{id}/chairs/{seat}/redeem`, the ticket as
/// `Authorization: Bearer`): `{game_id, seat, seat_token, decide_secs}`.
///
/// Needs no session, so no guest account, no closed-beta key and no guest
/// seat: the host vouched for the chair.
pub(crate) async fn redeem(
    State(state): State<Shared>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path((id, seat)): Path<(String, usize)>,
    Json(body): Json<RedeemBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    if !state.chair_tickets_enabled {
        return Err(err(StatusCode::FORBIDDEN, SWITCHED_OFF));
    }
    let ip = rate_limit_ip(&state.trusted_proxies, addr.ip(), &headers);
    if !state.chair_limiter.allow(&format!("redeem:{ip}")) {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many attempts"));
    }
    // The body is checked before the ticket is spent: a malformed one says
    // nothing about the ticket, and the bridge that sent it may send a
    // better one with the same ticket.
    if !delegate_name(&body.display_name) {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "a seat bridge sits under a language model's name (LLM-…)",
        ));
    }
    validate_deck(&body.deck)?;
    let ticket =
        bearer_token(&headers).ok_or_else(|| err(StatusCode::UNAUTHORIZED, TICKET_REFUSED))?;
    let door = wsticket::Door::Chair {
        game_id: id.clone(),
        seat,
    };
    let grant = state
        .chair_tickets
        .consume(ticket, &door, Instant::now())
        .map_err(|why| {
            tracing::debug!(?why, "a chair ticket opened nothing");
            err(StatusCode::UNAUTHORIZED, TICKET_REFUSED)
        })?;
    let wsticket::Grant::Chair { host, .. } = grant else {
        return Err(err(StatusCode::UNAUTHORIZED, TICKET_REFUSED));
    };
    let deck = delegated_deck(&host, body.deck);
    let seat_token = auth::new_token();
    let decide_secs = {
        let mut lobby = state.lobby.lock();
        let game = lobby
            .games
            .get_mut(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        // The host's word holds while it hosts: one that left, handed the
        // room on or was deleted since vouches for nothing.
        if !game.hosted_by(&host) {
            return Err(err(
                StatusCode::FORBIDDEN,
                "the host who handed this chair over no longer hosts the room",
            ));
        }
        open_chair(game, seat)?;
        let decide_secs = game.house_rules.decision_timeout_secs;
        let chair = &mut game.seats[seat];
        chair.delegate = Some(lobby::Delegate {
            by: host,
            name: body.display_name,
            // Not before the bridge says its mind answered (`ready`).
            ready: false,
        });
        chair.deck_name.clone_from(&deck.name);
        chair.deck = Some(deck);
        chair.seat_token_hash = Some(auth::token_hash(&seat_token));
        decide_secs
    };
    state.lobby_moved();
    Ok(Json(serde_json::json!({
        "game_id": id,
        "seat": seat,
        "seat_token": seat_token,
        "decide_secs": decide_secs,
    })))
}

/// The deck a seat bridge brought, as the chair keeps it: in memory, in
/// the host's name (it answers for the chair), never in the database.
fn delegated_deck(host: &str, body: DeckBody) -> store::Deck {
    let commanders = body.commander_names();
    let format = body
        .format
        .unwrap_or_else(|| baylee_cards::decks::format_of(&commanders).to_string());
    store::Deck {
        id: auth::new_id(),
        account_id: host.to_string(),
        kind: baylee_db::entity::deck::KIND_ACCOUNT.to_string(),
        name: body.name,
        format,
        description: None,
        origin: None,
        version: 1,
        cards: body.cards,
        sideboard: body.sideboard,
        commanders,
        // No pictures: a bridge has no account that could have uploaded
        // one (`check_pictures`).
        sleeve: None,
        playmat: None,
        updated_at: auth::now_secs(),
        offered: true,
    }
}

/// The chair a request's seat token holds at `id`: the game's lobby entry
/// and the chair's number.
fn by_seat_token<'a>(
    lobby: &'a mut lobby::Lobby,
    id: &str,
    headers: &HeaderMap,
) -> Result<(&'a mut lobby::LobbyGame, usize), (StatusCode, Json<ErrorBody>)> {
    let token = bearer_token(headers)
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "missing bearer token"))?;
    let game = lobby
        .games
        .get_mut(id)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
    let seat = seat_of_token(game, token)
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "invalid seat token"))?;
    Ok((game, seat))
}

/// A chair's own view of its room (`GET /lobby/games/{id}/chair`, the seat
/// token as `Authorization: Bearer`): `{game_id, seat, state, decide_secs}`.
/// What a seat bridge with no session waits for the start with, and asks
/// whether its room has closed.
pub(crate) async fn status(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let mut lobby = state.lobby.lock();
    let (game, seat) = by_seat_token(&mut lobby, &id, &headers)?;
    Ok(Json(serde_json::json!({
        "game_id": id,
        "seat": seat,
        "state": match game.state {
            lobby::LobbyState::Waiting => "waiting",
            lobby::LobbyState::Playing => "playing",
            lobby::LobbyState::Over => "over",
        },
        "decide_secs": game.house_rules.decision_timeout_secs,
        "ready": game.seats[seat].ready(),
    })))
}

/// What a seat bridge says of its chair (`POST …/chair/ready`).
#[derive(Deserialize)]
pub(crate) struct ReadyBody {
    /// Absent means ready.
    #[serde(default = "yes")]
    ready: bool,
}

/// `serde` default for [`ReadyBody::ready`].
const fn yes() -> bool {
    true
}

/// A host's seat bridge saying its mind answered its check and the chair
/// may play (`POST /lobby/games/{id}/chair/ready`, the seat token as
/// `Authorization: Bearer`). Until it does, the room does not start: a
/// chair whose model cannot answer would lose its game to the clock.
pub(crate) async fn ready(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<ReadyBody>,
) -> Result<StatusCode, (StatusCode, Json<ErrorBody>)> {
    {
        let mut lobby = state.lobby.lock();
        let (game, seat) = by_seat_token(&mut lobby, &id, &headers)?;
        if game.state != lobby::LobbyState::Waiting {
            return Err(err(StatusCode::CONFLICT, "game already started"));
        }
        let Some(delegate) = game.seats[seat].delegate.as_mut() else {
            return Err(err(
                StatusCode::FORBIDDEN,
                "a player says ready with their session, not their seat token",
            ));
        };
        delegate.ready = body.ready;
    }
    state.lobby_moved();
    Ok(StatusCode::NO_CONTENT)
}

/// A host's seat bridge giving its chair back before the game
/// (`POST /lobby/games/{id}/chair/leave`, the seat token as
/// `Authorization: Bearer`). A person leaves with their session
/// (`POST …/leave`); this is the delegate's only way out.
pub(crate) async fn leave(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorBody>)> {
    {
        let mut lobby = state.lobby.lock();
        let (game, seat) = by_seat_token(&mut lobby, &id, &headers)?;
        if game.state != lobby::LobbyState::Waiting {
            return Err(err(StatusCode::CONFLICT, "game already started"));
        }
        let chair = &mut game.seats[seat];
        if chair.delegate.is_none() {
            return Err(err(
                StatusCode::FORBIDDEN,
                "a player leaves with their session, not their seat token",
            ));
        }
        chair.vacate();
    }
    state.lobby_moved();
    Ok(StatusCode::NO_CONTENT)
}

/// Drops the unspent chair tickets `host` handed out for room `id`: it left
/// the room or handed it on, and vouches for no chair there any more.
pub(crate) fn revoke(state: &Shared, id: &str, host: &str) {
    state.chair_tickets.revoke(|grant| {
        matches!(
            grant,
            wsticket::Grant::Chair { game_id, host: by, .. } if game_id == id && by == host
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(host: &str) -> lobby::Lobby {
        let mut game = lobby::LobbyGame::blank("g1".into(), 0);
        game.host = Some(host.into());
        game.seats = (0..3).map(lobby::LobbySeat::open).collect();
        game.seats[0].account_id = Some(host.into());
        let mut lobby = lobby::Lobby::default();
        lobby.games.insert("g1".into(), game);
        lobby
    }

    /// Checked and issued under one hold of the lobby: a ticket exists only
    /// for a host who hosts the room at that moment, and a revocation run
    /// after the host left finds every ticket it handed out.
    #[test]
    fn a_ticket_is_issued_only_under_the_check_and_a_revocation_finds_it() {
        let tickets = wsticket::Tickets::new(Duration::from_secs(DEFAULT_SECS));
        let now = Instant::now();
        let mut lobby = room("host");
        assert!(hand_over(&lobby, &tickets, "g1", 1, "stranger".into(), now).is_err());
        assert!(
            hand_over(&lobby, &tickets, "g1", 0, "host".into(), now).is_err(),
            "taken"
        );
        assert!(hand_over(&lobby, &tickets, "g9", 1, "host".into(), now).is_err());
        assert_eq!(
            tickets.sweep(now + Duration::from_secs(DEFAULT_SECS)),
            0,
            "none issued"
        );
        let Ok(ticket) = hand_over(&lobby, &tickets, "g1", 1, "host".into(), now) else {
            panic!("issued");
        };
        // The host leaves: the lobby moves first, then its tickets go.
        lobby.games.get_mut("g1").unwrap().host = None;
        assert!(hand_over(&lobby, &tickets, "g1", 2, "host".into(), now).is_err());
        let revoked = tickets
            .revoke(|grant| matches!(grant, wsticket::Grant::Chair { host, .. } if host == "host"));
        assert_eq!(revoked, 1);
        let door = wsticket::Door::Chair {
            game_id: "g1".into(),
            seat: 1,
        };
        assert_eq!(
            tickets.consume(&ticket, &door, now),
            Err(wsticket::Refusal::Unknown)
        );
    }

    #[test]
    fn a_delegate_is_called_a_language_model() {
        for good in ["LLM-sonnet-5-5", "LLM-test", "LLM-x"] {
            assert!(delegate_name(good), "{good}");
        }
        for bad in [
            "House-AI",
            "HOUSE-house",
            "TEST-scripted",
            "Alice",
            "LLM-",
            "llm-test",
            "LLM- x",
        ] {
            assert!(!delegate_name(bad), "{bad}");
        }
    }
}
