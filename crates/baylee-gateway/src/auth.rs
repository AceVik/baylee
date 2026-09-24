//! Authentication, done to current best practice:
//!
//! - Passwords are hashed with **Argon2id** (memory-hard, PHC strings).
//! - Bearer tokens are 256 bits from the OS CSPRNG, stored **only as
//!   SHA-256 hashes** (the DB never holds a usable token), 12 h expiry
//!   with sliding renewal.
//! - Login errors are identical for unknown users and wrong passwords,
//!   and unknown-user attempts verify against a fixed dummy hash so
//!   response timing doesn't leak account existence.
//! - Sliding-window rate limiters throttle the auth endpoints. Signing in
//!   is counted per **account**, however it was named (#269), and the rest
//!   per IP; `RateLimiter` holds the window and the count and the caller
//!   says what a key is.
//! - All secret comparisons are constant-time (`subtle`).

use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash;
use argon2::{PasswordHasher, PasswordVerifier};
use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;
use uuid::Uuid;

/// Token lifetime (sliding).
#[allow(clippy::duration_suboptimal_units)]
pub const TOKEN_TTL: Duration = Duration::from_secs(12 * 3600);

/// Hash a password for storage (Argon2id, random salt).
///
/// # Panics
/// Only when the OS RNG is unavailable (unrecoverable anyway).
#[must_use]
pub fn hash_password(password: &str) -> String {
    // The salt is the library's own since password-hash 0.6:
    // `PasswordHasher::hash_password` draws `RECOMMENDED_SALT_LEN` bytes
    // from the OS CSPRNG itself, which is why this no longer reaches for
    // `getrandom` and a `SaltString` (the type is gone).
    let hash: PasswordHash = Argon2::default()
        .hash_password(password.as_bytes())
        .expect("argon2 hashing works");
    hash.to_string()
}

/// Argon2id hash of a fixed password, computed once with the **same**
/// parameters as a real one, because it is computed the same way.
///
/// A hard-coded literal drifted out of sync once already (m=65536,t=3
/// against m=19456,t=2) and made an unknown user measurably *slower* to
/// refuse than a real one — the exact leak the dummy is there to close.
/// Calling [`hash_password`] is what makes that impossible rather than
/// unlikely, and it is a `static` rather than a local so that a test can say
/// so out loud.
static DUMMY: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| hash_password("dummy-password"));

