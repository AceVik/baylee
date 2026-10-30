//! The admin console's routes (`docs/protocol.md` §"The admin console"):
//! how busy this gateway is, in aggregate numbers, and its closed-beta keys
//! made, listed and revoked as `baylee-gateway invite` does.
//!
//! They are served on a listener of their own and nowhere else: never on
//! the public port, which the reverse proxy forwards from the internet, and
//! never on the unix socket. `BAYLEE_ADMIN_TOKEN` switches them on;
//! `BAYLEE_ADMIN_BIND` (`127.0.0.1:28767`) says where, and it must be a
//! loopback address. The one caller meant is the feedback service's admin
//! UI on the same machine (`docs/feedback.md` §"The admin console"), which
//! signs its admins in itself and passes on who acted.
//!
//! Every request goes through [`guard`], in this order:
//!
//! 1. anything a browser marks, `Origin` or a `Sec-Fetch-*` header, is
//!    refused (`403`): a page anywhere, this machine's own included, may
//!    not drive the console, and no server-side caller sends them;
//! 2. a `Host` that is not a loopback name is refused (`403`), so a name
//!    rebound to `127.0.0.1` reaches nothing;
//! 3. past [`MAX_FAILURES`] wrong tokens in [`FAILURE_WINDOW`] every request
//!    is `429`, the right token included, until the window passes; past
//!    [`MAX_REQUESTS`] a minute, too;
//! 4. the bearer is either the constant-time checked `BAYLEE_ADMIN_TOKEN`
//!    bootstrap credential or a gateway-local token with a live, purpose-bound
//!    Central admin lease. Central rechecks explicit gateway membership on
//!    every request; the gateway owner has no implicit backend access;
//! 5. bootstrap mutations name their admin in [`ACTOR_HEADER`] (`400` without).
//!    SSO mutations instead use the lease's immutable Central account UUID and
//!    ignore caller-supplied actor headers. That identity goes into the audit line.
//!
//! Every change is logged on the target `baylee_gateway::audit`: who, what,
//! which key ids, when (the log's own time). Never a key, a token or a
//! key's note. The routes add no CORS header: the gateway's `*` is for the
//! public routes, and these are not one.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{delete, get, post, put};
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use subtle::ConstantTimeEq as _;
use time::OffsetDateTime;

use crate::{ErrorBody, Shared, auth, err, err_saying, invite};

/// The shortest `BAYLEE_ADMIN_TOKEN` the gateway starts with.
pub(crate) const MIN_TOKEN_CHARS: usize = 32;

/// Where the console listens unless `BAYLEE_ADMIN_BIND` says otherwise.
pub(crate) const DEFAULT_BIND: &str = "127.0.0.1:28767";

/// The header that names the admin who asked for a change.
pub(crate) const ACTOR_HEADER: &str = "x-baylee-admin";

/// The longest admin name, as the feedback service allows one.
const MAX_ACTOR_CHARS: usize = 64;

/// How long wrong tokens count.
const FAILURE_WINDOW: Duration = Duration::from_secs(300);

/// Wrong tokens in [`FAILURE_WINDOW`] before every request is refused.
const MAX_FAILURES: usize = 10;

/// Requests a minute, whatever their token: a console refreshing every
/// fifteen seconds asks four times a minute per open page.
const MAX_REQUESTS: usize = 600;

/// The largest body a console request may carry.
const MAX_BODY_BYTES: usize = 16 * 1024;

/// The limiter's one key: the listener is loopback only, so there is no
/// address worth telling apart.
const EVERYONE: &str = "console";

/// `BAYLEE_ADMIN_TOKEN` and `BAYLEE_ADMIN_BIND`, read and checked.
pub(crate) struct Settings {
    /// SHA-256 of the token.
    token: [u8; 32],
    /// Where to listen.
    pub(crate) bind: SocketAddr,
}

impl Settings {
    /// The console's settings, or `None` when `BAYLEE_ADMIN_TOKEN` is unset
    /// or empty and there is no console. `others` are the gateway's other
    /// secrets (`BAYLEE_AGENT_TOKEN`, `BAYLEE_FEEDBACK_TOKEN`,
    /// `BAYLEE_FEEDBACK_KEY`), none of which the admin token may be.
    ///
    /// # Errors
    ///
    /// A token shorter than [`MIN_TOKEN_CHARS`], holding whitespace or a
    /// control character, or equal to another secret; a bind that is not
    /// an address and port, or not a loopback one; a bind with no token.
    pub(crate) fn from_env(
        token: Option<&str>,
        bind: Option<&str>,
        others: &[Option<&str>],
    ) -> Result<Option<Self>, String> {
        let bind = bind.map(str::trim).filter(|b| !b.is_empty());
        let Some(token) = token.filter(|t| !t.is_empty()) else {
            return match bind {
                Some(_) => Err("BAYLEE_ADMIN_BIND is set, and BAYLEE_ADMIN_TOKEN is not".into()),
                None => Ok(None),
            };
        };
        if token.chars().count() < MIN_TOKEN_CHARS {
            return Err(format!(
                "BAYLEE_ADMIN_TOKEN: a token needs at least {MIN_TOKEN_CHARS} characters"
            ));
        }
        if token.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err("BAYLEE_ADMIN_TOKEN: a token is one word of printable characters".into());
        }
        if others.iter().flatten().any(|other| *other == token) {
            return Err(
                "BAYLEE_ADMIN_TOKEN: that token is already another secret of this gateway".into(),
            );
        }
        let bind = bind.unwrap_or(DEFAULT_BIND);
        let address: SocketAddr = bind
            .parse()
            .map_err(|_| format!("BAYLEE_ADMIN_BIND: {bind:?} is not an address and port"))?;
        if !address.ip().is_loopback() {
            return Err(format!(
                "BAYLEE_ADMIN_BIND: {address} is not a loopback address; the console is \
                 for this machine only"
            ));
        }
        Ok(Some(Self {
            token: digest(token),
            bind: address,
        }))
    }
}

fn digest(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

/// What the console's routes share.
pub(crate) struct Console {
    /// The gateway's state.
    pub(crate) app: Shared,
    /// SHA-256 of `BAYLEE_ADMIN_TOKEN`.
    token: [u8; 32],
    /// Wrong tokens.
    failures: auth::RateLimiter,
    /// Every request.
    requests: auth::RateLimiter,
}

pub(crate) type Admin = Arc<Console>;

/// The console's routes, guarded.
fn router(app: Shared, token: [u8; 32]) -> Router {
    let console = Arc::new(Console {
        app,
        token,
        failures: auth::RateLimiter::new(FAILURE_WINDOW, MAX_FAILURES),
        requests: auth::RateLimiter::new(Duration::from_secs(60), MAX_REQUESTS),
    });
    Router::new()
        .route("/admin/stats", get(stats))
        .route("/admin/live", get(live))
        .route("/admin/metrics", get(metrics))
        .route("/admin/sets", get(sets))
        .route("/admin/sets/{code}", get(one_set))
        .route("/admin/accounts", get(list_accounts))
        .route(
            "/admin/accounts/{id}",
            get(one_account).delete(crate::moderation::delete_account),
        )
        .route("/admin/invites", get(list_invites).post(create_invites))
        .route("/admin/invites/{id}", delete(revoke_invite))
        .route("/admin/cards/search", get(search_cards))
        .route("/admin/cards/blocks", get(list_blocks).post(block_card))
        .route("/admin/cards/blocks/{id}", delete(unblock_card))
        .route("/admin/llm/seathosts", get(llm_seathosts))
        .route("/admin/llm/profiles", get(llm_profiles))
        .route(
            "/admin/llm/seathosts/{host}/profiles/{id}",
            put(llm_write).delete(llm_delete),
        )
        .route(
            "/admin/llm/seathosts/{host}/profiles/{id}/enabled",
            post(llm_enabled),
        )
        .route(
            "/admin/llm/seathosts/{host}/profiles/{id}/probe",
            post(llm_probe),
        )
        .route(
            "/admin/llm/seathosts/{host}/profiles/{id}/key",
            post(llm_key),
        )
        // Acting on an account or a game (beta.8): `moderation.rs`.
        .merge(crate::moderation::console_routes())
        // Who may seat which hosted model (beta.8): `llmaccess.rs`.
        .merge(crate::llmaccess::console_routes())
        .merge(crate::termsadmin::console_routes())
        // Who comes in: `authmode.rs`.
        .merge(crate::authmode::console_routes())
        .fallback(no_such_route)
        .layer(axum::extract::DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(axum::middleware::from_fn_with_state(console.clone(), guard))
        .layer(axum::middleware::map_response(never_stored))
        .with_state(console)
}

/// Binds the console's listener, before the gateway serves anything, so a
/// port that is taken stops the start rather than a console that is
/// silently missing.
///
/// # Panics
///
/// When the address cannot be bound.
pub(crate) async fn bind(settings: &Settings) -> (tokio::net::TcpListener, u16) {
    let listener = tokio::net::TcpListener::bind(settings.bind)
        .await
        .unwrap_or_else(|e| panic!("BAYLEE_ADMIN_BIND {}: {e}", settings.bind));
    let port = listener
        .local_addr()
        .expect("a bound listener has an address")
        .port();
    (listener, port)
}

/// Serves the console on `listener`.
pub(crate) fn serve(listener: tokio::net::TcpListener, app: Shared, settings: &Settings) {
    let router = router(app, settings.token);
    tracing::info!(bind = %settings.bind, "admin console listening");
    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("gateway serves its admin console");
    });
}

pub(crate) type Refusal = (StatusCode, Json<ErrorBody>);

/// Who asked for a change, from [`ACTOR_HEADER`].
#[derive(Clone)]
pub(crate) struct Actor(pub(crate) String);

