//! baylee-gateway — accounts, decks, lobby, and hosted games.
//!
//! Security posture (see auth.rs): Argon2id password hashing, hashed
//! bearer tokens with sliding expiry, auth rate limiting, generic
//! credential errors, constant-time comparisons. TLS terminates at the
//! reverse proxy in front of this process (Caddy/nginx) — this service
//! must never be exposed on a plaintext listener in production.

mod auth;
mod engine;
mod lobby;
mod pool;
mod store;

use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use axum::routing::{get, post};
use baylee_protocol::v1::{self, Envelope};
use lobby::{Lobby, LobbyGame, LobbyState};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use store::{Account, Deck, Store, StoredToken};
use tracing_subscriber::EnvFilter;

/// Shared gateway state.
struct AppState {
    store: Mutex<Store>,
    limiter: auth::RateLimiter,
    lobby: Mutex<Lobby>,
    store_path: PathBuf,
    /// Signals the background writer that the store needs persisting
    /// (debounced — serializing the whole store must stay out of the
    /// request path).
    save_tx: tokio::sync::mpsc::UnboundedSender<()>,
    /// Registration toggle (`BAYLEE_REGISTRATION=off` to disable).
    registration_enabled: bool,
    /// Proxies whose `X-Forwarded-For` header may be trusted for rate
    /// limiting (`BAYLEE_TRUSTED_PROXIES`, comma-separated IPs). Empty =
    /// the header is never trusted; anyone can set it, so trusting it
    /// blindly disables the brute-force defense.
    trusted_proxies: Vec<IpAddr>,
    /// Every agent connected right now, and the games each was asked to run.
    ///
    /// Nothing here is persisted: an agent that reconnects is a new agent, and
    /// a game whose agent went away is a game whose engine either reports its
    /// own end or stops existing.
    agents: Mutex<engine::Agents>,
    /// The shared secret an agent proves itself with (`BAYLEE_AGENT_TOKEN`).
    ///
    /// Without one no agent may connect, and therefore no game can start: an
    /// unauthenticated control plane is a way to run processes on somebody
    /// else's machine.
    agent_token: Option<String>,
    /// The websocket an engine is told to dial back on (`BAYLEE_ENGINE_URL`).
    ///
    /// Loopback by default, which is right for a single-box deployment and
    /// wrong the moment an agent runs somewhere else.
    engine_url: String,
    /// The card catalog, when `DATABASE_URL` is configured.
    ///
    /// Optional on purpose: accounts, decks and lobbies live in the JSON
    /// store and need no database, so a gateway without Postgres still runs
    /// a full game — it just cannot serve card text.
    catalog: Option<baylee_catalog::Catalog>,
}

impl AppState {
    /// Ask the background writer to persist the store (cheap, debounced).
    fn request_save(&self) {
        let _ = self.save_tx.send(());
    }
}

type Shared = Arc<AppState>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(28766);
    let store_path = std::env::var("STORE_PATH")
        .map_or_else(|_| PathBuf::from("gateway-store.json"), PathBuf::from);
    let (save_tx, save_rx) = tokio::sync::mpsc::unbounded_channel();
    let catalog = connect_catalog().await;
    let agent_token = std::env::var("BAYLEE_AGENT_TOKEN")
        .ok()
        .filter(|t| !t.is_empty());
    if agent_token.is_none() {
        tracing::warn!("BAYLEE_AGENT_TOKEN is not set: no agent can connect, so no game can start");
    }
    let state = Arc::new(AppState {
        store: Mutex::new(Store::load(&store_path)),
        limiter: auth::RateLimiter::new(std::time::Duration::from_secs(300), 10),
        lobby: Mutex::new(Lobby::default()),
        agents: Mutex::new(engine::Agents::default()),
        agent_token,
        engine_url: std::env::var("BAYLEE_ENGINE_URL")
            .unwrap_or_else(|_| format!("ws://127.0.0.1:{port}/engine/ws")),
        store_path,
        save_tx,
        registration_enabled: std::env::var("BAYLEE_REGISTRATION")
            .map_or(true, |v| !matches!(v.as_str(), "off" | "0" | "false")),
        trusted_proxies: std::env::var("BAYLEE_TRUSTED_PROXIES")
            .unwrap_or_default()
            .split(',')
            .filter_map(|s| s.trim().parse::<IpAddr>().ok())
            .collect(),
        catalog,
    });
    spawn_store_writer(state.clone(), save_rx);
    spawn_cleanup(state.clone());

    let app = Router::new()
        .route("/auth/config", get(auth_config))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/me", get(me))
        .route("/decks", get(list_decks).post(create_deck))
        .route(
            "/decks/{id}",
            get(get_deck).put(update_deck).delete(delete_deck),
        )
        .route("/lobby/games", get(list_games).post(create_game))
        .route("/automation", get(list_automation).put(set_automation))
        .route("/settings", get(get_settings).put(put_settings))
        .route("/lobby/games/{id}/join", post(join_game))
        .route("/lobby/games/{id}/seats/{seat}", post(set_seat))
        .route("/lobby/games/{id}/ready", post(set_ready))
        .route("/lobby/games/{id}/start", post(start_room))
        .route("/lobby/games/{id}/host", post(hand_over))
        .route("/lobby/games/{id}/leave", post(leave_game))
        .route("/games/{id}/ws", get(game_ws))
        // The control and engine planes. Neither carries a player's traffic
        // and neither accepts a player's token; see `engine.rs`.
        .route("/agent/ws", get(engine::agent_ws))
        .route("/engine/ws", get(engine::engine_ws))
        .route("/pool", get(pool::pool))
        .route("/printings", get(pool::printings))
        .route("/catalog/text", get(catalog_text))
        .route("/catalog/search", get(catalog_search))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("bind gateway port");
    tracing::info!(port, "baylee-gateway listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("gateway serves");
}

/// Persists the store outside the request path: mutation handlers send a
/// dirty signal, this task debounces bursts and writes one snapshot.
fn spawn_store_writer(state: Shared, mut rx: tokio::sync::mpsc::UnboundedReceiver<()>) {
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            while rx.try_recv().is_ok() {}
            let snapshot = state.store.lock().clone();
            let path = state.store_path.clone();
            let written = tokio::task::spawn_blocking(move || snapshot.save(&path)).await;
            if let Ok(Err(e)) = written {
                tracing::warn!(%e, "store write failed");
            }
        }
    });
}

/// Periodically reclaims finished games (after a grace period), stale
/// waiting lobbies, and expired tokens — all three grew without bound.
fn spawn_cleanup(state: Shared) {
    /// How long a finished game stays joinable for reconnect/review.
    const OVER_GRACE_SECS: u64 = 3600;
    /// How long an unattended waiting lobby stays open.
    const WAITING_TIMEOUT_SECS: u64 = 2 * 3600;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(600));
        interval.tick().await;
        loop {
            interval.tick().await;
            let now = auth::now_secs();
            {
                let mut lobby = state.lobby.lock();
                // A game reaching its end is no longer something to discover
                // here: the engine says so on its own link, and losing that
                // link closes the game too. All that is left is reclaiming.
                lobby.games.retain(|_, g| match g.state {
                    LobbyState::Waiting => now.saturating_sub(g.created_at) < WAITING_TIMEOUT_SECS,
                    LobbyState::Over => g
                        .finished_at
                        .is_none_or(|t| now.saturating_sub(t) < OVER_GRACE_SECS),
                    LobbyState::Playing => true,
                });
            }
            let purged = {
                let mut store = state.store.lock();
                let before = store.tokens.len();
                store.tokens.retain(|_, t| t.expires_at > now);
                before - store.tokens.len()
            };
            if purged > 0 {
                state.request_save();
            }
        }
    });
}

