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
//! - `GET /ui/api/admin/accounts?q=&tag=&kind=&sort=&online=&offset=&limit=`:
//!   a page of accounts; the query is read into those seven fields and
//!   written out again. `tag` is one exact tag, the sender a chat report
//!   names (`client.chat.sender_tag`), so an admin can act on the account
//!   that wrote it.
//! - `GET /ui/api/admin/accounts/{id}`: one account, its decks and games.
//! - `GET /ui/api/admin/invites`: its closed-beta keys, never a key.
//! - `POST /ui/api/admin/invites` `{uses?, expires?, note?, count?}`: makes
//!   keys and answers them, the one time they are shown.
//! - `DELETE /ui/api/admin/invites/{id}`: revokes one.
//! - `GET /ui/api/admin/cards/search?q=`: the pool's cards by name, each
//!   saying whether it is blocked.
//! - `GET /ui/api/admin/cards/blocks`: every card block, newest first.
//! - `POST /ui/api/admin/cards/blocks` `{index, reason, until?}`: blocks a
//!   card on the gateway, for good or until a moment.
//! - `DELETE /ui/api/admin/cards/blocks/{id}`: lifts a block.
//! - `GET /ui/api/admin/audit`: the latest changes made through here.
//! - `GET /ui/api/admin/auth-mode`, `PUT` the same `{mode}`: who the
//!   gateway lets in (guests, Central's accounts, or both), and choosing it.
//! - `GET /ui/api/admin/offline`: offline games going now, this service's
//!   own anonymous count (`crate::alive`); needs no gateway.
//! - `/ui/api/admin/llm/…`: the hosted models (seat agents, their profiles,
//!   a profile written, deleted, switched, probed, its key set write-only),
//!   passed to the gateway's `/admin/llm/…`.
//! - `/ui/api/admin/llm/access…`: who may seat which hosted model (a
//!   profile's policy and its list of accounts), and
//!   `GET /ui/api/admin/accounts/lookup?q=`, the list editor's type-ahead
//!   over registered accounts (a prefix of two characters or more, at most
//!   ten); passed to the gateway's routes of the same names.
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
        .route(
            "/ui/api/admin/accounts/{id}",
            get(account).delete(crate::moderation::delete_account),
        )
        .route("/ui/api/admin/invites", get(invites).post(create))
        .route("/ui/api/admin/central", get(central))
        .route(
            "/ui/api/admin/central/token",
            put(central_paste).delete(central_remove),
        )
        .route("/ui/api/admin/invites/{id}", delete(revoke))
        .route("/ui/api/admin/cards/search", get(search_cards))
        .route(
            "/ui/api/admin/cards/blocks",
            get(card_blocks).post(block_card),
        )
        .route("/ui/api/admin/cards/blocks/{id}", delete(unblock_card))
        .route("/ui/api/admin/audit", get(audit))
        .route(
            "/ui/api/admin/auth-mode",
            get(auth_mode).put(choose_auth_mode),
        )
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
        .route("/ui/api/admin/accounts/lookup", get(lookup))
        .route("/ui/api/admin/llm/access", get(access_list))
        .route(
            "/ui/api/admin/llm/access/{id}",
            get(access_one).put(access_policy),
        )
        .route("/ui/api/admin/llm/access/{id}/allow", post(access_allow))
        .route(
            "/ui/api/admin/llm/access/{id}/disallow",
            post(access_disallow),
        )
}

/// `GET /ui/api/admin/offline`'s answer.
/// @openapi
#[derive(Serialize)]
struct Offline {
    /// @description Offline games heard from within the window.
    offline_now: usize,
    /// @description How long a game counts after its last beat, in seconds:
    /// 150.
    window_secs: i64,
}

/// `GET /ui/api/admin/offline`: how many offline games are going now
/// (`crate::alive`). This service's own number, so it wants the session
/// alone, not a gateway's console.
///
/// @route GET /ui/api/admin/offline
/// @scope feedback
/// @summary Offline games going now
/// @description How many games clients host themselves are going now, by
/// their anonymous beats (`POST /client/alive`). This service's own count:
/// it needs no gateway console.
/// @tag Console
/// @security session
/// @return 200: `$Offline`
/// @insert `SessionRefusals`
async fn offline(State(shared): State<Shared>, headers: HeaderMap) -> Result<Response, Refusal> {
    session(&shared.state, &shared.ui, &headers).await?;
    Ok(Json(Offline {
        offline_now: shared.ui.offline_now(),
        window_secs: crate::alive::TTL.whole_seconds(),
    })
    .into_response())
}

/// What the gateway answered: its status and its JSON (`null` for none).
pub(crate) struct Answer {
    pub(crate) status: u16,
    pub(crate) json: serde_json::Value,
}

/// Asks the gateway, off the runtime's threads.
pub(crate) async fn ask(
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
pub(crate) fn answer(answer: Answer) -> Response {
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
pub(crate) async fn signed_in(
    shared: &Shared,
    headers: &HeaderMap,
) -> Result<(String, Gateway), Refusal> {
    let admin = session(&shared.state, &shared.ui, headers).await?;
    let gateway = shared
        .state
        .config
        .gateway_admin()
        .cloned()
        .ok_or_else(not_configured)?;
    Ok((admin.name, gateway))
}

/// @route GET /ui/api/admin/stats
/// @scope feedback
/// @summary The gateway's numbers
/// @description The gateway's `GET /admin/stats`, passed on with its status: its counts.
/// @tag Console
/// @operation-id `consoleStats`
/// @security session
/// @insert `ConsoleRefusals`
/// @return 200: {} "The gateway's `GET /admin/stats` answer, as it is"
async fn stats(State(shared): State<Shared>, headers: HeaderMap) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    ask(&gateway, "GET", "/admin/stats".into(), &admin, None)
        .await
        .map(answer)
}

/// @route GET /ui/api/admin/live
/// @scope feedback
/// @summary Who is online and the tables open now
/// @description The gateway's `GET /admin/live`, passed on with its status.
/// @tag Console
/// @security session
/// @insert `ConsoleRefusals`
/// @return 200: {} "The gateway's `GET /admin/live` answer, as it is"
async fn live(State(shared): State<Shared>, headers: HeaderMap) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    ask(&gateway, "GET", "/admin/live".into(), &admin, None)
        .await
        .map(answer)
}

