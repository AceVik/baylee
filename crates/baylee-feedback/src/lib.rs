//! baylee-feedback — the service that keeps what players report (#308).
//!
//! Gateways pass reports on (`POST /reports` on a gateway, #307); this keeps
//! them in a database of its own and answers a small JSON API to whoever
//! holds its read token. `docs/feedback.md` is normative.
//!
//! - `POST /intake/reports`: a gateway, with its own intake token.
//! - `POST /client/reports`: a client signed in to no gateway, with no
//!   token at all, held to its limits instead ([`direct`]).
//! - `POST /client/alive`: an offline game still going, anonymously, for
//!   the admins' "offline now" ([`alive`]).
//! - `GET /reports`, `GET /reports/{id}`, `GET /reports/{id}/record`: the
//!   read token (or the admin token).
//! - `PATCH /reports/{id}` (status), `DELETE /reports/{id}`: the admin token.
//! - `/ui/api/…`: the web UI's JSON routes, for a signed-in admin (#311,
//!   [`ui`]); `FEEDBACK_WEB_DIR` serves the UI itself ([`web`]).
//! - `/ui/api/admin/…`: the same admins, passed on to a gateway's admin
//!   console (its numbers and closed-beta keys, [`console`]).
//!
//! Nothing it stores names a player: a report carries the gateway's
//! pseudonym for its reporter, and no request's address is kept or logged.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod admin;
pub mod alive;
pub mod console;
pub mod direct;
pub mod migration;
pub mod ui;
pub mod web;

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use axum::Router;
use axum::body::Bytes;
use axum::extract::{FromRef, Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use base64::Engine as _;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, Statement,
    TransactionTrait as _, Value,
};
use sea_orm_migration::MigratorTrait as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq as _;
use uuid::Uuid;

/// The longest report text, in characters, as a gateway allows it.
pub const MAX_TEXT_CHARS: usize = 20_000;

/// The largest `client` object, serialized.
pub const MAX_CLIENT_BYTES: usize = 2 * 1024 * 1024;

/// The largest game record, compressed, as a gateway stores one.
pub const MAX_RECORD_BYTES: usize = 64 * 1024 * 1024;

/// The largest intake body: a whole record in base64 and a whole client
/// object, with room for the rest.
pub const MAX_INTAKE_BYTES: usize = MAX_RECORD_BYTES / 3 * 4 + MAX_CLIENT_BYTES + 256 * 1024;

/// The shortest token the service accepts in its configuration.
pub const MIN_TOKEN_CHARS: usize = 16;

/// A token as it is kept: its SHA-256, compared in constant time.
type TokenHash = [u8; 32];

fn hash(token: &str) -> TokenHash {
    Sha256::digest(token.as_bytes()).into()
}

/// Who may do what, from the environment.
#[derive(Clone)]
pub struct Config {
    /// Each gateway's name and intake token.
    gateways: Vec<(String, TokenHash)>,
    /// `FEEDBACK_READ_TOKEN`.
    read: Option<TokenHash>,
    /// `FEEDBACK_ADMIN_TOKEN`.
    admin: Option<TokenHash>,
    /// `FEEDBACK_DIRECT_KEY`, hashed: the key direct reports' reporters are
    /// made with; `None` = the direct route is off.
    direct: Option<TokenHash>,
    /// `FEEDBACK_GATEWAY_ADMIN_URL` and `FEEDBACK_GATEWAY_ADMIN_TOKEN`: the
    /// gateway's admin console the UI's admin pages talk to ([`console`]);
    /// `None` = no admin pages.
    gateway_admin: Option<console::Gateway>,
}

impl Config {
    /// Reads `FEEDBACK_GATEWAY_TOKENS`, `FEEDBACK_READ_TOKEN`,
    /// `FEEDBACK_ADMIN_TOKEN`, `FEEDBACK_DIRECT_KEY`,
    /// `FEEDBACK_GATEWAY_ADMIN_URL` and `FEEDBACK_GATEWAY_ADMIN_TOKEN`.
    ///
    /// # Errors
    ///
    /// When a value is malformed; see [`Config::new`],
    /// [`Config::with_direct_key`] and [`Config::with_gateway_admin`]. One
    /// of the last two without the other is malformed too.
    pub fn from_env() -> Result<Self> {
        let var = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
        let config = Self::new(
            &var("FEEDBACK_GATEWAY_TOKENS").unwrap_or_default(),
            var("FEEDBACK_READ_TOKEN").as_deref(),
            var("FEEDBACK_ADMIN_TOKEN").as_deref(),
        )?;
        let config = match var("FEEDBACK_DIRECT_KEY") {
            Some(key) => config.with_direct_key(&key)?,
            None => config,
        };
        match (
            var("FEEDBACK_GATEWAY_ADMIN_URL"),
            var("FEEDBACK_GATEWAY_ADMIN_TOKEN"),
        ) {
            (Some(url), Some(token)) => config.with_gateway_admin(&url, &token),
            (None, None) => Ok(config),
            (Some(_), None) => {
                bail!("FEEDBACK_GATEWAY_ADMIN_URL is set, and FEEDBACK_GATEWAY_ADMIN_TOKEN is not")
            }
            (None, Some(_)) => {
                bail!("FEEDBACK_GATEWAY_ADMIN_TOKEN is set, and FEEDBACK_GATEWAY_ADMIN_URL is not")
            }
        }
    }

