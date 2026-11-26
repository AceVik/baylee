//! Gateway startup; individual transports remain separate modules.
use super::{
    AppState, Arc, EnvFilter, GUESTS_A_DAY_PER_ADDRESS, LARGE_BODIES_PER_ADDRESS, Lobby, Mutex,
    PathBuf, Router, Shared, SocketAddr, account, account_routes, admin, admission, announce,
    archive_replay, art, auth, authmode, banlists, blocks, catalog_search, catalog_text, central,
    chair, challenges, clock, connect_catalog, cors, cosmetics, create_game, deck_routes,
    departure, display_name, engine, game_cosmetics, game_ws, get, guest_cap, hand_over, health,
    impressum_url, info, join_game, keep, launch, leave_game, list_games, list_replays, list_seats,
    listen, lobby_stats, lobby_ws, metrics, namebook, open_database, pool, poolfacts, post,
    presence, put, record, recordexport, registration, rematch, replay_tape, replays, report, room,
    routes, seathost, sessions, set_ready, set_seat, share_replay, social, source, source_url,
    spawn_cleanup, spawn_session_sweep, spawn_ticket_sweep, start_room, switched_on, take_seat,
    terms, texts, tower, trusted_proxies, validate_the_dev_board, watch_ws, whole_body,
    write_port_file, ws_ticket, wsticket,
};
mod router;

