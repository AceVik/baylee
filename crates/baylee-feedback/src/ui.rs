//! The web UI's JSON routes (#311), under `/ui/api/`, for a signed-in admin.
//!
//! - `POST /ui/api/login` `{name, password}`: a session, as a cookie.
//! - `POST /ui/api/logout`: ends it, here and in the database.
//! - `GET /ui/api/me`: who is signed in.
//! - `GET /ui/api/reports?…`: the list, with the token API's filters plus
//!   `q`, `from`, `to`, `has_record` (`docs/feedback.md`).
//! - `GET /ui/api/reports/{id}`, `…/record`, `…/audit`.
//! - `PATCH /ui/api/reports/{id}` `{status?, issue?}`, `DELETE …`.
//! - `GET /ui/api/facets`: the gateways, reporters and statuses there are.
//!
//! A session is a random 256-bit token in an `HttpOnly; Secure;
//! SameSite=Strict` cookie, kept in the database only as its SHA-256. It
//! lapses after [`IDLE`] without use and [`ABSOLUTE`] after sign-in,
//! whichever comes first. Every route that changes something, sign-in and
//! sign-out included, also wants [`CSRF_HEADER`] and an `Origin` naming this
//! host: a form on another site can send neither.
//!
//! Sign-in is limited per address and per name ([`Limiter`]), in memory
//! only; nothing about a sign-in is stored but the session it makes.

use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, SocketAddr};
use std::path::{Path as FsPath, PathBuf};
use std::sync::{Arc, Mutex};

use axum::body::Bytes;
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use axum::{Extension, Router};
use sea_orm::{ConnectionTrait as _, DatabaseConnection, Statement};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::{
    AppState, Change, Filter, Listing, Refusal, Shared, Summary, admin, apply_change, db_down,
    delete_report, fetch, hash, listing, record_of, refuse, report_id,
};

/// A session unused this long is over.
pub const IDLE: Duration = Duration::hours(12);

/// A session this old is over, however busy.
pub const ABSOLUTE: Duration = Duration::days(7);

/// The cookie a session rides in. `__Host-` makes a browser refuse it
/// unless it is `Secure`, has `Path=/` and no `Domain`: no sibling host can
/// set or shadow it.
pub const COOKIE: &str = "__Host-baylee-feedback";

/// The header every changing request carries. A cross-site form cannot set
/// a header, and a cross-site `fetch` that sets one is preflighted, which
/// this service never answers with a yes.
pub const CSRF_HEADER: &str = "x-baylee-csrf";

/// How far back failed sign-ins count.
pub const WINDOW: Duration = Duration::minutes(15);

/// Failed sign-ins one address may have in [`WINDOW`].
pub const PER_ADDRESS: usize = 10;

/// Failed sign-ins one name may have in [`WINDOW`], from anywhere.
pub const PER_NAME: usize = 5;

/// The time, as the UI's sessions and limiter see it. A test hands in a
/// clock it moves itself.
pub type Clock = Arc<dyn Fn() -> OffsetDateTime + Send + Sync>;

/// The web UI's half of the service's state.
pub struct Ui {
    web_dir: Option<PathBuf>,
    trusted_proxies: Vec<IpAddr>,
    clock: Clock,
    limiter: Mutex<Limiter>,
    /// Direct reports per address and in all (`crate::direct`).
    direct: Mutex<crate::direct::Allowance>,
    /// Offline-game beats per address and in all (`crate::alive`).
    alive: Mutex<crate::direct::Allowance>,
    /// The offline games heard from lately (`crate::alive`).
    presence: Mutex<crate::alive::Presence>,
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            web_dir: None,
            trusted_proxies: Vec::new(),
            clock: Arc::new(OffsetDateTime::now_utc),
            limiter: Mutex::new(Limiter::default()),
            direct: Mutex::new(crate::direct::Allowance::default()),
            alive: Mutex::new(crate::direct::Allowance::default()),
            presence: Mutex::new(crate::alive::Presence::default()),
        }
    }
}

