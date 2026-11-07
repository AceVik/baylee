//! A stub Baylee Central on loopback, and a gateway bridged to it: for what a
//! gateway does with Central's answers, and with having none. It signs what
//! Central signs and answers the few requests a gateway makes of its own
//! accord; nothing here reaches a real service.

#![allow(dead_code)] // each test binary uses its own part of this

use super::common;
use baylee_account::wire::{Request, Response};
use baylee_account::{Endpoint, SignedToken, Token};
use central_crypto::jwt::{GatewayClaims, KeySet, Purpose, RevocationClaims, SigningKey};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

/// How long a signed revocation lives.
const MESSAGE_SECS: i64 = 60;

/// What the stub answers every request with.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Says {
    /// What was asked.
    Good,
    /// `401`: Central does not take the bridge's credential.
    Refused,
    /// `503`: Central is there and cannot answer.
    Down,
    /// `200` with a page that is no answer at all, as a proxy in the way sends.
    Garbage,
}

/// What the stub holds; every field may change while it serves.
pub struct Shared {
    /// Central's origin, which is the issuer of all it signs.
    origin: String,
    /// What it signs with.
    key: SigningKey,
    /// What it answers.
    says: Mutex<Says>,
    /// The account each first byte names.
    accounts: Mutex<HashMap<u8, uuid::Uuid>>,
    /// The revoked sessions in the order they ended: number one is first.
    revoked: Mutex<Vec<String>>,
    /// The gateway its messages are for.
    gateway: Mutex<String>,
    /// How long every answer takes, in milliseconds.
    delay: AtomicU64,
    /// How many answers it gave.
    answered: AtomicU64,
}

/// The stub, serving.
#[derive(Clone)]
pub struct Stub {
    pub port: u16,
    pub shared: Arc<Shared>,
}

fn now() -> i64 {
    let since = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after 1970");
    i64::try_from(since.as_secs()).expect("fits")
}

/// 32 lowercase hex digits nobody used yet.
fn fresh_id() -> String {
    uuid::Uuid::now_v7().simple().to_string()
}

/// The session `key` stands for, in Central's form.
fn sid_of(key: u8) -> String {
    format!("{key:02x}").repeat(16)
}

