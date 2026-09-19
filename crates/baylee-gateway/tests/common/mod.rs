//! Scaffolding shared by the gateway's end-to-end tests: a real gateway
//! process, a blocking HTTP client, and an agent that runs real engines.
//!
//! The gateway hosts no games itself — it routes between seat sockets and an
//! engine process an agent started. So a test that wants a game has to supply
//! the other half of that circle. It does it in-process rather than by
//! spawning binaries: the frames on both sockets are the real ones, the engine
//! is the real `EngineRunner`, and `cargo test -p baylee-gateway` does not
//! quietly depend on which other crates happen to have been built.

#![allow(dead_code)] // each test file uses its own slice of this

use baylee_engine_server::EngineRunner;
use baylee_protocol::v1::{self, Envelope};
use futures_util::{SinkExt, StreamExt};
use prost::Message as _;

/// A gateway process, torn down with the test.
pub struct Gateway {
    /// The port it listens on.
    pub port: u16,
    /// The shared secret an agent must present.
    pub agent_token: String,
    child: std::process::Child,
    store_path: std::path::PathBuf,
    schema: String,
}

impl Drop for Gateway {
    fn drop(&mut self) {
        // The process first: a gateway still holding connections into the
        // schema would make the drop wait on it.
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.store_path);
        ddl(&format!(
            "DROP SCHEMA IF EXISTS \"{}\" CASCADE",
            self.schema
        ));
    }
}

/// Where the tests' PostgreSQL is, or a panic that says how to start one.
///
/// A skip would be worse than this. The gateway keeps its accounts in
/// PostgreSQL now, so a suite that quietly passed without one would be
/// reporting that a gateway works when nothing had asked it to do anything.
pub fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| {
            panic!(
                "DATABASE_URL is not set, and the gateway keeps its accounts in PostgreSQL.\n  \
                 docker compose up -d\n  \
                 export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee"
            )
        })
}

/// Run one DDL statement against the test database.
///
/// On its own thread with its own runtime, because this is called from
/// `Drop` — which cannot await — and from tests that are already inside a
/// runtime, where building a second one in place would panic.
fn ddl(sql: &str) {
    let url = database_url();
    let sql = sql.to_owned();
    let done = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime for one statement")
            .block_on(async move {
                use sea_orm::ConnectionTrait as _;
                let db = sea_orm::Database::connect(&url)
                    .await
                    .expect("connecting to the test database");
                db.execute_unprepared(&sql).await.map(|_| ())
            })
    })
    .join();
    if let Ok(Err(e)) = done {
        eprintln!("test schema statement failed: {e}");
    }
}

/// Starts a gateway on a free port with an empty store.
///
/// `label` keeps two tests running at once from sharing a store file.
pub fn spawn_gateway(label: &str) -> Gateway {
    spawn_gateway_with(label, &[])
}

