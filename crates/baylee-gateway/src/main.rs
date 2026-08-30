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
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use axum::routing::{get, post};
use baylee_protocol::v1::{self, Envelope};
use lobby::{Lobby, LobbyGame, LobbyState};
use prost::Message as _;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use store::{Account, Deck, Store, StoredToken};

/// Shared gateway state.
struct AppState {
    store: Mutex<Store>,
    limiter: auth::RateLimiter,
    lobby: Mutex<Lobby>,
    store_path: PathBuf,
    /// Registration toggle (`BAYLEE_REGISTRATION=off` to disable).
    registration_enabled: bool,
}

type Shared = Arc<AppState>;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(28766);
    let store_path = std::env::var("STORE_PATH")
        .map_or_else(|_| PathBuf::from("gateway-store.json"), PathBuf::from);
    let state = Arc::new(AppState {
        store: Mutex::new(Store::load(&store_path)),
        limiter: auth::RateLimiter::new(std::time::Duration::from_secs(300), 10),
        lobby: Mutex::new(Lobby::default()),
        store_path,
        registration_enabled: std::env::var("BAYLEE_REGISTRATION")
            .map_or(true, |v| !matches!(v.as_str(), "off" | "0" | "false")),
    });

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
    axum::serve(listener, app).await.expect("gateway serves");
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

