//! baylee-feedback — the service that keeps what players report (#308).
//!
//! Gateways pass reports on (`POST /reports` on a gateway, #307); this keeps
//! them in a database of its own and answers a small JSON API to whoever
//! holds its read token. `docs/feedback.md` is normative.
//!
//! - `POST /intake/reports`: a gateway, with its own intake token.
//! - `GET /reports`, `GET /reports/{id}`, `GET /reports/{id}/record`: the
//!   read token (or the admin token).
//! - `PATCH /reports/{id}` (status), `DELETE /reports/{id}`: the admin token.
//!
//! Nothing it stores names a person: a report carries the gateway's
//! pseudonym for its reporter, and no request's address is kept or logged.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod migration;

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use base64::Engine as _;
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, Statement, Value};
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
}

impl Config {
    /// Reads `FEEDBACK_GATEWAY_TOKENS`, `FEEDBACK_READ_TOKEN` and
    /// `FEEDBACK_ADMIN_TOKEN`.
    ///
    /// # Errors
    ///
    /// When a value is malformed; see [`Config::new`].
    pub fn from_env() -> Result<Self> {
        let var = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
        Self::new(
            &var("FEEDBACK_GATEWAY_TOKENS").unwrap_or_default(),
            var("FEEDBACK_READ_TOKEN").as_deref(),
            var("FEEDBACK_ADMIN_TOKEN").as_deref(),
        )
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

/// Every route.
pub fn app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route(
            "/intake/reports",
            post(intake).layer(axum::extract::DefaultBodyLimit::max(MAX_INTAKE_BYTES)),
        )
        .route("/reports", get(list))
        .route("/reports/{id}", get(one).patch(set_status).delete(remove))
        .route("/reports/{id}/record", get(record))
        .with_state(state)
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
    let (record, complete) = match report.record {
        Some(r) => {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(r.gzip_base64)
                .map_err(|_| bad("the record is not base64"))?;
            if bytes.len() > MAX_RECORD_BYTES {
                return Err(refuse(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "the record is too large",
                ));
            }
            (Some(bytes), Some(r.complete))
        }
        None => (None, None),
    };
    let id = Uuid::now_v7();
    state
        .db
        .execute_raw(Statement::from_sql_and_values(
            state.db.get_database_backend(),
            "INSERT INTO feedback_report (id, gateway, gateway_name, gateway_url, \
                 gateway_version, reporter, kind, text, game_id, client, record, record_complete) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::jsonb, $11, $12)",
            [
                id.into(),
                gateway.into(),
                report.gateway.name.into(),
                report.gateway.url.into(),
                report.gateway.version.into(),
                report.reporter.into(),
                report.kind.name().into(),
                report.text.into(),
                report.game_id.into(),
                client.into(),
                Value::Bytes(record),
                complete.into(),
            ],
        ))
        .await
        .map_err(|e| db_down(&e))?;
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
    coalesce(octet_length(record), 0)::bigint AS record_bytes";

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
    #[serde(skip_serializing_if = "Option::is_none")]
    client: Option<serde_json::Value>,
}

impl Summary {
    fn of(row: &sea_orm::QueryResult, client: bool) -> Result<Self, sea_orm::DbErr> {
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
            client: if client {
                Some(row.try_get("", "client")?)
            } else {
                None
            },
        })
    }
}

/// The list's filters, all optional.
#[derive(Deserialize, Default)]
struct Filter {
    status: Option<Status>,
    kind: Option<Kind>,
    gateway: Option<String>,
    reporter: Option<String>,
    game_id: Option<String>,
    limit: Option<u64>,
    offset: Option<u64>,
}

#[derive(Serialize)]
struct Listing {
    reports: Vec<Summary>,
    total: i64,
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    filter: Result<Query<Filter>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<Listing>, Refusal> {
    reader(&state, &headers)?;
    let Query(filter) =
        filter.map_err(|_| refuse(StatusCode::BAD_REQUEST, "a filter is malformed"))?;
    let mut clauses: Vec<String> = Vec::new();
    let mut values: Vec<Value> = Vec::new();
    let mut add = |column: &str, value: Value| {
        values.push(value);
        clauses.push(format!("{column} = ${}", values.len()));
    };
    if let Some(status) = filter.status {
        add("status", status.name().into());
    }
    if let Some(kind) = filter.kind {
        add("kind", kind.name().into());
    }
    if let Some(gateway) = filter.gateway {
        add("gateway", gateway.into());
    }
    if let Some(reporter) = filter.reporter {
        add("reporter", reporter.into());
    }
    if let Some(game) = filter.game_id {
        add("game_id", game.into());
    }
    let clause = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    let limit = filter.limit.unwrap_or(50).clamp(1, 200);
    let offset = filter.offset.unwrap_or(0);
    let backend = state.db.get_database_backend();
    let rows = state
        .db
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
    let total = state
        .db
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
    Ok(Json(Listing { reports, total }))
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
    let id = report_id(&id)?;
    let row = state
        .db
        .query_one_raw(Statement::from_sql_and_values(
            state.db.get_database_backend(),
            "SELECT record FROM feedback_report WHERE id = $1 AND record IS NOT NULL",
            [id.into()],
        ))
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| refuse(StatusCode::NOT_FOUND, "no record for that report"))?;
    let bytes: Vec<u8> = row.try_get("", "record").map_err(|e| db_down(&e))?;
    let mut response = bytes.into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/gzip"),
    );
    if let Ok(name) = HeaderValue::from_str(&format!("attachment; filename=\"{id}.jsonl.gz\"")) {
        headers.insert(header::CONTENT_DISPOSITION, name);
    }
    Ok(response)
}

#[derive(Deserialize)]
struct StatusChange {
    status: Status,
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
    let done = state
        .db
        .execute_raw(Statement::from_sql_and_values(
            state.db.get_database_backend(),
            "UPDATE feedback_report SET status = $2, updated_at = now() WHERE id = $1",
            [id.into(), change.status.name().into()],
        ))
        .await
        .map_err(|e| db_down(&e))?;
    if done.rows_affected() == 0 {
        return Err(refuse(StatusCode::NOT_FOUND, "no such report"));
    }
    fetch(&state.db, id).await
}

async fn remove(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, Refusal> {
    admin(&state, &headers)?;
    let id = report_id(&id)?;
    let done = state
        .db
        .execute_raw(Statement::from_sql_and_values(
            state.db.get_database_backend(),
            "DELETE FROM feedback_report WHERE id = $1",
            [id.into()],
        ))
        .await
        .map_err(|e| db_down(&e))?;
    if done.rows_affected() == 0 {
        return Err(refuse(StatusCode::NOT_FOUND, "no such report"));
    }
    tracing::info!(report = %id, "report deleted");
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