/// Where a gateway on `port` is known to Central.
fn origin_of(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

impl Shared {
    /// The revocations `from` to `seq`, signed for the gateway.
    fn message(&self, from: u64, seq: u64) -> Zeroizing<String> {
        let revoked = self.revoked.lock().expect("the list");
        let first = usize::try_from(from - 1).expect("fits");
        let last = usize::try_from(seq).expect("fits");
        let now = now();
        let claims = RevocationClaims {
            iss: self.origin.clone(),
            aud: self.gateway.lock().expect("the name").clone(),
            jti: fresh_id(),
            iat: now,
            nbf: now,
            exp: now + MESSAGE_SECS,
            from,
            seq,
            sids: revoked[first..last].to_vec(),
        };
        self.key.sign(&claims).expect("signed")
    }

    /// The last number given.
    fn last(&self) -> u64 {
        let revoked = self.revoked.lock().expect("the list");
        u64::try_from(revoked.len()).expect("fits")
    }
}

impl Stub {
    /// Starts it, every answer taking `delay`.
    #[must_use]
    pub fn serve(delay: Duration) -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
        let port = listener.local_addr().expect("an address").port();
        let seed = SigningKey::generate_seed().expect("a seed");
        let shared = Arc::new(Shared {
            origin: origin_of(port),
            key: SigningKey::from_seed(&seed),
            says: Mutex::new(Says::Good),
            accounts: Mutex::default(),
            revoked: Mutex::default(),
            gateway: Mutex::default(),
            delay: AtomicU64::new(u64::try_from(delay.as_millis()).expect("fits")),
            answered: AtomicU64::new(0),
        });
        let serving = Arc::clone(&shared);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let shared = Arc::clone(&serving);
                std::thread::spawn(move || keep_answering(stream, &shared));
            }
        });
        Self { port, shared }
    }

    /// From now on it says `says` to everything.
    pub fn say(&self, says: Says) {
        *self.shared.says.lock().expect("the answer") = says;
    }

    /// How many answers it has given.
    #[must_use]
    pub fn answered(&self) -> u64 {
        self.shared.answered.load(Ordering::Relaxed)
    }

    /// Waits for an answer after the first `seen`.
    pub fn answers_after(&self, seen: u64) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while self.answered() <= seen {
            assert!(Instant::now() < deadline, "the gateway never asked Central");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// `key` names a new account.
    pub fn knows(&self, key: u8) {
        self.shared
            .accounts
            .lock()
            .expect("the map")
            .insert(key, uuid::Uuid::now_v7());
    }

    /// A gateway token for session `key` at the gateway on `port`.
    #[must_use]
    pub fn token(&self, port: u16, key: u8) -> Zeroizing<String> {
        self.shared
            .key
            .sign(&self.claims(port, key))
            .expect("signed")
    }

    /// What a token for session `key` says.
    fn claims(&self, port: u16, key: u8) -> GatewayClaims {
        let account = *self
            .shared
            .accounts
            .lock()
            .expect("the map")
            .get(&key)
            .expect("an account the stub knows");
        let now = now();
        GatewayClaims {
            iss: self.shared.origin.clone(),
            aud: origin_of(port),
            sub: account.to_string(),
            sid: sid_of(key),
            jti: fresh_id(),
            iat: now,
            nbf: now,
            exp: now + 3600,
            purpose: Purpose::Admit,
            name: None,
            tag: None,
            legacy_id: None,
            terms: None,
        }
    }

    /// `POST /auth/central/login` for session `key`, whose move from the
    /// legacy account `legacy` closed: the one sign-in that fixes a link.
    #[must_use]
    pub fn sign_in_moved(&self, port: u16, key: u8, legacy: &str) -> (u16, String) {
        let mut claims = self.claims(port, key);
        claims.legacy_id = Some(legacy.to_owned());
        let token = self.shared.key.sign(&claims).expect("signed");
        let body = serde_json::json!({ "token": token.as_str() }).to_string();
        common::http(port, "POST", "/auth/central/login", None, &body)
    }

    /// `POST /auth/central/login` with a fresh token for session `key`.
    #[must_use]
    pub fn sign_in(&self, port: u16, key: u8) -> (u16, String) {
        let token = self.token(port, key);
        let body = serde_json::json!({ "token": token.as_str() }).to_string();
        common::http(port, "POST", "/auth/central/login", None, &body)
    }

    /// Session `key` ends at Central, and gets the next number. The gateway
    /// hears of it when it asks, or when it is pushed.
    pub fn revoke(&self, key: u8) {
        self.shared
            .revoked
            .lock()
            .expect("the list")
            .push(sid_of(key));
    }

    /// Sends the last number alone to the gateway on `port`, as a push does.
    pub fn push(&self, port: u16) {
        let last = self.shared.last();
        let message = self.shared.message(last, last);
        let (status, _) = common::http_bytes(
            port,
            "POST",
            "/auth/central/invalidate",
            None,
            "application/jwt",
            message.as_bytes(),
        );
        assert_eq!(status, 204, "the gateway refused Central's message");
    }

    /// Makes the gateway on `port` ask, now rather than at its next round: it
    /// is pushed a number with one it never saw before it, for sessions no
    /// test holds, and a gap is what a gateway does not wait with.
    pub fn nudge(&self, port: u16) {
        const NOBODY: u8 = 0xff;
        self.revoke(NOBODY);
        self.revoke(NOBODY);
        self.push(port);
    }
}

/// One request's first line and body, once its headers and `Content-Length`
/// are in.
fn request_of(stream: &mut std::net::TcpStream) -> Option<(String, Vec<u8>)> {
    let mut raw = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let read = stream.read(&mut chunk).ok().filter(|n| *n > 0)?;
        raw.extend_from_slice(&chunk[..read]);
        let Some(end) = raw.windows(4).position(|w| w == b"\r\n\r\n") else {
            continue;
        };
        let head = String::from_utf8_lossy(&raw[..end]).into_owned();
        let length: usize = head
            .lines()
            .find_map(|line| {
                line.to_lowercase()
                    .strip_prefix("content-length: ")?
                    .parse()
                    .ok()
            })
            .unwrap_or(0);
        while raw.len() < end + 4 + length {
            let read = stream.read(&mut chunk).ok().filter(|n| *n > 0)?;
            raw.extend_from_slice(&chunk[..read]);
        }
        let line = head.lines().next().unwrap_or_default().to_owned();
        return Some((line, raw[end + 4..end + 4 + length].to_vec()));
    }
}

