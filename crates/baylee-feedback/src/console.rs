//! The admin pages' road to a gateway's admin console (`docs/feedback.md`
//! §"The admin console"): `/ui/api/admin/…`, for a signed-in admin only,
//! passed on to the gateway's `/admin/…` routes with the gateway's
//! `BAYLEE_ADMIN_TOKEN`, which never leaves this service.
//!
//! - `GET /ui/api/admin/stats`: the gateway's numbers, as it sends them.
//! - `GET /ui/api/admin/invites`: its closed-beta keys, never a key.
//! - `POST /ui/api/admin/invites` `{uses?, expires?, note?, count?}`: makes
//!   keys and answers them, the one time they are shown.
//! - `DELETE /ui/api/admin/invites/{id}`: revokes one.
//! - `GET /ui/api/admin/audit`: the latest changes made through here.
//!
//! The service builds every request itself: a body is read into the four
//! fields the gateway takes and written out again, and an id must be a
//! UUID, so a page can send the gateway nothing but those. A change wants
//! the session *and* the UI's CSRF proof ([`crate::ui::same_origin`]), as
//! every other change here does, and names the signed-in admin to the
//! gateway (`X-Baylee-Admin`), whose audit line carries it; this service
//! writes one of its own into `feedback_audit`, never with a key or a note.
//!
//! The gateway refusing the token (`401`, `403`) is answered `502`, never
//! passed on: a `401` here means "signed out" to the UI, and it is the
//! service's configuration that is wrong, not the admin's session.

use std::time::Duration;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{delete, get};
use sea_orm::{ConnectionTrait as _, Statement};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ui::{same_origin, session};
use crate::{Refusal, Shared, TokenHash, db_down, hash, refuse};

/// The shortest gateway admin token the service takes: the gateway's own
/// minimum for `BAYLEE_ADMIN_TOKEN`.
pub const MIN_TOKEN_CHARS: usize = 32;

/// How long the gateway may take to answer.
const TIMEOUT: Duration = Duration::from_secs(10);

/// The largest answer read from the gateway: a thousand keys' rows with
/// room to spare.
const MAX_ANSWER_BYTES: u64 = 1024 * 1024;

/// How many of the latest console changes `GET /ui/api/admin/audit` shows.
const AUDIT_ROWS: u32 = 50;

/// A gateway's admin console: where, and the token it wants.
#[derive(Clone)]
pub struct Gateway {
    /// `http://127.0.0.1:28767`, no trailing `/`.
    url: String,
    /// `BAYLEE_ADMIN_TOKEN`, sent and never shown.
    token: String,
    /// Its SHA-256, for telling it apart from the service's own tokens.
    token_hash: TokenHash,
}

impl Gateway {
    /// Checks `url` and `token` (see [`crate::Config::with_gateway_admin`]).
    pub(crate) fn new(url: &str, token: &str) -> anyhow::Result<Self> {
        let url = url.trim().trim_end_matches('/');
        let rest = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))
            .unwrap_or_default();
        if rest.is_empty()
            || rest
                .chars()
                .any(|c| matches!(c, '@' | '?' | '#') || c.is_whitespace() || c.is_control())
        {
            anyhow::bail!(
                "FEEDBACK_GATEWAY_ADMIN_URL: {url:?} is not a plain http:// or https:// address"
            );
        }
        if token.chars().count() < MIN_TOKEN_CHARS {
            anyhow::bail!(
                "FEEDBACK_GATEWAY_ADMIN_TOKEN: a token needs at least {MIN_TOKEN_CHARS} characters"
            );
        }
        if token.chars().any(|c| c.is_whitespace() || c.is_control()) {
            anyhow::bail!("FEEDBACK_GATEWAY_ADMIN_TOKEN: a token is one word");
        }
        Ok(Self {
            url: url.to_owned(),
            token: token.to_owned(),
            token_hash: hash(token),
        })
    }

    /// The token's SHA-256.
    pub(crate) fn token_hash(&self) -> TokenHash {
        self.token_hash
    }
}

pub(crate) fn routes() -> Router<Shared> {
    Router::new()
        .route("/ui/api/admin/stats", get(stats))
        .route("/ui/api/admin/invites", get(invites).post(create))
        .route("/ui/api/admin/invites/{id}", delete(revoke))
        .route("/ui/api/admin/audit", get(audit))
}

/// What the gateway answered: its status and its JSON (`null` for none).
struct Answer {
    status: u16,
    json: serde_json::Value,
}

