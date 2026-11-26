//! baylee-gateway — accounts, decks, lobby, and hosted games.
//!
//! Security posture (see auth.rs): no account password is taken or kept
//! here (accounts sign in at Baylee Central), hashed bearer tokens with
//! sliding expiry, rate limiting on the public doors, generic credential
//! errors, constant-time comparisons. TLS terminates at the
//! reverse proxy in front of this process (Caddy/nginx) — this service
//! must never be exposed on a plaintext listener in production, and it
//! listens on loopback unless `BAYLEE_BIND` names another address
//! (`launch.rs`).
#![deny(unsafe_code)]

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_cards_pool as _;

use baylee_protocol::refusal::Refusal as WireRefusal;

mod account;
mod admin;
mod admission;
mod art;
mod auth;
mod authmode;
mod banlists;
mod blocks;
mod central;
mod chair;
mod challenges;
mod clock;
mod cosmetics;
mod departure;
mod engine;
mod handle;
mod keep;
mod launch;
mod llmaccess;
mod lobby;
mod metrics;
mod moderation;
mod namebook;
mod openapi;
mod pool;
mod poolfacts;
mod presence;
mod ranked;
mod record;
mod recordexport;
mod registration;
mod replays;
mod report;
mod room;
mod routes;
mod seathost;
mod seatrate;
mod sessions;
mod sets;
mod social;
mod store;
mod terms;
mod termsadmin;
mod texts;
mod tower;
mod wsticket;

use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use axum::routing::{delete, get, post, put};
use baylee_protocol::v1::{self, Envelope};
use lobby::{Lobby, LobbyGame, LobbyState};
use parking_lot::Mutex;
use routes::{
    DeckBody, LobbyWsParams, MAX_SEATS, TICKET_REFUSED, WsParams, account_routes, ai_preset,
    archive_replay, auth_config, bearer_token, catalog_search, catalog_text, check_pictures,
    connect_catalog, create_game, deck_routes, display_name, expand, game_cosmetics, game_ws,
    get_settings, hand_over, health, house_deck, impressum_url, info, is_bidi_control, join_game,
    leave_game, legacy_token, list_games, list_replays, list_seats, listing, listing_page,
    lobby_stats, lobby_ws, parse_deck_lines, put_settings, rematch, replay_tape, seat_guests,
    seat_names, seat_of_token, set_ready, set_seat, share_replay, source, source_url, spend_ticket,
    start_room, table_prints, take_seat, try_start, validate_deck, validate_the_dev_board,
    watch_ws, ws_ticket,
};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use store::Deck;
use tokio::sync::broadcast::error::RecvError;
use tracing_subscriber::EnvFilter;

mod app;
use app::AppState;

type Shared = Arc<AppState>;

mod bootstrap;

#[tokio::main]
async fn main() {
    bootstrap::run().await;
}

/// Serves the same routes on a unix socket as well (`BAYLEE_UNIX_SOCKET`).
///
/// For the agent on this machine and the engines it starts: a socket file
/// the operating system guards by owner and group, not a port anybody on the
/// host could dial. Whatever arrives here is marked [`engine::ViaUnix`], and
/// that mark is what "local" means to `/health` and to a deploy: the games
/// this machine's agent runs, as opposed to an agent somewhere else.
///
/// A peer on a unix socket has no address, and the routes that limit by
/// address still ask for one, so every request here reads as loopback. It
/// is: the file is only reachable from this host.
///
/// A file left behind by an earlier run is removed first, since binding over
/// it fails; the new one is readable and writable by owner and group only
/// (0660).
#[cfg(unix)]
fn serve_unix(path: &std::path::Path, app: Router) {
    use std::os::unix::fs::PermissionsExt as _;
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => panic!("BAYLEE_UNIX_SOCKET {}: {e}", path.display()),
    }
    let listener = tokio::net::UnixListener::bind(path)
        .unwrap_or_else(|e| panic!("BAYLEE_UNIX_SOCKET {}: {e}", path.display()));
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o660))
        .unwrap_or_else(|e| panic!("BAYLEE_UNIX_SOCKET {}: {e}", path.display()));
    let loopback = SocketAddr::from(([127, 0, 0, 1], 0));
    let app = app
        .layer(axum::Extension(ConnectInfo(loopback)))
        .layer(axum::Extension(engine::ViaUnix));
    tracing::info!(path = %path.display(), "baylee-gateway listening on a unix socket");
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("gateway serves its unix socket");
    });
}

