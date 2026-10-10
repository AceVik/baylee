//! baylee-gateway — accounts, decks, lobby, and hosted games.
//!
//! Security posture (see auth.rs): Argon2id password hashing, hashed
//! bearer tokens with sliding expiry, auth rate limiting, generic
//! credential errors, constant-time comparisons. TLS terminates at the
//! reverse proxy in front of this process (Caddy/nginx) — this service
//! must never be exposed on a plaintext listener in production.

mod account;
mod admin;
mod art;
mod auth;
mod chair;
mod clock;
mod cosmetics;
mod engine;
mod handle;
mod invite;
mod lobby;
mod mail;
mod namebook;
mod pool;
mod presence;
mod record;
mod recordexport;
mod report;
mod room;
mod routes;
mod seatrate;
mod store;
mod terms;
mod texts;
mod wsticket;

use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use axum::routing::{delete, get, post};
use baylee_protocol::names;
use baylee_protocol::v1::{self, Envelope};
use lobby::{Lobby, LobbyGame, LobbyState};
use parking_lot::Mutex;
use routes::{
    DeckBody, LobbyWsParams, MAX_SEATS, TICKET_REFUSED, WsParams, account_routes, ai_preset,
    auth_config, bearer_token, catalog_search, catalog_text, check_pictures, connect_catalog,
    create_game, deck_routes, display_name, expand, game_cosmetics, game_ws, get_settings,
    hand_over, health, house_deck, info, is_bidi_control, join_game, leave_game, legacy_token,
    list_games, listing, listing_page, lobby_stats, lobby_ws, own_deck, parse_deck_lines,
    put_settings, rematch, seat_names, seat_of_token, set_ready, set_seat, source, source_url,
    spend_ticket, start_room, table_prints, take_seat, try_start, validate_deck,
    validate_the_dev_board, watch_ws, ws_ticket,
};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use store::{Confirmation, Deck, StoredToken};
use tokio::sync::broadcast::error::RecvError;
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
    /// Who may register (`BAYLEE_REGISTRATION`): anybody, somebody with a
    /// closed-beta key (`invite`, #317), or nobody (`off`).
    registration: invite::Registration,
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
    /// Where this build's source can be had (`BAYLEE_SOURCE_URL`, else the
    /// repository it was built from), already checked by [`source_url`].
    /// The AGPL's §13 offer, which `/source` and `/info` both carry.
    source_url: String,
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
    /// The ids of accounts just deleted (#292), for the lobby sockets
    /// signed in as them to close. A lobby socket resolves its session once,
    /// when it opens, so nothing else would tell it.
    departed: tokio::sync::broadcast::Sender<String>,
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
    /// Where `POST /reports` sends a report (#307), from
    /// `BAYLEE_FEEDBACK_URL`, `BAYLEE_FEEDBACK_TOKEN`, `BAYLEE_FEEDBACK_KEY`,
    /// and how many each account has sent.
    feedback: report::Feedback,
    /// Reports waiting for a game's engine to send its record as it stands
    /// (#323); see `record.rs`.
    record_flushes: record::Flushes,
    /// The unspent tickets a socket may be opened with (#294), in memory
    /// only; `BAYLEE_WS_TICKET_SECS` says how long each lives.
    tickets: wsticket::Tickets,
    /// Until when (unix seconds) a socket may still be opened with its
    /// token in the query, as clients from before #294 do
    /// ([`wsticket::LEGACY_UNTIL`], or `0` for `BAYLEE_WS_LEGACY_TOKENS=off`).
    legacy_until: u64,
    /// The unspent chair tickets hosts handed to their seat bridges
    /// (`chair.rs`), in memory only; `BAYLEE_CHAIR_TICKET_SECS` says how
    /// long each lives.
    chair_tickets: wsticket::Tickets,
    /// Chair tickets asked for, per host, and redemptions tried, per
    /// address (`chair.rs`).
    chair_limiter: auth::RateLimiter,
    /// Whether hosts may hand chairs to seat bridges at all
    /// (`BAYLEE_CHAIR_TICKETS=off` to disable), read as `BAYLEE_GUESTS` is.
    chair_tickets_enabled: bool,
    /// The handles of the accounts seated in the lobby, read once each
    /// (`namebook.rs`).
    names: namebook::NameBook,
    /// The accounts with a lobby socket open, as a count for
    /// `GET /lobby/stats` (`presence.rs`); memory only.
    presence: presence::Presence,
    /// When this process started serving (unix seconds), for the admin
    /// console's uptime.
    started_at: u64,
    /// The terms of use a player accepts (`BAYLEE_TERMS_PATH`, WG-1), read
    /// once at start; `None` when the gateway has none.
    terms: Option<terms::Terms>,
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
#[allow(clippy::too_many_lines)] // one statement per setting and route, read top to bottom
async fn main() {
    // The operator's command for closed-beta keys (#317), before anything
    // else: it prints keys on standard output, where a log line would be
    // read as one, and it needs no port, agent or setting but the database.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("invite") {
        std::process::exit(invite::cli(&args[1..]).await);
    }
    // The anonymised export of game records for training and balancing,
    // likewise the operator's and the database's alone.
    if args.first().map(String::as_str) == Some("records") {
        std::process::exit(recordexport::cli(&args[1..]).await);
    }
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
    let source_url = source_url(std::env::var("BAYLEE_SOURCE_URL").ok().as_deref())
        .unwrap_or_else(|why| panic!("BAYLEE_SOURCE_URL: {why}"));
    let guest_cap = guest_cap(std::env::var("BAYLEE_GUEST_CAP").ok().as_deref())
        .unwrap_or_else(|why| panic!("BAYLEE_GUEST_CAP: {why}"));
    let ticket_ttl = wsticket::ttl_from_env(std::env::var("BAYLEE_WS_TICKET_SECS").ok().as_deref())
        .unwrap_or_else(|why| panic!("BAYLEE_WS_TICKET_SECS: {why}"));
    let chair_ticket_ttl = wsticket::lifetime_from_env(
        std::env::var("BAYLEE_CHAIR_TICKET_SECS").ok().as_deref(),
        chair::DEFAULT_SECS,
    )
    .unwrap_or_else(|why| panic!("BAYLEE_CHAIR_TICKET_SECS: {why}"));
    let legacy_until =
        wsticket::legacy_from_env(std::env::var("BAYLEE_WS_LEGACY_TOKENS").ok().as_deref())
            .unwrap_or_else(|why| panic!("BAYLEE_WS_LEGACY_TOKENS: {why}"));
    let terms = terms::from_env(std::env::var_os("BAYLEE_TERMS_PATH").as_deref())
        .unwrap_or_else(|why| panic!("BAYLEE_TERMS_PATH: {why}"));
    let var = |name: &str| std::env::var(name).ok();
    let console = admin::Settings::from_env(
        var("BAYLEE_ADMIN_TOKEN").as_deref(),
        var("BAYLEE_ADMIN_BIND").as_deref(),
        &[
            var("BAYLEE_AGENT_TOKEN").as_deref(),
            var("BAYLEE_FEEDBACK_TOKEN").as_deref(),
            var("BAYLEE_FEEDBACK_KEY").as_deref(),
        ],
    )
    .unwrap_or_else(|why| panic!("{why}"));
    let store_path = std::env::var("STORE_PATH")
        .map_or_else(|_| PathBuf::from("gateway-store.json"), PathBuf::from);
    let db = open_database(&store_path).await;
    let catalog = connect_catalog(db.clone()).await;
    // Bound after the database and the catalog, and before anything that has
    // to know the port: with `PORT=0` the kernel chooses it (#279), and the
    // address an engine is told to dial back on below must be that one.
    let (listener, port) = listen(port).await;
    // The admin console's own listener (`admin.rs`), loopback only and
    // bound beside the public one, so a taken port stops the start.
    let console_listener = match &console {
        Some(settings) => Some(admin::bind(settings).await),
        None => None,
    };
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
        departed: tokio::sync::broadcast::channel(16).0,
        agents: Mutex::new(engine::Agents::default()),
        agent_token,
        engine_url: std::env::var("BAYLEE_ENGINE_URL")
            .unwrap_or_else(|_| format!("ws://127.0.0.1:{port}/engine/ws")),
        registration: invite::Registration::from_env(
            std::env::var("BAYLEE_REGISTRATION").ok().as_deref(),
        ),
        guests_enabled: switched_on(std::env::var("BAYLEE_GUESTS").ok().as_deref()),
        guest_cap,
        display_name,
        source_url,
        mail: mail::Mailer::from_env(),
        trusted_proxies: trusted_proxies(
            &std::env::var("BAYLEE_TRUSTED_PROXIES").unwrap_or_default(),
        ),
        catalog,
        texts: texts::TextCache::default(),
        art: Arc::new(art::ArtCache::from_env()),
        deck_images: Arc::new(cosmetics::Store::from_env()),
        feedback: report::Feedback::from_env(),
        record_flushes: record::Flushes::default(),
        tickets: wsticket::Tickets::new(ticket_ttl),
        legacy_until,
        chair_tickets: wsticket::Tickets::new(chair_ticket_ttl),
        chair_limiter: auth::RateLimiter::new(chair::LIMIT_WINDOW, chair::LIMIT_TRIES),
        chair_tickets_enabled: switched_on(std::env::var("BAYLEE_CHAIR_TICKETS").ok().as_deref()),
        names: namebook::NameBook::default(),
        presence: presence::Presence::default(),
        started_at: auth::now_secs(),
        terms,
    });
    // Before serving, so it is done by the time anybody can upload (#301).
    account::sweep_pictures(&state).await;
    spawn_cleanup(state.clone());
    spawn_ticket_sweep(state.clone());
    if let (Some(settings), Some((listener, admin_port))) = (&console, console_listener) {
        admin::serve(listener, state.clone(), settings);
        // Before `BAYLEE_PORT_FILE`, which says the gateway is up: whoever
        // waits for that finds this one written too.
        if let Some(path) = std::env::var_os("BAYLEE_ADMIN_PORT_FILE") {
            write_port_file(std::path::Path::new(&path), admin_port)
                .unwrap_or_else(|e| panic!("BAYLEE_ADMIN_PORT_FILE {}: {e}", path.display()));
        }
    }

    let app = Router::new()
        .route("/health", get(health))
        .route("/source", get(source))
        .route("/info", get(info))
        .merge(account_routes())
        .merge(deck_routes())
        .merge(report::routes())
        .route("/lobby/games", get(list_games).post(create_game))
        .route("/lobby/stats", get(lobby_stats))
        .route("/lobby/games/{id}/join", post(join_game))
        .route("/lobby/games/{id}/configure", post(room::configure))
        .route("/lobby/games/{id}/seat", post(take_seat))
        .route("/lobby/games/{id}/seats/{seat}", post(set_seat))
        .route("/lobby/games/{id}/ready", post(set_ready))
        .route("/lobby/games/{id}/start", post(start_room))
        .route("/lobby/games/{id}/host", post(hand_over))
        .route("/lobby/games/{id}/leave", post(leave_game))
        .route("/lobby/games/{id}/rematch", post(rematch))
        .route("/lobby/games/{id}/chairs/{seat}/ticket", post(chair::mint))
        .route(
            "/lobby/games/{id}/chairs/{seat}/redeem",
            post(chair::redeem),
        )
        .route("/lobby/games/{id}/chair", get(chair::status))
        .route("/lobby/games/{id}/chair/leave", post(chair::leave))
        .route("/lobby/games/{id}/chair/ready", post(chair::ready))
        .route("/ws-ticket", post(ws_ticket))
        .route("/lobby/ws", get(lobby_ws))
        .route("/games/{id}/ws", get(game_ws))
        .route("/games/{id}/watch", get(watch_ws))
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

    #[cfg(unix)]
    if let Some(path) = std::env::var_os("BAYLEE_UNIX_SOCKET").filter(|p| !p.is_empty()) {
        serve_unix(std::path::Path::new(&path), app.clone());
    }
    announce(port);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("gateway serves");
}