    /// Lets the UI's admins administer a gateway: its admin console at
    /// `url` (`http://127.0.0.1:28767`), asked with `token`, the gateway's
    /// `BAYLEE_ADMIN_TOKEN`.
    ///
    /// # Errors
    ///
    /// When `url` is not a plain `http://` or `https://` address (no
    /// credentials, query or fragment), or `token` is shorter than
    /// [`console::MIN_TOKEN_CHARS`], holds whitespace, or is one of this
    /// service's own tokens or its direct key.
    pub fn with_gateway_admin(mut self, url: &str, token: &str) -> Result<Self> {
        let h = hash(token);
        let taken = self.gateways.iter().any(|(_, t)| *t == h)
            || self.read == Some(h)
            || self.admin == Some(h)
            || self.direct == Some(h);
        if taken {
            bail!("FEEDBACK_GATEWAY_ADMIN_TOKEN: that token is already given to something else");
        }
        self.gateway_admin = Some(console::Gateway::new(url, token)?);
        Ok(self)
    }

    /// The gateway the admin pages talk to, if any.
    pub(crate) fn gateway_admin(&self) -> Option<&console::Gateway> {
        self.gateway_admin.as_ref()
    }

    /// Takes direct reports (`POST /client/reports`), their reporters made
    /// under `key`.
    ///
    /// # Errors
    ///
    /// When `key` is shorter than [`MIN_TOKEN_CHARS`] or is one of the
    /// tokens: a key that also opened a door would be a token in two places.
    pub fn with_direct_key(mut self, key: &str) -> Result<Self> {
        if key.chars().count() < MIN_TOKEN_CHARS {
            bail!("FEEDBACK_DIRECT_KEY: a key needs at least {MIN_TOKEN_CHARS} characters");
        }
        let h = hash(key);
        let taken = self.gateways.iter().any(|(_, t)| *t == h)
            || self.read == Some(h)
            || self.admin == Some(h)
            || self
                .gateway_admin
                .as_ref()
                .is_some_and(|g| g.token_hash() == h);
        if taken {
            bail!("FEEDBACK_DIRECT_KEY: that key is already a token");
        }
        self.direct = Some(h);
        Ok(self)
    }

    /// The key direct reports' reporters are made with, when they are taken.
    pub(crate) fn direct_key(&self) -> Option<&TokenHash> {
        self.direct.as_ref()
    }

    /// `gateways` is `name=token` pairs separated by commas.
    ///
    /// # Errors
    ///
    /// When a pair has no `=`, a name is empty, longer than 64 characters or
    /// holds anything but letters, digits, `.`, `-` and `_`, a token is
    /// shorter than [`MIN_TOKEN_CHARS`], or one token is given twice,
    /// anywhere: a token that opened two doors could not say which it was
    /// meant for.
    pub fn new(gateways: &str, read: Option<&str>, admin: Option<&str>) -> Result<Self> {
        let mut seen: Vec<TokenHash> = Vec::new();
        let mut take = |what: &str, token: &str| -> Result<TokenHash> {
            if token.chars().count() < MIN_TOKEN_CHARS {
                bail!("{what}: a token needs at least {MIN_TOKEN_CHARS} characters");
            }
            let h = hash(token);
            if seen.contains(&h) {
                bail!("{what}: that token is already given to something else");
            }
            seen.push(h);
            Ok(h)
        };
        let mut list = Vec::new();
        for pair in gateways.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let Some((name, token)) = pair.split_once('=') else {
                bail!("FEEDBACK_GATEWAY_TOKENS: `{pair}` is not name=token");
            };
            let name = name.trim();
            let fine = !name.is_empty()
                && name.chars().count() <= 64
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
            if !fine {
                bail!("FEEDBACK_GATEWAY_TOKENS: `{name}` is not a gateway name");
            }
            list.push((name.to_owned(), take(name, token.trim())?));
        }
        Ok(Self {
            gateways: list,
            read: read.map(|t| take("FEEDBACK_READ_TOKEN", t)).transpose()?,
            admin: admin.map(|t| take("FEEDBACK_ADMIN_TOKEN", t)).transpose()?,
            direct: None,
            gateway_admin: None,
        })
    }

    fn gateway(&self, token: &str) -> Option<&str> {
        let h = hash(token);
        // Every entry is compared, so how long it takes says nothing about
        // which one matched.
        self.gateways.iter().fold(None, |found, (name, known)| {
            if bool::from(known.ct_eq(&h)) {
                Some(name.as_str())
            } else {
                found
            }
        })
    }

    fn is(known: Option<&TokenHash>, token: &str) -> bool {
        known.is_some_and(|k| bool::from(k.ct_eq(&hash(token))))
    }
}