/// The gateway's TCP listener: every connection it accepts sends with
/// Nagle's algorithm off ([`send_at_once`]).
type Listening = axum::serve::TapIo<tokio::net::TcpListener, fn(&mut tokio::net::TcpStream)>;

/// Binds the gateway's port on `bind` (`BAYLEE_BIND`, loopback unless an
/// operator names another address, [`launch::bind_address`]), and answers
/// where it is listening: the port asked for, or with `PORT=0` the one the
/// kernel chose.
async fn listen(bind: IpAddr, port: u16) -> (Listening, SocketAddr) {
    use axum::serve::ListenerExt as _;
    let listener = tokio::net::TcpListener::bind((bind, port))
        .await
        .unwrap_or_else(|e| panic!("bind gateway port {}: {e}", SocketAddr::new(bind, port)));
    let address = listener
        .local_addr()
        .expect("a bound listener has an address");
    (listener.tap_io(send_at_once as fn(&mut _)), address)
}

/// Turns Nagle's algorithm off on an accepted connection (`TCP_NODELAY`).
///
/// The gateway relays frames that are small and follow each other closely:
/// an engine's view and then the question about it, forwarded to a seat. With
/// Nagle on, the second waits until the peer acknowledges the first, and
/// Linux holds an acknowledgement back for up to 40 ms: in a Linux container
/// the language-model seat's test game (`e2e_seat_llm`) took 49 ms an action
/// with Nagle on and 5 ms with it off (30.09.2026).
fn send_at_once(stream: &mut tokio::net::TcpStream) {
    if let Err(e) = stream.set_nodelay(true) {
        tracing::debug!(error = %e, "TCP_NODELAY was not set on a connection");
    }
}

/// Says the gateway is about to serve, and where: in the log, and its port
/// to `BAYLEE_PORT_FILE` when one is named.
fn announce(address: SocketAddr) {
    let port = address.port();
    tracing::info!(%address, port, "baylee-gateway listening");
    if let Some(path) = std::env::var_os("BAYLEE_PORT_FILE") {
        write_port_file(std::path::Path::new(&path), port)
            .unwrap_or_else(|e| panic!("BAYLEE_PORT_FILE {}: {e}", path.display()));
    }
}

/// Writes the port the gateway is listening on to `path`, for whoever
/// started it with `PORT=0` and needs to know where it went (#279).
///
/// The e2e suite is that caller. It used to pick a port by binding
/// `127.0.0.1:0`, reading the number and letting go, and the gateway bound
/// `0.0.0.0` on that number seconds later, after its database was up. Any
/// other process could take the port in that gap, and with several worktrees
/// gating at once one did, often enough to fail two feature gates in three.
/// Binding port 0 here and saying which port it was leaves no gap.
///
/// Written beside the target and renamed over it, so a reader polling for
/// the file never reads half a number. Called just before the gateway
/// serves, so the file's appearance also says it is past its database and
/// catalog.
fn write_port_file(path: &std::path::Path, port: u16) -> std::io::Result<()> {
    let partial = path.with_extension("partial");
    std::fs::write(&partial, format!("{port}\n"))?;
    std::fs::rename(&partial, path)
}

