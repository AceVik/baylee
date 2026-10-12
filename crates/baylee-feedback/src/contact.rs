//! `POST /site/contact`: a message from the landing page's contact form
//! (`web/site`, `docs/feedback.md` §"The landing page's contact form"), and
//! the web UI's routes that read and keep them.
//!
//! The page is served from another origin than this service, so the form
//! posts across origins. Only the origins in `FEEDBACK_CONTACT_ORIGINS` are
//! named in an answer (CORS), and a request whose `Origin` is any other, or
//! that names none, is refused: a browser always sends one with such a
//! `fetch`. Like the direct reports, the route has no credential to ask
//! for and stands on its limits: a small body, bounded fields, an
//! allowance per address and one for everyone together per hour (memory
//! only), and a honeypot field a person never sees. A filled honeypot is
//! answered like a message taken and nothing is stored.
//!
//! A message keeps what its sender typed (a name and an e-mail address only
//! if given), the page's language, when it came and whether an admin has
//! dealt with it. No address, no device, nothing about the request. It is
//! deleted [`DONE_KEPT`] after an admin marked it done, and [`KEPT`] after
//! it came in any case ([`sweep`]), or earlier by an admin.
//!
//! - `GET /ui/api/contact?status=new|done`: the newest [`LIST_ROWS`], and
//!   how many are open.
//! - `GET /ui/api/contact/{id}`: one, with its audit.
//! - `PATCH /ui/api/contact/{id}` `{status}`, `DELETE /ui/api/contact/{id}`:
//!   audited by admin name, never with the text or the address.
//!
//! Off until `FEEDBACK_CONTACT_ORIGINS` is set (`503` before then).

use axum::Router;
use axum::body::Bytes;
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement, TransactionTrait as _};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Refusal, Shared, db_down, refuse};

/// The largest body the route reads.
pub const MAX_BODY_BYTES: usize = 32 * 1024;

/// The longest message, in characters.
pub const MAX_MESSAGE_CHARS: usize = 5_000;

/// The longest name, in characters.
pub const MAX_NAME_CHARS: usize = 100;

/// The longest e-mail address, in characters (RFC 5321's path limit).
pub const MAX_EMAIL_CHARS: usize = 254;

/// Messages one address may send per [`crate::direct::WINDOW`].
pub const PER_ADDRESS: usize = 5;

/// Messages the service takes per [`crate::direct::WINDOW`] from everyone.
pub const PER_SERVICE: usize = 100;

/// How long a message is kept after an admin marked it done, in days.
pub const DONE_KEPT: i32 = 90;

/// How long any message is kept after it came, in days.
pub const KEPT: i32 = 365;

/// The most messages one list answers.
pub const LIST_ROWS: u32 = 500;

/// The languages the page is written in.
const LANGUAGES: [&str; 2] = ["en", "de"];

/// `entry` as an origin a contact form may be served from, lowercased:
/// `https://host[:port]`, or `http://` for a loopback host only.
#[must_use]
pub fn origin(entry: &str) -> Option<String> {
    let lower = entry.to_ascii_lowercase();
    let (scheme, authority) = lower
        .strip_prefix("https://")
        .map(|a| ("https", a))
        .or_else(|| lower.strip_prefix("http://").map(|a| ("http", a)))?;
    let clean = !authority.is_empty()
        && authority
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'));
    if !clean {
        return None;
    }
    let host = match authority.strip_prefix('[') {
        Some(v6) => format!("[{}]", v6.split_once(']')?.0),
        None => authority.split(':').next()?.to_owned(),
    };
    let rest = &authority[host.len()..];
    let port_fine = rest.is_empty()
        || rest.strip_prefix(':').is_some_and(|p| {
            !p.is_empty() && p.len() <= 5 && p.chars().all(|c| c.is_ascii_digit())
        });
    if host.is_empty() || host.starts_with('.') || host.ends_with('.') || !port_fine {
        return None;
    }
    if scheme == "http" && !matches!(host.as_str(), "localhost" | "127.0.0.1" | "[::1]") {
        return None;
    }
    Some(format!("{scheme}://{authority}"))
}