// ------------------------------------------------------------------- auth

#[derive(Deserialize)]
struct RegisterBody {
    email: String,
    display_name: String,
    password: String,
}

#[derive(Deserialize)]
struct Credentials {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
}

fn err(status: StatusCode, message: &'static str) -> (StatusCode, Json<ErrorBody>) {
    (status, Json(ErrorBody { error: message }))
}

/// The IP a rate limit is keyed on: the real peer address, unless the
/// peer itself is a configured trusted proxy — only then is its
/// `X-Forwarded-For` honored. Trusting the header unconditionally let any
/// client rotate it per request and disable the limiter entirely.
fn rate_limit_ip(state: &AppState, peer: IpAddr, headers: &HeaderMap) -> String {
    if state.trusted_proxies.contains(&peer)
        && let Some(forwarded) = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
    {
        return forwarded.trim().to_string();
    }
    peer.to_string()
}

/// Public auth configuration (clients check this before offering
/// registration).
async fn auth_config(State(state): State<Shared>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "registration_enabled": state.registration_enabled,
    }))
}

async fn register(
    State(state): State<Shared>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<RegisterBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    if !state.registration_enabled {
        return Err(err(StatusCode::FORBIDDEN, "registration is disabled"));
    }
    if !state
        .limiter
        .allow(&rate_limit_ip(&state, addr.ip(), &headers))
    {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many attempts"));
    }
    if !auth::valid_email(&body.email) {
        return Err(err(StatusCode::BAD_REQUEST, "invalid e-mail"));
    }
    if !auth::valid_display_name(&body.display_name) {
        return Err(err(StatusCode::BAD_REQUEST, "invalid display name"));
    }
    if !auth::valid_password(&body.email, &body.display_name, &body.password) {
        return Err(err(StatusCode::BAD_REQUEST, "invalid password"));
    }
    // Anti-enumeration: identical response AND identical work whether or
    // not the e-mail/display name was free — hashing always (~100 ms),
    // so timing can't tell "taken" (fast reject) from "created".
    // Argon2 is deliberately expensive and runs off the async worker.
    let password = body.password.clone();
    let password_hash = tokio::task::spawn_blocking(move || auth::hash_password(&password))
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "hashing failed"))?;
    let mut store = state.store.lock();
    let email_taken = store.account_by_email(&body.email).is_some();
    let name_taken = store.account_by_display_name(&body.display_name).is_some();
    if !email_taken && !name_taken {
        let email = body.email.to_lowercase();
        let account = Account {
            id: auth::new_id(),
            email,
            display_name: body.display_name,
            password_hash,
            created_at: auth::now_secs(),
        };
        let id = account.id.clone();
        store.accounts.insert(id, account);
        state.request_save();
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn login(
    State(state): State<Shared>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(creds): Json<Credentials>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    if !state
        .limiter
        .allow(&rate_limit_ip(&state, addr.ip(), &headers))
    {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many attempts"));
    }
    // Fetch the hash under a short lock; the expensive verify runs off
    // the async worker and outside the store lock.
    let account = {
        let store = state.store.lock();
        store
            .account_by_email(&creds.email)
            .map(|a| (a.id.clone(), a.password_hash.clone()))
    };
    let (account_id, stored_hash) = account.unzip();
    let password = creds.password.clone();
    let ok = tokio::task::spawn_blocking(move || {
        auth::verify_password(stored_hash.as_deref(), &password)
    })
    .await
    .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "verify failed"))?;
    let Some(account_id) = account_id else {
        return Err(err(StatusCode::UNAUTHORIZED, "invalid credentials"));
    };
    if !ok {
        return Err(err(StatusCode::UNAUTHORIZED, "invalid credentials"));
    }
    let issued = auth::IssuedToken::new();
    let mut store = state.store.lock();
    store.tokens.insert(
        auth::token_hash(&issued.token),
        StoredToken {
            token_hash: auth::token_hash(&issued.token),
            account_id,
            expires_at: issued.expires_at,
        },
    );
    state.request_save();
    Ok(Json(serde_json::json!({
        "token": issued.token,
        "expires_at": issued.expires_at,
    })))
}

/// Resolves the bearer token to an account id.
fn authed(state: &Shared, headers: &HeaderMap) -> Result<String, (StatusCode, Json<ErrorBody>)> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "missing bearer token"))?;
    let mut store = state.store.lock();
    let account_id = store
        .resolve_token(token, auth::now_secs())
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "invalid or expired token"))?;
    state.request_save();
    Ok(account_id)
}

// ------------------------------------------------------------------ catalog

/// How many printings one text request may ask for.
///
/// A commander table's whole print table is a few hundred entries, so this
/// covers a full game in one round trip and still bounds what a single request
/// can cost.
const MAX_TEXT_IDS: usize = 500;

/// How many unknown cards one request may pull from Scryfall.
///
/// Filling on demand is for the handful of cards a bulk snapshot missed, not
/// for populating an empty catalog — that is what `baylee-catalog ingest` is.
/// Scryfall's rate limit is a shared budget (`docs/legal.md` §3), so a single
/// client cannot be allowed to spend all of it.
const MAX_ONDEMAND_FILL: usize = 25;

/// Connects the card catalog when `DATABASE_URL` is configured.
///
/// A failure here is logged and otherwise ignored: card text is presentation,
/// and a gateway that cannot reach Postgres should still host games.
async fn connect_catalog() -> Option<baylee_catalog::Catalog> {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|u| !u.is_empty())?;
    match baylee_catalog::Catalog::connect(&url).await {
        Ok(catalog) => {
            if let Err(err) = catalog.migrate().await {
                tracing::error!(%err, "card catalog schema could not be applied");
                return None;
            }
            let count = catalog.count().await.unwrap_or(0);
            tracing::info!(printings = count, "card catalog connected");
            Some(catalog)
        }
        Err(err) => {
            tracing::error!(%err, "card catalog unavailable; text endpoints disabled");
            None
        }
    }
}

/// Query for `/catalog/text`.
#[derive(Deserialize)]
struct CatalogTextQuery {
    /// Comma-separated Scryfall printing ids.
    ids: String,
    /// Preferred language; English is the fallback.
    lang: Option<String>,
}

