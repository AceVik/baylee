//! baylee-gateway — accounts, decks, lobby, and hosted games.
//!
//! Security posture (see auth.rs): Argon2id password hashing, hashed
//! bearer tokens with sliding expiry, auth rate limiting, generic
//! credential errors, constant-time comparisons. TLS terminates at the
//! reverse proxy in front of this process (Caddy/nginx) — this service
//! must never be exposed on a plaintext listener in production.

mod art;
mod auth;
mod cosmetics;
mod engine;
mod handle;
mod lobby;
mod mail;
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
use store::{Confirmation, Deck, StoredToken};
use tracing_subscriber::EnvFilter;

/// Shared gateway state.
struct AppState {
    /// Accounts, sessions, decks, links, standing answers, preferences.
    ///
    /// `store.rs` is the only module that knows this is a database; every
    /// route here calls a function there and gets a plain struct back.
    db: sea_orm::DatabaseConnection,
    limiter: auth::RateLimiter,
    /// Sign-in attempts, counted **per address** rather than per IP.
    ///
    /// A separate limiter because it answers a different question. Guessing
    /// is aimed at one account, so the address is what the count belongs to;
    /// an IP is only a proxy for it, and a bad proxy in both directions. It
    /// punishes a household or an office behind one address for a neighbour's
    /// typing — which is how the owner locked themselves out of a development
    /// box where every request, mine included, arrives from `127.0.0.1` — and
    /// it stops nobody with more than one address to send from.
    ///
    /// The cost is stated rather than avoided: a stranger who knows an
    /// address can spend its eight attempts and make its owner wait out the
    /// window. That is a nuisance bounded by five minutes, against guessing
    /// bounded by nothing, and the register and resend routes keep their IP
    /// limit so account spam and mail amplification stay bounded by it.
    sign_in_limiter: auth::RateLimiter,
    lobby: Mutex<Lobby>,
    /// Registration toggle (`BAYLEE_REGISTRATION=off` to disable).
    registration_enabled: bool,
    /// Where confirmation mail goes, and — because it is the same
    /// question — whether an address has to be confirmed at all.
    mail: mail::Mailer,
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
    /// Fires whenever anything in the lobby changed, so `/lobby/ws` can push
    /// instead of every client asking every two seconds.
    ///
    /// It carries no payload on purpose. What a change *means* differs per
    /// account — whose room it is, which chair is theirs, what they searched
    /// for — so the socket re-renders the listing for its own reader rather
    /// than trying to broadcast one answer to everybody.
    lobby_changed: tokio::sync::broadcast::Sender<()>,
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
    /// The disk mirror of card art (`BAYLEE_ART_PATH`, `off` to disable).
    ///
    /// An `Arc` of its own because the warming task outlives the request that
    /// started it: a table's pictures keep arriving while the room is still
    /// being arranged. See `art.rs` for why this is the layer that may warm
    /// every deck at a table and a client is not.
    art: Arc<art::ArtCache>,
    /// Uploaded deck sleeves and playmats
    /// (`BAYLEE_DECK_IMAGE_PATH`, `off` to disable).
    ///
    /// An `Arc` because every upload and every read is handed to
    /// `spawn_blocking`: decoding a photograph on the runtime's thread
    /// would stall every socket the gateway is holding.
    deck_images: Arc<cosmetics::Store>,
}