/// What the page sends. `website` is the honeypot: a field the page hides
/// from people, which a form-filling robot fills.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContactIn {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    email: Option<String>,
    message: String,
    language: String,
    #[serde(default)]
    website: Option<String>,
}

/// A message as it is checked: blanks are no answer.
#[derive(Debug, PartialEq, Eq)]
pub struct Checked {
    /// The sender's name, if given.
    pub name: Option<String>,
    /// Where to answer, if given.
    pub email: Option<String>,
    /// What they wrote, trimmed.
    pub message: String,
    /// `en` or `de`.
    pub language: String,
}

/// Why a message is refused.
#[derive(Debug, PartialEq, Eq)]
pub enum Malformed {
    /// No message, or only whitespace.
    Empty,
    /// A field over its bound.
    TooLong,
    /// A control character where none belongs, an address that is not one,
    /// or a language the page is not written in.
    Malformed,
}

/// No control character but, in a message, line breaks and tabs.
fn printable(text: &str, multiline: bool) -> bool {
    text.chars()
        .all(|c| !c.is_control() || (multiline && matches!(c, '\n' | '\r' | '\t')))
}

/// An address with one `@`, something before it, a dot in the domain after
/// it and no whitespace: what a reply could go to, not a proof that it
/// would arrive.
fn address_shaped(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !email.chars().any(char::is_whitespace)
}

/// Checks a message's fields.
///
/// # Errors
///
/// [`Malformed`] says which way it is refused.
pub fn check(
    name: Option<&str>,
    email: Option<&str>,
    message: &str,
    language: &str,
) -> Result<Checked, Malformed> {
    let given = |v: Option<&str>| {
        v.map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_owned)
    };
    let name = given(name);
    let email = given(email);
    let message = message.trim().to_owned();
    if message.is_empty() {
        return Err(Malformed::Empty);
    }
    if message.chars().count() > MAX_MESSAGE_CHARS
        || name
            .as_ref()
            .is_some_and(|n| n.chars().count() > MAX_NAME_CHARS)
        || email
            .as_ref()
            .is_some_and(|e| e.chars().count() > MAX_EMAIL_CHARS)
    {
        return Err(Malformed::TooLong);
    }
    let fine = printable(&message, true)
        && name.as_deref().is_none_or(|n| printable(n, false))
        && email.as_deref().is_none_or(address_shaped)
        && LANGUAGES.contains(&language);
    if !fine {
        return Err(Malformed::Malformed);
    }
    Ok(Checked {
        name,
        email,
        message,
        language: language.to_owned(),
    })
}

/// The `Origin` a request names, if it is one of the configured ones.
fn allowed_origin<'h>(shared: &Shared, headers: &'h HeaderMap) -> Option<&'h str> {
    headers
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .filter(|o| shared.state.config.contact_origin(o))
}

/// Names `origin` in the answer, so the page may read it.
fn with_cors(mut response: Response, origin: Option<&str>) -> Response {
    let headers = response.headers_mut();
    headers.insert(header::VARY, HeaderValue::from_static("Origin"));
    if let Some(value) = origin.and_then(|o| HeaderValue::from_str(o).ok()) {
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, value);
    }
    response
}

/// The browser's preflight for [`post`]: yes to a configured origin only.
pub(crate) async fn preflight(State(shared): State<Shared>, headers: HeaderMap) -> Response {
    let Some(origin) = allowed_origin(&shared, &headers) else {
        return with_cors(StatusCode::FORBIDDEN.into_response(), None);
    };
    let mut response = StatusCode::NO_CONTENT.into_response();
    let h = response.headers_mut();
    h.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("POST, OPTIONS"),
    );
    h.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("content-type"),
    );
    h.insert(
        header::ACCESS_CONTROL_MAX_AGE,
        HeaderValue::from_static("86400"),
    );
    with_cors(response, Some(origin))
}

#[derive(Serialize)]
struct Taken {
    ok: bool,
}

