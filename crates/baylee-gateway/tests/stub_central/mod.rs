//! A stub Baylee Central on loopback, and a gateway bridged to it: for what a
//! gateway does with Central's answers, and with having none. It answers the
//! native protocol's few requests a gateway's sessions make; nothing here
//! reaches a real service.

#![allow(dead_code)] // each test binary uses its own part of this

use super::common;
use baylee_account::wire::{Identity, Request, Response};
use baylee_account::{Endpoint, Token};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use zeroize::Zeroizing;

/// What the stub says to a lease it is asked about.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Says {
    /// The session stands.
    Good,
    /// `401`: Central refuses the lease, a real revocation.
    Revoked,
    /// `503`: Central is there and cannot answer.
    Down,
    /// `200` with a page that is no answer at all, as a proxy in the way sends.
    Garbage,
}

/// What the stub holds; every field may change while it serves.
#[derive(Default)]
pub struct Shared {
    /// The account each lease's first byte names.
    identities: Mutex<HashMap<u8, [u8; 16]>>,
    /// What it says to each lease, by first byte; `Good` when unset.
    says: Mutex<HashMap<u8, Says>>,
    /// How long each lease's check takes, by first byte, on top of `delay`.
    slow: Mutex<HashMap<u8, Duration>>,
    /// How long every answer takes, in milliseconds.
    delay: AtomicU64,
    /// When the sessions end, in Unix seconds.
    expires: AtomicU64,
    /// How many answers it gave.
    answered: AtomicU64,
    /// The most requests it held at once, and how many it holds now.
    peak: AtomicU64,
    held: AtomicU64,
    /// Whose identity a sign-in is answered with, by lease first byte.
    signing_in: AtomicU8,
}

/// The stub, serving.
#[derive(Clone)]
pub struct Stub {
    pub port: u16,
    pub shared: Arc<Shared>,
}

impl Stub {
    /// Starts it, every answer taking `delay`.
    #[must_use]
    pub fn serve(delay: Duration) -> Self {
        let shared = Arc::new(Shared::default());
        shared.delay.store(
            u64::try_from(delay.as_millis()).expect("fits"),
            Ordering::Relaxed,
        );
        shared.expires.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("after 1970")
                .as_secs()
                + 3600,
            Ordering::Relaxed,
        );
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
        let port = listener.local_addr().expect("an address").port();
        let serving = Arc::clone(&shared);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let shared = Arc::clone(&serving);
                std::thread::spawn(move || keep_answering(stream, &shared));
            }
        });
        Self { port, shared }
    }

    /// From now on it says `says` of lease `key`.
    pub fn say(&self, key: u8, says: Says) {
        self.shared.says.lock().expect("the map").insert(key, says);
    }

    /// From now on lease `key`'s check takes `by` longer.
    pub fn slow_down(&self, key: u8, by: Duration) {
        self.shared.slow.lock().expect("the map").insert(key, by);
    }

    /// How many answers it has given.
    #[must_use]
    pub fn answered(&self) -> u64 {
        self.shared.answered.load(Ordering::Relaxed)
    }

    /// The most requests it has held at once.
    #[must_use]
    pub fn peak(&self) -> u64 {
        self.shared.peak.load(Ordering::Relaxed)
    }

    /// When the sessions it stands for end.
    #[must_use]
    pub fn expires(&self) -> i64 {
        i64::try_from(self.shared.expires.load(Ordering::Relaxed)).expect("fits")
    }

    /// A sign-in is answered as the account lease `key` names.
    pub fn shared_sign_in(&self, key: u8) {
        self.shared.signing_in.store(key, Ordering::Relaxed);
    }

    /// Lease `key` names `central`.
    pub fn knows(&self, key: u8, central: uuid::Uuid) {
        self.shared
            .identities
            .lock()
            .expect("the map")
            .insert(key, *central.as_bytes());
    }
}

/// One request's body, once its headers and `Content-Length` are in.
fn body_of(stream: &mut std::net::TcpStream) -> Option<Vec<u8>> {
    let mut raw = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let read = stream.read(&mut chunk).ok().filter(|n| *n > 0)?;
        raw.extend_from_slice(&chunk[..read]);
        let Some(end) = raw.windows(4).position(|w| w == b"\r\n\r\n") else {
            continue;
        };
        let head = String::from_utf8_lossy(&raw[..end]).to_lowercase();
        let length: usize = head
            .lines()
            .find_map(|line| line.strip_prefix("content-length: "))
            .and_then(|n| n.trim().parse().ok())
            .unwrap_or(0);
        while raw.len() < end + 4 + length {
            let read = stream.read(&mut chunk).ok().filter(|n| *n > 0)?;
            raw.extend_from_slice(&chunk[..read]);
        }
        return Some(raw[end + 4..end + 4 + length].to_vec());
    }
}