impl AppState {
    /// Tell every open `/lobby/ws` that the lobby moved.
    ///
    /// Deliberately fire-and-forget and deliberately not inside the lobby
    /// lock: a send that nobody is listening to is an error this does not
    /// care about, and holding a mutex while waking sockets is how a lobby
    /// route ends up waiting on a slow client.
    pub(crate) fn lobby_moved(&self) {
        let _ = self.lobby_changed.send(());
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
    validate_the_dev_board();
    let store_path = std::env::var("STORE_PATH")
        .map_or_else(|_| PathBuf::from("gateway-store.json"), PathBuf::from);
    let db = open_database(&store_path).await;
    let catalog = connect_catalog(db.clone()).await;
    let agent_token = std::env::var("BAYLEE_AGENT_TOKEN")
        .ok()
        .filter(|t| !t.is_empty());
    if agent_token.is_none() {
        tracing::warn!("BAYLEE_AGENT_TOKEN is not set: no agent can connect, so no game can start");
    }
    let state = Arc::new(AppState {
        db,
        limiter: auth::RateLimiter::new(std::time::Duration::from_secs(300), 10),
        sign_in_limiter: auth::RateLimiter::new(std::time::Duration::from_secs(300), 8),
        lobby: Mutex::new(Lobby::default()),
        // Small: a receiver that falls this far behind is one whose socket is
        // not draining, and the lag is handled by sending it the current
        // listing rather than by replaying what it missed.
        lobby_changed: tokio::sync::broadcast::channel(16).0,
        agents: Mutex::new(engine::Agents::default()),
        agent_token,
        engine_url: std::env::var("BAYLEE_ENGINE_URL")
            .unwrap_or_else(|_| format!("ws://127.0.0.1:{port}/engine/ws")),
        registration_enabled: std::env::var("BAYLEE_REGISTRATION")
            .map_or(true, |v| !matches!(v.as_str(), "off" | "0" | "false")),
        mail: mail::Mailer::from_env(),
        trusted_proxies: std::env::var("BAYLEE_TRUSTED_PROXIES")
            .unwrap_or_default()
            .split(',')
            .filter_map(|s| s.trim().parse::<IpAddr>().ok())
            .collect(),
        catalog,
        art: Arc::new(art::ArtCache::from_env()),
        deck_images: Arc::new(cosmetics::Store::from_env()),
    });
    spawn_cleanup(state.clone());

    let app = Router::new()
        .route("/source", get(source))
        .route("/auth/config", get(auth_config))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/confirm", get(confirm))
        .route("/auth/confirm/resend", post(resend_confirmation))
        .route("/auth/logout", post(logout))
        .route("/me", get(me))
        .route("/players/{handle}", get(player))
        .route("/decks", get(list_decks).post(create_deck))
        .route(
            "/decks/{id}",
            get(get_deck).put(update_deck).delete(delete_deck),
        )
        .route("/lobby/games", get(list_games).post(create_game))
        .route("/automation", get(list_automation).put(set_automation))
        .route("/settings", get(get_settings).put(put_settings))
        .route("/lobby/games/{id}/join", post(join_game))
        .route("/lobby/games/{id}/seat", post(take_seat))
        .route("/lobby/games/{id}/seats/{seat}", post(set_seat))
        .route("/lobby/games/{id}/ready", post(set_ready))
        .route("/lobby/games/{id}/start", post(start_room))
        .route("/lobby/games/{id}/host", post(hand_over))
        .route("/lobby/games/{id}/leave", post(leave_game))
        .route("/lobby/games/{id}/rematch", post(rematch))
        .route("/lobby/ws", get(lobby_ws))
        .route("/games/{id}/ws", get(game_ws))
        .route("/games/{id}/cosmetics", get(game_cosmetics))
        // The control and engine planes. Neither carries a player's traffic
        // and neither accepts a player's token; see `engine.rs`.
        .route("/agent/ws", get(engine::agent_ws))
        .route("/engine/ws", get(engine::engine_ws))
        .route("/pool", get(pool::pool))
        .route("/printings", get(pool::printings))
        .route("/catalog/text", get(catalog_text))
        .route("/catalog/search", get(catalog_search))
        // Public artwork, mirrored from Scryfall by printing id. Unauthenticated
        // like the CDN it stands in for, and an id cache rather than a proxy —
        // see `art.rs`.
        .route("/art/{size}/{face}/{a}/{b}/{file}", get(art::art))
        // Axum caps a body at 2 MB by default, which is under a phone
        // photograph. The cap that matters is the one in `cosmetics`,
        // checked again before anything is decoded.
        // The kind rides in the query rather than the path, and not by
        // preference: axum registers a route by its path before it looks at
        // the method, so `/images/{kind}` and `/images/{file}` are the same
        // route wearing two different parameter names, and building the
        // router panics. It panicked at startup, which meant the gateway
        // never bound its port and every end-to-end test in the crate failed
        // with "connection refused" — a message that says nothing about
        // routing. `/images` is the collection and `/images/{file}` is one
        // member of it, which is the shape that had no conflict to resolve.
        .route(
            "/images",
            post(cosmetics::upload).layer(axum::extract::DefaultBodyLimit::max(
                cosmetics::MAX_UPLOAD_BYTES,
            )),
        )
        .route("/images/{id}", get(cosmetics::serve))
        .layer(axum::middleware::from_fn(cors))
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

/// Open the one connection pool the gateway has, and take over a store file
/// if this is the first start against an empty database.
///
/// # Why this is fatal and the catalog is not
///
/// A gateway with no card catalog serves no card text and plays every game;
/// a gateway with no database has no accounts, so there is nothing it can do
/// but refuse, and refusing at startup is the only place it can do so
/// honestly. The alternative — answering every request with a 503 — is a
/// process that looks healthy to anything watching it.
async fn open_database(store_path: &std::path::Path) -> sea_orm::DatabaseConnection {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| {
            panic!(
                "DATABASE_URL is not set; the gateway keeps its accounts in PostgreSQL.\n  \
                 docker compose up -d\n  \
                 export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee"
            )
        });
    // Small by default and settable, because the number that binds is at the
    // other end: a stock PostgreSQL allows 100 connections in total, and a
    // test suite that spawns three dozen gateways has to fit inside that.
    let pool = std::env::var("BAYLEE_DB_POOL")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(baylee_db::DEFAULT_POOL);

    let db = match baylee_db::connect(&url, pool).await {
        Ok(db) => db,
        Err(e) => panic!("{e:#}"),
    };
    tracing::info!(url = %baylee_db::redacted(&url), pool, "database ready");

    match baylee_db::import::import_file(&db, store_path).await {
        Ok(Some(done)) => {
            tracing::info!(
                accounts = done.accounts,
                decks = done.decks,
                sessions = done.tokens,
                orphans = done.orphans,
                "imported {}",
                store_path.display()
            );
            // Moved, never deleted: until the database has been backed up
            // once, this file is the only copy of somebody's account.
            let aside = baylee_db::import::imported_name(store_path);
            if let Err(e) = std::fs::rename(store_path, &aside) {
                tracing::warn!(%e, "could not move {} aside", store_path.display());
            }
        }
        Ok(None) => {}
        Err(e) => tracing::error!("{e:#}"),
    }

    db
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
                // A finished game whose rematch room is still waiting has to
                // outlive its grace period: the pointer to that room is the
                // only thing that says it exists, and reaping it would send
                // the player who takes a moment longer to press the button
                // into a second room. Collected first, because `retain`
                // cannot look a game up in the map it is emptying.
                let awaited: std::collections::HashSet<String> = lobby
                    .games
                    .values()
                    .filter(|g| g.state == LobbyState::Waiting)
                    .filter_map(|g| g.parent.clone())
                    .collect();
                // A game reaching its end is no longer something to discover
                // here: the engine says so on its own link, and losing that
                // link closes the game too. All that is left is reclaiming.
                lobby.games.retain(|_, g| match g.state {
                    LobbyState::Waiting => now.saturating_sub(g.created_at) < WAITING_TIMEOUT_SECS,
                    LobbyState::Over => {
                        awaited.contains(&g.id)
                            || g.finished_at
                                .is_none_or(|t| now.saturating_sub(t) < OVER_GRACE_SECS)
                    }
                    LobbyState::Playing => true,
                });
            }
            // One `DELETE ... WHERE expires_at <= $1` on the index the
            // migration made for it, where this used to walk every session
            // the gateway had ever issued.
            match store::sweep_tokens(&state.db, now).await {
                Ok(purged) if purged > 0 => tracing::debug!(purged, "lapsed sessions swept"),
                Ok(_) => {}
                Err(e) => tracing::warn!("{e:#}"),
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
    /// The language to write to this account in. Defaulted, because a client
    /// written before this field existed still registers.
    #[serde(default)]
    lang: String,
}

/// What `POST /auth/confirm/resend` takes.
#[derive(Deserialize)]
struct ResendBody {
    email: String,
}

/// What `GET /auth/confirm` takes.
#[derive(Deserialize)]
struct ConfirmQuery {
    token: String,
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
/// What this gateway is, and where the source for exactly this build lives.
///
/// The AGPL's §13 obliges a program modified and offered to users over a
/// network to offer those users its Corresponding Source, and a gateway is
/// the one process here that meets that description. Unauthenticated on
/// purpose: an offer conditional on having an account is not an offer to the
/// people §13 is about. It names the commit rather than only the repository,
/// because "the source is on GitHub" does not say *which* source — a build
/// running a patch nobody published would answer that sentence truthfully
/// and still be hiding what it runs.
async fn source() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "baylee",
        "license": "AGPL-3.0-only",
        "version": baylee_build::short(),
        "commit": baylee_build::COMMIT,
        "build": baylee_build::BUILD_NUMBER,
        "built_at": baylee_build::BUILT_AT,
        "source": baylee_build::REPOSITORY,
        // Stated rather than implied: a reader who finds `dirty` true knows
        // the commit above does not fully describe what is running, which is
        // the one case where the offer would otherwise mislead.
        "dirty": baylee_build::DIRTY,
    }))
}

async fn auth_config(State(state): State<Shared>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "registration_enabled": state.registration_enabled,
        "confirmation_required": state.mail.required(),
        // Whether `GET /art/…` mirrors card images. A client that pointed at a
        // gateway with the mirror switched off would get a 404 for every card
        // and draw a whole table of constructed faces, so it is told here
        // rather than discovering it one blank card at a time.
        "art_cache": state.art.enabled(),
        "deck_images": state.deck_images.enabled(),
    }))
}

/// How long a confirmation link is good for.
const CONFIRM_TTL_SECS: u64 = 24 * 3600;

/// Mints a confirmation link for an account and mails it.
///
/// Returns without sending on a gateway with no mailer, which is the whole
/// reason the account was already marked confirmed by then: an unconfigured
/// gateway must behave exactly as it did before any of this existed.
async fn mail_confirmation(state: &Shared, account_id: &str) {
    if !state.mail.required() {
        return;
    }
    let issued = auth::IssuedToken::new();
    let now = auth::now_secs();
    // The old link stops working before the new one is written: a resend
    // that left both alive would be two working logins in one mailbox.
    if let Err(e) = store::clear_confirmations(&state.db, account_id, now).await {
        tracing::error!("{e:#}");
        return;
    }
    let Ok(Some(account)) = store::account(&state.db, account_id).await else {
        return;
    };
    let link = Confirmation {
        token_hash: auth::token_digest(&issued.token),
        account_id: account_id.to_string(),
        expires_at: now + CONFIRM_TTL_SECS,
    };
    if let Err(e) = store::put_confirmation(&state.db, link).await {
        tracing::error!("{e:#}");
        return;
    }
    state
        .mail
        .send_confirmation(
            &account.email,
            &account.display_name,
            &account.lang,
            &issued.token,
        )
        .await;
}