/// Open the one connection pool the gateway has, and take over a store file
/// if this is the first start against an empty database.
///
/// # Why this is fatal and the catalog is not
///
/// A gateway with no card catalog serves no card text and plays every game;
/// a gateway with no database has no accounts, so there is nothing it can do
/// but refuse, and refusing at startup is the only place it can do so
/// honestly. The alternative — answering every request with a 503 — is a
/// process that looks healthy to anything watching it.
async fn open_database(store_path: &std::path::Path) -> sea_orm::DatabaseConnection {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| {
            panic!(
                "DATABASE_URL is not set; the gateway keeps its accounts in PostgreSQL.\n  \
                 docker compose up -d\n  \
                 export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee"
            )
        });
    // Small by default and settable, because the number that binds is at the
    // other end: a stock PostgreSQL allows 100 connections in total, and a
    // test suite that spawns three dozen gateways has to fit inside that.
    let pool = std::env::var("BAYLEE_DB_POOL")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(baylee_db::DEFAULT_POOL);

    let db = match baylee_db::connect(&url, pool).await {
        Ok(db) => db,
        Err(e) => panic!("{e:#}"),
    };
    tracing::info!(url = %baylee_db::redacted(&url), pool, "database ready");

    // The precons this build plays, offered beside the house decks
    // (`docs/precons.md` §"House decks"). Not fatal, as the import below is
    // not: a sync that fails leaves the offer as the last one left it, and
    // every account and deck is still there to serve.
    match baylee_db::precons::sync(&db, baylee_db::precons::PLAYABLE).await {
        Ok(done) if done.unchanged => {
            tracing::info!(offered = done.offered, "precons unchanged");
        }
        Ok(done) => tracing::info!(
            offered = done.offered,
            added = done.added,
            changed = done.changed,
            returned = done.returned,
            withdrawn = done.withdrawn,
            "precons synced"
        ),
        Err(e) => tracing::error!("precon sync: {e:#}"),
    }

    match baylee_db::import::import_file(&db, store_path).await {
        Ok(Some(done)) => {
            tracing::info!(
                accounts = done.accounts,
                decks = done.decks,
                sessions = done.tokens,
                orphans = done.orphans,
                "imported {}",
                store_path.display()
            );
            // Moved, never deleted: until the database has been backed up
            // once, this file is the only copy of somebody's account.
            let aside = baylee_db::import::imported_name(store_path);
            if let Err(e) = std::fs::rename(store_path, &aside) {
                tracing::warn!(%e, "could not move {} aside", store_path.display());
            }
        }
        Ok(None) => {}
        Err(e) => tracing::error!("{e:#}"),
    }

    db
}