/// Card text for a set of printings.
///
/// Deliberately unauthenticated. This is public reference data — Scryfall
/// serves the same thing without a token — and a client has to be able to draw
/// a readable card before it has an account, which is exactly the case when a
/// card image fails to load on first launch.
async fn catalog_text(
    State(state): State<Shared>,
    Query(params): Query<CatalogTextQuery>,
) -> Result<Json<Vec<baylee_catalog::CardTextEntry>>, (StatusCode, Json<ErrorBody>)> {
    let catalog = state.catalog.as_ref().ok_or_else(|| {
        err(
            StatusCode::SERVICE_UNAVAILABLE,
            "card catalog not configured",
        )
    })?;
    let lang = params.lang.as_deref().unwrap_or("en").to_lowercase();

    // Only well-formed ids reach the query: they are bound parameters, so this
    // is not about injection, but one malformed id would fail the cast for the
    // whole batch and cost every other card its text.
    let ids: Vec<String> = params
        .ids
        .split(',')
        .map(str::trim)
        .filter(|id| uuid::Uuid::parse_str(id).is_ok())
        .map(str::to_lowercase)
        .take(MAX_TEXT_IDS)
        .collect();
    if ids.is_empty() {
        return Ok(Json(Vec::new()));
    }

    let mut found = catalog
        .text(&ids, &lang)
        .await
        .map_err(|e| catalog_error("looking up card text", &e))?;

    // Anything the catalog has never seen is fetched once and kept.
    let missing: Vec<String> = ids
        .iter()
        .filter(|id| !found.iter().any(|e| &&e.scryfall_id == id))
        .take(MAX_ONDEMAND_FILL)
        .cloned()
        .collect();
    if !missing.is_empty() {
        let wanted = missing.clone();
        let fetched = tokio::task::spawn_blocking(move || {
            wanted
                .iter()
                .filter_map(|id| baylee_catalog::ingest::fetch_one_blocking(id).ok())
                .collect::<Vec<_>>()
        })
        .await
        .unwrap_or_default();
        if !fetched.is_empty() {
            if let Err(err) = catalog.upsert(&fetched).await {
                tracing::warn!(%err, "storing on-demand cards failed");
            }
            found = catalog
                .text(&ids, &lang)
                .await
                .map_err(|e| catalog_error("looking up card text", &e))?;
        }
    }
    Ok(Json(found))
}

/// Query for `/catalog/search`.
#[derive(Deserialize)]
struct CatalogSearchQuery {
    /// Search terms.
    q: String,
    /// Preferred language.
    lang: Option<String>,
    /// Maximum hits.
    limit: Option<u64>,
}

/// Searches the card catalog — the entry point the deck builder will use.
async fn catalog_search(
    State(state): State<Shared>,
    Query(params): Query<CatalogSearchQuery>,
) -> Result<Json<Vec<baylee_catalog::SearchHit>>, (StatusCode, Json<ErrorBody>)> {
    let catalog = state.catalog.as_ref().ok_or_else(|| {
        err(
            StatusCode::SERVICE_UNAVAILABLE,
            "card catalog not configured",
        )
    })?;
    let lang = params.lang.as_deref().unwrap_or("en").to_lowercase();
    let limit = params.limit.unwrap_or(50).clamp(1, 200);
    let hits = catalog
        .search(params.q.trim(), &lang, limit)
        .await
        .map_err(|e| catalog_error("searching the catalog", &e))?;
    Ok(Json(hits))
}

/// Logs a catalog failure and returns a response that leaks nothing.
///
/// The error text can carry connection strings and SQL, neither of which
/// belongs in a client response.
fn catalog_error(what: &str, error: &impl std::fmt::Display) -> (StatusCode, Json<ErrorBody>) {
    tracing::error!(%error, "{what} failed");
    err(StatusCode::BAD_GATEWAY, "card catalog request failed")
}

async fn logout(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, Json<ErrorBody>)> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "missing bearer token"))?;
    let mut store = state.store.lock();
    store.tokens.remove(&auth::token_hash(token));
    state.request_save();
    Ok(StatusCode::NO_CONTENT)
}

/// The authenticated account's profile.
async fn me(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    let store = state.store.lock();
    let account = store
        .accounts
        .get(&account_id)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "account gone"))?;
    Ok(Json(serde_json::json!({
        "id": account.id,
        "email": account.email,
        "display_name": account.display_name,
    })))
}

// ------------------------------------------------------------------ decks

#[derive(Deserialize)]
struct DeckBody {
    name: String,
    cards: Vec<String>,
    /// Optional: a deck saved without one simply has no sideboard.
    #[serde(default)]
    sideboard: Vec<String>,
    commander: Option<String>,
}

/// Hard cap on the expanded card count of one deck. Comfortably above
/// every legal format size (100 for commander), far below anything that
/// could strain memory at game start.
const MAX_DECK_CARDS: u32 = 250;

/// Parsed lines as the flat card list the engine's preset takes, one entry
/// per copy, each carrying the printing its row named.
fn expand(lines: &[ParsedLine]) -> Vec<baylee_cards::decks::DeckCard> {
    let mut out = Vec::new();
    for line in lines {
        for _ in 0..line.count {
            out.push(baylee_cards::decks::DeckCard::chosen(
                line.index,
                &line.print,
            ));
        }
    }
    out
}

/// One parsed deck line: how many of which card, printed how.
struct ParsedLine {
    count: u32,
    index: baylee_core::ids::CardIndex,
    print: baylee_core::deckrow::PrintChoice,
}

/// Parses and validates deck lines. Shared by `validate_deck` and
/// `loaded_deck` so a deck can never pass one and explode the other:
/// counts are parsed here (1–4, unlimited for basic lands) and the
/// expanded total is capped at [`MAX_DECK_CARDS`].
///
/// The row grammar lives in `baylee_core::deckrow`, so a stored deck, an
/// exported file and an imported one are read by the same code. A row that
/// names only a card is the old form and still means what it meant.
fn parse_deck_lines(lines: &[String]) -> Result<Vec<ParsedLine>, (StatusCode, Json<ErrorBody>)> {
    let mut out = Vec::with_capacity(lines.len());
    let mut total: u32 = 0;
    for line in lines {
        let row = baylee_core::deckrow::parse(line).map_err(|e| match e {
            baylee_core::deckrow::RowError::Count => {
                err(StatusCode::BAD_REQUEST, "malformed card count")
            }
            baylee_core::deckrow::RowError::Finish => {
                err(StatusCode::BAD_REQUEST, "unknown finish")
            }
            baylee_core::deckrow::RowError::Lang => {
                err(StatusCode::BAD_REQUEST, "unknown language")
            }
            baylee_core::deckrow::RowError::Shape => {
                err(StatusCode::BAD_REQUEST, "malformed card line")
            }
        })?;
        let count = row.count;
        let Some(index) = baylee_cards::decks::by_name(&row.name) else {
            return Err(err(StatusCode::BAD_REQUEST, "unknown card"));
        };

        let basic_land = baylee_cards::by_index(index).is_some_and(|def| {
            def.faces[0]
                .supertypes
                .contains(baylee_core::types::SupertypeSet::BASIC)
                && def.faces[0]
                    .types
                    .contains(baylee_core::types::TypeSet::LAND)
        });
        if count == 0 || (!basic_land && count > 4) {
            return Err(err(
                StatusCode::BAD_REQUEST,
                "invalid card count (1-4, unlimited for basic lands)",
            ));
        }
        total = total
            .checked_add(count)
            .ok_or_else(|| err(StatusCode::BAD_REQUEST, "deck too large"))?;
        if total > MAX_DECK_CARDS {
            return Err(err(StatusCode::BAD_REQUEST, "deck too large"));
        }
        out.push(ParsedLine {
            count,
            index,
            print: row.print,
        });
    }
    Ok(out)
}

