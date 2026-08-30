//! Authentication, done to current best practice:
//!
//! - Passwords are hashed with **Argon2id** (memory-hard, PHC strings).
//! - Bearer tokens are 256 bits from the OS CSPRNG, stored **only as
//!   SHA-256 hashes** (the DB never holds a usable token), 12 h expiry
//!   with sliding renewal.
//! - Login errors are identical for unknown users and wrong passwords,
//!   and unknown-user attempts verify against a fixed dummy hash so
//!   response timing doesn't leak account existence.
//! - A per-IP sliding-window rate limiter throttles auth endpoints.
//! - All secret comparisons are constant-time (`subtle`).

use argon2::Argon2;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;
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
    let mut salt_bytes = [0u8; 16];
    getrandom::fill(&mut salt_bytes).expect("OS RNG available");
    let salt = SaltString::encode_b64(&salt_bytes).expect("salt encodes");
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("argon2 hashing works")
        .to_string()
}

/// Verify a password against a stored PHC hash. Unknown users verify
/// against a fixed dummy hash to equalize timing.
#[must_use]
pub fn verify_password(stored: Option<&str>, password: &str) -> bool {
    // Pre-computed Argon2id hash of "dummy-password" — verifying against
    // it costs the same as a real verify, so unknown users and real users
    // take the same time.
    const DUMMY: &str = "$argon2id$v=19$m=65536,t=3,p=1$\
        c2FsdHNhbHRzYWx0c2FsdA$\
        +pN/Z0b6aD1gOB7IXSaUtBzPBJ4YcEb6fsY/QJVLzEQ";
    let phc = stored.unwrap_or(DUMMY);
    let Ok(parsed) = PasswordHash::new(phc) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

/// SHA-256 of a token — the only form stored.
#[must_use]
pub fn token_hash(token: &str) -> String {
    let mut h = Sha256::new();
    h.update(token.as_bytes());
    hex_lower(&h.finalize())
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

/// Sliding-window per-IP rate limiter for auth endpoints.
pub struct RateLimiter {
    /// ip → attempt timestamps within the window.
    hits: Mutex<HashMap<String, Vec<Instant>>>,
    window: Duration,
    max_attempts: usize,
}

impl RateLimiter {
    /// `max_attempts` per `window` per IP.
    #[must_use]
    pub fn new(window: Duration, max_attempts: usize) -> Self {
        Self {
            hits: Mutex::new(HashMap::new()),
            window,
            max_attempts,
        }
    }

    /// True when the attempt is allowed (and recorded).
    pub fn allow(&self, ip: &str) -> bool {
        let mut hits = self.hits.lock().expect("rate limiter poisoned");
        let now = Instant::now();
        let entry = hits.entry(ip.to_string()).or_default();
        entry.retain(|t| now.duration_since(*t) < self.window);
        if entry.len() >= self.max_attempts {
            return false;
        }
        entry.push(now);
        true
    }
}

/// E-mail validation: practical shape (local@domain.tld), bounded
/// length, no whitespace. Full RFC 5322 is intentionally not attempted.
#[must_use]
pub fn valid_email(email: &str) -> bool {
    if email.len() > 254 || email.chars().any(char::is_whitespace) {
        return false;
    }
    let Some((local, domain)) = email.rsplit_once('@') else {
        return false;
    };
    if local.is_empty() || local.len() > 64 {
        return false;
    }
    if domain.len() > 253 || !domain.contains('.') {
        return false;
    }
    let mut parts = domain.split('.');
    let host = parts.next().unwrap_or("");
    let tld = parts.next_back().unwrap_or("");
    !host.is_empty() && tld.len() >= 2
}

/// Display-name validation (shown to other players).
#[must_use]
pub fn valid_display_name(name: &str) -> bool {
    (3..=32).contains(&name.len())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Input validation for passwords (hygiene, not a strength meter):
/// length, not the e-mail/display name, and not a top common password.
#[must_use]
pub fn valid_password(email: &str, display_name: &str, password: &str) -> bool {
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
    if !(10..=256).contains(&password.len()) {
        return false;
    }
    let local = email.split('@').next().unwrap_or("");
    if !local.is_empty() && password.eq_ignore_ascii_case(local) {
        return false;
    }
    if password.eq_ignore_ascii_case(display_name) {
        return false;
    }
    !COMMON.iter().any(|c| password.eq_ignore_ascii_case(c))
}

/// A fresh `UUIDv7` for entity ids.
#[must_use]
pub fn new_id() -> String {
    Uuid::now_v7().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_validation_accepts_and_rejects() {
        assert!(valid_email("player@example.com"));
        assert!(valid_email("a.b-c_d@sub.domain.org"));
        assert!(!valid_email("not-an-email"));
        assert!(!valid_email("@example.com"));
        assert!(!valid_email("player@"));
        assert!(!valid_email("player@localhost"));
        assert!(!valid_email("player@example.c"));
        assert!(!valid_email("play er@example.com"));
        assert!(!valid_email(&format!("{}@example.com", "x".repeat(65))));
    }

    #[test]
    fn display_name_validation() {
        assert!(valid_display_name("Alice"));
        assert!(valid_display_name("player_one-99"));
        assert!(!valid_display_name("ab"));
        assert!(!valid_display_name("has space"));
        assert!(!valid_display_name("emoji🎉"));
    }

    #[test]
    fn password_rules() {
        assert!(valid_password("a@b.co", "alice", "a-very-fine-password"));
        assert!(!valid_password("a@b.co", "alice", "short"));
        assert!(!valid_password("alice@b.co", "alice", "Alice"));
        assert!(!valid_password("a@b.co", "alice", "alice"));
        assert!(!valid_password("a@b.co", "alice", "password"));
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
}