impl Ui {
    /// `FEEDBACK_WEB_DIR` (unset or empty: no UI is served, `/` is 404) and
    /// `FEEDBACK_TRUSTED_PROXIES` (comma-separated addresses whose
    /// `X-Forwarded-For` the sign-in limiter believes).
    ///
    /// # Errors
    ///
    /// When `FEEDBACK_WEB_DIR` names no directory with an `index.html`:
    /// better to refuse to start than to serve 404s under a UI's name.
    pub fn from_env() -> anyhow::Result<Self> {
        let var = |name: &str| std::env::var(name).ok().filter(|v| !v.trim().is_empty());
        let mut ui = Self::default();
        if let Some(dir) = var("FEEDBACK_WEB_DIR") {
            ui = ui.with_web_dir(dir)?;
        }
        if let Some(list) = var("FEEDBACK_TRUSTED_PROXIES") {
            ui.trusted_proxies = trusted_proxies(&list)?;
        }
        Ok(ui)
    }

    /// Serves the built UI from `dir`.
    ///
    /// # Errors
    ///
    /// When `dir` has no `index.html`.
    pub fn with_web_dir(mut self, dir: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let dir: PathBuf = dir.into();
        let dir = dir
            .canonicalize()
            .map_err(|e| anyhow::anyhow!("FEEDBACK_WEB_DIR {}: {e}", dir.display()))?;
        if !dir.join("index.html").is_file() {
            anyhow::bail!("FEEDBACK_WEB_DIR {} holds no index.html", dir.display());
        }
        self.web_dir = Some(dir);
        Ok(self)
    }

    /// Reads the time from `clock` instead of the system's.
    #[must_use]
    pub fn with_clock(mut self, clock: Clock) -> Self {
        self.clock = clock;
        self
    }

    /// Believes `X-Forwarded-For` from these peers.
    #[must_use]
    pub fn with_trusted_proxies(mut self, proxies: Vec<IpAddr>) -> Self {
        self.trusted_proxies = proxies;
        self
    }

    /// The directory the UI is served from, if any.
    #[must_use]
    pub fn web_dir(&self) -> Option<&FsPath> {
        self.web_dir.as_deref()
    }

    fn now(&self) -> OffsetDateTime {
        (self.clock)()
    }

    /// The address a request is counted under ([`client_address`]): kept
    /// in memory for the allowances' window, never written down.
    pub(crate) fn address(&self, peer: Option<IpAddr>, headers: &HeaderMap) -> String {
        client_address(&self.trusted_proxies, peer, headers)
    }

    /// Whether an offline game's beat from `address` may be taken now;
    /// counted.
    pub(crate) fn allow_alive(&self, address: &str) -> bool {
        let now = self.now();
        self.alive
            .lock()
            .expect("the allowance is never poisoned")
            .take_within(
                address,
                now,
                crate::alive::PER_ADDRESS,
                crate::alive::PER_SERVICE,
            )
    }

    /// An offline game is still going.
    pub(crate) fn beat(&self, game: &str) {
        let now = self.now();
        self.presence
            .lock()
            .expect("the presence is never poisoned")
            .beat(game, now);
    }

    /// How many offline games are going now (`crate::alive`).
    pub(crate) fn offline_now(&self) -> usize {
        let now = self.now();
        self.presence
            .lock()
            .expect("the presence is never poisoned")
            .count(now)
    }

    /// Whether a direct report from `address` may be taken now; counted.
    pub(crate) fn allow_direct(&self, address: &str) -> bool {
        let now = self.now();
        self.direct
            .lock()
            .expect("the allowance is never poisoned")
            .take(address, now)
    }
}

/// `FEEDBACK_TRUSTED_PROXIES`, every entry an address.
///
/// # Errors
///
/// On an entry that is not one: unlike the gateway, which drops such an
/// entry, the service refuses to start, since it has no other setting whose
/// typo would go unnoticed as quietly.
pub fn trusted_proxies(raw: &str) -> anyhow::Result<Vec<IpAddr>> {
    raw.split(',')
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .map(|e| {
            e.parse()
                .map_err(|_| anyhow::anyhow!("FEEDBACK_TRUSTED_PROXIES: `{e}` is not an address"))
        })
        .collect()
}