fn validate_deck(body: &DeckBody) -> Result<(), (StatusCode, Json<ErrorBody>)> {
    if body.name.is_empty() || body.name.len() > 64 {
        return Err(err(StatusCode::BAD_REQUEST, "invalid deck name"));
    }
    if body.cards.is_empty() || body.cards.len() > 250 {
        return Err(err(StatusCode::BAD_REQUEST, "invalid card list"));
    }
    if body.sideboard.len() > 250 {
        return Err(err(StatusCode::BAD_REQUEST, "invalid sideboard"));
    }
    // The same parser, so a sideboard cannot hold what a deck could not.
    parse_deck_lines(&body.cards)?;
    parse_deck_lines(&body.sideboard)?;
    if let Some(c) = &body.commander
        && baylee_cards::decks::by_name(c).is_none()
    {
        return Err(err(StatusCode::BAD_REQUEST, "unknown commander"));
    }
    Ok(())
}

async fn list_decks(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    let store = state.store.lock();
    let decks: Vec<_> = store
        .decks
        .values()
        .filter(|d| d.account_id == account_id)
        .map(|d| {
            serde_json::json!({
                "id": d.id,
                "name": d.name,
                "cards": d.cards.len(),
                "sideboard": d.sideboard.len(),
                "commander": d.commander,
            })
        })
        .collect();
    Ok(Json(serde_json::json!(decks)))
}

async fn get_deck(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    let store = state.store.lock();
    let deck = store
        .decks
        .get(&id)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such deck"))?;
    if deck.account_id != account_id {
        return Err(err(StatusCode::FORBIDDEN, "not your deck"));
    }
    Ok(Json(serde_json::json!({
        "id": deck.id,
        "name": deck.name,
        "cards": deck.cards,
        "sideboard": deck.sideboard,
        "commander": deck.commander,
    })))
}

async fn create_deck(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<DeckBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    validate_deck(&body)?;
    let mut store = state.store.lock();
    let deck = Deck {
        id: auth::new_id(),
        account_id,
        name: body.name,
        cards: body.cards,
        sideboard: body.sideboard,
        commander: body.commander,
        updated_at: auth::now_secs(),
    };
    let id = deck.id.clone();
    store.decks.insert(id.clone(), deck);
    state.request_save();
    Ok(Json(serde_json::json!({ "deck_id": id })))
}

async fn update_deck(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<DeckBody>,
) -> Result<StatusCode, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    validate_deck(&body)?;
    let mut store = state.store.lock();
    let deck = store
        .decks
        .get_mut(&id)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such deck"))?;
    if deck.account_id != account_id {
        return Err(err(StatusCode::FORBIDDEN, "not your deck"));
    }
    deck.name = body.name;
    deck.cards = body.cards;
    deck.sideboard = body.sideboard;
    deck.commander = body.commander;
    deck.updated_at = auth::now_secs();
    state.request_save();
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_deck(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    let mut store = state.store.lock();
    match store.decks.get(&id) {
        Some(deck) if deck.account_id == account_id => {
            store.decks.remove(&id);
        }
        Some(_) => return Err(err(StatusCode::FORBIDDEN, "not your deck")),
        None => return Err(err(StatusCode::NOT_FOUND, "no such deck")),
    }
    state.request_save();
    Ok(StatusCode::NO_CONTENT)
}

// ------------------------------------------------------------------ lobby

/// The lobby listing as one account sees it.
///
/// Two locks, always in this order — store first, then lobby. Every other
/// path that needs both takes them the same way round.
fn listing(state: &Shared, account_id: &str) -> serde_json::Value {
    let wanted = state.lobby.lock().seated_accounts();
    let names: std::collections::HashMap<String, String> = {
        let store = state.store.lock();
        wanted
            .into_iter()
            .filter_map(|id| {
                store
                    .accounts
                    .get(&id)
                    .map(|a| (id, a.display_name.clone()))
            })
            .collect()
    };
    let lobby = state.lobby.lock();
    serde_json::json!(lobby.list_for(account_id, &names))
}

async fn list_games(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    Ok(Json(listing(&state, &account_id)))
}
#[derive(Deserialize)]
struct CreateGameBody {
    deck_id: String,
    /// `"ai"` is the one-tap game against the house AI and starts at once.
    /// Anything else opens a room the host configures.
    #[serde(default)]
    mode: String,
    /// How many chairs the table has. Two unless the host says otherwise.
    #[serde(default)]
    seats: Option<usize>,
    /// What the table is called in the list.
    #[serde(default)]
    name: String,
    /// A password for the room. Empty or absent leaves it open.
    #[serde(default)]
    password: String,
}

/// The most seats a room may have.
///
/// The most seats a room may have — the same eight `GamePreset::validate`
/// allows, so the gateway refuses exactly what the engine would.
const MAX_SEATS: usize = 8;

/// Builds a `LoadedDeck` from a stored deck.
/// Uses the same parser as validation, so counts are already bounded.
fn loaded_deck(
    deck: &Deck,
) -> Result<baylee_cards::decks::LoadedDeck, (StatusCode, Json<ErrorBody>)> {
    let main = expand(&parse_deck_lines(&deck.cards)?);
    let side = expand(&parse_deck_lines(&deck.sideboard)?);
    Ok(baylee_cards::decks::LoadedDeck {
        name: deck.name.clone(),
        main,
        sideboard: side,
        commanders: vec![],
    })
}

/// The deck an AI seat plays when the host did not give it one.
fn house_deck() -> Result<baylee_cards::decks::LoadedDeck, (StatusCode, Json<ErrorBody>)> {
    let text = std::fs::read_to_string("data/acceptance-decks.txt")
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "deck data missing"))?;
    baylee_cards::decks::load_acceptance(&text, "Victory")
        .map_err(|_e| err(StatusCode::INTERNAL_SERVER_ERROR, "house deck missing"))
}

/// Preset for a human-vs-AI game (house AI plays Victory).
fn ai_preset(
    deck: &Deck,
    seed: u64,
) -> Result<baylee_core::preset::GamePreset, (StatusCode, Json<ErrorBody>)> {
    let house = house_deck()?;
    let player = loaded_deck(deck)?;
    Ok(baylee_cards::decks::preset_for(seed, &player, &house))
}

/// Looks a deck up and checks it belongs to the account asking for it.
fn own_deck(
    state: &Shared,
    account_id: &str,
    deck_id: &str,
) -> Result<(String, Deck), (StatusCode, Json<ErrorBody>)> {
    let store = state.store.lock();
    let deck = store
        .decks
        .get(deck_id)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such deck"))?;
    if deck.account_id != account_id {
        return Err(err(StatusCode::FORBIDDEN, "not your deck"));
    }
    Ok((deck.name.clone(), deck.clone()))
}

