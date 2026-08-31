//! baylee-gateway — accounts, decks, lobby, and hosted games.
//!
//! Security posture (see auth.rs): Argon2id password hashing, hashed
//! bearer tokens with sliding expiry, auth rate limiting, generic
//! credential errors, constant-time comparisons. TLS terminates at the
//! reverse proxy in front of this process (Caddy/nginx) — this service
//! must never be exposed on a plaintext listener in production.

mod auth;
mod lobby;
mod store;

use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use axum::routing::{get, post};
use baylee_engine::choice::Pending;
use baylee_protocol::v1::{self, Envelope};
use lobby::{Lobby, LobbyGame, LobbySeat, LobbyState};
use parking_lot::Mutex;
use prost::Message as _;
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
    let state = Arc::new(AppState {
        store: Mutex::new(Store::load(&store_path)),
        limiter: auth::RateLimiter::new(std::time::Duration::from_secs(300), 10),
        lobby: Mutex::new(Lobby::default()),
        store_path,
        save_tx,
        registration_enabled: std::env::var("BAYLEE_REGISTRATION")
            .map_or(true, |v| !matches!(v.as_str(), "off" | "0" | "false")),
        trusted_proxies: std::env::var("BAYLEE_TRUSTED_PROXIES")
            .unwrap_or_default()
            .split(',')
            .filter_map(|s| s.trim().parse::<IpAddr>().ok())
            .collect(),
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
        .route("/lobby/games/{id}/join", post(join_game))
        .route("/games/{id}/ws", get(game_ws))
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
                // Flip live sessions that reached game over.
                for game in lobby.games.values_mut() {
                    if game.state == LobbyState::Playing
                        && game
                            .session
                            .as_ref()
                            .is_some_and(|s| matches!(s.pending(), Pending::GameOver(_)))
                    {
                        game.state = LobbyState::Over;
                        game.finished_at = Some(now);
                    }
                }
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
    commander: Option<String>,
}

/// Hard cap on the expanded card count of one deck. Comfortably above
/// every legal format size (100 for commander), far below anything that
/// could strain memory at game start.
const MAX_DECK_CARDS: u32 = 250;

/// One parsed deck line: how many of which card.
struct ParsedLine {
    count: u32,
    index: baylee_core::ids::CardIndex,
}

/// Parses and validates "N Card Name" lines. Shared by `validate_deck` and
/// `loaded_deck` so a deck can never pass one and explode the other:
/// counts are parsed here (1–4, unlimited for basic lands) and the
/// expanded total is capped at [`MAX_DECK_CARDS`].
fn parse_deck_lines(lines: &[String]) -> Result<Vec<ParsedLine>, (StatusCode, Json<ErrorBody>)> {
    let mut out = Vec::with_capacity(lines.len());
    let mut total: u32 = 0;
    for line in lines {
        let Some((count, name)) = line.split_once(' ') else {
            return Err(err(StatusCode::BAD_REQUEST, "malformed card line"));
        };
        let Ok(count) = count.trim().parse::<u32>() else {
            return Err(err(StatusCode::BAD_REQUEST, "malformed card count"));
        };
        let Some(index) = baylee_cards::decks::by_name(name.trim()) else {
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
        out.push(ParsedLine { count, index });
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
    parse_deck_lines(&body.cards)?;
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

async fn list_games(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let _ = authed(&state, &headers)?;
    let lobby = state.lobby.lock();
    Ok(Json(serde_json::json!(lobby.list())))
}

#[derive(Deserialize)]
struct CreateGameBody {
    deck_id: String,
    /// "ai" or "open" (waiting for a second human).
    mode: String,
}

/// Builds a `LoadedDeck` from a stored deck (card lines "N Card Name").
/// Uses the same parser as validation, so counts are already bounded.
fn loaded_deck(
    deck: &Deck,
) -> Result<baylee_cards::decks::LoadedDeck, (StatusCode, Json<ErrorBody>)> {
    let parsed = parse_deck_lines(&deck.cards)?;
    let mut main = Vec::new();
    for line in parsed {
        for _ in 0..line.count {
            main.push(line.index);
        }
    }
    Ok(baylee_cards::decks::LoadedDeck {
        name: deck.name.clone(),
        main,
        // Stored decks are a flat card list; the gateway has no sideboard
        // section to parse yet.
        sideboard: vec![],
        commanders: vec![],
    })
}

/// Preset for a human-vs-human game from both seats' decks.
fn hvh_preset(
    a: &Deck,
    b: &Deck,
    seed: u64,
) -> Result<baylee_core::preset::GamePreset, (StatusCode, Json<ErrorBody>)> {
    let da = loaded_deck(a)?;
    let db = loaded_deck(b)?;
    Ok(baylee_cards::decks::preset_for(seed, &da, &db))
}

/// Preset for a human-vs-AI game (house AI plays Victory).
fn ai_preset(
    deck: &Deck,
    seed: u64,
) -> Result<baylee_core::preset::GamePreset, (StatusCode, Json<ErrorBody>)> {
    let text = std::fs::read_to_string("data/acceptance-decks.txt")
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "deck data missing"))?;
    let house = baylee_cards::decks::load_acceptance(&text, "Victory")
        .map_err(|_e| err(StatusCode::INTERNAL_SERVER_ERROR, "house deck missing"))?;
    let player = loaded_deck(deck)?;
    Ok(baylee_cards::decks::preset_for(seed, &player, &house))
}

async fn create_game(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<CreateGameBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    let (deck_name, deck) = {
        let store = state.store.lock();
        let deck = store
            .decks
            .get(&body.deck_id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such deck"))?;
        if deck.account_id != account_id {
            return Err(err(StatusCode::FORBIDDEN, "not your deck"));
        }
        (deck.name.clone(), deck.clone())
    };
    let game_id = auth::new_id();
    let seat_token = auth::new_token();
    let mut lobby = state.lobby.lock();
    match body.mode.as_str() {
        "ai" => {
            let mut preset = ai_preset(&deck, auth::new_game_seed())?;
            preset.seats[0].controller = baylee_core::preset::SeatController::Open;
            let session = baylee_gamehost::Session::new(&preset)
                .ok_or_else(|| err(StatusCode::INTERNAL_SERVER_ERROR, "game failed to start"))?;
            let game = LobbyGame::playing(
                game_id.clone(),
                vec![
                    LobbySeat {
                        seat: 0,
                        account_id: Some(account_id.clone()),
                        seat_token_hash: Some(auth::token_hash(&seat_token)),
                        deck_name,
                        deck: Some(deck.clone()),
                    },
                    LobbySeat {
                        seat: 1,
                        account_id: None,
                        seat_token_hash: None,
                        deck_name: "house AI".to_string(),
                        deck: None,
                    },
                ],
                preset,
                session,
                auth::now_secs(),
            );
            lobby.games.insert(game_id.clone(), game);
        }
        "open" => {
            let mut game = LobbyGame::waiting(
                game_id.clone(),
                account_id,
                deck_name,
                deck.clone(),
                auth::now_secs(),
            );
            game.seats[0].seat_token_hash = Some(auth::token_hash(&seat_token));
            lobby.games.insert(game_id.clone(), game);
        }
        _ => return Err(err(StatusCode::BAD_REQUEST, "mode must be ai or open")),
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
}

async fn join_game(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<JoinGameBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    let (deck_name, deck) = {
        let store = state.store.lock();
        let deck = store
            .decks
            .get(&body.deck_id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such deck"))?;
        if deck.account_id != account_id {
            return Err(err(StatusCode::FORBIDDEN, "not your deck"));
        }
        (deck.name.clone(), deck.clone())
    };
    let mut lobby = state.lobby.lock();
    let game = lobby
        .games
        .get_mut(&id)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
    if game.state != LobbyState::Waiting {
        return Err(err(StatusCode::CONFLICT, "game already started"));
    }
    let seat_token = auth::new_token();
    game.seats[1] = lobby::LobbySeat {
        seat: 1,
        account_id: Some(account_id),
        seat_token_hash: Some(auth::token_hash(&seat_token)),
        deck_name,
        deck: Some(deck.clone()),
    };
    // Both seats filled: build the preset from BOTH players' decks.
    let creator_deck = game.seats[0]
        .deck
        .clone()
        .ok_or_else(|| err(StatusCode::INTERNAL_SERVER_ERROR, "creator deck missing"))?;
    let mut preset = hvh_preset(&creator_deck, &deck, auth::new_game_seed())?;
    // The deck helper presets every seat as AI; here BOTH chairs belong
    // to humans, or the game would silently play itself (and the humans
    // would never be asked).
    for seat in &mut preset.seats {
        seat.controller = baylee_core::preset::SeatController::Open;
    }
    game.preset = Some(preset.clone());
    game.session = Some(
        baylee_gamehost::Session::new(&preset)
            .ok_or_else(|| err(StatusCode::INTERNAL_SERVER_ERROR, "game failed to start"))?,
    );
    game.state = LobbyState::Playing;
    Ok(Json(serde_json::json!({
        "game_id": id,
        "seat": 1,
        "seat_token": seat_token,
    })))
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

/// Pumps (or acts) the session under a panic boundary and broadcasts
/// every routed envelope to all seat subscribers. A panicking rules path
/// kills ONE game (marked over) instead of the process.
///
/// Returns false when the game is gone or panicked — the socket closes.
fn drive_session<F>(state: &Shared, game_id: &str, step: F) -> bool
where
    F: FnOnce(
        &mut baylee_gamehost::Session,
        &mut dyn FnMut(baylee_core::ids::PlayerId, Envelope),
    ) -> Result<(), String>,
{
    let mut lobby = state.lobby.lock();
    let Some(game) = lobby.games.get_mut(game_id) else {
        return false;
    };
    let Some(session) = game.session.as_mut() else {
        return false;
    };
    let updates = game.updates.clone();
    let mut emit = |p: baylee_core::ids::PlayerId, env: Envelope| {
        // No receivers is fine (AI seats, seats without a live socket).
        let _ = updates.send((p.get(), env));
    };
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| step(session, &mut emit)));
    match result {
        Ok(Ok(())) => {
            if matches!(session.pending(), Pending::GameOver(_)) {
                game.state = LobbyState::Over;
                game.finished_at = Some(auth::now_secs());
            }
            true
        }
        Ok(Err(_)) => true, // illegal action etc. — the game lives on
        Err(_) => {
            tracing::error!(game_id, "rules engine panicked; game closed");
            game.state = LobbyState::Over;
            game.finished_at = Some(auth::now_secs());
            false
        }
    }
}

/// Applies one decoded frame from a seat's socket.
///
/// Returns false when the game is gone and the socket should close.
fn handle_client_frame(
    state: &Shared,
    game_id: &str,
    player: baylee_core::ids::PlayerId,
    envelope: Envelope,
) -> bool {
    match envelope.msg {
        Some(v1::envelope::Msg::PlayerAction(action_msg)) => {
            let Ok(action) = serde_json::from_slice::<baylee_engine::choice::PlayerAction>(
                &action_msg.action_json,
            ) else {
                return true;
            };
            drive_session(state, game_id, |session, emit| {
                let routed = session.act(player, action)?;
                for (p, env) in routed {
                    emit(p, env);
                }
                Ok(())
            })
        }
        // A reconnecting seat asks for whatever it missed. Read-only, so the
        // seats still playing see nothing and no AI seat is driven forward as
        // a side effect of somebody reconnecting.
        Some(v1::envelope::Msg::Resume(resume)) => {
            drive_session(state, game_id, |session, emit| {
                for env in session.resume(player, resume.last_seq) {
                    emit(player, env);
                }
                Ok(())
            })
        }
        _ => true,
    }
}

/// Re-arms the seat's decision deadline when the game has moved on.
///
/// The deadline is anchored to the session's sequence number, so it restarts
/// when the game actually advances rather than every time an envelope
/// addressed to the opponent wakes this task up.
fn refresh_decision_clock(
    state: &Shared,
    game_id: &str,
    player: baylee_core::ids::PlayerId,
    timeout_secs: u32,
    deadline: &mut Option<tokio::time::Instant>,
    clocked_seq: &mut Option<u64>,
) {
    let lobby = state.lobby.lock();
    match lobby.games.get(game_id).and_then(|g| g.session.as_ref()) {
        Some(session) if session.awaiting_seat() == Some(player) => {
            if *clocked_seq != Some(session.seq()) {
                *clocked_seq = Some(session.seq());
                *deadline = (timeout_secs > 0).then(|| {
                    tokio::time::Instant::now()
                        + std::time::Duration::from_secs(u64::from(timeout_secs))
                });
            }
        }
        _ => {
            *deadline = None;
            *clocked_seq = None;
        }
    }
}

/// Waits for a seat's decision deadline; waits forever when the seat is not
/// the one being asked, or the table sets no limit.
async fn decision_clock(deadline: Option<tokio::time::Instant>) {
    match deadline {
        Some(at) => tokio::time::sleep_until(at).await,
        None => std::future::pending().await,
    }
}

async fn run_game_socket(state: Shared, game_id: String, seat: usize, mut socket: WebSocket) {
    let player = baylee_core::ids::PlayerId::new(seat as u8);
    // Subscribe BEFORE the initial pump so this socket can't miss its own
    // first view; every envelope addressed to this seat arrives here,
    // including the ones produced by the opponent's actions.
    let mut rx = {
        let lobby = state.lobby.lock();
        let Some(game) = lobby.games.get(&game_id) else {
            return;
        };
        game.updates.subscribe()
    };
    if !drive_session(&state, &game_id, |session, emit| {
        for (p, env) in session.pump() {
            emit(p, env);
        }
        Ok(())
    }) {
        return;
    }
    // The decision clock lives here, never in the engine or the session: the
    // rules kernel is deterministic and must not read a wall clock. Each
    // socket runs only its own seat's clock, so the seats of a duel cannot
    // both fire the same timeout.
    let timeout_secs = {
        let lobby = state.lobby.lock();
        lobby
            .games
            .get(&game_id)
            .and_then(|g| g.session.as_ref())
            .map_or(0, baylee_gamehost::Session::decision_timeout_secs)
    };
    // The deadline is anchored to the sequence number it was set for, so it
    // restarts when the game actually moves — not every time an envelope
    // addressed to the opponent happens to wake this task up.
    let mut deadline: Option<tokio::time::Instant> = None;
    let mut clocked_seq: Option<u64> = None;
    loop {
        refresh_decision_clock(
            &state,
            &game_id,
            player,
            timeout_secs,
            &mut deadline,
            &mut clocked_seq,
        );
        tokio::select! {
            () = decision_clock(deadline) => {
                // Out of time. The house agent answers for the seat, which is
                // guaranteed to be legal for whatever was asked.
                tracing::info!(game_id, seat, "decision timed out; answering for the seat");
                let alive = drive_session(&state, &game_id, |session, emit| {
                    let Some((p, action)) = session.timeout_action() else {
                        return Ok(());
                    };
                    // Re-check under the lock. The deadline was armed while
                    // this seat owed the answer, but the opponent may have
                    // acted between the timer firing and this closure
                    // running — and answering for whoever is being asked
                    // *now* would let one seat's expired clock take another
                    // seat's decision.
                    if p != player {
                        return Ok(());
                    }
                    for (q, env) in session.act(p, action)? {
                        emit(q, env);
                    }
                    Ok(())
                });
                if !alive {
                    return;
                }
            }
            frame = futures_util::StreamExt::next(&mut socket) => {
                match frame {
                    Some(Ok(Message::Binary(data))) => {
                        let Ok(envelope) = Envelope::decode(data) else {
                            continue;
                        };
                        if !handle_client_frame(&state, &game_id, player, envelope) {
                            return;
                        }
                    }
                    // Pings are answered by axum; ignore other frame kinds.
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => return,
                }
            }
            update = rx.recv() => {
                match update {
                    Ok((p, env)) => {
                        if p == player.get() && send_envelope(&mut socket, env).await.is_err() {
                            return;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        // Dropping the player was the old answer. Now that a
                        // seat's whole state can be rebuilt on demand, resend
                        // it instead: the gap in the stream stops mattering.
                        tracing::warn!(game_id, seat, n, "seat socket lagged; resyncing");
                        let snapshot = {
                            let lobby = state.lobby.lock();
                            match lobby.games.get(&game_id).and_then(|g| g.session.as_ref()) {
                                Some(session) => session.snapshot(player),
                                None => return,
                            }
                        };
                        for env in snapshot {
                            if send_envelope(&mut socket, env).await.is_err() {
                                return;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
                }
            }
        }
    }
}

async fn send_envelope(socket: &mut WebSocket, env: Envelope) -> Result<(), ()> {
    let bytes = env.encode_to_vec();
    futures_util::SinkExt::send(socket, Message::Binary(bytes.into()))
        .await
        .map_err(|_| ())
}