/// Whether `name` may stand in an audit line: what the feedback service
/// allows as an admin's name, 1–64 letters, digits, `.`, `-`, `_`.
fn valid_actor(name: &str) -> bool {
    (1..=MAX_ACTOR_CHARS).contains(&name.chars().count())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Whether `host` (a `Host` header) names this machine: `localhost` or a
/// loopback address, with or without a port.
fn loopback_host(host: &str) -> bool {
    let name = if let Some(rest) = host.strip_prefix('[') {
        // `[::1]:28767`
        rest.split_once(']').map_or("", |(inside, _)| inside)
    } else {
        match host.rsplit_once(':') {
            Some((name, port))
                if !name.contains(':') && port.bytes().all(|b| b.is_ascii_digit()) =>
            {
                name
            }
            _ => host,
        }
    };
    name.eq_ignore_ascii_case("localhost")
        || name.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

/// The checks every console request passes before a handler sees it, that
/// do not need the token: no browser's marks, a loopback `Host`.
fn from_this_machine(headers: &HeaderMap) -> Result<(), Refusal> {
    let browser = headers.contains_key(header::ORIGIN)
        || headers.contains_key("sec-fetch-site")
        || headers.contains_key("sec-fetch-mode");
    if browser {
        return Err(err(
            StatusCode::FORBIDDEN,
            "the admin console takes no request from a browser",
        ));
    }
    let host = headers
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();
    if !loopback_host(host) {
        return Err(err(
            StatusCode::FORBIDDEN,
            "the admin console answers this machine only",
        ));
    }
    Ok(())
}

/// Whether the request's bearer token is `BAYLEE_ADMIN_TOKEN`, compared as
/// digests in constant time.
fn token_matches(known: &[u8; 32], headers: &HeaderMap) -> bool {
    let presented = crate::bearer_token(headers).unwrap_or_default();
    bool::from(known.ct_eq(&digest(presented)))
}

/// See the module's documentation for the order.
async fn guard(State(console): State<Admin>, mut request: Request, next: Next) -> Response {
    let headers = request.headers();
    if let Err(refusal) = from_this_machine(headers) {
        return refusal.into_response();
    }
    if console.failures.spent(EVERYONE) || !console.requests.allow(EVERYONE) {
        let mut refusal = err(StatusCode::TOO_MANY_REQUESTS, "too many attempts").into_response();
        refusal.headers_mut().insert(
            header::RETRY_AFTER,
            HeaderValue::from(FAILURE_WINDOW.as_secs()),
        );
        return refusal;
    }
    let central_actor = if token_matches(&console.token, headers) {
        None
    } else {
        match crate::central::admin_actor(
            &console.app,
            crate::bearer_token(headers).unwrap_or_default(),
        )
        .await
        {
            Ok(Some(actor)) => Some(Actor(actor)),
            Ok(None) => {
                console.failures.allow(EVERYONE);
                return err(
                    StatusCode::UNAUTHORIZED,
                    "gateway administrator authorization required",
                )
                .into_response();
            }
            Err(status) => {
                return err(status, "gateway administrator authorization unavailable")
                    .into_response();
            }
        }
    };
    let actor = central_actor.or_else(|| {
        headers
            .get(ACTOR_HEADER)
            .and_then(|v| v.to_str().ok())
            .filter(|name| valid_actor(name))
            .map(|name| Actor(name.to_owned()))
    });
    let changes = !matches!(*request.method(), Method::GET | Method::HEAD);
    match actor {
        Some(actor) => {
            request.extensions_mut().insert(actor);
        }
        None if changes => {
            return err(
                StatusCode::BAD_REQUEST,
                "a change names its admin in X-Baylee-Admin: 1-64 letters, digits, . - _",
            )
            .into_response();
        }
        None => {}
    }
    next.run(request).await
}

/// Nothing the console answers is kept by anything between it and its
/// reader.
async fn never_stored(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

async fn no_such_route() -> Refusal {
    err(StatusCode::NOT_FOUND, "no such route")
}

/// A moment as the console reads it: `2026-10-08T12:00:00Z`.
pub(crate) fn iso(at: OffsetDateTime) -> String {
    let at = at.to_offset(time::UtcOffset::UTC);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        at.year(),
        u8::from(at.month()),
        at.day(),
        at.hour(),
        at.minute(),
        at.second()
    )
}

/// How many of something there were since three moments.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleSince {
    /// @description Since midnight UTC today.
    today_utc: i64,
    /// @description In the last 7 × 24 hours.
    last_7d: i64,
    /// @description In the last 30 × 24 hours.
    last_30d: i64,
}

fn since(counts: baylee_db::stats::Since) -> ConsoleSince {
    ConsoleSince {
        today_utc: counts.today_utc,
        last_7d: counts.last_7d,
        last_30d: counts.last_30d,
    }
}

fn db_failed(e: &sea_orm::DbErr) -> Refusal {
    tracing::error!("admin console: {e}");
    err(
        StatusCode::SERVICE_UNAVAILABLE,
        "the gateway's database is unavailable",
    )
}

/// `GET /admin/stats`'s engine agents.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleAgentCounts {
    /// @description Engine agents connected now.
    connected: usize,
    /// @description Of those, the ones on this machine's unix socket.
    local: usize,
    /// @description How many games they may run together; `null` when one
    /// of them has no bound.
    capacity: Option<u64>,
    /// @description The games they were asked to run.
    games: usize,
}

/// The agents connected now: how many, how many on this machine's unix
/// socket, how many games they may run together (`null` when one of them
/// has no bound) and how many they were asked to.
fn agents(app: &crate::AppState) -> ConsoleAgentCounts {
    let agents = app.agents.lock();
    let connected = agents.connected.values();
    let unlimited = connected.clone().any(|agent| agent.capacity == 0);
    ConsoleAgentCounts {
        connected: agents.connected.len(),
        local: connected.clone().filter(|agent| agent.local).count(),
        capacity: (!unlimited).then(|| {
            connected
                .clone()
                .map(|agent| u64::from(agent.capacity))
                .sum::<u64>()
        }),
        games: connected.map(|agent| agent.games.len()).sum::<usize>(),
    }
}

/// How many days `GET /admin/stats` answers in `daily`.
const DAILY_DAYS: u32 = 30;

/// `GET /admin/stats`: counts, and nothing that names anybody.
///
/// @route GET /admin/stats
/// @scope admin
/// @summary How busy this gateway is, in counts
/// @description Counts and sums only, no field naming a person: accounts,
/// guests, who is online, games, engine agents, friends and chat, beta
/// keys, and the last 30 UTC days.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn stats(State(console): State<Admin>) -> Result<Json<ConsoleStats>, Refusal> {
    let app = &console.app;
    let now = OffsetDateTime::now_utc();
    let stored = baylee_db::stats::read(&app.db, now)
        .await
        .map_err(|e| db_failed(&e))?;
    let daily = baylee_db::console::daily(&app.db, now, DAILY_DAYS)
        .await
        .map_err(|e| db_failed(&e))?;

    let (in_lobby, seated, players, running, local_running, waiting, awaiting_engine) = {
        let lobby = app.lobby.lock();
        let seated = lobby.playing_accounts();
        let in_lobby = app.presence.accounts();
        let players = seated.union(&in_lobby).count();
        let running = lobby.running();
        (
            in_lobby.len(),
            seated.len(),
            players,
            running.clone().count(),
            running.clone().filter(|game| game.engine_local).count(),
            lobby.waiting().count(),
            running
                .filter(|game| game.engine.is_none())
                .map(|game| game.seats.len())
                .sum::<usize>(),
        )
    };
    let gateway = ConsoleGateway {
        build: crate::routes::Build::CURRENT,
        name: app.display_name.clone(),
        registration: app.registration,
        uptime_secs: crate::auth::now_secs().saturating_sub(app.started_at),
        terms: app.terms.current().is_some(),
        central: app.connected(),
        auth_mode: app.auth_mode.current(),
        auth_mode_source: app.auth_mode.source().as_str(),
    };
    let social = crate::social::console_counts(app).await;
    Ok(Json(ConsoleStats {
        at: iso(now),
        gateway,
        accounts: ConsoleAccountCounts {
            registered: stored.registered,
            with_email: stored.with_email,
            confirmed_email: stored.confirmed_email,
            admitted_by_key: stored.admitted_by_key,
            created: since(stored.registered_since),
            decks: stored.decks,
        },
        guests: ConsoleGuestCounts {
            enabled: app.auth_mode.current().guests(),
            live: stored.guests,
            cap: app.guest_cap,
            created: since(stored.guests_since),
        },
        online: ConsoleOnline {
            players,
            in_lobby,
            seated,
            sessions_live: stored.sessions_live,
            accounts_signed_in: stored.accounts_signed_in,
        },
        games: ConsoleGameCounts {
            running,
            local_running,
            waiting,
            seats_awaiting_engine: awaiting_engine,
            recorded: stored.games_recorded,
            started: since(stored.games_started),
            finished: stored.games_finished,
            finished_since: since(stored.games_finished_since),
            record_bytes: stored.record_bytes,
            avg_secs_30d: stored.avg_game_secs_30d,
        },
        agents: agents(app),
        social,
        invites: ConsoleInviteCounts {
            total: stored.invites_total,
            active: stored.invites_active,
            used_up: stored.invites_used_up,
            expired: stored.invites_expired,
            revoked: stored.invites_revoked,
            uses_left: stored.invite_uses_left,
            admitted: stored.admitted_by_key,
        },
        daily,
    }))
}

/// `GET /admin/stats`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleStats {
    /// @description When the counts were taken, RFC 3339 in UTC.
    /// @openapi format: date-time
    at: String,
    /// @description This gateway and its build.
    gateway: ConsoleGateway,
    /// @description Registered accounts.
    accounts: ConsoleAccountCounts,
    /// @description Guest accounts.
    guests: ConsoleGuestCounts,
    /// @description Who is online now.
    online: ConsoleOnline,
    /// @description Games, in the lobby now and as recorded.
    games: ConsoleGameCounts,
    /// @description The engine agents connected now.
    agents: ConsoleAgentCounts,
    /// @description Friends and chat: `devices`, `friendships`, `groups`,
    /// `pending_slots` and `sockets`, all counts; `null` when the gateway
    /// has none or its counts could not be read.
    /// @openapi
    /// ```yaml
    /// type: [object, 'null']
    /// properties:
    ///   devices: { type: integer }
    ///   friendships: { type: integer }
    ///   groups: { type: integer }
    ///   pending_slots: { type: integer }
    ///   sockets: { type: integer }
    /// ```
    social: serde_json::Value,
    /// @description The closed-beta keys.
    invites: ConsoleInviteCounts,
    /// @description The last 30 UTC days, oldest first, today last.
    daily: Vec<baylee_db::console::Day>,
}

/// `GET /admin/stats`'s gateway.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleGateway {
    #[serde(flatten)]
    build: crate::routes::Build,
    /// @description The name it shows (`BAYLEE_GATEWAY_NAME`), `null` for
    /// none.
    name: Option<String>,
    /// @description Who may make an account.
    registration: crate::invite::Registration,
    /// @description Seconds this process has served.
    uptime_secs: u64,
    /// @description Whether it has terms of use.
    terms: bool,
    /// @description Whether it is connected to Baylee Central
    /// (`BAYLEE_CENTRAL_CONFIG_FILE`), which is how a Central sign-in
    /// reaches it.
    central: bool,
    /// @description Who comes in: guests, Baylee Central's accounts, or
    /// both (`GET /admin/auth-mode`).
    auth_mode: crate::authmode::AuthMode,
    /// @description Where that comes from: the operator's
    /// `BAYLEE_AUTH_MODE`, an admin's choice, or the default.
    /// @openapi enum: [environment, console, default]
    auth_mode_source: &'static str,
}

/// `GET /admin/stats`'s registered accounts.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleAccountCounts {
    /// @description Accounts that are not guests.
    registered: i64,
    /// @description Of those, the ones with an e-mail address.
    with_email: i64,
    /// @description Of those, the ones whose address was confirmed.
    confirmed_email: i64,
    /// @description Registered accounts a closed-beta key let in, that
    /// still exist.
    admitted_by_key: i64,
    /// @description Registered accounts made in each window that still
    /// exist.
    created: ConsoleSince,
    /// @description Players' own decks.
    decks: i64,
}

/// `GET /admin/stats`'s guests.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleGuestCounts {
    /// @description Whether guests are let in (the auth mode is `guests`
    /// or `mixed`).
    enabled: bool,
    /// @description Guest accounts now, each one a live session leads to
    /// until the next sweep.
    live: i64,
    /// @description How many guests may be live at once
    /// (`BAYLEE_GUEST_CAP`); `null` for no cap.
    cap: Option<u64>,
    /// @description Guests made in each window that still exist.
    created: ConsoleSince,
}

/// `GET /admin/stats`'s presence.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleOnline {
    /// @description Accounts with a lobby socket open or a chair at a
    /// running game (`GET /lobby/stats`'s `players_online`).
    players: usize,
    /// @description Accounts with a lobby socket open.
    in_lobby: usize,
    /// @description Accounts in a chair of a running game.
    seated: usize,
    /// @description Sessions that have not lapsed, which is not presence:
    /// a guest's lasts 30 days.
    sessions_live: i64,
    /// @description Accounts with at least one such session.
    accounts_signed_in: i64,
}