/// Asks the gateway, off the runtime's threads.
async fn ask(
    gateway: &Gateway,
    method: &'static str,
    path: String,
    actor: &str,
    body: Option<Vec<u8>>,
) -> Result<Answer, Refusal> {
    let gateway = gateway.clone();
    let actor = actor.to_owned();
    let answered = tokio::task::spawn_blocking(move || send(&gateway, method, &path, &actor, body))
        .await
        .unwrap_or_else(|_| Err("the request to the gateway failed".into()));
    let (status, bytes) = answered.map_err(|why| {
        tracing::warn!("admin console: the gateway did not answer: {why}");
        refuse(StatusCode::BAD_GATEWAY, "the gateway did not answer")
    })?;
    match status {
        401 | 403 => {
            tracing::error!(
                status,
                "admin console: the gateway refused this service's token \
                 (FEEDBACK_GATEWAY_ADMIN_TOKEN)"
            );
            return Err(refuse(
                StatusCode::BAD_GATEWAY,
                "the gateway refused this service's console token",
            ));
        }
        429 => {
            return Err(refuse(
                StatusCode::SERVICE_UNAVAILABLE,
                "the gateway's console is shut after too many wrong tokens; try again later",
            ));
        }
        200..=299 | 400 | 404 | 503 => {}
        _ => {
            tracing::warn!(status, "admin console: the gateway answered unexpectedly");
            return Err(refuse(
                StatusCode::BAD_GATEWAY,
                "the gateway answered unexpectedly",
            ));
        }
    }
    let json = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes)
            .map_err(|_| refuse(StatusCode::BAD_GATEWAY, "the gateway answered unexpectedly"))?
    };
    Ok(Answer { status, json })
}

/// One request to the gateway: its status and body. Blocking.
fn send(
    gateway: &Gateway,
    method: &str,
    path: &str,
    actor: &str,
    body: Option<Vec<u8>>,
) -> Result<(u16, Vec<u8>), String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .http_status_as_error(false)
        // The token goes to the gateway and nowhere else, whatever proxy
        // the environment names.
        .proxy(None)
        .max_redirects(0)
        .build()
        .into();
    let url = format!("{}{path}", gateway.url);
    let bearer = format!("Bearer {}", gateway.token);
    let response = match (method, body) {
        ("POST", Some(body)) => agent
            .post(&url)
            .header("Authorization", &bearer)
            .header("X-Baylee-Admin", actor)
            .header("Content-Type", "application/json")
            .send(&body[..]),
        ("DELETE", _) => agent
            .delete(&url)
            .header("Authorization", &bearer)
            .header("X-Baylee-Admin", actor)
            .call(),
        _ => agent
            .get(&url)
            .header("Authorization", &bearer)
            .header("X-Baylee-Admin", actor)
            .call(),
    };
    let mut response = response.map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    let bytes = response
        .body_mut()
        .with_config()
        .limit(MAX_ANSWER_BYTES)
        .read_to_vec()
        .map_err(|e| e.to_string())?;
    Ok((status, bytes))
}

/// The gateway's answer as this service's.
fn answer(answer: Answer) -> Response {
    let status = StatusCode::from_u16(answer.status).unwrap_or(StatusCode::BAD_GATEWAY);
    let mut response = if answer.json.is_null() {
        status.into_response()
    } else {
        (status, Json(answer.json)).into_response()
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn not_configured() -> Refusal {
    refuse(
        StatusCode::NOT_FOUND,
        "no gateway console is configured for this service",
    )
}

/// The signed-in admin's name and the gateway, or why not.
async fn signed_in(shared: &Shared, headers: &HeaderMap) -> Result<(String, Gateway), Refusal> {
    let admin = session(&shared.state, &shared.ui, headers).await?;
    let gateway = shared
        .state
        .config
        .gateway_admin()
        .cloned()
        .ok_or_else(not_configured)?;
    Ok((admin.name, gateway))
}

async fn stats(State(shared): State<Shared>, headers: HeaderMap) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    ask(&gateway, "GET", "/admin/stats".into(), &admin, None)
        .await
        .map(answer)
}

async fn invites(State(shared): State<Shared>, headers: HeaderMap) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    ask(&gateway, "GET", "/admin/invites".into(), &admin, None)
        .await
        .map(answer)
}

/// What `POST /ui/api/admin/invites` takes, and all it passes on.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Order {
    #[serde(skip_serializing_if = "Option::is_none")]
    uses: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u32>,
}

/// One line for the audit: what was made, never a key or the note.
fn made_detail(made: &serde_json::Value) -> String {
    let ids: Vec<&str> = made["keys"]
        .as_array()
        .map(|keys| keys.iter().filter_map(|k| k["id"].as_str()).collect())
        .unwrap_or_default();
    format!(
        "{} key(s), {} use(s) each, {}; ids {}",
        ids.len(),
        made["uses"].as_i64().unwrap_or(0),
        made["expires_at"]
            .as_str()
            .map_or_else(|| "never expiring".to_owned(), |at| format!("until {at}")),
        ids.join(", ")
    )
}