/// Serves the same routes on a unix socket as well (`BAYLEE_UNIX_SOCKET`).
///
/// For the agent on this machine and the engines it starts: a socket file
/// the operating system guards by owner and group, not a port anybody on the
/// host could dial. Whatever arrives here is marked [`engine::ViaUnix`], and
/// that mark is what "local" means to `/health` and to a deploy: the games
/// this machine's agent runs, as opposed to an agent somewhere else.
///
/// A peer on a unix socket has no address, and the routes that limit by
/// address still ask for one, so every request here reads as loopback. It
/// is: the file is only reachable from this host.
///
/// A file left behind by an earlier run is removed first, since binding over
/// it fails; the new one is readable and writable by owner and group only
/// (0660).
#[cfg(unix)]
fn serve_unix(path: &std::path::Path, app: Router) {
    use std::os::unix::fs::PermissionsExt as _;
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => panic!("BAYLEE_UNIX_SOCKET {}: {e}", path.display()),
    }
    let listener = tokio::net::UnixListener::bind(path)
        .unwrap_or_else(|e| panic!("BAYLEE_UNIX_SOCKET {}: {e}", path.display()));
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o660))
        .unwrap_or_else(|e| panic!("BAYLEE_UNIX_SOCKET {}: {e}", path.display()));
    let loopback = SocketAddr::from(([127, 0, 0, 1], 0));
    let app = app
        .layer(axum::Extension(ConnectInfo(loopback)))
        .layer(axum::Extension(engine::ViaUnix));
    tracing::info!(path = %path.display(), "baylee-gateway listening on a unix socket");
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("gateway serves its unix socket");
    });
}