/// `GET /admin/stats`'s games.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleGameCounts {
    /// @description Games being played (as `/health` counts them).
    running: usize,
    /// @description Of those, games whose engine runs on this machine.
    local_running: usize,
    /// @description Rooms waiting for players.
    waiting: usize,
    /// @description Seats of running games whose engine has not dialled
    /// back yet.
    seats_awaiting_engine: usize,
    /// @description Games with a record: every hosted game whose engine
    /// sent the first piece of it.
    recorded: i64,
    /// @description Recorded games started in each window.
    started: ConsoleSince,
    /// @description Recorded games whose record is complete.
    finished: i64,
    /// @description Of those, the ones finished in each window.
    finished_since: ConsoleSince,
    /// @description The bytes every record holds together.
    record_bytes: i64,
    /// @description The mean length in seconds of the games finished in
    /// the last 30 days; `0` for none.
    avg_secs_30d: i64,
}

/// `GET /admin/stats`'s closed-beta keys.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleInviteCounts {
    /// @description Keys ever made.
    total: i64,
    /// @description Keys that admit somebody now: not revoked, not
    /// expired, a use left.
    active: i64,
    /// @description Keys not revoked and not expired, with no use left.
    used_up: i64,
    /// @description Keys not revoked whose expiry has passed.
    expired: i64,
    /// @description Keys revoked.
    revoked: i64,
    /// @description The uses the active keys have left, together.
    uses_left: i64,
    /// @description The existing accounts a key let in.
    admitted: i64,
}

/// The session lifetimes, as the store's console reads want them.
fn lifetimes() -> baylee_db::console::Lifetimes {
    let of = |d: Duration| time::Duration::try_from(d).unwrap_or(time::Duration::ZERO);
    baylee_db::console::Lifetimes {
        account: of(crate::auth::Lifetime::ACCOUNT.ttl),
        guest: of(crate::auth::Lifetime::GUEST.ttl),
    }
}

/// A unix second as the console reads moments.
fn iso_secs(secs: u64) -> String {
    i64::try_from(secs)
        .ok()
        .and_then(|s| OffsetDateTime::from_unix_timestamp(s).ok())
        .map_or_else(String::new, iso)
}

/// Where an account is, as far as this gateway's memory says.
#[derive(Default)]
struct Whereabouts {
    /// A lobby socket is open.
    lobby: bool,
    /// In a chair of a running game, and which.
    playing: Option<String>,
    /// In a chair of a room still waiting, and which.
    waiting: Option<String>,
}

/// A chair as the live view shows it, before names are known.
struct ChairNow {
    seat: usize,
    kind: crate::lobby::SeatKind,
    ai: Option<String>,
    account: Option<String>,
    delegate: Option<(String, String)>,
    /// What the room's listing says of a hosted model here, or `null`.
    hosted: serde_json::Value,
    deck: String,
    format: Option<String>,
    ready: bool,
    team: Option<u8>,
}

/// A table as the live view shows it, before names are known.
#[expect(
    clippy::struct_excessive_bools,
    reason = "a table's facts the console shows side by side"
)]
struct TableNow {
    id: String,
    name: String,
    playing: bool,
    host: Option<String>,
    created_at: u64,
    locked: bool,
    rematch: bool,
    decide_secs: u32,
    engine: bool,
    engine_local: bool,
    agent: Option<String>,
    chairs: Vec<ChairNow>,
}

/// What the lobby holds now, copied out under its lock.
fn tables_now(app: &crate::AppState) -> Vec<TableNow> {
    let agent_names: std::collections::HashMap<String, String> = app
        .agents
        .lock()
        .connected
        .iter()
        .map(|(id, agent)| (id.clone(), agent.name.clone()))
        .collect();
    let lobby = app.lobby.lock();
    let mut tables: Vec<TableNow> = lobby
        .waiting()
        .chain(lobby.running())
        .map(|game| TableNow {
            id: game.id.clone(),
            name: game.name.clone(),
            playing: game.state == crate::lobby::LobbyState::Playing,
            host: game.host.clone(),
            created_at: game.created_at,
            locked: game.password_hash.is_some(),
            rematch: game.parent.is_some(),
            decide_secs: game.house_rules.decision_timeout_secs,
            engine: game.engine.is_some(),
            engine_local: game.engine_local,
            agent: game
                .agent_id
                .as_ref()
                .map(|id| agent_names.get(id).cloned().unwrap_or_else(|| id.clone())),
            chairs: game
                .seats
                .iter()
                .map(|seat| ChairNow {
                    seat: seat.seat,
                    kind: seat.kind,
                    ai: seat.ai.clone(),
                    account: seat.account_id.clone(),
                    delegate: seat
                        .delegate
                        .as_ref()
                        .map(|d| (d.name.clone(), d.by.clone())),
                    hosted: crate::seathost::chair_json(seat),
                    deck: seat.deck_name.clone(),
                    format: seat.deck.as_ref().map(|d| d.format.clone()),
                    ready: game.seat_ready(seat),
                    team: seat.team,
                })
                .collect(),
        })
        .collect();
    // Waiting rooms first, then the newest; the id breaks a tie.
    tables.sort_by(|a, b| {
        a.playing
            .cmp(&b.playing)
            .then(b.created_at.cmp(&a.created_at))
            .then(a.id.cmp(&b.id))
    });
    tables
}

/// `GET /admin/metrics`: the last hour of server samples (`metrics.rs`),
/// nothing per person.
///
/// @route GET /admin/metrics
/// @scope admin
/// @summary The last hour of server samples
/// @description What the machine is doing, sampled every 5 seconds into
/// a ring of the last hour (720 samples) in memory: the host's CPU, load,
/// memory, disk and network, this process's sockets and request rate, and
/// the CPU and memory of each engine process on this machine. The host's
/// fields are read off `/proc` and are `null` off Linux.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "Baylee Central or the gateway's database is unavailable while an admin lease is checked"
async fn metrics(State(console): State<Admin>) -> Json<crate::metrics::MetricsSnapshot> {
    let app = &console.app;
    let uptime = crate::auth::now_secs().saturating_sub(app.started_at);
    Json(
        app.metrics
            .snapshot(&iso(OffsetDateTime::now_utc()), uptime),
    )
}

/// `GET /admin/sets`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleSets {
    /// @description When it was read, RFC 3339 in UTC.
    /// @openapi format: date-time
    at: String,
    /// @description The release this build is.
    version: &'static str,
    /// @description The cards of the corpus: every row of the ledger.
    corpus: usize,
    /// @description The pool this build plays, over every set.
    pool: PoolTotals,
    /// @description Every set, in the ledger's order (release order by
    /// first printing); a set counts the cards first printed in it.
    sets: Vec<crate::sets::SetProgress>,
}

/// `GET /admin/sets`'s pool.
/// @openapi
#[derive(Serialize)]
pub(crate) struct PoolTotals {
    /// @description The pool's cards: implemented, generated, partial and
    /// unimplemented together.
    cards: usize,
    /// @description Cards rules-complete and played by a card test.
    implemented: usize,
    /// @description Cards read in full that no card test plays yet;
    /// playable.
    generated: usize,
    /// @description Cards only partly there.
    partial: usize,
    /// @description Cards that compile as stubs.
    unimplemented: usize,
    /// @description Ledger rows with no pool card at all.
    absent: usize,
}

/// `GET /admin/sets`: how far the pool is through each set, in release
/// order (`sets.rs`).
///
/// @route GET /admin/sets
/// @scope admin
/// @summary How far the pool is through each set
/// @description How far this build is through each set, in release order
/// by first printing, from the ledger and the compiled pool; the numbers
/// are whatever this binary was built with.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "Baylee Central or the gateway's database is unavailable while an admin lease is checked"
async fn sets() -> Json<ConsoleSets> {
    let whole = crate::sets::pool_totals();
    Json(ConsoleSets {
        at: iso(OffsetDateTime::now_utc()),
        version: baylee_build::short(),
        corpus: whole.total,
        pool: PoolTotals {
            cards: whole.implemented + whole.generated + whole.partial + whole.unimplemented,
            implemented: whole.implemented,
            generated: whole.generated,
            partial: whole.partial,
            unimplemented: whole.unimplemented,
            absent: whole.absent,
        },
        sets: crate::sets::all().to_vec(),
    })
}

/// `GET /admin/sets/{code}`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleSet {
    #[serde(flatten)]
    set: crate::sets::SetProgress,
    /// @description The set's cards, in ledger order.
    cards: Vec<crate::sets::SetCard>,
}

/// `GET /admin/sets/{code}`: one set's cards and how far each is.
///
/// @route GET /admin/sets/{code}
/// @scope admin
/// @summary One set's cards
/// @description One set's row as `GET /admin/sets` lists it, and its
/// cards with how far each is. The gateway serves no card image here.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @path-param code: String "Scryfall's set code, any case"
/// @return 400: `$ErrorBody` "Not a set code: more than 8 characters, or not letters and digits"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 404: `$ErrorBody` "No such set in the ledger"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "Baylee Central or the gateway's database is unavailable while an admin lease is checked"
async fn one_set(Path(code): Path<String>) -> Result<Json<ConsoleSet>, Refusal> {
    if code.len() > 8 || !code.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(err(StatusCode::BAD_REQUEST, "not a set code"));
    }
    let set = crate::sets::one(&code).ok_or_else(|| err(StatusCode::NOT_FOUND, "no such set"))?;
    Ok(Json(ConsoleSet {
        cards: crate::sets::cards(set),
        set: set.clone(),
    }))
}

/// `GET /admin/live`: who is here now and at which table, by handle.
///
/// @route GET /admin/live
/// @scope admin
/// @summary Who is here now, and at which table
/// @description Every account with a lobby socket open or a chair at a
/// table, by handle; the waiting rooms and then the running games, newest
/// first, with their chairs; and the engine agents connected.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn live(State(console): State<Admin>) -> Result<Json<ConsoleLive>, Refusal> {
    let app = &console.app;
    let tables = tables_now(app);
    let mut whereabouts: std::collections::BTreeMap<String, Whereabouts> = app
        .presence
        .accounts()
        .into_iter()
        .map(|id| {
            (
                id,
                Whereabouts {
                    lobby: true,
                    ..Whereabouts::default()
                },
            )
        })
        .collect();
    for table in &tables {
        for chair in &table.chairs {
            let sitter = chair
                .account
                .clone()
                .or_else(|| chair.delegate.as_ref().map(|(_, by)| by.clone()));
            if let Some(id) = sitter {
                let at = whereabouts.entry(id).or_default();
                if table.playing {
                    at.playing = Some(table.id.clone());
                } else {
                    at.waiting = Some(table.id.clone());
                }
            }
        }
    }
    let mut wanted: Vec<uuid::Uuid> = whereabouts
        .keys()
        .filter_map(|id| uuid::Uuid::parse_str(id).ok())
        .collect();
    wanted.extend(
        tables
            .iter()
            .filter_map(|t| t.host.as_deref())
            .filter_map(|id| uuid::Uuid::parse_str(id).ok()),
    );
    wanted.sort_unstable();
    wanted.dedup();
    let names: std::collections::HashMap<String, baylee_db::console::Named> =
        baylee_db::console::named(&app.db, &wanted)
            .await
            .map_err(|e| db_failed(&e))?
            .into_iter()
            .map(|(id, named)| (id.to_string(), named))
            .collect();
    let handle = |id: &str| {
        names
            .get(id)
            .map(|n| crate::handle::handle(&n.display_name, n.tag))
    };
    let guest = |id: &str| names.get(id).map(|n| n.guest);

    let players: Vec<LivePlayer> = whereabouts
        .iter()
        .filter(|(id, _)| names.contains_key(*id))
        .map(|(id, at)| LivePlayer {
            id: id.clone(),
            handle: handle(id),
            guest: guest(id),
            in_lobby: at.lobby,
            playing: at.playing.clone(),
            waiting: at.waiting.clone(),
        })
        .collect();
    let tables: Vec<LiveTable> = tables
        .iter()
        .map(|t| table_json(t, &handle, &guest))
        .collect();
    Ok(Json(ConsoleLive {
        at: iso(OffsetDateTime::now_utc()),
        players,
        tables,
        agents: agent_list(app),
    }))
}