#[allow(clippy::too_many_lines)] // one statement per setting and route, read top to bottom
pub(super) async fn run() {
    // The command line first, before a setting is read, the database opened
    // or a port bound: it serves only when it says nothing (`launch.rs`).
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| {
            arg.into_string()
                .unwrap_or_else(|_| launch::refuse("an argument is not UTF-8"))
        })
        .collect();
    match launch::command(&args).unwrap_or_else(|why| launch::refuse(&why)) {
        launch::Command::Serve => {}
        launch::Command::Help => {
            println!("{}", launch::USAGE);
            return;
        }
        // The closed beta's key command (#317) is gone with its keys. Asked
        // for anyway (a wrapper an older deploy installed, a habit), it says
        // so and stops: it never starts a gateway in its place, which would
        // migrate the database the shell names.
        launch::Command::Invite => {
            eprintln!("{}", registration::INVITE_COMMAND_GONE);
            std::process::exit(2);
        }
        // The anonymised export of game records for training and balancing,
        // the operator's and the database's alone.
        launch::Command::Records(rest) => std::process::exit(recordexport::cli(rest).await),
    }
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(launch::DEFAULT_PORT);
    let bind = launch::bind_address(std::env::var("BAYLEE_BIND").ok().as_deref())
        .unwrap_or_else(|why| panic!("{why}"));
    validate_the_dev_board();
    let display_name = display_name(std::env::var("BAYLEE_GATEWAY_NAME").ok().as_deref())
        .unwrap_or_else(|why| panic!("BAYLEE_GATEWAY_NAME: {why}"));
    let source_url = source_url(std::env::var("BAYLEE_SOURCE_URL").ok().as_deref())
        .unwrap_or_else(|why| panic!("BAYLEE_SOURCE_URL: {why}"));
    let impressum_url = impressum_url(std::env::var("BAYLEE_IMPRESSUM_URL").ok().as_deref())
        .unwrap_or_else(|why| panic!("BAYLEE_IMPRESSUM_URL: {why}"));
    let guest_cap = guest_cap(std::env::var("BAYLEE_GUEST_CAP").ok().as_deref())
        .unwrap_or_else(|why| panic!("BAYLEE_GUEST_CAP: {why}"));
    let registration =
        registration::Registration::from_env(std::env::var("BAYLEE_REGISTRATION").ok().as_deref())
            .unwrap_or_else(|why| panic!("BAYLEE_REGISTRATION: {why}"));
    let auth_mode = authmode::Setting::from_env(std::env::var("BAYLEE_AUTH_MODE").ok().as_deref())
        .unwrap_or_else(|why| panic!("BAYLEE_AUTH_MODE: {why}"));
    // Where reports go and each account's budget for them, refused as early
    // as the other settings: before the database, a typo stops the start.
    let feedback = report::Feedback::from_env()
        .unwrap_or_else(|why| panic!("reports configuration rejected: {why}"));
    if std::env::var_os("BAYLEE_GUESTS").is_some() {
        tracing::warn!(
            "BAYLEE_GUESTS is read no more: the console or BAYLEE_AUTH_MODE decides who comes in \
             (guests, central or mixed; unset is central)"
        );
    }
    let reconnect_secs =
        clock::reconnect_from_env(std::env::var("BAYLEE_RECONNECT_SECS").ok().as_deref())
            .unwrap_or_else(|why| panic!("BAYLEE_RECONNECT_SECS: {why}"));
    let room_grace = wsticket::lifetime_from_env(
        std::env::var("BAYLEE_ROOM_GRACE_SECS").ok().as_deref(),
        departure::ROOM_GRACE_SECS,
    )
    .unwrap_or_else(|why| panic!("BAYLEE_ROOM_GRACE_SECS: {why}"));
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
    let admission =
        admission::Hold::from_env(std::env::var("BAYLEE_ADMISSION_HOLD").ok().as_deref())
            .unwrap_or_else(|why| panic!("BAYLEE_ADMISSION_HOLD: {why}"));
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
    let replay_jobs = replays::jobs_from_env(var("BAYLEE_REPLAY_JOBS").as_deref())
        .unwrap_or_else(|why| panic!("BAYLEE_REPLAY_JOBS: {why}"));
    let replay_timeout = replays::timeout_from_env(var("BAYLEE_REPLAY_JOB_SECS").as_deref())
        .unwrap_or_else(|why| panic!("BAYLEE_REPLAY_JOB_SECS: {why}"));
    let social = social::Social::from_env(
        var("BAYLEE_SOCIAL").as_deref(),
        var("BAYLEE_SOCIAL_KEEP_DAYS").as_deref(),
    )
    .unwrap_or_else(|why| panic!("{why}"));
    let seathost_token = seathost::token_from_env(
        var("BAYLEE_SEATHOST_TOKEN").as_deref(),
        &[
            var("BAYLEE_AGENT_TOKEN").as_deref(),
            var("BAYLEE_FEEDBACK_TOKEN").as_deref(),
            var("BAYLEE_FEEDBACK_KEY").as_deref(),
            var("BAYLEE_ADMIN_TOKEN").as_deref(),
        ],
    )
    .unwrap_or_else(|why| panic!("{why}"));
    let store_path = std::env::var("STORE_PATH")
        .map_or_else(|_| PathBuf::from("gateway-store.json"), PathBuf::from);
    let db = open_database(&store_path).await;
    // A version an admin published wins over the file and the default.
    let terms = match baylee_db::terms::current(&db).await {
        Ok(Some(published)) => Some(
            terms::Terms::from_published(published)
                .unwrap_or_else(|why| panic!("published terms of use: {why}")),
        ),
        Ok(None) => terms,
        Err(e) => panic!("published terms of use: {e}"),
    };
    let catalog = connect_catalog(db.clone()).await;
    // Bound after the database and the catalog, and before anything that has
    // to know the port: with `PORT=0` the kernel chooses it (#279), and the
    // address an engine is told to dial back on below must be that one.
    let (listener, address) = listen(bind, port).await;
    let port = address.port();
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
    let (lobby_changed, _) = tokio::sync::broadcast::channel(16);
    let state = Arc::new(AppState {
        tower: tower::Tower::from_env()
            .unwrap_or_else(|()| panic!("Tower host configuration rejected")),
        central: central::Link::from_env()
            .unwrap_or_else(|()| panic!("central bridge configuration rejected")),
        central_barrier: tokio::sync::RwLock::new(()),
        db,
        hellos: auth::Gauge::new(engine::HELLOS_PER_ADDRESS),
        sockets: auth::Gauge::new(presence::SOCKETS_PER_HOLDER),
        bodies: auth::Gauge::new(LARGE_BODIES_PER_ADDRESS),
        watch_tickets: auth::RateLimiter::new(std::time::Duration::from_mins(1), 30),
        deck_shelves: auth::Gauge::new(2),
        shelf_turns: tokio::sync::Semaphore::new(4),
        deck_states: auth::RateLimiter::new(std::time::Duration::from_hours(1), 600),
        search_limiter: auth::RateLimiter::new(std::time::Duration::from_mins(1), 240),
        search_turns: tokio::sync::Semaphore::new(3),
        game_limiter: auth::RateLimiter::new(std::time::Duration::from_hours(1), 120),
        limiter: auth::RateLimiter::new(std::time::Duration::from_secs(300), 10),
        guest_limiter: auth::RateLimiter::new(
            std::time::Duration::from_hours(24),
            GUESTS_A_DAY_PER_ADDRESS,
        ),
        sign_in_limiter: auth::RateLimiter::new(std::time::Duration::from_secs(300), 8),
        lobby: Mutex::new(Lobby::default()),
        ranked: crate::ranked::Host::default(),
        challenge_starts: tokio::sync::Mutex::new(()),
        challenge_owner: std::sync::atomic::AtomicBool::new(false),
        // Small: a receiver that falls this far behind is one whose socket is
        // not draining, and the lag is handled by sending it the current
        // listing rather than by replaying what it missed.
        lobby_changed: lobby_changed.clone(),
        departed: tokio::sync::broadcast::channel(16).0,
        sessions: sessions::Book::default(),
        invalidate_limiter: auth::RateLimiter::new(std::time::Duration::from_secs(60), 600),
        token_limiter: auth::RateLimiter::new(std::time::Duration::from_secs(60), 60),
        agents: Mutex::new(engine::Agents::default()),
        metrics: metrics::Metrics::default(),
        agent_token,
        engine_url: std::env::var("BAYLEE_ENGINE_URL")
            .unwrap_or_else(|_| launch::engine_url(bind, port)),
        registration,
        auth_mode,
        guest_cap,
        display_name,
        source_url,
        impressum_url,
        trusted_proxies: trusted_proxies(
            &std::env::var("BAYLEE_TRUSTED_PROXIES").unwrap_or_default(),
        ),
        catalog,
        texts: texts::TextCache::default(),
        pool_facts: poolfacts::PoolFactsCache::default(),
        official_lists: banlists::OfficialLists::default(),
        art: Arc::new(art::ArtCache::from_env()),
        deck_images: Arc::new(cosmetics::Store::from_env()),
        feedback,
        record_flushes: record::Flushes::default(),
        keeper: keep::Keeper::default(),
        tickets: wsticket::Tickets::new(ticket_ttl),
        legacy_until,
        chair_tickets: wsticket::Tickets::new(chair_ticket_ttl),
        chair_limiter: auth::RateLimiter::new(chair::LIMIT_WINDOW, chair::LIMIT_TRIES),
        chair_tickets_enabled: switched_on(std::env::var("BAYLEE_CHAIR_TICKETS").ok().as_deref()),
        names: namebook::NameBook::default(),
        presence: presence::Presence::with_notifier(lobby_changed),
        reconnect_secs,
        room_grace,
        started_at: auth::now_secs(),
        terms: terms::Book::new(terms),
        seathosts: Mutex::new(seathost::Registry::default()),
        seathost_token,
        seathost_bridge_url: std::env::var("BAYLEE_SEATHOST_BRIDGE_URL")
            .ok()
            .filter(|url| !url.trim().is_empty())
            .unwrap_or_else(|| format!("http://127.0.0.1:{port}")),
        admission,
        blocks: blocks::Blocks::default(),
        replay_jobs: replays::Jobs::new(replay_jobs, replay_timeout),
        social,
    });
    // Before serving: a deck holding a blocked card must not slip into the
    // first game after a restart.
    state
        .blocks
        .reload(&state.db)
        .await
        .unwrap_or_else(|e| panic!("card blocks: {e}"));
    blocks::spawn_refresh(state.clone());
    // Before serving, so it is done by the time anybody can upload (#301).
    account::sweep_pictures(&state).await;
    // The running games kept before a restart, back in the lobby and waiting
    // for their engines, before the writer that keeps them starts.
    keep::restore(&state).await;
    keep::spawn(state.clone());
    spawn_cleanup(state.clone());
    // A stop stores the running games' records first (`record::flush_on_stop`).
    record::flush_on_stop(state.clone());
    spawn_ticket_sweep(state.clone());
    spawn_session_sweep(state.clone());
    central::spawn_local_check(state.clone());
    metrics::spawn(state.clone());
    presence::spawn(state.clone());
    // A pasted token's link first: whether Central's sign-in reaches this
    // gateway depends on it.
    central::restore(&state).await;
    central::spawn_sync(state.clone());
    state
        .auth_mode
        .restore(&state.db)
        .await
        .unwrap_or_else(|e| panic!("auth mode: {e}"));
    let mode = state.auth_mode.current();
    if mode.central() && !state.connected() {
        tracing::warn!(
            mode = mode.as_str(),
            "this gateway takes Baylee Central's accounts and is not connected to Central \
             (a token pasted into the console, or BAYLEE_CENTRAL_CONFIG_FILE): no Central \
             sign-in reaches it yet, so it lets in {}; allow guests in the console or with \
             BAYLEE_AUTH_MODE, or connect it",
            if mode.guests() {
                "guests only"
            } else {
                "nobody"
            }
        );
    }
    let _central_heartbeat = central::Heartbeat::start(state.clone());
    let _tower_heartbeat = tower::Heartbeat::start(state.clone());
    let _ranked_host = crate::ranked::worker(state.clone());
    let _challenge_delivery = challenges::start_delivery(state.clone());
    central::spawn_terms_publisher(state.clone());
    social::sweep::sweep(&state).await;
    social::presence::spawn(state.clone());
    if let (Some(settings), Some((listener, admin_port))) = (&console, console_listener) {
        admin::serve(listener, state.clone(), settings);
        // Before `BAYLEE_PORT_FILE`, which says the gateway is up: whoever
        // waits for that finds this one written too.
        if let Some(path) = std::env::var_os("BAYLEE_ADMIN_PORT_FILE") {
            write_port_file(std::path::Path::new(&path), admin_port)
                .unwrap_or_else(|e| panic!("BAYLEE_ADMIN_PORT_FILE {}: {e}", path.display()));
        }
    }

    let app = router::build(&state);

    #[cfg(unix)]
    if let Some(path) = std::env::var_os("BAYLEE_UNIX_SOCKET").filter(|p| !p.is_empty()) {
        crate::serve_unix(std::path::Path::new(&path), app.clone());
    }
    state.central.bound(port);
    announce(address);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("gateway serves");
}