/// What every handler shares.
pub struct AppState {
    /// The service's own database.
    pub db: DatabaseConnection,
    /// The tokens.
    pub config: Config,
}

/// Opens the database and brings its schema up to date.
///
/// # Errors
///
/// When the server is unreachable or a migration fails.
pub async fn connect(url: &str, pool: u32) -> Result<DatabaseConnection> {
    let mut options = ConnectOptions::new(url.to_owned());
    options
        .max_connections(pool)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(600))
        .sqlx_logging(false);
    let db = Database::connect(options)
        .await
        .context("connecting to the feedback database")?;
    migration::Migrator::up(&db, None)
        .await
        .context("applying migrations")?;
    Ok(db)
}

/// Every route, with the web UI's defaults: no static files, the system
/// clock, no trusted proxy.
pub fn app(state: Arc<AppState>) -> Router {
    app_with(state, ui::Ui::default())
}

/// What the router hands each handler: the service's state and the UI's.
#[derive(Clone)]
pub(crate) struct Shared {
    state: Arc<AppState>,
    ui: Arc<ui::Ui>,
}

impl FromRef<Shared> for Arc<AppState> {
    fn from_ref(shared: &Shared) -> Self {
        shared.state.clone()
    }
}

impl FromRef<Shared> for Arc<ui::Ui> {
    fn from_ref(shared: &Shared) -> Self {
        shared.ui.clone()
    }
}

/// Every route, the web UI configured by `ui`.
pub fn app_with(state: Arc<AppState>, ui: ui::Ui) -> Router {
    let serves_files = ui.web_dir().is_some();
    let shared = Shared {
        state,
        ui: Arc::new(ui),
    };
    let router = Router::new()
        .route("/health", get(health))
        .route(
            "/intake/reports",
            post(intake).layer(axum::extract::DefaultBodyLimit::max(MAX_INTAKE_BYTES)),
        )
        .route(
            "/client/reports",
            post(direct::post)
                .options(direct::preflight)
                .layer(axum::extract::DefaultBodyLimit::max(direct::MAX_BODY_BYTES))
                .layer(axum::middleware::map_response(direct::any_origin)),
        )
        .route(
            "/client/alive",
            post(alive::post)
                .options(direct::preflight)
                .layer(axum::extract::DefaultBodyLimit::max(alive::MAX_BODY_BYTES))
                .layer(axum::middleware::map_response(direct::any_origin)),
        )
        .route("/reports", get(list))
        .route("/reports/{id}", get(one).patch(set_status).delete(remove))
        .route("/reports/{id}/record", get(record))
        .merge(ui::routes());
    let router = if serves_files {
        router.fallback(web::serve)
    } else {
        router
    };
    router
        .layer(axum::middleware::map_response(web::security_headers))
        .with_state(shared)
}

// ------------------------------------------------------------------ errors

#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
}

type Refusal = (StatusCode, Json<ErrorBody>);

fn refuse(status: StatusCode, error: &'static str) -> Refusal {
    (status, Json(ErrorBody { error }))
}

fn db_down(e: &sea_orm::DbErr) -> Refusal {
    tracing::error!("{e}");
    refuse(
        StatusCode::SERVICE_UNAVAILABLE,
        "the database is unavailable",
    )
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}

/// Whether the caller may read: the read token or the admin token.
fn reader(state: &AppState, headers: &HeaderMap) -> Result<(), Refusal> {
    let token = bearer(headers).unwrap_or_default();
    if Config::is(state.config.read.as_ref(), token)
        || Config::is(state.config.admin.as_ref(), token)
    {
        Ok(())
    } else {
        Err(refuse(StatusCode::UNAUTHORIZED, "a read token is needed"))
    }
}

