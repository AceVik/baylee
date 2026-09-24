//! baylee-gateway — accounts, decks, lobby, and hosted games.
//!
//! Security posture (see auth.rs): Argon2id password hashing, hashed
//! bearer tokens with sliding expiry, auth rate limiting, generic
//! credential errors, constant-time comparisons. TLS terminates at the
//! reverse proxy in front of this process (Caddy/nginx) — this service
//! must never be exposed on a plaintext listener in production.

mod art;
mod auth;
mod clock;
mod cosmetics;
mod engine;
mod handle;
mod lobby;
mod mail;
mod pool;
mod store;
mod texts;

use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use axum::routing::{get, post};
use baylee_protocol::names;
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
    /// Accounts, sessions, decks, links, preferences.
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
    /// Whether a player may play as a guest (`BAYLEE_GUESTS=off` to
    /// disable; #269).
    guests_enabled: bool,
    /// How many guests there may be at once (`BAYLEE_GUEST_CAP`, 1000 unless
    /// set, `0` for no bound); `None` when unbounded. See [`guest_cap`].
    guest_cap: Option<u64>,
    /// What this gateway calls itself to a client (`BAYLEE_GATEWAY_NAME`),
    /// already checked by [`display_name`]. `None` when unset, and the client
    /// names it by its address instead.
    display_name: Option<String>,
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
    /// The card catalog, when its schema could be applied.
    ///
    /// `None` is not "no database": `DATABASE_URL` is required and this
    /// process refuses to start without one, so by the time this field is
    /// built the connection already works. What it records is the *second*
    /// half failing on its own — most often a `unaccent` extension the role
    /// may not create — and that stays non-fatal, because card text is
    /// presentation. A gateway with `None` here plays every game and serves
    /// no card text; the client draws faces from what the engine projects.
    ///
    /// `GET /health` spells this apart from an empty one: `off` is this
    /// field, `empty` is a catalog nobody has ingested into.
    catalog: Option<baylee_catalog::Catalog>,
    /// The pool's card text per language, until the catalog's data stamp
    /// moves. See `texts.rs`.
    texts: texts::TextCache,
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
    let display_name = display_name(std::env::var("BAYLEE_GATEWAY_NAME").ok().as_deref())
        .unwrap_or_else(|why| panic!("BAYLEE_GATEWAY_NAME: {why}"));
    let guest_cap = guest_cap(std::env::var("BAYLEE_GUEST_CAP").ok().as_deref())
        .unwrap_or_else(|why| panic!("BAYLEE_GUEST_CAP: {why}"));
    let store_path = std::env::var("STORE_PATH")
        .map_or_else(|_| PathBuf::from("gateway-store.json"), PathBuf::from);
    let db = open_database(&store_path).await;
    let catalog = connect_catalog(db.clone()).await;
    // Bound after the database and the catalog, and before anything that has
    // to know the port: with `PORT=0` the kernel chooses it (#279), and the
    // address an engine is told to dial back on below must be that one.
    let (listener, port) = listen(port).await;
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
        registration_enabled: switched_on(std::env::var("BAYLEE_REGISTRATION").ok().as_deref()),
        guests_enabled: switched_on(std::env::var("BAYLEE_GUESTS").ok().as_deref()),
        guest_cap,
        display_name,
        mail: mail::Mailer::from_env(),
        trusted_proxies: trusted_proxies(
            &std::env::var("BAYLEE_TRUSTED_PROXIES").unwrap_or_default(),
        ),
        catalog,
        texts: texts::TextCache::default(),
        art: Arc::new(art::ArtCache::from_env()),
        deck_images: Arc::new(cosmetics::Store::from_env()),
    });
    spawn_cleanup(state.clone());

    let app = Router::new()
        .route("/health", get(health))
        .route("/source", get(source))
        .route("/info", get(info))
        .route("/auth/config", get(auth_config))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/guest", post(guest))
        .route("/auth/confirm", get(confirm))
        .route("/auth/confirm/resend", post(resend_confirmation))
        .route("/auth/logout", post(logout))
        .route("/me", get(me))
        .route("/players/{handle}", get(player))
        .merge(deck_routes())
        .route("/lobby/games", get(list_games).post(create_game))
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
        // Card art, mirrored from Scryfall by printing id for this gateway's
        // own players: a session is required, and it is an id cache rather
        // than a proxy — see `art.rs`.
        .route("/art/{size}/{face}/{a}/{b}/{file}", get(art::art))
        // Axum caps a body at 2 MB by default, which is under a phone
        // photograph. The cap that matters is the one in `cosmetics`,
        // checked again before anything is decoded.
        // The kind rides in the query rather than the path, and not by
        // preference: axum registers a route by its path before it looks at
        // the method, so `/images/{kind}` and `/images/{file}` are the same
        // route wearing two different parameter names, and building the
        // router panics. It panicked at startup, which meant the gateway
        // never served and every end-to-end test in the crate failed
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

    announce(port);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("gateway serves");
}

/// Binds the gateway's port, and answers which port that is: the one asked
/// for, or with `PORT=0` the one the kernel chose.
async fn listen(port: u16) -> (tokio::net::TcpListener, u16) {
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("bind gateway port");
    let port = listener
        .local_addr()
        .expect("a bound listener has an address")
        .port();
    (listener, port)
}

/// Says the gateway is about to serve, and where: in the log, and to
/// `BAYLEE_PORT_FILE` when one is named.
fn announce(port: u16) {
    tracing::info!(port, "baylee-gateway listening");
    if let Some(path) = std::env::var_os("BAYLEE_PORT_FILE") {
        write_port_file(std::path::Path::new(&path), port)
            .unwrap_or_else(|e| panic!("BAYLEE_PORT_FILE {}: {e}", path.display()));
    }
}

