//! Tickets for opening a websocket (#294).
//!
//! A browser's `WebSocket` cannot set a header, so every socket a player
//! opens used to carry its secret in the query string: the account's session
//! on `/lobby/ws?token=`, the seat token on `/games/{id}/ws?token=`. A URL is
//! what a reverse proxy writes to its access log, what a browser keeps in its
//! history and what a crash report quotes, and those two tokens live for
//! hours.
//!
//! So a client trades its bearer token for a ticket first, over an ordinary
//! request that *can* carry a header (`POST /ws-ticket`), and dials with
//! `?ticket=` instead. A ticket:
//!
//! - is 256 random bits, kept here only as its SHA-256 (as seat tokens are);
//! - opens **one** socket: it is removed the moment an upgrade presents it,
//!   whether or not the rest of the upgrade passes, so a second use fails;
//! - opens only the socket it was issued for: a lobby ticket is bound to the
//!   session that asked for it, a seat ticket to one seat of one game and to
//!   the seat token that proved it, so neither opens the other's door;
//! - dies [`DEFAULT_SECS`] after it was issued if nobody used it
//!   (`BAYLEE_WS_TICKET_SECS`, [`MIN_SECS`]..=[`MAX_SECS`]);
//! - lives in this process's memory and nowhere else. Each holder may have
//!   [`PER_HOLDER`] outstanding, the oldest giving way to a new one, and the
//!   whole gateway [`MAX_OUTSTANDING`], so asking for tickets cannot be used
//!   to grow the gateway's memory.
//!
//! A ticket that reaches a log is useless by the time anybody reads it: it
//! was either spent on the upgrade it was made for or expired unspent.
//!
//! Every function here takes the time it is asked at, so the tests can stand
//! at the exact instant a ticket dies. The routes pass `Instant::now()`.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use parking_lot::Mutex;

/// How long an unused ticket lives when the operator says nothing (owner,
/// 27.09.2026). Long enough for a phone to finish a TLS handshake on a bad
/// network; a client that misses it fetches another, which is one round trip.
pub const DEFAULT_SECS: u64 = 45;

/// The shortest lifetime `BAYLEE_WS_TICKET_SECS` may set. Zero would be a
/// gateway nobody can open a socket on.
pub const MIN_SECS: u64 = 1;

/// The longest lifetime `BAYLEE_WS_TICKET_SECS` may set: ten minutes. Past
/// that a ticket is a second session token with a URL for a home, which is
/// what #294 exists to end.
pub const MAX_SECS: u64 = 600;

/// How many unspent tickets one holder (a session, or a seat token) may have
/// at once. A client asks for one per dial and retries twice at most, so
/// eight is never reached by a client that behaves; one that does not only
/// replaces its own oldest.
pub const PER_HOLDER: usize = 8;

/// How many unspent tickets the whole gateway holds before it refuses to
/// issue more (`503`). About six megabytes at the worst, and far above what
/// the guest cap and the sign-in limits let anyone reach.
pub const MAX_OUTSTANDING: usize = 65_536;

/// The end of the window in which a socket may still be opened the old way,
/// with its token in the query string: **2026-11-01T00:00:00Z**, so the last
/// day it works is 31.10.2026 (UTC). Clients from before #294 (beta.1) send
/// nothing else, and this is how long they have to be updated.
///
/// Enforced at run time by [`legacy_open`], not by a test that fails on the
/// date: the gateway refuses the old query on its own from this second on,
/// and the code path is then dead and can be removed.
pub const LEGACY_UNTIL: u64 = 1_793_491_200;

/// The same date, as `docs/protocol.md` and the log write it.
pub const LEGACY_UNTIL_DATE: &str = "2026-10-31";

/// Which socket an upgrade is opening, as its route says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Door {
    /// `/lobby/ws`.
    Lobby,
    /// `/games/{game_id}/ws`.
    Seat {
        /// The game in the path.
        game_id: String,
    },
    /// `/games/{game_id}/watch`: a spectator's socket.
    Watch {
        /// The game in the path.
        game_id: String,
    },
    /// `POST /lobby/games/{game_id}/chairs/{seat}/redeem`: a chair a host
    /// handed to a seat bridge (`chair.rs`). Not a socket, but the same kind
    /// of secret: single use, short-lived, bound.
    Chair {
        /// The game in the path.
        game_id: String,
        /// The chair in the path.
        seat: usize,
    },
}

