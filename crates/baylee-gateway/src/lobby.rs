//! The lobby: open games and seat bindings.
//!
//! A lobby game becomes a *running* game once its seats are filled: the
//! gateway asks an agent to start an engine for it, that engine dials back,
//! and from then on the gateway only routes. Seat tokens (256-bit, stored
//! hashed) bind a websocket to exactly one seat of one account; the engine
//! token does the same for the one process allowed to play the game.

use baylee_core::preset::GamePreset;
use baylee_protocol::v1::Envelope;
use std::collections::HashMap;
use tokio::sync::{broadcast, mpsc, watch};

/// Lobby state of a game.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum LobbyState {
    /// Waiting for a second seat.
    Waiting,
    /// Both seats filled; the game is running.
    Playing,
    /// Finished.
    Over,
}

/// What is meant to sit in a seat.
///
/// Separate from whether anyone *has*: a human seat with no account is a
/// chair waiting for someone, and an AI seat is filled the moment it is
/// configured. Collapsing the two would make "is this table full?" ask the
/// wrong question.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SeatKind {
    /// A person, once one takes it.
    Human,
    /// The house AI, at a named difficulty.
    Ai,
}

/// A seat in a lobby game.
#[derive(Clone, Debug)]
pub struct LobbySeat {
    /// Seat index.
    pub seat: usize,
    /// Whether this chair is for a person or for the AI.
    pub kind: SeatKind,
    /// The AI's difficulty profile, by the key `AIProfile::NAMED` lists.
    /// `None` for a human seat.
    pub ai: Option<String>,
    /// Account id when a human took the seat (`None` = AI or open).
    pub account_id: Option<String>,
    /// SHA-256 of the seat token (empty until issued).
    pub seat_token_hash: Option<String>,
    /// The deck the seat plays.
    pub deck_name: String,
    /// The seat's full deck. Present once the seat has one — a human's own
    /// choice, or the deck the host gave an AI — and what the preset is built
    /// from when the game starts.
    pub deck: Option<crate::store::Deck>,
    /// Whether the person in this chair has said they are ready.
    ///
    /// Separate from having a deck, because they answer different questions:
    /// a chair with a deck in it is *able* to play, and this is the player
    /// saying they want to. Before it existed a room started the instant the
    /// last deck was chosen, which meant a player who picked a deck to look
    /// at it was already in a game.
    pub said_ready: bool,
    /// Which team this chair plays for, or `None` for a chair that plays for
    /// itself.
    ///
    /// A property of the *chair*, not of whoever is sitting in it: it is the
    /// host's arrangement of the table, and it survives a player leaving so
    /// that a 2v2 stays a 2v2 while it waits for someone to take the empty
    /// seat back.
    pub team: Option<u8>,
    /// Where in the arrival order this player sits, for handing the room on.
    ///
    /// Not the seat index: chairs may be taken in any order, and "the next
    /// player who joined" is a question about time. `None` for an empty or
    /// AI chair.
    pub joined_seq: Option<u64>,
}

impl LobbySeat {
    /// An empty chair for a person.
    #[must_use]
    pub fn open(seat: usize) -> Self {
        Self {
            seat,
            kind: SeatKind::Human,
            ai: None,
            account_id: None,
            seat_token_hash: None,
            deck_name: String::new(),
            deck: None,
            said_ready: false,
            team: None,
            joined_seq: None,
        }
    }

    /// Whether the seat is settled enough for the game to start: somebody is
    /// in it, they have something to play, and they have said so.
    #[must_use]
    pub fn ready(&self) -> bool {
        match self.kind {
            SeatKind::Human => self.account_id.is_some() && self.deck.is_some() && self.said_ready,
            // An AI the host gave no deck plays the house deck, so there is
            // nothing left to wait for.
            SeatKind::Ai => true,
        }
    }

    /// Empties the chair, keeping only which chair it is.
    ///
    /// A seat is reset in three places — a player leaving, the host turning a
    /// chair over to the AI, and a chair turned back to a human — and each
    /// one that forgot a field left something of the last occupant behind.
    pub fn vacate(&mut self) {
        let team = self.team;
        *self = Self::open(self.seat);
        // The team is the table's shape, which nobody changed by standing up.
        self.team = team;
    }
}

/// The gateway's end of one engine process.
///
/// Everything the gateway has to say to a game goes down this channel, so a
/// game with no link is a game nobody can play — which is exactly what a seat
/// socket waits for before it announces itself.
pub type EngineLink = mpsc::UnboundedSender<Envelope>;