/// `GET /admin/live`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleLive {
    /// @description When it was read, RFC 3339 in UTC.
    /// @openapi format: date-time
    at: String,
    /// @description Every account with a lobby socket open or a chair at a
    /// waiting or running table.
    players: Vec<LivePlayer>,
    /// @description The waiting rooms first, then the running games,
    /// newest first.
    tables: Vec<LiveTable>,
    /// @description The engine agents connected now, by id.
    agents: Vec<LiveAgent>,
}

/// An account that is here now.
/// @openapi
#[derive(Serialize)]
pub(crate) struct LivePlayer {
    /// @description The account's id.
    /// @openapi format: uuid
    id: String,
    /// @description Its handle, `Name#tag`.
    handle: Option<String>,
    /// @description Whether it is a guest.
    guest: Option<bool>,
    /// @description Whether a lobby socket of it is open.
    in_lobby: bool,
    /// @description The id of the running game it sits in, or `null`.
    playing: Option<String>,
    /// @description The id of the waiting room it sits in, or `null`.
    waiting: Option<String>,
}

/// A table of the live view.
/// @openapi
#[expect(
    clippy::struct_excessive_bools,
    reason = "a table's facts the console shows side by side"
)]
#[derive(Serialize)]
pub(crate) struct LiveTable {
    /// @description The game's id.
    id: String,
    /// @description The room's name.
    name: String,
    /// @description Whether it is waiting for players or being played.
    /// @openapi enum: [waiting, playing]
    state: &'static str,
    /// @description The host's handle, or `null`.
    host: Option<String>,
    /// @description The host's account id, or `null`.
    host_id: Option<String>,
    /// @description When the room was opened, RFC 3339 in UTC.
    /// @openapi format: date-time
    created_at: String,
    /// @description Whether the room has a password (never what).
    locked: bool,
    /// @description Whether it is a rematch of an earlier game.
    rematch: bool,
    /// @description Seconds a player has for each decision.
    decide_secs: u32,
    /// @description Whether its engine is attached.
    engine: bool,
    /// @description Whether its engine runs on this machine.
    engine_local: bool,
    /// @description The engine agent running it, by name (its id when the
    /// agent is gone), or `null`.
    agent: Option<String>,
    /// @description Its chairs.
    seats: Vec<LiveSeat>,
}

/// A chair of a table in the live view.
/// @openapi
#[derive(Serialize)]
pub(crate) struct LiveSeat {
    /// @description The chair's number.
    seat: usize,
    /// @description Whether the chair is for a person or the house AI.
    kind: crate::lobby::SeatKind,
    /// @description The house AI's profile, `null` for a person's chair.
    ai: Option<String>,
    /// @description The account in it, or that handed it to a seat bridge;
    /// `null` for none.
    account_id: Option<String>,
    /// @description The handle of the account in it, or `null`.
    player: Option<String>,
    /// @description Whether that account is a guest.
    guest: Option<bool>,
    /// @description The name a seat bridge sits under (`LLM-…`), or
    /// `null`.
    bridge: Option<String>,
    /// @description The handle of the account that handed the chair to the
    /// bridge, or `null`.
    bridged_by: Option<String>,
    /// @description The hosted model ordered for this chair, or `null`.
    /// @openapi
    /// ```yaml
    /// type: [object, 'null']
    /// properties:
    ///   profile: { type: string }
    ///   label: { type: string }
    ///   vendor: { type: string }
    ///   model: { type: string }
    ///   state: { type: string, enum: [starting, ready, failed] }
    ///   note: { type: [string, 'null'] }
    /// ```
    hosted: serde_json::Value,
    /// @description The deck's name.
    deck: String,
    /// @description The deck's format, or `null`.
    format: Option<String>,
    /// @description Whether the chair is ready.
    ready: bool,
    /// @description The chair's team, `null` for none.
    team: Option<u8>,
}

/// An engine agent in the live view.
/// @openapi
#[derive(Serialize)]
pub(crate) struct LiveAgent {
    /// @description The agent's id.
    id: String,
    /// @description Its name (`BAYLEE_AGENT_NAME`).
    name: String,
    /// @description Whether it came in on this machine's unix socket.
    local: bool,
    /// @description How many games it may run at once; `0` for no bound.
    capacity: u32,
    /// @description How many it runs.
    games: usize,
}

/// One table of the live view, its chairs named by `handle`.
fn table_json(
    t: &TableNow,
    handle: &impl Fn(&str) -> Option<String>,
    guest: &impl Fn(&str) -> Option<bool>,
) -> LiveTable {
    let seats: Vec<LiveSeat> = t
        .chairs
        .iter()
        .map(|c| {
            let sitter = c
                .account
                .as_deref()
                .or_else(|| c.delegate.as_ref().map(|(_, by)| by.as_str()));
            LiveSeat {
                seat: c.seat,
                kind: c.kind,
                ai: c.ai.clone(),
                account_id: sitter.map(str::to_owned),
                player: c.account.as_deref().and_then(handle),
                guest: sitter.and_then(guest),
                bridge: c.delegate.as_ref().map(|(name, _)| name.clone()),
                bridged_by: c.delegate.as_ref().and_then(|(_, by)| handle(by)),
                hosted: c.hosted.clone(),
                deck: c.deck.clone(),
                format: c.format.clone(),
                ready: c.ready,
                team: c.team,
            }
        })
        .collect();
    LiveTable {
        id: t.id.clone(),
        name: t.name.clone(),
        state: if t.playing { "playing" } else { "waiting" },
        host: t.host.as_deref().and_then(handle),
        host_id: t.host.clone(),
        created_at: iso_secs(t.created_at),
        locked: t.locked,
        rematch: t.rematch,
        decide_secs: t.decide_secs,
        engine: t.engine,
        engine_local: t.engine_local,
        agent: t.agent.clone(),
        seats,
    }
}

/// The agents connected now, one row each.
fn agent_list(app: &crate::AppState) -> Vec<LiveAgent> {
    let agents = app.agents.lock();
    let mut list: Vec<_> = agents.connected.iter().collect();
    list.sort_by(|a, b| a.0.cmp(b.0));
    list.into_iter()
        .map(|(id, agent)| LiveAgent {
            id: id.clone(),
            name: agent.name.clone(),
            local: agent.local,
            capacity: agent.capacity,
            games: agent.games.len(),
        })
        .collect()
}

/// What `GET /admin/accounts` reads from its query string.
/// @openapi
#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct AccountsQuery {
    /// @description Part of a username or display name, a `#tag` in hex,
    /// or an account's id; at most 100 characters. Empty finds every
    /// account.
    q: String,
    /// @description Exactly one account by its tag: `af03`, `#af03` or a
    /// whole handle `Alice#af03` (the name is not checked); registered or
    /// guest. At most 100 characters.
    /// @internal
    /// Exactly one tag (`af03`, `#af03` or a whole handle `Alice#af03`):
    /// the sender of a reported chat message, whose tag the report names.
    tag: String,
    /// @description Which accounts: `all` (the default), `registered` or
    /// `guest`.
    /// @openapi enum: [all, registered, guest]
    kind: String,
    /// @description The order: `newest` (the default), `oldest`, `name`,
    /// `games` or `active`.
    /// @openapi enum: [newest, oldest, name, games, active]
    sort: String,
    /// @description Only accounts with a lobby socket open or a chair at a
    /// table.
    online: bool,
    /// @description How many accounts of the order to skip.
    offset: u32,
    /// @description How many to answer, 1 to 200 (a number outside is
    /// taken as the nearest bound); 50 when absent.
    /// @openapi
    /// ```yaml
    /// minimum: 1
    /// maximum: 200
    /// ```
    limit: Option<u32>,
}

/// The accounts with a lobby socket open or a chair at a table.
fn online_ids(app: &crate::AppState) -> std::collections::BTreeSet<String> {
    let mut ids = app.presence.accounts();
    ids.extend(app.lobby.lock().seated_accounts());
    ids
}

/// The tag `typed` names exactly: `af03`, `#af03` or `Alice#af03` (the name
/// is not checked: a tag is unique, and a name may have changed since the
/// handle was seen). `None` for anything else.
fn exact_tag(typed: &str) -> Option<i32> {
    let typed = typed.trim();
    if typed.contains(crate::handle::SEP) {
        crate::handle::parse_tag(typed)
    } else {
        crate::handle::parse_tag(&format!("{}{typed}", crate::handle::SEP))
    }
}

/// `GET /admin/accounts?q=&tag=&kind=&sort=&online=&offset=&limit=`: a page
/// of accounts, each with whether it is online now. `tag` narrows it to the
/// one account with that tag, registered or guest.
///
/// @route GET /admin/accounts
/// @scope admin
/// @summary A page of accounts
/// @description A page of accounts, registered and guests, each with
/// whether it is online now, and how many match in all. Never a password
/// hash, a token or an e-mail address (only whether there is one).
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @query-params `$AccountsQuery`
/// @return 400: `$ErrorBody` "A `kind`, `sort` or `tag` it does not know, or a search over 100 characters (a parameter of the wrong type is refused in plain text)"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn list_accounts(
    State(console): State<Admin>,
    axum::extract::Query(query): axum::extract::Query<AccountsQuery>,
) -> Result<Json<ConsoleAccounts>, Refusal> {
    let app = &console.app;
    let kind = baylee_db::console::Kind::parse(&query.kind)
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "kind is all, registered or guest"))?;
    let order = baylee_db::console::Order::parse(&query.sort).ok_or_else(|| {
        err(
            StatusCode::BAD_REQUEST,
            "sort is newest, oldest, name, games or active",
        )
    })?;
    if query.q.chars().count() > 100 || query.tag.chars().count() > 100 {
        return Err(err(StatusCode::BAD_REQUEST, "the search is too long"));
    }
    let tag = if query.tag.trim().is_empty() {
        None
    } else {
        Some(exact_tag(&query.tag).ok_or_else(|| {
            err(
                StatusCode::BAD_REQUEST,
                "tag is a tag in hex, such as af03 or Alice#af03",
            )
        })?)
    };
    let online = online_ids(app);
    let playing = app.lobby.lock().playing_accounts();
    let in_lobby = app.presence.accounts();
    let only = query.online.then(|| {
        online
            .iter()
            .filter_map(|id| uuid::Uuid::parse_str(id).ok())
            .collect::<Vec<_>>()
    });
    let (total, rows) = baylee_db::console::accounts(
        &app.db,
        &baylee_db::console::Query {
            text: query.q,
            tag,
            kind,
            only,
            order,
            offset: query.offset,
            limit: query.limit.unwrap_or(50),
        },
        OffsetDateTime::now_utc(),
        lifetimes(),
    )
    .await
    .map_err(|e| db_failed(&e))?;
    let accounts: Vec<ConsoleAccount> = rows
        .into_iter()
        .map(|row| {
            let id = row.id.to_string();
            ConsoleAccount {
                handle: crate::handle::handle(&row.display_name, row.tag),
                in_lobby: in_lobby.contains(&id),
                playing: playing.contains(&id),
                online: online.contains(&id),
                account: row,
            }
        })
        .collect();
    Ok(Json(ConsoleAccounts {
        total,
        online: online.len(),
        accounts,
    }))
}