/// Periodically reclaims finished games (after a grace period), stale
/// waiting lobbies, expired sessions and the guests they were the way into,
/// and expired confirmation links (#293) — all of them grew without bound.
fn spawn_cleanup(state: Shared) {
    /// How long a finished game stays joinable for reconnect/review.
    const OVER_GRACE_SECS: u64 = 3600;
    /// How long an unattended waiting lobby stays open.
    const WAITING_TIMEOUT_SECS: u64 = 2 * 3600;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(600));
        interval.tick().await;
        loop {
            interval.tick().await;
            let now = auth::now_secs();
            {
                let mut lobby = state.lobby.lock();
                // A finished game whose rematch room is still waiting has to
                // outlive its grace period: the pointer to that room is the
                // only thing that says it exists, and reaping it would send
                // the player who takes a moment longer to press the button
                // into a second room. Collected first, because `retain`
                // cannot look a game up in the map it is emptying.
                let awaited: std::collections::HashSet<String> = lobby
                    .games
                    .values()
                    .filter(|g| g.state == LobbyState::Waiting)
                    .filter_map(|g| g.parent.clone())
                    .collect();
                // A game reaching its end is no longer something to discover
                // here: the engine says so on its own link, and losing that
                // link closes the game too. All that is left is reclaiming.
                lobby.games.retain(|_, g| match g.state {
                    LobbyState::Waiting => now.saturating_sub(g.created_at) < WAITING_TIMEOUT_SECS,
                    LobbyState::Over => {
                        awaited.contains(&g.id)
                            || g.finished_at
                                .is_none_or(|t| now.saturating_sub(t) < OVER_GRACE_SECS)
                    }
                    LobbyState::Playing => true,
                });
            }
            // One `DELETE ... WHERE expires_at <= $1` on the index the
            // migration made for it, where this used to walk every session
            // the gateway had ever issued.
            match store::sweep_tokens(&state.db, now).await {
                Ok(purged) if purged > 0 => tracing::debug!(purged, "lapsed sessions swept"),
                Ok(_) => {}
                Err(e) => tracing::warn!("{e:#}"),
            }
            match store::sweep_confirmations(&state.db, now).await {
                Ok(swept) if swept > 0 => {
                    tracing::debug!(swept, "expired confirmation links swept");
                }
                Ok(_) => {}
                Err(e) => tracing::warn!("{e:#}"),
            }
            // After the sessions, because a guest goes with its last one
            // (#269), and out of the lobby with it (#292).
            match account::depart(&state, store::purge_guests(&state.db, now, None)).await {
                Ok(gone) if !gone.accounts.is_empty() => {
                    tracing::info!(purged = gone.accounts.len(), "idle guests deleted");
                }
                Ok(_) => {}
                Err(e) => tracing::warn!("{e:#}"),
            }
            social::sweep::sweep(&state).await;
        }
    });
}

/// Drops expired socket tickets (#294) once a minute.
///
/// Not the ten-minute loop above: a ticket lives 45 seconds by default, and
/// what this bounds is how long a dead one takes up room. The store is
/// bounded without it ([`wsticket::MAX_OUTSTANDING`], and it sweeps itself
/// when full); this keeps it small in the ordinary case.
fn spawn_ticket_sweep(state: Shared) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.tick().await;
        loop {
            interval.tick().await;
            let now = std::time::Instant::now();
            central::sweep(&state);
            let swept = state.tickets.sweep(now) + state.chair_tickets.sweep(now);
            if swept > 0 {
                tracing::debug!(swept, "expired socket tickets swept");
            }
        }
    });
}

/// Ends the sessions that ran out, so an idle socket closes on time and the
/// book stays the size of what is live. Once a second, because this is the
/// only thing that wakes a socket with nothing to say; a frame needs no sweep,
/// since [`sessions::Live::stands`] reads the clock itself.
fn spawn_session_sweep(state: Shared) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            state.sessions.sweep(sessions::now());
        }
    });
}

// ------------------------------------------------------------------- auth

/// A refusal.
///
/// @description A refusal: one English sentence in `error`, and the parts
/// a client words itself where a refusal has them.
/// @openapi
#[derive(Serialize)]
struct ErrorBody {
    /// @description What went wrong, as one sentence for a person.
    /// @internal
    /// Borrowed for the constant messages, owned for the few that have to
    /// name a value back to the caller.
    ///
    /// `Cow` rather than `String` everywhere: almost every refusal in this
    /// file is a fixed sentence, and making them all allocate to serve the
    /// handful that cannot would be paying for the exception on every path.
    error: std::borrow::Cow<'static, str>,
    /// The ban an account is refused with (beta.8): only on a `403` from a
    /// Central sign-in or a session of a banned account (`moderation.rs`).
    #[serde(skip_serializing_if = "Option::is_none")]
    ban: Option<moderation::BanBody>,
    /// Whose terms of use the account has to accept first (`gateway`, the only
    /// ones a gateway enforces): only on the `403` of a session that has not (`terms_admit`).
    /// @openapi enum: [gateway]
    #[serde(skip_serializing_if = "Option::is_none")]
    terms_required: Option<&'static str>,
    /// Why a table's list turned a deck away: only on the `409` of a route
    /// that sat a deck down, and only to the player whose deck it is.
    #[serde(skip_serializing_if = "Option::is_none")]
    refused: Option<Box<baylee_cardtext::DeckRefusal>>,
    /// Why, for a client to say in its own words (`docs/protocol.md`
    /// §"Refusals"): only on a refusal that has a stable reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    refusal: Option<Box<WireRefusal>>,
}