/// A lobby game.
pub struct LobbyGame {
    /// Game id (`UUIDv7`).
    pub id: String,
    /// State.
    pub state: LobbyState,
    /// The account that opened the room and configures it. `None` for the
    /// two-seat tables that predate rooms.
    pub host: Option<String>,
    /// What the host called the table. Empty is fine; the list falls back to
    /// the host's name.
    pub name: String,
    /// Seats, in turn order. Two or more.
    pub seats: Vec<LobbySeat>,
    /// The preset the engine is asked to build the game from (present once
    /// both seats are decided).
    pub preset: Option<GamePreset>,
    /// SHA-256 of the token the engine proves itself with. One game's worth
    /// of authority: it is issued when the engine is ordered and is useless
    /// for anything but attaching to this game.
    pub engine_token_hash: Option<String>,
    /// The agent that was asked to run this game, so the gateway knows who to
    /// tell when it is over.
    pub agent_id: Option<String>,
    /// The engine process, once it has dialled in.
    pub engine: Option<EngineLink>,
    /// Flips to true when an engine is attached. A seat socket may open the
    /// moment the lobby says "playing", which is before the engine exists;
    /// this is what it waits on rather than polling.
    pub ready: watch::Sender<bool>,
    /// Per-game update fan-out: every `(seat, encoded envelope)` the engine
    /// produces is broadcast here, so every connected seat socket receives
    /// its own messages — not just the seat that happened to act (human-vs-
    /// human depends on this; filtering per-socket used to drop the
    /// opponent's envelopes entirely).
    ///
    /// The payload is the encoded player-facing envelope, not a decoded one:
    /// the gateway forwards the bytes the engine handed it and never has to
    /// understand them.
    pub updates: broadcast::Sender<(u8, Vec<u8>)>,
    /// When the game was created (unix seconds).
    pub created_at: u64,
    /// When the game ended (unix seconds), for the cleanup grace period.
    pub finished_at: Option<u64>,
    /// SHA-256 of the room's password, if the host set one.
    ///
    /// Hashed rather than kept, because it is a password and people reuse
    /// them — but SHA-256 rather than Argon2 like an account's, because this
    /// one guards a table for an evening and is checked on every join. The
    /// listing says only whether a room *has* one.
    pub password_hash: Option<String>,
    /// Hands out [`LobbySeat::joined_seq`]. Monotonic for the room's life, so
    /// a player who leaves and comes back is at the back of the queue.
    pub next_seq: u64,
}

impl LobbyGame {
    /// A room: `chairs` seats, the host in the first one, the rest open.
    ///
    /// The host owns the table until it starts — who else may sit, which
    /// chairs the AI takes and at what difficulty. Everyone else configures
    /// exactly one thing, which is the deck they themselves will play.
    #[must_use]
    pub fn room(
        id: String,
        account_id: String,
        deck_name: String,
        deck: crate::store::Deck,
        chairs: usize,
        name: String,
        created_at: u64,
    ) -> Self {
        let mut seats: Vec<LobbySeat> = (0..chairs).map(LobbySeat::open).collect();
        if let Some(first) = seats.first_mut() {
            first.account_id = Some(account_id.clone());
            first.deck_name = deck_name;
            first.deck = Some(deck);
            first.joined_seq = Some(0);
        }
        Self {
            state: LobbyState::Waiting,
            seats,
            host: Some(account_id),
            name,
            next_seq: 1,
            ..Self::blank(id, created_at)
        }
    }

    /// The next arrival number, for a player sitting down.
    pub fn claim_seq(&mut self) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        seq
    }

    /// Whether `account_id` may arrange this room.
    #[must_use]
    pub fn hosted_by(&self, account_id: &str) -> bool {
        self.host.as_deref() == Some(account_id)
    }

    /// Hands the room to the player who joined next, and says whether it
    /// found one.
    ///
    /// The rule is arrival order, not seat order: the chairs of a room are
    /// taken in whatever order people pick them, and "who has been here
    /// longest" is the only answer that does not depend on where they chose
    /// to sit. A room this leaves with no host at all has nobody to arrange
    /// it and is the caller's to close.
    pub fn hand_over_host(&mut self) -> bool {
        let host = self.host.clone();
        let next = self
            .seats
            .iter()
            .filter(|s| s.kind == SeatKind::Human && s.account_id.is_some())
            .filter(|s| s.account_id != host)
            .min_by_key(|s| s.joined_seq.unwrap_or(u64::MAX));
        match next.and_then(|s| s.account_id.clone()) {
            Some(account_id) => {
                self.host = Some(account_id);
                true
            }
            None => false,
        }
    }

    /// A game whose seats are decided and whose engine has been ordered.
    #[must_use]
    pub fn playing(id: String, seats: Vec<LobbySeat>, preset: GamePreset, created_at: u64) -> Self {
        Self {
            state: LobbyState::Playing,
            seats,
            preset: Some(preset),
            ..Self::blank(id, created_at)
        }
    }

    /// The fields every game starts with, whatever else is true of it.
    fn blank(id: String, created_at: u64) -> Self {
        Self {
            id,
            state: LobbyState::Waiting,
            host: None,
            name: String::new(),
            seats: Vec::new(),
            preset: None,
            engine_token_hash: None,
            agent_id: None,
            engine: None,
            ready: watch::channel(false).0,
            updates: broadcast::channel(256).0,
            created_at,
            finished_at: None,
            password_hash: None,
            next_seq: 0,
        }
    }

    /// Marks the game finished. Idempotent: the engine says a game is over
    /// and its socket then closes, and both paths land here.
    pub fn finish(&mut self, now: u64) {
        if self.state != LobbyState::Over {
            self.state = LobbyState::Over;
            self.finished_at = Some(now);
        }
        self.engine = None;
        // `send_replace` for the same reason as the attach path: a game that
        // ends while nobody is watching must still read "not ready".
        self.ready.send_replace(false);
    }
}