/// Builds the preset a room's seats add up to.
///
/// The controller is set per seat here and nowhere else: the deck helper
/// presets every chair as an AI, which is right for the print table and wrong
/// for everyone at the table who is a person.
fn room_preset(
    seats: &[lobby::LobbySeat],
    seed: u64,
) -> Result<baylee_core::preset::GamePreset, (StatusCode, Json<ErrorBody>)> {
    let mut loaded = Vec::with_capacity(seats.len());
    for seat in seats {
        match &seat.deck {
            Some(deck) => loaded.push(loaded_deck(deck)?),
            // Only reachable for an AI seat the host left alone; a human seat
            // with no deck is not ready and the room has not started.
            None => loaded.push(house_deck()?),
        }
    }
    let refs: Vec<&baylee_cards::decks::LoadedDeck> = loaded.iter().collect();
    let mut preset = baylee_cards::decks::preset_for_all(seed, &refs);
    for (spec, seat) in preset.seats.iter_mut().zip(seats) {
        spec.controller = match seat.kind {
            // `Open` rather than `Human`: the gateway knows the account, the
            // engine knows only that a person answers for this chair.
            lobby::SeatKind::Human => baylee_core::preset::SeatController::Open,
            lobby::SeatKind::Ai => baylee_core::preset::SeatController::Ai(
                seat.ai
                    .as_deref()
                    .and_then(baylee_core::preset::AIProfile::named)
                    .unwrap_or_default(),
            ),
        };
    }
    Ok(preset)
}

/// Starts a room whose seats are all settled.
///
/// A room used to start itself the moment the last chair became ready, which
/// read well until "ready" stopped meaning "has a deck": a player who picked
/// a deck to look at it was already in a game. Now every chair says it is
/// ready and the host says go, which is two different statements by two
/// different people and needs both.
///
/// Returns whether the game was started. The engine is ordered *outside* the
/// lobby lock, and a failure to order one puts the room back the way it was
/// rather than leaving a table nobody can play at.
fn try_start(state: &Shared, id: &str) -> Result<bool, (StatusCode, Json<ErrorBody>)> {
    {
        let mut lobby = state.lobby.lock();
        let Some(game) = lobby.games.get_mut(id) else {
            return Ok(false);
        };
        if game.state != LobbyState::Waiting || !game.seats.iter().all(lobby::LobbySeat::ready) {
            return Ok(false);
        }
        game.preset = Some(room_preset(&game.seats, auth::new_game_seed())?);
        game.state = LobbyState::Playing;
    }
    if let Err(reason) = engine::start_engine(state, id) {
        let mut lobby = state.lobby.lock();
        if let Some(game) = lobby.games.get_mut(id) {
            game.state = LobbyState::Waiting;
            game.preset = None;
        }
        tracing::error!(game_id = id, reason, "could not start a game");
        return Err(err(StatusCode::SERVICE_UNAVAILABLE, "no engine available"));
    }
    Ok(true)
}

async fn create_game(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<CreateGameBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    let (deck_name, deck) = own_deck(&state, &account_id, &body.deck_id)?;
    let game_id = auth::new_id();
    let seat_token = auth::new_token();

    // The one-tap game against the house AI keeps its own path: it is a whole
    // table decided in one request, and nobody is going to configure it.
    if body.mode == "ai" {
        {
            let mut preset = ai_preset(&deck, auth::new_game_seed())?;
            preset.seats[0].controller = baylee_core::preset::SeatController::Open;
            let mut seats = vec![lobby::LobbySeat::open(0), lobby::LobbySeat::open(1)];
            seats[0].account_id = Some(account_id.clone());
            seats[0].seat_token_hash = Some(auth::token_hash(&seat_token));
            seats[0].deck_name = deck_name;
            seats[0].deck = Some(deck.clone());
            seats[1].kind = lobby::SeatKind::Ai;
            seats[1].ai = Some("steady".to_string());
            seats[1].deck_name = "house AI".to_string();
            let game = LobbyGame::playing(game_id.clone(), seats, preset, auth::now_secs());
            state.lobby.lock().games.insert(game_id.clone(), game);
        }
        if let Err(reason) = engine::start_engine(&state, &game_id) {
            state.lobby.lock().games.remove(&game_id);
            tracing::error!(game_id, reason, "could not start a game");
            return Err(err(StatusCode::SERVICE_UNAVAILABLE, "no engine available"));
        }
        return Ok(Json(serde_json::json!({
            "game_id": game_id,
            "seat": 0,
            "seat_token": seat_token,
        })));
    }

    let chairs = body.seats.unwrap_or(2);
    if !(2..=MAX_SEATS).contains(&chairs) {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "a table seats between two and eight",
        ));
    }
    {
        let mut lobby = state.lobby.lock();
        let mut game = LobbyGame::room(
            game_id.clone(),
            account_id,
            deck_name,
            deck,
            chairs,
            body.name.chars().take(60).collect(),
            auth::now_secs(),
        );
        game.seats[0].seat_token_hash = Some(auth::token_hash(&seat_token));
        if !body.password.is_empty() {
            game.password_hash = Some(auth::token_hash(&body.password));
        }
        lobby.games.insert(game_id.clone(), game);
    }
    Ok(Json(serde_json::json!({
        "game_id": game_id,
        "seat": 0,
        "seat_token": seat_token,
    })))
}

#[derive(Deserialize)]
struct JoinGameBody {
    deck_id: String,
    /// Which chair to take. The first free one when the body does not say.
    #[serde(default)]
    seat: Option<usize>,
    /// The room's password, for a room that has one.
    #[serde(default)]
    password: Option<String>,
}