/// The link in the mail.
///
/// Plain text rather than a redirect into the client: the gateway does not
/// know where a client is served from — a native build is not served at all —
/// and a redirect target read off a header is how an open redirect happens.
async fn confirm(
    State(state): State<Shared>,
    Query(query): Query<ConfirmQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let now = auth::now_secs();
    // Read and removed in one step, so a link followed twice works once —
    // including when the second request is a mail client prefetching it.
    let found = store::take_confirmation(&state.db, &auth::token_digest(&query.token))
        .await
        .map_err(|e| db_down(&e))?;
    let Some(found) = found else {
        return Err(err(StatusCode::BAD_REQUEST, "that link is not valid"));
    };
    if found.expires_at <= now {
        return Err(err(StatusCode::BAD_REQUEST, "that link has expired"));
    }
    store::confirm_account(&state.db, &found.account_id, now)
        .await
        .map_err(|e| db_down(&e))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// Sends the link again.
///
/// Answers `{"ok":true}` whatever happened, for the same reason registration
/// does: a route that said "no such account" would be an address oracle, and
/// this one needs no password to call.
async fn resend_confirmation(
    State(state): State<Shared>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<ResendBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    if !state
        .limiter
        .allow(&rate_limit_ip(&state, addr.ip(), &headers))
    {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many attempts"));
    }
    let account_id = store::account_by_email(&state.db, &body.email)
        .await
        .map_err(|e| db_down(&e))?
        .filter(|a| a.confirmed_at.is_none())
        .map(|a| a.id);
    if let Some(account_id) = account_id {
        mail_confirmation(&state, &account_id).await;
    }
    Ok(Json(serde_json::json!({ "ok": true })))
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
    // not the e-mail was free — hashing always (~100 ms), so timing can't
    // tell "taken" (fast reject) from "created". Argon2 is deliberately
    // expensive and runs off the async worker.
    //
    // A display name is no longer among the things that can be taken, which
    // makes this strictly stronger than it was: a refusal used to be able to
    // mean "that name exists", and the only thing it can mean now is that
    // the address does.
    let password = body.password.clone();
    let password_hash = tokio::task::spawn_blocking(move || auth::hash_password(&password))
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "hashing failed"))?;
    let now = auth::now_secs();
    let account = store::NewAccount {
        email: body.email.to_lowercase(),
        display_name: body.display_name,
        password_hash,
        created_at: now,
        // A gateway that sends no mail confirms on the spot. That is
        // not a weaker rule than it looks: it is the same rule, asked
        // of a gateway that never asked the question.
        confirmed_at: (!state.mail.required()).then_some(now),
        lang: body.lang,
    };
    // The refusal is the unique index's, not a check's. Reading "is this
    // address free" and then writing is two statements another registration
    // can slip between, and both of them would have read "free".
    //
    // What comes back is the row the database made: the tag is its to hand
    // out, so the account only exists in full once it has been written.
    let created = store::create_account(&state.db, account)
        .await
        .map_err(|e| db_down(&e))?;
    if let Some(made) = &created {
        mail_confirmation(&state, &made.id).await;
    }
    // Anti-enumeration again, and the reason the mail is sent before the
    // answer rather than after it: the answer is the same either way, so it
    // must not be *timed* differently either.
    Ok(Json(
        serde_json::json!({ "ok": true, "confirmation_required": state.mail.required() }),
    ))
}

async fn login(
    State(state): State<Shared>,
    Json(creds): Json<Credentials>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    // Eight tries at **this address**, not at this machine. Lower-cased for
    // the key because `Store::account_by_email` matches that way, so two
    // spellings of one address are one account and have to be one count.
    if !state
        .sign_in_limiter
        .allow(&creds.email.to_ascii_lowercase())
    {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many attempts"));
    }
    // Fetch the hash under a short lock; the expensive verify runs off
    // the async worker and outside the store lock.
    let account = store::account_by_email(&state.db, &creds.email)
        .await
        .map_err(|e| db_down(&e))?;
    let stored_hash = account.as_ref().map(|a| a.password_hash.clone());
    let account_id = account.as_ref().map(|a| a.id.clone());
    let confirmed_at = account.and_then(|a| a.confirmed_at);
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
    // Getting it right is what the window was counting towards. Leaving the
    // typos on the clock would refuse the next sign-in from a player who has
    // just proved who they are.
    state
        .sign_in_limiter
        .forget(&creds.email.to_ascii_lowercase());
    // Checked *after* the password, deliberately: answering "confirm your
    // e-mail first" to a wrong password would tell a stranger the address
    // exists, which is the one thing every other answer on this route is
    // careful not to say.
    if state.mail.required() && confirmed_at.is_none() {
        return Err(err(
            StatusCode::FORBIDDEN,
            "confirm your e-mail address first",
        ));
    }
    let issued = auth::IssuedToken::new();
    store::put_token(
        &state.db,
        StoredToken {
            token_hash: auth::token_digest(&issued.token),
            account_id,
            expires_at: issued.expires_at,
        },
    )
    .await
    .map_err(|e| db_down(&e))?;
    Ok(Json(serde_json::json!({
        "token": issued.token,
        "expires_at": issued.expires_at,
    })))
}

/// Resolves the bearer token to an account id.
///
/// One indexed read per authenticated request, and a write only once the
/// session is past its half-life — see `store::resolve_token` for why the
/// sliding expiry stopped sliding on every request.
async fn authed(
    state: &Shared,
    headers: &HeaderMap,
) -> Result<String, (StatusCode, Json<ErrorBody>)> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "missing bearer token"))?;
    store::resolve_token(&state.db, token, auth::now_secs())
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "invalid or expired token"))
}

/// A database that refused, as an answer a client can act on.
///
/// 503 rather than 500: the gateway itself is fine and the thing behind it
/// is not, and that difference is what tells a client to try again later
/// instead of to stop.
fn db_down(e: &anyhow::Error) -> (StatusCode, Json<ErrorBody>) {
    tracing::error!("{e:#}");
    err(
        StatusCode::SERVICE_UNAVAILABLE,
        "the gateway's database is unavailable",
    )
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
async fn connect_catalog(db: sea_orm::DatabaseConnection) -> Option<baylee_catalog::Catalog> {
    // The pool the gateway already opened, not a second one against the same
    // URL: two pools are twice the backend processes for no more concurrency,
    // and one `search_path` is what lets a test put the whole gateway in a
    // schema of its own.
    let catalog = baylee_catalog::Catalog::from_connection(db);
    if let Err(err) = catalog.migrate().await {
        // Not fatal, unlike the account tables. A gateway with no card
        // catalog serves no card text and plays every game; the client draws
        // faces from what the engine projects.
        tracing::error!(%err, "card catalog schema could not be applied");
        return None;
    }
    let count = catalog.count().await.unwrap_or(0);
    tracing::info!(printings = count, "card catalog connected");
    Some(catalog)
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
    store::drop_token(&state.db, token)
        .await
        .map_err(|e| db_down(&e))?;
    Ok(StatusCode::NO_CONTENT)
}

/// The authenticated account's profile.
async fn me(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let account = store::account(&state.db, &account_id)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "account gone"))?;
    Ok(Json(serde_json::json!({
        "id": account.id,
        "email": account.email,
        "display_name": account.display_name,
        // The two halves separately, because this is the one caller that
        // wants them apart: a settings screen shows the name in a field a
        // player can edit and the tag beside it as something they cannot.
        "tag": handle::tag_text(account.tag),
        "handle": handle::handle(&account.display_name, account.tag),
    })))
}