fn err(status: StatusCode, message: &'static str) -> (StatusCode, Json<ErrorBody>) {
    (
        status,
        Json(ErrorBody {
            error: std::borrow::Cow::Borrowed(message),
            ban: None,
            terms_required: None,
            refused: None,
            // One constant, whichever route met the hold.
            refusal: (message == admission::HELD)
                .then(|| Box::new(baylee_protocol::refusal::Refusal::new("maintenance"))),
        }),
    )
}

/// The same, for a refusal that has to quote what the caller sent.
///
/// A message that says only "bad clock" makes the caller guess; one that
/// names the clocks there are answers the request and the next one.
fn err_saying(status: StatusCode, message: String) -> (StatusCode, Json<ErrorBody>) {
    (
        status,
        Json(ErrorBody {
            error: std::borrow::Cow::Owned(message),
            ban: None,
            terms_required: None,
            refused: None,
            refusal: None,
        }),
    )
}

/// Whether a switch is on: whether hosts may hand chairs to seat bridges
/// (`BAYLEE_CHAIR_TICKETS`), as `BAYLEE_GUESTS` was read until the auth
/// mode replaced it. `BAYLEE_REGISTRATION` has a reader of its own
/// ([`registration::Registration::from_env`]), which refuses the closed
/// beta's `invite` and reads the rest the same way, so that an operator
/// still learns one reading.
///
/// **Three spellings shut the door and every other value leaves it open**,
/// which is the wrong way round for the one switch an operator reaches for
/// when they want it shut: `no`, `OFF` and a variable exported empty all
/// read as on, and nothing says so at the moment it is set.
///
/// Left as it stands rather than widened on a guess. Accepting more words
/// and refusing the ones it does not know are different decisions, and the
/// second is the one this workspace took next door — a `dev-table` board
/// specification that does not resolve refuses to start the gateway rather
/// than failing at the moment somebody presses Start. The day either is
/// taken, the test named after this sentence is what changes.
fn switched_on(raw: Option<&str>) -> bool {
    !matches!(raw, Some("off" | "0" | "false"))
}

/// How many guests a gateway takes when its operator says nothing.
const DEFAULT_GUEST_CAP: u64 = 1000;

/// How many guests one address may make in a day.
const GUESTS_A_DAY_PER_ADDRESS: usize = 30;

/// A guest that is never used lasts this many days, so this many days of one
/// address's guests must stay under [`DEFAULT_GUEST_CAP`].
const _: () = assert!(GUESTS_A_DAY_PER_ADDRESS as u64 * 30 < DEFAULT_GUEST_CAP);

/// How many guests there may be at once (`BAYLEE_GUEST_CAP`, #269), or
/// `None` for no bound.
///
/// The per-address limiter bounds how fast one address makes guests, and
/// nothing about how many a crowd of addresses makes; this bounds the
/// accounts nobody registered. Unset is a thousand, `0` is no bound, and
/// anything else that is not a count stops the gateway starting, as a bad
/// name does: a cap that read a typo as "no cap" would be found out by the
/// disk.
fn guest_cap(raw: Option<&str>) -> Result<Option<u64>, String> {
    match raw.map(str::trim) {
        None | Some("") => Ok(Some(DEFAULT_GUEST_CAP)),
        Some(count) => match count.parse::<u64>() {
            Ok(0) => Ok(None),
            Ok(cap) => Ok(Some(cap)),
            Err(_) => Err(format!("{count:?} is not a count of guests")),
        },
    }
}