/// Writes the port the gateway is listening on to `path`, for whoever
/// started it with `PORT=0` and needs to know where it went (#279).
///
/// The e2e suite is that caller. It used to pick a port by binding
/// `127.0.0.1:0`, reading the number and letting go, and the gateway bound
/// `0.0.0.0` on that number seconds later, after its database was up. Any
/// other process could take the port in that gap, and with several worktrees
/// gating at once one did, often enough to fail two feature gates in three.
/// Binding port 0 here and saying which port it was leaves no gap.
///
/// Written beside the target and renamed over it, so a reader polling for
/// the file never reads half a number. Called just before the gateway
/// serves, so the file's appearance also says it is past its database and
/// catalog.
fn write_port_file(path: &std::path::Path, port: u16) -> std::io::Result<()> {
    let partial = path.with_extension("partial");
    std::fs::write(&partial, format!("{port}\n"))?;
    std::fs::rename(&partial, path)
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
/// waiting lobbies, expired tokens and the guests they were the way into —
/// all of them grew without bound.
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
            // After the sessions, because a guest goes with its last one
            // (#269).
            match store::purge_guests(&state.db, now, None).await {
                Ok(purged) if purged > 0 => tracing::info!(purged, "idle guests deleted"),
                Ok(_) => {}
                Err(e) => tracing::warn!("{e:#}"),
            }
        }
    });
}

// ------------------------------------------------------------------- auth

#[derive(Deserialize)]
struct RegisterBody {
    /// The name to sign in with (#269). Defaulted so that a client from
    /// before usernames, which sends an address instead, is told what is
    /// missing rather than handed a deserialiser's error.
    #[serde(default)]
    username: String,
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
    /// A username, or until the end of 2026 an address (#269, #280). `email`
    /// is what a client from before usernames calls the same field.
    #[serde(alias = "email")]
    username: String,
    password: String,
}

#[derive(Serialize)]
struct ErrorBody {
    /// Borrowed for the constant messages, owned for the few that have to
    /// name a value back to the caller.
    ///
    /// `Cow` rather than `String` everywhere: almost every refusal in this
    /// file is a fixed sentence, and making them all allocate to serve the
    /// handful that cannot would be paying for the exception on every path.
    error: std::borrow::Cow<'static, str>,
}

fn err(status: StatusCode, message: &'static str) -> (StatusCode, Json<ErrorBody>) {
    (
        status,
        Json(ErrorBody {
            error: std::borrow::Cow::Borrowed(message),
        }),
    )
}

/// The same, for a refusal that has to quote what the caller sent.
///
/// A message that says only "bad clock" makes the caller guess; one that
/// names the clocks there are answers the request and the next one.
fn err_saying(status: StatusCode, message: String) -> (StatusCode, Json<ErrorBody>) {
    (
        status,
        Json(ErrorBody {
            error: std::borrow::Cow::Owned(message),
        }),
    )
}

/// Whether a switch is on: whether this gateway lets a stranger make an
/// account (`BAYLEE_REGISTRATION`), and whether it lets one play as a guest
/// (`BAYLEE_GUESTS`, #269), one reading for both so that an operator learns
/// one.
///
/// **Three spellings shut the door and every other value leaves it open**,
/// which is the wrong way round for the one switch an operator reaches for
/// when they want it shut: `no`, `OFF` and a variable exported empty all
/// read as on, and nothing says so at the moment it is set.
///
/// Left as it stands rather than widened on a guess. Accepting more words
/// and refusing the ones it does not know are different decisions, and the
/// second is the one this workspace took next door — a `dev-table` board
/// specification that does not resolve refuses to start the gateway rather
/// than failing at the moment somebody presses Start. The day either is
/// taken, the test named after this sentence is what changes.
fn switched_on(raw: Option<&str>) -> bool {
    !matches!(raw, Some("off" | "0" | "false"))
}

/// How many guests there may be at once (`BAYLEE_GUEST_CAP`, #269), or
/// `None` for no bound.
///
/// The per-address limiter bounds how fast one address makes guests, and
/// nothing about how many a crowd of addresses makes; this bounds the
/// accounts nobody registered. Unset is a thousand, `0` is no bound, and
/// anything else that is not a count stops the gateway starting, as a bad
/// name does: a cap that read a typo as "no cap" would be found out by the
/// disk.
fn guest_cap(raw: Option<&str>) -> Result<Option<u64>, String> {
    /// How many guests a gateway takes when its operator says nothing.
    const DEFAULT_GUEST_CAP: u64 = 1000;
    match raw.map(str::trim) {
        None | Some("") => Ok(Some(DEFAULT_GUEST_CAP)),
        Some(count) => match count.parse::<u64>() {
            Ok(0) => Ok(None),
            Ok(cap) => Ok(Some(cap)),
            Err(_) => Err(format!("{count:?} is not a count of guests")),
        },
    }
}

/// The proxies whose `X-Forwarded-For` this gateway believes
/// (`BAYLEE_TRUSTED_PROXIES`, comma-separated addresses).
///
/// An entry that is not an address is dropped rather than refused, and that
/// direction is the deliberate one: an unread entry is a proxy that is
/// **not** trusted, so the limiter keys on the proxy's own address and
/// everyone behind it shares one budget. Strict, and quiet — which is the
/// right way round for a list whose other failure is switching the
/// brute-force defence off.
fn trusted_proxies(raw: &str) -> Vec<IpAddr> {
    raw.split(',')
        .filter_map(|entry| entry.trim().parse::<IpAddr>().ok())
        .collect()
}

/// The IP a rate limit is keyed on: the real peer address, unless the peer
/// itself is a configured trusted proxy — only then is its
/// `X-Forwarded-For` read at all. Trusting the header from anybody let a
/// client rotate it per request and switch the limiter off.
///
/// **Read from the right.** A proxy either replaces the header with the
/// address it is talking to or appends that address to whatever arrived,
/// and this repository tells an operator to do neither, so the rule has to
/// be right for both. On a replacing proxy there is one entry and the two
/// directions are the same. On an appending one the client writes the left
/// half itself: `X-Forwarded-For: <anything>` arrives, the proxy appends the
/// address it actually sees, and reading from the left hands the limiter a
/// string the attacker chose — the very thing trusting the header from
/// anybody did.
///
/// Entries that are themselves on the list are hops and are stepped over,
/// so a chain ends at the first address nobody vouched for. An entry that
/// is not an address at all ends the walk at the peer rather than being
/// stepped over, because everything left of it was written by whoever sent
/// it.
fn rate_limit_ip(trusted: &[IpAddr], peer: IpAddr, headers: &HeaderMap) -> String {
    if !trusted.contains(&peer) {
        return peer.to_string();
    }
    let Some(forwarded) = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
    else {
        return peer.to_string();
    };
    for entry in forwarded.rsplit(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let Ok(address) = entry.parse::<IpAddr>() else {
            return peer.to_string();
        };
        if !trusted.contains(&address) {
            return address.to_string();
        }
    }
    peer.to_string()
}

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
    let mut body = build_fields();
    body.insert("name".into(), "baylee".into());
    body.insert("license".into(), "AGPL-3.0-only".into());
    body.insert("source".into(), baylee_build::REPOSITORY.into());
    Json(body.into())
}