/// @route GET /ui/api/admin/metrics
/// @scope feedback
/// @summary The server's last hour
/// @description The gateway's `GET /admin/metrics`, passed on with its status: the last hour of the server's samples.
/// @tag Console
/// @security session
/// @insert `ConsoleRefusals`
/// @return 200: {} "The gateway's `GET /admin/metrics` answer, as it is"
async fn metrics(State(shared): State<Shared>, headers: HeaderMap) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    ask(&gateway, "GET", "/admin/metrics".into(), &admin, None)
        .await
        .map(answer)
}

/// @route GET /ui/api/admin/sets
/// @scope feedback
/// @summary How far the pool is through each set
/// @description The gateway's `GET /admin/sets`, passed on with its status.
/// @tag Console
/// @security session
/// @insert `ConsoleRefusals`
/// @return 200: {} "The gateway's `GET /admin/sets` answer, as it is"
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

/// @route GET /ui/api/admin/sets/{code}
/// @scope feedback
/// @summary One set's cards
/// @description The gateway's `GET /admin/sets/{code}`, the code lower-cased, passed on with its status.
/// @tag Console
/// @security session
/// @insert `ConsoleRefusals`
/// @return 200: {} "The gateway's `GET /admin/sets/{code}` answer, as it is"
/// @path-param code: String "A set's code: one to eight letters and digits"
/// @return 400: `$ErrorBody` "Not a set code"
/// @return 404: `$ErrorBody` "No gateway console is configured, or the gateway knows no such set"
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
///
/// @description The search for accounts, passed on to the gateway field by
/// field; any other parameter is `400`.
/// @openapi
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct AccountsQuery {
    /// @description What to search for in names.
    q: Option<String>,
    /// @description One exact tag: a reported chat message's sender
    /// (`client.chat.sender_tag`).
    /// @internal
    /// One exact tag: a reported chat message's sender (`sender_tag`).
    tag: Option<String>,
    /// @description `all`, `registered` or `guest`.
    kind: Option<String>,
    /// @description The order, as the gateway takes it.
    sort: Option<String>,
    /// @description Only accounts online now (`true`).
    online: Option<bool>,
    /// @description How many accounts to skip.
    offset: Option<u32>,
    /// @description How many accounts a page holds.
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
        for (key, value) in [
            ("q", &self.q),
            ("tag", &self.tag),
            ("kind", &self.kind),
            ("sort", &self.sort),
        ] {
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

/// @route GET /ui/api/admin/accounts
/// @scope feedback
/// @summary Search the gateway's accounts
/// @description The gateway's `GET /admin/accounts`, passed on with its status. The query is read into its seven fields and written out again.
/// @tag Console
/// @security session
/// @insert `ConsoleRefusals`
/// @return 200: {} "The gateway's `GET /admin/accounts` answer, as it is"
/// @query-params `$AccountsQuery`
/// @return 400: `$ErrorBody` "Not a search for accounts: a parameter it does not take or a value of the wrong type, or the gateway's own refusal"
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

/// @route GET /ui/api/admin/accounts/{id}
/// @scope feedback
/// @summary One account
/// @description The gateway's `GET /admin/accounts/{id}`, passed on with its status: one account, its decks and latest games.
/// @tag Console
/// @security session
/// @insert `ConsoleRefusals`
/// @return 200: {} "The gateway's `GET /admin/accounts/{id}` answer, as it is"
/// @path-param id: String format=uuid "The account's id"
/// @return 400: `$ErrorBody` "Not an account's id"
/// @return 404: `$ErrorBody` "No gateway console is configured, or the gateway has no such account"
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

/// @route GET /ui/api/admin/invites
/// @scope feedback
/// @summary The gateway's beta keys
/// @description The gateway's `GET /admin/invites`, passed on with its status: its keys, newest first, never a key itself.
/// @tag Console
/// @security session
/// @insert `ConsoleRefusals`
/// @return 200: {} "The gateway's `GET /admin/invites` answer, as it is"
async fn invites(State(shared): State<Shared>, headers: HeaderMap) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    ask(&gateway, "GET", "/admin/invites".into(), &admin, None)
        .await
        .map(answer)
}

/// What `POST /ui/api/admin/invites` takes, and all it passes on.
///
/// @description Keys to make, as `invite create` takes them.
/// @openapi
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Order {
    /// @description How often each key may be used.
    #[serde(skip_serializing_if = "Option::is_none")]
    uses: Option<u32>,
    /// @description When the keys expire, from now: `30d`, `12h`, ….
    #[serde(skip_serializing_if = "Option::is_none")]
    expires: Option<String>,
    /// @description A note kept with the keys at the gateway.
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    /// @description How many keys to make.
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u32>,
}

/// @route GET /ui/api/admin/central
/// @scope feedback
/// @summary The gateway's link to Baylee Central
/// @description The gateway's `GET /admin/central`, passed on with its status: where its link to Baylee Central comes from and, for a pasted token, what Central says about it; never the token, only its last four characters.
/// @tag Console
/// @security session
/// @insert `ConsoleRefusals`
/// @return 200: {} "The gateway's `GET /admin/central` answer, as it is"
async fn central(State(shared): State<Shared>, headers: HeaderMap) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    ask(&gateway, "GET", "/admin/central".into(), &admin, None)
        .await
        .map(answer)
}

/// What `PUT /ui/api/admin/central/token` takes, and all it passes on.
///
/// @description A gateway token, as the website showed it.
/// @openapi
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CentralToken {
    /// @description `bgt_` and 64 lowercase hex digits.
    token: String,
}

