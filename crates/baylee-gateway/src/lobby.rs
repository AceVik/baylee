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
    /// in it, they have something to play, they can reach the table, and they
    /// have said so.
    ///
    /// The third of those is the one that is not obvious. Every way of taking
    /// a chair issues its seat token in the same breath — creating a room,
    /// creating the one-tap game, joining — so a chair with an account and no
    /// token could not exist, and the check would have been noise. A rematch
    /// room is the first place it can: it copies the arrangement of a table
    /// that is over, and a token can only be minted into a reply to the
    /// player it belongs to. Such a chair is *reserved*, not taken, and a
    /// game that started on one would be a game its player could not open a
    /// socket to.
    #[must_use]
    pub fn ready(&self) -> bool {
        match self.kind {
            SeatKind::Human => {
                self.account_id.is_some()
                    && self.deck.is_some()
                    && self.seat_token_hash.is_some()
                    && self.said_ready
            }
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

    /// The same chair at the next table: who was in it, what they brought,
    /// and which side they played for.
    ///
    /// The mirror of [`LobbySeat::vacate`] and the same trap — a field
    /// forgotten here carries something of the finished game into the new
    /// one. Exactly two must not travel. A seat token names one game for the
    /// whole of its life, and `said_ready` is a statement about *this* table
    /// that only the player pressing the button gets to make.
    #[must_use]
    pub fn again(&self) -> Self {
        Self {
            seat_token_hash: None,
            said_ready: false,
            ..self.clone()
        }
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
    /// The clock this table plays at, chosen when the room was opened.
    ///
    /// Kept on the room rather than only in the preset because a room has no
    /// preset until it starts — the listing has to be able to say what pace a
    /// table plays at while a player is still deciding whether to sit down.
    pub house_rules: baylee_core::preset::HouseRules,
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
    /// The room opened to play this table again, once somebody asked for one.
    ///
    /// It lives on the finished game rather than in a map of its own because
    /// "has a rematch been opened yet" is a question about *this* table, and
    /// every caller already has it in hand. It is what makes the route
    /// idempotent: the second player to press the button joins the room the
    /// first one opened instead of opening a second one beside it.
    pub rematch: Option<String>,
    /// The finished game this room is a rematch of, if it is one.
    ///
    /// Two readers, and the second is the reason it is a field on the room
    /// rather than a flag on the route. The reaper answers what the pointer
    /// above cannot — a finished game whose room is still waiting has to
    /// outlive its grace period, or the player who takes a minute longer to
    /// press the button opens a second table instead of joining the first.
    /// And `set_ready` reads it to start such a room the moment its last
    /// chair is ready: everyone here has already asked to play again, so a
    /// room where all of them said so and nobody could press start would be
    /// stuck for no reason anyone at the table could see.
    pub parent: Option<String>,
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
            house_rules: baylee_core::preset::HouseRules::default(),
            engine_token_hash: None,
            agent_id: None,
            engine: None,
            ready: watch::channel(false).0,
            updates: broadcast::channel(256).0,
            created_at,
            finished_at: None,
            password_hash: None,
            next_seq: 0,
            rematch: None,
            parent: None,
        }
    }

    /// A fresh room with a finished table's arrangement already in it.
    ///
    /// Everything that made the table what it was travels: how many chairs,
    /// who sat in them, which were the AI and at what difficulty, the sides
    /// they played for, the name and the password. Nothing of the *game*
    /// does — [`LobbyGame::blank`] supplies the preset, the engine link and
    /// the broadcast, so the room builds its own from the seats when it
    /// starts rather than inheriting one that was already played.
    ///
    /// The host is the old host, and where there was none it is whoever
    /// joined earliest — the rule that already applies when a host leaves.
    /// A two-seat table against the house predates rooms and has no host at
    /// all, and a room nobody may arrange is one nobody can change their
    /// mind at.
    #[must_use]
    pub fn rematch_of(parent: &Self, id: String, created_at: u64) -> Self {
        Self {
            state: LobbyState::Waiting,
            host: parent.host.clone().or_else(|| {
                parent
                    .seats
                    .iter()
                    .filter(|s| s.kind == SeatKind::Human)
                    .min_by_key(|s| s.joined_seq.unwrap_or(u64::MAX))
                    .and_then(|s| s.account_id.clone())
            }),
            name: parent.name.clone(),
            seats: parent.seats.iter().map(LobbySeat::again).collect(),
            // The clock travels with the arrangement. Pressing *play again*
            // at a blitz table is a request for another blitz game, and
            // there is no screen between the button and the new room on
            // which anybody could have said otherwise.
            house_rules: parent.house_rules.clone(),
            password_hash: parent.password_hash.clone(),
            next_seq: parent.next_seq,
            parent: Some(parent.id.clone()),
            ..Self::blank(id, created_at)
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
                    // Whether this room is the next table of one that
                    // finished. A player takes their chair here by pressing
                    // rematch, not ready: the chair is *reserved* until they
                    // do, because that is the only reply their seat token can
                    // be minted into. A client that could not tell the two
                    // rooms apart would send the wrong one, be answered
                    // `200`, and leave the player looking at a chair that
                    // still says it is not ready.
                    "rematch": g.parent.is_some(),
                    // What pace this table plays at, stated once, here.
                    //
                    // The limit is *not* sent to a seat during a game — see
                    // `#69`, which draws its warning from the remaining time
                    // alone so that a thirty-second table does not get a
                    // clock for three seconds. But the same design asks for
                    // the limit to be said once in the room, and this is the
                    // room: a player choosing a table is choosing a pace,
                    // and finding out by losing a decision is not a choice.
                    "clock": {
                        "decide_secs": g.house_rules.decision_timeout_secs,
                        "reconnect_secs": g.house_rules.reconnect_window_secs,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn deck() -> crate::store::Deck {
        crate::store::Deck {
            id: "deck".into(),
            account_id: "someone".into(),
            kind: "account".into(),
            name: "Deck".into(),
            format: "freeform".into(),
            description: None,
            origin: None,
            version: 1,
            cards: vec!["60 Forest".into()],
            sideboard: vec![],
            commanders: vec![],
            sleeve: None,
            playmat: None,
            updated_at: 0,
        }
    }

    /// A chair with everything in it, so a field this module forgets
    /// somewhere is a field this test can see move.
    fn occupied(seat: usize, account: &str, joined: u64) -> LobbySeat {
        LobbySeat {
            seat,
            kind: SeatKind::Human,
            ai: None,
            account_id: Some(account.into()),
            seat_token_hash: Some("hash".into()),
            deck_name: "Deck".into(),
            deck: Some(deck()),
            said_ready: true,
            team: Some(1),
            joined_seq: Some(joined),
        }
    }

    /// Four things have to be true before a chair may start a game, and the
    /// third is the one that is not obvious: a **reserved** chair — one a
    /// rematch room copied over, with an account and a deck but no token —
    /// would be a game its player cannot open a socket to. A token is only
    /// ever minted into a reply to the player it belongs to.
    /// A two-chair room hosted by `host`, filed under `id`.
    fn room_in(
        lobby: &mut Lobby,
        id: &str,
        name: &str,
        host: &str,
        created_at: u64,
        state: LobbyState,
    ) {
        let mut game = LobbyGame::room(
            id.to_string(),
            host.to_string(),
            "Deck".to_string(),
            deck(),
            2,
            name.to_string(),
            created_at,
        );
        game.state = state;
        lobby.games.insert(id.to_string(), game);
    }

    fn ids(rows: &[serde_json::Value]) -> Vec<String> {
        rows.iter()
            .map(|r| r["id"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    fn names_of(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(id, name)| ((*id).to_string(), (*name).to_string()))
            .collect()
    }

    fn lobby() -> (Lobby, HashMap<String, String>) {
        let mut lobby = Lobby::default();
        // Two waiting and two playing, with a tie on `created_at` so the id
        // has something to break.
        room_in(&mut lobby, "b", "Second", "amy", 100, LobbyState::Waiting);
        room_in(&mut lobby, "a", "First", "bob", 100, LobbyState::Waiting);
        room_in(&mut lobby, "c", "Older", "amy", 50, LobbyState::Waiting);
        room_in(&mut lobby, "d", "Running", "bob", 200, LobbyState::Playing);
        room_in(
            &mut lobby,
            "e",
            "Also running",
            "amy",
            10,
            LobbyState::Playing,
        );
        (lobby, names_of(&[("amy", "Amy"), ("bob", "Bob")]))
    }

    /// **The order is total and paging is a partition of it.** Games live in
    /// a `HashMap`, so before there were pages the listing came out in
    /// whatever order the map felt like — invisible while there was only
    /// ever one page, and a listing that hands out rows twice and drops
    /// others as soon as there is a second.
    ///
    /// Waiting rooms first, then the newest, then the id, so two rooms
    /// opened in the same second never swap places between requests. The
    /// partition is the assertion that matters: every page concatenated is
    /// the whole listing, once each.
    #[test]
    fn the_listing_has_one_order_and_every_page_is_a_slice_of_it() {
        let (lobby, names) = lobby();
        let all = |offset, limit| {
            lobby.page_for(
                "amy",
                &names,
                &LobbyQuery {
                    offset,
                    limit: Some(limit),
                    ..LobbyQuery::default()
                },
            )
        };

        let (rows, total) = all(0, 100);
        assert_eq!(total, 5);
        assert_eq!(
            ids(&rows),
            vec!["a", "b", "c", "d", "e"],
            "waiting first, then newest, then the id breaking the tie"
        );
        assert_eq!(
            ids(&all(0, 100).0),
            ids(&rows),
            "and the same order on the next request, whatever the map does"
        );

        let mut walked = Vec::new();
        for page in 0..3 {
            let (rows, count) = all(page * 2, 2);
            assert_eq!(count, total, "the total is of the listing, not the page");
            walked.extend(ids(&rows));
        }
        assert_eq!(walked, ids(&rows), "no row handed out twice and none lost");
        assert!(
            all(5, 2).0.is_empty(),
            "and walking past the end is an empty page rather than a wrap"
        );
    }

    /// A finished table is in no listing at all — not the page, not the
    /// unpaged list, and not the roll of who is sitting down. It is the one
    /// state that is filtered in three places, so it is asked in three.
    #[test]
    fn a_finished_table_is_in_no_listing() {
        let (mut lobby, names) = lobby();
        assert_eq!(lobby.seated_accounts(), vec!["amy", "bob"]);

        for id in ["a", "b", "c", "d"] {
            lobby.games.get_mut(id).expect("filed").state = LobbyState::Over;
        }
        let (rows, total) = lobby.page_for("amy", &names, &LobbyQuery::default());
        assert_eq!((ids(&rows), total), (vec!["e".to_string()], 1));
        assert_eq!(ids(&lobby.list_for("amy", &names)), vec!["e"]);
        assert_eq!(
            lobby.seated_accounts(),
            vec!["amy"],
            "and nobody is still sitting at a table that is over"
        );
    }

    /// The search box reads the table's name **and** its host's, because a
    /// player looking for a table knows one or the other and rarely both —
    /// and the host is matched on the display name a client sees, not on the
    /// account id it never gets.
    #[test]
    fn the_search_box_reads_a_name_and_a_host() {
        let (lobby, names) = lobby();
        let find = |q: &str, waiting_only| {
            let (rows, _) = lobby.page_for(
                "amy",
                &names,
                &LobbyQuery {
                    q: q.to_string(),
                    waiting_only,
                    ..LobbyQuery::default()
                },
            );
            ids(&rows)
        };

        assert_eq!(find("", false), vec!["a", "b", "c", "d", "e"]);
        assert_eq!(
            find("   ", false),
            vec!["a", "b", "c", "d", "e"],
            "blank is no search"
        );
        assert_eq!(find("first", false), vec!["a"], "case does not matter");
        assert_eq!(find("RUNN", false), vec!["d", "e"]);
        assert_eq!(
            find("bob", false),
            vec!["a", "d"],
            "the host's display name, which is what a client is shown"
        );
        assert!(
            find("amy@example.test", false).is_empty(),
            "and not an account id, which no listing carries"
        );
        assert_eq!(
            find("", true),
            vec!["a", "b", "c"],
            "waiting only leaves out the tables already playing"
        );
        assert_eq!(find("runn", true), Vec::<String>::new());
    }

    /// A page size is a cap and not a suggestion: the listing is built by
    /// rendering every row, so an unbounded `limit` makes one request as
    /// expensive as the whole lobby is large. Nought is not a page either.
    #[test]
    fn a_page_size_is_clamped_at_both_ends() {
        let page = |limit| {
            LobbyQuery {
                limit,
                ..LobbyQuery::default()
            }
            .page()
        };
        assert_eq!(page(None), LobbyQuery::DEFAULT_LIMIT);
        assert_eq!(page(Some(0)), 1, "a page of nothing is not a page");
        assert_eq!(page(Some(10)), 10);
        assert_eq!(page(Some(LobbyQuery::MAX_LIMIT + 1)), LobbyQuery::MAX_LIMIT);
        assert_eq!(page(Some(usize::MAX)), LobbyQuery::MAX_LIMIT);
    }

    /// The same query arrives typed from `GET /lobby/games` and as **text**
    /// from the lobby socket, whose token and query come out of one query
    /// string and are flattened — and a flattened struct is deserialized
    /// from a map of strings, so `offset=8` reached a `usize` field as
    /// `"8"` and was refused. Both readings are now one struct, so the two
    /// routes cannot drift.
    ///
    /// An unreadable number is the default rather than a `400`: a listing is
    /// not worth refusing over a typo in a page number.
    #[test]
    fn a_query_reads_the_same_written_as_text_or_as_numbers() {
        let typed: LobbyQuery =
            serde_json::from_str(r#"{"q":"x","offset":8,"limit":5,"waiting_only":true}"#)
                .expect("the typed shape");
        let text: LobbyQuery =
            serde_json::from_str(r#"{"q":"x","offset":"8","limit":"5","waiting_only":"true"}"#)
                .expect("the flattened shape");
        assert_eq!(
            (typed.offset, typed.limit, typed.waiting_only),
            (8, Some(5), true)
        );
        assert_eq!(
            (text.offset, text.limit, text.waiting_only),
            (typed.offset, typed.limit, typed.waiting_only)
        );

        let junk: LobbyQuery =
            serde_json::from_str(r#"{"offset":"eight","limit":"lots","waiting_only":"perhaps"}"#)
                .expect("a typo is not a refusal");
        assert_eq!(
            (junk.offset, junk.limit, junk.waiting_only),
            (0, None, false)
        );
        assert_eq!(junk.page(), LobbyQuery::DEFAULT_LIMIT);

        let empty: LobbyQuery = serde_json::from_str("{}").expect("a caller that asks for nothing");
        assert_eq!(
            (empty.offset, empty.limit, empty.waiting_only, empty.q),
            (0, None, false, String::new())
        );
        for yes in ["true", "1", "yes"] {
            let q: LobbyQuery =
                serde_json::from_str(&format!(r#"{{"waiting_only":"{yes}"}}"#)).expect("a flag");
            assert!(q.waiting_only, "{yes}");
        }
    }

    #[test]
    fn a_reserved_chair_is_not_a_ready_one() {
        let ready = occupied(0, "someone", 0);
        assert!(ready.ready());

        for spoil in [
            (|s: &mut LobbySeat| s.account_id = None) as fn(&mut LobbySeat),
            |s: &mut LobbySeat| s.deck = None,
            |s: &mut LobbySeat| s.seat_token_hash = None,
            |s: &mut LobbySeat| s.said_ready = false,
        ] {
            let mut seat = ready.clone();
            spoil(&mut seat);
            assert!(!seat.ready(), "a chair missing one of the four was ready");
        }

        let mut ai = LobbySeat::open(1);
        ai.kind = SeatKind::Ai;
        assert!(
            ai.ready(),
            "an AI the host gave no deck plays the house deck, so there is \
             nothing left to wait for"
        );
    }

    /// Standing up leaves nothing of the last occupant behind — a seat is
    /// reset in three places and each one that forgot a field left
    /// something. The team is the exception on purpose: it is the table's
    /// shape, which nobody changed by standing up.
    #[test]
    fn vacating_a_chair_keeps_the_chair_and_the_side_and_nothing_else() {
        let mut seat = occupied(3, "someone", 9);
        seat.kind = SeatKind::Ai;
        seat.vacate();

        let mut expected = LobbySeat::open(3);
        expected.team = Some(1);
        assert_eq!(seat.seat, 3);
        assert_eq!(seat.team, Some(1), "the side is the table's, not theirs");
        assert_eq!(seat.account_id, None);
        assert_eq!(seat.seat_token_hash, None);
        assert_eq!(seat.deck_name, String::new());
        assert!(seat.deck.is_none());
        assert!(!seat.said_ready);
        assert_eq!(seat.joined_seq, None);
        assert_eq!(seat.kind, SeatKind::Human, "an emptied chair is a person's");
        assert!(seat.ai.is_none());
        // Field by field above, and then the whole struct: `Debug` prints
        // every field it has, so a field added to `LobbySeat` and forgotten
        // by `vacate` shows up here without this test being touched.
        assert_eq!(
            format!("{seat:?}"),
            format!("{expected:?}"),
            "a field vacate forgot"
        );
    }

    /// The mirror, and the same trap: everything travels to the next table
    /// except the two that must not. A seat token names one game for the
    /// whole of its life, and saying ready is a statement about *this*
    /// table that only the player pressing the button gets to make.
    #[test]
    fn a_chair_at_the_next_table_brings_everything_but_its_token_and_its_yes() {
        let seat = occupied(2, "someone", 4);
        let next = seat.again();

        assert_eq!(next.seat_token_hash, None);
        assert!(!next.said_ready);

        let mut without = next.clone();
        without.seat_token_hash = seat.seat_token_hash.clone();
        without.said_ready = seat.said_ready;
        assert_eq!(
            format!("{without:?}"),
            format!("{seat:?}"),
            "something else was left behind, or invented"
        );
    }

    /// Arrival order and not seat order: the chairs of a room are taken in
    /// whatever order people pick them, so "who has been here longest" is
    /// the only answer that does not depend on where they chose to sit. A
    /// room this leaves with no host has nobody to arrange it.
    #[test]
    fn the_room_is_handed_to_whoever_has_been_here_longest() {
        let mut room = LobbyGame::room(
            "room".into(),
            "host".into(),
            "Deck".into(),
            deck(),
            4,
            "Table".into(),
            0,
        );
        // Seat 3 sat down before seat 1, and an AI chair is nobody.
        room.seats[3] = occupied(3, "early", 1);
        room.seats[1] = occupied(1, "late", 2);
        room.seats[2].kind = SeatKind::Ai;

        assert!(room.hosted_by("host"));
        assert!(!room.hosted_by("early"));

        assert!(room.hand_over_host());
        assert!(
            room.hosted_by("early"),
            "the earliest arrival takes it, whichever chair they are in"
        );

        // The chair the first host was in empties; the next-earliest
        // arrival takes it. (While they are still seated they would take it
        // straight back, which is the same rule.)
        room.seats[0].vacate();
        assert!(room.hand_over_host());
        assert!(room.hosted_by("late"));

        room.seats[1].vacate();
        room.seats[3].vacate();
        assert!(
            !room.hand_over_host(),
            "a room with nobody left to arrange it says so"
        );
    }
}