/// The same, with extra environment for the gateway process.
///
/// Used by the confirmation tests, which need a gateway that has a mailer:
/// whether an address must be confirmed is not a switch of its own, it is
/// whether `BAYLEE_SMTP_URL` is set.
#[allow(dead_code)] // not every test binary in this directory needs it
pub fn spawn_gateway_with(label: &str, env: &[(&str, String)]) -> Gateway {
    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("a free port");
    let port = probe.local_addr().expect("bound").port();
    drop(probe);
    let store_path = std::env::temp_dir().join(format!("baylee-gateway-{label}-{port}.json"));
    let _ = std::fs::remove_file(&store_path);
    let agent_token = format!("test-agent-secret-{port}");
    // A schema per gateway, so three dozen of these run against one server
    // without seeing each other's accounts — and so none of them can touch
    // the catalog's tables in `public`, which on a developer's machine hold
    // half a gigabyte that took three minutes to ingest. The name carries
    // the test's, so a schema left behind by a crash says which one left it.
    let schema = format!("t_{label}_{port}").replace(|c: char| !c.is_ascii_alphanumeric(), "_");
    ddl(&format!("DROP SCHEMA IF EXISTS \"{schema}\" CASCADE"));
    ddl(&format!("CREATE SCHEMA \"{schema}\""));
    let base = database_url();
    let sep = if base.contains('?') { '&' } else { '?' };
    let scoped = format!("{base}{sep}options=-c%20search_path%3D{schema},public");
    // Beside the store and named the same way, so a failure that outlives the
    // run leaves both halves of the evidence in one place.
    let stderr_path = std::env::temp_dir().join(format!("baylee-gateway-{label}-{port}.stderr"));
    let loud = std::env::var("GATEWAY_DEBUG").is_ok();
    let child = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-gateway"))
        // The gateway reads `data/acceptance-decks.txt` for the house deck by
        // a workspace-relative path; a test binary's working directory is its
        // own crate, which is not where that file is.
        .current_dir(workspace_root())
        .env("PORT", port.to_string())
        .env("STORE_PATH", &store_path)
        .env("BAYLEE_AGENT_TOKEN", &agent_token)
        .env("DATABASE_URL", &scoped)
        // Two, not the default eight. What binds is the other end: a stock
        // PostgreSQL allows a hundred connections in total, and this suite
        // has three dozen gateways alive at once. At eight apiece it would
        // ask for nearly three times what the server has, and fail in
        // whichever test happened to be last — which is the worst way for a
        // suite to fail, because it is a different test every run.
        .env("BAYLEE_DB_POOL", "2")
        // No card-art mirror. Starting a game warms every printing at the
        // table, and these tests start a lot of games: left on, the suite
        // fetches a few hundred images from Scryfall and writes them into the
        // working directory — which it did, once, before this line existed.
        // A test that reaches the network is a test that fails on a train.
        .env("BAYLEE_ART_PATH", "off")
        // Nor an image directory. Same rule as the line above: a new
        // env switch with an "on" default is live in this whole suite
        // without any test mentioning it, and a suite that writes
        // files into the working directory leaves a mess.
        .env("BAYLEE_DECK_IMAGE_PATH", "off")
        .env("RUST_LOG", if loud { "info" } else { "off" })
        .envs(env.iter().map(|(k, v)| (*k, v.as_str())))
        .stdout(std::process::Stdio::null())
        // Kept, not discarded. A gateway that fails on the way up says why on
        // its stderr, and throwing that away leaves every test in the crate
        // failing with "connection refused" from the *first* request — which
        // names the symptom and not one thing about the cause. It cost a
        // debugging session to a router the process never got past building.
        .stderr(if loud {
            std::process::Stdio::inherit()
        } else {
            std::fs::File::create(&stderr_path)
                .map_or_else(|_| std::process::Stdio::null(), Into::into)
        })
        .spawn()
        .expect("spawn gateway");
    // Thirty seconds, and the number is a measurement rather than a margin.
    // This loop waited five, which was enough alone and not enough beside
    // anything else: a full gate running in a neighbouring worktree on
    // 18.09.2026 took three e2e tests down at load ~4, purely because the
    // gateway needed longer than five seconds to come up. Five trees share
    // one CPU here, so the budget has to cover a gateway starting while
    // another tree is compiling. Waiting costs nothing when the server is
    // already up — the loop exits on the first answer.
    //
    // `GET /health` and not `TcpStream::connect`, which is the same change
    // the dev tooling wants: an open port says `bind` succeeded and nothing
    // else. It happens to be a sound readiness signal here by accident of
    // ordering — `main` binds last, after the database and the catalog — and
    // an accident of ordering is exactly what stops being true the day
    // somebody binds earlier to shorten startup. Asking the route that
    // answers the question costs one round trip and cannot rot that way.
    let mut up = false;
    for _ in 0..300 {
        if let Some((200, _)) = try_http(port, "GET", "/health") {
            up = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(
        up,
        "the gateway never answered GET /health on port {port}. Its own words:\n{}",
        std::fs::read_to_string(&stderr_path)
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "(nothing on stderr — run with GATEWAY_DEBUG=1)".to_string())
    );
    Gateway {
        port,
        agent_token,
        child,
        store_path,
        schema,
    }
}

/// One unauthenticated request that is allowed to fail.
///
/// [`http`] panics on every step, which is right for a test making a request
/// of a server it has already been told is up, and wrong for the one caller
/// that is asking *whether* it is up: there, a refused connection is the
/// expected answer for the first second or so. `None` means "not yet", and
/// only the poll loop in [`spawn_gateway_with`] has any business seeing it.
fn try_http(port: u16, method: &str, path: &str) -> Option<(u16, String)> {
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).ok()?;
    let request =
        format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).ok()?;
    let mut raw = String::new();
    stream.read_to_string(&mut raw).ok()?;
    let status: u16 = raw.split_whitespace().nth(1)?.parse().ok()?;
    Some((
        status,
        raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string(),
    ))
}

/// Minimal blocking HTTP/1.1 client. The gateway is a separate process;
/// nothing else needs this test's runtime thread.
pub fn http(port: u16, method: &str, path: &str, token: Option<&str>, body: &str) -> (u16, String) {
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect http");
    let auth = token.map_or(String::new(), |t| format!("Authorization: Bearer {t}\r\n"));
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n{auth}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).expect("write http");
    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read http");
    let status: u16 = raw
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("http status");
    let body = raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, body)
}

/// The value of a **numeric** field in a JSON response body.
///
/// Its own function rather than a looser `json_field`, because the string
/// one looks for `"field":"` and a number has no opening quote: asked for a
/// count it does not find the marker at all and panics inside the harness,
/// which reads as a broken test rather than as the wrong accessor.
pub fn json_number(body: &str, field: &str) -> i64 {
    let marker = format!("\"{field}\":");
    let start = body.find(&marker).expect("field present") + marker.len();
    let rest = &body[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '-')
        .unwrap_or(rest.len());
    rest[..end].parse().expect("a number")
}