/// @route PUT /ui/api/admin/central/token
/// @scope feedback
/// @summary Link the gateway to Baylee Central with a token
/// @description The gateway's `PUT /admin/central/token`, passed on with its status: the gateway asks Baylee Central about the token, keeps it and uses it at once. This service writes an audit row without the token.
/// @tag Console
/// @operation-id `linkCentral`
/// @security session + csrf
/// @insert `ConsoleChangeRefusals`
/// @body `$CentralToken`
/// @return 200: {} "Linked, as the gateway's `GET /admin/central` answers"
/// @return 400: `$ErrorBody` "Not a token, or the gateway's own refusal"
/// @return 409: `$ErrorBody` "The gateway's operator file sets the link"
/// @return 422: `$ErrorBody` "Baylee Central does not know the token, or it was revoked or has expired"
async fn central_paste(
    State(shared): State<Shared>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, Refusal> {
    same_origin(&headers)?;
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let token: CentralToken = serde_json::from_slice(&body)
        .map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a gateway token"))?;
    let body = serde_json::to_vec(&token)
        .map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a gateway token"))?;
    drop(token);
    let done = ask(
        &gateway,
        "PUT",
        "/admin/central/token".into(),
        &admin,
        Some(body),
    )
    .await?;
    if done.status == 200 {
        let detail = format!(
            "gateway {}",
            done.json["central"]["gateway"]
                .as_str()
                .unwrap_or("unknown")
        );
        tracing::info!(admin, action = "gateway.central.link", %detail, "admin console");
        if let Err(e) = note(&shared, &admin, "gateway.central.link", &detail).await {
            tracing::error!("admin console: the audit row was not written: {e}");
        }
    }
    Ok(answer(done))
}

/// @route DELETE /ui/api/admin/central/token
/// @scope feedback
/// @summary Remove the gateway's pasted token
/// @description The gateway's `DELETE /admin/central/token`, passed on with its status: the gateway forgets the token and is unlinked from Baylee Central at once.
/// @tag Console
/// @operation-id `unlinkCentral`
/// @security session + csrf
/// @insert `ConsoleChangeRefusals`
/// @return 204: "Unlinked"
/// @return 404: `$ErrorBody` "No token is pasted"
/// @return 409: `$ErrorBody` "The gateway's operator file sets the link"
async fn central_remove(
    State(shared): State<Shared>,
    headers: HeaderMap,
) -> Result<Response, Refusal> {
    same_origin(&headers)?;
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let done = ask(
        &gateway,
        "DELETE",
        "/admin/central/token".into(),
        &admin,
        None,
    )
    .await?;
    if done.status == 204 {
        tracing::info!(admin, action = "gateway.central.unlink", "admin console");
        if let Err(e) = note(&shared, &admin, "gateway.central.unlink", "").await {
            tracing::error!("admin console: the audit row was not written: {e}");
        }
    }
    Ok(answer(done))
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

/// @route POST /ui/api/admin/invites
/// @scope feedback
/// @summary Make beta keys
/// @description The gateway's `POST /admin/invites`, passed on with its status. Its `201` carries the keys, the only time they are shown; this service writes an audit row with their ids, uses and expiry, never a key or the note.
/// @tag Console
/// @operation-id `createInvites`
/// @security session + csrf
/// @insert `ConsoleChangeRefusals`
/// @return 201: {} "Made, as the gateway answers: `keys` (each an `id` and its `key`), `uses`, `expires_at` and `note`"
/// @body `$Order`
/// @return 400: `$ErrorBody` "Not a request for keys, or the gateway's own refusal"
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

/// @route DELETE /ui/api/admin/invites/{id}
/// @scope feedback
/// @summary Revoke a beta key
/// @description The gateway's `DELETE /admin/invites/{id}`, passed on with its status; a key revoked is an audit row.
/// @tag Console
/// @operation-id `revokeInvite`
/// @security session + csrf
/// @insert `ConsoleChangeRefusals`
/// @return 204: "Revoked"
/// @path-param id: String format=uuid "The key's id"
/// @return 400: `$ErrorBody` "Not a key's id"
/// @return 404: `$ErrorBody` "No gateway console is configured, or the gateway has no such key, or it is revoked already"
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

// ------------------------------------------------------------ who comes in

/// @route GET /ui/api/admin/auth-mode
/// @scope feedback
/// @summary Who the gateway lets in
/// @description The gateway's `GET /admin/auth-mode`, passed on with its status: the mode in force (`guests`, `central` or `mixed`), where it comes from, and whether the gateway is connected to Baylee Central.
/// @tag Console
/// @security session
/// @insert `ConsoleRefusals`
/// @return 200: {} "The gateway's `GET /admin/auth-mode` answer, as it is"
async fn auth_mode(State(shared): State<Shared>, headers: HeaderMap) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    ask(&gateway, "GET", "/admin/auth-mode".into(), &admin, None)
        .await
        .map(answer)
}

/// What `PUT /ui/api/admin/auth-mode` takes, and all it passes on.
///
/// @description The mode to put in force.
/// @openapi
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AuthModeChoice {
    /// @description `guests`, `central` or `mixed`.
    /// @openapi enum: [guests, central, mixed]
    mode: String,
}

/// @route PUT /ui/api/admin/auth-mode
/// @scope feedback
/// @summary Choose who the gateway lets in
/// @description The gateway's `PUT /admin/auth-mode`, passed on with its status: the mode is in force for every sign-in from now on, and a session already made goes on. This service writes an audit row naming the mode.
/// @tag Console
/// @operation-id `chooseAuthMode`
/// @security session + csrf
/// @insert `ConsoleChangeRefusals`
/// @body `$AuthModeChoice`
/// @return 200: {} "In force, as the gateway's `GET /admin/auth-mode` answers"
/// @return 400: `$ErrorBody` "Not a mode, or the gateway's own refusal"
/// @return 409: `$ErrorBody` "The gateway's operator sets the mode (`BAYLEE_AUTH_MODE`)"
async fn choose_auth_mode(
    State(shared): State<Shared>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, Refusal> {
    same_origin(&headers)?;
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let choice: AuthModeChoice = serde_json::from_slice(&body)
        .ok()
        .filter(|choice: &AuthModeChoice| {
            matches!(choice.mode.as_str(), "guests" | "central" | "mixed")
        })
        .ok_or_else(|| refuse(StatusCode::BAD_REQUEST, "not a mode"))?;
    let body =
        serde_json::to_vec(&choice).map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a mode"))?;
    let done = ask(
        &gateway,
        "PUT",
        "/admin/auth-mode".into(),
        &admin,
        Some(body),
    )
    .await?;
    if done.status == 200 {
        let detail = format!("mode {}", choice.mode);
        tracing::info!(admin, action = "gateway.auth_mode.set", %detail, "admin console");
        if let Err(e) = note(&shared, &admin, "gateway.auth_mode.set", &detail).await {
            tracing::error!("admin console: the audit row was not written: {e}");
        }
    }
    Ok(answer(done))
}

// ------------------------------------------------------------ blocked cards

/// What `GET /ui/api/admin/cards/search` takes, and all it passes on.
///
/// @description The search for cards; any other parameter is `400`.
/// @openapi
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct CardSearch {
    /// @description A piece of a card's name.
    q: String,
}

/// @route GET /ui/api/admin/cards/search
/// @scope feedback
/// @summary Find the pool's cards by name
/// @description The gateway's `GET /admin/cards/search`, passed on with its status: each card says whether it is blocked.
/// @tag Console
/// @operation-id `searchCards`
/// @security session
/// @insert `ConsoleRefusals`
/// @return 200: {} "The gateway's `GET /admin/cards/search` answer, as it is"
/// @query-params `$CardSearch`
/// @return 400: `$ErrorBody` "Not a search for cards, or the gateway's own refusal"
async fn search_cards(
    State(shared): State<Shared>,
    headers: HeaderMap,
    query: Result<axum::extract::Query<CardSearch>, axum::extract::rejection::QueryRejection>,
) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let axum::extract::Query(query) =
        query.map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a search for cards"))?;
    ask(
        &gateway,
        "GET",
        format!("/admin/cards/search?q={}", encoded(&query.q)),
        &admin,
        None,
    )
    .await
    .map(answer)
}

/// @route GET /ui/api/admin/cards/blocks
/// @scope feedback
/// @summary Every card block
/// @description The gateway's `GET /admin/cards/blocks`, passed on with its status: every block, newest first.
/// @tag Console
/// @security session
/// @insert `ConsoleRefusals`
/// @return 200: {} "The gateway's `GET /admin/cards/blocks` answer, as it is"
async fn card_blocks(
    State(shared): State<Shared>,
    headers: HeaderMap,
) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    ask(&gateway, "GET", "/admin/cards/blocks".into(), &admin, None)
        .await
        .map(answer)
}