fn admin(state: &AppState, headers: &HeaderMap) -> Result<(), Refusal> {
    if Config::is(
        state.config.admin.as_ref(),
        bearer(headers).unwrap_or_default(),
    ) {
        Ok(())
    } else {
        Err(refuse(
            StatusCode::UNAUTHORIZED,
            "the admin token is needed",
        ))
    }
}

// ------------------------------------------------------------------ shapes

/// What a report is about.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Something is broken.
    Bug,
    /// Something could be better.
    Improvement,
    /// Anything the player wants to say.
    Feedback,
    /// The client crashed.
    Crash,
    /// None of these.
    Other,
}

impl Kind {
    const fn name(self) -> &'static str {
        match self {
            Self::Bug => "bug",
            Self::Improvement => "improvement",
            Self::Feedback => "feedback",
            Self::Crash => "crash",
            Self::Other => "other",
        }
    }
}

/// Where a report stands.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Nobody has looked at it.
    New,
    /// Looked at, and kept.
    Triaged,
    /// Being worked on.
    InProgress,
    /// Fixed or answered.
    Resolved,
    /// Will not be acted on.
    WontFix,
    /// Another report says the same.
    Duplicate,
}

impl Status {
    const fn name(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Triaged => "triaged",
            Self::InProgress => "in_progress",
            Self::Resolved => "resolved",
            Self::WontFix => "wont_fix",
            Self::Duplicate => "duplicate",
        }
    }
}

#[derive(Deserialize)]
struct GatewayIn {
    name: Option<String>,
    url: Option<String>,
    version: String,
}

#[derive(Deserialize)]
struct RecordIn {
    complete: bool,
    gzip_base64: String,
}

/// Who wrote a report's record.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// The gateway, from what the game's engine sent it.
    #[default]
    Gateway,
    /// A client, of a game it hosted itself: unverified, never a gateway's.
    Client,
}

impl Origin {
    const fn name(self) -> &'static str {
        match self {
            Self::Gateway => "gateway",
            Self::Client => "client",
        }
    }
}

/// How a report reached the service.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    /// A gateway passed it on (`POST /intake/reports`).
    Gateway,
    /// A client sent it itself, unauthenticated (`POST /client/reports`).
    Direct,
}

impl Channel {
    const fn name(self) -> &'static str {
        match self {
            Self::Gateway => "gateway",
            Self::Direct => "direct",
        }
    }
}

/// A report about to be kept, from either door.
pub(crate) struct NewReport {
    gateway: String,
    gateway_name: Option<String>,
    gateway_url: Option<String>,
    gateway_version: String,
    reporter: String,
    kind: Kind,
    text: String,
    game_id: Option<String>,
    /// The `client` object, serialized.
    client: String,
    /// The record and whether its end is in it.
    record: Option<(Vec<u8>, bool)>,
    record_origin: Origin,
    channel: Channel,
}

/// Keeps `report`; its id.
pub(crate) async fn store(db: &DatabaseConnection, report: NewReport) -> Result<Uuid, Refusal> {
    let id = Uuid::now_v7();
    let (record, complete, origin) = match report.record {
        Some((bytes, complete)) => (
            Some(bytes),
            Some(complete),
            Some(report.record_origin.name()),
        ),
        None => (None, None, None),
    };
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO feedback_report (id, gateway, gateway_name, gateway_url, \
             gateway_version, reporter, kind, text, game_id, client, record, record_complete, \
             record_origin, channel) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::jsonb, $11, $12, $13, $14)",
        [
            id.into(),
            report.gateway.into(),
            report.gateway_name.into(),
            report.gateway_url.into(),
            report.gateway_version.into(),
            report.reporter.into(),
            report.kind.name().into(),
            report.text.into(),
            report.game_id.into(),
            report.client.into(),
            Value::Bytes(record),
            complete.into(),
            origin.into(),
            report.channel.name().into(),
        ],
    ))
    .await
    .map_err(|e| db_down(&e))?;
    Ok(id)
}

/// What a gateway sends (`docs/feedback.md` §"Intake").
#[derive(Deserialize)]
struct Intake {
    gateway: GatewayIn,
    reporter: String,
    kind: Kind,
    text: String,
    #[serde(default)]
    game_id: Option<String>,
    client: serde_json::Value,
    #[serde(default)]
    record: Option<RecordIn>,
    /// Who wrote `record`: a client's is checked ([`direct::check_record`])
    /// and held to a client's bounds, and kept marked as the client's.
    #[serde(default)]
    record_origin: Origin,
}

#[derive(Serialize)]
struct Created {
    report_id: String,
}

// ------------------------------------------------------------------ routes

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "ok": true,
        "version": baylee_build::short(),
        "source": baylee_build::REPOSITORY,
    }))
}