/// Which binary this is, spelled once for every route that says so.
///
/// `/source`, `/health` and `/info` all answer it, and all three build it
/// here from the same `baylee_build` constants, so "I rebuilt it" is
/// checkable against any of them and no two can drift into disagreeing about
/// one binary.
fn build_fields() -> serde_json::Map<String, serde_json::Value> {
    let mut fields = serde_json::Map::new();
    fields.insert("version".into(), baylee_build::short().into());
    fields.insert("commit".into(), baylee_build::COMMIT.into());
    fields.insert("build".into(), baylee_build::BUILD_NUMBER.into());
    fields.insert("built_at".into(), baylee_build::BUILT_AT.into());
    // Stated rather than implied: a reader who finds `dirty` true knows the
    // commit above does not fully describe what is running, which is the one
    // case where the §13 offer would otherwise mislead.
    fields.insert("dirty".into(), baylee_build::DIRTY.into());
    fields
}

/// What a client asks before it saves this gateway: the name to show, the
/// build, and the two versions that decide whether the two can talk.
///
/// Unauthenticated, because it is asked before there is an account, and it
/// carries nothing a stranger may not read. `protocol_version` is the
/// envelope a seat socket speaks. `view_version` is the view shape *this
/// gateway's build* was compiled with, and it is a promise about nothing
/// else: the gateway never decodes a view, and an agent's engine says no
/// version when it attaches. A deployment runs one build on both sides, so
/// the number is the right early warning; the check that decides is still
/// the client's own, on the first `GameStatic` of a game.
async fn info(State(state): State<Shared>) -> Json<serde_json::Value> {
    let mut body = build_fields();
    if let Some(name) = &state.display_name {
        body.insert("name".into(), name.clone().into());
    }
    body.insert(
        "protocol_version".into(),
        baylee_protocol::PROTOCOL_VERSION.into(),
    );
    body.insert("view_version".into(), baylee_view::VIEW_VERSION.into());
    Json(body.into())
}

/// The longest name `BAYLEE_GATEWAY_NAME` may set, in characters.
///
/// The client caps what it shows at the same length on its own
/// (`baylee_client_core::lobby::gateway_info::MAX_NAME_CHARS`), because a gateway
/// is a stranger to it and this check only binds gateways built from here.
const MAX_NAME_CHARS: usize = 64;

/// The name a gateway shows a client, out of `BAYLEE_GATEWAY_NAME`.
///
/// Unset or blank is no name, and the client shows the address. A name that
/// is too long, or that carries a control character or a bidirectional
/// override, is refused rather than trimmed. The override can make a line
/// display differently from what it says, and the two were written as one
/// string by somebody, so the operator is told at startup rather than every
/// player seeing a quietly altered one.
fn display_name(raw: Option<&str>) -> Result<Option<String>, String> {
    let Some(name) = raw.map(str::trim).filter(|name| !name.is_empty()) else {
        return Ok(None);
    };
    let length = name.chars().count();
    if length > MAX_NAME_CHARS {
        return Err(format!(
            "{length} characters, and a name has {MAX_NAME_CHARS} at most"
        ));
    }
    if let Some(bad) = name.chars().find(|&c| c.is_control() || is_bidi_control(c)) {
        return Err(format!("it contains U+{:04X}", u32::from(bad)));
    }
    Ok(Some(name.to_owned()))
}