/// What `POST /ui/api/admin/cards/blocks` takes, and all it passes on.
///
/// @description A card to block on the gateway.
/// @openapi
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BlockOrder {
    /// @description The card, by its index in the gateway's pool.
    index: u32,
    /// @description Why, as the players are told it.
    reason: String,
    /// @description When the block ends, RFC 3339; absent for no end.
    /// @openapi format: date-time
    #[serde(default, skip_serializing_if = "Option::is_none")]
    until: Option<String>,
}

/// One line for the audit: which card, which block, until when; never
/// the reason, which may name a person.
fn block_detail(made: &serde_json::Value) -> String {
    format!(
        "{} (#{}), {}; block {}",
        made["name"].as_str().unwrap_or("?"),
        made["index"].as_u64().unwrap_or(0),
        made["until"]
            .as_str()
            .map_or_else(|| "without an end".to_owned(), |at| format!("until {at}")),
        made["id"].as_str().unwrap_or("?"),
    )
}

/// @route POST /ui/api/admin/cards/blocks
/// @scope feedback
/// @summary Block a card
/// @description The gateway's `POST /admin/cards/blocks`, passed on with its status; a block made is an audit row with the card, its end and the block's id, never the reason.
/// @tag Console
/// @operation-id `blockCard`
/// @security session + csrf
/// @insert `ConsoleChangeRefusals`
/// @return 201: {} "Blocked: the block, as the gateway answers it"
/// @body `$BlockOrder`
/// @return 400: `$ErrorBody` "Not a request to block a card, or the gateway's own refusal"
/// @return 404: `$ErrorBody` "No gateway console is configured, or no card of the gateway's pool has that index"
/// @return 409: `$ErrorBody` "The card is blocked already (the gateway's)"
async fn block_card(
    State(shared): State<Shared>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, Refusal> {
    same_origin(&headers)?;
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let order: BlockOrder = serde_json::from_slice(&body)
        .map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a request to block a card"))?;
    let body = serde_json::to_vec(&order)
        .map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a request to block a card"))?;
    let made = ask(
        &gateway,
        "POST",
        "/admin/cards/blocks".into(),
        &admin,
        Some(body),
    )
    .await?;
    if made.status == 201 {
        let detail = block_detail(&made.json);
        tracing::info!(admin, action = "gateway.card.block", %detail, "admin console");
        if let Err(e) = note(&shared, &admin, "gateway.card.block", &detail).await {
            tracing::error!("admin console: the audit row was not written: {e}");
        }
    }
    Ok(answer(made))
}

/// @route DELETE /ui/api/admin/cards/blocks/{id}
/// @scope feedback
/// @summary Lift a card block
/// @description The gateway's `DELETE /admin/cards/blocks/{id}`, passed on with its status; a block lifted is an audit row.
/// @tag Console
/// @operation-id `unblockCard`
/// @security session + csrf
/// @insert `ConsoleChangeRefusals`
/// @return 204: "Lifted"
/// @path-param id: String format=uuid "The block's id"
/// @return 400: `$ErrorBody` "Not a block's id"
/// @return 404: `$ErrorBody` "No gateway console is configured, or the gateway has no such block still in force"
async fn unblock_card(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, Refusal> {
    same_origin(&headers)?;
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let id =
        Uuid::parse_str(&id).map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a block's id"))?;
    let done = ask(
        &gateway,
        "DELETE",
        format!("/admin/cards/blocks/{id}"),
        &admin,
        None,
    )
    .await?;
    if done.status == 204 {
        let detail = format!("block {id}");
        tracing::info!(admin, action = "gateway.card.unblock", %detail, "admin console");
        if let Err(e) = note(&shared, &admin, "gateway.card.unblock", &detail).await {
            tracing::error!("admin console: the audit row was not written: {e}");
        }
    }
    Ok(answer(done))
}

// ------------------------------------------------------------ hosted models

/// Whether `name` may be a seat agent's name or a profile's id, as the
/// gateway takes them: 1-64 of `A-Za-z0-9._-`.
///
/// The rule of `baylee_protocol::seathost::valid_name`, which this service
/// does not link: dots alone (`.`, `..`) are refused, since a path made of
/// them is normalised to another route.
fn llm_name(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
        && !name.bytes().all(|b| b == b'.')
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

/// @route GET /ui/api/admin/llm/seathosts
/// @scope feedback
/// @summary The seat agents
/// @description The gateway's `GET /admin/llm/seathosts`, passed on with its status: the seat agents connected and the profiles each runs.
/// @tag Models
/// @security session
/// @insert `ConsoleRefusals`
/// @return 200: {} "The gateway's `GET /admin/llm/seathosts` answer, as it is"
async fn llm_seathosts(
    State(shared): State<Shared>,
    headers: HeaderMap,
) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    ask(&gateway, "GET", "/admin/llm/seathosts".into(), &admin, None)
        .await
        .map(answer)
}

/// @route GET /ui/api/admin/llm/profiles
/// @scope feedback
/// @summary The hosted profiles
/// @description The gateway's `GET /admin/llm/profiles`, passed on with its status.
/// @tag Models
/// @security session
/// @insert `ConsoleRefusals`
/// @return 200: {} "The gateway's `GET /admin/llm/profiles` answer, as it is"
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
///
/// @description A hosted profile's definition, as the gateway takes it
/// (its document's §"Hosted language-model seats").
/// @openapi
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LlmDefinition {
    /// @description The name the profile is shown under.
    label: String,
    /// @description Who makes the model.
    vendor: String,
    /// @description Whether new games may seat it.
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    /// @description How many games may seat it at once.
    #[serde(skip_serializing_if = "Option::is_none")]
    max_games: Option<u32>,
    /// @description The caps of the profile's spend book.
    #[serde(skip_serializing_if = "Option::is_none")]
    caps: Option<serde_json::Value>,
    /// @description Whether it is a canary.
    #[serde(skip_serializing_if = "Option::is_none")]
    canary: Option<bool>,
    /// @description Another profile of the same seat agent, by id (1–64 of
    /// `A-Za-z0-9._-`), that new games start on past `max_games`; the seat
    /// agent says whether it may be one.
    /// @internal
    /// Another profile of the same seat agent, by id (`llm_name`): the seat
    /// agent says whether it may be one.
    #[serde(skip_serializing_if = "Option::is_none")]
    fallback: Option<String>,
    /// @description The rest of the profile, in the seat settings file's
    /// format.
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

/// @route PUT /ui/api/admin/llm/seathosts/{host}/profiles/{id}
/// @scope feedback
/// @summary Write a hosted profile
/// @description The gateway's `PUT /admin/llm/seathosts/{host}/profiles/{id}`, passed on with its status: the definition goes to that seat agent. A profile written is an audit row naming `<host>/<id>`, never the definition.
/// @tag Models
/// @operation-id `writeLlmProfile`
/// @security session + csrf
/// @insert `ConsoleChangeRefusals`
/// @return 200: {} "Written, as the gateway answers"
/// @path-param host: String "The seat agent's name: 1–64 of `A-Za-z0-9._-`"
/// @path-param id: String "The profile's id: 1–64 of `A-Za-z0-9._-`"
/// @body `$LlmDefinition`
/// @return 400: `$ErrorBody` "A name, an id or a fallback that is not 1–64 of `A-Za-z0-9._-`, not a profile's definition, or the seat agent's own refusal, passed back"
/// @return 404: `$ErrorBody` "No gateway console is configured, or the gateway has no such seat agent"
/// @return 504: `$ErrorBody` "The seat agent did not answer in time (the gateway's)"
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
    if definition.fallback.as_deref().is_some_and(|f| !llm_name(f)) {
        return Err(refuse(
            StatusCode::BAD_REQUEST,
            "a fallback is a profile's id: 1-64 of A-Z a-z 0-9 . - _",
        ));
    }
    let body = serde_json::to_vec(&definition)
        .map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a profile's definition"))?;
    let done = ask(&gateway, "PUT", path, &admin, Some(body)).await?;
    if done.status == 200 {
        llm_noted(&shared, &admin, "gateway.llm.profile.write", &host, &id).await;
    }
    Ok(answer(done))
}

/// @route DELETE /ui/api/admin/llm/seathosts/{host}/profiles/{id}
/// @scope feedback
/// @summary Delete a hosted profile
/// @description The gateway's `DELETE /admin/llm/seathosts/{host}/profiles/{id}`, passed on with its status; a profile deleted is an audit row.
/// @tag Models
/// @operation-id `deleteLlmProfile`
/// @security session + csrf
/// @insert `ConsoleChangeRefusals`
/// @return 204: "Deleted"
/// @path-param host: String "The seat agent's name: 1–64 of `A-Za-z0-9._-`"
/// @path-param id: String "The profile's id: 1–64 of `A-Za-z0-9._-`"
/// @return 400: `$ErrorBody` "A name or an id that is not 1–64 of `A-Za-z0-9._-`, or the seat agent's own refusal"
/// @return 404: `$ErrorBody` "No gateway console is configured, or the gateway has no such seat agent or profile"
/// @return 504: `$ErrorBody` "The seat agent did not answer in time (the gateway's)"
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

/// `POST …/profiles/{id}/enabled`.
/// @openapi
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LlmEnabled {
    /// @description Whether new games may seat the profile.
    enabled: bool,
}

/// @route POST /ui/api/admin/llm/seathosts/{host}/profiles/{id}/enabled
/// @scope feedback
/// @summary Switch a hosted profile on or off
/// @description The gateway's `POST …/profiles/{id}/enabled`, passed on with its status; a switch is an audit row.
/// @tag Models
/// @operation-id `switchLlmProfile`
/// @security session + csrf
/// @insert `ConsoleChangeRefusals`
/// @return 204: "Switched"
/// @path-param host: String "The seat agent's name: 1–64 of `A-Za-z0-9._-`"
/// @path-param id: String "The profile's id: 1–64 of `A-Za-z0-9._-`"
/// @body `$LlmEnabled`
/// @return 400: `$ErrorBody` "A name or an id that is not 1–64 of `A-Za-z0-9._-`, not an `enabled` boolean, or the seat agent's own refusal"
/// @return 404: `$ErrorBody` "No gateway console is configured, or the gateway has no such seat agent or profile"
/// @return 504: `$ErrorBody` "The seat agent did not answer in time (the gateway's)"
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

/// @route POST /ui/api/admin/llm/seathosts/{host}/profiles/{id}/probe
/// @scope feedback
/// @summary Probe a hosted profile
/// @description The gateway's `POST …/profiles/{id}/probe`, passed on with its status: the seat agent tries the profile's model. A probe asked for is an audit row.
/// @tag Models
/// @operation-id `probeLlmProfile`
/// @security session + csrf
/// @insert `ConsoleChangeRefusals`
/// @return 202: "Asked for; the seat agent tries the model"
/// @path-param host: String "The seat agent's name: 1–64 of `A-Za-z0-9._-`"
/// @path-param id: String "The profile's id: 1–64 of `A-Za-z0-9._-`"
/// @return 400: `$ErrorBody` "A name or an id that is not 1–64 of `A-Za-z0-9._-`, or the seat agent's own refusal"
/// @return 404: `$ErrorBody` "No gateway console is configured, or the gateway has no such seat agent or profile"
/// @return 504: `$ErrorBody` "The seat agent did not answer in time (the gateway's)"
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
///
/// @description A profile's API key, write-only.
/// @openapi
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LlmKey {
    /// @description The key, or `null` to forget the one kept.
    /// @openapi format: password
    key: Option<String>,
}

/// @route POST /ui/api/admin/llm/seathosts/{host}/profiles/{id}/key
/// @scope feedback
/// @summary Set or forget a hosted profile's key
/// @description The gateway's `POST …/profiles/{id}/key`, passed on with its status. The key is held in memory for this request only: it is in no row and no log line, and an error naming it is never passed back. A key set or forgotten is an audit row naming `<host>/<id>`.
/// @tag Models
/// @operation-id `setLlmKey`
/// @security session + csrf
/// @insert `ConsoleChangeRefusals`
/// @return 204: "Kept, or forgotten"
/// @path-param host: String "The seat agent's name: 1–64 of `A-Za-z0-9._-`"
/// @path-param id: String "The profile's id: 1–64 of `A-Za-z0-9._-`"
/// @body `$LlmKey`
/// @return 400: `$ErrorBody` "A name or an id that is not 1–64 of `A-Za-z0-9._-`, not a `key` string or `null`, or the seat agent's own refusal"
/// @return 404: `$ErrorBody` "No gateway console is configured, or the gateway has no such seat agent or profile"
/// @return 504: `$ErrorBody` "The seat agent did not answer in time (the gateway's)"
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

// ------------------------------------------------- who may seat which model

/// What `GET /ui/api/admin/accounts/lookup` takes, and all it passes on.
///
/// @description The type-ahead's search; any other parameter is `400`.
/// @openapi
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct Lookup {
    /// @description The start of a registered account's name, two characters
    /// or more.
    q: String,
}

/// @route GET /ui/api/admin/accounts/lookup
/// @scope feedback
/// @summary Look up registered accounts
/// @description The gateway's `GET /admin/accounts/lookup`, passed on with its status: at most ten registered accounts whose name starts so, for the access list's type-ahead.
/// @tag Models
/// @operation-id `lookupAccounts`
/// @security session
/// @insert `ConsoleRefusals`
/// @return 200: {} "The gateway's `GET /admin/accounts/lookup` answer, as it is"
/// @query-params `$Lookup`
/// @return 400: `$ErrorBody` "Not a search for accounts, or the gateway's own refusal"
async fn lookup(
    State(shared): State<Shared>,
    headers: HeaderMap,
    query: Result<axum::extract::Query<Lookup>, axum::extract::rejection::QueryRejection>,
) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let axum::extract::Query(query) =
        query.map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a search for accounts"))?;
    ask(
        &gateway,
        "GET",
        format!("/admin/accounts/lookup?q={}", encoded(&query.q)),
        &admin,
        None,
    )
    .await
    .map(answer)
}

/// `{id}`, checked: a profile's id as the gateway takes it.
fn access_path(id: &str) -> Result<String, Refusal> {
    if llm_name(id) {
        Ok(format!("/admin/llm/access/{id}"))
    } else {
        Err(refuse(
            StatusCode::BAD_REQUEST,
            "a profile's id is 1-64 of A-Z a-z 0-9 . - _",
        ))
    }
}

/// @route GET /ui/api/admin/llm/access
/// @scope feedback
/// @summary Who may seat each hosted model
/// @description The gateway's `GET /admin/llm/access`, passed on with its status: each profile's policy, default mark and list.
/// @tag Models
/// @security session
/// @insert `ConsoleRefusals`
/// @return 200: {} "The gateway's `GET /admin/llm/access` answer, as it is"
async fn access_list(
    State(shared): State<Shared>,
    headers: HeaderMap,
) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    ask(&gateway, "GET", "/admin/llm/access".into(), &admin, None)
        .await
        .map(answer)
}