/// `GET /admin/accounts`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleAccounts {
    /// @description How many accounts match, over every page.
    total: i64,
    /// @description How many accounts are online now, matching or not.
    online: usize,
    /// @description This page of them.
    accounts: Vec<ConsoleAccount>,
}

/// One account as the console lists it.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleAccount {
    #[serde(flatten)]
    account: baylee_db::console::Account,
    /// @description Its handle, `Name#tag`.
    handle: String,
    /// @description Whether a lobby socket of it is open.
    in_lobby: bool,
    /// @description Whether it sits in a running game.
    playing: bool,
    /// @description Whether it has a lobby socket open or a chair at a
    /// table.
    online: bool,
}

/// `GET /admin/accounts/{id}`: one account with its decks and latest games.
///
/// @route GET /admin/accounts/{id}
/// @scope admin
/// @summary One account, its decks and latest games
/// @description The account's row as `GET /admin/accounts` lists it, the
/// table it sits at, its decks (without their cards) and its latest 25
/// games.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @path-param id: String format=uuid "The account's id"
/// @return 400: `$ErrorBody` "The id is not a UUID"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 404: `$ErrorBody` "No such account"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn one_account(
    State(console): State<Admin>,
    Path(id): Path<String>,
) -> Result<Json<ConsoleAccountDetail>, Refusal> {
    let app = &console.app;
    let uuid = uuid::Uuid::parse_str(&id)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "not an account's id"))?;
    let detail = baylee_db::console::account(&app.db, uuid, OffsetDateTime::now_utc(), lifetimes())
        .await
        .map_err(|e| db_failed(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such account"))?;
    let id = uuid.to_string();
    let in_lobby = app.presence.accounts().contains(&id);
    let table = {
        let lobby = app.lobby.lock();
        lobby
            .waiting()
            .chain(lobby.running())
            .find(|game| {
                game.seats.iter().any(|seat| {
                    seat.account_id.as_deref() == Some(id.as_str())
                        || seat.delegate.as_ref().is_some_and(|d| d.by == id)
                })
            })
            .map(|game| AccountTable {
                id: game.id.clone(),
                name: game.name.clone(),
                state: if game.state == crate::lobby::LobbyState::Playing {
                    "playing"
                } else {
                    "waiting"
                },
            })
    };
    Ok(Json(ConsoleAccountDetail {
        handle: crate::handle::handle(&detail.account.display_name, detail.account.tag),
        in_lobby,
        online: in_lobby || table.is_some(),
        table,
        detail,
    }))
}

/// `GET /admin/accounts/{id}`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleAccountDetail {
    #[serde(flatten)]
    detail: baylee_db::console::Detail,
    /// @description Its handle, `Name#tag`.
    handle: String,
    /// @description Whether a lobby socket of it is open.
    in_lobby: bool,
    /// @description The table it sits at, or `null`.
    table: Option<AccountTable>,
    /// @description Whether it has a lobby socket open or sits at a table.
    online: bool,
}

/// The table an account sits at.
/// @openapi
#[derive(Serialize)]
pub(crate) struct AccountTable {
    /// @description The game's id.
    id: String,
    /// @description The room's name.
    name: String,
    /// @description Whether it is waiting for players or being played.
    /// @openapi enum: [waiting, playing]
    state: &'static str,
}

/// One closed-beta key as the console lists it: never the key, which is
/// not kept.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ListedInvite {
    /// @description The key's id, which `DELETE /admin/invites/{id}` takes.
    id: uuid::Uuid,
    /// @description When it was made, RFC 3339 in UTC.
    /// @openapi format: date-time
    created_at: String,
    /// @description Who it is for, or `null`.
    note: Option<String>,
    /// @description How many more accounts it may admit.
    uses_left: i32,
    /// @description When it stops admitting anybody, RFC 3339 in UTC;
    /// `null` for never.
    /// @openapi format: date-time
    expires_at: Option<String>,
    /// @description When it was revoked, RFC 3339 in UTC, or `null`.
    /// @openapi format: date-time
    revoked_at: Option<String>,
    /// @description How many accounts that still exist it admitted.
    admitted: i64,
    /// @description What it can still do: `revoked`, `expired`, `used_up`
    /// (no use left) or `active`.
    /// @openapi
    /// ```yaml
    /// enum: [active, used_up, expired, revoked]
    /// ```
    state: &'static str,
}

/// One key as the console lists it: what `invite list` shows, never the
/// key, which is not kept.
fn listed(row: &baylee_db::invites::Invite, now: OffsetDateTime) -> ListedInvite {
    let state = if row.revoked_at.is_some() {
        "revoked"
    } else if row.expires_at.is_some_and(|at| at <= now) {
        "expired"
    } else if row.uses_left == 0 {
        "used_up"
    } else {
        "active"
    };
    ListedInvite {
        id: row.id,
        created_at: iso(row.created_at),
        note: row.note.clone(),
        uses_left: row.uses_left,
        expires_at: row.expires_at.map(iso),
        revoked_at: row.revoked_at.map(iso),
        admitted: row.admitted,
        state,
    }
}

/// `GET /admin/invites`: every key but the keys, newest first.
///
/// @route GET /admin/invites
/// @scope admin
/// @summary Every closed-beta key, never the keys
/// @description Every key made, newest first, with what it can still do.
/// Never a key: only its hash is stored, so no route can show one again.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn list_invites(State(console): State<Admin>) -> Result<Json<Vec<ListedInvite>>, Refusal> {
    let now = OffsetDateTime::now_utc();
    let rows = baylee_db::invites::list(&console.app.db)
        .await
        .map_err(|e| db_failed(&e))?;
    Ok(Json(
        rows.iter().rev().map(|row| listed(row, now)).collect(),
    ))
}

/// What `POST /admin/invites` reads: `invite create`'s flags as fields.
///
/// @description `baylee-gateway invite create`'s flags as fields, read by
/// the command's own parser: the same defaults, bounds and refusals.
/// Every field may be left out.
/// @openapi
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateBody {
    /// @description How many accounts each key admits, 1 to 1000; 1 when
    /// absent.
    /// @openapi minimum: 1
    /// @openapi maximum: 1000
    uses: Option<i64>,
    /// @description How long the keys admit anybody, in days or hours, 1
    /// to 3650 of them: `30d`, `12h`; for ever when absent.
    /// @openapi example: 30d
    expires: Option<String>,
    /// @description Who the keys are for: one line, at most 100
    /// characters.
    note: Option<String>,
    /// @description How many keys, 1 to 100; 1 when absent.
    /// @openapi minimum: 1
    /// @openapi maximum: 100
    count: Option<i64>,
}

/// `POST /admin/invites`: the keys made, the one time they are shown.
/// @openapi
#[derive(Serialize)]
pub(crate) struct MadeInvites {
    /// @description Each key and its id.
    keys: Vec<MadeInvite>,
    /// @description How many accounts each key admits.
    uses: i32,
    /// @description When they stop admitting anybody, RFC 3339 in UTC;
    /// `null` for never.
    /// @openapi format: date-time
    expires_at: Option<String>,
    /// @description Who they are for, `null` for nobody named.
    note: Option<String>,
}

/// One key made.
/// @openapi
#[derive(Serialize)]
pub(crate) struct MadeInvite {
    /// @description Its id.
    id: uuid::Uuid,
    /// @description The key, to hand on: it is never shown again.
    key: String,
}

/// `POST /admin/invites`: makes keys as `invite create` does, and answers
/// them, the one time they are shown.
///
/// @route POST /admin/invites
/// @scope admin
/// @summary Make closed-beta keys
/// @description Makes keys as `baylee-gateway invite create` does, in one
/// transaction, and answers them: the one time they exist outside the hand
/// they are given to.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @header-param X-Baylee-Admin: String required `pattern=^[\w.-]{1,64}$` "The acting admin's name, for the audit line; ignored when the bearer is a Central admin lease, whose account acts"
/// @body `$CreateBody`
/// @return 201: `$MadeInvites` "The keys, shown this once"
/// @return 400: `$ErrorBody` "No valid `X-Baylee-Admin`, not a request for keys (an unknown field included), or a value `invite create` refuses, in its words"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 413: "The body is over 16 KiB"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn create_invites(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    body: Bytes,
) -> Result<Response, Refusal> {
    let body: CreateBody = serde_json::from_slice(&body)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "not a request for keys"))?;
    let order = invite::order(
        body.uses,
        body.expires.as_deref(),
        body.note.as_deref(),
        body.count,
    )
    .map_err(|why| err_saying(StatusCode::BAD_REQUEST, why))?;
    let expires_at = order.expires.map(|d| OffsetDateTime::now_utc() + d);
    let made = invite::make(
        &console.app.db,
        order.uses,
        expires_at,
        order.note.as_deref(),
        order.count,
    )
    .await
    .map_err(|why| {
        tracing::error!("admin console: {why}");
        err(
            StatusCode::SERVICE_UNAVAILABLE,
            "the gateway's database is unavailable",
        )
    })?;
    let ids: Vec<String> = made.iter().map(|(id, _)| id.to_string()).collect();
    tracing::info!(
        target: "baylee_gateway::audit",
        admin = %actor,
        action = "invite.create",
        count = order.count,
        uses = order.uses,
        expires = %expires_at.map_or_else(|| "never".into(), iso),
        ids = %ids.join(","),
        "admin console: closed beta keys made"
    );
    let keys: Vec<MadeInvite> = made
        .into_iter()
        .map(|(id, key)| MadeInvite { id, key })
        .collect();
    Ok((
        StatusCode::CREATED,
        Json(MadeInvites {
            keys,
            uses: order.uses,
            expires_at: expires_at.map(iso),
            note: order.note.filter(|n| !n.is_empty()),
        }),
    )
        .into_response())
}

/// `DELETE /admin/invites/{id}`: revokes a key as `invite revoke` does.
/// The row stays, as it does there: a revoked key is still listed.
///
/// @route DELETE /admin/invites/{id}
/// @scope admin
/// @summary Revoke a closed-beta key
/// @description Revokes a key as `baylee-gateway invite revoke` does; it
/// admits nobody from now on and stays listed, revoked.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @header-param X-Baylee-Admin: String required `pattern=^[\w.-]{1,64}$` "The acting admin's name, for the audit line; ignored when the bearer is a Central admin lease, whose account acts"
/// @path-param id: String format=uuid "The key's id"
/// @return 204: "Revoked"
/// @return 400: `$ErrorBody` "No valid `X-Baylee-Admin`, or an id that is not a UUID"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 404: `$ErrorBody` "No key by that id that is not revoked already"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn revoke_invite(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    Path(id): Path<String>,
) -> Result<StatusCode, Refusal> {
    let id = uuid::Uuid::parse_str(&id).map_err(|_| {
        err_saying(
            StatusCode::BAD_REQUEST,
            format!("{id:?} is not a key's id; `invite list` shows them"),
        )
    })?;
    let revoked = baylee_db::invites::revoke(&console.app.db, id, OffsetDateTime::now_utc())
        .await
        .map_err(|e| db_failed(&e))?;
    if !revoked {
        return Err(err_saying(
            StatusCode::NOT_FOUND,
            format!("no key {id} that is not revoked already"),
        ));
    }
    tracing::info!(
        target: "baylee_gateway::audit",
        admin = %actor,
        action = "invite.revoke",
        id = %id,
        "admin console: closed beta key revoked"
    );
    Ok(StatusCode::NO_CONTENT)
}