/// The characters that reorder how a line of text is displayed (Unicode's
/// embeddings, overrides and isolates) without being shown themselves.
///
/// The client drops the same set from whatever a gateway sends it (its own
/// `is_bidi_control` in `baylee_client_core::lobby::gateway_info`); the two
/// lists are one list.
fn is_bidi_control(c: char) -> bool {
    matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

/// How long any one probe in `/health` may take.
///
/// Bounded, because a health route that hangs is strictly worse than one that
/// answers "down": a monitor blocked on a socket reports nothing at all, and
/// "nothing" is indistinguishable from "not scraped yet". Two seconds is long
/// enough that a loaded but working database still answers — the e2e suite
/// runs three dozen gateways against one server at a pool of two apiece — and
/// short enough that the caller gets a verdict rather than a timeout of its
/// own.
const HEALTH_PROBE: std::time::Duration = std::time::Duration::from_secs(2);

/// `GET /health` — whether this gateway can do its job, and what it cannot.
///
/// The route exists because the only evidence this process was alive used to
/// be a line in its log and an open port, and an open port only says that
/// `bind` succeeded. That is a weaker claim than it looks from *both* sides:
/// it says nothing about the states below, and — since binding is the last
/// thing `main` does — it also cannot be observed until everything else has
/// already worked.
///
/// Unauthenticated, because a monitor that needs a token is a monitor nobody
/// wires up, and because there is nothing here to protect. No account name,
/// no token, no store path, no configured URL, no counts that are not already
/// visible in the lobby listing: every field is a bit or a number about this
/// process, and the version is one `/source` already serves to anyone.
///
/// # What the status code carries
///
/// Exactly one question — is the database there — because that is the only
/// state this process cannot work around. `DATABASE_URL` is required: a
/// gateway whose database has gone away keeps its port open and its log
/// quiet while answering every route that matters with a 503, which is the
/// precise failure this route was asked for.
///
/// Everything else is a field and never a code. A gateway with no agent
/// connected hosts no games, and it is still a legitimate thing to be
/// running — the e2e suite spawns three dozen agentless ones — so
/// `agents.connected: 0` is reported rather than escalated. The same goes for
/// a catalog that was never ingested: no card text is a thinner client, not a
/// broken gateway.
async fn health(State(state): State<Shared>) -> (StatusCode, Json<serde_json::Value>) {
    let database = matches!(
        tokio::time::timeout(HEALTH_PROBE, state.db.ping()).await,
        Ok(Ok(()))
    );

    // Four states rather than a bit, because they want four different
    // answers from whoever is reading. `off` is a choice, `empty` wants an
    // ingest, `projection_missing` wants `baylee-catalog project` and is the
    // one that answers every search with nothing while erroring at nobody,
    // and `unreachable` is the database being gone — already in the code
    // above, repeated here so one field is not read as covering for another.
    let catalog = match state.catalog.as_ref() {
        None => serde_json::json!({ "state": "off" }),
        Some(catalog) => match tokio::time::timeout(HEALTH_PROBE, catalog.readiness()).await {
            Ok(Ok(found)) => serde_json::json!({
                "state": match (found.cards, found.projection) {
                    (true, true) => "ready",
                    (true, false) => "projection_missing",
                    _ => "empty",
                },
                "cards": found.cards,
                "projection": found.projection,
            }),
            _ => serde_json::json!({ "state": "unreachable" }),
        },
    };

    let (agents_connected, agent_games) = {
        let agents = state.agents.lock();
        (
            agents.connected.len(),
            agents
                .connected
                .values()
                .map(|agent| agent.games.len())
                .sum::<usize>(),
        )
    };

    // Counted from the lobby rather than from the agents, because the two
    // disagree in the one case worth seeing: a game the gateway has ordered
    // but whose engine has not dialled back yet is on an agent's list and has
    // no `EngineLink`. That gap is what `seats_awaiting_engine` is.
    let (games_running, games_waiting, seats_awaiting_engine) = {
        let lobby = state.lobby.lock();
        let playing = lobby
            .games
            .values()
            .filter(|game| game.state == LobbyState::Playing);
        (
            playing.clone().count(),
            lobby
                .games
                .values()
                .filter(|game| game.state == LobbyState::Waiting)
                .count(),
            playing
                .filter(|game| game.engine.is_none())
                .map(|game| game.seats.len())
                .sum::<usize>(),
        )
    };

    let code = if database {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let mut body = build_fields();
    body.insert("ok".into(), database.into());
    body.insert("database".into(), database.into());
    body.insert("catalog".into(), catalog);
    body.insert(
        "agents".into(),
        serde_json::json!({
            "connected": agents_connected,
            "games": agent_games,
        }),
    );
    body.insert(
        "games".into(),
        serde_json::json!({
            "running": games_running,
            "waiting": games_waiting,
            "seats_awaiting_engine": seats_awaiting_engine,
        }),
    );
    (code, Json(body.into()))
}

/// Public auth configuration (clients check this before offering
/// registration).
async fn auth_config(State(state): State<Shared>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "registration_enabled": state.registration_enabled,
        // Whether "play as a guest" is offered (#269).
        "guests_enabled": state.guests_enabled,
        // Whether `GET /art/…` mirrors card images. A client that pointed at a
        // gateway with the mirror switched off would get a 404 for every card
        // and draw a whole table of constructed faces, so it is told here
        // rather than discovering it one blank card at a time.
        "art_cache": state.art.enabled(),
        "deck_images": state.deck_images.enabled(),
        // The clocks a room may be opened at, so a client builds its picker
        // from what this gateway actually accepts instead of hard-coding a
        // list that goes stale the day one is added. The first is the
        // default, which is what a room gets by saying nothing.
        "clocks": clock::PRESETS.iter().map(|preset| serde_json::json!({
            "name": preset.name,
            "decide_secs": preset.decision_timeout_secs,
            "reconnect_secs": preset.reconnect_window_secs,
            "blurb": preset.blurb,
        })).collect::<Vec<_>>(),
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
    // An account without an address has nowhere to be sent a link.
    let Ok(Some(account)) = store::account(&state.db, account_id).await else {
        return;
    };
    let Some(address) = account.email else {
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
            &address,
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
        .allow(&rate_limit_ip(&state.trusted_proxies, addr.ip(), &headers))
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
        .allow(&rate_limit_ip(&state.trusted_proxies, addr.ip(), &headers))
    {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many attempts"));
    }
    let Ok(username) = names::username(&body.username) else {
        return Err(err(StatusCode::BAD_REQUEST, "invalid username"));
    };
    if !auth::valid_display_name(&body.display_name) {
        return Err(err(StatusCode::BAD_REQUEST, "invalid display name"));
    }
    if !auth::valid_password(&username.shown, &body.display_name, &body.password) {
        return Err(err(StatusCode::BAD_REQUEST, "invalid password"));
    }
    // Argon2 is deliberately expensive and runs off the async worker.
    let password = body.password.clone();
    let password_hash = tokio::task::spawn_blocking(move || auth::hash_password(&password))
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "hashing failed"))?;
    let account = store::NewAccount {
        username: username.shown,
        username_key: username.key,
        display_name: body.display_name,
        password_hash,
        created_at: auth::now_secs(),
        lang: body.lang,
    };
    // The refusal is the unique index's, not a check's. Reading "is this
    // name free" and then writing is two statements another registration
    // can slip between, and both of them would have read "free".
    //
    // And it is said openly (#269). An address could be registered without
    // saying whether it existed, because its owner would get the mail; a
    // username has to be chosen, so a taken one has to be named, and a name
    // that can be chosen can be found. The per-IP limiter above is what
    // bounds that: ten tries in five minutes. What it finds is a login name
    // and no more — the username is shown to nobody but its owner, and the
    // password is still the other half.
    match store::create_account(&state.db, account)
        .await
        .map_err(|e| db_down(&e))?
    {
        Some(_) => Ok(Json(serde_json::json!({ "ok": true }))),
        None => Err(err(StatusCode::CONFLICT, "that username is taken")),
    }
}

async fn login(
    State(state): State<Shared>,
    Json(creds): Json<Credentials>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    // The account first, so that the tries are counted against it.
    //
    // By address only while there are players who have not yet learnt the
    // name they were given (#269): until 31.12.2026, when #280 removes it,
    // and the answer below tells them their username. A username cannot hold an `@`, so the two
    // never mean the same input. Anything that is not a username at all is
    // simply nobody.
    let account = if creds.username.contains('@') {
        store::account_by_email(&state.db, &creds.username).await
    } else {
        match names::username(&creds.username) {
            Ok(name) => store::account_by_username_key(&state.db, &name.key).await,
            Err(_) => Ok(None),
        }
    }
    .map_err(|e| db_down(&e))?;
    // Eight tries at **one account**, however it was named: by its username
    // and by its address, in any case, the tries are one count. A name that
    // is nobody's is counted under what was typed, so guessing at it is
    // bounded too, and it is answered exactly as a wrong password is.
    let budget = match &account {
        Some(account) => format!("account:{}", account.id),
        None => format!("typed:{}", creds.username.to_lowercase()),
    };
    if !state.sign_in_limiter.allow(&budget) {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many attempts"));
    }
    // The expensive verify runs off the async worker, and against a dummy
    // hash when there is no account, so the two take the same time.
    // A guest has no hash, and no username or address to be found by
    // either; were one found, the dummy would refuse it like a stranger.
    let stored_hash = account.as_ref().and_then(|a| a.password_hash.clone());
    let password = creds.password.clone();
    let ok = tokio::task::spawn_blocking(move || {
        auth::verify_password(stored_hash.as_deref(), &password)
    })
    .await
    .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "verify failed"))?;
    let Some(account) = account else {
        return Err(err(StatusCode::UNAUTHORIZED, "invalid credentials"));
    };
    if !ok {
        return Err(err(StatusCode::UNAUTHORIZED, "invalid credentials"));
    }
    // Getting it right is what the window was counting towards. Leaving the
    // typos on the clock would refuse the next sign-in from a player who has
    // just proved who they are.
    state.sign_in_limiter.forget(&budget);
    // An unconfirmed address no longer keeps anyone out (#269): the account
    // signs in with its name, and the address is only a way to reach them.
    let account_id = account.id;
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
        // Its own name, to the one player who may see it: a player who signed
        // in with an address is told the username they were given (#269).
        "username": account.username,
    })))
}