async fn join_game(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<JoinGameBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    let (deck_name, deck) = own_deck(&state, &account_id, &body.deck_id)?;
    let seat_token = auth::new_token();
    let seat = {
        let mut lobby = state.lobby.lock();
        let game = lobby
            .games
            .get_mut(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        if game.state != LobbyState::Waiting {
            return Err(err(StatusCode::CONFLICT, "game already started"));
        }
        // Before anything else is checked, and answered the same way whether
        // the password was wrong or missing: a locked room should not tell a
        // stranger how full it is.
        if let Some(want) = &game.password_hash {
            let given = body.password.as_deref().unwrap_or_default();
            if &auth::token_hash(given) != want {
                return Err(err(StatusCode::FORBIDDEN, "wrong password"));
            }
        }
        if game
            .seats
            .iter()
            .any(|s| s.account_id.as_ref() == Some(&account_id))
        {
            return Err(err(StatusCode::CONFLICT, "you are already at this table"));
        }
        let seq = game.claim_seq();
        let free =
            |s: &&mut lobby::LobbySeat| s.kind == lobby::SeatKind::Human && s.account_id.is_none();
        let chair = match body.seat {
            Some(at) => game
                .seats
                .iter_mut()
                .filter(free)
                .find(|s| s.seat == at)
                .ok_or_else(|| err(StatusCode::CONFLICT, "that seat is taken"))?,
            None => game
                .seats
                .iter_mut()
                .find(|s| free(s))
                .ok_or_else(|| err(StatusCode::CONFLICT, "the table is full"))?,
        };
        chair.account_id = Some(account_id);
        chair.seat_token_hash = Some(auth::token_hash(&seat_token));
        chair.deck_name = deck_name;
        chair.deck = Some(deck);
        chair.joined_seq = Some(seq);
        chair.seat
    };
    Ok(Json(serde_json::json!({
        "game_id": id,
        "seat": seat,
        "seat_token": seat_token,
    })))
}

#[derive(Deserialize)]
struct SeatBody {
    /// `"human"` or `"ai"`. Absent leaves the chair as it is.
    #[serde(default)]
    kind: Option<String>,
    /// Which difficulty an AI chair plays at.
    #[serde(default)]
    ai: Option<String>,
    /// The deck this chair plays.
    #[serde(default)]
    deck_id: Option<String>,
}

/// Configures one seat of a room.
///
/// Two authorities, deliberately narrow. The **host** arranges the table:
/// which chairs are people and which are the AI, how hard the AI plays, and
/// what an AI chair brings. A **player** changes exactly one thing, their own
/// deck — including the host, whose own chair is theirs as a player and not
/// as the host.
async fn set_seat(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path((id, seat)): Path<(String, usize)>,
    Json(body): Json<SeatBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    // Looked up before the lobby lock: the store has its own, and taking two
    // in one order here and the other order elsewhere is how deadlocks start.
    let chosen = match &body.deck_id {
        Some(deck_id) => Some(own_deck(&state, &account_id, deck_id)?),
        None => None,
    };
    {
        let mut lobby = state.lobby.lock();
        let game = lobby
            .games
            .get_mut(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        if game.state != LobbyState::Waiting {
            return Err(err(StatusCode::CONFLICT, "game already started"));
        }
        let is_host = game.host.as_ref() == Some(&account_id);
        let chair = game
            .seats
            .get_mut(seat)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such seat"))?;
        let is_mine = chair.account_id.as_ref() == Some(&account_id);
        if !is_mine && !is_host {
            return Err(err(StatusCode::FORBIDDEN, "not your seat"));
        }
        // The host arranges chairs, but never out from under someone else who
        // is sitting in one.
        if let Some(kind) = &body.kind {
            if !is_host {
                return Err(err(StatusCode::FORBIDDEN, "only the host arranges seats"));
            }
            if chair.account_id.is_some() && !is_mine {
                return Err(err(StatusCode::CONFLICT, "someone is sitting there"));
            }
            match kind.as_str() {
                "ai" => {
                    let deck = chair.deck.take();
                    let deck_name = std::mem::take(&mut chair.deck_name);
                    let profile = chair.ai.take();
                    chair.vacate();
                    chair.kind = lobby::SeatKind::Ai;
                    chair.ai = Some(profile.unwrap_or_else(|| "steady".to_string()));
                    chair.deck = deck;
                    chair.deck_name = if chair.deck.is_some() {
                        deck_name
                    } else {
                        "house AI".to_string()
                    };
                }
                "human" => {
                    // An AI's deck was the host's choice for a chair that is
                    // now waiting for a person, who brings their own.
                    if chair.account_id.is_none() {
                        chair.vacate();
                    } else {
                        chair.kind = lobby::SeatKind::Human;
                        chair.ai = None;
                    }
                }
                _ => return Err(err(StatusCode::BAD_REQUEST, "kind must be human or ai")),
            }
        }
        if let Some(profile) = &body.ai {
            if !is_host {
                return Err(err(StatusCode::FORBIDDEN, "only the host arranges seats"));
            }
            if baylee_core::preset::AIProfile::named(profile).is_none() {
                return Err(err(StatusCode::BAD_REQUEST, "no such AI"));
            }
            if chair.kind != lobby::SeatKind::Ai {
                return Err(err(StatusCode::CONFLICT, "that seat is not an AI"));
            }
            chair.ai = Some(profile.clone());
        }
        if let Some((deck_name, deck)) = chosen {
            // A player sets their own deck; the host sets an AI's. Nobody
            // sets a deck for another person.
            let allowed = is_mine || (is_host && chair.kind == lobby::SeatKind::Ai);
            if !allowed {
                return Err(err(StatusCode::FORBIDDEN, "not your seat"));
            }
            chair.deck_name = deck_name;
            chair.deck = Some(deck);
            // A deck they have not seen yet is not a deck they said yes to.
            // Without this the host could swap an opponent into a game they
            // had already declared themselves ready for.
            chair.said_ready = false;
        }
    }
    Ok(Json(listing(&state, &account_id)))
}

#[derive(Deserialize)]
struct ReadyBody {
    /// Absent means ready; a client withdraws by sending `false`.
    #[serde(default = "yes")]
    ready: bool,
}

/// `serde` default for [`ReadyBody::ready`].
const fn yes() -> bool {
    true
}

/// Says whether the caller is ready to play.
///
/// Only ever about the caller's own chair — the host arranges the table, but
/// nobody declares anyone else ready.
async fn set_ready(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<ReadyBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    {
        let mut lobby = state.lobby.lock();
        let game = lobby
            .games
            .get_mut(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        if game.state != LobbyState::Waiting {
            return Err(err(StatusCode::CONFLICT, "game already started"));
        }
        let chair = game
            .seats
            .iter_mut()
            .find(|s| s.account_id.as_ref() == Some(&account_id))
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "you are not at this table"))?;
        if body.ready && chair.deck.is_none() {
            return Err(err(StatusCode::CONFLICT, "pick a deck first"));
        }
        chair.said_ready = body.ready;
    }
    Ok(Json(listing(&state, &account_id)))
}

/// Starts the room. The host's call, and only once every chair is ready.
async fn start_room(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    {
        let lobby = state.lobby.lock();
        let game = lobby
            .games
            .get(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        if !game.hosted_by(&account_id) {
            return Err(err(StatusCode::FORBIDDEN, "only the host starts the game"));
        }
        if game.state != LobbyState::Waiting {
            return Err(err(StatusCode::CONFLICT, "game already started"));
        }
        if !game.seats.iter().all(lobby::LobbySeat::ready) {
            return Err(err(StatusCode::CONFLICT, "not everyone is ready"));
        }
    }
    // Outside the lock it took to check: `try_start` takes it again, and
    // ordering the engine must not happen underneath it.
    if !try_start(&state, &id)? {
        return Err(err(StatusCode::CONFLICT, "the room is no longer ready"));
    }
    Ok(Json(listing(&state, &account_id)))
}

#[derive(Deserialize)]
struct HostBody {
    /// The chair to hand the room to.
    seat: usize,
}

/// Hands the room to another player.
///
/// By seat rather than by name or account: the caller is looking at a
/// listing of chairs, and a seat index is the one handle in it that cannot
/// be ambiguous when two people share a display name.
async fn hand_over(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<HostBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    {
        let mut lobby = state.lobby.lock();
        let game = lobby
            .games
            .get_mut(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        if !game.hosted_by(&account_id) {
            return Err(err(StatusCode::FORBIDDEN, "not your room"));
        }
        if game.state != LobbyState::Waiting {
            return Err(err(StatusCode::CONFLICT, "game already started"));
        }
        let chair = game
            .seats
            .get(body.seat)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such seat"))?;
        let Some(new_host) = chair.account_id.clone() else {
            return Err(err(StatusCode::CONFLICT, "nobody is sitting there"));
        };
        game.host = Some(new_host);
    }
    Ok(Json(listing(&state, &account_id)))
}

/// Gives up a seat, handing the room on if the host is the one leaving.
async fn leave_game(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    let mut lobby = state.lobby.lock();
    let game = lobby
        .games
        .get_mut(&id)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
    if game.state != LobbyState::Waiting {
        return Err(err(StatusCode::CONFLICT, "game already started"));
    }
    let Some(chair) = game
        .seats
        .iter_mut()
        .find(|s| s.account_id.as_ref() == Some(&account_id))
    else {
        return Err(err(StatusCode::NOT_FOUND, "you are not at this table"));
    };
    chair.vacate();
    // A room outlives its host: it passes to whoever has been here longest,
    // and only a room with nobody left in it is closed. The earlier version
    // closed it the moment the host stood up, which threw everyone else out
    // of a table they were sitting at.
    if game.hosted_by(&account_id) && !game.hand_over_host() {
        game.finish(auth::now_secs());
    }
    Ok(StatusCode::NO_CONTENT)
}

// ------------------------------------------------------- standing answers

/// Upper bound on remembered answers per account. Generous next to any real
/// card pool, and small enough that a caller cannot grow the store with one
/// request.
const MAX_STANDING_ANSWERS: usize = 512;

#[derive(Deserialize)]
struct AutomationBody {
    answers: Vec<store::StandingAnswer>,
}

/// The account's remembered answers.
async fn list_automation(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    let store = state.store.lock();
    let answers = store
        .automation
        .get(&account_id)
        .cloned()
        .unwrap_or_default();
    Ok(Json(serde_json::json!({ "answers": answers })))
}

/// Replaces the account's remembered answers.
///
/// References are validated against the card registry here rather than
/// trusted: an answer for a card that does not exist could never fire, and
/// storing junk from a client is how a store becomes unreadable later.
async fn set_automation(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<AutomationBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    if body.answers.len() > MAX_STANDING_ANSWERS {
        return Err(err(StatusCode::BAD_REQUEST, "too many remembered answers"));
    }
    for a in &body.answers {
        if baylee_cards::by_index(baylee_core::ids::CardIndex::new(a.card)).is_none() {
            return Err(err(StatusCode::BAD_REQUEST, "unknown card"));
        }
    }
    let mut answers = body.answers;
    // One answer per ability, in a stable order: the engine keeps its own
    // sorted list, and a duplicate would mean the stored preference and the
    // engine's disagree about which one won.
    answers.sort_by_key(|a| (a.card, a.ability));
    answers.dedup_by_key(|a| (a.card, a.ability));
    let count = answers.len();
    state.store.lock().automation.insert(account_id, answers);
    state.request_save();
    Ok(Json(serde_json::json!({ "stored": count })))
}

/// Upper bound on a stored preferences blob.
///
/// A full keymap with every action bound twice, both phase rails and the
/// automation flags is under two kilobytes; sixteen leaves room for whatever
/// the client learns to remember next, and still means a thousand accounts
/// cost the store sixteen megabytes at the very worst.
const MAX_SETTINGS_BYTES: usize = 16 * 1024;

/// The account's client preferences, verbatim as they were stored.
///
/// `{}` for an account that has never saved any, which is the same thing the
/// client would do with them: fall back to its own defaults.
async fn get_settings(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    let store = state.store.lock();
    let settings = store
        .settings
        .get(&account_id)
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    Ok(Json(settings))
}

/// Replaces the account's client preferences.
///
/// The body *is* the preferences object — there is no wrapper, because there
/// is nothing else to say about it. The gateway checks only the two things it
/// can check without knowing what a keymap is: that this is an object, and
/// that it is not being used as free storage.
async fn put_settings(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    if !body.is_object() {
        return Err(err(StatusCode::BAD_REQUEST, "settings must be an object"));
    }
    let bytes = serde_json::to_string(&body).map_or(usize::MAX, |s| s.len());
    if bytes > MAX_SETTINGS_BYTES {
        return Err(err(StatusCode::PAYLOAD_TOO_LARGE, "settings too large"));
    }
    state.store.lock().settings.insert(account_id, body);
    state.request_save();
    Ok(Json(serde_json::json!({ "stored": bytes })))
}

/// The account's remembered answers, in the shape the engine reads them.
///
/// The gateway cannot build a `PlayerAction` — it does not link the engine —
/// so what travels is the stored preference itself, and the engine turns it
/// back into the handle it keeps its automation under. That handle is the one
/// thing here that can silently be wrong: a wrong handle never fires, with no
/// error and no log, and the seat is simply asked a question it believed it
/// had answered for good.
fn standing_payload(answers: &[store::StandingAnswer]) -> Vec<u8> {
    let wire: Vec<baylee_protocol::StandingAnswer> = answers
        .iter()
        .map(|a| baylee_protocol::StandingAnswer {
            card: a.card,
            ability: a.ability,
            yes: a.yes,
        })
        .collect();
    serde_json::to_vec(&wire).unwrap_or_else(|_| b"[]".to_vec())
}

/// What a seat's account has remembered, ready to hand to the engine.
fn standing_for_seat(state: &Shared, game_id: &str, seat: usize) -> Vec<u8> {
    let account_id = {
        let lobby = state.lobby.lock();
        lobby
            .games
            .get(game_id)
            .and_then(|g| g.seats.iter().find(|s| s.seat == seat))
            .and_then(|s| s.account_id.clone())
    };
    let Some(account_id) = account_id else {
        return b"[]".to_vec();
    };
    let answers = state
        .store
        .lock()
        .automation
        .get(&account_id)
        .cloned()
        .unwrap_or_default();
    standing_payload(&answers)
}

// ---------------------------------------------------------------- game ws

#[derive(Deserialize)]
struct WsParams {
    token: String,
}

async fn game_ws(
    State(state): State<Shared>,
    Path(id): Path<String>,
    Query(params): Query<WsParams>,
    ws: WebSocketUpgrade,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorBody>)> {
    let seat = {
        let lobby = state.lobby.lock();
        let game = lobby
            .games
            .get(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        let token_hash = auth::token_hash(&params.token);
        game.seats
            .iter()
            .find(|s| {
                s.seat_token_hash
                    .as_ref()
                    .is_some_and(|h| auth::ct_eq(h, &token_hash))
            })
            .map(|s| s.seat)
            .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "invalid seat token"))?
    };
    Ok(ws.on_upgrade(move |socket| run_game_socket(state, id, seat, socket)))
}

/// The names shown at each seat of a game, in seat order.
///
/// The rules kernel has never heard of an account, so the roster is assembled
/// here and handed to the engine with the preset. The two locks are taken one
/// after the other rather than nested: the lobby says which account sits
/// where, the store says what that account is called, and nothing in between
/// needs both at once.
fn seat_names(state: &Shared, game_id: &str) -> Vec<String> {
    let accounts: Vec<Option<String>> = {
        let lobby = state.lobby.lock();
        match lobby.games.get(game_id) {
            Some(game) => game.seats.iter().map(|s| s.account_id.clone()).collect(),
            None => return Vec::new(),
        }
    };
    let store = state.store.lock();
    accounts
        .into_iter()
        .map(|id| match id {
            Some(id) => store.accounts.get(&id).map_or_else(
                || "Unknown player".to_string(),
                |account| account.display_name.clone(),
            ),
            // An empty chair in a running game is the house playing it.
            None => "House AI".to_string(),
        })
        .collect()
}

/// How long a seat socket waits for its game's engine to attach.
///
/// A seat may open its socket the moment the lobby says "playing", which is
/// before the agent has finished starting the process. Generous, because the
/// alternative is a client that has to poll and guess.
const ENGINE_WAIT_SECS: u64 = 30;

/// The biggest frame a seat may send.
///
/// A player's frame is a `PlayerActionMsg` carrying a JSON action — hundreds
/// of bytes at most. The gateway forwards these without decoding them, so
/// this is the only bound on what one seat can make the engine read.
const MAX_SEAT_FRAME: usize = 64 * 1024;

/// Waits until the game's engine is attached.
async fn engine_ready(ready: &mut tokio::sync::watch::Receiver<bool>) -> bool {
    let wait = async {
        loop {
            if *ready.borrow_and_update() {
                return true;
            }
            if ready.changed().await.is_err() {
                return false;
            }
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(ENGINE_WAIT_SECS), wait)
        .await
        .unwrap_or(false)
}

/// Sends one frame to a game's engine. False when there is no engine to send
/// to, which is the end of this socket.
fn to_engine(state: &Shared, game_id: &str, msg: v1::envelope::Msg) -> bool {
    let lobby = state.lobby.lock();
    lobby.games.get(game_id).is_some_and(|game| {
        game.engine
            .as_ref()
            .is_some_and(|tx| tx.send(Envelope { msg: Some(msg) }).is_ok())
    })
}

/// One seat's socket: everything it says goes to the engine tagged with its
/// seat, and everything the engine addresses to that seat comes back.
///
/// The gateway never decodes either direction. It cannot: it does not link the
/// rules kernel, and the whole point of the engine plane is that it does not
/// have to.
async fn run_game_socket(state: Shared, game_id: String, seat: usize, mut socket: WebSocket) {
    // Subscribe BEFORE announcing the seat, so this socket cannot miss its
    // own first view; every envelope addressed to this seat arrives here,
    // including the ones produced by the opponent's actions.
    let (mut rx, mut ready) = {
        let lobby = state.lobby.lock();
        let Some(game) = lobby.games.get(&game_id) else {
            return;
        };
        (game.updates.subscribe(), game.ready.subscribe())
    };
    if !engine_ready(&mut ready).await {
        tracing::warn!(game_id, seat, "no engine attached; seat socket closing");
        return;
    }
    let attach = v1::envelope::Msg::SeatAttached(v1::SeatAttached {
        seat: seat as u32,
        standing_json: standing_for_seat(&state, &game_id, seat),
        resync: false,
    });
    if !to_engine(&state, &game_id, attach) {
        return;
    }
    loop {
        tokio::select! {
            frame = socket.recv() => {
                match frame {
                    Some(Ok(Message::Binary(data))) => {
                        if data.len() > MAX_SEAT_FRAME {
                            tracing::warn!(game_id, seat, len = data.len(), "oversized seat frame");
                            continue;
                        }
                        let tagged = v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                            seat: seat as u32,
                            envelope: data.into(),
                        });
                        if !to_engine(&state, &game_id, tagged) {
                            break;
                        }
                    }
                    // Pings are answered by axum; ignore other frame kinds.
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => break,
                }
            }
            update = rx.recv() => {
                match update {
                    Ok((p, bytes)) => {
                        if p == seat as u8
                            && futures_util::SinkExt::send(&mut socket, Message::Binary(bytes.into()))
                                .await
                                .is_err()
                        {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        // Dropping the player was the old answer. Now that a
                        // seat's whole state can be rebuilt on demand, ask for
                        // it instead: the gap in the stream stops mattering.
                        tracing::warn!(game_id, seat, n, "seat socket lagged; resyncing");
                        let resync = v1::envelope::Msg::SeatAttached(v1::SeatAttached {
                            seat: seat as u32,
                            standing_json: b"[]".to_vec(),
                            resync: true,
                        });
                        if !to_engine(&state, &game_id, resync) {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
    // The engine runs a decision clock only for a seat that can answer, so it
    // has to be told when one walks away.
    to_engine(
        &state,
        &game_id,
        v1::envelope::Msg::SeatDetached(v1::SeatDetached { seat: seat as u32 }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_engine::choice::{PlayerAction, StandingAnswer};

    fn ondu_cleric() -> u32 {
        baylee_cards::by_oracle_id("f4232466-dd6a-49bf-be6c-95905c3ded17")
            .expect("the card pool has Ondu Cleric")
            .index
            .get()
    }

    /// A stored answer has to arrive as the exact handle the engine keeps its
    /// automation under. The two ends of that are now in different processes,
    /// so the test drives both: what the gateway sends, read by the code the
    /// engine reads it with.
    #[test]
    fn a_stored_answer_becomes_the_handle_the_engine_uses() {
        let stored = vec![
            store::StandingAnswer {
                card: ondu_cleric(),
                ability: 0,
                yes: true,
            },
            store::StandingAnswer {
                card: ondu_cleric(),
                ability: baylee_core::ids::AbilityRef::ENTERS,
                yes: false,
            },
        ];
        let actions = baylee_engine_server::standing_answers(&standing_payload(&stored));
        assert_eq!(
            actions,
            vec![
                PlayerAction::SetStandingAnswer {
                    ability: baylee_core::ids::AbilityRef::new(
                        baylee_core::ids::CardIndex::new(ondu_cleric()),
                        0
                    ),
                    answer: Some(StandingAnswer::Yes),
                },
                PlayerAction::SetStandingAnswer {
                    ability: baylee_core::ids::AbilityRef::new(
                        baylee_core::ids::CardIndex::new(ondu_cleric()),
                        baylee_core::ids::AbilityRef::ENTERS
                    ),
                    answer: Some(StandingAnswer::No),
                },
            ]
        );
    }

    /// The reserved indices address abilities that are not listed on the
    /// card, so a round trip through the store and over the engine link must
    /// not confuse them with ability 0.
    #[test]
    fn reserved_ability_handles_survive_the_store() {
        let stored = vec![store::StandingAnswer {
            card: ondu_cleric(),
            ability: baylee_core::ids::AbilityRef::MIRACLE,
            yes: true,
        }];
        let json = serde_json::to_string(&stored).expect("serializes");
        let back: Vec<store::StandingAnswer> = serde_json::from_str(&json).expect("round trips");
        assert_eq!(back, stored);
        let actions = baylee_engine_server::standing_answers(&standing_payload(&back));
        let PlayerAction::SetStandingAnswer { ability, .. } = &actions[0] else {
            panic!("expected a standing answer")
        };
        assert!(!ability.is_listed_ability());
    }

    /// A seat with no account — the house playing an empty chair — has no
    /// remembered answers, and must not send the engine something it cannot
    /// parse in place of that.
    #[test]
    fn no_answers_is_an_empty_list_the_engine_can_read() {
        assert!(baylee_engine_server::standing_answers(&standing_payload(&[])).is_empty());
        assert!(baylee_engine_server::standing_answers(b"[]").is_empty());
    }

    /// A store written before standing answers existed still loads.
    #[test]
    fn an_older_store_file_still_loads() {
        let old = r#"{"accounts":{},"tokens":{},"decks":{}}"#;
        let store: store::Store = serde_json::from_str(old).expect("older store loads");
        assert!(store.automation.is_empty());
    }
}