/// The account a handle names.
///
/// `GET /players/Alice%23af03`, or `GET /players/%23af03` when all that was
/// pasted was the tag. What comes back is what one player may know about
/// another: the id, the name and the tag — never the address.
///
/// Behind a session, because the tags are sequential and walking them would
/// otherwise list every account on the gateway to anyone who asked. A signed
/// -in player can still walk them, which is the cost of a tag a person can
/// type; it is the same cost a `BattleTag` has, and the reason this answers
/// nothing an opponent could not read off a lobby row.
async fn player(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(typed): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let _ = authed(&state, &headers).await?;
    let tag = handle::parse_tag(&typed)
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "a handle carries a #tag"))?;
    let found = store::account_by_tag(&state.db, tag)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such player"))?;
    Ok(Json(serde_json::json!({
        "id": found.id,
        "display_name": found.display_name,
        "tag": handle::tag_text(found.tag),
        "handle": handle::handle(&found.display_name, found.tag),
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
    /// Image id of the deck's sleeve, from `POST /images?kind=sleeve`.
    ///
    /// Not validated against the image store: a sleeve that is not there is a
    /// deck that draws the generated back, which is what a deck with no sleeve
    /// does anyway. Refusing the whole deck over a decoration would be a much
    /// worse failure than losing the decoration.
    #[serde(default)]
    sleeve: Option<String>,
    /// Image id of the deck's playmat.
    #[serde(default)]
    playmat: Option<String>,
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
///
/// The copy limit is counted **per card, not per row**. Once a printing became
/// part of a row's identity, `4 Lightning Bolt (LEA)` and `4 Lightning Bolt
/// (M10)` were two rows that each passed a per-row check — eight Bolts through
/// a route whose own error message says 1–4. The client's builder had it right
/// all along ("the copy limit is on the *card*", `deckbuilder/builder.rs`); it
/// was this side that counted the wrong thing, and being the enforcing side is
/// what made it a way to cheat rather than a display bug.
///
/// Per *list*, because this runs once for the deck and once for the sideboard
/// — which is the same split `DeckBuilder::add_print` applies.
fn parse_deck_lines(lines: &[String]) -> Result<Vec<ParsedLine>, (StatusCode, Json<ErrorBody>)> {
    let mut out = Vec::with_capacity(lines.len());
    let mut copies: std::collections::BTreeMap<u32, u32> = std::collections::BTreeMap::new();
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
        if count == 0 {
            return Err(err(
                StatusCode::BAD_REQUEST,
                "invalid card count (1-4, unlimited for basic lands)",
            ));
        }
        if !basic_land {
            let seen = copies.entry(index.get()).or_insert(0);
            *seen = seen.saturating_add(count);
            if *seen > 4 {
                return Err(err(
                    StatusCode::BAD_REQUEST,
                    "invalid card count (1-4, unlimited for basic lands)",
                ));
            }
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
    if let Some(c) = &body.commander {
        // Two questions, not one. The name has to resolve, *and* the card
        // it resolves to has to be allowed to lead a deck (CR 903.3): the
        // check used to stop at the first, so any card in the pool could be
        // named as a commander and the engine would seat it in the command
        // zone without complaint.
        let Some(index) = baylee_cards::decks::by_name(c) else {
            return Err(err(StatusCode::BAD_REQUEST, "unknown commander"));
        };
        let eligible = baylee_cards::by_index(index).is_some_and(|def| {
            !matches!(def.commander, baylee_cards_dsl::CommanderRule::NotEligible)
        });
        if !eligible {
            return Err(err(
                StatusCode::BAD_REQUEST,
                "that card cannot be a commander",
            ));
        }
    }
    Ok(())
}

async fn list_decks(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    // One indexed read on `(account_id, updated_at DESC)`, where this used
    // to walk every deck on the gateway to find one player's.
    let decks: Vec<_> = store::decks_of(&state.db, &account_id)
        .await
        .map_err(|e| db_down(&e))?
        .iter()
        .map(|d| {
            serde_json::json!({
                "id": d.id,
                "name": d.name,
                "cards": d.cards.len(),
                "sideboard": d.sideboard.len(),
                "commander": d.commander,
                "sleeve": d.sleeve,
                "playmat": d.playmat,
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
    let account_id = authed(&state, &headers).await?;
    let deck = store::deck(&state.db, &id)
        .await
        .map_err(|e| db_down(&e))?
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
        "sleeve": deck.sleeve,
        "playmat": deck.playmat,
    })))
}

async fn create_deck(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<DeckBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    validate_deck(&body)?;
    let id = store::create_deck(
        &state.db,
        store::NewDeck {
            account_id,
            name: body.name,
            cards: body.cards,
            sideboard: body.sideboard,
            commander: body.commander,
            sleeve: body.sleeve,
            playmat: body.playmat,
            updated_at: auth::now_secs(),
        },
    )
    .await
    .map_err(|e| db_down(&e))?
    .ok_or_else(|| err(StatusCode::INTERNAL_SERVER_ERROR, "account gone"))?;
    Ok(Json(serde_json::json!({ "deck_id": id })))
}

async fn update_deck(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<DeckBody>,
) -> Result<StatusCode, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    validate_deck(&body)?;
    let deck = store::deck(&state.db, &id)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such deck"))?;
    if deck.account_id != account_id {
        return Err(err(StatusCode::FORBIDDEN, "not your deck"));
    }
    store::put_deck(
        &state.db,
        Deck {
            name: body.name,
            cards: body.cards,
            sideboard: body.sideboard,
            commander: body.commander,
            sleeve: body.sleeve,
            playmat: body.playmat,
            updated_at: auth::now_secs(),
            ..deck
        },
    )
    .await
    .map_err(|e| db_down(&e))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_deck(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    // The ownership is inside the `DELETE`, so the row can only ever be
    // removed by the account that owns it. Telling "not yours" from "not
    // there" still takes a read, and the two answers are worth keeping
    // apart: one is a bug in the client and the other is a stale list.
    if store::delete_deck(&state.db, &id, &account_id)
        .await
        .map_err(|e| db_down(&e))?
    {
        return Ok(StatusCode::NO_CONTENT);
    }
    match store::deck(&state.db, &id).await.map_err(|e| db_down(&e))? {
        Some(_) => Err(err(StatusCode::FORBIDDEN, "not your deck")),
        None => Err(err(StatusCode::NOT_FOUND, "no such deck")),
    }
}

// ------------------------------------------------------------------ lobby

/// The lobby listing as one account sees it, whole.
///
/// What the mutating routes answer with: a player who just arranged a chair
/// is looking at one room, not at a page of the lobby, and handing them back
/// a page would make the room they are in vanish from their own screen if it
/// happened to fall off the end of it.
async fn listing(state: &Shared, account_id: &str) -> serde_json::Value {
    let names = seated_names(state).await;
    let lobby = state.lobby.lock();
    serde_json::json!({
        "games": lobby.list_for(account_id, &names),
    })
}

/// One page of the listing, searched and counted.
async fn listing_page(
    state: &Shared,
    account_id: &str,
    query: &lobby::LobbyQuery,
) -> serde_json::Value {
    let names = seated_names(state).await;
    let lobby = state.lobby.lock();
    let (games, total) = lobby.page_for(account_id, &names, query);
    serde_json::json!({
        "games": games,
        "total": total,
        "offset": query.offset,
        "limit": query.page(),
    })
}

/// Display names for every account sitting at a visible table.
///
/// The lobby guard is taken, read and dropped before the database is asked
/// anything, and that order is now load-bearing rather than tidy: a
/// `parking_lot` guard held across an `.await` makes the whole future
/// `!Send`, which axum refuses to accept as a handler. The ordering was
/// already right — it is the reason this conversion was small.
///
/// One query for every name, not one per chair.
async fn seated_names(state: &Shared) -> std::collections::HashMap<String, String> {
    let wanted = state.lobby.lock().seated_accounts();
    match store::display_names(&state.db, wanted).await {
        Ok(names) => names,
        Err(e) => {
            tracing::error!("{e:#}");
            std::collections::HashMap::new()
        }
    }
}

/// The lobby, searched and paged.
async fn list_games(
    State(state): State<Shared>,
    headers: HeaderMap,
    Query(query): Query<lobby::LobbyQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    Ok(Json(listing_page(&state, &account_id, &query).await))
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
    let mut main = expand(&parse_deck_lines(&deck.cards)?);
    let side = expand(&parse_deck_lines(&deck.sideboard)?);
    // The commander is stored by name and resolved here, the same way the
    // rows are — and it is *moved* out of the list rather than copied.
    // `DeckBuilder::set_commander` seats the leader among the rows on
    // purpose (a commander outside the list is a deck nobody meant to
    // build), so what arrives here is a hundred rows with the commander
    // among them. Copying it would make a 101st card that sits in the
    // library and the command zone at once: drawable, and two of a legend.
    //
    // Moving it also keeps its printing. The player picked one for that
    // row, and the card that goes to the command zone is the piece of
    // cardboard they picked. A name that resolves to no row at all — a deck
    // posted straight to the API — still gets its commander, at the
    // registry's reference printing.
    let commanders = deck
        .commander
        .as_deref()
        .and_then(baylee_cards::decks::by_name)
        .map(|index| {
            let card = match main.iter().position(|c| c.index == index) {
                Some(at) => main.remove(at),
                None => baylee_cards::decks::DeckCard::plain(index),
            };
            vec![card]
        })
        .unwrap_or_default();
    Ok(baylee_cards::decks::LoadedDeck {
        name: deck.name.clone(),
        main,
        sideboard: side,
        commanders,
    })
}

/// Every printing at a table, deduplicated, as the ids the art cache keys on.
///
/// The gateway is the only party that may hold this list whole: a *seat* is
/// entitled to its own deck's printings and earns the rest by seeing the cards,
/// which is why `GameStatic.prints` is a list of holes. Warming from here tells
/// no client anything — it only means the picture is already local by the time
/// the rules let that client ask for it. See `art.rs`.
fn table_prints(preset: &baylee_core::preset::GamePreset) -> Vec<String> {
    // Deduplicated because a deck plays four of a card and the four are one
    // picture — a hundred-card deck is about sixty distinct printings.
    let mut ids: Vec<String> = preset
        .prints
        .iter()
        .map(|p| p.scryfall_id.to_string())
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// The deck an AI seat plays when the host did not give it one.
fn house_deck() -> Result<baylee_cards::decks::LoadedDeck, (StatusCode, Json<ErrorBody>)> {
    let text = std::fs::read_to_string("data/acceptance-decks.txt")
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "deck data missing"))?;
    baylee_cards::decks::load_acceptance(&text, "Victory")
        .map_err(|_e| err(StatusCode::INTERNAL_SERVER_ERROR, "house deck missing"))
}

/// Every response carries the headers a browser needs to read it.
///
/// The browser client is served from somewhere else by construction: the page
/// is a `trunk serve` (or a static host) and the gateway is a different
/// origin, which `?gateway=…` exists to say. So every request it makes is
/// cross-origin, and without these headers a browser discards the answer —
/// a `GET /pool` is a *simple* request that is sent and then thrown away, and
/// anything carrying `Authorization` is not sent at all, because the
/// preflight was answered `405 Method Not Allowed`. Measured: a gateway with
/// no CORS refuses `OPTIONS /auth/login` outright, which is the whole lobby.
///
/// `*` rather than an allowlist, and that is a decision rather than a
/// shortcut. This gateway authenticates with a **bearer token in a header**
/// and sets no cookie anywhere, so a browser sends nothing ambient with a
/// cross-origin request: a page on another origin can reach these routes as
/// an anonymous client and no further, which is exactly what any HTTP client
/// can already do. `Allow-Credentials` is therefore never sent, and the two
/// together are the pair that must not drift apart — `*` with credentials is
/// the combination the fetch spec refuses, and for good reason.
///
/// The preflight is answered here rather than routed, because axum resolves a
/// path to a `MethodRouter` that knows only the methods a handler registered:
/// an `OPTIONS` to `/auth/login` is a 405 before any handler sees it, and a
/// per-route `options(…)` would have to be typed out on all sixty routes and
/// forgotten on the sixty-first.
async fn cors(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::http::{HeaderValue, Method, header};

    /// What the client is allowed to send. `authorization` is the bearer
    /// token and `content-type` is what makes a JSON body a JSON body — both
    /// are what turns an otherwise simple request into a preflighted one.
    const ALLOW_HEADERS: &str = "authorization, content-type";
    /// Every method this gateway routes.
    const ALLOW_METHODS: &str = "GET, POST, PUT, DELETE, OPTIONS";

    let preflight = request.method() == Method::OPTIONS;
    let mut response = if preflight {
        // Answered whole, and never passed on: the route this is a preflight
        // *for* has no `OPTIONS` handler and would answer 405.
        let mut empty = axum::response::Response::new(axum::body::Body::empty());
        *empty.status_mut() = StatusCode::NO_CONTENT;
        empty
    } else {
        next.run(request).await
    };
    let headers = response.headers_mut();
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    if preflight {
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static(ALLOW_METHODS),
        );
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static(ALLOW_HEADERS),
        );
        // A day, so a lobby that asks sixty questions asks this one once.
        headers.insert(
            header::ACCESS_CONTROL_MAX_AGE,
            HeaderValue::from_static("86400"),
        );
    }
    response
}

/// The cards this gateway puts on every named seat's battlefield before turn
/// one, read from `BAYLEE_DEV_SEAT_BOARD`.
///
/// Same variable and same parser as the client's offline harness
/// (`baylee_cards::decks::deal_named`) — `0:Reflecting Pool; 1:Reflecting
/// Pool` seats one on each side of a duel — because a board dealt here and a
/// board dealt there have to be the same board or neither is evidence about
/// the other.
///
/// Behind the `dev-table` feature, and not merely behind the variable. A
/// gateway is somebody's server, and one that seats cards from its own
/// environment is a table whose operator can stack it silently; the feature
/// is what keeps those routes out of a build meant to be run for other
/// people. `validate_the_dev_board` refuses to start a gateway whose spec
/// does not resolve, so a failure here is a name that stopped resolving
/// mid-run rather than a typo.
#[cfg(feature = "dev-table")]
fn deal_the_dev_board(
    preset: &mut baylee_core::preset::GamePreset,
) -> Result<(), (StatusCode, Json<ErrorBody>)> {
    let Ok(spec) = std::env::var("BAYLEE_DEV_SEAT_BOARD") else {
        return Ok(());
    };
    baylee_cards::decks::deal_named(preset, &spec, baylee_cards::decks::DevZone::Battlefield)
        .map_err(|why| {
            tracing::error!("BAYLEE_DEV_SEAT_BOARD: {why}");
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                "the dev board could not be dealt",
            )
        })
}

/// No board is dealt in a build without the `dev-table` feature.
#[cfg(not(feature = "dev-table"))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the shape is the feature-gated twin's"
)]
fn deal_the_dev_board(
    _preset: &mut baylee_core::preset::GamePreset,
) -> Result<(), (StatusCode, Json<ErrorBody>)> {
    Ok(())
}