/// What `POST /auth/guest` takes (#269). Every field is optional, and an
/// empty body is a guest called `Guest`.
#[derive(Deserialize, Default)]
struct GuestBody {
    /// The name other players see, under the display-name rule.
    #[serde(default)]
    display_name: Option<String>,
    /// The language it asks in.
    #[serde(default)]
    lang: String,
}

/// The name a guest is seen by when it chose none.
const GUEST_NAME: &str = "Guest";

/// `POST /auth/guest`: an account with no username, no address and no
/// password, and its session, in one answer (#269).
///
/// The session is the guest: nothing signs in as one, so it lives as long
/// as its session does ([`auth::Lifetime::GUEST`], 29 to 30 days from the
/// last call made with its token) and goes with it, decks included
/// ([`store::purge_guests`]). Bounded twice: per address by the limiter registration uses, and in all
/// by [`AppState::guest_cap`]. The count is read before the write, so a
/// burst of requests at the edge can pass it by as many as are in flight;
/// the cap bounds a flood, not a queue.
async fn guest(
    State(state): State<Shared>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<GuestBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    if !state.guests_enabled {
        return Err(err(StatusCode::FORBIDDEN, "this gateway takes no guests"));
    }
    if !state
        .limiter
        .allow(&rate_limit_ip(&state.trusted_proxies, addr.ip(), &headers))
    {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many attempts"));
    }
    let display_name = body
        .display_name
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| GUEST_NAME.to_owned());
    if !auth::valid_display_name(&display_name) {
        return Err(err(StatusCode::BAD_REQUEST, "invalid display name"));
    }
    if let Some(cap) = state.guest_cap {
        let guests = store::guest_count(&state.db)
            .await
            .map_err(|e| db_down(&e))?;
        if guests >= cap {
            return Err(err(
                StatusCode::SERVICE_UNAVAILABLE,
                "no guest seats free, sign up or try later",
            ));
        }
    }
    let issued = auth::IssuedToken::lasting(auth::Lifetime::GUEST);
    let account = store::create_guest(
        &state.db,
        store::NewGuest {
            display_name,
            created_at: auth::now_secs(),
            lang: body.lang,
        },
        auth::token_digest(&issued.token),
        issued.expires_at,
    )
    .await
    .map_err(|e| db_down(&e))?;
    Ok(Json(serde_json::json!({
        "token": issued.token,
        "expires_at": issued.expires_at,
        "guest": true,
        "handle": handle::handle(&account.display_name, account.tag),
    })))
}

/// Resolves the bearer token to an account id.
///
/// One indexed read per authenticated request, and a write only once the
/// session is due a renewal — see `store::resolve_token` for why the
/// sliding expiry stopped sliding on every request.
async fn authed(
    state: &Shared,
    headers: &HeaderMap,
) -> Result<String, (StatusCode, Json<ErrorBody>)> {
    authed_session(state, headers)
        .await
        .map(|session| session.account_id)
}

/// Resolves the bearer token to its session: the account, and whether it
/// is a guest's, for the routes a guest may not use.
async fn authed_session(
    state: &Shared,
    headers: &HeaderMap,
) -> Result<store::Session, (StatusCode, Json<ErrorBody>)> {
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

/// How many printings or cards one text request may ask for.
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
    /// Comma-separated Scryfall oracle ids: text by card.
    oracle_ids: Option<String>,
    /// Comma-separated Scryfall printing ids: text by printing, answered
    /// under the id asked for. Kept for clients that predate `oracle_ids`.
    ids: Option<String>,
    /// Preferred language; English is the fallback.
    lang: Option<String>,
}

/// The well-formed ids in a comma-separated list, lowercased, at most
/// [`MAX_TEXT_IDS`].
///
/// They are bound parameters, so this is not about injection: one malformed
/// id would fail the cast for the whole batch and cost every other card its
/// text.
fn text_ids(list: &str) -> Vec<String> {
    list.split(',')
        .map(str::trim)
        .filter(|id| uuid::Uuid::parse_str(id).is_ok())
        .map(str::to_lowercase)
        .take(MAX_TEXT_IDS)
        .collect()
}