/// The address a sign-in is counted under: the peer, unless the peer is a
/// trusted proxy, and then the rightmost `X-Forwarded-For` entry no trusted
/// proxy wrote (the gateway's rule, `rate_limit_ip`).
fn client_address(trusted: &[IpAddr], peer: Option<IpAddr>, headers: &HeaderMap) -> String {
    let Some(peer) = peer else {
        return "unknown".to_owned();
    };
    if !trusted.contains(&peer) {
        return peer.to_string();
    }
    let Some(forwarded) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) else {
        return peer.to_string();
    };
    for entry in forwarded.rsplit(',').map(str::trim) {
        if entry.is_empty() {
            continue;
        }
        let Ok(address) = entry.parse::<IpAddr>() else {
            return peer.to_string();
        };
        if !trusted.contains(&address) {
            return address.to_string();
        }
    }
    peer.to_string()
}

// ------------------------------------------------------------------ limiter

/// Failed sign-ins in the last [`WINDOW`], per address and per name.
///
/// Memory only: a restart forgets it, and no address is ever written down.
#[derive(Default)]
pub struct Limiter {
    failures: HashMap<String, VecDeque<OffsetDateTime>>,
}

impl Limiter {
    /// The most keys it holds before it sweeps out every stale one.
    const SWEEP_AT: usize = 4096;

    fn recent(&mut self, key: &str, now: OffsetDateTime) -> usize {
        let Some(times) = self.failures.get_mut(key) else {
            return 0;
        };
        while times.front().is_some_and(|t| now - *t >= WINDOW) {
            times.pop_front();
        }
        times.len()
    }

    /// Whether a sign-in from `address` as `name` may be tried now.
    fn allows(&mut self, address: &str, name: &str, now: OffsetDateTime) -> bool {
        self.recent(&address_key(address), now) < PER_ADDRESS
            && self.recent(&name_key(name), now) < PER_NAME
    }

    fn fail(&mut self, address: &str, name: &str, now: OffsetDateTime) {
        if self.failures.len() >= Self::SWEEP_AT {
            self.failures
                .retain(|_, times| times.back().is_some_and(|t| now - *t < WINDOW));
        }
        for key in [address_key(address), name_key(name)] {
            self.failures.entry(key).or_default().push_back(now);
        }
    }

    fn forget_name(&mut self, name: &str) {
        self.failures.remove(&name_key(name));
    }
}

fn address_key(address: &str) -> String {
    format!("a:{address}")
}

/// A name as the limiter counts it: cut to the longest a name can be, so a
/// megabyte of name is one key like any other.
fn name_key(name: &str) -> String {
    format!(
        "n:{}",
        name.chars()
            .take(admin::MAX_NAME_CHARS + 1)
            .collect::<String>()
    )
}

// ------------------------------------------------------------------ sessions

/// A signed-in admin.
pub(crate) struct Session {
    token_hash: [u8; 32],
    /// The admin's name.
    pub(crate) name: String,
}

fn cookie_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == COOKIE)
        .map(|(_, value)| value.to_owned())
}

fn signed_out() -> Refusal {
    refuse(StatusCode::UNAUTHORIZED, "not signed in")
}

/// The session the request's cookie names, if it is live; using it moves
/// its idle clock.
pub(crate) async fn session(
    state: &AppState,
    ui: &Ui,
    headers: &HeaderMap,
) -> Result<Session, Refusal> {
    let token = cookie_token(headers).ok_or_else(signed_out)?;
    let token_hash = hash(&token);
    let db = &state.db;
    let now = ui.now();
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT a.name, s.created_at, s.last_seen_at FROM feedback_session s \
             JOIN feedback_admin a ON a.id = s.admin_id WHERE s.token_hash = $1",
            [token_hash.to_vec().into()],
        ))
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(signed_out)?;
    let name: String = row.try_get("", "name").map_err(|e| db_down(&e))?;
    let created: OffsetDateTime = row.try_get("", "created_at").map_err(|e| db_down(&e))?;
    let seen: OffsetDateTime = row.try_get("", "last_seen_at").map_err(|e| db_down(&e))?;
    let (sql, values): (&str, Vec<sea_orm::Value>) =
        if now - seen >= IDLE || now - created >= ABSOLUTE {
            (
                "DELETE FROM feedback_session WHERE token_hash = $1",
                vec![token_hash.to_vec().into()],
            )
        } else {
            (
                "UPDATE feedback_session SET last_seen_at = $2 WHERE token_hash = $1",
                vec![token_hash.to_vec().into(), now.into()],
            )
        };
    let lapsed = sql.starts_with("DELETE");
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        sql,
        values,
    ))
    .await
    .map_err(|e| db_down(&e))?;
    if lapsed {
        return Err(signed_out());
    }
    Ok(Session { token_hash, name })
}

