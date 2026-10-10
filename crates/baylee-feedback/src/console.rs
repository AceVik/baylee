//! The admin pages' road to a gateway's admin console (`docs/feedback.md`
//! §"The admin console"): `/ui/api/admin/…`, for a signed-in admin only,
//! passed on to the gateway's `/admin/…` routes with the gateway's
//! `BAYLEE_ADMIN_TOKEN`, which never leaves this service.
//!
//! - `GET /ui/api/admin/stats`: the gateway's numbers, as it sends them.
//! - `GET /ui/api/admin/live`: who is online and the tables open now.
//! - `GET /ui/api/admin/metrics`: the last hour of the server's samples.
//! - `GET /ui/api/admin/sets`, `GET /ui/api/admin/sets/{code}`: how far
//!   the pool is through each set, and one set's cards; a code is letters
//!   and digits, lower-cased here.
//! - `GET /ui/api/admin/accounts?q=&kind=&sort=&online=&offset=&limit=`:
//!   a page of accounts; the query is read into those six fields and
//!   written out again.
//! - `GET /ui/api/admin/accounts/{id}`: one account, its decks and games.
//! - `GET /ui/api/admin/invites`: its closed-beta keys, never a key.
//! - `POST /ui/api/admin/invites` `{uses?, expires?, note?, count?}`: makes
//!   keys and answers them, the one time they are shown.
//! - `DELETE /ui/api/admin/invites/{id}`: revokes one.
//! - `GET /ui/api/admin/audit`: the latest changes made through here.
//! - `GET /ui/api/admin/offline`: offline games going now, this service's
//!   own anonymous count (`crate::alive`); needs no gateway.
//! - `/ui/api/admin/llm/…`: the hosted models (seat agents, their profiles,
//!   a profile written, deleted, switched, probed, its key set write-only),
//!   passed to the gateway's `/admin/llm/…`.
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
use axum::routing::{delete, get, post, put};
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
        .route("/ui/api/admin/live", get(live))
        .route("/ui/api/admin/metrics", get(metrics))
        .route("/ui/api/admin/sets", get(sets))
        .route("/ui/api/admin/sets/{code}", get(one_set))
        .route("/ui/api/admin/accounts", get(accounts))
        .route("/ui/api/admin/accounts/{id}", get(account))
        .route("/ui/api/admin/invites", get(invites).post(create))
        .route("/ui/api/admin/invites/{id}", delete(revoke))
        .route("/ui/api/admin/audit", get(audit))
        .route("/ui/api/admin/offline", get(offline))
        .route("/ui/api/admin/llm/seathosts", get(llm_seathosts))
        .route("/ui/api/admin/llm/profiles", get(llm_profiles))
        .route(
            "/ui/api/admin/llm/seathosts/{host}/profiles/{id}",
            put(llm_write).delete(llm_delete),
        )
        .route(
            "/ui/api/admin/llm/seathosts/{host}/profiles/{id}/enabled",
            post(llm_enabled),
        )
        .route(
            "/ui/api/admin/llm/seathosts/{host}/profiles/{id}/probe",
            post(llm_probe),
        )
        .route(
            "/ui/api/admin/llm/seathosts/{host}/profiles/{id}/key",
            post(llm_key),
        )
}

/// `GET /ui/api/admin/offline`: how many offline games are going now
/// (`crate::alive`). This service's own number, so it wants the session
/// alone, not a gateway's console.
async fn offline(State(shared): State<Shared>, headers: HeaderMap) -> Result<Response, Refusal> {
    session(&shared.state, &shared.ui, &headers).await?;
    Ok(Json(serde_json::json!({
        "offline_now": shared.ui.offline_now(),
        "window_secs": crate::alive::TTL.whole_seconds(),
    }))
    .into_response())
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
        200..=299 | 400 | 404 | 409 | 503 | 504 => {}
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
        ("PUT", Some(body)) => agent
            .put(&url)
            .header("Authorization", &bearer)
            .header("X-Baylee-Admin", actor)
            .header("Content-Type", "application/json")
            .send(&body[..]),
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

async fn live(State(shared): State<Shared>, headers: HeaderMap) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    ask(&gateway, "GET", "/admin/live".into(), &admin, None)
        .await
        .map(answer)
}

async fn metrics(State(shared): State<Shared>, headers: HeaderMap) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    ask(&gateway, "GET", "/admin/metrics".into(), &admin, None)
        .await
        .map(answer)
}

