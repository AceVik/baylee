//! Sessions, done to current best practice. No password is checked here:
//! accounts sign in at Baylee Central, and the legacy accounts' hashes went
//! to Central's legacy store at the cutover (`docs/central-login.md`).
//!
//! - Bearer tokens are 256 bits from the OS CSPRNG, stored **only as
//!   SHA-256 hashes** (the DB never holds a usable token), 12 h expiry
//!   with sliding renewal; 30 days for a guest's, whose session is the
//!   whole of its account ([`Lifetime`]).
//! - Sliding-window rate limiters throttle the public doors per IP (and a
//!   closed beta's key tries); `RateLimiter` holds the window and the count
//!   and the caller says what a key is.
//! - All secret comparisons are constant-time (`subtle`).

use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;
use uuid::Uuid;

/// Token lifetime (sliding).
#[allow(clippy::duration_suboptimal_units)]
pub const TOKEN_TTL: Duration = Duration::from_secs(12 * 3600);

/// A guest's token lifetime (sliding), and so a guest's: its session is the
/// only way into it, and the account goes once no session is left (#269).
#[allow(clippy::duration_suboptimal_units)]
pub const GUEST_TOKEN_TTL: Duration = Duration::from_secs(30 * 24 * 3600);

/// How long a session lives unused, and how often a use moves that on.
///
/// A use renews a session only once `renew_every` has passed since it was
/// last renewed, so that a session in steady use costs a write per
/// `renew_every` rather than one per request (`store::resolve_token`). The
/// price is that a session lapses up to `renew_every` earlier than its last
/// use plus `ttl`, which is the direction an error should go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lifetime {
    /// How long a session lives from its last renewal.
    pub ttl: Duration,
    /// How long after a renewal a use renews it again.
    pub renew_every: Duration,
}

impl Lifetime {
    /// An account's: twelve hours, renewed at the half-life.
    #[allow(clippy::duration_suboptimal_units)]
    pub const ACCOUNT: Self = Self {
        ttl: TOKEN_TTL,
        renew_every: Duration::from_secs(6 * 3600),
    };

    /// A guest's: thirty days, renewed at most once a day. At the half-life,
    /// a guest who played on day 16 would keep an account that a guest who
    /// played on day 14 lost on day 30; renewed daily, a guest lives 29 to 30
    /// days past the last call made with its token (any call renews, not
    /// only a game), which the client tells them as about thirty days after
    /// their last visit.
    #[allow(clippy::duration_suboptimal_units)]
    pub const GUEST: Self = Self {
        ttl: GUEST_TOKEN_TTL,
        renew_every: Duration::from_secs(24 * 3600),
    };

    /// The lifetime of a session of a guest's, or of an account's.
    #[must_use]
    pub const fn of(guest: bool) -> Self {
        if guest { Self::GUEST } else { Self::ACCOUNT }
    }

    /// Whether a use renews a session that has `left` seconds to run.
    #[must_use]
    pub const fn renews(self, left: u64) -> bool {
        left < self
            .ttl
            .as_secs()
            .saturating_sub(self.renew_every.as_secs())
    }
}

/// SHA-256 of a token, as the thirty-two bytes it is — the only form stored.
///
/// The database column is `bytea`, so nothing encodes on the way in and
/// nothing decodes on the way out. A hash is not text; storing it as sixty-
/// four hex characters was twice the bytes and a column that would accept
/// `hello`.
#[must_use]
pub fn token_digest(token: &str) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(token.as_bytes());
    h.finalize().to_vec()
}

/// The same digest as hex, for the things that are not rows.
///
/// A room's password and a seat's token are compared inside a `HashMap` the
/// gateway holds, never stored, so they are strings and stay strings.
#[must_use]
pub fn token_hash(token: &str) -> String {
    hex_lower(&token_digest(token))
}

/// A fresh 256-bit bearer token (hex).
#[must_use]
pub fn new_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("OS RNG available");
    hex_lower(&bytes)
}