/// `POST /site/contact`.
pub(crate) async fn post(
    State(shared): State<Shared>,
    peer: Option<axum::Extension<ConnectInfo<std::net::SocketAddr>>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let origin = allowed_origin(&shared, &headers).map(str::to_owned);
    let answer = take(&shared, peer, &headers, origin.is_some(), &body).await;
    with_cors(answer.into_response(), origin.as_deref())
}

async fn take(
    shared: &Shared,
    peer: Option<axum::Extension<ConnectInfo<std::net::SocketAddr>>>,
    headers: &HeaderMap,
    origin_allowed: bool,
    body: &[u8],
) -> Result<(StatusCode, Json<Taken>), Refusal> {
    if !shared.state.config.takes_contact() {
        return Err(refuse(
            StatusCode::SERVICE_UNAVAILABLE,
            "contact messages are not taken here",
        ));
    }
    if !origin_allowed {
        return Err(refuse(StatusCode::FORBIDDEN, "not from the contact page"));
    }
    let address = shared
        .ui
        .address(peer.map(|axum::Extension(ConnectInfo(a))| a.ip()), headers);
    if !shared.ui.allow_contact(&address) {
        return Err(refuse(StatusCode::TOO_MANY_REQUESTS, "too many messages"));
    }
    if body.len() > MAX_BODY_BYTES {
        return Err(refuse(
            StatusCode::PAYLOAD_TOO_LARGE,
            "the message is too large",
        ));
    }
    let bad = |why| refuse(StatusCode::BAD_REQUEST, why);
    let input: ContactIn = serde_json::from_slice(body).map_err(|_| bad("not a message"))?;
    if input
        .website
        .as_deref()
        .is_some_and(|w| !w.trim().is_empty())
    {
        // Answered as if taken, so a robot learns nothing to adjust.
        tracing::info!("a contact message filled the honeypot and was dropped");
        return Ok((StatusCode::CREATED, Json(Taken { ok: true })));
    }
    let checked = check(
        input.name.as_deref(),
        input.email.as_deref(),
        &input.message,
        &input.language,
    )
    .map_err(|why| match why {
        Malformed::Empty => bad("a message says something"),
        Malformed::TooLong => refuse(StatusCode::PAYLOAD_TOO_LARGE, "a field is too long"),
        Malformed::Malformed => bad("a field is malformed"),
    })?;
    let db = &shared.state.db;
    sweep(db).await.map_err(|e| db_down(&e))?;
    let id = Uuid::now_v7();
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO feedback_contact (id, name, email, message, language) \
         VALUES ($1, $2, $3, $4, $5)",
        [
            id.into(),
            checked.name.into(),
            checked.email.into(),
            checked.message.into(),
            checked.language.into(),
        ],
    ))
    .await
    .map_err(|e| db_down(&e))?;
    tracing::info!(contact = %id, "a contact message was taken");
    Ok((StatusCode::CREATED, Json(Taken { ok: true })))
}

/// Deletes every message past its time: [`DONE_KEPT`] days after it was
/// marked done, [`KEPT`] days after it came. Run before a message is taken
/// and before the list is read, so nothing older is ever shown.
///
/// # Errors
///
/// When the database fails.
pub async fn sweep(db: &impl ConnectionTrait) -> Result<u64, sea_orm::DbErr> {
    let done = db
        .execute_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "DELETE FROM feedback_contact \
             WHERE (status = 'done' AND done_at <= now() - make_interval(days => $1)) \
                OR created_at <= now() - make_interval(days => $2)",
            [DONE_KEPT.into(), KEPT.into()],
        ))
        .await?;
    let gone = done.rows_affected();
    if gone > 0 {
        tracing::info!(
            count = gone,
            "contact messages past their time were deleted"
        );
    }
    Ok(gone)
}

// ------------------------------------------------------------------ the UI

pub(crate) fn routes() -> Router<Shared> {
    Router::new().route("/ui/api/contact", get(list)).route(
        "/ui/api/contact/{id}",
        get(one).patch(change).delete(remove),
    )
}