/// Verify a password against a stored PHC hash. Unknown users verify
/// against a fixed dummy hash to equalize timing.
#[must_use]
pub fn verify_password(stored: Option<&str>, password: &str) -> bool {
    let phc = stored.unwrap_or(&DUMMY);
    let Ok(parsed) = PasswordHash::new(phc) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
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

fn hex_lower(bytes: &[u8]) -> String {
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
    /// Issue a fresh token.
    #[must_use]
    pub fn new() -> Self {
        Self {
            token: new_token(),
            expires_at: now_secs() + TOKEN_TTL.as_secs(),
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
        let mut hits = self.hits.lock();
        let now = Instant::now();
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

/// Input validation for passwords (hygiene, not a strength meter):
/// length, not the username or the display name, and not a top common
/// password.
#[must_use]
pub fn valid_password(username: &str, display_name: &str, password: &str) -> bool {
    const COMMON: &[&str] = &[
        "password",
        "password1",
        "12345678",
        "123456789",
        "1234567890",
        "qwerty123",
        "letmein123",
        "iloveyou",
        "dragon123",
        "master123",
        "monkey123",
        "abc12345",
    ];
    // Eight, which is what NIST SP 800-63B asks for, rather than the ten this
    // started at. The bound that matters is the upper one (a hash is not a
    // place to put a megabyte) and the checks below it; a longer minimum buys
    // very little against an offline attack on an Argon2id hash and costs a
    // real player a rejected password on a game account.
    if !(8..=256).contains(&password.len()) {
        return false;
    }
    // Either name in any case: the username is half of the credential, and
    // the display name is on every table the player sits at.
    if password.eq_ignore_ascii_case(username) || password.eq_ignore_ascii_case(display_name) {
        return false;
    }
    !COMMON.iter().any(|c| password.eq_ignore_ascii_case(c))
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn password_rules() {
        assert!(valid_password(
            "wonderland",
            "Alice",
            "a-very-fine-password"
        ));
        assert!(!valid_password("wonderland", "Alice", "short"));
        // Eight is the floor, and seven is under it.
        assert!(valid_password("wonderland", "Alice", "8charact"));
        assert!(!valid_password("wonderland", "Alice", "7chars!"));
        // Neither name, in any case, and each long enough that the length
        // is not what refuses it.
        assert!(!valid_password("wonderland", "Alice", "WonderLand"));
        assert!(!valid_password("wonderland", "Rabbit_hole", "rabbit_HOLE"));
        assert!(valid_password("wonderland", "Rabbit_hole", "wonderland2"));
        assert!(!valid_password("wonderland", "Alice", "password"));
    }

    #[test]
    fn hash_and_verify_roundtrip() {
        let hash = hash_password("a-very-fine-password");
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password(Some(&hash), "a-very-fine-password"));
        assert!(!verify_password(Some(&hash), "wrong-password-xx"));
    }

    #[test]
    fn unknown_user_verifies_against_dummy_without_leaking() {
        // Unknown e-mail: dummy-hash path returns false (and costs the
        // same work, so timing doesn't leak account existence).
        assert!(!verify_password(None, "a-very-fine-password"));
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

    /// **The dummy an unknown user is refused against costs what a real one
    /// costs**, which is the whole of what it is for: the work has to be the
    /// same or the refusal is faster and the timing says the account does
    /// not exist.
    ///
    /// A hard-coded literal drifted once already — m=65536,t=3 against
    /// m=19456,t=2 — and made the unknown user the *slower* of the two.
    /// Computing it through `hash_password` is what closed that, and this is
    /// the assertion that says so rather than leaving it to a reader to
    /// notice.
    #[test]
    fn an_unknown_user_is_refused_against_a_hash_that_cost_what_a_real_one_costs() {
        let real = PasswordHash::new(&hash_password("a-very-fine-password"))
            .expect("a hash this crate just wrote");
        let dummy = PasswordHash::new(&DUMMY).expect("and the one it refuses with");

        assert_eq!(real.algorithm.as_str(), dummy.algorithm.as_str());
        assert_eq!(
            real.params.to_string(),
            dummy.params.to_string(),
            "the work an unknown user costs is the work a real one costs"
        );
        assert_eq!(
            format!("{:?}", real.version),
            format!("{:?}", dummy.version)
        );
        assert_ne!(
            real.salt.map(|s| s.to_string()),
            dummy.salt.map(|s| s.to_string()),
            "and it is the same recipe rather than the same hash"
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
    fn a_spent_budget_comes_back_when_the_window_has_passed() {
        let limiter = RateLimiter::new(Duration::from_millis(120), 2);

        assert!(limiter.allow("a@b.c"));
        assert!(limiter.allow("a@b.c"));
        assert!(!limiter.allow("a@b.c"), "spent inside the window");

        std::thread::sleep(Duration::from_millis(220));
        assert!(
            limiter.allow("a@b.c"),
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

        assert!(limiter.allow("a@b.c"));
        assert!(limiter.allow("a@b.c"));
        assert!(!limiter.allow("a@b.c"), "spent");

        std::thread::sleep(Duration::from_millis(80));
        *limiter.last_sweep.lock() = Instant::now();
        assert!(
            limiter.allow("a@b.c"),
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

        for n in 0..50 {
            assert!(limiter.allow(&format!("stranger-{n}")));
        }
        assert_eq!(limiter.hits.lock().len(), 50, "one entry each, so far");

        std::thread::sleep(Duration::from_millis(160));
        assert!(limiter.allow("the attempt that sweeps"));
        assert_eq!(
            limiter.hits.lock().len(),
            1,
            "fifty keys with nothing inside the window are fifty keys gone"
        );
    }
}