/// The gateway's TCP listener: every connection it accepts sends with
/// Nagle's algorithm off ([`send_at_once`]).
type Listening = axum::serve::TapIo<tokio::net::TcpListener, fn(&mut tokio::net::TcpStream)>;

/// Binds the gateway's port, and answers which port that is: the one asked
/// for, or with `PORT=0` the one the kernel chose.
async fn listen(port: u16) -> (Listening, u16) {
    use axum::serve::ListenerExt as _;
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("bind gateway port");
    let port = listener
        .local_addr()
        .expect("a bound listener has an address")
        .port();
    (listener.tap_io(send_at_once as fn(&mut _)), port)
}

/// Turns Nagle's algorithm off on an accepted connection (`TCP_NODELAY`).
///
/// The gateway relays frames that are small and follow each other closely:
/// an engine's view and then the question about it, forwarded to a seat. With
/// Nagle on, the second waits until the peer acknowledges the first, and
/// Linux holds an acknowledgement back for up to 40 ms: in a Linux container
/// the language-model seat's test game (`e2e_seat_llm`) took 49 ms an action
/// with Nagle on and 5 ms with it off (30.09.2026).
fn send_at_once(stream: &mut tokio::net::TcpStream) {
    if let Err(e) = stream.set_nodelay(true) {
        tracing::debug!(error = %e, "TCP_NODELAY was not set on a connection");
    }
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

    // The precons this build plays, offered beside the house decks
    // (`docs/precons.md` §"House decks"). Not fatal, as the import below is
    // not: a sync that fails leaves the offer as the last one left it, and
    // every account and deck is still there to serve.
    match baylee_db::precons::sync(&db, baylee_db::precons::PLAYABLE).await {
        Ok(done) if done.unchanged => {
            tracing::info!(offered = done.offered, "precons unchanged");
        }
        Ok(done) => tracing::info!(
            offered = done.offered,
            added = done.added,
            changed = done.changed,
            returned = done.returned,
            withdrawn = done.withdrawn,
            "precons synced"
        ),
        Err(e) => tracing::error!("precon sync: {e:#}"),
    }

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
/// waiting lobbies, expired sessions and the guests they were the way into,
/// and expired confirmation links (#293) — all of them grew without bound.
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
            match store::sweep_confirmations(&state.db, now).await {
                Ok(swept) if swept > 0 => {
                    tracing::debug!(swept, "expired confirmation links swept");
                }
                Ok(_) => {}
                Err(e) => tracing::warn!("{e:#}"),
            }
            // After the sessions, because a guest goes with its last one
            // (#269), and out of the lobby with it (#292).
            match account::depart(&state, store::purge_guests(&state.db, now, None)).await {
                Ok(gone) if !gone.accounts.is_empty() => {
                    tracing::info!(purged = gone.accounts.len(), "idle guests deleted");
                }
                Ok(_) => {}
                Err(e) => tracing::warn!("{e:#}"),
            }
        }
    });
}

/// Drops expired socket tickets (#294) once a minute.
///
/// Not the ten-minute loop above: a ticket lives 45 seconds by default, and
/// what this bounds is how long a dead one takes up room. The store is
/// bounded without it ([`wsticket::MAX_OUTSTANDING`], and it sweeps itself
/// when full); this keeps it small in the ordinary case.
fn spawn_ticket_sweep(state: Shared) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.tick().await;
        loop {
            interval.tick().await;
            let now = std::time::Instant::now();
            let swept = state.tickets.sweep(now) + state.chair_tickets.sweep(now);
            if swept > 0 {
                tracing::debug!(swept, "expired socket tickets swept");
            }
        }
    });
}

// ------------------------------------------------------------------- auth

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

/// Whether a switch is on: whether this gateway lets one play as a guest
/// (`BAYLEE_GUESTS`, #269). `BAYLEE_REGISTRATION` has three answers since
/// #317 ([`invite::Registration::from_env`]) and reads its two old ones the
/// same way, so that an operator still learns one reading.
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
    let token = bearer_token(headers)
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

// ------------------------------------------------------------- cross-origin

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

#[cfg(test)]
mod tests;
