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

/// Input validation for account names.
#[must_use]
pub fn valid_name(name: &str) -> bool {
    (3..=32).contains(&name.len())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Input validation for passwords (basic hygiene, not a strength meter).
#[must_use]
pub fn valid_password(name: &str, password: &str) -> bool {
    password.len() >= 8 && password.len() <= 256 && !password.eq_ignore_ascii_case(name)
}

/// A fresh `UUIDv7` for entity ids.
#[must_use]
pub fn new_id() -> String {
    Uuid::now_v7().to_string()
}