/// A message as the UI shows it.
#[derive(Serialize)]
struct Shown {
    id: String,
    created_at: String,
    updated_at: String,
    name: Option<String>,
    email: Option<String>,
    message: String,
    language: String,
    status: String,
    done_at: Option<String>,
}

const COLUMNS: &str = "id::text AS id, \
    to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at, \
    to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at, \
    name, email, message, language, status, \
    to_char(done_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS done_at";

impl Shown {
    fn of(row: &sea_orm::QueryResult) -> Result<Self, sea_orm::DbErr> {
        Ok(Self {
            id: row.try_get("", "id")?,
            created_at: row.try_get("", "created_at")?,
            updated_at: row.try_get("", "updated_at")?,
            name: row.try_get("", "name")?,
            email: row.try_get("", "email")?,
            message: row.try_get("", "message")?,
            language: row.try_get("", "language")?,
            status: row.try_get("", "status")?,
            done_at: row.try_get("", "done_at")?,
        })
    }
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(default)]
    status: Option<String>,
}

#[derive(Serialize)]
struct Listing {
    /// Messages not yet marked done, all of them.
    open: i64,
    /// Messages the filter matches, all of them.
    total: i64,
    /// The newest of those, at most [`LIST_ROWS`].
    messages: Vec<Shown>,
}

fn status_filter(status: Option<&str>) -> Result<Option<&'static str>, Refusal> {
    match status {
        None | Some("" | "all") => Ok(None),
        Some("new") => Ok(Some("new")),
        Some("done") => Ok(Some("done")),
        Some(_) => Err(refuse(StatusCode::BAD_REQUEST, "a filter is malformed")),
    }
}

async fn list(
    State(shared): State<Shared>,
    headers: HeaderMap,
    query: Result<Query<ListQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<Listing>, Refusal> {
    crate::ui::session(&shared.state, &shared.ui, &headers).await?;
    let Query(query) =
        query.map_err(|_| refuse(StatusCode::BAD_REQUEST, "a filter is malformed"))?;
    let status = status_filter(query.status.as_deref())?;
    let db = &shared.state.db;
    sweep(db).await.map_err(|e| db_down(&e))?;
    let down = |e: sea_orm::DbErr| db_down(&e);
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            format!(
                "SELECT {COLUMNS} FROM feedback_contact \
                 WHERE ($1::text IS NULL OR status = $1) \
                 ORDER BY created_at DESC, id DESC LIMIT {LIST_ROWS}"
            ),
            [status.into()],
        ))
        .await
        .map_err(down)?;
    let counts = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT count(*) FILTER (WHERE status = 'new') AS open, \
                    count(*) FILTER (WHERE $1::text IS NULL OR status = $1) AS total \
             FROM feedback_contact",
            [status.into()],
        ))
        .await
        .map_err(down)?
        .ok_or_else(|| refuse(StatusCode::SERVICE_UNAVAILABLE, "no count"))?;
    Ok(Json(Listing {
        open: counts.try_get("", "open").map_err(down)?,
        total: counts.try_get("", "total").map_err(down)?,
        messages: rows
            .iter()
            .map(Shown::of)
            .collect::<Result<_, _>>()
            .map_err(down)?,
    }))
}

#[derive(Serialize)]
struct AuditEntry {
    at: String,
    actor: String,
    action: String,
    detail: Option<String>,
}

#[derive(Serialize)]
struct One {
    #[serde(flatten)]
    message: Shown,
    audit: Vec<AuditEntry>,
}

fn contact_id(id: &str) -> Result<Uuid, Refusal> {
    Uuid::parse_str(id).map_err(|_| refuse(StatusCode::NOT_FOUND, "no such message"))
}