/// @route GET /ui/api/admin/llm/access/{id}
/// @scope feedback
/// @summary Who may seat one hosted model
/// @description The gateway's `GET /admin/llm/access/{id}`, passed on with its status.
/// @tag Models
/// @security session
/// @insert `ConsoleRefusals`
/// @return 200: {} "The gateway's `GET /admin/llm/access/{id}` answer, as it is"
/// @path-param id: String "The profile's id: 1–64 of `A-Za-z0-9._-`"
/// @return 400: `$ErrorBody` "A profile's id that is not 1–64 of `A-Za-z0-9._-`"
async fn access_one(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, Refusal> {
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let path = access_path(&id)?;
    ask(&gateway, "GET", path, &admin, None).await.map(answer)
}

/// What `PUT /ui/api/admin/llm/access/{id}` takes, and all it passes on.
///
/// @description Who may seat a hosted model.
/// @openapi
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AccessPolicy {
    /// @description `registered` (every registered account), `listed` (the
    /// accounts on its list) or `nobody`; the gateway refuses any other.
    /// @openapi enum: [registered, listed, nobody]
    policy: String,
    /// @description Whether it is the model a registered account is offered
    /// first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default_for_registered: Option<bool>,
}

/// One audit line's detail for a policy: the profile, the policy, and
/// whether it is the default one; nothing else.
fn policy_detail(id: &str, order: &AccessPolicy) -> String {
    let word = match order.policy.as_str() {
        "registered" => "registered",
        "listed" => "listed",
        "nobody" => "nobody",
        _ => "?",
    };
    match order.default_for_registered {
        Some(true) => format!("{id}: {word}, the default"),
        Some(false) => format!("{id}: {word}, not the default"),
        None => format!("{id}: {word}"),
    }
}

/// @route PUT /ui/api/admin/llm/access/{id}
/// @scope feedback
/// @summary Set who may seat a hosted model
/// @description The gateway's `PUT /admin/llm/access/{id}`, passed on with its status; a policy set is an audit row with the profile, the policy and the default mark.
/// @tag Models
/// @operation-id `setLlmAccessPolicy`
/// @security session + csrf
/// @insert `ConsoleChangeRefusals`
/// @return 200: {} "Set, as the gateway answers"
/// @path-param id: String "The profile's id: 1–64 of `A-Za-z0-9._-`"
/// @body `$AccessPolicy`
/// @return 400: `$ErrorBody` "A profile's id that is not 1–64 of `A-Za-z0-9._-`, not a policy, or the gateway's own refusal"
async fn access_policy(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Response, Refusal> {
    same_origin(&headers)?;
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let path = access_path(&id)?;
    let order: AccessPolicy = serde_json::from_slice(&body)
        .map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a policy"))?;
    let body =
        serde_json::to_vec(&order).map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a policy"))?;
    let done = ask(&gateway, "PUT", path, &admin, Some(body)).await?;
    if done.status == 200 {
        let detail = policy_detail(&id, &order);
        tracing::info!(admin, action = "gateway.llm.access.policy", %detail, "admin console");
        if let Err(e) = note(&shared, &admin, "gateway.llm.access.policy", &detail).await {
            tracing::error!("admin console: the audit row was not written: {e}");
        }
    }
    Ok(answer(done))
}

/// What `…/allow` and `…/disallow` take, and all they pass on: account
/// ids, each a UUID, at most 200.
///
/// @description Accounts for a hosted model's list.
/// @openapi
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AccessAccounts {
    /// @description The accounts, by id, 1 to 200.
    /// @openapi
    /// ```yaml
    /// minItems: 1
    /// ```
    /// @openapi
    /// ```yaml
    /// maxItems: 200
    /// ```
    accounts: Vec<Uuid>,
}

/// An audit line's detail for a change to a list: the profile and the
/// account ids; never a username or a handle.
fn accounts_detail(id: &str, accounts: &[Uuid]) -> String {
    let ids: Vec<String> = accounts.iter().map(Uuid::to_string).collect();
    format!("{id}: {}", ids.join(","))
}

async fn access_change(
    shared: &Shared,
    headers: &HeaderMap,
    id: &str,
    body: &[u8],
    allow: bool,
) -> Result<Response, Refusal> {
    same_origin(headers)?;
    let (admin, gateway) = signed_in(shared, headers).await?;
    let path = access_path(id)?;
    let change: AccessAccounts = serde_json::from_slice(body).map_err(|_| {
        refuse(
            StatusCode::BAD_REQUEST,
            "send {\"accounts\": [\"<id>\", …]}",
        )
    })?;
    if change.accounts.is_empty() || change.accounts.len() > 200 {
        return Err(refuse(
            StatusCode::BAD_REQUEST,
            "name 1 to 200 accounts at a time",
        ));
    }
    let body = serde_json::to_vec(&change)
        .map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a list of accounts"))?;
    let (suffix, action) = if allow {
        ("allow", "gateway.llm.access.allow")
    } else {
        ("disallow", "gateway.llm.access.disallow")
    };
    let done = ask(
        &gateway,
        "POST",
        format!("{path}/{suffix}"),
        &admin,
        Some(body),
    )
    .await?;
    if done.status == 200 {
        let detail = accounts_detail(id, &change.accounts);
        tracing::info!(admin, action, %detail, "admin console");
        if let Err(e) = note(shared, &admin, action, &detail).await {
            tracing::error!("admin console: the audit row was not written: {e}");
        }
    }
    Ok(answer(done))
}

/// @route POST /ui/api/admin/llm/access/{id}/allow
/// @scope feedback
/// @summary Put accounts on a hosted model's list
/// @description The gateway's `POST /admin/llm/access/{id}/allow`, passed on with its status; a change is an audit row with the profile and the account ids.
/// @tag Models
/// @operation-id `allowLlmAccounts`
/// @security session + csrf
/// @insert `ConsoleChangeRefusals`
/// @return 200: {} "Added, as the gateway answers"
/// @path-param id: String "The profile's id: 1–64 of `A-Za-z0-9._-`"
/// @body `$AccessAccounts`
/// @return 400: `$ErrorBody` "A profile's id that is not 1–64 of `A-Za-z0-9._-`, not a list of account ids, not 1 to 200 of them, or the gateway's own refusal"
async fn access_allow(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Response, Refusal> {
    access_change(&shared, &headers, &id, &body, true).await
}

/// @route POST /ui/api/admin/llm/access/{id}/disallow
/// @scope feedback
/// @summary Take accounts off a hosted model's list
/// @description The gateway's `POST /admin/llm/access/{id}/disallow`, passed on with its status; a change is an audit row with the profile and the account ids.
/// @tag Models
/// @operation-id `disallowLlmAccounts`
/// @security session + csrf
/// @insert `ConsoleChangeRefusals`
/// @return 200: {} "Taken off, as the gateway answers"
/// @path-param id: String "The profile's id: 1–64 of `A-Za-z0-9._-`"
/// @body `$AccessAccounts`
/// @return 400: `$ErrorBody` "A profile's id that is not 1–64 of `A-Za-z0-9._-`, not a list of account ids, not 1 to 200 of them, or the gateway's own refusal"
async fn access_disallow(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Response, Refusal> {
    access_change(&shared, &headers, &id, &body, false).await
}

/// An audit row about no report: a change made through the console.
pub(crate) async fn note(
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

/// One change made through the admin console.
/// @openapi
#[derive(Serialize)]
struct ConsoleChange {
    /// @description When, RFC 3339 in UTC.
    /// @openapi format: date-time
    at: String,
    /// @description The admin who made it.
    actor: String,
    /// @description What: `gateway.invite.create`, `gateway.card.block`,
    /// `gateway.account.ban`, `gateway.llm.profile.write`, ….
    action: String,
    /// @description Ids and ends; never a key, a password, a note or a
    /// reason.
    detail: Option<String>,
}

/// @route GET /ui/api/admin/audit
/// @scope feedback
/// @summary The latest changes made through the console
/// @description The 50 latest changes made through the admin console,
/// newest first: this service's own audit rows, not the gateway's.
/// @tag Console
/// @operation-id `consoleAudit`
/// @security session
/// @insert `SessionRefusals`
/// @return 404: `$ErrorBody` "No gateway console is configured for this service (`FEEDBACK_GATEWAY_ADMIN_URL`)"
async fn audit(
    State(shared): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<Vec<ConsoleChange>>, Refusal> {
    signed_in(&shared, &headers).await?;
    let db = &shared.state.db;
    let rows = db
        .query_all_raw(Statement::from_string(
            db.get_database_backend(),
            format!(
                "SELECT to_char(at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS at, \
                     actor, action, detail FROM feedback_audit WHERE report_id IS NULL AND contact_id IS NULL \
                 ORDER BY at DESC, id DESC LIMIT {AUDIT_ROWS}"
            ),
        ))
        .await
        .map_err(|e| db_down(&e))?;
    rows.iter()
        .map(|row| {
            Ok(ConsoleChange {
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
    fn an_account_search_passes_on_its_seven_fields_and_nothing_else() {
        assert_eq!(AccountsQuery::default().path(), "/admin/accounts");
        let query = AccountsQuery {
            q: Some("Al ice#af&x=1".into()),
            tag: Some("Alice#af03&x=1".into()),
            kind: Some("guest".into()),
            sort: Some(String::new()),
            online: Some(true),
            offset: Some(50),
            limit: Some(25),
        };
        assert_eq!(
            query.path(),
            "/admin/accounts?q=Al%20ice%23af%26x%3D1&tag=Alice%23af03%26x%3D1&kind=guest&online=true&offset=50&limit=25"
        );
        // A tag alone, and a blank one is none.
        let alone = AccountsQuery {
            tag: Some("af03".into()),
            ..AccountsQuery::default()
        };
        assert_eq!(alone.path(), "/admin/accounts?tag=af03");
        let blank = AccountsQuery {
            tag: Some(String::new()),
            ..AccountsQuery::default()
        };
        assert_eq!(blank.path(), "/admin/accounts");
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

    #[test]
    fn an_access_path_takes_only_a_profiles_id() {
        assert_eq!(
            access_path("sonnet-5.5_a").ok().as_deref(),
            Some("/admin/llm/access/sonnet-5.5_a")
        );
        assert!(access_path(&"a".repeat(64)).is_ok());
        for id in ["", "a/b", "a/../b", "a b", "a?x=1", "a%2Fb", "ä"] {
            let Err(refused) = access_path(id) else {
                panic!("{id:?} passed");
            };
            assert_eq!(refused.0, StatusCode::BAD_REQUEST, "{id:?}");
        }
        assert!(access_path(&"a".repeat(65)).is_err(), "65 characters");
        // `..` is made of allowed characters; it must still not reach the
        // gateway as a path segment.
        assert!(access_path("..").is_err(), "{:?}", access_path("..").ok());
    }

    #[test]
    fn a_policys_audit_line_names_the_profile_the_policy_and_the_default() {
        let order = |policy: &str, default: Option<bool>| AccessPolicy {
            policy: policy.into(),
            default_for_registered: default,
        };
        assert_eq!(
            policy_detail("sonnet", &order("registered", None)),
            "sonnet: registered"
        );
        assert_eq!(
            policy_detail("sonnet", &order("registered", Some(true))),
            "sonnet: registered, the default"
        );
        assert_eq!(
            policy_detail("opus", &order("listed", Some(false))),
            "opus: listed, not the default"
        );
        assert_eq!(
            policy_detail("opus", &order("nobody", None)),
            "opus: nobody"
        );
        // Whatever else a caller typed as a policy is never echoed.
        assert_eq!(
            policy_detail("opus", &order("<script>alert(1)</script>", None)),
            "opus: ?"
        );
        assert_eq!(
            policy_detail("opus", &order("", Some(true))),
            "opus: ?, the default"
        );
    }

    #[test]
    fn a_lists_audit_line_names_ids_only() {
        let a = Uuid::parse_str("0199aaaa-0000-7000-8000-000000000001").unwrap();
        let b = Uuid::parse_str("0199aaaa-0000-7000-8000-000000000002").unwrap();
        assert_eq!(
            accounts_detail("opus", &[a, b]),
            "opus: 0199aaaa-0000-7000-8000-000000000001,0199aaaa-0000-7000-8000-000000000002"
        );
        assert_eq!(
            accounts_detail("opus", &[a]),
            "opus: 0199aaaa-0000-7000-8000-000000000001"
        );
        assert_eq!(accounts_detail("opus", &[]), "opus: ");
    }
}