fn fits(value: Option<&str>, chars: usize) -> bool {
    value.is_none_or(|v| v.chars().count() <= chars && !v.chars().any(char::is_control))
}

async fn intake(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<Created>), Refusal> {
    let Some(gateway) = state.config.gateway(bearer(&headers).unwrap_or_default()) else {
        return Err(refuse(StatusCode::UNAUTHORIZED, "not a known gateway"));
    };
    let bad = |why| refuse(StatusCode::BAD_REQUEST, why);
    let report: Intake = serde_json::from_slice(&body).map_err(|_| bad("not a report"))?;
    if !report.client.is_object() {
        return Err(bad("client must be an object"));
    }
    if report.text.chars().count() > MAX_TEXT_CHARS {
        return Err(refuse(
            StatusCode::PAYLOAD_TOO_LARGE,
            "the text is too long",
        ));
    }
    let client = serde_json::to_string(&report.client).map_err(|_| bad("client"))?;
    if client.len() > MAX_CLIENT_BYTES {
        return Err(refuse(
            StatusCode::PAYLOAD_TOO_LARGE,
            "the client details are too large",
        ));
    }
    let reporter_fine = !report.reporter.is_empty()
        && report.reporter.len() <= 128
        && report.reporter.chars().all(|c| c.is_ascii_alphanumeric());
    if !reporter_fine
        || !fits(report.game_id.as_deref(), 128)
        || !fits(report.gateway.name.as_deref(), 256)
        || !fits(report.gateway.url.as_deref(), 256)
        || !fits(Some(&report.gateway.version), 128)
    {
        return Err(bad("a field is malformed"));
    }
    let record = match (report.record, report.record_origin) {
        (Some(r), Origin::Client) => Some(
            direct::ClientRecord {
                complete: r.complete,
                gzip_base64: r.gzip_base64,
            }
            .bytes()?,
        ),
        (Some(r), Origin::Gateway) => {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(r.gzip_base64)
                .map_err(|_| bad("the record is not base64"))?;
            if bytes.len() > MAX_RECORD_BYTES {
                return Err(refuse(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "the record is too large",
                ));
            }
            Some((bytes, r.complete))
        }
        (None, _) => None,
    };
    let id = store(
        &state.db,
        NewReport {
            gateway: gateway.to_owned(),
            gateway_name: report.gateway.name,
            gateway_url: report.gateway.url,
            gateway_version: report.gateway.version,
            reporter: report.reporter,
            kind: report.kind,
            text: report.text,
            game_id: report.game_id,
            client,
            record,
            record_origin: report.record_origin,
            channel: Channel::Gateway,
        },
    )
    .await?;
    tracing::info!(report = %id, gateway, kind = report.kind.name(), "report taken");
    Ok((
        StatusCode::CREATED,
        Json(Created {
            report_id: id.to_string(),
        }),
    ))
}

/// The columns a summary shows, as the list and the detail select them.
const SUMMARY: &str = "id::text AS id, \
    to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at, \
    to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at, \
    gateway, gateway_name, gateway_url, gateway_version, reporter, kind, status, text, game_id, \
    record IS NOT NULL AS has_record, record_complete, \
    coalesce(octet_length(record), 0)::bigint AS record_bytes, issue_number, \
    channel, record_origin";

/// The repository a report's issue lives in; the UI opens new issues there.
pub const ISSUE_REPOSITORY: &str = "https://github.com/AceVik/baylee";

/// A report as the read API shows it.
#[derive(Serialize)]
struct Summary {
    id: String,
    created_at: String,
    updated_at: String,
    gateway: String,
    gateway_name: Option<String>,
    gateway_url: Option<String>,
    gateway_version: String,
    reporter: String,
    kind: String,
    status: String,
    text: String,
    game_id: Option<String>,
    has_record: bool,
    record_complete: Option<bool>,
    record_bytes: i64,
    /// How it came: `gateway` (passed on by one) or `direct` (a client
    /// sent it itself, unauthenticated).
    channel: String,
    /// Who wrote the record: `gateway`, or `client` (unverified); `None`
    /// without one.
    record_origin: Option<String>,
    /// The GitHub issue it is linked to, in [`ISSUE_REPOSITORY`].
    issue_number: Option<i32>,
    /// That issue's page, made here from the number rather than stored, so
    /// no link the UI renders was ever typed by anyone.
    issue_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    client: Option<serde_json::Value>,
}