/// A fresh seed for one game's shuffle, from the OS CSPRNG. Determinism
/// lives inside a game (seeded RNG); the seed itself must differ per game
/// or every match starts with the same library order.
#[must_use]
pub fn new_game_seed() -> u64 {
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).expect("OS RNG available");
    u64::from_le_bytes(bytes)
}

/// Constant-time string equality.
#[must_use]
pub fn ct_eq(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

pub(crate) fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, b| {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// An issued token (account binding + expiry).
#[derive(Clone, Debug)]
pub struct IssuedToken {
    /// The bearer token (returned to the client ONCE).
    pub token: String,
    /// Expiry (unix seconds).
    pub expires_at: u64,
}

impl IssuedToken {
    /// Issue a fresh token that lives as long as `lifetime` says.
    #[must_use]
    pub fn lasting(lifetime: Lifetime) -> Self {
        Self {
            token: new_token(),
            expires_at: now_secs() + lifetime.ttl.as_secs(),
        }
    }
}

/// Unix seconds now.
#[must_use]
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Sliding-window rate limiter for auth endpoints.
///
/// It counts keys and has no opinion about what a key *is*: an IP for the
/// routes where there is no account yet to name, the account for the one
/// route that is aimed at a particular account. That is the whole reason
/// it takes a `&str` — keying sign-ins on the machine punished a household
/// for one member's typing, and on a development box it made every scripted
/// call and the owner's own typing share a single budget.
pub struct RateLimiter {
    /// key → attempt timestamps within the window.
    hits: Mutex<HashMap<String, Vec<Instant>>>,
    /// Last time dead entries were swept (bounds the map's growth —
    /// unique keys must not grow it without limit).
    last_sweep: Mutex<Instant>,
    window: Duration,
    max_attempts: usize,
}

impl RateLimiter {
    /// `max_attempts` per `window` per key.
    #[must_use]
    pub fn new(window: Duration, max_attempts: usize) -> Self {
        Self {
            hits: Mutex::new(HashMap::new()),
            last_sweep: Mutex::new(Instant::now()),
            window,
            max_attempts,
        }
    }

    /// True when the attempt is allowed (and recorded).
    pub fn allow(&self, key: &str) -> bool {
        self.allow_at(key, Instant::now())
    }

    /// [`Self::allow`] at `now`, which the tests move on by hand.
    pub(crate) fn allow_at(&self, key: &str, now: Instant) -> bool {
        let mut hits = self.hits.lock();
        // Periodically drop entries with no hits inside the window.
        let mut last = self.last_sweep.lock();
        if now.duration_since(*last) >= self.window {
            hits.retain(|_, v| {
                v.retain(|t| now.duration_since(*t) < self.window);
                !v.is_empty()
            });
            *last = now;
        }
        drop(last);
        let entry = hits.entry(key.to_string()).or_default();
        entry.retain(|t| now.duration_since(*t) < self.window);
        if entry.len() >= self.max_attempts {
            return false;
        }
        entry.push(now);
        true
    }

    /// Drops a key's history, so what it has spent no longer counts.
    ///
    /// For the one thing a sliding window cannot see on its own: the attempts
    /// were a person getting their own password right in the end. Without
    /// this, seven typos followed by a success leave the eighth try spent,
    /// and signing out and back in is refused by a limiter that has already
    /// been satisfied. Only a caller that *knows* the attempt succeeded may
    /// call it, which is why it is not part of `allow`.
    pub fn forget(&self, key: &str) {
        self.hits.lock().remove(key);
    }

    /// Whether `key` has spent every attempt in the window, without
    /// counting one: for a limit on *failures*, which has to be asked
    /// before the attempt is judged and counted only when it fails.
    pub fn spent(&self, key: &str) -> bool {
        self.spent_at(key, Instant::now())
    }

    /// [`Self::spent`] at `now`.
    pub(crate) fn spent_at(&self, key: &str, now: Instant) -> bool {
        self.hits.lock().get(key).is_some_and(|hits| {
            hits.iter()
                .filter(|t| now.duration_since(**t) < self.window)
                .count()
                >= self.max_attempts
        })
    }

    /// Takes back the latest attempt under `key`, for one that did not
    /// happen after all.
    pub fn give_back(&self, key: &str) {
        if let Some(hits) = self.hits.lock().get_mut(key) {
            hits.pop();
        }
    }
}

/// Display-name validation (shown to other players).
///
/// Three to sixteen characters, ASCII letters and digits with `_` and `-`
/// between them, beginning and ending on a letter or a digit.
///
/// Sixteen rather than thirty-two because the name is drawn on a seat's own
/// bar, where the shelf projects 151 px on an eight-seat ring and a long
/// name is what pushes that bar down a density. The character set is a
/// refusal list as much as an allow list: `@` would make a name look like an
/// address on a screen that also shows addresses, a space makes two names
/// that read alike (`Alice Smith` and `Alice  Smith`), and `#` is the one
/// character that must never appear in a name, because [`crate::handle`]
/// puts it between a name and a tag. A leading or trailing `_` is the
/// cheapest way to dress a name up as somebody else's.
///
/// What this no longer does is decide who gets the name. Names are not
/// unique — `Alice#3f0a` and `Alice#af03` are two people — so this refuses
/// *shapes*, never claims.
#[must_use]
pub fn valid_display_name(name: &str) -> bool {
    if !(3..=16).contains(&name.len()) {
        return false;
    }
    let plain = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric());
    plain(name.chars().next())
        && plain(name.chars().next_back())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// A fresh `UUIDv7` for something that lives only in this process.
///
/// A game, a room and an agent are named by one of these, and none of them
/// is a row: the lobby is a `HashMap` that dies with the gateway. Anything
/// that *is* a row — an account, a deck — is given its id by Postgres
/// (`DEFAULT uuidv7()`), because a `UUIDv7` is ordered by the clock of
/// whatever minted it and several gateways have several clocks.
#[must_use]
pub fn new_id() -> String {
    Uuid::now_v7().to_string()
}

/// A password hashed as this gateway hashed them (Argon2id under argon2's
/// defaults, a PHC string): for the cutover export's tests, which must parse
/// the gateway's own hashes.
#[cfg(test)]
pub fn hash_password(password: &str) -> String {
    use argon2::{Argon2, PasswordHasher, password_hash::phc::PasswordHash};
    let hash: PasswordHash = Argon2::default()
        .hash_password(password.as_bytes())
        .expect("argon2 hashing works");
    hash.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The table of cases baylee-central's `central_api::profile` tests too:
    /// Central keeps the same rule for the names the bridge carries and the
    /// ones people choose there, and refuses a name that breaks it.
    const SHARED_NAMES: [(&str, bool); 13] = [
        ("Alice", true),
        ("player_one-99", true),
        ("ab", false),
        ("has space", false),
        ("emoji🎉", false),
        ("sixteen_chars_yz", true),
        ("seventeen_chars_x", false),
        ("alice@example.com", false),
        ("Alice#af03", false),
        ("_Alice", false),
        ("Alice-", false),
        ("abc", true),
        ("a_-b", true),
    ];

    #[test]
    fn the_display_name_rule_is_centrals() {
        for (name, valid) in SHARED_NAMES {
            assert_eq!(valid_display_name(name), valid, "{name}");
            assert_eq!(
                central_api::profile::valid_display_name(name),
                valid,
                "Central reads {name} as this gateway does"
            );
        }
    }

    #[test]
    fn display_name_validation() {
        assert!(valid_display_name("Alice"));
        assert!(valid_display_name("player_one-99"));
        assert!(!valid_display_name("ab"));
        assert!(!valid_display_name("has space"));
        assert!(!valid_display_name("emoji🎉"));
        // Sixteen, not thirty-two.
        assert!(valid_display_name("sixteen_chars_yz"));
        assert!(!valid_display_name("seventeen_chars_x"));
        // An address is not a name, and a name may not carry the one
        // character that separates a name from its tag.
        assert!(!valid_display_name("alice@example.com"));
        assert!(!valid_display_name("Alice#af03"));
        // Dressing a name up as somebody else's with a separator on the end.
        assert!(!valid_display_name("_Alice"));
        assert!(!valid_display_name("Alice-"));
    }

    /// A session is renewed once its interval has passed since the last
    /// renewal and not a second before: an account's at its half-life, a
    /// guest's a day in (#269).
    #[test]
    fn a_session_is_renewed_once_its_interval_has_passed() {
        let hour = 3600;
        let day = 24 * hour;
        let account = Lifetime::of(false);
        assert_eq!(account, Lifetime::ACCOUNT);
        assert!(!account.renews(12 * hour), "fresh");
        assert!(!account.renews(6 * hour), "exactly at the half-life");
        assert!(account.renews(6 * hour - 1));
        let guest = Lifetime::of(true);
        assert_eq!(guest, Lifetime::GUEST);
        assert!(!guest.renews(30 * day), "fresh");
        assert!(!guest.renews(29 * day), "exactly a day in");
        assert!(guest.renews(29 * day - 1));
        assert!(
            guest.renews(12 * hour),
            "half a day left is due, whatever an account's would be"
        );
    }

    #[test]
    fn tokens_are_random_and_hashed() {
        let a = new_token();
        let b = new_token();
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        assert_ne!(token_hash(&a), a);
        assert!(ct_eq(&a, &a));
        assert!(!ct_eq(&a, &b));
    }

    #[test]
    fn rate_limiter_windows() {
        let limiter = RateLimiter::new(Duration::from_secs(60), 2);
        assert!(limiter.allow("1.2.3.4"));
        assert!(limiter.allow("1.2.3.4"));
        assert!(!limiter.allow("1.2.3.4"));
        assert!(limiter.allow("5.6.7.8"));
    }

    /// Getting the password right in the end is the thing a sliding window
    /// cannot see, so the caller that saw it says so.
    #[test]
    fn what_a_key_has_spent_can_be_given_back() {
        let limiter = RateLimiter::new(Duration::from_secs(60), 2);
        assert!(limiter.allow("a@b.c"));
        assert!(limiter.allow("a@b.c"));
        assert!(!limiter.allow("a@b.c"), "spent");
        limiter.forget("a@b.c");
        assert!(limiter.allow("a@b.c"), "and handed back");
        // One key's history, not everybody's.
        assert!(limiter.allow("d@e.f"));
        assert!(limiter.allow("d@e.f"));
        limiter.forget("a@b.c");
        assert!(
            !limiter.allow("d@e.f"),
            "somebody else's count is untouched"
        );
    }

    /// **One digest, two spellings, and they have to agree.** A session token
    /// is looked up as the thirty-two bytes a `bytea` column holds; a room's
    /// password and a seat's token never reach a row and are compared as hex
    /// inside a map. A difference between the two would be a token that
    /// cannot be found by the thing that stored it.
    #[test]
    fn the_stored_digest_and_the_written_one_are_the_same_digest() {
        let token = new_token();

        assert_eq!(
            token_digest(&token).len(),
            32,
            "SHA-256 is thirty-two bytes"
        );
        assert_eq!(token_hash(&token), hex_lower(&token_digest(&token)));
        assert_eq!(token_hash(&token).len(), 64);
        assert!(
            token_hash(&token)
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "lower-case hex, which is what the column and the map both hold"
        );
        assert_ne!(token_digest(&token), token_digest(&format!("{token}x")));
    }

    /// A token is 256 bits of the OS CSPRNG written as hex, so the length is
    /// not the measure — sixty-four *hex* characters is thirty-two bytes and
    /// sixty-four of anything else is not.
    #[test]
    fn a_token_is_thirty_two_bytes_of_randomness_and_not_sixty_four_characters() {
        let token = new_token();
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(new_token(), new_token());
    }

    /// Constant time is about the comparison and not about the lengths: two
    /// strings of different lengths are not equal, and saying so is not a
    /// leak, because the length of a token is public.
    #[test]
    fn a_comparison_of_two_lengths_is_a_refusal_and_not_a_panic() {
        assert!(ct_eq("", ""));
        assert!(!ct_eq("a", "ab"));
        assert!(!ct_eq("ab", "a"));
        assert!(!ct_eq("", "a"));
        assert!(ct_eq(&"x".repeat(64), &"x".repeat(64)));
    }

    /// **The window slides**, which is the half the counting tests cannot
    /// see: eight wrong passwords are a lockout for five minutes and not for
    /// ever, so a key that has spent its budget gets it back by waiting.
    #[test]
    fn asking_whether_a_budget_is_spent_spends_none_of_it() {
        let limiter = RateLimiter::new(Duration::from_secs(60), 2);
        let now = Instant::now();
        for _ in 0..10 {
            assert!(!limiter.spent_at("console", now), "asking counted");
        }
        assert!(limiter.allow_at("console", now));
        assert!(!limiter.spent_at("console", now));
        assert!(limiter.allow_at("console", now));
        assert!(limiter.spent_at("console", now), "two of two spent");
        assert!(
            !limiter.spent_at("console", now + Duration::from_secs(61)),
            "the window moved on"
        );
        assert!(!limiter.spent_at("someone else", now));
    }

    // The clock is moved by hand in these: with real sleeps of 80–220 ms a
    // loaded runner swept the map mid-loop and failed them (run 40622679934).
    #[test]
    fn a_spent_budget_comes_back_when_the_window_has_passed() {
        let limiter = RateLimiter::new(Duration::from_millis(120), 2);
        let now = Instant::now();

        assert!(limiter.allow_at("a@b.c", now));
        assert!(limiter.allow_at("a@b.c", now));
        assert!(!limiter.allow_at("a@b.c", now), "spent inside the window");

        assert!(
            limiter.allow_at("a@b.c", now + Duration::from_millis(220)),
            "and handed back by the window moving on, with nobody saying so"
        );
    }

    /// The same budget coming back **between two sweeps**, which is the half
    /// the test above cannot see: the periodic sweep prunes every key, and
    /// by the time a key's hits are stale a sweep is usually due anyway, so
    /// it reaches them first. Removing `allow`'s own `retain` leaves that
    /// test green.
    ///
    /// So the sweep is put out of reach by saying it has just run, and what
    /// is left is the entry pruning its own hits. A key that attempts often
    /// enough to stay in the map is exactly the key this matters for.
    #[test]
    fn an_entry_prunes_its_own_hits_when_no_sweep_is_due() {
        let limiter = RateLimiter::new(Duration::from_millis(40), 2);
        let now = Instant::now();

        assert!(limiter.allow_at("a@b.c", now));
        assert!(limiter.allow_at("a@b.c", now));
        assert!(!limiter.allow_at("a@b.c", now), "spent");

        let later = now + Duration::from_millis(80);
        *limiter.last_sweep.lock() = later;
        assert!(
            limiter.allow_at("a@b.c", later),
            "the sweep is not due, and the entry's own hits are outside the \
             window"
        );
    }

    /// **A unique key is not a way to grow this map**, which is why the
    /// sweep exists: the keys are typed names and IP addresses a stranger
    /// chooses, and one attempt each would otherwise be one entry
    /// each for as long as the process runs.
    ///
    /// The sweep is periodic rather than per call, so it happens on the
    /// first attempt after a window has passed — and it runs before that
    /// attempt is recorded, which is why one key is left and not two.
    #[test]
    fn a_key_nobody_uses_again_does_not_stay_in_the_map() {
        let limiter = RateLimiter::new(Duration::from_millis(80), 2);
        let now = Instant::now();

        for n in 0..50 {
            assert!(limiter.allow_at(&format!("stranger-{n}"), now));
        }
        assert_eq!(limiter.hits.lock().len(), 50, "one entry each, so far");

        assert!(limiter.allow_at("the attempt that sweeps", now + Duration::from_millis(160)));
        assert_eq!(
            limiter.hits.lock().len(),
            1,
            "fifty keys with nothing inside the window are fifty keys gone"
        );
    }
}