/// What a ticket was issued for, and by whom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Grant {
    /// The lobby feed of one account, asked for with one session.
    Lobby {
        /// The account the session belongs to.
        account_id: String,
        /// SHA-256 of the session token, so the upgrade can ask whether that
        /// session still stands (signed out, expired, account deleted).
        session: Vec<u8>,
    },
    /// One seat of one game, asked for with that seat's token.
    Seat {
        /// The game.
        game_id: String,
        /// The seat the token opened.
        seat: usize,
        /// The seat token's hash, so a ticket from before the token was
        /// handed out again (`POST /lobby/games/{id}/seat`) opens nothing.
        seat_token_hash: String,
    },
    /// A spectator's view of one game, asked for with a session.
    Watch {
        /// The game.
        game_id: String,
        /// SHA-256 of the session token that asked.
        session: Vec<u8>,
    },
    /// One open chair of one waiting room, handed by its host to a seat
    /// bridge (`chair.rs`): whoever redeems it sits there as the host's
    /// delegate, with no account of its own.
    Chair {
        /// The room.
        game_id: String,
        /// The chair.
        seat: usize,
        /// The host's account, who answers for the chair. The redemption
        /// checks that it still hosts the room.
        host: String,
        /// The hosted-model order it was minted for (`seathost.rs`), which
        /// the gateway minted rather than the host's client: redeemed even
        /// where hosts may hand no chair over (`BAYLEE_CHAIR_TICKETS=off`),
        /// and only while the chair still holds that order.
        order: Option<String>,
    },
}

impl Grant {
    /// Whether this grant opens `door`.
    fn opens(&self, door: &Door) -> bool {
        match (self, door) {
            (Self::Lobby { .. }, Door::Lobby) => true,
            (Self::Seat { game_id, .. }, Door::Seat { game_id: asked })
            | (Self::Watch { game_id, .. }, Door::Watch { game_id: asked }) => game_id == asked,
            (
                Self::Chair { game_id, seat, .. },
                Door::Chair {
                    game_id: asked,
                    seat: at,
                },
            ) => game_id == asked && seat == at,
            _ => false,
        }
    }

    /// Who is holding the ticket, for [`PER_HOLDER`]: the credential that
    /// asked for it.
    fn holder(&self) -> String {
        match self {
            Self::Lobby { session, .. } => format!("session:{}", hex(session)),
            Self::Seat {
                seat_token_hash, ..
            } => format!("seat:{seat_token_hash}"),
            Self::Chair { host, .. } => format!("chair:{host}"),
            Self::Watch { session, .. } => format!("watch:{}", hex(session)),
        }
    }
}

/// Why an upgrade's ticket opened nothing.
///
/// All three are answered the same way on the wire (`401 ticket expired or
/// used`): a stranger learns nothing from which it was. They are told apart
/// here for the tests and the debug log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Never issued, already used, swept, or given way to a newer one.
    Unknown,
    /// Issued, unused, and older than the lifetime.
    Expired,
    /// Issued for another socket, game or seat. Spent all the same.
    WrongDoor,
}

/// The gateway is holding [`MAX_OUTSTANDING`] tickets already.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Full;

/// One unspent ticket.
struct Entry {
    grant: Grant,
    holder: String,
    issued: Instant,
}

#[derive(Default)]
struct Inner {
    /// By the ticket's hash (hex).
    live: HashMap<String, Entry>,
    /// Each holder's unspent tickets, oldest first.
    by_holder: HashMap<String, VecDeque<String>>,
}

impl Inner {
    fn remove(&mut self, key: &str) -> Option<Entry> {
        let entry = self.live.remove(key)?;
        if let Some(queue) = self.by_holder.get_mut(&entry.holder) {
            queue.retain(|k| k != key);
            if queue.is_empty() {
                self.by_holder.remove(&entry.holder);
            }
        }
        Some(entry)
    }
}

/// The unspent tickets of this gateway.
pub struct Tickets {
    ttl: Duration,
    inner: Mutex<Inner>,
}