impl Summary {
    fn of(row: &sea_orm::QueryResult, client: bool) -> Result<Self, sea_orm::DbErr> {
        let issue_number: Option<i32> = row.try_get("", "issue_number")?;
        Ok(Self {
            id: row.try_get("", "id")?,
            created_at: row.try_get("", "created_at")?,
            updated_at: row.try_get("", "updated_at")?,
            gateway: row.try_get("", "gateway")?,
            gateway_name: row.try_get("", "gateway_name")?,
            gateway_url: row.try_get("", "gateway_url")?,
            gateway_version: row.try_get("", "gateway_version")?,
            reporter: row.try_get("", "reporter")?,
            kind: row.try_get("", "kind")?,
            status: row.try_get("", "status")?,
            text: row.try_get("", "text")?,
            game_id: row.try_get("", "game_id")?,
            has_record: row.try_get("", "has_record")?,
            record_complete: row.try_get("", "record_complete")?,
            record_bytes: row.try_get("", "record_bytes")?,
            channel: row.try_get("", "channel")?,
            record_origin: row.try_get("", "record_origin")?,
            issue_number,
            issue_url: issue_number.map(|n| format!("{ISSUE_REPOSITORY}/issues/{n}")),
            client: if client {
                Some(row.try_get("", "client")?)
            } else {
                None
            },
        })
    }
}

/// The list's filters, all optional. The token API and the UI read the
/// same ones.
#[derive(Deserialize, Default)]
struct Filter {
    status: Option<Status>,
    kind: Option<Kind>,
    gateway: Option<String>,
    reporter: Option<String>,
    game_id: Option<String>,
    /// Words the report's text holds, case aside.
    q: Option<String>,
    /// The first day, `YYYY-MM-DD` in UTC, inclusive.
    from: Option<String>,
    /// The last day, `YYYY-MM-DD` in UTC, inclusive.
    to: Option<String>,
    /// Whether it carries a game record.
    has_record: Option<bool>,
    /// How it came.
    channel: Option<Channel>,
    limit: Option<u64>,
    offset: Option<u64>,
}

/// `YYYY-MM-DD`, a real day, or nothing.
fn day(text: &str) -> Option<time::Date> {
    let mut parts = text.split('-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return None;
    }
    let month = time::Month::try_from(m.parse::<u8>().ok()?).ok()?;
    time::Date::from_calendar_date(y.parse().ok()?, month, d.parse().ok()?).ok()
}

/// `text` with `LIKE`'s wildcards taken literally, inside `%…%`.
fn contains_pattern(text: &str) -> String {
    let mut pattern = String::with_capacity(text.len() + 2);
    pattern.push('%');
    for c in text.chars() {
        if matches!(c, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(c);
    }
    pattern.push('%');
    pattern
}

#[derive(Serialize)]
struct Listing {
    reports: Vec<Summary>,
    total: i64,
}

async fn listing(db: &DatabaseConnection, filter: Filter) -> Result<Listing, Refusal> {
    let malformed = || refuse(StatusCode::BAD_REQUEST, "a filter is malformed");
    let mut clauses: Vec<String> = Vec::new();
    let mut values: Vec<Value> = Vec::new();
    let mut add = |clause: &str, value: Value| {
        values.push(value);
        clauses.push(clause.replace('?', &format!("${}", values.len())));
    };
    if let Some(status) = filter.status {
        add("status = ?", status.name().into());
    }
    if let Some(kind) = filter.kind {
        add("kind = ?", kind.name().into());
    }
    if let Some(gateway) = filter.gateway {
        add("gateway = ?", gateway.into());
    }
    if let Some(reporter) = filter.reporter {
        add("reporter = ?", reporter.into());
    }
    if let Some(game) = filter.game_id {
        add("game_id = ?", game.into());
    }
    if let Some(channel) = filter.channel {
        add("channel = ?", channel.name().into());
    }
    if let Some(q) = filter.q.filter(|q| !q.trim().is_empty()) {
        if q.chars().count() > 200 {
            return Err(malformed());
        }
        add(
            "text ILIKE ? ESCAPE '\\'",
            contains_pattern(q.trim()).into(),
        );
    }
    if let Some(from) = filter.from {
        let from = day(&from).ok_or_else(malformed)?;
        add(
            "created_at >= (?::date)::timestamp AT TIME ZONE 'UTC'",
            from.to_string().into(),
        );
    }
    if let Some(to) = filter.to {
        let to = day(&to).ok_or_else(malformed)?;
        add(
            "created_at < (?::date + 1)::timestamp AT TIME ZONE 'UTC'",
            to.to_string().into(),
        );
    }
    match filter.has_record {
        Some(true) => clauses.push("record IS NOT NULL".to_owned()),
        Some(false) => clauses.push("record IS NULL".to_owned()),
        None => {}
    }
    let clause = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    let limit = filter.limit.unwrap_or(50).clamp(1, 200);
    // Past Postgres's `bigint` an offset is an error; far below it, nothing.
    let offset = filter.offset.unwrap_or(0).min(1 << 40);
    let backend = db.get_database_backend();
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            backend,
            format!(
                "SELECT {SUMMARY} FROM feedback_report {clause} \
                 ORDER BY created_at DESC, id DESC LIMIT {limit} OFFSET {offset}"
            ),
            values.clone(),
        ))
        .await
        .map_err(|e| db_down(&e))?;
    let total = db
        .query_one_raw(Statement::from_sql_and_values(
            backend,
            format!("SELECT count(*) AS n FROM feedback_report {clause}"),
            values,
        ))
        .await
        .map_err(|e| db_down(&e))?
        .map_or(Ok(0), |row| row.try_get::<i64>("", "n"))
        .map_err(|e| db_down(&e))?;
    let reports = rows
        .iter()
        .map(|row| Summary::of(row, false))
        .collect::<Result<_, _>>()
        .map_err(|e| db_down(&e))?;
    Ok(Listing { reports, total })
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    filter: Result<Query<Filter>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<Listing>, Refusal> {
    reader(&state, &headers)?;
    let Query(filter) =
        filter.map_err(|_| refuse(StatusCode::BAD_REQUEST, "a filter is malformed"))?;
    listing(&state.db, filter).await.map(Json)
}