// ------------------------------------------------------------ blocked cards

/// The most cards `GET /admin/cards/search` answers.
const SEARCH_LIMIT: usize = 25;

/// The latest a block may end: ten years ahead.
const MAX_BLOCK_DAYS: i64 = 3650;

/// What `GET /admin/cards/search` reads.
/// @openapi
#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct CardQuery {
    /// @description Part of a card's English name (any case), or its
    /// index; at most 100 characters. Empty finds nothing.
    q: String,
}

/// `GET /admin/cards/search`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct FoundCards {
    /// @description The cards found, at most 25.
    cards: Vec<FoundCard>,
}

/// A card of the pool `GET /admin/cards/search` found.
/// @openapi
#[derive(Serialize)]
pub(crate) struct FoundCard {
    /// @description Its `CardIndex`, which a block takes.
    index: u32,
    /// @description Its English name.
    name: String,
    /// @description Its printed type line.
    type_line: String,
    /// @description Its mana cost (`docs/mana-notation.md`), empty for a
    /// land.
    mana_cost: String,
    /// @description How far this build plays it.
    /// @openapi enum: [implemented, generated, partial, unimplemented]
    coverage: &'static str,
    /// @description The Scryfall id of its printing in the pool.
    scryfall_id: &'static str,
    /// @description The block in force on it, or `null`.
    blocked: Option<BlockInForce>,
}

/// A block in force, as a player is told it.
/// @openapi
#[derive(Serialize)]
pub(crate) struct BlockInForce {
    /// @description The card's `CardIndex`.
    index: u32,
    /// @description The card's English name.
    name: String,
    /// @description Why, in the admin's words, which the players are told.
    reason: String,
    /// @description When it ends by itself, RFC 3339 in UTC; `null` for
    /// never.
    /// @openapi format: date-time
    until: Option<String>,
}

/// `GET /admin/cards/search?q=`: the pool's cards whose English name holds
/// `q` (any case), or whose index is `q`; those starting with it first,
/// then by name; at most [`SEARCH_LIMIT`]. Each says whether it is blocked
/// now. Only the pool: a card this build cannot play is in no deck here.
///
/// @route GET /admin/cards/search
/// @scope admin
/// @summary Find cards of the pool to block
/// @description The pool's cards whose English name holds `q` (any case)
/// or whose index is `q`, those starting with it first, then by name, at
/// most 25; each with the block in force on it, if any.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @query-params `$CardQuery`
/// @return 400: `$ErrorBody` "The search is over 100 characters"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "Baylee Central or the gateway's database is unavailable while an admin lease is checked"
async fn search_cards(
    State(console): State<Admin>,
    axum::extract::Query(query): axum::extract::Query<CardQuery>,
) -> Result<Json<FoundCards>, Refusal> {
    let q = query.q.trim();
    if q.chars().count() > 100 {
        return Err(err(StatusCode::BAD_REQUEST, "the search is too long"));
    }
    if q.is_empty() {
        return Ok(Json(FoundCards { cards: Vec::new() }));
    }
    let needle = q.to_lowercase();
    let by_index = q.parse::<u32>().ok();
    let mut found: Vec<(bool, &baylee_cards::pool::PoolCard)> = baylee_cards::pool::rows()
        .iter()
        .filter_map(|card| {
            let name = card.english_name.to_lowercase();
            if by_index == Some(card.index) {
                Some((true, card))
            } else if name.contains(&needle) {
                Some((name.starts_with(&needle), card))
            } else {
                None
            }
        })
        .collect();
    found.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.english_name.cmp(&b.1.english_name))
            .then(a.1.index.cmp(&b.1.index))
    });
    let now = OffsetDateTime::now_utc();
    let cards: Vec<FoundCard> = found
        .into_iter()
        .take(SEARCH_LIMIT)
        .map(|(_, card)| FoundCard {
            index: card.index,
            name: card.english_name.clone(),
            type_line: card.type_line.clone(),
            mana_cost: card.mana_cost.clone(),
            coverage: card.coverage,
            scryfall_id: card.scryfall_id,
            blocked: console
                .app
                .blocks
                .of(card.index, now)
                .map(|b| BlockInForce {
                    index: b.index,
                    name: b.name,
                    reason: b.reason,
                    until: b.until.map(crate::blocks::iso),
                }),
        })
        .collect();
    Ok(Json(FoundCards { cards }))
}

/// One card block as the console lists it.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ListedBlock {
    /// @description The block's id, which lifting it takes.
    id: uuid::Uuid,
    /// @description The card's `CardIndex`.
    index: u32,
    /// @description The card's English name: this build's, else the one
    /// stored with the block.
    name: String,
    /// @description Why, in the admin's words, which the players are told.
    reason: String,
    /// @description When it was made, RFC 3339 in UTC.
    /// @openapi format: date-time
    created_at: String,
    /// @description The admin who made it.
    created_by: String,
    /// @description When it ends by itself, RFC 3339 in UTC; `null` for
    /// never.
    /// @openapi format: date-time
    until: Option<String>,
    /// @description When it was lifted, RFC 3339 in UTC, or `null`.
    /// @openapi format: date-time
    lifted_at: Option<String>,
    /// @description The admin who lifted it, or `null`.
    lifted_by: Option<String>,
    /// @description `active`, `expired` (its `until` passed) or `lifted`.
    /// @openapi enum: [active, expired, lifted]
    state: &'static str,
}

/// One block as the console lists it.
fn block_listed(row: &baylee_db::card_blocks::CardBlock, now: OffsetDateTime) -> ListedBlock {
    let state = if row.lifted_at.is_some() {
        "lifted"
    } else if row.in_force(now) {
        "active"
    } else {
        "expired"
    };
    ListedBlock {
        id: row.id,
        index: row.card_index,
        name: crate::blocks::pool_name(row.card_index)
            .unwrap_or(&row.card_name)
            .to_owned(),
        reason: row.reason.clone(),
        created_at: iso(row.created_at),
        created_by: row.created_by.clone(),
        until: row.until.map(iso),
        lifted_at: row.lifted_at.map(iso),
        lifted_by: row.lifted_by.clone(),
        state,
    }
}

/// `GET /admin/cards/blocks`: every block made, newest first, with its
/// `state`: `active`, `expired` or `lifted`.
///
/// @route GET /admin/cards/blocks
/// @scope admin
/// @summary Every card block made
/// @description Every card block made, newest first, in force or not:
/// the list is also the history.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn list_blocks(State(console): State<Admin>) -> Result<Json<Vec<ListedBlock>>, Refusal> {
    let now = OffsetDateTime::now_utc();
    let rows = baylee_db::card_blocks::list(&console.app.db)
        .await
        .map_err(|e| db_failed(&e))?;
    Ok(Json(
        rows.iter().map(|row| block_listed(row, now)).collect(),
    ))
}

/// What `POST /admin/cards/blocks` reads.
/// @openapi
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct BlockBody {
    /// @description The card's `CardIndex`; a card of this build's pool.
    index: u32,
    /// @description Why, which the players are told: 1 to 200 characters
    /// after trimming, one line.
    reason: String,
    /// @description When the block ends, RFC 3339, in the future and at
    /// most ten years ahead; absent or `null` for a block without an end.
    /// @openapi format: date-time
    /// @internal
    /// RFC 3339; absent or `null` for a block without an end.
    #[serde(default)]
    until: Option<String>,
}

/// The reason, trimmed and checked: 1 to 200 characters, no control
/// characters.
fn block_reason(reason: &str) -> Result<String, Refusal> {
    let reason = reason.trim();
    let chars = reason.chars().count();
    if chars == 0 || chars > baylee_db::card_blocks::MAX_REASON_CHARS {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "a block gives its reason in 1-200 characters",
        ));
    }
    if reason.chars().any(char::is_control) {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "a reason is one line without control characters",
        ));
    }
    Ok(reason.to_owned())
}

/// When a block ends: an RFC 3339 moment after `now` and at most
/// [`MAX_BLOCK_DAYS`] ahead; `None` for never.
fn block_until(
    until: Option<&str>,
    now: OffsetDateTime,
) -> Result<Option<OffsetDateTime>, Refusal> {
    let Some(until) = until.map(str::trim).filter(|u| !u.is_empty()) else {
        return Ok(None);
    };
    let at = OffsetDateTime::parse(until, &time::format_description::well_known::Rfc3339).map_err(
        |_| {
            err(
                StatusCode::BAD_REQUEST,
                "until is a moment in RFC 3339, such as 2026-10-20T18:00:00Z",
            )
        },
    )?;
    if at <= now {
        return Err(err(StatusCode::BAD_REQUEST, "until is already past"));
    }
    if at > now + time::Duration::days(MAX_BLOCK_DAYS) {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "until is more than ten years ahead; block it without an end instead",
        ));
    }
    Ok(Some(at))
}

/// `POST /admin/cards/blocks`: blocks a card of the pool, for good or until
/// a moment. `201` with the block as listed; `409` when the card is blocked
/// already (lift that one first); `404` for an index the pool has not.
///
/// @route POST /admin/cards/blocks
/// @scope admin
/// @summary Block a card on this gateway
/// @description Blocks a card of the pool, for good or until a moment: a
/// deck holding it does not sit down at a table here, and its players are
/// told the reason. A game already running is not touched.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @header-param X-Baylee-Admin: String required `pattern=^[\w.-]{1,64}$` "The acting admin's name, for the audit line and the block's `created_by`; ignored when the bearer is a Central admin lease, whose account acts"
/// @body `$BlockBody`
/// @return 201: `$ListedBlock` "The block, as listed"
/// @return 400: `$ErrorBody` "No valid `X-Baylee-Admin`, not a block (an unknown field included), a reason outside 1-200 characters or of two lines, or an `until` that is not RFC 3339, has passed or is over ten years ahead"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 404: `$ErrorBody` "No card of this pool has that index"
/// @return 409: `$ErrorBody` "The card has a block in force already; lift it first"
/// @return 413: "The body is over 16 KiB"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn block_card(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    body: Bytes,
) -> Result<Response, Refusal> {
    let body: BlockBody = serde_json::from_slice(&body)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "not a request to block a card"))?;
    let now = OffsetDateTime::now_utc();
    let reason = block_reason(&body.reason)?;
    let until = block_until(body.until.as_deref(), now)?;
    let name = crate::blocks::pool_name(body.index)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no card of this pool has that index"))?;
    let made = baylee_db::card_blocks::create(
        &console.app.db,
        &baylee_db::card_blocks::NewBlock {
            card_index: body.index,
            card_name: name.to_owned(),
            reason,
            created_by: actor.clone(),
            until,
        },
        now,
    )
    .await
    .map_err(|e| db_failed(&e))?
    .ok_or_else(|| {
        err_saying(
            StatusCode::CONFLICT,
            format!("{name} is blocked already; lift that block first"),
        )
    })?;
    reload_blocks(&console.app).await;
    // Never the reason: like a key's note, it may name a person.
    tracing::info!(
        target: "baylee_gateway::audit",
        admin = %actor,
        action = "card.block",
        id = %made.id,
        index = body.index,
        card = %name,
        until = %until.map_or_else(|| "never".into(), iso),
        "admin console: card blocked"
    );
    Ok((StatusCode::CREATED, Json(block_listed(&made, now))).into_response())
}