/// A fresh session token: 256 bits from the OS, as hex.
fn new_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("OS RNG available");
    bytes.iter().fold(String::with_capacity(64), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn session_cookie(token: &str) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{COOKIE}={token}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={}",
        ABSOLUTE.whole_seconds()
    ))
    .expect("a token is hex")
}

fn cleared_cookie() -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{COOKIE}=; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=0"
    ))
    .expect("a fixed header")
}

// ------------------------------------------------------------------ CSRF

/// Whether a changing request comes from this UI: it carries
/// [`CSRF_HEADER`], its `Origin` names the host it was sent to, and, where
/// the browser says, it was sent from this origin.
pub(crate) fn same_origin(headers: &HeaderMap) -> Result<(), Refusal> {
    let cross = || refuse(StatusCode::FORBIDDEN, "cross-site request refused");
    let text = |name| {
        headers
            .get(name)
            .and_then(|v: &HeaderValue| v.to_str().ok())
    };
    if text(CSRF_HEADER) != Some("1") {
        return Err(cross());
    }
    if text("sec-fetch-site").is_some_and(|site| site != "same-origin") {
        return Err(cross());
    }
    let host = text(header::HOST.as_str()).ok_or_else(cross)?;
    let origin = text(header::ORIGIN.as_str()).ok_or_else(cross)?;
    let authority = origin
        .strip_prefix("https://")
        .or_else(|| origin.strip_prefix("http://"))
        .ok_or_else(cross)?;
    if authority.eq_ignore_ascii_case(host) {
        Ok(())
    } else {
        Err(cross())
    }
}

// ------------------------------------------------------------------ routes

pub(crate) fn routes() -> Router<Shared> {
    Router::new()
        .route("/ui/api/login", post(login))
        .route("/ui/api/logout", post(logout))
        .route("/ui/api/me", get(me))
        .route("/ui/api/facets", get(facets))
        .route("/ui/api/stats", get(stats))
        .route("/ui/api/reports", get(reports))
        .route(
            "/ui/api/reports/{id}",
            get(report).patch(change).delete(delete),
        )
        .route("/ui/api/reports/{id}/record", get(record))
        .route("/ui/api/reports/{id}/audit", get(audit_trail))
        .merge(crate::console::routes())
        .route("/ui/api/{*rest}", axum::routing::any(no_such_route))
}

async fn no_such_route() -> Refusal {
    refuse(StatusCode::NOT_FOUND, "no such route")
}

#[derive(Deserialize)]
struct Credentials {
    name: String,
    password: String,
}

#[derive(Serialize)]
struct Me {
    name: String,
    /// Whether a gateway's admin console is configured, so the UI shows
    /// its admin pages (`/ui/api/admin/…`).
    gateway_admin: bool,
}