/// The value of a string field in a JSON response body.
pub fn json_field<'a>(body: &'a str, field: &str) -> &'a str {
    let marker = format!("\"{field}\":\"");
    let start = body.find(&marker).expect("field present") + marker.len();
    let rest = &body[start..];
    let end = rest.find('"').expect("field ends");
    &rest[..end]
}

/// Registers an account and logs in, returning the bearer token.
pub fn login(port: u16, email: &str, name: &str) -> String {
    let register = format!(
        "{{\"email\":\"{email}\",\"display_name\":\"{name}\",\"password\":\"a-very-fine-password\"}}"
    );
    let (status, body) = http(port, "POST", "/auth/register", None, &register);
    assert_eq!(status, 200, "register: {body}");
    let creds = format!("{{\"email\":\"{email}\",\"password\":\"a-very-fine-password\"}}");
    let (status, body) = http(port, "POST", "/auth/login", None, &creds);
    assert_eq!(status, 200, "login: {body}");
    json_field(&body, "token").to_string()
}

/// A live websocket to something on the gateway.
pub type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// How long anything in this suite waits for something that is merely slow.
///
/// One constant rather than a number per loop, because every one of those
/// numbers was chosen against an idle machine and they were all wrong in the
/// same way. **A test that assumes an idle box is asserting something about
/// the machine rather than about the code**: five worktrees share one CPU
/// here, and on 18.09.2026 a full gate in a neighbouring tree took three e2e
/// tests down at load ~4 — not because anything was broken, but because a
/// five-second budget does not cover a server starting beside a compile.
///
/// Thirty seconds is not a margin for a hang. Nothing here waits out the
/// budget on the happy path: every loop below exits on its first success, so
/// a raised ceiling costs a passing run nothing at all and only changes which
/// failures are real. A genuine hang still fails, thirty seconds later, with
/// the same message.
pub const WAIT_BUDGET: std::time::Duration = std::time::Duration::from_secs(30);

/// How many 100 ms attempts fit in [`WAIT_BUDGET`].
pub const WAIT_TRIES: u32 = 300;

/// One poll interval, so a loop states its budget instead of its arithmetic.
pub const WAIT_STEP: std::time::Duration = std::time::Duration::from_millis(100);

/// Connects an agent and waits until the gateway has registered it.
///
/// Returning only after the welcome matters: a game created before any agent
/// is registered is refused, which is correct and is not what a test about
/// something else wants to discover.
pub async fn attach_agent(gateway: &Gateway) -> tokio::task::JoinHandle<()> {
    attach_agent_watching(gateway).await.0
}

/// The same agent, plus every `GamePreset` the gateway hands an engine.
///
/// The preset is the one thing in this circle that a test cannot otherwise
/// see: it travels gateway → engine as JSON on a socket no player holds, so
/// a test asking "did the room's choice actually reach the rules" has to
/// stand where the engine stands. Dropping the receiver is fine — the send
/// is unbounded and its error is ignored — which is why [`attach_agent`] can
/// be this function with the channel thrown away.
pub async fn attach_agent_watching(
    gateway: &Gateway,
) -> (
    tokio::task::JoinHandle<()>,
    tokio::sync::mpsc::UnboundedReceiver<baylee_core::preset::GamePreset>,
) {
    let (presets, seen) = tokio::sync::mpsc::unbounded_channel();
    (attach_agent_inner(gateway, presets).await, seen)
}

async fn attach_agent_inner(
    gateway: &Gateway,
    presets: tokio::sync::mpsc::UnboundedSender<baylee_core::preset::GamePreset>,
) -> tokio::task::JoinHandle<()> {
    let url = format!("ws://127.0.0.1:{}/agent/ws", gateway.port);
    let mut ws = dial(&url).await.expect("agent socket");
    send(
        &mut ws,
        &Envelope {
            msg: Some(v1::envelope::Msg::AgentHello(v1::AgentHello {
                token: gateway.agent_token.clone(),
                name: "test-agent".to_string(),
                capacity: 0,
            })),
        },
    )
    .await;
    let welcome = next_msg(&mut ws).await;
    assert!(
        matches!(welcome, Some(v1::envelope::Msg::AgentWelcome(_))),
        "the gateway did not welcome the agent: {welcome:?}"
    );
    tokio::spawn(async move {
        while let Some(msg) = next_msg(&mut ws).await {
            if let v1::envelope::Msg::StartEngine(start) = msg {
                tokio::spawn(run_engine(start, presets.clone()));
            }
        }
    })
}