impl Tickets {
    /// An empty store whose tickets live `ttl`.
    #[must_use]
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            inner: Mutex::new(Inner::default()),
        }
    }

    /// How long an unused ticket lives.
    #[must_use]
    pub const fn ttl(&self) -> Duration {
        self.ttl
    }

    /// How many tickets are unspent right now, expired ones included until
    /// the next sweep.
    #[cfg(test)]
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.lock().live.len()
    }

    /// Issues a ticket for `grant` and returns it. The ticket itself is
    /// handed out once and kept nowhere; only its hash is stored.
    ///
    /// # Errors
    /// [`Full`] when the gateway holds [`MAX_OUTSTANDING`] live tickets
    /// even after dropping the expired ones.
    pub fn issue(&self, grant: Grant, now: Instant) -> Result<String, Full> {
        let ticket = crate::auth::new_token();
        let key = crate::auth::token_hash(&ticket);
        let holder = grant.holder();
        let mut inner = self.inner.lock();
        if inner.live.len() >= MAX_OUTSTANDING {
            sweep_inner(&mut inner, self.ttl, now);
            if inner.live.len() >= MAX_OUTSTANDING {
                return Err(Full);
            }
        }
        // The holder's oldest gives way. Not a refusal: a client that lost
        // count of its tickets still gets the one it asked for, and can only
        // ever crowd out its own.
        while inner
            .by_holder
            .get(&holder)
            .is_some_and(|queue| queue.len() >= PER_HOLDER)
        {
            let Some(oldest) = inner
                .by_holder
                .get_mut(&holder)
                .and_then(VecDeque::pop_front)
            else {
                break;
            };
            inner.live.remove(&oldest);
        }
        inner
            .by_holder
            .entry(holder.clone())
            .or_default()
            .push_back(key.clone());
        inner.live.insert(
            key,
            Entry {
                grant,
                holder,
                issued: now,
            },
        );
        Ok(ticket)
    }

    /// Spends `ticket` on an upgrade through `door`.
    ///
    /// The ticket is removed before anything about it is checked, so it is
    /// spent even when it is refused: a presented ticket never opens a
    /// second socket, whatever happened to the first attempt.
    ///
    /// A ticket is alive for exactly [`Tickets::ttl`]: at `issued + ttl` it
    /// is expired.
    ///
    /// # Errors
    /// The [`Refusal`] that says why it opened nothing.
    pub fn consume(&self, ticket: &str, door: &Door, now: Instant) -> Result<Grant, Refusal> {
        let key = crate::auth::token_hash(ticket);
        let entry = self.inner.lock().remove(&key).ok_or(Refusal::Unknown)?;
        if now.saturating_duration_since(entry.issued) >= self.ttl {
            return Err(Refusal::Expired);
        }
        if !entry.grant.opens(door) {
            return Err(Refusal::WrongDoor);
        }
        Ok(entry.grant)
    }

    /// Drops every unspent ticket whose grant `dead` says is no longer
    /// meant (a host who left the room it handed chairs of), and says how
    /// many that was. The redemption checks the same thing again; this only
    /// keeps a ticket nobody may use from being kept.
    pub fn revoke(&self, dead: impl Fn(&Grant) -> bool) -> usize {
        let mut inner = self.inner.lock();
        let gone: Vec<String> = inner
            .live
            .iter()
            .filter(|(_, e)| dead(&e.grant))
            .map(|(k, _)| k.clone())
            .collect();
        for key in &gone {
            inner.remove(key);
        }
        gone.len()
    }

    /// Drops every expired ticket and says how many that was.
    pub fn sweep(&self, now: Instant) -> usize {
        sweep_inner(&mut self.inner.lock(), self.ttl, now)
    }
}