/// Card text for a set of cards (`oracle_ids=`) or printings (`ids=`).
///
/// Deliberately unauthenticated. This is public reference data — Scryfall
/// serves the same thing without a token — and a client has to be able to draw
/// a readable card before it has an account, which is exactly the case when a
/// card image fails to load on first launch.
///
/// Asked by card, a card the catalog lacks is simply not answered: the
/// client has the English Oracle compiled in and asks Scryfall itself.
/// Asked by printing, a printing it lacks is fetched once and kept, as it
/// always was for the clients that still ask that way.
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

    if let Some(list) = params.oracle_ids.as_deref() {
        let found = card_text(&state, catalog, &text_ids(list), &lang)
            .await
            .map_err(|e| catalog_error("looking up card text", &e))?;
        return Ok(Json(found));
    }

    let ids = text_ids(params.ids.as_deref().unwrap_or_default());
    if ids.is_empty() {
        return Ok(Json(Vec::new()));
    }
    let mut asked = catalog
        .cards_of(&ids)
        .await
        .map_err(|e| catalog_error("looking up card text", &e))?;

    // Anything the catalog has never seen is fetched once and kept.
    let missing: Vec<String> = ids
        .iter()
        .filter(|id| !asked.iter().any(|(known, _)| known == *id))
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
        if let Err(err) = catalog.upsert(&fetched).await {
            tracing::warn!(%err, "storing on-demand cards failed");
        } else if !fetched.is_empty() {
            let filled = catalog
                .cards_of(&missing)
                .await
                .map_err(|e| catalog_error("looking up card text", &e))?;
            // Not a data stamp: see `texts` for why a fill re-reads its own
            // cards instead.
            let cards: Vec<String> = filled.iter().map(|(_, card)| card.clone()).collect();
            if let Err(err) = state.texts.refresh(catalog, &cards).await {
                tracing::warn!(%err, "re-reading filled cards failed");
            }
            asked.extend(filled);
        }
    }

    let mut cards: Vec<String> = asked.iter().map(|(_, card)| card.clone()).collect();
    cards.sort_unstable();
    cards.dedup();
    let by_card: std::collections::HashMap<String, baylee_catalog::CardTextEntry> =
        card_text(&state, catalog, &cards, &lang)
            .await
            .map_err(|e| catalog_error("looking up card text", &e))?
            .into_iter()
            .map(|entry| (entry.oracle_id.clone(), entry))
            .collect();
    Ok(Json(
        asked
            .into_iter()
            .filter_map(|(scryfall_id, card)| {
                Some(baylee_catalog::CardTextEntry {
                    scryfall_id,
                    ..by_card.get(&card)?.clone()
                })
            })
            .collect(),
    ))
}