async fn login(
    State(shared): State<Shared>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, Refusal> {
    same_origin(&headers)?;
    let Shared { state, ui } = shared;
    let credentials: Credentials = serde_json::from_slice(&body)
        .map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a sign-in"))?;
    let address = client_address(
        &ui.trusted_proxies,
        peer.map(|Extension(ConnectInfo(a))| a.ip()),
        &headers,
    );
    let now = ui.now();
    let allowed = ui
        .limiter
        .lock()
        .expect("the limiter is never poisoned")
        .allows(&address, &credentials.name, now);
    if !allowed {
        let mut refusal = refuse(
            StatusCode::TOO_MANY_REQUESTS,
            "too many failed sign-ins; try again later",
        )
        .into_response();
        refusal.headers_mut().insert(
            header::RETRY_AFTER,
            HeaderValue::from(WINDOW.whole_seconds()),
        );
        return Ok(refusal);
    }
    // An unknown name and a known one cost the same Argon2 run and get the
    // same answer.
    let found = if admin::valid_name(&credentials.name) {
        admin::credentials(&state.db, &credentials.name)
            .await
            .map_err(|e| db_down(&e))?
    } else {
        None
    };
    let (id, stored) = found.map_or((None, None), |(id, h)| (Some(id), Some(h)));
    let password = credentials.password;
    let fits =
        tokio::task::spawn_blocking(move || admin::verify_password(stored.as_deref(), &password))
            .await
            .unwrap_or(false);
    let Some(id) = id.filter(|_| fits) else {
        ui.limiter
            .lock()
            .expect("the limiter is never poisoned")
            .fail(&address, &credentials.name, now);
        tracing::info!("a sign-in failed");
        return Err(refuse(StatusCode::UNAUTHORIZED, "wrong name or password"));
    };
    ui.limiter
        .lock()
        .expect("the limiter is never poisoned")
        .forget_name(&credentials.name);
    let token = new_token();
    let db = &state.db;
    // Every lapsed session goes on the way, so the table holds live ones
    // and the ones lapsed since the last sign-in, nothing older.
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "DELETE FROM feedback_session WHERE last_seen_at <= $1 OR created_at <= $2",
        [(now - IDLE).into(), (now - ABSOLUTE).into()],
    ))
    .await
    .map_err(|e| db_down(&e))?;
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO feedback_session (token_hash, admin_id, created_at, last_seen_at) \
         VALUES ($1, $2, $3, $3)",
        [hash(&token).to_vec().into(), id.into(), now.into()],
    ))
    .await
    .map_err(|e| db_down(&e))?;
    tracing::info!(admin = credentials.name, "an admin signed in");
    let mut response = Json(Me {
        name: credentials.name,
        gateway_admin: state.config.gateway_admin().is_some(),
    })
    .into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, session_cookie(&token));
    Ok(response)
}

async fn logout(State(shared): State<Shared>, headers: HeaderMap) -> Response {
    let cleared = |mut response: Response| {
        response
            .headers_mut()
            .insert(header::SET_COOKIE, cleared_cookie());
        response
    };
    if let Err(refusal) = same_origin(&headers) {
        return refusal.into_response();
    }
    let signed_in = match session(&shared.state, &shared.ui, &headers).await {
        Ok(s) => s,
        Err(refusal) => return cleared(refusal.into_response()),
    };
    let db = &shared.state.db;
    if let Err(e) = db
        .execute_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "DELETE FROM feedback_session WHERE token_hash = $1",
            [signed_in.token_hash.to_vec().into()],
        ))
        .await
    {
        return db_down(&e).into_response();
    }
    tracing::info!(admin = signed_in.name, "an admin signed out");
    cleared(StatusCode::NO_CONTENT.into_response())
}

async fn me(State(shared): State<Shared>, headers: HeaderMap) -> Result<Json<Me>, Refusal> {
    let signed_in = session(&shared.state, &shared.ui, &headers).await?;
    Ok(Json(Me {
        name: signed_in.name,
        gateway_admin: shared.state.config.gateway_admin().is_some(),
    }))
}

