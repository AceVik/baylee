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
///
/// @description A message from the landing page's contact form; every
/// field named, no other taken. Blank `name` and `email` are none.
/// @openapi
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContactIn {
    /// @description The sender's name, if given: at most 100 characters, no
    /// control character.
    #[serde(default)]
    name: Option<String>,
    /// @description Where to answer, if given: at most 254 characters, one
    /// `@` with something before it and a dot in the domain after it, no
    /// whitespace.
    #[serde(default)]
    email: Option<String>,
    /// @description What they wrote: 1 to 5000 characters after trimming;
    /// line breaks and tabs are the only control characters taken.
    message: String,
    /// @description The page's language when it was sent.
    /// @openapi enum: [en, de]
    language: String,
    /// @description The honeypot: the page hides it from people. Filled, the
    /// message is answered as taken and dropped.
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
///
/// @route OPTIONS /site/contact
/// @scope feedback
/// @summary A browser's preflight for the contact form
/// @description Says yes only to an `Origin` in `FEEDBACK_CONTACT_ORIGINS`:
/// `POST`, `Content-Type`, for a day. Every answer says `Vary: Origin`.
/// @tag Site
/// @operation-id `preflightContact`
/// @security none
/// @header-param Origin: String required "The page's origin"
/// @return 204: "Allowed"
/// @return-header 204 Access-Control-Allow-Origin: String "The asking origin"
/// @return-header 204 Access-Control-Allow-Methods: String "`POST, OPTIONS`"
/// @return-header 204 Access-Control-Allow-Headers: String "`content-type`"
/// @return-header 204 Access-Control-Max-Age: String "`86400`"
/// @return 403: "The `Origin` is missing or not a configured one"
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

/// A contact message taken.
/// @openapi
#[derive(Serialize)]
struct ContactTaken {
    /// @description Always `true`.
    ok: bool,
}

/// `POST /site/contact`.
///
/// @route POST /site/contact
/// @scope feedback
/// @summary Send a message from the landing page
/// @description The landing page's contact form posts here across origins.
/// Only a page served at one of `FEEDBACK_CONTACT_ORIGINS` may: every answer
/// says `Vary: Origin` and names the asking origin in
/// `Access-Control-Allow-Origin` only when it is a configured one, never with
/// credentials. 5 messages per address and 100 from everyone per hour, every
/// request past the origin check counted. A filled honeypot is answered
/// `201` and nothing is stored. Off until `FEEDBACK_CONTACT_ORIGINS` is set.
/// @tag Site
/// @operation-id `postContact`
/// @security none
/// @header-param Origin: String required "The page's origin, one of `FEEDBACK_CONTACT_ORIGINS`"
/// @body `$ContactIn`
/// @return 201: `$ContactTaken` "Taken, or dropped for its honeypot"
/// @return 400: `$ErrorBody` "Not a message: malformed JSON, an unknown field, an empty message, a malformed name or address, or another language"
/// @return 403: `$ErrorBody` "The `Origin` is missing or not one of `FEEDBACK_CONTACT_ORIGINS`"
/// @return 413: `$ErrorBody` "A field over its bound, or the body over 32 KiB"
/// @return 429: `$ErrorBody` "Too many messages from this address or from everyone in the last hour"
/// @return 503: `$ErrorBody` "Contact messages are not taken here (`FEEDBACK_CONTACT_ORIGINS` is unset), or the database is unavailable"
pub(crate) async fn post(
    State(shared): State<Shared>,
    peer: Option<axum::Extension<ConnectInfo<std::net::SocketAddr>>>,
    headers: HeaderMap,
    body: Result<Bytes, axum::extract::rejection::BytesRejection>,
) -> Response {
    let origin = allowed_origin(&shared, &headers).map(str::to_owned);
    // A body past the route's limit is refused here, not by the extractor,
    // so the refusal carries CORS and the page can say what went wrong.
    let body = body.ok();
    let answer = take(&shared, peer, &headers, origin.is_some(), body.as_deref()).await;
    with_cors(answer.into_response(), origin.as_deref())
}

async fn take(
    shared: &Shared,
    peer: Option<axum::Extension<ConnectInfo<std::net::SocketAddr>>>,
    headers: &HeaderMap,
    origin_allowed: bool,
    body: Option<&[u8]>,
) -> Result<(StatusCode, Json<ContactTaken>), Refusal> {
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
    let Some(body) = body.filter(|b| b.len() <= MAX_BODY_BYTES) else {
        return Err(refuse(
            StatusCode::PAYLOAD_TOO_LARGE,
            "the message is too large",
        ));
    };
    let bad = |why| refuse(StatusCode::BAD_REQUEST, why);
    let input: ContactIn = serde_json::from_slice(body).map_err(|_| bad("not a message"))?;
    if input
        .website
        .as_deref()
        .is_some_and(|w| !w.trim().is_empty())
    {
        // Answered as if taken, so a robot learns nothing to adjust.
        tracing::info!("a contact message filled the honeypot and was dropped");
        return Ok((StatusCode::CREATED, Json(ContactTaken { ok: true })));
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
    Ok((StatusCode::CREATED, Json(ContactTaken { ok: true })))
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
///
/// @description A contact message as the web UI shows it.
/// @openapi
#[derive(Serialize)]
struct ContactMessage {
    /// @description Its id.
    /// @openapi format: uuid
    id: String,
    /// @description When it came, RFC 3339 in UTC.
    /// @openapi format: date-time
    created_at: String,
    /// @description When it last changed, RFC 3339 in UTC.
    /// @openapi format: date-time
    updated_at: String,
    /// @description The sender's name, if given.
    name: Option<String>,
    /// @description Where to answer, if given.
    email: Option<String>,
    /// @description What they wrote.
    message: String,
    /// @description The page's language when it was sent.
    /// @openapi enum: [en, de]
    language: String,
    /// @description Whether an admin has dealt with it.
    /// @openapi enum: [new, done]
    status: String,
    /// @description When it was marked done, RFC 3339 in UTC.
    /// @openapi format: date-time
    done_at: Option<String>,
}

const COLUMNS: &str = "id::text AS id, \
    to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at, \
    to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at, \
    name, email, message, language, status, \
    to_char(done_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS done_at";

impl ContactMessage {
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

/// `GET /ui/api/contact`'s filter.
/// @openapi
#[derive(Deserialize)]
struct ContactFilter {
    /// @description `new` or `done`; absent, empty or `all` for every
    /// message.
    /// @openapi enum: [new, done, all, '']
    #[serde(default)]
    status: Option<String>,
}

/// The contact messages.
/// @openapi
#[derive(Serialize)]
struct ContactListing {
    /// @description Messages not yet marked done, all of them.
    /// @internal
    /// Messages not yet marked done, all of them.
    open: i64,
    /// @description Messages the filter matches, all of them.
    /// @internal
    /// Messages the filter matches, all of them.
    total: i64,
    /// @description The newest of those, at most 500.
    /// @internal
    /// The newest of those, at most [`LIST_ROWS`].
    messages: Vec<ContactMessage>,
}

fn status_filter(status: Option<&str>) -> Result<Option<&'static str>, Refusal> {
    match status {
        None | Some("" | "all") => Ok(None),
        Some("new") => Ok(Some("new")),
        Some("done") => Ok(Some("done")),
        Some(_) => Err(refuse(StatusCode::BAD_REQUEST, "a filter is malformed")),
    }
}

/// @route GET /ui/api/contact
/// @scope feedback
/// @summary The contact messages
/// @description The newest 500 messages the filter matches, how many it
/// matches and how many are open. Messages past their time are deleted
/// first.
/// @tag Contact
/// @operation-id `listContact`
/// @security session
/// @query-params `$ContactFilter`
/// @insert `SessionRefusals`
/// @return 400: `$ErrorBody` "A filter is malformed"
async fn list(
    State(shared): State<Shared>,
    headers: HeaderMap,
    query: Result<Query<ContactFilter>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<ContactListing>, Refusal> {
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
    Ok(Json(ContactListing {
        open: counts.try_get("", "open").map_err(down)?,
        total: counts.try_get("", "total").map_err(down)?,
        messages: rows
            .iter()
            .map(ContactMessage::of)
            .collect::<Result<_, _>>()
            .map_err(down)?,
    }))
}

/// One line of a contact message's audit.
/// @openapi
#[derive(Serialize)]
struct ContactAuditEntry {
    /// @description When, RFC 3339 in UTC.
    /// @openapi format: date-time
    at: String,
    /// @description The admin.
    actor: String,
    /// @description `status` or `delete`.
    action: String,
    /// @description The new status, for `status`.
    detail: Option<String>,
}

/// One contact message with its audit.
/// @openapi
#[derive(Serialize)]
struct ContactDetail {
    #[serde(flatten)]
    message: ContactMessage,
    /// @description Who changed it and when, oldest first.
    audit: Vec<ContactAuditEntry>,
}

fn contact_id(id: &str) -> Result<Uuid, Refusal> {
    Uuid::parse_str(id).map_err(|_| refuse(StatusCode::NOT_FOUND, "no such message"))
}

async fn fetch(db: &DatabaseConnection, id: Uuid) -> Result<Json<ContactDetail>, Refusal> {
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
    Ok(Json(ContactDetail {
        message: ContactMessage::of(&row).map_err(down)?,
        audit: audit
            .iter()
            .map(|row| {
                Ok(ContactAuditEntry {
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

/// @route GET /ui/api/contact/{id}
/// @scope feedback
/// @summary One contact message
/// @description One message with its audit.
/// @tag Contact
/// @operation-id `getContact`
/// @security session
/// @path-param id: String format=uuid "The message's id"
/// @insert `SessionRefusals`
/// @return 404: `$ErrorBody` "No such message"
async fn one(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ContactDetail>, Refusal> {
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

/// `PATCH /ui/api/contact/{id}`.
/// @openapi
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContactChange {
    /// @description `new`, or `done` once an admin has dealt with it.
    /// @openapi enum: [new, done]
    status: String,
}

/// @route PATCH /ui/api/contact/{id}
/// @scope feedback
/// @summary Mark a contact message new or done
/// @description Sets a message's status and answers it with its audit. The
/// change is an audit row with the admin, never the text or the address. A
/// message is deleted 90 days after it was marked done.
/// @tag Contact
/// @operation-id `changeContact`
/// @security session + csrf
/// @path-param id: String format=uuid "The message's id"
/// @body `$ContactChange`
/// @insert `SessionChangeRefusals`
/// @return 400: `$ErrorBody` "Not a change, or not a status"
/// @return 404: `$ErrorBody` "No such message"
async fn change(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Json<ContactDetail>, Refusal> {
    crate::ui::same_origin(&headers)?;
    let signed_in = crate::ui::session(&shared.state, &shared.ui, &headers).await?;
    let id = contact_id(&id)?;
    let change: ContactChange = serde_json::from_slice(&body)
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

/// @route DELETE /ui/api/contact/{id}
/// @scope feedback
/// @summary Delete a contact message
/// @description Deletes a message now. The deletion is an audit row that
/// outlives it.
/// @tag Contact
/// @operation-id `deleteContact`
/// @security session + csrf
/// @path-param id: String format=uuid "The message's id"
/// @insert `SessionChangeRefusals`
/// @return 204: "Deleted"
/// @return 404: `$ErrorBody` "No such message"
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A message with nothing else filled in, for the cases that vary one field.
    fn ok_with(
        name: Option<&str>,
        email: Option<&str>,
        message: &str,
        language: &str,
    ) -> Result<Checked, Malformed> {
        check(name, email, message, language)
    }

    /// A page's origin is `https://host[:port]`, lowercased, or `http://`
    /// for a loopback host only.
    #[test]
    fn an_origin_is_https_or_loopback_http_and_comes_back_lowercased() {
        for (entry, want) in [
            ("https://gateway.baylee.app", "https://gateway.baylee.app"),
            ("https://Example.org:8443", "https://example.org:8443"),
            ("http://localhost:8000", "http://localhost:8000"),
            ("http://127.0.0.1:8000", "http://127.0.0.1:8000"),
            ("http://[::1]:8000", "http://[::1]:8000"),
        ] {
            assert_eq!(origin(entry).as_deref(), Some(want), "{entry}");
        }
    }

    /// Plain http to a public host, a path, a query, credentials, another
    /// scheme, an empty or broken authority and whitespace are no origin.
    #[test]
    fn an_origin_with_a_path_credentials_or_plain_http_to_the_world_is_refused() {
        for entry in [
            "http://gateway.baylee.app",
            "https://x/",
            "https://x/path",
            "https://x?q",
            "https://user@x",
            "ftp://x",
            "https://",
            "https://x:",
            "https://x:123456",
            "https://a b",
            " https://x",
            "",
            "gateway.baylee.app",
        ] {
            assert_eq!(origin(entry), None, "`{entry}`");
        }
    }

    /// Blank name and e-mail are no answer, and the message loses its
    /// outer whitespace.
    #[test]
    fn blank_optional_fields_are_none_and_the_message_is_trimmed() {
        let checked = ok_with(Some("  "), Some(""), "  hello \n", "en").unwrap();
        assert_eq!(
            checked,
            Checked {
                name: None,
                email: None,
                message: "hello".to_owned(),
                language: "en".to_owned(),
            }
        );
        let checked = ok_with(Some(" Max "), Some(" max@example.org "), "hi", "de").unwrap();
        assert_eq!(checked.name.as_deref(), Some("Max"));
        assert_eq!(checked.email.as_deref(), Some("max@example.org"));
        assert_eq!(checked.language, "de");
    }

    /// No message, or only whitespace, is said to be empty.
    #[test]
    fn an_empty_or_whitespace_message_is_empty() {
        for message in ["", "   ", "\n\t \r\n"] {
            assert_eq!(
                ok_with(None, None, message, "en"),
                Err(Malformed::Empty),
                "{message:?}"
            );
        }
    }

    /// The message bound counts characters, not bytes, and is inclusive.
    #[test]
    fn a_message_of_the_longest_length_is_taken_and_one_more_is_too_long() {
        assert!(ok_with(None, None, &"a".repeat(MAX_MESSAGE_CHARS), "en").is_ok());
        assert!(ok_with(None, None, &"ä".repeat(MAX_MESSAGE_CHARS), "en").is_ok());
        assert_eq!(
            ok_with(None, None, &"a".repeat(MAX_MESSAGE_CHARS + 1), "en"),
            Err(Malformed::TooLong)
        );
        assert_eq!(
            ok_with(None, None, &"ä".repeat(MAX_MESSAGE_CHARS + 1), "en"),
            Err(Malformed::TooLong)
        );
    }

    /// A name and an address have bounds of their own.
    #[test]
    fn a_name_or_an_address_over_its_bound_is_too_long() {
        assert!(ok_with(Some(&"n".repeat(MAX_NAME_CHARS)), None, "hi", "en").is_ok());
        assert_eq!(
            ok_with(Some(&"n".repeat(MAX_NAME_CHARS + 1)), None, "hi", "en"),
            Err(Malformed::TooLong)
        );
        let address = |len: usize| format!("{}@x.de", "a".repeat(len - "@x.de".len()));
        assert!(ok_with(None, Some(&address(MAX_EMAIL_CHARS)), "hi", "en").is_ok());
        assert_eq!(
            ok_with(None, Some(&address(MAX_EMAIL_CHARS + 1)), "hi", "en"),
            Err(Malformed::TooLong)
        );
    }

    /// What a reply could go to: one `@`, something before it, a dot in
    /// the domain, no whitespace.
    #[test]
    fn an_email_without_an_at_a_second_at_a_domain_dot_or_with_whitespace_is_malformed() {
        for email in [
            "max.example.org",
            "a@b@example.org",
            "max@example",
            "max@.org",
            "max@example.",
            "@example.org",
            "ma x@example.org",
            "max@exa mple.org",
        ] {
            assert_eq!(
                ok_with(None, Some(email), "hi", "en"),
                Err(Malformed::Malformed),
                "{email:?}"
            );
        }
        assert!(ok_with(None, Some("max@example.org"), "hi", "en").is_ok());
    }

    /// The page is written in English and German only.
    #[test]
    fn a_language_other_than_english_or_german_is_malformed() {
        for language in ["en", "de"] {
            assert!(ok_with(None, None, "hi", language).is_ok(), "{language}");
        }
        for language in ["fr", "EN", "", "en ", "english"] {
            assert_eq!(
                ok_with(None, None, "hi", language),
                Err(Malformed::Malformed),
                "{language:?}"
            );
        }
    }

    /// A message keeps its line breaks and tabs; any other control
    /// character is refused, and a name takes none at all.
    #[test]
    fn control_characters_are_refused_but_line_breaks_and_tabs_belong_in_a_message() {
        let checked = ok_with(None, None, "one\ntwo\tthree\r\nfour", "en").unwrap();
        assert_eq!(checked.message, "one\ntwo\tthree\r\nfour");
        for message in ["a\u{0}b", "a\u{7}b", "a\u{1b}[31mb", "a\u{7f}b"] {
            assert_eq!(
                ok_with(None, None, message, "en"),
                Err(Malformed::Malformed),
                "{message:?}"
            );
        }
        for name in ["Ma\u{0}x", "Ma\nx", "Ma\tx"] {
            assert_eq!(
                ok_with(Some(name), None, "hi", "en"),
                Err(Malformed::Malformed),
                "{name:?}"
            );
        }
    }

    /// An entry that is no origin refuses the whole setting, and the error
    /// names it.
    #[test]
    fn a_bad_contact_origin_entry_refuses_the_configuration() {
        let config = || crate::Config::new("", None, None).unwrap();
        for list in [
            "https://site.example/",
            "https://site.example, http://site.example",
            "https://site.example,ftp://x",
            "site.example",
        ] {
            let error = config().with_contact_origins(list).err().expect(list);
            assert!(
                error.to_string().contains("FEEDBACK_CONTACT_ORIGINS"),
                "{list}"
            );
        }
    }

    /// A comma list of origins is taken; blanks and repeats are dropped, and
    /// a request's `Origin` matches without regard to case.
    #[test]
    fn contact_origins_match_case_insensitively_and_nothing_else_does() {
        let config = crate::Config::new("", None, None)
            .unwrap()
            .with_contact_origins(
                " https://Site.example , http://localhost:8000,,https://site.example ",
            )
            .unwrap();
        assert!(config.takes_contact());
        assert!(config.contact_origin("https://site.example"));
        assert!(config.contact_origin("HTTPS://SITE.EXAMPLE"));
        assert!(config.contact_origin("http://localhost:8000"));
        for other in [
            "https://evil.example",
            "https://site.example.evil.example",
            "http://site.example",
            "https://site.example:8443",
            "https://site.example/",
            "null",
            "",
        ] {
            assert!(!config.contact_origin(other), "{other}");
        }
    }

    /// Without origins the route is off, and an empty list leaves it off.
    #[test]
    fn contact_is_off_without_origins() {
        let config = crate::Config::new("", None, None).unwrap();
        assert!(!config.takes_contact());
        assert!(!config.contact_origin("https://site.example"));
        let config = config.with_contact_origins("").unwrap();
        assert!(!config.takes_contact());
        let config = config.with_contact_origins(" , ").unwrap();
        assert!(!config.takes_contact());
    }
}