/// What one request is answered: its status and body, and how long it takes
/// beyond every answer's delay.
fn answer_to(body: &[u8], shared: &Shared) -> (&'static str, Vec<u8>, Duration) {
    let known = |key: u8| {
        shared
            .identities
            .lock()
            .expect("the map")
            .get(&key)
            .copied()
    };
    let expires = i64::try_from(shared.expires.load(Ordering::Relaxed)).expect("fits");
    let good = |response: &Response| {
        (
            "200 OK",
            central_api::encode(response).expect("encoded"),
            Duration::ZERO,
        )
    };
    let refused = ("503 Service Unavailable", Vec::new(), Duration::ZERO);
    match central_api::decode::<Request>(body) {
        Ok(Request::GatewayAuthorize { lease, .. }) => {
            let key = lease.to_secret_bytes()[0];
            let says = shared
                .says
                .lock()
                .expect("the map")
                .get(&key)
                .copied()
                .unwrap_or(Says::Good);
            let slow = shared
                .slow
                .lock()
                .expect("the map")
                .get(&key)
                .copied()
                .unwrap_or_default();
            match (says, known(key)) {
                (Says::Good, Some(account)) => {
                    let (status, body, _) = good(&Response::GatewayAuthorization {
                        identity: Identity { account, epoch: 1 },
                        expires_at: expires,
                        central_terms: true,
                        gateway_terms: true,
                    });
                    (status, body, slow)
                }
                (Says::Revoked, _) => ("401 Unauthorized", Vec::new(), slow),
                (Says::Garbage, _) => ("200 OK", b"<html>a proxy</html>".to_vec(), slow),
                _ => (refused.0, Vec::new(), slow),
            }
        }
        Ok(Request::RedeemGateway { ticket, .. }) => {
            // A ticket of zeros is a stranger's: refused as Central refuses.
            if ticket.to_secret_bytes().iter().all(|b| *b == 0) {
                return ("401 Unauthorized", Vec::new(), Duration::ZERO);
            }
            let key = shared.signing_in.load(Ordering::Relaxed);
            known(key).map_or(refused, |account| {
                good(&Response::GatewayIdentity {
                    identity: Identity { account, epoch: 1 },
                    legacy_id: None,
                    lease: Some(Token::from_secret_bytes(Zeroizing::new([key; 32]))),
                    expires_at: Some(expires),
                })
            })
        }
        _ => refused,
    }
}

/// Answers `stream` until it closes, keeping it open as Central's server does.
fn keep_answering(mut stream: std::net::TcpStream, shared: &Shared) {
    while let Some(body) = body_of(&mut stream) {
        let now = shared.held.fetch_add(1, Ordering::Relaxed) + 1;
        shared.peak.fetch_max(now, Ordering::Relaxed);
        let (status, reply, extra) = answer_to(&body, shared);
        std::thread::sleep(Duration::from_millis(shared.delay.load(Ordering::Relaxed)) + extra);
        shared.held.fetch_sub(1, Ordering::Relaxed);
        shared.answered.fetch_add(1, Ordering::Relaxed);
        let kind = if reply.starts_with(b"<html") {
            "text/html"
        } else {
            "application/json"
        };
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\n\r\n",
            reply.len()
        );
        if stream.write_all(head.as_bytes()).is_err() || stream.write_all(&reply).is_err() {
            return;
        }
    }
}

/// A gateway whose bridge is `stub`, and the file its settings are in.
#[must_use]
pub fn gateway(stub: &Stub, label: &str) -> (common::Gateway, Settings) {
    let endpoint =
        Endpoint::development(&format!("http://127.0.0.1:{}", stub.port)).expect("an origin");
    let bridge = Token::generate().expect("a token");
    let file = std::env::temp_dir().join(format!("baylee-stub-{}.json", uuid::Uuid::now_v7()));
    std::fs::write(
        &file,
        serde_json::to_vec(&serde_json::json!({"endpoint": endpoint.origin(), "bridge": bridge}))
            .expect("settings"),
    )
    .expect("written");
    let gw = common::spawn_gateway_with(
        label,
        &[(
            "BAYLEE_CENTRAL_CONFIG_FILE",
            file.to_string_lossy().into_owned(),
        )],
    );
    (gw, Settings(file))
}

/// The gateway's settings file, gone with the test.
pub struct Settings(std::path::PathBuf);

impl Drop for Settings {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// An account linked to a new Central identity with a session on lease `key`,
/// which the stub knows. Answers its bearer.
#[must_use]
pub fn linked(gw: &common::Gateway, stub: &Stub, key: u8) -> String {
    use sha2::Digest as _;
    let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let id = uuid::Uuid::now_v7();
    let central = uuid::Uuid::now_v7();
    let expires = stub.expires();
    let token = hex(&[key; 32]);
    let digest = hex(&sha2::Sha256::digest(token.as_bytes()));
    let lease = hex(&[key; 32]);
    gw.sql(&format!(
        "INSERT INTO account (id, display_name, created_at, lang, guest, central_only)
             VALUES ('{id}', 'Player{key}', clock_timestamp(), 'en', false, true);
         INSERT INTO central_account_link (account_id, central_id, snapshot_digest, linked_at)
             VALUES ('{id}', '{central}', decode(repeat('00', 32), 'hex'), clock_timestamp());
         INSERT INTO session_token (token_hash, account_id, expires_at)
             VALUES (decode('{digest}', 'hex'), '{id}', to_timestamp({expires}));
         INSERT INTO central_gateway_session (token_hash, lease, expires_at, purpose)
             VALUES (decode('{digest}', 'hex'), decode('{lease}', 'hex'),
                 to_timestamp({expires}), 'admit');"
    ));
    stub.knows(key, central);
    token
}