fn client_ip(headers: &HeaderMap) -> String {
    // Behind a TLS-terminating proxy, the real client ip is forwarded;
    // never trusted for auth decisions, only for rate limiting.
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .unwrap_or("unknown")
        .trim()
        .to_string()
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
    headers: HeaderMap,
    Json(body): Json<RegisterBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    if !state.registration_enabled {
        return Err(err(StatusCode::FORBIDDEN, "registration is disabled"));
    }
    if !state.limiter.allow(&client_ip(&headers)) {
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
    let mut store = state.store.lock().expect("store poisoned");
    // Anti-enumeration: identical success response whether or not the
    // e-mail/display name was free. Only on genuine availability is an
    // account actually created. (E-mail verification would hook in here.)
    let email_taken = store.account_by_email(&body.email).is_some();
    let name_taken = store.account_by_display_name(&body.display_name).is_some();
    if !email_taken && !name_taken {
        let email = body.email.to_lowercase();
        let account = Account {
            id: auth::new_id(),
            email,
            display_name: body.display_name,
            password_hash: auth::hash_password(&body.password),
            created_at: auth::now_secs(),
        };
        let id = account.id.clone();
        store.accounts.insert(id, account);
        let _ = store.save(&state.store_path);
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn login(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(creds): Json<Credentials>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    if !state.limiter.allow(&client_ip(&headers)) {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many attempts"));
    }
    let mut store = state.store.lock().expect("store poisoned");
    // Identical work for unknown users (dummy verify) and real ones.
    let (account_id, ok) = {
        let account = store.account_by_email(&creds.email);
        let ok = auth::verify_password(account.map(|a| a.password_hash.as_str()), &creds.password);
        (account.map(|a| a.id.clone()), ok)
    };
    let Some(account_id) = account_id else {
        return Err(err(StatusCode::UNAUTHORIZED, "invalid credentials"));
    };
    if !ok {
        return Err(err(StatusCode::UNAUTHORIZED, "invalid credentials"));
    }
    let issued = auth::IssuedToken::new();
    store.tokens.insert(
        auth::token_hash(&issued.token),
        StoredToken {
            token_hash: auth::token_hash(&issued.token),
            account_id,
            expires_at: issued.expires_at,
        },
    );
    let _ = store.save(&state.store_path);
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
    let mut store = state.store.lock().expect("store poisoned");
    let account_id = store
        .resolve_token(token, auth::now_secs())
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "invalid or expired token"))?;
    let _ = store.save(&state.store_path);
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
    let mut store = state.store.lock().expect("store poisoned");
    store.tokens.remove(&auth::token_hash(token));
    let _ = store.save(&state.store_path);
    Ok(StatusCode::NO_CONTENT)
}

/// The authenticated account's profile.
async fn me(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    let store = state.store.lock().expect("store poisoned");
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
        let Some(index) = baylee_ai::decks::by_name(name.trim()) else {
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
        && baylee_ai::decks::by_name(c).is_none()
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
    let store = state.store.lock().expect("store poisoned");
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
    let store = state.store.lock().expect("store poisoned");
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
    let mut store = state.store.lock().expect("store poisoned");
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
    let _ = store.save(&state.store_path);
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
    let mut store = state.store.lock().expect("store poisoned");
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
    let _ = store.save(&state.store_path);
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_deck(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    let mut store = state.store.lock().expect("store poisoned");
    match store.decks.get(&id) {
        Some(deck) if deck.account_id == account_id => {
            store.decks.remove(&id);
        }
        Some(_) => return Err(err(StatusCode::FORBIDDEN, "not your deck")),
        None => return Err(err(StatusCode::NOT_FOUND, "no such deck")),
    }
    let _ = store.save(&state.store_path);
    Ok(StatusCode::NO_CONTENT)
}

// ------------------------------------------------------------------ lobby

async fn list_games(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let _ = authed(&state, &headers)?;
    let lobby = state.lobby.lock().expect("lobby poisoned");
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
fn loaded_deck(deck: &Deck) -> Result<baylee_ai::decks::LoadedDeck, (StatusCode, Json<ErrorBody>)> {
    let parsed = parse_deck_lines(&deck.cards)?;
    let mut main = Vec::new();
    for line in parsed {
        for _ in 0..line.count {
            main.push(line.index);
        }
    }
    Ok(baylee_ai::decks::LoadedDeck {
        name: deck.name.clone(),
        main,
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
    Ok(baylee_ai::decks::preset_for(seed, &da, &db))
}

/// Preset for a human-vs-AI game (house AI plays Victory).
fn ai_preset(
    deck: &Deck,
    seed: u64,
) -> Result<baylee_core::preset::GamePreset, (StatusCode, Json<ErrorBody>)> {
    let text = std::fs::read_to_string("data/acceptance-decks.txt")
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "deck data missing"))?;
    let house = baylee_ai::decks::load_acceptance(&text, "Victory")
        .map_err(|_e| err(StatusCode::INTERNAL_SERVER_ERROR, "house deck missing"))?;
    let player = loaded_deck(deck)?;
    Ok(baylee_ai::decks::preset_for(seed, &player, &house))
}

async fn create_game(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<CreateGameBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers)?;
    let (deck_name, deck) = {
        let store = state.store.lock().expect("store poisoned");
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
    let mut lobby = state.lobby.lock().expect("lobby poisoned");
    match body.mode.as_str() {
        "ai" => {
            let mut preset = ai_preset(&deck, auth::new_game_seed())?;
            preset.seats[0].controller = baylee_core::preset::SeatController::Open;
            let session = baylee_gamehost::Session::new(&preset)
                .ok_or_else(|| err(StatusCode::INTERNAL_SERVER_ERROR, "game failed to start"))?;
            let game = LobbyGame {
                id: game_id.clone(),
                state: LobbyState::Playing,
                seats: vec![
                    lobby::LobbySeat {
                        seat: 0,
                        account_id: Some(account_id.clone()),
                        seat_token_hash: Some(auth::token_hash(&seat_token)),
                        deck_name,
                        deck: Some(deck.clone()),
                    },
                    lobby::LobbySeat {
                        seat: 1,
                        account_id: None,
                        seat_token_hash: None,
                        deck_name: "house AI".to_string(),
                        deck: None,
                    },
                ],
                preset: Some(preset),
                session: Some(session),
                created_at: auth::now_secs(),
            };
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
        let store = state.store.lock().expect("store poisoned");
        let deck = store
            .decks
            .get(&body.deck_id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such deck"))?;
        if deck.account_id != account_id {
            return Err(err(StatusCode::FORBIDDEN, "not your deck"));
        }
        (deck.name.clone(), deck.clone())
    };
    let mut lobby = state.lobby.lock().expect("lobby poisoned");
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
    let preset = hvh_preset(&creator_deck, &deck, auth::new_game_seed())?;
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
        let lobby = state.lobby.lock().expect("lobby poisoned");
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

async fn run_game_socket(state: Shared, game_id: String, seat: usize, mut socket: WebSocket) {
    let player = baylee_core::ids::PlayerId::new(seat as u8);
    // Initial pump: send everything addressed to this seat.
    let initial: Vec<Envelope> = {
        let mut lobby = state.lobby.lock().expect("lobby poisoned");
        let Some(game) = lobby.games.get_mut(&game_id) else {
            return;
        };
        let Some(session) = game.session.as_mut() else {
            return;
        };
        session
            .pump()
            .into_iter()
            .filter(|(p, _)| *p == player)
            .map(|(_, env)| env)
            .collect()
    };
    for env in initial {
        if send_envelope(&mut socket, env).await.is_err() {
            return;
        }
    }
    while let Some(Ok(frame)) = futures_util::StreamExt::next(&mut socket).await {
        let Message::Binary(data) = frame else {
            continue;
        };
        let Ok(envelope) = Envelope::decode(data) else {
            continue;
        };
        let Some(v1::envelope::Msg::PlayerAction(action_msg)) = envelope.msg else {
            continue;
        };
        let Ok(action) =
            serde_json::from_slice::<baylee_engine::choice::PlayerAction>(&action_msg.action_json)
        else {
            continue;
        };
        let replies: Vec<Envelope> = {
            let mut lobby = state.lobby.lock().expect("lobby poisoned");
            let Some(game) = lobby.games.get_mut(&game_id) else {
                return;
            };
            let Some(session) = game.session.as_mut() else {
                return;
            };
            match session.act(player, action) {
                Ok(routed) => routed
                    .into_iter()
                    .filter(|(p, _)| *p == player)
                    .map(|(_, env)| env)
                    .collect(),
                Err(_) => continue,
            }
        };
        for env in replies {
            if send_envelope(&mut socket, env).await.is_err() {
                return;
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