async fn fetch(db: &DatabaseConnection, id: Uuid) -> Result<Json<One>, Refusal> {
    let down = |e: sea_orm::DbErr| db_down(&e);
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            format!("SELECT {COLUMNS} FROM feedback_contact WHERE id = $1"),
            [id.into()],
        ))
        .await
        .map_err(down)?
        .ok_or_else(|| refuse(StatusCode::NOT_FOUND, "no such message"))?;
    let audit = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT to_char(at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS at, \
                 actor, action, detail FROM feedback_audit WHERE contact_id = $1 \
             ORDER BY at, id",
            [id.into()],
        ))
        .await
        .map_err(down)?;
    Ok(Json(One {
        message: Shown::of(&row).map_err(down)?,
        audit: audit
            .iter()
            .map(|row| {
                Ok(AuditEntry {
                    at: row.try_get("", "at")?,
                    actor: row.try_get("", "actor")?,
                    action: row.try_get("", "action")?,
                    detail: row.try_get("", "detail")?,
                })
            })
            .collect::<Result<_, sea_orm::DbErr>>()
            .map_err(down)?,
    }))
}

async fn one(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<One>, Refusal> {
    crate::ui::session(&shared.state, &shared.ui, &headers).await?;
    fetch(&shared.state.db, contact_id(&id)?).await
}

/// One audit row about a contact message: who, what, never its text.
async fn audit(
    db: &impl ConnectionTrait,
    actor: &str,
    contact: Uuid,
    action: &str,
    detail: Option<&str>,
) -> Result<(), sea_orm::DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO feedback_audit (id, actor, contact_id, action, detail) \
         VALUES ($1, $2, $3, $4, $5)",
        [
            Uuid::now_v7().into(),
            actor.into(),
            contact.into(),
            action.into(),
            detail.map(str::to_owned).into(),
        ],
    ))
    .await
    .map(|_| ())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Change {
    status: String,
}

async fn change(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Json<One>, Refusal> {
    crate::ui::same_origin(&headers)?;
    let signed_in = crate::ui::session(&shared.state, &shared.ui, &headers).await?;
    let id = contact_id(&id)?;
    let change: Change = serde_json::from_slice(&body)
        .map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a change"))?;
    let status = match change.status.as_str() {
        "new" => "new",
        "done" => "done",
        _ => return Err(refuse(StatusCode::BAD_REQUEST, "not a status")),
    };
    let db = &shared.state.db;
    let down = |e: sea_orm::DbErr| db_down(&e);
    let tx = db.begin().await.map_err(down)?;
    let done = tx
        .execute_raw(Statement::from_sql_and_values(
            tx.get_database_backend(),
            "UPDATE feedback_contact SET status = $2, updated_at = now(), \
                 done_at = CASE WHEN $2 = 'done' THEN coalesce(done_at, now()) ELSE NULL END \
             WHERE id = $1",
            [id.into(), status.into()],
        ))
        .await
        .map_err(down)?;
    if done.rows_affected() == 0 {
        return Err(refuse(StatusCode::NOT_FOUND, "no such message"));
    }
    audit(&tx, &signed_in.name, id, "status", Some(status))
        .await
        .map_err(down)?;
    tx.commit().await.map_err(down)?;
    fetch(db, id).await
}

async fn remove(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, Refusal> {
    crate::ui::same_origin(&headers)?;
    let signed_in = crate::ui::session(&shared.state, &shared.ui, &headers).await?;
    let id = contact_id(&id)?;
    let db = &shared.state.db;
    let down = |e: sea_orm::DbErr| db_down(&e);
    let tx = db.begin().await.map_err(down)?;
    let gone = tx
        .execute_raw(Statement::from_sql_and_values(
            tx.get_database_backend(),
            "DELETE FROM feedback_contact WHERE id = $1",
            [id.into()],
        ))
        .await
        .map_err(down)?;
    if gone.rows_affected() == 0 {
        return Err(refuse(StatusCode::NOT_FOUND, "no such message"));
    }
    audit(&tx, &signed_in.name, id, "delete", None)
        .await
        .map_err(down)?;
    tx.commit().await.map_err(down)?;
    tracing::info!(contact = %id, admin = signed_in.name, "a contact message was deleted");
    Ok(StatusCode::NO_CONTENT)
}