async fn reports(
    State(shared): State<Shared>,
    headers: HeaderMap,
    filter: Result<Query<Filter>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<Listing>, Refusal> {
    session(&shared.state, &shared.ui, &headers).await?;
    let Query(filter) =
        filter.map_err(|_| refuse(StatusCode::BAD_REQUEST, "a filter is malformed"))?;
    listing(&shared.state.db, filter).await.map(Json)
}

async fn report(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Summary>, Refusal> {
    session(&shared.state, &shared.ui, &headers).await?;
    fetch(&shared.state.db, report_id(&id)?).await
}

async fn record(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, Refusal> {
    session(&shared.state, &shared.ui, &headers).await?;
    record_of(&shared.state.db, report_id(&id)?).await
}

async fn change(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Json<Summary>, Refusal> {
    same_origin(&headers)?;
    let signed_in = session(&shared.state, &shared.ui, &headers).await?;
    let id = report_id(&id)?;
    let change: Change = serde_json::from_slice(&body)
        .map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a change"))?;
    apply_change(&shared.state.db, id, &signed_in.name, change).await
}

async fn delete(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, Refusal> {
    same_origin(&headers)?;
    let signed_in = session(&shared.state, &shared.ui, &headers).await?;
    delete_report(&shared.state.db, report_id(&id)?, &signed_in.name).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
struct AuditEntry {
    at: String,
    actor: String,
    action: String,
    detail: Option<String>,
}

async fn audit_trail(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Vec<AuditEntry>>, Refusal> {
    session(&shared.state, &shared.ui, &headers).await?;
    let id = report_id(&id)?;
    let db = &shared.state.db;
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT to_char(at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS at, \
                 actor, action, detail FROM feedback_audit WHERE report_id = $1 \
             ORDER BY at, id",
            [id.into()],
        ))
        .await
        .map_err(|e| db_down(&e))?;
    rows.iter()
        .map(|row| {
            Ok(AuditEntry {
                at: row.try_get("", "at")?,
                actor: row.try_get("", "actor")?,
                action: row.try_get("", "action")?,
                detail: row.try_get("", "detail")?,
            })
        })
        .collect::<Result<_, sea_orm::DbErr>>()
        .map(Json)
        .map_err(|e| db_down(&e))
}

#[derive(Serialize)]
struct Count {
    value: String,
    count: i64,
}

#[derive(Serialize)]
struct Facets {
    gateways: Vec<Count>,
    statuses: Vec<Count>,
    kinds: Vec<Count>,
    /// The reporters with the most reports, at most [`TOP_REPORTERS`].
    reporters: Vec<Count>,
}

/// How many reporters the facets name.
const TOP_REPORTERS: u32 = 50;

async fn counts(db: &DatabaseConnection, column: &str, limit: u32) -> Result<Vec<Count>, Refusal> {
    let rows = db
        .query_all_raw(Statement::from_string(
            db.get_database_backend(),
            format!(
                "SELECT {column} AS value, count(*) AS count FROM feedback_report \
                 GROUP BY {column} ORDER BY count(*) DESC, {column} LIMIT {limit}"
            ),
        ))
        .await
        .map_err(|e| db_down(&e))?;
    rows.iter()
        .map(|row| {
            Ok(Count {
                value: row.try_get("", "value")?,
                count: row.try_get("", "count")?,
            })
        })
        .collect::<Result<_, sea_orm::DbErr>>()
        .map_err(|e| db_down(&e))
}

async fn facets(State(shared): State<Shared>, headers: HeaderMap) -> Result<Json<Facets>, Refusal> {
    session(&shared.state, &shared.ui, &headers).await?;
    let db = &shared.state.db;
    Ok(Json(Facets {
        gateways: counts(db, "gateway", 1000).await?,
        statuses: counts(db, "status", 100).await?,
        kinds: counts(db, "kind", 100).await?,
        reporters: counts(db, "reporter", TOP_REPORTERS).await?,
    }))
}

/// How many days back `GET /ui/api/stats` counts.
const STATS_DAYS: u32 = 30;

/// One day's reports of one kind.
#[derive(Serialize)]
struct DayKind {
    /// `YYYY-MM-DD`, UTC.
    day: String,
    kind: String,
    count: i64,
}

/// `GET /ui/api/stats`: reports per UTC day and kind over the last
/// [`STATS_DAYS`] days, for the overview's error-rate chart. Counts only.
#[derive(Serialize)]
struct ReportStats {
    days: u32,
    rows: Vec<DayKind>,
}

async fn stats(
    State(shared): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<ReportStats>, Refusal> {
    session(&shared.state, &shared.ui, &headers).await?;
    let db = &shared.state.db;
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD') AS day, kind, \
                 count(*) AS count FROM feedback_report \
             WHERE created_at >= now() - make_interval(days => $1) \
             GROUP BY day, kind ORDER BY day, kind",
            [i32::try_from(STATS_DAYS).unwrap_or(30).into()],
        ))
        .await
        .map_err(|e| db_down(&e))?;
    rows.iter()
        .map(|row| {
            Ok(DayKind {
                day: row.try_get("", "day")?,
                kind: row.try_get("", "kind")?,
                count: row.try_get("", "count")?,
            })
        })
        .collect::<Result<_, sea_orm::DbErr>>()
        .map(|rows| {
            Json(ReportStats {
                days: STATS_DAYS,
                rows,
            })
        })
        .map_err(|e| db_down(&e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(minutes: i64) -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH + Duration::minutes(minutes)
    }

    #[test]
    fn a_name_is_refused_after_its_failures_from_anywhere_and_forgiven_by_the_window() {
        let mut limiter = Limiter::default();
        for n in 0..PER_NAME {
            assert!(limiter.allows(&format!("10.0.0.{n}"), "viktor", at(0)));
            limiter.fail(&format!("10.0.0.{n}"), "viktor", at(0));
        }
        assert!(!limiter.allows("10.0.1.1", "viktor", at(1)));
        assert!(limiter.allows("10.0.1.1", "someone", at(1)));
        assert!(limiter.allows("10.0.1.1", "viktor", at(15)));
    }

    #[test]
    fn an_address_is_refused_after_its_failures_whatever_name_it_tries() {
        let mut limiter = Limiter::default();
        for n in 0..PER_ADDRESS {
            assert!(limiter.allows("10.0.0.1", &format!("name{n}"), at(0)));
            limiter.fail("10.0.0.1", &format!("name{n}"), at(0));
        }
        assert!(!limiter.allows("10.0.0.1", "fresh", at(14)));
        assert!(limiter.allows("10.0.0.2", "fresh", at(14)));
    }

    #[test]
    fn a_huge_name_is_one_key() {
        assert_eq!(name_key(&"x".repeat(10_000)), name_key(&"x".repeat(10_001)));
    }

    #[test]
    fn a_sign_in_is_counted_under_the_proxys_client_only_behind_a_trusted_proxy() {
        let proxy: IpAddr = "127.0.0.1".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "6.6.6.6, 1.2.3.4".parse().unwrap());
        assert_eq!(
            client_address(&[proxy], Some(proxy), &headers),
            "1.2.3.4",
            "the rightmost entry the proxy wrote, not the one the client chose"
        );
        assert_eq!(client_address(&[], Some(proxy), &headers), "127.0.0.1");
        let stranger: IpAddr = "9.9.9.9".parse().unwrap();
        assert_eq!(
            client_address(&[proxy], Some(stranger), &headers),
            "9.9.9.9"
        );
        assert_eq!(client_address(&[proxy], None, &headers), "unknown");
    }

    #[test]
    fn a_changing_request_names_this_host_and_carries_the_header() {
        let request = |pairs: &[(&'static str, &'static str)]| {
            let mut h = HeaderMap::new();
            for (k, v) in pairs {
                h.insert(*k, v.parse().unwrap());
            }
            same_origin(&h).is_ok()
        };
        let host = ("host", "feedback.example");
        let origin = ("origin", "https://feedback.example");
        let csrf = (CSRF_HEADER, "1");
        assert!(request(&[host, origin, csrf]));
        assert!(request(&[
            host,
            origin,
            csrf,
            ("sec-fetch-site", "same-origin")
        ]));
        assert!(!request(&[host, origin]), "no header");
        assert!(!request(&[host, csrf]), "no origin");
        assert!(!request(&[host, ("origin", "https://evil.example"), csrf]));
        assert!(!request(&[host, ("origin", "null"), csrf]));
        assert!(!request(&[host, origin, (CSRF_HEADER, "0")]));
        assert!(!request(&[
            host,
            origin,
            csrf,
            ("sec-fetch-site", "cross-site")
        ]));
        assert!(!request(&[
            host,
            origin,
            csrf,
            ("sec-fetch-site", "same-site")
        ]));
    }

    #[test]
    fn the_cookie_is_read_among_others_and_only_under_its_own_name() {
        let mut h = HeaderMap::new();
        h.insert(
            header::COOKIE,
            format!("a=b; {COOKIE}=abc; c=d").parse().unwrap(),
        );
        assert_eq!(cookie_token(&h).as_deref(), Some("abc"));
        let mut h = HeaderMap::new();
        h.insert(header::COOKIE, "baylee-feedback=abc".parse().unwrap());
        assert_eq!(cookie_token(&h), None);
    }

    #[test]
    fn the_session_cookie_is_host_only_http_only_secure_and_strict() {
        let cookie = session_cookie("ab").to_str().unwrap().to_owned();
        for part in ["HttpOnly", "Secure", "SameSite=Strict", "Path=/"] {
            assert!(cookie.contains(part), "{cookie}");
        }
        assert!(cookie.starts_with("__Host-"));
        assert!(!cookie.contains("Domain"));
    }
}