fn report_id(id: &str) -> Result<Uuid, Refusal> {
    Uuid::parse_str(id).map_err(|_| refuse(StatusCode::NOT_FOUND, "no such report"))
}

async fn one(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Summary>, Refusal> {
    reader(&state, &headers)?;
    let id = report_id(&id)?;
    fetch(&state.db, id).await
}

async fn fetch(db: &DatabaseConnection, id: Uuid) -> Result<Json<Summary>, Refusal> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            format!("SELECT {SUMMARY}, client FROM feedback_report WHERE id = $1"),
            [id.into()],
        ))
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| refuse(StatusCode::NOT_FOUND, "no such report"))?;
    Summary::of(&row, true).map(Json).map_err(|e| db_down(&e))
}

/// The game record, as the gzip stream the gateway stored.
async fn record(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, Refusal> {
    reader(&state, &headers)?;
    record_of(&state.db, report_id(&id)?).await
}

async fn record_of(db: &DatabaseConnection, id: Uuid) -> Result<Response, Refusal> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT record, record_origin FROM feedback_report \
             WHERE id = $1 AND record IS NOT NULL",
            [id.into()],
        ))
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| refuse(StatusCode::NOT_FOUND, "no record for that report"))?;
    let bytes: Vec<u8> = row.try_get("", "record").map_err(|e| db_down(&e))?;
    let origin: Option<String> = row.try_get("", "record_origin").map_err(|e| db_down(&e))?;
    let mut response = bytes.into_response();
    let headers = response.headers_mut();
    // Who wrote it, on the record itself: a client's is unverified.
    if let Ok(origin) = HeaderValue::from_str(origin.as_deref().unwrap_or("gateway")) {
        headers.insert("x-record-origin", origin);
    }
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/gzip"),
    );
    if let Ok(name) = HeaderValue::from_str(&format!("attachment; filename=\"{id}.jsonl.gz\"")) {
        headers.insert(header::CONTENT_DISPOSITION, name);
    }
    Ok(response)
}

/// What a change does to a report's issue.
#[derive(Clone, Copy, Default)]
enum IssueChange {
    /// `issue` was left out: the link stays as it is.
    #[default]
    Keep,
    /// `"issue": null`: no issue any more.
    Unlink,
    /// `"issue": n`.
    Link(u32),
}

impl<'de> Deserialize<'de> for IssueChange {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Option::<u32>::deserialize(d)?.map_or(Self::Unlink, Self::Link))
    }
}

/// What an admin may change about a report: its status, its issue, or
/// both. `issue: null` unlinks it; leaving `issue` out leaves it alone.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Change {
    #[serde(default)]
    status: Option<Status>,
    #[serde(default)]
    issue: IssueChange,
}

/// Who changed something, as the audit names them: an admin by name, or
/// `token` for the admin token.
pub(crate) const TOKEN_ACTOR: &str = "token";

async fn audit(
    db: &impl ConnectionTrait,
    actor: &str,
    report: Uuid,
    action: &str,
    detail: Option<String>,
) -> Result<(), sea_orm::DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO feedback_audit (id, actor, report_id, action, detail) \
         VALUES ($1, $2, $3, $4, $5)",
        [
            Uuid::now_v7().into(),
            actor.into(),
            report.into(),
            action.into(),
            detail.into(),
        ],
    ))
    .await
    .map(|_| ())
}