/// An answer: its status, its content type and its body.
type Answer = (&'static str, &'static str, Vec<u8>);

/// What one request is answered while the stub says what was asked.
fn good_answer(line: &str, body: &[u8], shared: &Shared) -> Answer {
    let json = |response: &Response| {
        let bytes = central_api::encode(response).expect("encoded");
        ("200 OK", "application/json", bytes)
    };
    if line.starts_with("GET /.well-known/jwks.json") {
        let keys = KeySet::new(vec![shared.key.public().clone()]).expect("a key set");
        let document = keys.to_jwks().expect("a document");
        return ("200 OK", "application/jwk-set+json", document);
    }
    match central_api::decode::<Request>(body) {
        Ok(Request::RegisterInvalidate { .. }) => json(&Response::Done),
        Ok(Request::GatewayRevocations { after }) => {
            let last = shared.last();
            let token = (after < last).then(|| {
                SignedToken::new(shared.message(after + 1, last)).expect("a token of a fair size")
            });
            json(&Response::GatewayRevocations {
                token,
                last,
                floor: 1,
            })
        }
        _ => ("503 Service Unavailable", "text/plain", Vec::new()),
    }
}

/// What one request is answered.
fn answer_to(line: &str, body: &[u8], shared: &Shared) -> Answer {
    match *shared.says.lock().expect("the answer") {
        Says::Good => good_answer(line, body, shared),
        Says::Refused => ("401 Unauthorized", "text/plain", Vec::new()),
        Says::Down => ("503 Service Unavailable", "text/plain", Vec::new()),
        Says::Garbage => ("200 OK", "text/html", b"<html>a proxy</html>".to_vec()),
    }
}

/// Answers `stream` until it closes, keeping it open as Central's server does.
fn keep_answering(mut stream: std::net::TcpStream, shared: &Shared) {
    while let Some((line, body)) = request_of(&mut stream) {
        let (status, kind, reply) = answer_to(&line, &body, shared);
        std::thread::sleep(Duration::from_millis(shared.delay.load(Ordering::Relaxed)));
        shared.answered.fetch_add(1, Ordering::Relaxed);
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\n\r\n",
            reply.len()
        );
        if stream.write_all(head.as_bytes()).is_err() || stream.write_all(&reply).is_err() {
            return;
        }
    }
}

/// A gateway whose bridge is `stub`, and the file its settings are in, which
/// may not have heard from Central yet.
#[must_use]
pub fn gateway_starting(stub: &Stub, label: &str) -> (common::Gateway, Settings) {
    gateway_starting_with(stub, label, &[])
}

/// [`gateway_starting`] with more of the gateway's environment.
#[must_use]
pub fn gateway_starting_with(
    stub: &Stub,
    label: &str,
    env: &[(&str, String)],
) -> (common::Gateway, Settings) {
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
    let mut env = env.to_vec();
    env.push((
        "BAYLEE_CENTRAL_CONFIG_FILE",
        file.to_string_lossy().into_owned(),
    ));
    let gw = common::spawn_gateway_with(label, &env);
    *stub.shared.gateway.lock().expect("the name") = origin_of(gw.port);
    (gw, Settings(file))
}

/// [`gateway_starting`], once it has caught up with Central.
#[must_use]
pub fn gateway(stub: &Stub, label: &str) -> (common::Gateway, Settings) {
    gateway_with(stub, label, &[])
}

/// [`gateway`] with more of the gateway's environment.
#[must_use]
pub fn gateway_with(
    stub: &Stub,
    label: &str,
    env: &[(&str, String)],
) -> (common::Gateway, Settings) {
    let (gw, settings) = gateway_starting_with(stub, label, env);
    common::central_ready(gw.port);
    (gw, settings)
}

/// The gateway's settings file, gone with the test.
pub struct Settings(std::path::PathBuf);

impl Drop for Settings {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// An account of Central's signed in as session `key`. Answers its bearer.
pub fn linked(gw: &common::Gateway, stub: &Stub, key: u8) -> String {
    stub.knows(key);
    let (status, answer) = stub.sign_in(gw.port, key);
    assert_eq!(status, 200, "{answer}");
    common::json_field(&answer, "token").to_owned()
}