/// `DELETE /admin/cards/blocks/{id}`: lifts a block in force. `204`; `404`
/// for no such block, or one lifted or ended already. The row stays.
///
/// @route DELETE /admin/cards/blocks/{id}
/// @scope admin
/// @summary Lift a card block
/// @description Lifts a block in force; its decks may sit down again at
/// once. The block stays listed, lifted.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @header-param X-Baylee-Admin: String required `pattern=^[\w.-]{1,64}$` "The acting admin's name, for the audit line and the block's `lifted_by`; ignored when the bearer is a Central admin lease, whose account acts"
/// @path-param id: String format=uuid "The block's id"
/// @return 204: "Lifted"
/// @return 400: `$ErrorBody` "No valid `X-Baylee-Admin`, or an id that is not a UUID"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 404: `$ErrorBody` "No block by that id that is still in force"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn unblock_card(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    Path(id): Path<String>,
) -> Result<StatusCode, Refusal> {
    let id =
        uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "not a block's id"))?;
    let lifted =
        baylee_db::card_blocks::lift(&console.app.db, id, &actor, OffsetDateTime::now_utc())
            .await
            .map_err(|e| db_failed(&e))?
            .ok_or_else(|| {
                err_saying(
                    StatusCode::NOT_FOUND,
                    format!("no block {id} that is still in force"),
                )
            })?;
    reload_blocks(&console.app).await;
    tracing::info!(
        target: "baylee_gateway::audit",
        admin = %actor,
        action = "card.unblock",
        id = %id,
        index = lifted.card_index,
        card = %lifted.card_name,
        "admin console: card block lifted"
    );
    Ok(StatusCode::NO_CONTENT)
}

/// Reads the blocks again after a change; a failure leaves the periodic
/// refresh (`blocks::spawn_refresh`) to catch up.
async fn reload_blocks(app: &crate::AppState) {
    if let Err(e) = app.blocks.reload(&app.db).await {
        tracing::error!("card blocks: could not read them again after a change: {e}");
    }
}

// ---------------------------------------------------------- hosted models

/// `GET /admin/llm/seathosts`: every seat agent connected, and what it
/// reported (`docs/protocol.md` §"Hosted language-model seats").
///
/// @route GET /admin/llm/seathosts
/// @scope admin
/// @summary Every seat agent connected
/// @description Every seat agent connected, with the profiles it
/// reported, as it reported them: the gateway derives none of it.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @return 200: `{ type: object, required: [seathosts], properties: { seathosts: { type: array, items: { type: object, required: [name, local, capacity, games, connected_secs, profiles], properties: { name: { type: string, description: "The seat agent's name" }, local: { type: boolean, description: "Whether it came in on this machine's unix socket" }, capacity: { type: integer, minimum: 0, description: "How many hosted games it may run at once" }, games: { type: integer, minimum: 0, description: "How many it runs" }, connected_secs: { type: integer, minimum: 0, description: "Seconds since it connected" }, profiles: { type: array, description: "Its profiles, as it reported them", items: { type: object } } } } } } }`
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "Baylee Central or the gateway's database is unavailable while an admin lease is checked"
async fn llm_seathosts(State(console): State<Admin>) -> Json<serde_json::Value> {
    Json(crate::seathost::seathosts_json(&console.app))
}

/// `GET /admin/llm/profiles`: one row per profile, over every seat agent.
///
/// @route GET /admin/llm/profiles
/// @scope admin
/// @summary Every hosted profile, over every seat agent
/// @description One row per profile id, ordered as the players' list,
/// with the state of each seat agent that holds it, as they reported it.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @return 200: `{ type: object, required: [profiles], properties: { profiles: { type: array, items: { allOf: [ { $ref: "#/components/schemas/HostedProfile" }, { type: object, required: [hosts], properties: { hosts: { type: array, description: "Each seat agent that holds the profile, and what it reported", items: { type: object, properties: { host: { type: string, description: "The seat agent's name" }, state: { type: string, enum: [available, busy, probing, exhausted, needs_login, failing, disabled] }, until_unix: { type: [integer, "null"], description: "When that state is expected to end, Unix seconds" }, games: { type: integer, minimum: 0 }, max_games: { type: [integer, "null"], minimum: 0 }, enabled: { type: boolean }, canary: { type: boolean }, caps: { description: "The profile's spend caps, as reported" }, spent: { description: "What it spent, as reported" }, key: { type: string, enum: [kept, absent, none_needed, unavailable], description: "Whether a key is kept; never the key" }, last_error: { type: [string, "null"] }, last_ok_unix: { type: [integer, "null"] }, fallback: { type: [string, "null"], description: "The definition's fallback profile" }, diverts: { type: [string, "null"], description: "The fallback new games start on while the profile is not available" } } } } } } ] } } } }`
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "Baylee Central or the gateway's database is unavailable while an admin lease is checked"
async fn llm_profiles(State(console): State<Admin>) -> Json<serde_json::Value> {
    Json(crate::seathost::profiles_json(&console.app))
}

/// One audit line for a change to a hosted profile: who, what, which seat
/// agent and profile; never a key, an address or a definition's text.
fn llm_audit(actor: &str, action: &str, host: &str, id: &str) {
    tracing::info!(
        target: "baylee_gateway::audit",
        admin = %actor,
        action,
        seathost = %host,
        profile = %id,
        "admin console: hosted profile changed"
    );
}

/// `PUT /admin/llm/seathosts/{host}/profiles/{id}`: written.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ProfileWritten {
    /// @description Always `true`: the seat agent wrote the definition.
    ok: bool,
}

/// `PUT /admin/llm/seathosts/{host}/profiles/{id}`: a profile's definition.
///
/// @route PUT /admin/llm/seathosts/{host}/profiles/{id}
/// @scope admin
/// @summary Write a hosted profile's definition
/// @description Hands a profile's definition to the seat agent, which
/// checks and writes it; answered once it did. Nothing secret is in a
/// definition: a key goes to `…/key`.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @header-param X-Baylee-Admin: String required `pattern=^[\w.-]{1,64}$` "The acting admin's name, for the audit line; ignored when the bearer is a Central admin lease, whose account acts"
/// @path-param host: String "The seat agent's name: 1-64 of A-Z a-z 0-9 . - _"
/// @path-param id: String "The profile's id: 1-64 of A-Z a-z 0-9 . - _"
/// @body `$Definition`
/// @return 200: `$ProfileWritten` "The seat agent wrote it"
/// @return 400: `$ErrorBody` "No valid `X-Baylee-Admin`, not a profile, a name or id outside 1-64 of A-Z a-z 0-9 . - _, or the seat agent's refusal in its words"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 404: `$ErrorBody` "No such seat agent connected"
/// @return 413: "The body is over 16 KiB"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "Baylee Central or the gateway's database is unavailable while an admin lease is checked"
/// @return 504: `$ErrorBody` "The seat agent did not answer within 10 seconds"
async fn llm_write(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    Path((host, id)): Path<(String, String)>,
    body: Bytes,
) -> Result<Json<ProfileWritten>, Refusal> {
    let definition: baylee_protocol::seathost::Definition = serde_json::from_slice(&body)
        .map_err(|e| err_saying(StatusCode::BAD_REQUEST, format!("not a profile: {e}")))?;
    crate::seathost::ask(
        &console.app,
        &host,
        &id,
        &actor,
        crate::seathost::Ask::Write(definition),
    )
    .await?;
    llm_audit(&actor, "llm.profile.write", &host, &id);
    Ok(Json(ProfileWritten { ok: true }))
}

/// `DELETE /admin/llm/seathosts/{host}/profiles/{id}`.
///
/// @route DELETE /admin/llm/seathosts/{host}/profiles/{id}
/// @scope admin
/// @summary Forget a hosted profile
/// @description The seat agent forgets the profile's definition; a key
/// kept for it stays in its store.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @header-param X-Baylee-Admin: String required `pattern=^[\w.-]{1,64}$` "The acting admin's name, for the audit line; ignored when the bearer is a Central admin lease, whose account acts"
/// @path-param host: String "The seat agent's name: 1-64 of A-Z a-z 0-9 . - _"
/// @path-param id: String "The profile's id: 1-64 of A-Z a-z 0-9 . - _"
/// @return 204: "Forgotten"
/// @return 400: `$ErrorBody` "No valid `X-Baylee-Admin`, a name or id outside 1-64 of A-Z a-z 0-9 . - _, or the seat agent's refusal in its words"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 404: `$ErrorBody` "No such seat agent connected"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "Baylee Central or the gateway's database is unavailable while an admin lease is checked"
/// @return 504: `$ErrorBody` "The seat agent did not answer within 10 seconds"
async fn llm_delete(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    Path((host, id)): Path<(String, String)>,
) -> Result<StatusCode, Refusal> {
    crate::seathost::ask(
        &console.app,
        &host,
        &id,
        &actor,
        crate::seathost::Ask::Delete,
    )
    .await?;
    llm_audit(&actor, "llm.profile.delete", &host, &id);
    Ok(StatusCode::NO_CONTENT)
}

/// What `…/enabled` takes.
/// @openapi
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct EnabledBody {
    /// @description Whether the profile is offered.
    enabled: bool,
}