/// Refuses to start on a `BAYLEE_DEV_SEAT_BOARD` that does not resolve.
///
/// The spec is dealt into a throwaway table of `MAX_SEATS` chairs, which is
/// every name and every seat index the real thing could be asked for — so a
/// typo is a gateway that does not come up, and not a room that refuses to
/// start with two people already sitting at it. `dev-reload` sets the
/// precedent: a development switch that quietly does nothing is worse than
/// one that is loud.
///
/// # Panics
///
/// On a name the registry does not answer to, deliberately.
#[cfg(feature = "dev-table")]
fn validate_the_dev_board() {
    let Ok(spec) = std::env::var("BAYLEE_DEV_SEAT_BOARD") else {
        return;
    };
    // Eight chairs because `GamePreset::validate` bounds a table at eight,
    // and empty decks because only the names and the seat indices are being
    // resolved here — the real preset is built per room, from real decks.
    let empty = baylee_cards::decks::LoadedDeck {
        name: String::new(),
        main: vec![],
        sideboard: vec![],
        commanders: vec![],
    };
    let chairs = [&empty; 8];
    let mut probe = baylee_cards::decks::preset_for_all(0, &chairs);
    if let Err(why) = baylee_cards::decks::deal_named(
        &mut probe,
        &spec,
        baylee_cards::decks::DevZone::Battlefield,
    ) {
        panic!("BAYLEE_DEV_SEAT_BOARD: {why}");
    }
    tracing::info!("BAYLEE_DEV_SEAT_BOARD is set: every game starts with `{spec}` on the table");
}

