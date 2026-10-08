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
//! 4. the bearer token is compared with `BAYLEE_ADMIN_TOKEN` in constant
//!    time, as SHA-256 digests, so neither its length nor where a guess
//!    first differs shows in the time an answer takes; missing and wrong
//!    get the same `401`;
//! 5. a request that changes something names its admin in
//!    [`ACTOR_HEADER`] (`400` without), which goes into the audit line.
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
use axum::routing::{delete, get};
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
struct Console {
    /// The gateway's state.
    app: Shared,
    /// SHA-256 of `BAYLEE_ADMIN_TOKEN`.
    token: [u8; 32],
    /// Wrong tokens.
    failures: auth::RateLimiter,
    /// Every request.
    requests: auth::RateLimiter,
}

type Admin = Arc<Console>;

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
        .route("/admin/invites", get(list_invites).post(create_invites))
        .route("/admin/invites/{id}", delete(revoke_invite))
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

type Refusal = (StatusCode, Json<ErrorBody>);

/// Who asked for a change, from [`ACTOR_HEADER`].
#[derive(Clone)]
struct Actor(String);

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
    if !token_matches(&console.token, headers) {
        console.failures.allow(EVERYONE);
        tracing::warn!("admin console: a request with a wrong or no token");
        return err(StatusCode::UNAUTHORIZED, "the admin token is needed").into_response();
    }
    let actor = headers
        .get(ACTOR_HEADER)
        .and_then(|v| v.to_str().ok())
        .filter(|name| valid_actor(name))
        .map(|name| Actor(name.to_owned()));
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
fn iso(at: OffsetDateTime) -> String {
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

fn since(counts: baylee_db::stats::Since) -> serde_json::Value {
    serde_json::json!({
        "today_utc": counts.today_utc,
        "last_7d": counts.last_7d,
        "last_30d": counts.last_30d,
    })
}

fn db_failed(e: &sea_orm::DbErr) -> Refusal {
    tracing::error!("admin console: {e}");
    err(
        StatusCode::SERVICE_UNAVAILABLE,
        "the gateway's database is unavailable",
    )
}

/// The agents connected now: how many, how many on this machine's unix
/// socket, how many games they may run together (`null` when one of them
/// has no bound) and how many they were asked to.
fn agents(app: &crate::AppState) -> serde_json::Value {
    let agents = app.agents.lock();
    let connected = agents.connected.values();
    let unlimited = connected.clone().any(|agent| agent.capacity == 0);
    serde_json::json!({
        "connected": agents.connected.len(),
        "local": connected.clone().filter(|agent| agent.local).count(),
        "capacity": (!unlimited).then(|| {
            connected.clone().map(|agent| u64::from(agent.capacity)).sum::<u64>()
        }),
        "games": connected.map(|agent| agent.games.len()).sum::<usize>(),
    })
}

/// `GET /admin/stats`: counts, and nothing that names anybody.
async fn stats(State(console): State<Admin>) -> Result<Json<serde_json::Value>, Refusal> {
    let app = &console.app;
    let now = OffsetDateTime::now_utc();
    let stored = baylee_db::stats::read(&app.db, now)
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
    let mut gateway = crate::routes::build_fields();
    gateway.insert(
        "name".into(),
        app.display_name
            .clone()
            .map_or(serde_json::Value::Null, Into::into),
    );
    gateway.insert("registration".into(), app.registration.wire().into());
    Ok(Json(serde_json::json!({
        "at": iso(now),
        "gateway": gateway,
        "accounts": {
            "registered": stored.registered,
            "with_email": stored.with_email,
            "confirmed_email": stored.confirmed_email,
            "admitted_by_key": stored.admitted_by_key,
            "created": since(stored.registered_since),
        },
        "guests": {
            "enabled": app.guests_enabled,
            "live": stored.guests,
            "cap": app.guest_cap,
        },
        "online": {
            "players": players,
            "in_lobby": in_lobby,
            "seated": seated,
            "sessions_live": stored.sessions_live,
            "accounts_signed_in": stored.accounts_signed_in,
        },
        "games": {
            "running": running,
            "local_running": local_running,
            "waiting": waiting,
            "seats_awaiting_engine": awaiting_engine,
            "recorded": stored.games_recorded,
            "started": since(stored.games_started),
            "finished": stored.games_finished,
            "finished_since": since(stored.games_finished_since),
        },
        "agents": agents(app),
        "invites": {
            "total": stored.invites_total,
            "active": stored.invites_active,
            "used_up": stored.invites_used_up,
            "expired": stored.invites_expired,
            "revoked": stored.invites_revoked,
            "uses_left": stored.invite_uses_left,
            "admitted": stored.admitted_by_key,
        },
    })))
}

/// One key as the console lists it: what `invite list` shows, never the
/// key, which is not kept.
fn listed(row: &baylee_db::invites::Invite, now: OffsetDateTime) -> serde_json::Value {
    let state = if row.revoked_at.is_some() {
        "revoked"
    } else if row.expires_at.is_some_and(|at| at <= now) {
        "expired"
    } else if row.uses_left == 0 {
        "used_up"
    } else {
        "active"
    };
    serde_json::json!({
        "id": row.id,
        "created_at": iso(row.created_at),
        "note": row.note,
        "uses_left": row.uses_left,
        "expires_at": row.expires_at.map(iso),
        "revoked_at": row.revoked_at.map(iso),
        "admitted": row.admitted,
        "state": state,
    })
}

/// `GET /admin/invites`: every key but the keys, newest first.
async fn list_invites(State(console): State<Admin>) -> Result<Json<serde_json::Value>, Refusal> {
    let now = OffsetDateTime::now_utc();
    let rows = baylee_db::invites::list(&console.app.db)
        .await
        .map_err(|e| db_failed(&e))?;
    Ok(Json(serde_json::Value::Array(
        rows.iter().rev().map(|row| listed(row, now)).collect(),
    )))
}

/// What `POST /admin/invites` reads: `invite create`'s flags as fields.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateBody {
    uses: Option<i64>,
    expires: Option<String>,
    note: Option<String>,
    count: Option<i64>,
}

/// `POST /admin/invites`: makes keys as `invite create` does, and answers
/// them, the one time they are shown.
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
    let keys: Vec<serde_json::Value> = made
        .into_iter()
        .map(|(id, key)| serde_json::json!({ "id": id, "key": key }))
        .collect();
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "keys": keys,
            "uses": order.uses,
            "expires_at": expires_at.map(iso),
            "note": order.note.filter(|n| !n.is_empty()),
        })),
    )
        .into_response())
}

/// `DELETE /admin/invites/{id}`: revokes a key as `invite revoke` does.
/// The row stays, as it does there: a revoked key is still listed.
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
        let state = |r| listed(&r, at)["state"].as_str().unwrap().to_owned();
        assert_eq!(state(row(1, None, None)), "active");
        assert_eq!(state(row(1, Some(later), None)), "active");
        assert_eq!(state(row(0, None, None)), "used_up");
        assert_eq!(state(row(1, Some(at), None)), "expired");
        assert_eq!(state(row(1, None, Some(at))), "revoked");
        let shown = listed(&row(2, Some(later), None), at);
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
