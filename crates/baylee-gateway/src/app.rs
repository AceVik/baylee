//! Shared gateway state, independent of startup and route assembly.
use super::{
    Arc, IpAddr, Lobby, Mutex, admission, art, auth, authmode, banlists, blocks, central,
    cosmetics, engine, metrics, namebook, poolfacts, presence, record, registration, replays,
    report, seathost, sessions, social, terms, texts, tower, wsticket,
};

/// Shared gateway state.
pub(super) struct AppState {
    pub(super) tower: Option<Arc<tower::Tower>>,
    pub(super) central: central::Link,
    pub(super) central_barrier: tokio::sync::RwLock<()>,
    /// Accounts, sessions, decks, links, preferences.
    ///
    /// `store.rs` is the only module that knows this is a database; every
    /// route here calls a function there and gets a plain struct back.
    pub(super) db: sea_orm::DatabaseConnection,
    /// Sockets open on `/agent/ws`, `/engine/ws` and `/seathost/ws` that
    /// have said no hello yet, per address (`engine::hello_place`): whatever
    /// they send is read before anything about them is known.
    pub(super) hellos: auth::Gauge,
    /// Open sockets, per account or seat token (`presence::socket_place`).
    pub(super) sockets: auth::Gauge,
    /// Large request bodies on their way, per address (`whole_body`).
    pub(super) bodies: auth::Gauge,
    /// Spectators' tickets, per account: thirty a minute.
    pub(super) watch_tickets: auth::RateLimiter,
    /// Who is reading its whole shelf of decks now (`GET /decks`), per
    /// account, and how many may be at once on the gateway.
    pub(super) deck_shelves: auth::Gauge,
    pub(super) shelf_turns: tokio::sync::Semaphore,
    /// Past states of decks written, per account: each is kept, so six
    /// hundred an hour, far past what a builder session comes to.
    pub(super) deck_states: auth::RateLimiter,
    /// Catalog searches, per account, and how many may be at once on the
    /// gateway: one letter takes a fifth of a second of a pool of eight.
    pub(super) search_limiter: auth::RateLimiter,
    pub(super) search_turns: tokio::sync::Semaphore,
    /// Rooms and games made, per account: 120 an hour.
    pub(super) game_limiter: auth::RateLimiter,
    pub(super) limiter: auth::RateLimiter,
    /// New guests per address in a day, beside [`Self::limiter`]: at this
    /// rate no address reaches [`DEFAULT_GUEST_CAP`] before its first guests
    /// lapse, so none can shut the guest door by itself.
    pub(super) guest_limiter: auth::RateLimiter,
    /// Tries of a locked room's password, per account and room: eight in
    /// five minutes, on a budget of their own beside [`Self::limiter`].
    pub(super) sign_in_limiter: auth::RateLimiter,
    pub(super) lobby: Mutex<Lobby>,
    pub(super) challenge_starts: tokio::sync::Mutex<()>,
    pub(super) challenge_owner: std::sync::atomic::AtomicBool,
    /// Who may come in new (`BAYLEE_REGISTRATION`): anybody (`open`) or
    /// nobody (`off`).
    pub(super) registration: registration::Registration,
    /// Who comes in: guests, Baylee Central's accounts, or both
    /// (`BAYLEE_AUTH_MODE` or the console, `authmode.rs`).
    pub(super) auth_mode: authmode::Setting,
    /// How many guests there may be at once (`BAYLEE_GUEST_CAP`, 1000 unless
    /// set, `0` for no bound); `None` when unbounded. See [`guest_cap`].
    pub(super) guest_cap: Option<u64>,
    /// What this gateway calls itself to a client (`BAYLEE_GATEWAY_NAME`),
    /// already checked by [`display_name`]. `None` when unset, and the client
    /// names it by its address instead.
    pub(super) display_name: Option<String>,
    /// Where this build's source can be had (`BAYLEE_SOURCE_URL`, else the
    /// repository it was built from), already checked by [`source_url`].
    /// The AGPL's §13 offer, which `/source` and `/info` both carry.
    pub(super) source_url: String,
    /// This gateway's legal notice (Impressum), `BAYLEE_IMPRESSUM_URL`,
    /// already checked by [`impressum_url`]; `None` when unset. `/info`
    /// carries it for the client to link.
    pub(super) impressum_url: Option<String>,
    /// Proxies whose `X-Forwarded-For` header may be trusted for rate
    /// limiting (`BAYLEE_TRUSTED_PROXIES`, comma-separated IPs). Empty =
    /// the header is never trusted; anyone can set it, so trusting it
    /// blindly disables the brute-force defense.
    pub(super) trusted_proxies: Vec<IpAddr>,
    /// Every agent connected right now, and the games each was asked to run.
    ///
    /// Nothing here is persisted: an agent that reconnects is a new agent.
    /// Losing its control link closes its old game assignments; the agent
    /// stops and reaps those engines before reconnecting.
    pub(super) agents: Mutex<engine::Agents>,
    /// The server metrics the admin console shows (`metrics.rs`): an hour
    /// of samples in memory, and the public routes' request counter.
    pub(super) metrics: metrics::Metrics,
    /// Fires whenever anything in the lobby changed, so `/lobby/ws` can push
    /// instead of every client asking every two seconds.
    ///
    /// It carries no payload on purpose. What a change *means* differs per
    /// account — whose room it is, which chair is theirs, what they searched
    /// for — so the socket re-renders the listing for its own reader rather
    /// than trying to broadcast one answer to everybody.
    pub(super) lobby_changed: tokio::sync::broadcast::Sender<()>,
    /// The ids of accounts just deleted (#292), for the lobby sockets
    /// signed in as them to close. A lobby socket resolves its session once,
    /// when it opens, so nothing else would tell it.
    pub(super) departed: tokio::sync::broadcast::Sender<String>,
    /// Every session in memory: the state each of its sockets watches
    /// (`sessions.rs`). The check before a frame reaches an engine reads this.
    pub(super) sessions: sessions::Book,
    /// How many revocations one address may push here: the route is open to
    /// anyone, and a signature is worth checking only so often. Per address,
    /// so that a stranger spends its own budget and never Central's.
    pub(super) invalidate_limiter: auth::RateLimiter,
    /// How many gateway tokens one address may have checked: a sign-in and a
    /// refresh verify a signature here, for whoever sends one. Generous, since
    /// a household or a LAN party shares an address.
    pub(super) token_limiter: auth::RateLimiter,
    /// The shared secret an agent proves itself with (`BAYLEE_AGENT_TOKEN`).
    ///
    /// Without one no agent may connect, and therefore no game can start: an
    /// unauthenticated control plane is a way to run processes on somebody
    /// else's machine.
    pub(super) agent_token: Option<String>,
    /// The websocket an engine is told to dial back on (`BAYLEE_ENGINE_URL`).
    ///
    /// Loopback by default, which is right for a single-box deployment and
    /// wrong the moment an agent runs somewhere else.
    pub(super) engine_url: String,
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
    pub(super) catalog: Option<baylee_catalog::Catalog>,
    /// The pool's card text per language, until the catalog's data stamp
    /// moves. See `texts.rs`.
    pub(super) texts: texts::TextCache,
    /// The pool's table (`GET /pool/facts`), until the catalog's data stamp
    /// moves. See `poolfacts.rs`.
    pub(super) pool_facts: poolfacts::PoolFactsCache,
    /// Where the official ban lists stand against the catalog. See
    /// `banlists.rs`.
    pub(super) official_lists: banlists::OfficialLists,
    /// The disk mirror of card art (`BAYLEE_ART_PATH`, `off` to disable).
    ///
    /// An `Arc` of its own because the warming task outlives the request that
    /// started it: a table's pictures keep arriving while the room is still
    /// being arranged. See `art.rs` for why this is the layer that may warm
    /// every deck at a table and a client is not.
    pub(super) art: Arc<art::ArtCache>,
    /// Uploaded deck sleeves and playmats
    /// (`BAYLEE_DECK_IMAGE_PATH`, `off` to disable).
    ///
    /// An `Arc` because every upload and every read is handed to
    /// `spawn_blocking`: decoding a photograph on the runtime's thread
    /// would stall every socket the gateway is holding.
    pub(super) deck_images: Arc<cosmetics::Store>,
    /// Where `POST /reports` sends a report (#307), from
    /// `BAYLEE_FEEDBACK_URL`, `BAYLEE_FEEDBACK_TOKEN`, `BAYLEE_FEEDBACK_KEY`,
    /// and how many each account has sent.
    pub(super) feedback: report::Feedback,
    /// Reports waiting for a game's engine to send its record as it stands
    /// (#323); see `record.rs`.
    pub(super) record_flushes: record::Flushes,
    /// The unspent tickets a socket may be opened with (#294), in memory
    /// only; `BAYLEE_WS_TICKET_SECS` says how long each lives.
    pub(super) tickets: wsticket::Tickets,
    /// Until when (unix seconds) a socket may still be opened with its
    /// token in the query, as clients from before #294 do
    /// ([`wsticket::LEGACY_UNTIL`], or `0` for `BAYLEE_WS_LEGACY_TOKENS=off`).
    pub(super) legacy_until: u64,
    /// The unspent chair tickets hosts handed to their seat bridges
    /// (`chair.rs`), in memory only; `BAYLEE_CHAIR_TICKET_SECS` says how
    /// long each lives.
    pub(super) chair_tickets: wsticket::Tickets,
    /// Chair tickets asked for, per host, and redemptions tried, per
    /// address (`chair.rs`).
    pub(super) chair_limiter: auth::RateLimiter,
    /// Whether hosts may hand chairs to seat bridges at all
    /// (`BAYLEE_CHAIR_TICKETS=off` to disable), read by [`switched_on`].
    pub(super) chair_tickets_enabled: bool,
    /// The handles of the accounts seated in the lobby, read once each
    /// (`namebook.rs`).
    pub(super) names: namebook::NameBook,
    /// The accounts with a lobby socket open, as a count for
    /// `GET /lobby/stats` (`presence.rs`); memory only.
    pub(super) presence: presence::Presence,
    /// How long a lost seat is waited for while other players are at the
    /// table (`BAYLEE_RECONNECT_SECS`, default 180), for a room that names
    /// no window of its own (`clock.rs`).
    pub(super) reconnect_secs: u32,
    /// How long a waiting room keeps the chair of a player whose client went
    /// away (`BAYLEE_ROOM_GRACE_SECS`, default 60; `departure.rs`).
    pub(super) room_grace: std::time::Duration,
    /// When this process started serving (unix seconds), for the admin
    /// console's uptime.
    pub(super) started_at: u64,
    /// The terms of use a player accepts (WG-1): published in the console,
    /// else `BAYLEE_TERMS_PATH`, else the default; and what Baylee Central
    /// confirmed of them (`terms.rs`).
    pub(super) terms: terms::Book,
    /// The seat agents connected now, and their hosted profiles
    /// (`seathost.rs`); memory only.
    pub(super) seathosts: Mutex<seathost::Registry>,
    /// The secret a seat agent proves itself with
    /// (`BAYLEE_SEATHOST_TOKEN`); none, and no hosted model is offered.
    pub(super) seathost_token: Option<String>,
    /// Where a hosted seat's bridge dials (`BAYLEE_SEATHOST_BRIDGE_URL`,
    /// else this gateway's loopback port).
    pub(super) seathost_bridge_url: String,
    /// The file whose existence stops every new game
    /// (`BAYLEE_ADMISSION_HOLD`, `admission.rs`), for a deploy draining
    /// every agent's games.
    pub(super) admission: admission::Hold,
    /// The cards blocked on this gateway (`blocks.rs`).
    pub(super) blocks: blocks::Blocks,
    /// The replay jobs running now (`replays.rs`, `BAYLEE_REPLAY_JOBS`);
    /// memory only.
    pub(super) replay_jobs: replays::Jobs,
    /// Friends, presence and chat (`social/`, `BAYLEE_SOCIAL`,
    /// `BAYLEE_SOCIAL_KEEP_DAYS`); memory beside the database's tables.
    pub(super) social: social::Social,
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

    /// The pool's table as a deck check reads it; `None` without a filled
    /// catalog, or while it cannot be read, and the check then says what it
    /// could not ask.
    pub(crate) async fn pool_table(&self) -> Option<Arc<baylee_catalog::PoolFacts>> {
        let catalog = self.catalog.as_ref()?;
        match self.pool_facts.get(catalog).await {
            Ok(held) => held.table.clone(),
            Err(e) => {
                tracing::warn!(%e, "reading the pool's table failed; a deck check goes without");
                None
            }
        }
    }

    /// Whether this gateway is connected to Baylee Central
    /// (`BAYLEE_CENTRAL_CONFIG_FILE`): how a Central sign-in reaches it.
    pub(crate) fn connected(&self) -> bool {
        self.central.is_some()
    }
}