/// What a caller wants out of the listing.
///
/// Every field has a sane absence, so a client that asks for nothing gets the
/// first page of everything — which is what `GET /lobby/games` did before any
/// of this existed.
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct LobbyQuery {
    /// Free text matched against the table's name and its host's name.
    #[serde(default)]
    pub q: String,
    /// How many rows to skip.
    #[serde(default, deserialize_with = "loose_usize")]
    pub offset: usize,
    /// How many rows to return. Clamped to [`LobbyQuery::MAX_LIMIT`].
    #[serde(default, deserialize_with = "loose_opt_usize")]
    pub limit: Option<usize>,
    /// Whether to leave out rooms that are already playing.
    #[serde(default, deserialize_with = "loose_bool")]
    pub waiting_only: bool,
}

/// A query-string value that may arrive typed or as the text it was written
/// as.
///
/// The lobby socket takes its token *and* this query out of one query string,
/// which serde flattens — and a flattened struct is deserialized from a map of
/// **strings**, so `offset=8` reaches a `usize` field as `"8"` and is refused.
/// The HTTP route, which parses the same struct without a flatten, never saw
/// it. Rather than keep two shapes of the one query in step, both read either.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum Loose<T> {
    /// What `GET /lobby/games` sends it as.
    Typed(T),
    /// What the flattened socket query sends it as.
    Text(String),
}

/// A `usize` written either way.
fn loose_usize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<usize, D::Error> {
    Ok(loose_opt_usize(d)?.unwrap_or_default())
}

/// An optional `usize` written either way. An unreadable number is `None`
/// rather than a `400`: a listing is not worth refusing over a typo in a page
/// number, and the default page is a perfectly good answer.
fn loose_opt_usize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<usize>, D::Error> {
    use serde::Deserialize as _;
    Ok(match Option::<Loose<usize>>::deserialize(d)? {
        Some(Loose::Typed(n)) => Some(n),
        Some(Loose::Text(text)) => text.trim().parse().ok(),
        None => None,
    })
}

/// A flag written either way. Absent, empty and unreadable all mean `false`.
fn loose_bool<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    use serde::Deserialize as _;
    Ok(match Option::<Loose<bool>>::deserialize(d)? {
        Some(Loose::Typed(flag)) => flag,
        Some(Loose::Text(text)) => matches!(text.trim(), "true" | "1" | "yes"),
        None => false,
    })
}

impl LobbyQuery {
    /// How many rows one page may hold, whatever a caller asks for.
    ///
    /// A cap rather than a suggestion: the listing is built by rendering every
    /// row, and an unbounded `limit` makes one request as expensive as the
    /// whole lobby is large.
    pub const MAX_LIMIT: usize = 100;
    /// How many rows a caller that did not say gets.
    pub const DEFAULT_LIMIT: usize = 25;

    /// The page size this query actually gets.
    #[must_use]
    pub fn page(&self) -> usize {
        self.limit
            .unwrap_or(Self::DEFAULT_LIMIT)
            .clamp(1, Self::MAX_LIMIT)
    }

    /// Whether a game matches the text being searched for.
    ///
    /// Name *and* host, because a player looking for a table knows one or the
    /// other and rarely both. Case-insensitive on the plain lowercase mapping
    /// — this is a search box, not a collation.
    fn matches(&self, game: &LobbyGame, host_name: Option<&str>) -> bool {
        if self.waiting_only && game.state != LobbyState::Waiting {
            return false;
        }
        if self.q.trim().is_empty() {
            return true;
        }
        let needle = self.q.trim().to_lowercase();
        game.name.to_lowercase().contains(&needle)
            || host_name.is_some_and(|h| h.to_lowercase().contains(&needle))
    }
}

/// The lobby registry.
#[derive(Default)]
pub struct Lobby {
    /// Games by id.
    pub games: HashMap<String, LobbyGame>,
}