/// One engine, for one game, over the real engine link.
///
/// No decision clock: a test that wants a seat to run out of time can say so
/// itself, and a clock running under every other test would only add a way for
/// them to fail on a slow machine.
async fn run_engine(
    start: v1::StartEngine,
    presets: tokio::sync::mpsc::UnboundedSender<baylee_core::preset::GamePreset>,
) {
    let Some(mut ws) = dial(&start.gateway_url).await else {
        return;
    };
    send(
        &mut ws,
        &Envelope {
            msg: Some(v1::envelope::Msg::EngineHello(v1::EngineHello {
                game_id: start.game_id.clone(),
                token: start.engine_token.clone(),
            })),
        },
    )
    .await;
    let mut runner = EngineRunner::new();
    while let Some(msg) = next_msg(&mut ws).await {
        if let v1::envelope::Msg::GameSetup(setup) = &msg
            && let Ok(preset) = serde_json::from_slice(&setup.preset_json)
        {
            let _ = presets.send(preset);
        }
        // This harness runs no timer, so it never has a deadline armed and
        // has nothing to read off one. Every question therefore reaches a
        // seat with its allowance whole, which is what an untimed in-process
        // runner should show.
        for out in runner.handle(Envelope { msg: Some(msg) }, None) {
            send(&mut ws, &out).await;
        }
        if runner.finished() {
            break;
        }
    }
}

/// Dials a websocket, retrying while the listener comes up.
pub async fn dial(url: &str) -> Option<Socket> {
    for _ in 0..WAIT_TRIES {
        if let Ok((socket, _)) = tokio_tungstenite::connect_async(url).await {
            return Some(socket);
        }
        tokio::time::sleep(WAIT_STEP).await;
    }
    None
}

/// The same, for a seat socket that is refused until the engine is behind it.
///
/// Three test files had written this loop out by hand with three different
/// spellings and the same five-second budget. One of them is the reason it is
/// here: a seat socket is opened *while* the engine is still being started by
/// an agent, so this waits on two processes rather than one, and it is the
/// loop with the least reason of any of them to assume an idle machine.
pub async fn dial_seat(url: &str) -> Socket {
    dial(url)
        .await
        .unwrap_or_else(|| panic!("the seat socket never opened within {WAIT_BUDGET:?}: {url}"))
}

async fn send(ws: &mut Socket, envelope: &Envelope) {
    let _ = ws
        .send(tokio_tungstenite::tungstenite::Message::Binary(
            envelope.encode_to_vec().into(),
        ))
        .await;
}

/// The next protocol message on a socket, or `None` when it closes.
async fn next_msg(ws: &mut Socket) -> Option<v1::envelope::Msg> {
    loop {
        let frame = ws.next().await?.ok()?;
        if !frame.is_binary() {
            continue;
        }
        if let Ok(envelope) = Envelope::decode(frame.into_data())
            && let Some(msg) = envelope.msg
        {
            return Some(msg);
        }
    }
}

/// The workspace root, two levels above this crate.
fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root is above this crate")
}

/// The same client for a body that is not text, and an answer that is not
/// either.
///
/// A picture cannot go through [`http`]: a JPEG is not UTF-8 in either
/// direction, and `read_to_string` on the reply fails before the status can
/// be read. The header block still is text, so it is split off as bytes and
/// only then read as one.
#[allow(dead_code)] // only the cosmetics tests upload anything
pub fn http_bytes(
    port: u16,
    method: &str,
    path: &str,
    token: Option<&str>,
    content_type: &str,
    body: &[u8],
) -> (u16, Vec<u8>) {
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect http");
    let auth = token.map_or(String::new(), |t| format!("Authorization: Bearer {t}\r\n"));
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: {content_type}\r\n{auth}Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).expect("write head");
    stream.write_all(body).expect("write body");
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).expect("read http");
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("a header block");
    let header = String::from_utf8_lossy(&raw[..split]).to_string();
    let status: u16 = header
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("http status");
    (status, raw[split + 4..].to_vec())
}

/// A request that also says what came back in the **header block**.
///
/// [`http`] throws the headers away, which is right for every test that is
/// about a body and useless for one that is about a header — CORS is decided
/// entirely in headers, and a preflight has no body at all. `extra` is sent
/// verbatim, each pair on its own line, because a preflight is defined by
/// `Origin` and `Access-Control-Request-Method` being present.
pub fn http_headers(port: u16, method: &str, path: &str, extra: &[(&str, &str)]) -> (u16, String) {
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect http");
    let mut extras = String::new();
    for (name, value) in extra {
        extras.push_str(name);
        extras.push_str(": ");
        extras.push_str(value);
        extras.push_str("\r\n");
    }
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n{extras}Content-Length: 0\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).expect("write http");
    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read http");
    let status: u16 = raw
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("http status");
    let head = raw.split("\r\n\r\n").next().unwrap_or("").to_lowercase();
    (status, head)
}