/// Nothing to validate in a build without the `dev-table` feature.
#[cfg(not(feature = "dev-table"))]
fn validate_the_dev_board() {}

/// Preset for a human-vs-AI game (house AI plays Victory).
fn ai_preset(
    deck: &Deck,
    seed: u64,
) -> Result<baylee_core::preset::GamePreset, (StatusCode, Json<ErrorBody>)> {
    let house = house_deck()?;
    let player = loaded_deck(deck)?;
    let mut preset = baylee_cards::decks::preset_for(seed, &player, &house);
    deal_the_dev_board(&mut preset)?;
    Ok(preset)
}

/// Looks a deck up and checks it belongs to the account asking for it.
async fn own_deck(
    state: &Shared,
    account_id: &str,
    deck_id: &str,
) -> Result<(String, Deck), (StatusCode, Json<ErrorBody>)> {
    let deck = store::deck(&state.db, deck_id)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such deck"))?;
    if deck.account_id != account_id {
        return Err(err(StatusCode::FORBIDDEN, "not your deck"));
    }
    Ok((deck.name.clone(), deck))
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
        spec.team = seat.team;
    }
    deal_the_dev_board(&mut preset)?;
    // The engine refuses a table with only one side on it, and so does the
    // lobby — here rather than at the first state-based action, so the room
    // says why instead of starting a game that is already over.
    if preset.validate().is_err() {
        return Err(err(
            StatusCode::CONFLICT,
            "every seat is on the same team; a game needs at least two sides",
        ));
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
    let prints;
    {
        let mut lobby = state.lobby.lock();
        let Some(game) = lobby.games.get_mut(id) else {
            return Ok(false);
        };
        if game.state != LobbyState::Waiting || !game.seats.iter().all(lobby::LobbySeat::ready) {
            return Ok(false);
        }
        let preset = room_preset(&game.seats, auth::new_game_seed())?;
        prints = table_prints(&preset);
        game.preset = Some(preset);
        game.state = LobbyState::Playing;
    }
    // Outside the lobby lock: it spawns a task rather than doing the work, but
    // a mutex held across anything that touches the network is how a lobby
    // route starts waiting on Scryfall.
    state.art.warm(prints);
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
    let account_id = authed(&state, &headers).await?;
    let (deck_name, deck) = own_deck(&state, &account_id, &body.deck_id).await?;
    let game_id = auth::new_id();
    let seat_token = auth::new_token();

    // The one-tap game against the house AI keeps its own path: it is a whole
    // table decided in one request, and nobody is going to configure it.
    if body.mode == "ai" {
        {
            let mut preset = ai_preset(&deck, auth::new_game_seed())?;
            preset.seats[0].controller = baylee_core::preset::SeatController::Open;
            state.art.warm(table_prints(&preset));
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
        state.lobby_moved();
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
    state.lobby_moved();
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
    let account_id = authed(&state, &headers).await?;
    let (deck_name, deck) = own_deck(&state, &account_id, &body.deck_id).await?;
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
    state.lobby_moved();
    Ok(Json(serde_json::json!({
        "game_id": id,
        "seat": seat,
        "seat_token": seat_token,
    })))
}

/// `POST /lobby/games/{id}/seat` — hand this account its chair back.
///
/// A seat token is issued once, at the join, and the gateway keeps only its
/// hash. A client that restarts has lost it for good, and every other route
/// answers the wrong question: `join` refuses ("you are already at this
/// table", and "game already started" once it is running), and the listing
/// shows the player their own table with no way into it. Everything else
/// about reconnecting is already built — the engine holds the chair open
/// (`Deadline::StandIn`, `Session::stand_in`, `SeatAttached`) and the client
/// knows how to re-dial (`reconnect.rs`, twelve attempts) — and all of it
/// hung on a secret only the dead process knew.
///
/// So this is deliberately *not* a join: no deck is chosen, no chair changes
/// hands, nothing another player can see moves. It asks one question — is
/// this account already sitting here — and a table that is **playing** is
/// exactly when the answer matters most.
///
/// Reissuing invalidates the old token, which is the right way round: the
/// chair's secret is whatever was handed out last, so a copy kept by some
/// older client cannot go on answering for a seat its owner has taken back.
async fn take_seat(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let seat_token = auth::new_token();
    let seat = {
        let mut lobby = state.lobby.lock();
        let game = lobby
            .games
            .get_mut(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        // A finished game has nothing left to answer for. The other two
        // states both do: before the start the seat screen waits for the
        // room, after it there is a game to catch up on.
        if game.state == LobbyState::Over {
            return Err(err(StatusCode::CONFLICT, "that game is over"));
        }
        let chair = game
            .seats
            .iter_mut()
            .find(|s| s.account_id.as_ref() == Some(&account_id))
            .ok_or_else(|| err(StatusCode::FORBIDDEN, "you are not at this table"))?;
        chair.seat_token_hash = Some(auth::token_hash(&seat_token));
        chair.seat
    };
    // No `lobby_moved()`: the arrangement of the table is exactly as it was.
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
    /// Which team this chair plays for. Teams are numbered from 1, and `0`
    /// puts the chair back on its own side — a sentinel rather than a
    /// `null`, so "leave the team alone" stays the absent field it is for
    /// every other setting here.
    #[serde(default)]
    team: Option<u8>,
}

/// Configures one seat of a room.
///
/// Two authorities, deliberately narrow. The **host** arranges the table:
/// which chairs are people and which are the AI, how hard the AI plays, what
/// an AI chair brings, and which side each chair plays for. A **player**
/// changes exactly one thing, their own deck — including the host, whose own
/// chair is theirs as a player and not as the host.
///
/// Teams are the host's because they are the format, not a preference: a
/// player who could pick their own would pick the winning one, and a table
/// whose sides can change under the people at it is not the table they
/// agreed to sit at.
async fn set_seat(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path((id, seat)): Path<(String, usize)>,
    Json(body): Json<SeatBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    // Looked up before the lobby lock: the store has its own, and taking two
    // in one order here and the other order elsewhere is how deadlocks start.
    let chosen = match &body.deck_id {
        Some(deck_id) => Some(own_deck(&state, &account_id, deck_id).await?),
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
        if let Some(team) = body.team {
            if !is_host {
                return Err(err(StatusCode::FORBIDDEN, "only the host arranges seats"));
            }
            if team > MAX_SEATS as u8 {
                return Err(err(StatusCode::BAD_REQUEST, "no such team"));
            }
            let moved = chair.team != (team > 0).then_some(team);
            chair.team = (team > 0).then_some(team);
            // Being moved to another side changes the game they said yes to,
            // exactly as a swapped deck does.
            if moved {
                chair.said_ready = false;
            }
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
    state.lobby_moved();
    Ok(Json(listing(&state, &account_id).await))
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
    let account_id = authed(&state, &headers).await?;
    let rematch = {
        let mut lobby = state.lobby.lock();
        let game = lobby
            .games
            .get_mut(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        if game.state != LobbyState::Waiting {
            return Err(err(StatusCode::CONFLICT, "game already started"));
        }
        let is_rematch = game.parent.is_some();
        let chair = game
            .seats
            .iter_mut()
            .find(|s| s.account_id.as_ref() == Some(&account_id))
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "you are not at this table"))?;
        if body.ready && chair.deck.is_none() {
            return Err(err(StatusCode::CONFLICT, "pick a deck first"));
        }
        chair.said_ready = body.ready;
        is_rematch
    };
    // A rematch room starts itself, and the rule belongs to the *room* rather
    // than to the button that opened it. A chair there is only ready once its
    // player has claimed it through `/rematch`, so this route cannot be what
    // makes the table complete — but it can be what completes it *again*,
    // after someone withdrew and changed their mind, and then the host may
    // well be sitting on the waiting veil with no way to press start.
    //
    // Outside the lock, for the reason `start_room` gives, and unconditional
    // because `try_start` answers `Ok(false)` for a room that is not ready.
    if rematch {
        try_start(&state, &id)?;
    }
    state.lobby_moved();
    Ok(Json(listing(&state, &account_id).await))
}

/// Starts the room. The host's call, and only once every chair is ready.
async fn start_room(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
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
    state.lobby_moved();
    Ok(Json(listing(&state, &account_id).await))
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
    let account_id = authed(&state, &headers).await?;
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
    state.lobby_moved();
    Ok(Json(listing(&state, &account_id).await))
}

/// Gives up a seat, handing the room on if the host is the one leaving.
async fn leave_game(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
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
    drop(lobby);
    state.lobby_moved();
    Ok(StatusCode::NO_CONTENT)
}

/// Plays the same table again — or joins the room somebody already opened.
///
/// A rematch is not a new room the players have to find each other in a
/// second time. The first press builds one with the whole arrangement copied
/// — the chairs, the sides they play for, who was in them, which ones were
/// the AI and how hard, the name and the password — and every press after
/// that joins *that* room. Which is what the route being idempotent buys:
/// two people pressing at the same moment sit down at one table, not two.
///
/// Pressing it is also the ready statement. `said_ready` means "I want to
/// play", and there is nothing else a button that says *play again* could be
/// saying — a caller seated un-ready would be left on the waiting veil for a
/// game nobody had told the gateway to start.
///
/// So the room starts itself once every chair is ready. That is not the rule
/// taken out of `set_seat`: that one fired when a player picked a deck to
/// look at it, and this one fires when everyone at the table has asked for
/// another game. An AI chair is ready as soon as it is configured, so a solo
/// player is one press from the next game, which is the case that matters
/// most — the game they are asking to repeat was itself one tap.
async fn rematch(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let seat_token = auth::new_token();
    // Lobby, then store, then lobby again — never both at once, which is the
    // rule `seat_names` states and every path here keeps. Nothing in this
    // crate nests the two, so there is no order to nest them in, and the one
    // that wanted both is this: a rematch plays each deck as it stands *now*,
    // so the chairs' deck ids are read out of the finished table before the
    // store is asked what those decks say today. `None` where the caller
    // addressed the room instead: it exists already, and nothing about it is
    // being built from a deck.
    let played: Option<Vec<Option<String>>> = {
        let lobby = state.lobby.lock();
        let game = lobby
            .games
            .get(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        // Who sat there, not who knows the id: a game's id was in every
        // player's own listing while the table was waiting, so it is no
        // secret and cannot be what the check rests on.
        if !game
            .seats
            .iter()
            .any(|s| s.account_id.as_ref() == Some(&account_id))
        {
            return Err(err(StatusCode::FORBIDDEN, "you were not at that table"));
        }
        match game.state {
            LobbyState::Over => Some(
                game.seats
                    .iter()
                    .map(|s| s.deck.as_ref().map(|d| d.id.clone()))
                    .collect(),
            ),
            // The room itself. Both ends address the same rematch, because
            // the two people asking are looking at different things: the one
            // who just finished has the game's id on the screen in front of
            // them, and the one who went back to the lobby has the room's.
            LobbyState::Waiting if game.parent.is_some() => None,
            _ => return Err(err(StatusCode::CONFLICT, "that game is not over yet")),
        }
    };
    // One query for the whole table's decks rather than one per chair.
    let wanted: Vec<String> = played.iter().flatten().flatten().cloned().collect();
    let known = store::decks_by_id(&state.db, wanted)
        .await
        .map_err(|e| db_down(&e))?;
    let fresh: Vec<Option<Deck>> = played
        .iter()
        .flatten()
        .map(|deck| deck.as_deref().and_then(|d| known.get(d)).cloned())
        .collect();
    let (room_id, seat) = {
        let mut lobby = state.lobby.lock();
        // `played` is `None` when the caller addressed the rematch room
        // itself, which is then the room and there is nothing to open.
        let room_id = if played.is_none() {
            id.clone()
        } else {
            let opened = lobby
                .games
                .get(&id)
                .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?
                .rematch
                .clone();
            // A pointer whose room has been reaped is the same situation as
            // no pointer at all. Asked again here rather than above, because
            // two players pressing at once must not each open one.
            if let Some(open) = opened.filter(|open| lobby.games.contains_key(open)) {
                open
            } else {
                let room_id = auth::new_id();
                let mut room = {
                    let over = lobby
                        .games
                        .get(&id)
                        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
                    LobbyGame::rematch_of(over, room_id.clone(), auth::now_secs())
                };
                // Seat for seat with what was read above: a finished game's
                // chairs do not move, every route that rearranges them
                // wanting a room that is still waiting. A deck deleted in
                // between leaves its chair the copy the table played.
                for (chair, fresh) in room.seats.iter_mut().zip(&fresh) {
                    if let Some(deck) = fresh {
                        chair.deck_name.clone_from(&deck.name);
                        chair.deck = Some(deck.clone());
                    }
                }
                lobby.games.insert(room_id.clone(), room);
                if let Some(over) = lobby.games.get_mut(&id) {
                    over.rematch = Some(room_id.clone());
                }
                room_id
            }
        };
        let room = lobby
            .games
            .get_mut(&room_id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        if room.state != LobbyState::Waiting {
            return Err(err(StatusCode::CONFLICT, "the rematch has already started"));
        }
        let chair = room
            .seats
            .iter_mut()
            .find(|s| s.account_id.as_ref() == Some(&account_id))
            .ok_or_else(|| err(StatusCode::FORBIDDEN, "you were not at that table"))?;
        chair.seat_token_hash = Some(auth::token_hash(&seat_token));
        chair.said_ready = true;
        (room_id, chair.seat)
    };
    // Outside both locks, for the reason `start_room` gives: ordering an
    // engine must not happen underneath the lobby lock. A room that is not
    // ready yet stays waiting and the ticket is still good — whoever presses
    // last is the one who starts it.
    try_start(&state, &room_id)?;
    state.lobby_moved();
    Ok(Json(serde_json::json!({
        "game_id": room_id,
        "seat": seat,
        "seat_token": seat_token,
    })))
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
    let account_id = authed(&state, &headers).await?;
    let answers = store::automation_of(&state.db, &account_id)
        .await
        .map_err(|e| db_down(&e))?;
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
    let account_id = authed(&state, &headers).await?;
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
    store::put_automation(&state.db, &account_id, answers)
        .await
        .map_err(|e| db_down(&e))?;
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
    let account_id = authed(&state, &headers).await?;
    let settings = store::settings_of(&state.db, &account_id)
        .await
        .map_err(|e| db_down(&e))?
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
    let account_id = authed(&state, &headers).await?;
    if !body.is_object() {
        return Err(err(StatusCode::BAD_REQUEST, "settings must be an object"));
    }
    let bytes = serde_json::to_string(&body).map_or(usize::MAX, |s| s.len());
    if bytes > MAX_SETTINGS_BYTES {
        return Err(err(StatusCode::PAYLOAD_TOO_LARGE, "settings too large"));
    }
    store::put_settings(&state.db, &account_id, body)
        .await
        .map_err(|e| db_down(&e))?;
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
async fn standing_for_seat(state: &Shared, game_id: &str, seat: usize) -> Vec<u8> {
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
    // The guard above is read and dropped before this: an answer that never
    // arrives is a question the player is asked again, but a lobby lock held
    // across a query is every other route waiting on the database.
    match store::automation_of(&state.db, &account_id).await {
        Ok(answers) => standing_payload(&answers),
        Err(e) => {
            tracing::error!("{e:#}");
            b"[]".to_vec()
        }
    }
}

// ---------------------------------------------------------------- game ws

#[derive(Deserialize)]
struct WsParams {
    token: String,
}

/// What `/lobby/ws` is opened with: an account token, and the same search a
/// `GET /lobby/games` would carry.
#[derive(Deserialize)]
struct LobbyWsParams {
    /// The account bearer token.
    ///
    /// In the query string rather than a header because a browser's
    /// `WebSocket` cannot set one — the same reason the seat socket does it,
    /// and the same trade: a URL is likelier to be logged, which is why this
    /// is the account token and not something longer-lived.
    token: String,
    /// The page and search this reader wants, flattened so the socket URL and
    /// the HTTP route take the identical parameters.
    #[serde(flatten)]
    query: lobby::LobbyQuery,
}

/// The lobby's push channel.
///
/// The listing used to be re-read every two seconds by every client sitting
/// in the lobby, because nothing could tell them a chair had moved. This can:
/// it sends the listing on connect and again whenever anything in the lobby
/// changes, rendered for *this* reader with *their* search.
async fn lobby_ws(
    State(state): State<Shared>,
    Query(params): Query<LobbyWsParams>,
    ws: WebSocketUpgrade,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorBody>)> {
    let account_id = store::resolve_token(&state.db, &params.token, auth::now_secs())
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "invalid or expired token"))?;
    Ok(ws.on_upgrade(move |socket| run_lobby_socket(state, account_id, params.query, socket)))
}

/// Pushes the listing to one reader until they go away.
async fn run_lobby_socket(
    state: Shared,
    account_id: String,
    query: lobby::LobbyQuery,
    mut socket: WebSocket,
) {
    // Subscribed before the first send, so a change that lands while the
    // opening listing is being rendered is not lost between the two.
    let mut changed = state.lobby_changed.subscribe();
    loop {
        let payload = listing_page(&state, &account_id, &query).await.to_string();
        if socket.send(Message::Text(payload.into())).await.is_err() {
            return;
        }
        // A reader that fell behind is sent the state of the world, not the
        // history it missed: the payload is the whole listing every time, so
        // one late send says everything the skipped ones would have.
        match changed.recv().await {
            Ok(()) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
            Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
        }
    }
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

/// `GET /games/{id}/cosmetics?token=…` — every seat's sleeve and playmat.
///
/// The one route decorations travel on, and the reason they do not travel with
/// the game: `GameStatic` is rules data and a view is what a seat is entitled
/// to know, while a sleeve is a fact about a *deck*. The gateway is the layer
/// that knows which deck sits in which chair, so answering here costs no
/// `VIEW_VERSION` and leaves the engine as ignorant of decoration as it is of
/// card text.
///
/// Authorised by the same seat token as the game socket, which is the honest
/// bound: this says nothing a player will not see the moment the first card is
/// drawn face-down in front of them, and nothing at all about a deck's
/// contents.
async fn game_cosmetics(
    State(state): State<Shared>,
    Path(id): Path<String>,
    Query(params): Query<WsParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let lobby = state.lobby.lock();
    let game = lobby
        .games
        .get(&id)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
    let token_hash = auth::token_hash(&params.token);
    if !game.seats.iter().any(|s| {
        s.seat_token_hash
            .as_ref()
            .is_some_and(|h| auth::ct_eq(h, &token_hash))
    }) {
        return Err(err(StatusCode::UNAUTHORIZED, "invalid seat token"));
    }
    // Seats with nothing set are left out rather than sent as empty objects:
    // a table where nobody uploaded anything is `{}`, and the client's answer
    // to "no entry" and to "an entry with no sleeve" has to be the same thing
    // anyway — draw the generated back.
    let mut seats = cosmetics::TableCosmetics::new();
    for seat in &game.seats {
        let Some(deck) = seat.deck.as_ref() else {
            continue;
        };
        let worn = cosmetics::SeatCosmetics {
            sleeve: deck.sleeve.clone(),
            playmat: deck.playmat.clone(),
        };
        if !worn.is_empty() {
            seats.insert(seat.seat, worn);
        }
    }
    Ok(Json(serde_json::json!(seats)))
}

/// The names shown at each seat of a game, in seat order.
///
/// The rules kernel has never heard of an account, so the roster is assembled
/// here and handed to the engine with the preset. The two locks are taken one
/// after the other rather than nested: the lobby says which account sits
/// where, the store says what that account is called, and nothing in between
/// needs both at once.
async fn seat_names(state: &Shared, game_id: &str) -> Vec<String> {
    let accounts: Vec<Option<String>> = {
        let lobby = state.lobby.lock();
        match lobby.games.get(game_id) {
            Some(game) => game.seats.iter().map(|s| s.account_id.clone()).collect(),
            None => return Vec::new(),
        }
    };
    let names = match store::display_names(&state.db, accounts.iter().flatten().cloned()).await {
        Ok(names) => names,
        Err(e) => {
            tracing::error!("{e:#}");
            std::collections::HashMap::new()
        }
    };
    accounts
        .into_iter()
        .map(|id| match id {
            Some(id) => names
                .get(&id)
                .cloned()
                .unwrap_or_else(|| "Unknown player".to_string()),
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
        standing_json: standing_for_seat(&state, &game_id, seat).await,
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
mod tests;