impl Lobby {
    /// Every account id sitting at a visible table.
    ///
    /// The caller resolves these to display names against the store, which
    /// this module has no business locking.
    #[must_use]
    pub fn seated_accounts(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .games
            .values()
            .filter(|g| g.state != LobbyState::Over)
            .flat_map(|g| g.seats.iter().filter_map(|s| s.account_id.clone()))
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Games visible in the lobby (waiting or playing), searched and paged.
    ///
    /// `me` is the account asking, so a seat can say whether it is theirs
    /// without the answer having to carry anyone's account id. `names` maps
    /// the ids from [`Lobby::seated_accounts`] to display names: a room is
    /// arranged in the open, and "who is that" is answered with a name.
    ///
    /// Returns the page and how many rows matched in total, so a client can
    /// say "25 of 140" without asking twice.
    ///
    /// **The order is fixed and it has to be.** Games live in a `HashMap`, so
    /// before there were pages the listing came out in whatever order the map
    /// felt like — which nobody could see, because there was only ever one
    /// page. Paging that would hand out rows twice and drop others. Rooms
    /// still waiting come first (they are the ones a player can do something
    /// about), then the newest, and the id breaks a tie so two rooms opened
    /// in the same second never swap places between requests.
    #[must_use]
    pub fn page_for(
        &self,
        me: &str,
        names: &HashMap<String, String>,
        query: &LobbyQuery,
    ) -> (Vec<serde_json::Value>, usize) {
        let mut matched: Vec<&LobbyGame> = self
            .games
            .values()
            .filter(|g| g.state != LobbyState::Over)
            .filter(|g| {
                query.matches(
                    g,
                    g.host
                        .as_ref()
                        .and_then(|h| names.get(h))
                        .map(String::as_str),
                )
            })
            .collect();
        matched.sort_by(|a, b| {
            let waiting = |g: &LobbyGame| u8::from(g.state != LobbyState::Waiting);
            waiting(a)
                .cmp(&waiting(b))
                .then(b.created_at.cmp(&a.created_at))
                .then(a.id.cmp(&b.id))
        });
        let total = matched.len();
        let rows = matched
            .into_iter()
            .skip(query.offset)
            .take(query.page())
            .map(|g| self.row(g, me, names))
            .collect();
        (rows, total)
    }

    /// Every visible game, for a caller that wants no paging at all.
    #[must_use]
    pub fn list_for(&self, me: &str, names: &HashMap<String, String>) -> Vec<serde_json::Value> {
        self.games
            .values()
            .filter(|g| g.state != LobbyState::Over)
            .map(|g| self.row(g, me, names))
            .collect()
    }

    /// One row of the listing.
    #[expect(
        clippy::unused_self,
        reason = "a method so the row shape stays beside the two callers that \
                  render it, rather than a free function reachable from \
                  anywhere in the crate"
    )]
    fn row(&self, g: &LobbyGame, me: &str, names: &HashMap<String, String>) -> serde_json::Value {
        serde_json::json!({
                    "id": g.id,
                    "name": g.name,
                    "host": g.host.as_ref().and_then(|h| names.get(h)),
                    "yours": g.host.as_deref() == Some(me),
                    // Whether, never what: a client needs to know to ask for
                    // a password, and nothing else about it belongs on a
                    // listing every signed-in player can read.
                    "locked": g.password_hash.is_some(),
                    // Whether the room could start if the host said so. It is
                    // the host's button, but every player can see why it is
                    // not lit yet.
                    "startable": g.state == LobbyState::Waiting
                        && g.seats.iter().all(LobbySeat::ready),
                    "state": match g.state {
                        LobbyState::Waiting => "waiting",
                        LobbyState::Playing => "playing",
                        LobbyState::Over => "over",
                    },
                    // Everything a player needs to decide whether to sit
                    // down: how many chairs, which are people, which are the
                    // AI and how hard, and what everyone brought. A room is
                    // configured in the open — that is what makes it a room
                    // rather than a matchmaking queue.
                    "seats": g.seats.iter().map(|s| {
                        serde_json::json!({
                            "seat": s.seat,
                            "kind": s.kind,
                            "ai": s.ai,
                            "taken": s.account_id.is_some(),
                            // A name, never an account id: the listing is
                            // public to every signed-in player, and knowing
                            // who is at a table does not require knowing
                            // their account.
                            "player": s.account_id.as_ref().and_then(|a| names.get(a)),
                            "you": s.account_id.as_deref() == Some(me),
                            "host": s.account_id.is_some() && s.account_id == g.host,
                            "deck": s.deck_name,
                            "ready": s.ready(),
                            // `null` for a chair that plays for itself, which
                            // is every chair at a table with no teams on it.
                            "team": s.team,
                        })
                    }).collect::<Vec<_>>(),
        })
    }
}