async fn sets(State(shared): State<Shared>, headers: HeaderMap) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    ask(&gateway, "GET", "/admin/sets".into(), &admin, None)
        .await
        .map(answer)
}

/// A set code as Scryfall writes them: letters and digits, a few of them.
fn is_set_code(code: &str) -> bool {
    (1..=8).contains(&code.len()) && code.bytes().all(|b| b.is_ascii_alphanumeric())
}

async fn one_set(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(code): Path<String>,
) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    if !is_set_code(&code) {
        return Err(refuse(StatusCode::BAD_REQUEST, "not a set code"));
    }
    ask(
        &gateway,
        "GET",
        format!("/admin/sets/{}", code.to_ascii_lowercase()),
        &admin,
        None,
    )
    .await
    .map(answer)
}

/// What `GET /ui/api/admin/accounts` takes, and all it passes on.
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct AccountsQuery {
    q: Option<String>,
    kind: Option<String>,
    sort: Option<String>,
    online: Option<bool>,
    offset: Option<u32>,
    limit: Option<u32>,
}

/// `text` with every byte but the unreserved ones percent-encoded.
fn encoded(text: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

impl AccountsQuery {
    /// The gateway's query string, built from the fields alone.
    fn path(&self) -> String {
        let mut pairs: Vec<String> = Vec::new();
        for (key, value) in [("q", &self.q), ("kind", &self.kind), ("sort", &self.sort)] {
            if let Some(value) = value.as_deref().filter(|v| !v.is_empty()) {
                pairs.push(format!("{key}={}", encoded(value)));
            }
        }
        if let Some(online) = self.online {
            pairs.push(format!("online={online}"));
        }
        if let Some(offset) = self.offset {
            pairs.push(format!("offset={offset}"));
        }
        if let Some(limit) = self.limit {
            pairs.push(format!("limit={limit}"));
        }
        if pairs.is_empty() {
            "/admin/accounts".into()
        } else {
            format!("/admin/accounts?{}", pairs.join("&"))
        }
    }
}

async fn accounts(
    State(shared): State<Shared>,
    headers: HeaderMap,
    query: Result<axum::extract::Query<AccountsQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let axum::extract::Query(query) =
        query.map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a search for accounts"))?;
    ask(&gateway, "GET", query.path(), &admin, None)
        .await
        .map(answer)
}

async fn account(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let id =
        Uuid::parse_str(&id).map_err(|_| refuse(StatusCode::BAD_REQUEST, "not an account's id"))?;
    ask(
        &gateway,
        "GET",
        format!("/admin/accounts/{id}"),
        &admin,
        None,
    )
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

// ------------------------------------------------------------ hosted models

/// Whether `name` may be a seat agent's name or a profile's id, as the
/// gateway takes them: 1-64 of `A-Za-z0-9._-`.
fn llm_name(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

/// `{host}` and `{id}`, checked.
fn llm_path(host: &str, id: &str) -> Result<String, Refusal> {
    if llm_name(host) && llm_name(id) {
        Ok(format!("/admin/llm/seathosts/{host}/profiles/{id}"))
    } else {
        Err(refuse(
            StatusCode::BAD_REQUEST,
            "a seat agent's name and a profile's id are 1-64 of A-Z a-z 0-9 . - _",
        ))
    }
}

async fn llm_seathosts(
    State(shared): State<Shared>,
    headers: HeaderMap,
) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    ask(&gateway, "GET", "/admin/llm/seathosts".into(), &admin, None)
        .await
        .map(answer)
}

async fn llm_profiles(
    State(shared): State<Shared>,
    headers: HeaderMap,
) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    ask(&gateway, "GET", "/admin/llm/profiles".into(), &admin, None)
        .await
        .map(answer)
}

/// A hosted profile's definition, all the gateway takes: read into these
/// fields and written out again (`docs/protocol.md` §"Hosted
/// language-model seats").
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LlmDefinition {
    label: String,
    vendor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_games: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    caps: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    canary: Option<bool>,
    profile: serde_json::Value,
}

/// A change that succeeded, noted: the action and `<host>/<id>`, nothing
/// else (never a key or a definition).
async fn llm_noted(shared: &Shared, admin: &str, action: &str, host: &str, id: &str) {
    let detail = format!("{host}/{id}");
    tracing::info!(admin, action, %detail, "admin console");
    if let Err(e) = note(shared, admin, action, &detail).await {
        tracing::error!("admin console: the audit row was not written: {e}");
    }
}

async fn llm_write(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path((host, id)): Path<(String, String)>,
    body: Bytes,
) -> Result<Response, Refusal> {
    same_origin(&headers)?;
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let path = llm_path(&host, &id)?;
    let definition: LlmDefinition = serde_json::from_slice(&body)
        .map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a profile's definition"))?;
    let body = serde_json::to_vec(&definition)
        .map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a profile's definition"))?;
    let done = ask(&gateway, "PUT", path, &admin, Some(body)).await?;
    if done.status == 200 {
        llm_noted(&shared, &admin, "gateway.llm.profile.write", &host, &id).await;
    }
    Ok(answer(done))
}

async fn llm_delete(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path((host, id)): Path<(String, String)>,
) -> Result<Response, Refusal> {
    same_origin(&headers)?;
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let path = llm_path(&host, &id)?;
    let done = ask(&gateway, "DELETE", path, &admin, None).await?;
    if done.status == 204 {
        llm_noted(&shared, &admin, "gateway.llm.profile.delete", &host, &id).await;
    }
    Ok(answer(done))
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LlmEnabled {
    enabled: bool,
}

async fn llm_enabled(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path((host, id)): Path<(String, String)>,
    body: Bytes,
) -> Result<Response, Refusal> {
    same_origin(&headers)?;
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let path = llm_path(&host, &id)?;
    let switch: LlmEnabled = serde_json::from_slice(&body)
        .map_err(|_| refuse(StatusCode::BAD_REQUEST, "send {\"enabled\": true|false}"))?;
    let body = serde_json::to_vec(&switch)
        .map_err(|_| refuse(StatusCode::BAD_REQUEST, "send {\"enabled\": true|false}"))?;
    let done = ask(
        &gateway,
        "POST",
        format!("{path}/enabled"),
        &admin,
        Some(body),
    )
    .await?;
    if done.status == 204 {
        let action = if switch.enabled {
            "gateway.llm.profile.enable"
        } else {
            "gateway.llm.profile.disable"
        };
        llm_noted(&shared, &admin, action, &host, &id).await;
    }
    Ok(answer(done))
}

async fn llm_probe(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path((host, id)): Path<(String, String)>,
) -> Result<Response, Refusal> {
    same_origin(&headers)?;
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let path = llm_path(&host, &id)?;
    let done = ask(
        &gateway,
        "POST",
        format!("{path}/probe"),
        &admin,
        Some(b"{}".to_vec()),
    )
    .await?;
    if done.status == 202 {
        llm_noted(&shared, &admin, "gateway.llm.profile.probe", &host, &id).await;
    }
    Ok(answer(done))
}

/// A key, or `null` to forget it. No `Debug`: nothing may print it; held
/// for this request only.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LlmKey {
    key: Option<String>,
}

async fn llm_key(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path((host, id)): Path<(String, String)>,
    body: Bytes,
) -> Result<Response, Refusal> {
    same_origin(&headers)?;
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let path = llm_path(&host, &id)?;
    let key: LlmKey = serde_json::from_slice(&body).map_err(|_| {
        refuse(
            StatusCode::BAD_REQUEST,
            "send {\"key\": \"…\"} or {\"key\": null}",
        )
    })?;
    let set = key.key.is_some();
    let body =
        serde_json::to_vec(&key).map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a key"))?;
    drop(key);
    let done = ask(&gateway, "POST", format!("{path}/key"), &admin, Some(body)).await?;
    if done.status == 204 {
        let action = if set {
            "gateway.llm.key.set"
        } else {
            "gateway.llm.key.delete"
        };
        llm_noted(&shared, &admin, action, &host, &id).await;
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
    fn an_account_search_passes_on_its_six_fields_and_nothing_else() {
        assert_eq!(AccountsQuery::default().path(), "/admin/accounts");
        let query = AccountsQuery {
            q: Some("Al ice#af&x=1".into()),
            kind: Some("guest".into()),
            sort: Some(String::new()),
            online: Some(true),
            offset: Some(50),
            limit: Some(25),
        };
        assert_eq!(
            query.path(),
            "/admin/accounts?q=Al%20ice%23af%26x%3D1&kind=guest&online=true&offset=50&limit=25"
        );
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