/// Text for `cards` in `lang`, in oracle-id order: a pool card's from the
/// language held in memory, any other from the catalog.
async fn card_text(
    state: &AppState,
    catalog: &baylee_catalog::Catalog,
    cards: &[String],
    lang: &str,
) -> anyhow::Result<Vec<baylee_catalog::CardTextEntry>> {
    if cards.is_empty() {
        return Ok(Vec::new());
    }
    let language = state.texts.get(catalog, lang).await?;
    let mut found = Vec::with_capacity(cards.len());
    let mut rest = Vec::new();
    for card in cards {
        match language.entry(card) {
            Some(entry) => found.push(entry.clone()),
            None => rest.push(card.clone()),
        }
    }
    if !rest.is_empty() {
        found.extend(catalog.text_by_card(&rest, language.lang()).await?);
    }
    found.sort_by(|a, b| a.oracle_id.cmp(&b.oracle_id));
    found.dedup_by(|a, b| a.oracle_id == b.oracle_id);
    Ok(found)
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
    let owner = store::drop_token(&state.db, token)
        .await
        .map_err(|e| db_down(&e))?;
    // A guest's session is the guest (#269): signed out, nothing can reach
    // it again, so it goes now rather than at the next sweep. The client
    // asked the player first. An account's is left alone: the query deletes
    // guests only.
    if let Some(account_id) = owner {
        store::purge_guests(&state.db, auth::now_secs(), Some(&account_id))
            .await
            .map_err(|e| db_down(&e))?;
    }
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
        // Only ever to its owner: nothing that shows one player to another
        // carries it (#269).
        "username": account.username,
        // A guest's client says what a guest is (#269): gone about thirty
        // days after its last visit.
        "guest": account.guest,
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
    /// The deck's commanders: none, one, or two under the partner rule.
    #[serde(default)]
    commanders: Vec<String>,
    /// The one commander an older client sends.
    ///
    /// A reader's tolerance, not a second field: a build made before the
    /// partner rule existed still saves decks, and refusing it would lose
    /// the commander rather than the feature. Nothing writes this back.
    #[serde(default)]
    commander: Option<String>,
    /// What the deck plays. A body that does not say keeps what the deck
    /// already said, and a new deck that does not say is read off its
    /// commander — the same sentence `baylee_cards::decks::format_of` reads.
    #[serde(default)]
    format: Option<String>,
    /// What the deck is for, in its owner's words. Absent leaves it alone;
    /// an empty string clears it.
    #[serde(default)]
    description: Option<String>,
    /// What this change was called, for the deck's history.
    ///
    /// Only read when the cards actually change — a save that renames the
    /// deck writes no version, so there is nothing for a summary to be
    /// attached to.
    #[serde(default)]
    summary: Option<String>,
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

impl DeckBody {
    /// The commanders this request names, however it named them.
    ///
    /// One list out of two fields: `commanders` is what a current client
    /// sends and `commander` is what an older one sends, and a body that
    /// somehow carries both is read as the list — the newer field is the one
    /// that can say everything the older one can.
    fn commander_names(&self) -> Vec<String> {
        if self.commanders.is_empty() {
            self.commander.clone().into_iter().collect()
        } else {
            self.commanders.clone()
        }
    }
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
/// Which of the two facts a name the pool does not know actually stands for.
///
/// `baylee_cards::decks::by_name` answers `None` for a typo and for Black
/// Lotus alike, and the second is the common one: this build compiles 2716 of
/// the ledger's 33 694 cards, so **92 %** of the real cards a player might
/// type are real and unavailable here. Telling that player `unknown card`
/// sends them hunting a spelling mistake they did not make — the one thing
/// the message rules out is the thing that is true.
///
/// The ledger is what can tell them apart, because it numbers every card
/// there is rather than this pool. It is asked only here, on a path that has
/// already missed in the pool's perfect hash, so a deck that imports cleanly
/// never reaches it at all.
///
/// It does not make such a card playable and does not hint that it might: the
/// deck is refused either way, and only the reason changes. `if_no_card` is
/// what to say when the name is nothing, which differs by where it was
/// written.
/// What a real card this build compiles nothing for is called, wherever it
/// is refused. One string, because a player who meets it twice through two
/// routes has met one problem.
const EXISTS_UNPLAYABLE: &str = "that card exists but this server cannot play it";

fn no_such_card(name: &str, if_no_card: &'static str) -> &'static str {
    if baylee_cards_index::row_by_name(name).is_some() {
        EXISTS_UNPLAYABLE
    } else {
        if_no_card
    }
}

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
            baylee_core::deckrow::RowError::Note => {
                err(StatusCode::BAD_REQUEST, "card note too long")
            }
            baylee_core::deckrow::RowError::Shape => {
                err(StatusCode::BAD_REQUEST, "malformed card line")
            }
        })?;
        let count = row.count;
        let Some(index) = baylee_cards::decks::by_name(&row.name) else {
            return Err(err(
                StatusCode::BAD_REQUEST,
                no_such_card(&row.name, "unknown card"),
            ));
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
    // Three questions, not one. Every name has to resolve, every card it
    // resolves to has to be allowed to lead a deck (CR 903.3) — the check
    // used to stop at the first, so any card in the pool could be named as a
    // commander and the engine would seat it without complaint — and a
    // second one has to be allowed to lead it *with the first*.
    let named = body.commander_names();
    if named.len() > 2 {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "a deck has at most two commanders",
        ));
    }
    let mut leaders = Vec::with_capacity(named.len());
    for name in &named {
        let Some(index) = baylee_cards::decks::by_name(name) else {
            return Err(err(
                StatusCode::BAD_REQUEST,
                no_such_card(name, "unknown commander"),
            ));
        };
        let Some(leader) = baylee_cards::decks::leader_of(index) else {
            return Err(err(StatusCode::BAD_REQUEST, "unknown commander"));
        };
        if !leader.eligible {
            return Err(err(
                StatusCode::BAD_REQUEST,
                "that card cannot be a commander",
            ));
        }
        leaders.push(leader);
    }
    if let [first, second] = leaders.as_slice()
        && !baylee_cards::decks::may_lead_together(first, second)
    {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "those two cards cannot lead one deck",
        ));
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
                "commanders": d.commanders,
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
    let deck = readable_deck(&state, &id, &account_id).await?;
    Ok(Json(serde_json::json!({
        "id": deck.id,
        "kind": deck.kind,
        "name": deck.name,
        "format": deck.format,
        "description": deck.description,
        "version": deck.version,
        "copied_from": deck.origin.as_ref().map(|(deck, _)| deck.clone()),
        "copied_version": deck.origin.as_ref().map(|(_, version)| *version),
        "cards": deck.cards,
        "sideboard": deck.sideboard,
        "commanders": deck.commanders,
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
    let commanders = body.commander_names();
    let format = body
        .format
        .clone()
        .unwrap_or_else(|| format_of(&commanders));
    let id = store::create_deck(
        &state.db,
        store::NewDeck {
            account_id,
            name: body.name,
            format,
            description: body.description.filter(|d| !d.is_empty()),
            origin: None,
            cards: body.cards,
            sideboard: body.sideboard,
            commanders,
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
    let commanders = body.commander_names();
    let format = body.format.clone().unwrap_or_else(|| deck.format.clone());
    let description = match body.description {
        // An empty string is somebody clearing the field; leaving it out is
        // somebody saving a deck without touching it.
        Some(text) if text.is_empty() => None,
        Some(text) => Some(text),
        None => deck.description.clone(),
    };
    store::put_deck(
        &state.db,
        Deck {
            name: body.name,
            format,
            description,
            cards: body.cards,
            sideboard: body.sideboard,
            commanders,
            sleeve: body.sleeve,
            playmat: body.playmat,
            updated_at: auth::now_secs(),
            ..deck
        },
        body.summary.filter(|s| !s.is_empty()),
    )
    .await
    .map_err(|e| db_down(&e))?;
    Ok(StatusCode::NO_CONTENT)
}

/// Everything a deck is reached through, in one place.
///
/// Split off the router `main` builds because there are now eight of them —
/// the deck itself, the shared decks anybody may take, and the history. It
/// is also the one place where the order matters: `/decks/shared` is a
/// static segment and has to be matched as one rather than as a deck whose
/// id is the word "shared".
fn deck_routes() -> Router<Shared> {
    Router::new()
        .route("/decks", get(list_decks).post(create_deck))
        .route("/decks/shared", get(list_shared_decks))
        .route(
            "/decks/{id}",
            get(get_deck).put(update_deck).delete(delete_deck),
        )
        .route("/decks/{id}/copy", post(copy_deck))
        .route("/decks/{id}/history", get(deck_history))
        .route("/decks/{id}/versions/{version}", get(deck_version))
        .route("/decks/{id}/versions/{version}/revert", post(revert_deck))
}

/// What a deck plays, when nobody said.
///
/// A deck that named a commander is playing Commander — the same sentence
/// `baylee_cards::decks::format_of` reads off a loaded deck, said here about
/// a deck that is only a list of rows so far.
fn format_of(commanders: &[String]) -> String {
    if commanders.is_empty() {
        "freeform".to_string()
    } else {
        "commander".to_string()
    }
}

/// The decks anybody may play: what this project publishes and what came in
/// a box.
///
/// Readable by any signed-in account and owned by none of them, which is
/// what makes them the thing a copy starts from.
async fn list_shared_decks(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let _ = authed(&state, &headers).await?;
    let decks: Vec<_> = store::shared_decks(&state.db)
        .await
        .map_err(|e| db_down(&e))?
        .iter()
        .map(|d| {
            serde_json::json!({
                "id": d.id,
                "kind": d.kind,
                "name": d.name,
                "format": d.format,
                "description": d.description,
                "cards": d.cards.len(),
                "sideboard": d.sideboard.len(),
                "commanders": d.commanders,
                "version": d.version,
            })
        })
        .collect();
    Ok(Json(serde_json::json!(decks)))
}

/// One deck's history: every state it no longer holds, newest first.
///
/// The current cards are **not** in the list — they are the deck, and
/// `version` says which number they carry. A caller drawing a timeline puts
/// the deck at the top and these underneath it.
async fn deck_history(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let deck = readable_deck(&state, &id, &account_id).await?;
    let past: Vec<_> = store::deck_history(&state.db, &id)
        .await
        .map_err(|e| db_down(&e))?
        .iter()
        .map(|v| {
            serde_json::json!({
                "version": v.version,
                "cards": v.cards.len(),
                "sideboard": v.sideboard.len(),
                "commanders": v.commanders,
                "summary": v.summary,
                "superseded_at": v.superseded_at,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({
        "version": deck.version,
        "updated_at": deck.updated_at,
        "past": past,
    })))
}

/// One superseded state, in full.
async fn deck_version(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path((id, version)): Path<(String, i32)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let deck = readable_deck(&state, &id, &account_id).await?;
    if version == deck.version {
        // The current state is not in the history table and never will be.
        // Answering it from the deck itself is what makes "show me version
        // N" a question the caller can ask about any N it was given.
        return Ok(Json(serde_json::json!({
            "version": deck.version,
            "cards": deck.cards,
            "sideboard": deck.sideboard,
            "commanders": deck.commanders,
            "current": true,
        })));
    }
    let past = store::deck_at_version(&state.db, &id, version)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such version"))?;
    Ok(Json(serde_json::json!({
        "version": past.version,
        "cards": past.cards,
        "sideboard": past.sideboard,
        "commanders": past.commanders,
        "summary": past.summary,
        "superseded_at": past.superseded_at,
        "current": false,
    })))
}

/// Put an earlier state back, as a new change.
///
/// Not a rewind: the deck's present is archived exactly as any other save
/// archives it, and the old lists become the new head one version higher.
/// So a revert can itself be reverted, and nothing in the history is ever
/// removed or rewritten.
async fn revert_deck(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path((id, version)): Path<(String, i32)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let deck = store::deck(&state.db, &id)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such deck"))?;
    if deck.account_id != account_id {
        return Err(err(StatusCode::FORBIDDEN, "not your deck"));
    }
    if version == deck.version {
        return Err(err(
            StatusCode::CONFLICT,
            "that is the deck's current state",
        ));
    }
    let past = store::deck_at_version(&state.db, &id, version)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such version"))?;
    let next = deck.version + 1;
    store::put_deck(
        &state.db,
        Deck {
            cards: past.cards,
            sideboard: past.sideboard,
            commanders: past.commanders,
            updated_at: auth::now_secs(),
            ..deck
        },
        Some(format!("zurück auf Version {version}")),
    )
    .await
    .map_err(|e| db_down(&e))?;
    Ok(Json(serde_json::json!({ "version": next })))
}

/// Take a copy of a deck anybody may play, as an account's own.
///
/// The copy is an ordinary account deck from the moment it exists — its own
/// history starts at version 1 — and it remembers which deck and which of
/// that deck's versions it came from, so "this is the Kess precon as it was"
/// survives the original moving on.
async fn copy_deck(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let source = readable_deck(&state, &id, &account_id).await?;
    let copy = store::create_deck(
        &state.db,
        store::NewDeck {
            account_id,
            name: source.name.clone(),
            format: source.format.clone(),
            description: source.description.clone(),
            origin: Some((source.id.clone(), source.version)),
            cards: source.cards.clone(),
            sideboard: source.sideboard.clone(),
            commanders: source.commanders.clone(),
            // The sleeve and the mat are pictures somebody uploaded, and the
            // image store hands them out by account. A copy starts with the
            // generated back rather than a reference it may not be able to
            // read.
            sleeve: None,
            playmat: None,
            updated_at: auth::now_secs(),
        },
    )
    .await
    .map_err(|e| db_down(&e))?
    .ok_or_else(|| err(StatusCode::INTERNAL_SERVER_ERROR, "account gone"))?;
    Ok(Json(serde_json::json!({ "deck_id": copy })))
}

/// A deck this account may read: their own, or one that belongs to nobody.
///
/// The two are one question because every route that shows a deck asks it,
/// and a route that asked only the first would make a house deck invisible
/// to the player it was published for.
async fn readable_deck(
    state: &Shared,
    id: &str,
    account_id: &str,
) -> Result<store::Deck, (StatusCode, Json<ErrorBody>)> {
    let deck = store::deck(&state.db, id)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such deck"))?;
    if deck.kind == baylee_db::entity::deck::KIND_ACCOUNT && deck.account_id != account_id {
        return Err(err(StatusCode::FORBIDDEN, "not your deck"));
    }
    Ok(deck)
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
    /// Which clock this table plays at, by name. See `clock::PRESETS`.
    #[serde(default)]
    clock: Option<String>,
    /// Seconds to answer one question, overriding whatever `clock` gave.
    #[serde(default)]
    decision_timeout_secs: Option<u32>,
    /// Seconds a seat may be gone before the house answers for it, same.
    #[serde(default)]
    reconnect_window_secs: Option<u32>,
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
    //
    // Each leader in turn, so a deck led by two of them loses both from the
    // library and neither of them twice.
    let commanders = deck
        .commanders
        .iter()
        .filter_map(|name| baylee_cards::decks::by_name(name))
        .map(|index| match main.iter().position(|c| c.index == index) {
            Some(at) => main.remove(at),
            None => baylee_cards::decks::DeckCard::plain(index),
        })
        .collect();
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
        let mut preset = room_preset(&game.seats, auth::new_game_seed())?;
        // The room's clock, onto the preset the engine is about to be given.
        // This is the one line the whole ticket was missing: the wire already
        // carried `HouseRules` — the gateway sends the preset as JSON and
        // gamehost has always decoded it — so every table ran the default
        // purely because nothing here ever wrote to this field.
        preset.house_rules = game.house_rules.clone();
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
    // Before anything is built, because a room refused for its clock should
    // cost nothing and leave nothing behind.
    let house_rules = clock::resolve(
        body.clock.as_deref(),
        body.decision_timeout_secs,
        body.reconnect_window_secs,
    )
    .map_err(|reason| err_saying(StatusCode::BAD_REQUEST, reason))?;

    // The one-tap game against the house AI keeps its own path: it is a whole
    // table decided in one request, and nobody is going to configure it.
    if body.mode == "ai" {
        {
            let mut preset = ai_preset(&deck, auth::new_game_seed())?;
            preset.house_rules = house_rules.clone();
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
            let mut game = LobbyGame::playing(game_id.clone(), seats, preset, auth::now_secs());
            game.house_rules = house_rules;
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
        game.house_rules = house_rules;
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

// --------------------------------------------------------------- settings

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
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "invalid or expired token"))?
        .account_id;
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