/// The proxies whose `X-Forwarded-For` this gateway believes
/// (`BAYLEE_TRUSTED_PROXIES`, comma-separated addresses).
///
/// An entry that is not an address is dropped rather than refused, and that
/// direction is the deliberate one: an unread entry is a proxy that is
/// **not** trusted, so the limiter keys on the proxy's own address and
/// everyone behind it shares one budget. Strict, and quiet — which is the
/// right way round for a list whose other failure is switching the
/// brute-force defence off.
fn trusted_proxies(raw: &str) -> Vec<IpAddr> {
    raw.split(',')
        .filter_map(|entry| entry.trim().parse::<IpAddr>().ok())
        .collect()
}

/// The IP a rate limit is keyed on: the real peer address, unless the peer
/// itself is a configured trusted proxy — only then is its
/// `X-Forwarded-For` read at all. Trusting the header from anybody let a
/// client rotate it per request and switch the limiter off.
///
/// **Read from the right.** A proxy either replaces the header with the
/// address it is talking to or appends that address to whatever arrived,
/// and this repository tells an operator to do neither, so the rule has to
/// be right for both. On a replacing proxy there is one entry and the two
/// directions are the same. On an appending one the client writes the left
/// half itself: `X-Forwarded-For: <anything>` arrives, the proxy appends the
/// address it actually sees, and reading from the left hands the limiter a
/// string the attacker chose — the very thing trusting the header from
/// anybody did.
///
/// Entries that are themselves on the list are hops and are stepped over,
/// so a chain ends at the first address nobody vouched for. An entry that
/// is not an address at all ends the walk at the peer rather than being
/// stepped over, because everything left of it was written by whoever sent
/// it.
fn client_address(trusted: &[IpAddr], peer: IpAddr, headers: &HeaderMap) -> IpAddr {
    if !trusted.contains(&peer) {
        return peer;
    }
    let Some(forwarded) = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
    else {
        return peer;
    };
    for entry in forwarded.rsplit(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let Ok(address) = entry.parse::<IpAddr>() else {
            return peer;
        };
        if !trusted.contains(&address) {
            return address;
        }
    }
    peer
}

/// A place of `gauge` for the address behind `peer`, or `Err` while that
/// address holds all of its own.
///
/// `None` for this machine, on the unix socket or on loopback: with no
/// trusted proxy naming anyone behind it that is every peer, and one budget
/// for them all would let a stranger spend the local engines'.
fn address_place(
    gauge: &auth::Gauge,
    trusted: &[IpAddr],
    peer: SocketAddr,
    headers: &HeaderMap,
    local: bool,
) -> Result<Option<auth::Held>, ()> {
    if local
        || client_address(trusted, peer.ip(), headers)
            .to_canonical()
            .is_loopback()
    {
        return Ok(None);
    }
    gauge
        .take(&rate_limit_ip(trusted, peer.ip(), headers))
        .map(Some)
        .ok_or(())
}

/// What [`client_address`] is counted under: an IPv6 address by its /64,
/// which one subscriber usually holds whole, so that a single line cannot
/// pass for as many clients as it has addresses and walk past every limit;
/// an IPv4 address, or one mapped into IPv6, as itself.
fn rate_limit_ip(trusted: &[IpAddr], peer: IpAddr, headers: &HeaderMap) -> String {
    match client_address(trusted, peer, headers).to_canonical() {
        IpAddr::V6(v6) => {
            let s = v6.segments();
            format!("{:x}:{:x}:{:x}:{:x}::/64", s[0], s[1], s[2], s[3])
        }
        v4 @ IpAddr::V4(_) => v4.to_string(),
    }
}

/// Resolves the bearer token to an account id.
///
/// One indexed read per authenticated request, and a write only once the
/// session is due a renewal — see `store::resolve_token` for why the
/// sliding expiry stopped sliding on every request.
async fn authed(
    state: &Shared,
    headers: &HeaderMap,
) -> Result<String, (StatusCode, Json<ErrorBody>)> {
    authed_session(state, headers)
        .await
        .map(|session| session.account_id)
}