/// `POST /admin/llm/seathosts/{host}/profiles/{id}/enabled`.
///
/// @route POST /admin/llm/seathosts/{host}/profiles/{id}/enabled
/// @scope admin
/// @summary Switch a hosted profile on or off
/// @description Tells the seat agent to offer the profile, or not.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @header-param X-Baylee-Admin: String required `pattern=^[\w.-]{1,64}$` "The acting admin's name, for the audit line; ignored when the bearer is a Central admin lease, whose account acts"
/// @path-param host: String "The seat agent's name: 1-64 of A-Z a-z 0-9 . - _"
/// @path-param id: String "The profile's id: 1-64 of A-Z a-z 0-9 . - _"
/// @body `$EnabledBody`
/// @return 204: "Done"
/// @return 400: `$ErrorBody` "No valid `X-Baylee-Admin`, not a body with one boolean `enabled`, a name or id outside 1-64 of A-Z a-z 0-9 . - _, or the seat agent's refusal in its words"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 404: `$ErrorBody` "No such seat agent connected"
/// @return 413: "The body is over 16 KiB"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "Baylee Central or the gateway's database is unavailable while an admin lease is checked"
/// @return 504: `$ErrorBody` "The seat agent did not answer within 10 seconds"
async fn llm_enabled(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    Path((host, id)): Path<(String, String)>,
    body: Bytes,
) -> Result<StatusCode, Refusal> {
    let body: EnabledBody = serde_json::from_slice(&body)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "send {\"enabled\": true|false}"))?;
    let (action, audit) = if body.enabled {
        (
            baylee_protocol::seathost::ControlAction::Enable,
            "llm.profile.enable",
        )
    } else {
        (
            baylee_protocol::seathost::ControlAction::Disable,
            "llm.profile.disable",
        )
    };
    crate::seathost::ask(
        &console.app,
        &host,
        &id,
        &actor,
        crate::seathost::Ask::Control(action),
    )
    .await?;
    llm_audit(&actor, audit, &host, &id);
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /admin/llm/seathosts/{host}/profiles/{id}/probe`.
///
/// @route POST /admin/llm/seathosts/{host}/profiles/{id}/probe
/// @scope admin
/// @summary Check a hosted profile now
/// @description Asks the seat agent to run the profile's check now, and
/// its paid canary where the profile has one; the result shows in the
/// profile's state later. No body.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @header-param X-Baylee-Admin: String required `pattern=^[\w.-]{1,64}$` "The acting admin's name, for the audit line; ignored when the bearer is a Central admin lease, whose account acts"
/// @path-param host: String "The seat agent's name: 1-64 of A-Z a-z 0-9 . - _"
/// @path-param id: String "The profile's id: 1-64 of A-Z a-z 0-9 . - _"
/// @return 202: "The seat agent took the order"
/// @return 400: `$ErrorBody` "No valid `X-Baylee-Admin`, a name or id outside 1-64 of A-Z a-z 0-9 . - _, or the seat agent's refusal in its words"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 404: `$ErrorBody` "No such seat agent connected"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "Baylee Central or the gateway's database is unavailable while an admin lease is checked"
/// @return 504: `$ErrorBody` "The seat agent did not answer within 10 seconds"
async fn llm_probe(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    Path((host, id)): Path<(String, String)>,
) -> Result<StatusCode, Refusal> {
    crate::seathost::ask(
        &console.app,
        &host,
        &id,
        &actor,
        crate::seathost::Ask::Control(baylee_protocol::seathost::ControlAction::Probe),
    )
    .await?;
    llm_audit(&actor, "llm.profile.probe", &host, &id);
    Ok(StatusCode::ACCEPTED)
}

/// What `…/key` takes: a key, or `null` to forget it. No `Debug`: nothing
/// may print it.
///
/// @description A provider's API key for a hosted profile, or `null` to
/// forget the one kept.
/// @openapi
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyBody {
    /// @description The key, trimmed; `null` forgets it. Never empty.
    key: Option<String>,
}

/// `POST /admin/llm/seathosts/{host}/profiles/{id}/key`: write-only. The
/// key goes to the seat agent over the unix socket and nowhere else; no
/// route answers it, the audit line names only that it was set.
///
/// @route POST /admin/llm/seathosts/{host}/profiles/{id}/key
/// @scope admin
/// @summary Set or forget a hosted profile's key
/// @description Write-only: the key goes to the seat agent, which keeps
/// it in its key store, and nowhere else; no route answers it, only
/// whether one is kept. Only for a seat agent on this machine's unix
/// socket.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @header-param X-Baylee-Admin: String required `pattern=^[\w.-]{1,64}$` "The acting admin's name, for the audit line; ignored when the bearer is a Central admin lease, whose account acts"
/// @path-param host: String "The seat agent's name: 1-64 of A-Z a-z 0-9 . - _"
/// @path-param id: String "The profile's id: 1-64 of A-Z a-z 0-9 . - _"
/// @body `$KeyBody`
/// @return 204: "Set, or forgotten"
/// @return 400: `$ErrorBody` "No valid `X-Baylee-Admin`, not a body with one `key`, an empty key, a name or id outside 1-64 of A-Z a-z 0-9 . - _, or the seat agent's refusal in its words"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 404: `$ErrorBody` "No such seat agent connected"
/// @return 413: "The body is over 16 KiB"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The seat agent is not on this machine's unix socket (keys travel only over it), or Baylee Central or the database is unavailable while an admin lease is checked"
/// @return 504: `$ErrorBody` "The seat agent did not answer within 10 seconds"
async fn llm_key(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    Path((host, id)): Path<(String, String)>,
    body: Bytes,
) -> Result<StatusCode, Refusal> {
    let KeyBody { key } = serde_json::from_slice(&body).map_err(|_| {
        err(
            StatusCode::BAD_REQUEST,
            "send {\"key\": \"…\"} or {\"key\": null}",
        )
    })?;
    let key = key.map(|k| k.trim().to_string());
    if key.as_ref().is_some_and(String::is_empty) {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "an empty key; send null to forget one",
        ));
    }
    let audit = if key.is_some() {
        "llm.key.set"
    } else {
        "llm.key.delete"
    };
    crate::seathost::ask(
        &console.app,
        &host,
        &id,
        &actor,
        crate::seathost::Ask::Key(key),
    )
    .await?;
    llm_audit(&actor, audit, &host, &id);
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "an-admin-token-of-thirty-two-chars-or-more";

    #[test]
    fn without_a_token_there_is_no_console() {
        assert!(matches!(Settings::from_env(None, None, &[]), Ok(None)));
        assert!(matches!(Settings::from_env(Some(""), None, &[]), Ok(None)));
        assert!(
            Settings::from_env(None, Some("127.0.0.1:1"), &[]).is_err(),
            "a bind with no token is a typo, not a console"
        );
    }

    #[test]
    fn a_token_is_long_one_word_and_no_other_secret() {
        let ok = Settings::from_env(Some(TOKEN), None, &[Some("agent"), None]).unwrap();
        let ok = ok.expect("a console");
        assert_eq!(ok.bind, DEFAULT_BIND.parse().unwrap());
        assert_eq!(ok.token, digest(TOKEN));
        assert!(Settings::from_env(Some(&"x".repeat(31)), None, &[]).is_err());
        assert!(Settings::from_env(Some(&"x".repeat(32)), None, &[]).is_ok());
        assert!(Settings::from_env(Some(&format!("{TOKEN} x")), None, &[]).is_err());
        assert!(Settings::from_env(Some(&format!("{TOKEN}\n")), None, &[]).is_err());
        for others in [[Some(TOKEN), None], [None, Some(TOKEN)]] {
            assert!(
                Settings::from_env(Some(TOKEN), None, &others).is_err(),
                "{others:?}"
            );
        }
    }

    #[test]
    fn the_console_binds_loopback_only() {
        for bind in ["127.0.0.1:0", "127.0.0.2:28767", "[::1]:28767"] {
            let settings = Settings::from_env(Some(TOKEN), Some(bind), &[]);
            assert!(matches!(settings, Ok(Some(_))), "{bind}");
        }
        for bind in [
            "0.0.0.0:28767",
            "[::]:28767",
            "192.168.0.1:28767",
            "localhost:28767",
            "127.0.0.1",
            "nonsense",
        ] {
            assert!(
                Settings::from_env(Some(TOKEN), Some(bind), &[]).is_err(),
                "{bind}"
            );
        }
    }

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(*k, v.parse().unwrap());
        }
        h
    }

    #[test]
    fn a_browsers_request_or_a_strange_host_is_refused() {
        fn ok(pairs: &[(&'static str, &'static str)]) -> bool {
            from_this_machine(&headers(pairs)).is_ok()
        }
        assert!(ok(&[("host", "127.0.0.1:28767")]));
        assert!(ok(&[("host", "localhost:28767")]));
        assert!(ok(&[("host", "LOCALHOST")]));
        assert!(ok(&[("host", "[::1]:28767")]));
        assert!(ok(&[("host", "127.0.0.1")]));
        assert!(!ok(&[]), "no host");
        assert!(!ok(&[("host", "evil.example:28767")]), "a rebound name");
        assert!(!ok(&[("host", "127.0.0.1.evil.example")]));
        assert!(!ok(&[("host", "192.168.0.10:28767")]));
        assert!(!ok(&[("host", "[::1")]));
        for mark in [
            ("origin", "http://127.0.0.1:28767"),
            ("origin", "null"),
            ("sec-fetch-site", "same-origin"),
            ("sec-fetch-mode", "navigate"),
        ] {
            assert!(!ok(&[("host", "127.0.0.1:28767"), mark]), "{mark:?}");
        }
    }

    #[test]
    fn only_the_token_itself_matches() {
        let known = digest(TOKEN);
        let with = |value: &'static str| headers(&[("authorization", value)]);
        assert!(token_matches(
            &known,
            &with("Bearer an-admin-token-of-thirty-two-chars-or-more")
        ));
        assert!(!token_matches(&known, &HeaderMap::new()));
        assert!(!token_matches(&known, &with("Bearer ")));
        assert!(!token_matches(
            &known,
            &with("Bearer an-admin-token-of-thirty-two-chars-or-mor")
        ));
        assert!(!token_matches(
            &known,
            &with("Bearer an-admin-token-of-thirty-two-chars-or-more!")
        ));
        assert!(!token_matches(
            &known,
            &with("an-admin-token-of-thirty-two-chars-or-more")
        ));
    }

    /// The console's page sends `Date.toISOString()`, fractional seconds
    /// included; an offset is RFC 3339 too.
    #[test]
    fn a_block_ends_at_any_rfc_3339_moment_ahead() {
        let now = OffsetDateTime::from_unix_timestamp(1_800_000_000).expect("a moment");
        for until in [
            "2027-01-20T18:00:00.000Z",
            "2027-01-20T18:00:00Z",
            "2027-01-20T20:00:00+02:00",
        ] {
            let at = block_until(Some(until), now)
                .unwrap_or_else(|_| panic!("{until} is refused"))
                .expect("an end");
            assert_eq!(iso(at), "2027-01-20T18:00:00Z", "{until}");
        }
        assert!(matches!(block_until(None, now), Ok(None)));
        assert!(matches!(block_until(Some(" "), now), Ok(None)));
        assert!(block_until(Some("2027-01-15T07:59:59Z"), now).is_err());
        assert!(block_until(Some("2027-01-20 18:00"), now).is_err());
    }

    #[test]
    fn an_actor_is_a_plain_name() {
        assert!(valid_actor("viktor"));
        assert!(valid_actor("e2e-admin.1_x"));
        assert!(valid_actor(&"a".repeat(64)));
        assert!(!valid_actor(&"a".repeat(65)));
        assert!(!valid_actor(""));
        assert!(!valid_actor("two words"));
        assert!(!valid_actor("ünïcode"));
        assert!(!valid_actor("line\nbreak"));
    }

    #[test]
    fn a_key_is_listed_by_what_it_can_still_do() {
        let at = OffsetDateTime::from_unix_timestamp(1_790_000_000).unwrap();
        let row = |uses_left, expires_at, revoked_at| baylee_db::invites::Invite {
            id: uuid::Uuid::nil(),
            created_at: at,
            note: Some("for Max".into()),
            uses_left,
            expires_at,
            revoked_at,
            admitted: 0,
        };
        let later = at + time::Duration::days(1);
        let state = |r| {
            serde_json::to_value(listed(&r, at)).unwrap()["state"]
                .as_str()
                .unwrap()
                .to_owned()
        };
        assert_eq!(state(row(1, None, None)), "active");
        assert_eq!(state(row(1, Some(later), None)), "active");
        assert_eq!(state(row(0, None, None)), "used_up");
        assert_eq!(state(row(1, Some(at), None)), "expired");
        assert_eq!(state(row(1, None, Some(at))), "revoked");
        let shown = serde_json::to_value(listed(&row(2, Some(later), None), at)).unwrap();
        assert_eq!(shown["created_at"], "2026-09-21T14:13:20Z");
        assert_eq!(shown["expires_at"], "2026-09-22T14:13:20Z");
        assert_eq!(shown["note"], "for Max");
        let mut fields: Vec<&str> = shown
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        fields.sort_unstable();
        assert_eq!(
            fields,
            [
                "admitted",
                "created_at",
                "expires_at",
                "id",
                "note",
                "revoked_at",
                "state",
                "uses_left"
            ],
            "no field could hold a key"
        );
    }
}