fn sweep_inner(inner: &mut Inner, ttl: Duration, now: Instant) -> usize {
    let dead: Vec<String> = inner
        .live
        .iter()
        .filter(|(_, e)| now.saturating_duration_since(e.issued) >= ttl)
        .map(|(k, _)| k.clone())
        .collect();
    for key in &dead {
        inner.remove(key);
    }
    dead.len()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, b| {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// `BAYLEE_WS_TICKET_SECS`: how long an unused ticket lives.
///
/// Unset or empty is [`DEFAULT_SECS`]. Anything that is not a whole number
/// of seconds in [`MIN_SECS`]..=[`MAX_SECS`] refuses startup: a lifetime
/// that was misread is either a gateway nobody can dial or a ticket that is
/// a session token again, and neither should be found out in production.
///
/// # Errors
/// The sentence the gateway refuses to start with.
pub fn ttl_from_env(raw: Option<&str>) -> Result<Duration, String> {
    lifetime_from_env(raw, DEFAULT_SECS)
}

/// [`ttl_from_env`] with another default, for a store of tickets that are
/// not a socket's (`BAYLEE_CHAIR_TICKET_SECS`, `chair.rs`): the same bounds,
/// for the same reasons.
///
/// # Errors
/// The sentence the gateway refuses to start with.
pub fn lifetime_from_env(raw: Option<&str>, default_secs: u64) -> Result<Duration, String> {
    match raw.map(str::trim) {
        None | Some("") => Ok(Duration::from_secs(default_secs)),
        Some(text) => match text.parse::<u64>() {
            Ok(secs) if (MIN_SECS..=MAX_SECS).contains(&secs) => Ok(Duration::from_secs(secs)),
            Ok(secs) => Err(format!("{secs} seconds is outside {MIN_SECS}..={MAX_SECS}")),
            Err(_) => Err(format!("{text:?} is not a whole number of seconds")),
        },
    }
}

/// `BAYLEE_WS_LEGACY_TOKENS`: when the old `?token=` stops opening sockets.
///
/// Unset or empty is [`LEGACY_UNTIL`]; `off` ends the window now (an
/// operator whose clients are all updated, and the tests). Nothing can make
/// the window longer, and any other value refuses startup.
///
/// # Errors
/// The sentence the gateway refuses to start with.
pub fn legacy_from_env(raw: Option<&str>) -> Result<u64, String> {
    match raw.map(str::trim) {
        None | Some("") => Ok(LEGACY_UNTIL),
        Some("off") => Ok(0),
        Some(text) => Err(format!(
            "{text:?}: only \"off\" is understood (the window ends on its own after \
             {LEGACY_UNTIL_DATE})"
        )),
    }
}

/// Whether a socket may still be opened with its token in the query, at
/// `now` (unix seconds), for a window ending at `until`.
#[must_use]
pub const fn legacy_open(until: u64, now: u64) -> bool {
    now < until
}

#[cfg(test)]
mod tests {
    use super::*;

    const TTL: Duration = Duration::from_secs(DEFAULT_SECS);

    fn lobby(session: &[u8]) -> Grant {
        Grant::Lobby {
            account_id: "acct".into(),
            session: session.to_vec(),
        }
    }

    fn seat(game: &str, seat: usize, token_hash: &str) -> Grant {
        Grant::Seat {
            game_id: game.into(),
            seat,
            seat_token_hash: token_hash.into(),
        }
    }

    fn door(game: &str) -> Door {
        Door::Seat {
            game_id: game.into(),
        }
    }

    #[test]
    fn a_ticket_opens_its_door_once() {
        let store = Tickets::new(TTL);
        let t0 = Instant::now();
        let ticket = store.issue(lobby(b"s1"), t0).unwrap();
        assert_eq!(store.consume(&ticket, &Door::Lobby, t0), Ok(lobby(b"s1")));
        assert_eq!(
            store.consume(&ticket, &Door::Lobby, t0),
            Err(Refusal::Unknown),
            "a second use fails"
        );
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn a_ticket_is_256_random_bits_and_only_its_hash_is_kept() {
        let store = Tickets::new(TTL);
        let t0 = Instant::now();
        let a = store.issue(lobby(b"s"), t0).unwrap();
        let b = store.issue(lobby(b"s"), t0).unwrap();
        assert_eq!(a.len(), 64, "32 bytes, hex");
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
        let inner = store.inner.lock();
        assert!(!inner.live.contains_key(&a), "the ticket itself is no key");
        assert!(inner.live.contains_key(&crate::auth::token_hash(&a)));
    }

    #[test]
    fn a_ticket_dies_at_exactly_its_lifetime() {
        let store = Tickets::new(TTL);
        let t0 = Instant::now();
        let last = (t0 + TTL)
            .checked_sub(Duration::from_millis(1))
            .expect("an instant a millisecond earlier");
        let early = store.issue(lobby(b"s"), t0).unwrap();
        assert!(
            store.consume(&early, &Door::Lobby, last).is_ok(),
            "alive a millisecond before"
        );
        let late = store.issue(lobby(b"s"), t0).unwrap();
        assert_eq!(
            store.consume(&late, &Door::Lobby, t0 + TTL),
            Err(Refusal::Expired),
            "dead at issued + ttl"
        );
        assert_eq!(
            store.consume(&late, &Door::Lobby, t0),
            Err(Refusal::Unknown),
            "and an expired ticket is spent by being shown"
        );
    }

    #[test]
    fn a_lobby_ticket_opens_no_seat_and_a_seat_ticket_no_lobby() {
        let store = Tickets::new(TTL);
        let t0 = Instant::now();
        let for_lobby = store.issue(lobby(b"s"), t0).unwrap();
        assert_eq!(
            store.consume(&for_lobby, &door("g1"), t0),
            Err(Refusal::WrongDoor)
        );
        assert_eq!(
            store.consume(&for_lobby, &Door::Lobby, t0),
            Err(Refusal::Unknown),
            "refused at the wrong door is spent"
        );
        let for_seat = store.issue(seat("g1", 0, "h"), t0).unwrap();
        assert_eq!(
            store.consume(&for_seat, &Door::Lobby, t0),
            Err(Refusal::WrongDoor)
        );
    }

    #[test]
    fn a_seat_ticket_opens_its_own_game_only() {
        let store = Tickets::new(TTL);
        let t0 = Instant::now();
        let ticket = store.issue(seat("g1", 1, "h"), t0).unwrap();
        assert_eq!(
            store.consume(&ticket, &door("g2"), t0),
            Err(Refusal::WrongDoor)
        );
        let ticket = store.issue(seat("g1", 1, "h"), t0).unwrap();
        assert_eq!(
            store.consume(&ticket, &door("g1"), t0),
            Ok(seat("g1", 1, "h"))
        );
    }

    /// The seat and the session a ticket names come back to the upgrade,
    /// which is what compares them with the seat's current token and the
    /// session's standing: the store's half is handing back exactly what it
    /// was given, never another holder's grant.
    #[test]
    fn the_grant_handed_back_is_the_one_issued() {
        let store = Tickets::new(TTL);
        let t0 = Instant::now();
        let mine = store.issue(seat("g1", 0, "mine"), t0).unwrap();
        let theirs = store.issue(seat("g1", 1, "theirs"), t0).unwrap();
        let a = store.issue(lobby(b"alice"), t0).unwrap();
        let b = store.issue(lobby(b"bob"), t0).unwrap();
        assert_eq!(
            store.consume(&theirs, &door("g1"), t0),
            Ok(seat("g1", 1, "theirs"))
        );
        assert_eq!(
            store.consume(&mine, &door("g1"), t0),
            Ok(seat("g1", 0, "mine"))
        );
        assert_eq!(store.consume(&b, &Door::Lobby, t0), Ok(lobby(b"bob")));
        assert_eq!(store.consume(&a, &Door::Lobby, t0), Ok(lobby(b"alice")));
    }

    #[test]
    fn one_holder_keeps_at_most_its_cap_and_the_oldest_gives_way() {
        let store = Tickets::new(TTL);
        let t0 = Instant::now();
        let first = store.issue(lobby(b"greedy"), t0).unwrap();
        let rest: Vec<String> = (1..PER_HOLDER)
            .map(|_| store.issue(lobby(b"greedy"), t0).unwrap())
            .collect();
        let other = store.issue(lobby(b"quiet"), t0).unwrap();
        assert_eq!(store.len(), PER_HOLDER + 1);
        let newest = store.issue(lobby(b"greedy"), t0).unwrap();
        assert_eq!(store.len(), PER_HOLDER + 1, "one in, one out");
        assert_eq!(
            store.consume(&first, &Door::Lobby, t0),
            Err(Refusal::Unknown),
            "the oldest gave way"
        );
        for ticket in rest.iter().chain([&newest, &other]) {
            assert!(store.consume(ticket, &Door::Lobby, t0).is_ok());
        }
        assert!(
            store.inner.lock().by_holder.is_empty(),
            "no index outlives its tickets"
        );
    }

    #[test]
    fn the_gateway_as_a_whole_is_bounded() {
        let store = Tickets::new(TTL);
        let t0 = Instant::now();
        for n in 0..MAX_OUTSTANDING {
            store.issue(lobby(format!("s{n}").as_bytes()), t0).unwrap();
        }
        assert_eq!(store.issue(lobby(b"one more"), t0), Err(Full));
        assert!(
            store.issue(lobby(b"one more"), t0 + TTL).is_ok(),
            "full of expired tickets is not full"
        );
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn a_sweep_drops_the_expired_and_keeps_the_living() {
        let store = Tickets::new(TTL);
        let t0 = Instant::now();
        let old = store.issue(lobby(b"a"), t0).unwrap();
        let young = store
            .issue(lobby(b"a"), t0 + Duration::from_secs(10))
            .unwrap();
        assert_eq!(store.sweep(t0 + TTL), 1);
        assert_eq!(store.len(), 1);
        assert_eq!(
            store.consume(&old, &Door::Lobby, t0 + TTL),
            Err(Refusal::Unknown)
        );
        assert!(store.consume(&young, &Door::Lobby, t0 + TTL).is_ok());
        assert!(store.inner.lock().by_holder.is_empty());
    }

    fn chair(game: &str, at: usize, host: &str) -> Grant {
        Grant::Chair {
            game_id: game.into(),
            seat: at,
            host: host.into(),
            order: None,
        }
    }

    fn chair_door(game: &str, at: usize) -> Door {
        Door::Chair {
            game_id: game.into(),
            seat: at,
        }
    }

    #[test]
    fn a_chair_ticket_opens_its_own_chair_of_its_own_room_only() {
        let store = Tickets::new(TTL);
        let t0 = Instant::now();
        for wrong in [chair_door("g1", 2), chair_door("g2", 1), door("g1")] {
            let ticket = store.issue(chair("g1", 1, "host"), t0).unwrap();
            assert_eq!(
                store.consume(&ticket, &wrong, t0),
                Err(Refusal::WrongDoor),
                "{wrong:?}"
            );
            assert_eq!(
                store.consume(&ticket, &chair_door("g1", 1), t0),
                Err(Refusal::Unknown),
                "refused at the wrong chair is spent"
            );
        }
        let seat_ticket = store.issue(seat("g1", 1, "h"), t0).unwrap();
        assert_eq!(
            store.consume(&seat_ticket, &chair_door("g1", 1), t0),
            Err(Refusal::WrongDoor),
            "a seat ticket is no chair ticket"
        );
        let ticket = store.issue(chair("g1", 1, "host"), t0).unwrap();
        assert_eq!(
            store.consume(&ticket, &chair_door("g1", 1), t0),
            Ok(chair("g1", 1, "host"))
        );
    }

    #[test]
    fn a_revoked_grant_opens_nothing_and_the_others_stay() {
        let store = Tickets::new(TTL);
        let t0 = Instant::now();
        let gone = store.issue(chair("g1", 1, "left"), t0).unwrap();
        let kept = store.issue(chair("g1", 2, "stayed"), t0).unwrap();
        let socket = store.issue(lobby(b"s"), t0).unwrap();
        let revoked =
            store.revoke(|grant| matches!(grant, Grant::Chair { host, .. } if host == "left"));
        assert_eq!(revoked, 1);
        assert_eq!(
            store.consume(&gone, &chair_door("g1", 1), t0),
            Err(Refusal::Unknown)
        );
        assert!(store.consume(&kept, &chair_door("g1", 2), t0).is_ok());
        assert!(store.consume(&socket, &Door::Lobby, t0).is_ok());
        assert!(store.inner.lock().by_holder.is_empty());
    }

    #[test]
    fn the_lifetime_is_read_within_its_bounds_or_refused() {
        assert_eq!(ttl_from_env(None), Ok(Duration::from_secs(45)));
        assert_eq!(ttl_from_env(Some("")), Ok(Duration::from_secs(45)));
        assert_eq!(ttl_from_env(Some(" 1 ")), Ok(Duration::from_secs(1)));
        assert_eq!(ttl_from_env(Some("600")), Ok(Duration::from_secs(600)));
        for bad in [
            "0",
            "601",
            "86400",
            "-5",
            "45s",
            "1.5",
            "lots",
            "18446744073709551616",
        ] {
            assert!(ttl_from_env(Some(bad)).is_err(), "{bad:?} was accepted");
        }
    }

    #[test]
    fn the_old_query_token_has_a_dated_window_that_only_shortens() {
        assert_eq!(legacy_from_env(None), Ok(LEGACY_UNTIL));
        assert_eq!(legacy_from_env(Some("")), Ok(LEGACY_UNTIL));
        assert_eq!(legacy_from_env(Some("off")), Ok(0));
        for bad in ["on", "2027-01-01", "9999999999", "1"] {
            assert!(legacy_from_env(Some(bad)).is_err(), "{bad:?} was accepted");
        }
        // 2026-10-31T23:59:59Z is the last second; midnight is past it.
        assert!(legacy_open(LEGACY_UNTIL, LEGACY_UNTIL - 1));
        assert!(!legacy_open(LEGACY_UNTIL, LEGACY_UNTIL));
        assert!(!legacy_open(0, 1_790_000_000), "off is closed today");
        assert_eq!(
            LEGACY_UNTIL % 86_400,
            0,
            "the window ends at a midnight, UTC"
        );
        assert_eq!(
            (LEGACY_UNTIL - 1_767_225_600) / 86_400,
            304,
            "304 days after 2026-01-01 is 2026-11-01"
        );
    }
}