/// Resolves the bearer token to its session: the account, and whether it
/// is a guest's, for the routes a guest may not use. A session whose account
/// has not accepted the terms in force is refused here (`terms_admit`).
async fn authed_session(
    state: &Shared,
    headers: &HeaderMap,
) -> Result<store::Session, (StatusCode, Json<ErrorBody>)> {
    let session = authed_session_any_terms(state, headers).await?;
    terms_admit(&session)?;
    Ok(session)
}

/// [`authed_session`] without the terms: for the few routes a session that
/// has not accepted them still has (`POST /account/terms`, signing out,
/// deleting the account here).
async fn authed_session_any_terms(
    state: &Shared,
    headers: &HeaderMap,
) -> Result<store::Session, (StatusCode, Json<ErrorBody>)> {
    let token = bearer_token(headers)
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "missing bearer token"))?;
    let session = central::resolve_digest(state, auth::token_digest(token))
        .await
        .map_err(|status| err(status, "account authorization unavailable"))?
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "invalid or expired token"))?;
    moderation::admit(&session)?;
    Ok(session)
}

/// `403` with `terms_required` (`central` or `gateway`) for a session whose
/// account has not accepted the terms in force: the client shows that
/// sheet, and nothing goes on under an older version.
pub(crate) fn terms_admit(session: &store::Session) -> Result<(), (StatusCode, Json<ErrorBody>)> {
    let Some(document) = session.terms.document() else {
        return Ok(());
    };
    let (status, Json(mut body)) = err(
        StatusCode::FORBIDDEN,
        "accept the terms of use in force to go on",
    );
    body.terms_required = Some(document);
    body.refusal = Some(Box::new(baylee_protocol::refusal::Refusal::new("terms")));
    Err((status, Json(body)))
}

/// A database that refused, as an answer a client can act on.
///
/// 503 rather than 500: the gateway itself is fine and the thing behind it
/// is not, and that difference is what tells a client to try again later
/// instead of to stop.
fn db_down(e: &anyhow::Error) -> (StatusCode, Json<ErrorBody>) {
    tracing::error!("{e:#}");
    err(
        StatusCode::SERVICE_UNAVAILABLE,
        "the gateway's database is unavailable",
    )
}

// --------------------------------------------------------------- whole bodies

/// How long a request's body has to arrive whole: a report's or a picture's
/// worth over a slow phone connection.
const BODY_WITHIN: std::time::Duration = std::time::Duration::from_secs(120);

/// The most any route reads. Each route's own, smaller limit still applies
/// to what is handed on.
const LARGEST_BODY: usize = if cosmetics::MAX_UPLOAD_BYTES > report::MAX_BODY_BYTES {
    cosmetics::MAX_UPLOAD_BYTES
} else {
    report::MAX_BODY_BYTES
};

/// How many large bodies one address may have on their way at once, read
/// before anything about the sender is known.
const LARGE_BODIES_PER_ADDRESS: usize = 16;

/// The largest body that needs no place among those: what most routes read.
const SMALL_BODY: u64 = 64 * 1024;

/// Reads a request's body before anything else happens to the request.
///
/// A stranger can start a request and never finish it, and nothing else
/// bounds how long that takes. Every route reads its body whole anyway, so
/// reading it here costs nothing and means no route, and no lock a route
/// runs under (`central::barrier`), ever waits on one.
///
/// A body that may be large holds a place of its address's until it is in,
/// so that one address cannot keep more than a known amount of memory here
/// however many connections it opens.
async fn whole_body(
    State(state): State<Shared>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    via_unix: Option<axum::Extension<engine::ViaUnix>>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::body::HttpBody as _;
    use axum::response::IntoResponse as _;
    let (parts, body) = request.into_parts();
    let small = body.size_hint().upper().is_some_and(|n| n <= SMALL_BODY);
    let place = if small {
        None
    } else {
        let local = via_unix.is_some();
        let trusted = &state.trusted_proxies;
        let Ok(place) = address_place(&state.bodies, trusted, peer, &parts.headers, local) else {
            return StatusCode::TOO_MANY_REQUESTS.into_response();
        };
        place
    };
    let read = read_body(body, LARGEST_BODY, BODY_WITHIN).await;
    drop(place);
    match read {
        Ok(bytes) => {
            next.run(axum::extract::Request::from_parts(parts, bytes.into()))
                .await
        }
        Err(status) => status.into_response(),
    }
}