async fn create(
    State(shared): State<Shared>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, Refusal> {
    same_origin(&headers)?;
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let order: Order = serde_json::from_slice(&body)
        .map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a request for keys"))?;
    let body = serde_json::to_vec(&order)
        .map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a request for keys"))?;
    let made = ask(
        &gateway,
        "POST",
        "/admin/invites".into(),
        &admin,
        Some(body),
    )
    .await?;
    if made.status == 201 {
        let detail = made_detail(&made.json);
        tracing::info!(admin, action = "gateway.invite.create", %detail, "admin console");
        // The keys are answered whatever happens to the audit row: they
        // are stored at the gateway, and shown only now.
        if let Err(e) = note(&shared, &admin, "gateway.invite.create", &detail).await {
            tracing::error!("admin console: the audit row was not written: {e}");
        }
    }
    Ok(answer(made))
}

async fn revoke(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, Refusal> {
    same_origin(&headers)?;
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let id = Uuid::parse_str(&id).map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a key's id"))?;
    let done = ask(
        &gateway,
        "DELETE",
        format!("/admin/invites/{id}"),
        &admin,
        None,
    )
    .await?;
    if done.status == 204 {
        let detail = format!("key {id}");
        tracing::info!(admin, action = "gateway.invite.revoke", %detail, "admin console");
        if let Err(e) = note(&shared, &admin, "gateway.invite.revoke", &detail).await {
            tracing::error!("admin console: the audit row was not written: {e}");
        }
    }
    Ok(answer(done))
}

/// An audit row about no report: a change made through the console.
async fn note(
    shared: &Shared,
    actor: &str,
    action: &str,
    detail: &str,
) -> Result<(), sea_orm::DbErr> {
    let db = &shared.state.db;
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO feedback_audit (id, actor, report_id, action, detail) \
         VALUES ($1, $2, NULL, $3, $4)",
        [
            Uuid::now_v7().into(),
            actor.into(),
            action.into(),
            detail.into(),
        ],
    ))
    .await
    .map(|_| ())
}

#[derive(Serialize)]
struct Change {
    at: String,
    actor: String,
    action: String,
    detail: Option<String>,
}

async fn audit(
    State(shared): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<Vec<Change>>, Refusal> {
    signed_in(&shared, &headers).await?;
    let db = &shared.state.db;
    let rows = db
        .query_all_raw(Statement::from_string(
            db.get_database_backend(),
            format!(
                "SELECT to_char(at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS at, \
                     actor, action, detail FROM feedback_audit WHERE report_id IS NULL \
                 ORDER BY at DESC, id DESC LIMIT {AUDIT_ROWS}"
            ),
        ))
        .await
        .map_err(|e| db_down(&e))?;
    rows.iter()
        .map(|row| {
            Ok(Change {
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

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "a-gateway-admin-token-of-32-chars-at-least";

    #[test]
    fn a_console_is_a_plain_address_and_a_long_token() {
        let g = Gateway::new("http://127.0.0.1:28767/", TOKEN).unwrap();
        assert_eq!(g.url, "http://127.0.0.1:28767");
        assert!(Gateway::new("https://admin.example", TOKEN).is_ok());
        for url in [
            "127.0.0.1:28767",
            "ftp://127.0.0.1",
            "http://",
            "http://user:pw@127.0.0.1",
            "http://127.0.0.1/?x=1",
            "http://127.0.0.1/#x",
            "http://127.0.0.1 /",
        ] {
            assert!(Gateway::new(url, TOKEN).is_err(), "{url}");
        }
        assert!(Gateway::new("http://127.0.0.1", &"x".repeat(31)).is_err());
        assert!(Gateway::new("http://127.0.0.1", &format!("{TOKEN} x")).is_err());
    }

    #[test]
    fn the_audit_line_names_ids_and_never_a_key_or_a_note() {
        let made = serde_json::json!({
            "keys": [
                { "id": "0199aaaa-0000-7000-8000-000000000001", "key": "BAYLEE-AAAA-BBBB-CCCC-DDDD" },
                { "id": "0199aaaa-0000-7000-8000-000000000002", "key": "BAYLEE-EEEE-FFFF-GGGG-HHHH" },
            ],
            "uses": 3,
            "expires_at": "2026-11-07T09:00:00Z",
            "note": "for Max",
        });
        let line = made_detail(&made);
        assert_eq!(
            line,
            "2 key(s), 3 use(s) each, until 2026-11-07T09:00:00Z; ids \
             0199aaaa-0000-7000-8000-000000000001, 0199aaaa-0000-7000-8000-000000000002"
        );
        assert!(!line.contains("BAYLEE-") && !line.contains("Max"));
    }
}