/// Applies `change` to report `id` as `actor`, and notes each part of it in
/// the audit, in one transaction.
async fn apply_change(
    db: &DatabaseConnection,
    id: Uuid,
    actor: &str,
    change: Change,
) -> Result<Json<Summary>, Refusal> {
    let bad = |why| refuse(StatusCode::BAD_REQUEST, why);
    let issue = match change.issue {
        IssueChange::Keep => None,
        IssueChange::Unlink => Some(None),
        IssueChange::Link(n) => Some(Some(
            i32::try_from(n)
                .ok()
                .filter(|n| *n > 0)
                .ok_or_else(|| bad("not an issue number"))?,
        )),
    };
    if change.status.is_none() && issue.is_none() {
        return Err(bad("nothing to change"));
    }
    let down = |e: sea_orm::DbErr| db_down(&e);
    let tx = db.begin().await.map_err(down)?;
    let done = tx
        .execute_raw(Statement::from_sql_and_values(
            tx.get_database_backend(),
            "UPDATE feedback_report SET \
                 status = coalesce($2, status), \
                 issue_number = CASE WHEN $3 THEN $4 ELSE issue_number END, \
                 updated_at = now() \
             WHERE id = $1",
            [
                id.into(),
                change.status.map(Status::name).into(),
                issue.is_some().into(),
                issue.flatten().into(),
            ],
        ))
        .await
        .map_err(down)?;
    if done.rows_affected() == 0 {
        return Err(refuse(StatusCode::NOT_FOUND, "no such report"));
    }
    if let Some(status) = change.status {
        audit(&tx, actor, id, "status", Some(status.name().to_owned()))
            .await
            .map_err(down)?;
    }
    if let Some(issue) = issue {
        audit(&tx, actor, id, "issue", issue.map(|n| format!("#{n}")))
            .await
            .map_err(down)?;
    }
    tx.commit().await.map_err(down)?;
    fetch(db, id).await
}

async fn set_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Json<Summary>, Refusal> {
    admin(&state, &headers)?;
    let id = report_id(&id)?;
    let change: StatusChange = serde_json::from_slice(&body)
        .map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a status"))?;
    apply_change(
        &state.db,
        id,
        TOKEN_ACTOR,
        Change {
            status: Some(change.status),
            issue: IssueChange::Keep,
        },
    )
    .await
}

#[derive(Deserialize)]
struct StatusChange {
    status: Status,
}

async fn delete_report(db: &DatabaseConnection, id: Uuid, actor: &str) -> Result<(), Refusal> {
    let down = |e: sea_orm::DbErr| db_down(&e);
    let tx = db.begin().await.map_err(down)?;
    let done = tx
        .execute_raw(Statement::from_sql_and_values(
            tx.get_database_backend(),
            "DELETE FROM feedback_report WHERE id = $1",
            [id.into()],
        ))
        .await
        .map_err(down)?;
    if done.rows_affected() == 0 {
        return Err(refuse(StatusCode::NOT_FOUND, "no such report"));
    }
    audit(&tx, actor, id, "delete", None).await.map_err(down)?;
    tx.commit().await.map_err(down)?;
    tracing::info!(report = %id, actor, "report deleted");
    Ok(())
}

async fn remove(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, Refusal> {
    admin(&state, &headers)?;
    let id = report_id(&id)?;
    delete_report(&state.db, id, TOKEN_ACTOR).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: &str = "0123456789abcdef";

    #[test]
    fn a_configuration_names_each_gateway_by_its_token() {
        let config = Config::new(&format!("eu={T}a, us-1={T}b"), Some(&format!("{T}r")), None)
            .expect("a fine configuration");
        assert_eq!(config.gateway(&format!("{T}a")), Some("eu"));
        assert_eq!(config.gateway(&format!("{T}b")), Some("us-1"));
        assert_eq!(
            config.gateway(&format!("{T}r")),
            None,
            "the read token is not a gateway's"
        );
        assert_eq!(config.gateway(""), None);
        assert!(Config::is(config.read.as_ref(), &format!("{T}r")));
        assert!(!Config::is(config.admin.as_ref(), &format!("{T}r")));
    }

    #[test]
    fn a_configuration_that_could_be_misread_is_refused() {
        for (gateways, read) in [
            ("eu", None),
            ("eu=short", None),
            ("=0123456789abcdef", None),
            ("e u=0123456789abcdef", None),
            ("eu=0123456789abcdef,us=0123456789abcdef", None),
            ("eu=0123456789abcdef", Some("0123456789abcdef")),
        ] {
            assert!(
                Config::new(gateways, read, None).is_err(),
                "{gateways} {read:?}"
            );
        }
    }
}