/// `body` whole, or `408` when it takes longer than `within` and `413` when
/// it runs past `limit`.
async fn read_body(
    body: axum::body::Body,
    limit: usize,
    within: std::time::Duration,
) -> Result<bytes::Bytes, StatusCode> {
    match tokio::time::timeout(within, axum::body::to_bytes(body, limit)).await {
        Ok(Ok(bytes)) => Ok(bytes),
        Ok(Err(_)) => Err(StatusCode::PAYLOAD_TOO_LARGE),
        Err(_) => Err(StatusCode::REQUEST_TIMEOUT),
    }
}

// ------------------------------------------------------------- cross-origin

/// Every response carries the headers a browser needs to read it.
///
/// The browser client is served from somewhere else by construction: the page
/// is a `trunk serve` (or a static host) and the gateway is a different
/// origin, which `?gateway=…` exists to say. So every request it makes is
/// cross-origin, and without these headers a browser discards the answer —
/// a `GET /pool` is a *simple* request that is sent and then thrown away, and
/// anything carrying `Authorization` is not sent at all, because the
/// preflight was answered `405 Method Not Allowed`. Measured: a gateway with
/// no CORS refuses `OPTIONS /auth/login` outright, which is the whole lobby.
///
/// `*` rather than an allowlist, and that is a decision rather than a
/// shortcut. This gateway authenticates with a **bearer token in a header**
/// and sets no cookie anywhere, so a browser sends nothing ambient with a
/// cross-origin request: a page on another origin can reach these routes as
/// an anonymous client and no further, which is exactly what any HTTP client
/// can already do. `Allow-Credentials` is therefore never sent, and the two
/// together are the pair that must not drift apart — `*` with credentials is
/// the combination the fetch spec refuses, and for good reason.
///
/// The preflight is answered here rather than routed, because axum resolves a
/// path to a `MethodRouter` that knows only the methods a handler registered:
/// an `OPTIONS` to any route is a 405 before any handler sees it, and a
/// per-route `options(…)` would have to be typed out on all sixty routes and
/// forgotten on the sixty-first.
async fn cors(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::http::{HeaderValue, Method, header};

    /// What the client is allowed to send. `authorization` is the bearer
    /// token and `content-type` is what makes a JSON body a JSON body — both
    /// are what turns an otherwise simple request into a preflighted one.
    const ALLOW_HEADERS: &str = "authorization, content-type";
    /// Every method this gateway routes.
    const ALLOW_METHODS: &str = "GET, POST, PUT, PATCH, DELETE, OPTIONS";

    let preflight = request.method() == Method::OPTIONS;
    let mut response = if preflight {
        // Answered whole, and never passed on: the route this is a preflight
        // *for* has no `OPTIONS` handler and would answer 405.
        let mut empty = axum::response::Response::new(axum::body::Body::empty());
        *empty.status_mut() = StatusCode::NO_CONTENT;
        empty
    } else {
        next.run(request).await
    };
    let headers = response.headers_mut();
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    if preflight {
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static(ALLOW_METHODS),
        );
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static(ALLOW_HEADERS),
        );
        // A day, so a lobby that asks sixty questions asks this one once.
        headers.insert(
            header::ACCESS_CONTROL_MAX_AGE,
            HeaderValue::from_static("86400"),
        );
    }
    response
}

#[cfg(test)]
mod tests;
