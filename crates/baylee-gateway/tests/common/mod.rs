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

pub mod central;

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
    port_file: std::path::PathBuf,
    schema: String,
    /// Where the process's log lines go (`tracing` writes to stdout), for
    /// the tests that read what the gateway said.
    log_path: std::path::PathBuf,
}

impl Gateway {
    /// Everything the gateway has logged so far. Empty unless the test set
    /// `RUST_LOG`: the harness starts every gateway with it off.
    pub fn logs(&self) -> String {
        std::fs::read_to_string(&self.log_path).unwrap_or_default()
    }

    /// The gateway's process id, for the load bench's `ps` readings.
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// The test database, scoped to this gateway's own schema: where a test
    /// seeds what the gateway will read, such as catalog rows.
    pub fn database_url(&self) -> String {
        let base = database_url();
        let sep = if base.contains('?') { '&' } else { '?' };
        format!(
            "{base}{sep}options=-c%20search_path%3D{},public",
            self.schema
        )
    }

    /// Runs one statement in this gateway's schema, for a row no route
    /// writes any more, such as an account with an address (#269). Panics
    /// when it fails: a fixture that was not written makes every assertion
    /// after it mean something else.
    pub fn sql(&self, statement: &str) {
        sql_in(&self.schema, statement);
    }

    /// The one number a query in this gateway's schema answers, for what no
    /// route says, such as whether a guest's decks went with it (#269).
    /// Panics when there is no such number, for the reason [`Self::sql`]
    /// does.
    pub fn scalar(&self, query: &str) -> i64 {
        let url = self.database_url();
        let query = query.to_owned();
        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("a runtime for one query")
                .block_on(async move {
                    use sea_orm::ConnectionTrait as _;
                    let db = sea_orm::Database::connect(&url)
                        .await
                        .expect("connecting to the gateway's schema");
                    db.query_one_raw(sea_orm::Statement::from_string(
                        sea_orm::DbBackend::Postgres,
                        query.clone(),
                    ))
                    .await
                    .unwrap_or_else(|e| panic!("{query}: {e}"))
                    .unwrap_or_else(|| panic!("{query}: no row"))
                    .try_get_by_index::<i64>(0)
                    .unwrap_or_else(|e| panic!("{query}: {e}"))
                })
        })
        .join()
        .expect("the query's thread")
    }

    /// The one text a query in this gateway's schema answers, such as an
    /// account's id, which no route says. Panics as [`Self::scalar`] does.
    #[allow(dead_code)] // not every test binary in this directory needs it
    pub fn text(&self, query: &str) -> String {
        let url = self.database_url();
        let query = query.to_owned();
        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("a runtime for one query")
                .block_on(async move {
                    use sea_orm::ConnectionTrait as _;
                    let db = sea_orm::Database::connect(&url)
                        .await
                        .expect("connecting to the gateway's schema");
                    db.query_one_raw(sea_orm::Statement::from_string(
                        sea_orm::DbBackend::Postgres,
                        query.clone(),
                    ))
                    .await
                    .unwrap_or_else(|e| panic!("{query}: {e}"))
                    .unwrap_or_else(|| panic!("{query}: no row"))
                    .try_get_by_index::<String>(0)
                    .unwrap_or_else(|e| panic!("{query}: {e}"))
                })
        })
        .join()
        .expect("the query's thread")
    }
}

/// Runs `statement` in the gateway schema `schema`. Panics when it fails, as
/// [`Gateway::sql`] says.
fn sql_in(schema: &str, statement: &str) {
    let base = database_url();
    let sep = if base.contains('?') { '&' } else { '?' };
    let url = format!("{base}{sep}options=-c%20search_path%3D{schema},public");
    let statement = statement.to_owned();
    let done = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime for one statement")
            .block_on(async move {
                use sea_orm::ConnectionTrait as _;
                let db = sea_orm::Database::connect(&url)
                    .await
                    .expect("connecting to the gateway's schema");
                db.execute_unprepared(&statement).await.map(|_| ())
            })
    })
    .join()
    .expect("the statement's thread");
    if let Err(e) = done {
        panic!("{e}");
    }
}

/// Each live gateway's schema by its port, for [`login`], which is handed a
/// port.
static SCHEMAS: std::sync::Mutex<std::collections::BTreeMap<u16, String>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

impl Drop for Gateway {
    fn drop(&mut self) {
        if let Ok(mut schemas) = SCHEMAS.lock() {
            schemas.remove(&self.port);
        }
        // The process first: a gateway still holding connections into the
        // schema would make the drop wait on it.
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.store_path);
        let _ = std::fs::remove_file(&self.port_file);
        let _ = std::fs::remove_file(&self.log_path);
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

/// The same, with extra environment for the gateway process: a console's
/// token, a closed beta, a Baylee Central bridge, and the like.
#[allow(dead_code)] // not every test binary in this directory needs it
#[allow(clippy::too_many_lines)] // one process started and waited for, top to bottom
pub fn spawn_gateway_with(label: &str, env: &[(&str, String)]) -> Gateway {
    // No port is chosen here (#279). This used to bind `127.0.0.1:0`, read
    // the number and let go, and the gateway bound `0.0.0.0` on it seconds
    // later, after its database was up: any other process could take the
    // port in between, and with several worktrees gating at once one did,
    // two feature gates in three. The gateway now binds port 0 itself and
    // writes the port it got to `BAYLEE_PORT_FILE`, read below.
    //
    // So everything that used to be named after the port is named after this
    // process and a count of the gateways it has started, which is unique
    // across concurrent test binaries without asking the kernel anything.
    static STARTED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let id = format!(
        "{}_{}",
        std::process::id(),
        STARTED.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let store_path = std::env::temp_dir().join(format!("baylee-gateway-{label}-{id}.json"));
    let _ = std::fs::remove_file(&store_path);
    let port_file = std::env::temp_dir().join(format!("baylee-gateway-{label}-{id}.port"));
    let _ = std::fs::remove_file(&port_file);
    let agent_token = format!("test-agent-secret-{id}");
    // A schema per gateway, so three dozen of these run against one server
    // without seeing each other's accounts — and so none of them can touch
    // the catalog's tables in `public`, which on a developer's machine hold
    // half a gigabyte that took three minutes to ingest. The name carries
    // the test's, so a schema left behind by a crash says which one left it.
    let schema = format!("t_{label}_{id}").replace(|c: char| !c.is_ascii_alphanumeric(), "_");
    ddl(&format!("DROP SCHEMA IF EXISTS \"{schema}\" CASCADE"));
    ddl(&format!("CREATE SCHEMA \"{schema}\""));
    let base = database_url();
    let sep = if base.contains('?') { '&' } else { '?' };
    let scoped = format!("{base}{sep}options=-c%20search_path%3D{schema},public");
    // Beside the store and named the same way, so a failure that outlives the
    // run leaves both halves of the evidence in one place.
    let stderr_path = std::env::temp_dir().join(format!("baylee-gateway-{label}-{id}.stderr"));
    let log_path = std::env::temp_dir().join(format!("baylee-gateway-{label}-{id}.log"));
    let cwd = std::env::temp_dir().join(format!("baylee-gateway-{label}-{id}.cwd"));
    let _ = std::fs::remove_dir_all(&cwd);
    std::fs::create_dir_all(&cwd).expect("the gateway's working directory");
    let loud = std::env::var("GATEWAY_DEBUG").is_ok();
    let child = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-gateway"))
        // Started from an empty directory of its own: a gateway reads nothing
        // from where it was started (the house deck once came from a
        // cwd-relative `data/`, and every gateway started elsewhere answered
        // an AI seat with a 500), and what it would write there lands here
        // rather than in the repository.
        .current_dir(&cwd)
        .env("PORT", "0")
        .env("BAYLEE_BIND", "127.0.0.1")
        .env("BAYLEE_PORT_FILE", &port_file)
        .env("STORE_PATH", &store_path)
        .env("BAYLEE_AGENT_TOKEN", &agent_token)
        .env("DATABASE_URL", &scoped)
        // A developer's live bridge must not leak into synthetic tests.
        // Explicit fixture overrides below may still select their own mocks.
        .env_remove("BAYLEE_CENTRAL_CONFIG_FILE")
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
        // Nor terms of use: unset is the default terms every gateway ships,
        // and a Central-bound session that has not accepted them is refused.
        // The terms tests name their own.
        .env("BAYLEE_TERMS_PATH", "off")
        // Both doors: unset is `central`, which on a gateway with no Central
        // lets nobody in, and most of this suite comes in as a guest. The
        // auth mode's own tests name theirs; a blank one is the default.
        .env("BAYLEE_AUTH_MODE", "mixed")
        .env_remove("BAYLEE_GUESTS")
        .env("RUST_LOG", if loud { "info" } else { "off" })
        .envs(env.iter().map(|(k, v)| (*k, v.as_str())))
        .stdout(if loud {
            std::process::Stdio::inherit()
        } else {
            std::fs::File::create(&log_path)
                .map_or_else(|_| std::process::Stdio::null(), Into::into)
        })
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
    // Own the process before polling readiness: any early exit/assertion must
    // run the same cleanup as a completed test, not merely drop a Child handle.
    let mut gateway = Gateway {
        port: 0,
        agent_token,
        child,
        store_path,
        port_file,
        schema,
        log_path,
    };
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
    //
    // The port file comes first: the gateway writes it once its listener is
    // bound, which is after the database and the catalog. A gateway that
    // dies on the way up never writes it, and is noticed here the moment it
    // exits rather than after the whole budget.
    let their_words = || {
        std::fs::read_to_string(&stderr_path)
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "(nothing on stderr — run with GATEWAY_DEBUG=1)".to_string())
    };
    let mut port = None;
    let mut up = false;
    for _ in 0..300 {
        if let Ok(Some(status)) = gateway.child.try_wait() {
            panic!(
                "the gateway exited on the way up ({status}). Its own words:\n{}",
                their_words()
            );
        }
        if port.is_none() {
            port = std::fs::read_to_string(&gateway.port_file)
                .ok()
                .and_then(|text| text.trim().parse::<u16>().ok());
        }
        if let Some(port) = port
            && let Some((200, _)) = try_http(port, "GET", "/health")
        {
            up = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(
        up,
        "the gateway never answered GET /health (port file {}: {port:?}). Its own words:\n{}",
        gateway.port_file.display(),
        their_words()
    );
    gateway.port = port.expect("a gateway that answered has a port");
    SCHEMAS
        .lock()
        .expect("the schemas' lock")
        .insert(gateway.port, gateway.schema.clone());
    gateway
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

/// The official gateway's own Argon2id hash of `synthetic legacy password`,
/// the hash a fixture account keeps, as an account from before the cutover
/// does (`docs/central-login.md` §"Cutover").
pub const LEGACY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$wittOwBy6Zxq/y0DGjZasA$fBlUSay3ZkJb5bDps9DdGXWgriHX9cjNYnp4ZXs80go";

/// An account that signs in with `username` and is seen as `name`, and a
/// session for it: the bearer token.
///
/// Nobody registers or signs in with a password at a gateway any more
/// (`docs/central-login.md` §"Cutover"), so the fixture writes the account
/// and its session as the database holds them: an account from before the
/// cutover with a session, which every route below the sign-in treats as it
/// treats any session. Names are the test's own, letters, digits and
/// `_-.` only.
pub fn login(port: u16, username: &str, name: &str) -> String {
    let plain = |text: &str| {
        text.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
    };
    assert!(
        plain(username) && plain(name),
        "a fixture name: {username} {name}"
    );
    let schema = SCHEMAS
        .lock()
        .expect("the schemas' lock")
        .get(&port)
        .cloned()
        .expect("a gateway this harness started");
    let hex = |bytes: &[u8]| {
        use std::fmt::Write as _;
        bytes.iter().fold(String::new(), |mut out, b| {
            write!(out, "{b:02x}").expect("writing to a String");
            out
        })
    };
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("OS randomness");
    let token = hex(&bytes);
    let digest = {
        use sha2::Digest as _;
        hex(&sha2::Sha256::digest(token.as_bytes()))
    };
    let id = uuid::Uuid::now_v7();
    let key = username.to_ascii_lowercase();
    // Made in whole seconds, as the gateway makes every account it writes
    // (`store::Account::created_at` is unix seconds): a fixture a guest made
    // in the same second would otherwise be newer than the guest, and the
    // console's newest-first order would put them the wrong way round.
    sql_in(
        &schema,
        &format!(
            "INSERT INTO account (id, username, username_key, display_name, password_hash,
                 created_at, lang, guest)
             VALUES ('{id}', '{username}', '{key}', '{name}', '{LEGACY_HASH}',
                 date_trunc('second', clock_timestamp()), 'en', false);
             INSERT INTO session_token (token_hash, account_id, expires_at)
             VALUES (decode('{digest}', 'hex'), '{id}', clock_timestamp() + interval '12 hours');"
        ),
    );
    token
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
    (
        attach_agent_inner(gateway, presets, EngineProbe::new().1, None, 0).await,
        seen,
    )
}

/// The same agent, with every game it runs shuffled from `seed` instead of
/// the seed the gateway drew: the same game on every run, for a test that
/// plays one through and must not wait on a deal it has never seen.
pub async fn attach_agent_seeded(gateway: &Gateway, seed: u64) -> tokio::task::JoinHandle<()> {
    let presets = tokio::sync::mpsc::unbounded_channel().0;
    attach_agent_inner(gateway, presets, EngineProbe::new().1, Some(seed), 0).await
}

/// What a test can see of, and change about, the engines an agent runs:
/// every engine this agent starts reports to one probe, so it is for tests
/// with one game (#323).
pub struct EngineProbe {
    /// Seat frames actually received by the engine, for authorization boundary tests.
    pub seat_frames: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    /// The game's `snapshot_hash` after the last frame its engine handled;
    /// `None` until the game is built.
    pub hash: tokio::sync::watch::Receiver<Option<u64>>,
    /// How many `FlushRecord`s the gateway sent the engine.
    pub flushes: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    /// While set, the engine passes over a `FlushRecord` without a word, as
    /// an engine from before the message does.
    pub mute: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl EngineProbe {
    /// How many `FlushRecord`s the gateway has sent so far.
    pub fn flushes(&self) -> usize {
        self.flushes.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Makes the engine pass over every `FlushRecord` from now on, or answer
    /// them again.
    pub fn set_mute(&self, mute: bool) {
        self.mute.store(mute, std::sync::atomic::Ordering::SeqCst);
    }
}

/// The engine's half of an [`EngineProbe`].
#[derive(Clone)]
struct ProbeEnd {
    seat_frames: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    hash: std::sync::Arc<tokio::sync::watch::Sender<Option<u64>>>,
    flushes: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    mute: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Whether the engine runs its clocks, as `baylee-engine-server` does.
    clocked: bool,
}

impl EngineProbe {
    fn new() -> (Self, ProbeEnd) {
        let (hash, seen) = tokio::sync::watch::channel(None);
        let flushes = std::sync::Arc::default();
        let seat_frames = std::sync::Arc::default();
        let mute = std::sync::Arc::default();
        (
            Self {
                seat_frames: std::sync::Arc::clone(&seat_frames),
                hash: seen,
                flushes: std::sync::Arc::clone(&flushes),
                mute: std::sync::Arc::clone(&mute),
            },
            ProbeEnd {
                seat_frames,
                hash: std::sync::Arc::new(hash),
                flushes,
                mute,
                clocked: false,
            },
        )
    }
}

/// An agent whose engines run their clocks — decision, reconnect window,
/// and the hold — as `baylee-engine-server` does, for the tests about a
/// seat whose connection is gone.
#[allow(dead_code)] // only the departure tests need clocks
pub async fn attach_agent_clocked(gateway: &Gateway) -> tokio::task::JoinHandle<()> {
    let (_, mut end) = EngineProbe::new();
    end.clocked = true;
    let presets = tokio::sync::mpsc::unbounded_channel().0;
    attach_agent_inner(gateway, presets, end, None, 0).await
}

/// The agent of [`attach_agent`], with a probe into its engines.
pub async fn attach_agent_probed(gateway: &Gateway) -> (tokio::task::JoinHandle<()>, EngineProbe) {
    let (probe, end) = EngineProbe::new();
    let presets = tokio::sync::mpsc::unbounded_channel().0;
    (
        attach_agent_inner(gateway, presets, end, None, 0).await,
        probe,
    )
}

/// An agent that takes at most `capacity` engines at once (a replay job is
/// one), with a probe into them: the tests about a job counting against
/// an agent's room.
#[allow(dead_code)] // only the replay tests need a bounded agent
pub async fn attach_agent_with_capacity(
    gateway: &Gateway,
    capacity: u32,
) -> (tokio::task::JoinHandle<()>, EngineProbe) {
    let (probe, end) = EngineProbe::new();
    let presets = tokio::sync::mpsc::unbounded_channel().0;
    (
        attach_agent_inner(gateway, presets, end, None, capacity).await,
        probe,
    )
}

async fn attach_agent_inner(
    gateway: &Gateway,
    presets: tokio::sync::mpsc::UnboundedSender<baylee_core::preset::GamePreset>,
    probe: ProbeEnd,
    seed: Option<u64>,
    capacity: u32,
) -> tokio::task::JoinHandle<()> {
    let url = format!("ws://127.0.0.1:{}/agent/ws", gateway.port);
    let mut ws = dial(&url).await.expect("agent socket");
    send(
        &mut ws,
        &Envelope {
            msg: Some(v1::envelope::Msg::AgentHello(v1::AgentHello {
                token: gateway.agent_token.clone(),
                name: "test-agent".to_string(),
                capacity,
                protocol_version: baylee_protocol::PROTOCOL_VERSION,
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
                tokio::spawn(run_engine(start, presets.clone(), probe.clone(), seed));
            }
        }
    })
}

/// One engine, for one game, over the real engine link.
///
/// No decision clock: a test that wants a seat to run out of time can say so
/// itself, and a clock running under every other test would only add a way for
/// them to fail on a slow machine. `seed`, where given, replaces the one in
/// the game's preset before the engine reads it.
async fn run_engine(
    start: v1::StartEngine,
    presets: tokio::sync::mpsc::UnboundedSender<baylee_core::preset::GamePreset>,
    probe: ProbeEnd,
    seed: Option<u64>,
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
                protocol_version: baylee_protocol::PROTOCOL_VERSION,
                pid: 0,
                build: baylee_build::short().to_owned(),
            })),
        },
    )
    .await;
    let mut runner = EngineRunner::new();
    let wall_ms = || {
        u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap()
    };
    let mut armed = baylee_engine_server::Armed::default();
    loop {
        let delay = runner
            .entrance_deadline()
            .map(|at| std::time::Duration::from_millis(at.saturating_sub(wall_ms())));
        let now = tokio::time::Instant::now();
        if probe.clocked {
            armed.sync(&runner.clocks(), |clock| {
                now + std::time::Duration::from_secs(u64::from(clock.secs))
            });
        }
        let next = armed.next();
        let out = tokio::select! {
            () = async {
                match next {
                    Some((_, at)) => tokio::time::sleep_until(at).await,
                    None => std::future::pending::<()>().await,
                }
            } => {
                let Some((clock, _)) = next else { continue };
                armed.fired(clock);
                runner.tell_time(wall_ms());
                runner.timeout(clock)
            }
            () = async {
                if let Some(delay) = delay { tokio::time::sleep(delay).await; }
                else { std::future::pending::<()>().await; }
            } => {
                runner.tell_time(wall_ms());
                runner.finish_entrance()
            }
            msg = next_msg(&mut ws) => {
                let Some(mut msg) = msg else { break };
                if matches!(&msg, v1::envelope::Msg::SeatFrame(_)) {
                    probe.seat_frames.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
                // A replay job, as `baylee-engine-server` runs one: the
                // tape's pieces, then the socket closes.
                if let v1::envelope::Msg::ProjectRecord(order) = msg {
                    project(&mut ws, order, &probe).await;
                    break;
                }
                if let v1::envelope::Msg::FlushRecord(_) = &msg {
                    probe.flushes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if probe.mute.load(std::sync::atomic::Ordering::SeqCst) {
                        continue;
                    }
                }
                if let (Some(seed), v1::envelope::Msg::GameSetup(setup)) = (seed, &mut msg) {
                    let mut preset: baylee_core::preset::GamePreset =
                        serde_json::from_slice(&setup.preset_json).expect("the gateway's preset");
                    preset.seed = seed;
                    setup.preset_json = serde_json::to_vec(&preset).expect("a preset");
                }
                if let v1::envelope::Msg::GameSetup(setup) = &msg
                    && let Ok(preset) = serde_json::from_slice(&setup.preset_json) {
                    let _ = presets.send(preset);
                }
                runner.tell_time(wall_ms());
                runner.handle(Envelope { msg: Some(msg) }, &[])
            }
        };
        for envelope in out {
            send(&mut ws, &envelope).await;
        }
        // After the frames went out: a test that reads the hash and then
        // asks the gateway something is asking about a game whose every
        // answer is already on the wire.
        probe.hash.send_replace(
            runner
                .session()
                .map(baylee_gamehost::Session::snapshot_hash),
        );
        if runner.finished() {
            break;
        }
    }
}

/// A replay job, as `baylee-engine-server` runs one: the tape's pieces on
/// the engine link. A muted probe's engine never answers (the `504` test).
async fn project(ws: &mut Socket, order: v1::ProjectRecord, probe: &ProbeEnd) {
    if probe.mute.load(std::sync::atomic::Ordering::SeqCst) {
        std::future::pending::<()>().await;
    }
    let pieces = tokio::task::spawn_blocking(move || {
        let mut pieces = Vec::new();
        baylee_engine_server::projection::run(&order, baylee_build::short(), &mut |piece| {
            pieces.push(piece);
        });
        pieces
    })
    .await
    .expect("the projection runs");
    for piece in pieces {
        send(
            ws,
            &Envelope {
                msg: Some(v1::envelope::Msg::TapeChunk(piece)),
            },
        )
        .await;
    }
}

/// Dials a websocket, retrying while the listener comes up.
///
/// With Nagle's algorithm off, as the agent and the engine dial: the engine
/// this harness runs sends its frames in bursts, and with Nagle on each after
/// the first waited for an acknowledgement Linux delays by up to 40 ms.
pub async fn dial(url: &str) -> Option<Socket> {
    for _ in 0..WAIT_TRIES {
        if let Ok((socket, _)) = tokio_tungstenite::connect_async_with_config(url, None, true).await
        {
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
///
/// And it says `SeatReady` (#256), as a client does once it has drawn its
/// table: the engine asks nothing and runs no clock until every seat has,
/// and this harness's engine runs no deadline to open the table without
/// one, so a seat socket that stayed silent would hang its test. Sent at
/// once rather than after the first view, which changes nothing the engine
/// can see: the gateway attaches the seat before it forwards a frame from it.
///
/// Opened on [`baylee_protocol::seat_socket_path`], as the client opens it,
/// so every seat socket here says which protocol it speaks (#271).
///
/// Each attempt spends a fresh ticket (#294), as the client does on every
/// dial: a refused upgrade has already spent the one it showed.
pub async fn dial_seat(port: u16, game_id: &str, seat_token: &str) -> Socket {
    let mut opened = None;
    for _ in 0..WAIT_TRIES {
        let url = seat_url(port, game_id, seat_token);
        if let Ok((socket, _)) =
            tokio_tungstenite::connect_async_with_config(&url, None, true).await
        {
            opened = Some(socket);
            break;
        }
        tokio::time::sleep(WAIT_STEP).await;
    }
    let mut ws = opened.unwrap_or_else(|| {
        panic!("the seat socket of {game_id} never opened within {WAIT_BUDGET:?}")
    });
    send(
        &mut ws,
        &Envelope {
            msg: Some(v1::envelope::Msg::SeatReady(v1::SeatReady {})),
        },
    )
    .await;
    ws
}

/// A seat socket's URL on a gateway listening on `port`, carrying a ticket
/// just bought with `seat_token` (#294): good for one upgrade.
pub fn seat_url(port: u16, game_id: &str, seat_token: &str) -> String {
    seat_url_with(port, game_id, &seat_ticket(port, game_id, seat_token))
}

/// A seat socket's URL for a ticket the caller already holds.
pub fn seat_url_with(port: u16, game_id: &str, ticket: &str) -> String {
    format!(
        "ws://127.0.0.1:{port}{}",
        baylee_protocol::seat_socket_path(game_id, ticket)
    )
}

/// `POST /ws-ticket` as the client sends it: the bearer in the header, the
/// socket in the body. The status and the body, unread.
pub fn ask_ticket(port: u16, bearer: &str, body: &str) -> (u16, String) {
    http(
        port,
        "POST",
        baylee_protocol::WS_TICKET_PATH,
        Some(bearer),
        body,
    )
}

/// A ticket for a seat's socket, bought with its seat token.
pub fn seat_ticket(port: u16, game_id: &str, seat_token: &str) -> String {
    let (status, body) = ask_ticket(
        port,
        seat_token,
        &format!("{{\"socket\":\"seat\",\"game\":\"{game_id}\"}}"),
    );
    assert_eq!(status, 200, "a seat ticket: {body}");
    json_field(&body, "ticket").to_string()
}

/// A ticket for the lobby feed, bought with a session.
pub fn lobby_ticket(port: u16, session: &str) -> String {
    let (status, body) = ask_ticket(port, session, r#"{"socket":"lobby"}"#);
    assert_eq!(status, 200, "a lobby ticket: {body}");
    json_field(&body, "ticket").to_string()
}

/// The lobby feed's URL with a fresh ticket, and whatever search follows.
pub fn lobby_url(port: u16, session: &str, search: &str) -> String {
    let ticket = lobby_ticket(port, session);
    format!("ws://127.0.0.1:{port}/lobby/ws?ticket={ticket}{search}")
}

pub async fn send(ws: &mut Socket, envelope: &Envelope) {
    let _ = ws
        .send(tokio_tungstenite::tungstenite::Message::Binary(
            envelope.encode_to_vec().into(),
        ))
        .await;
}

/// The next protocol message on a socket, or `None` when it closes.
pub async fn next_msg(ws: &mut Socket) -> Option<v1::envelope::Msg> {
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

/// A request whose answer is read whole: the status, the header block
/// (lower-cased) and the body as bytes. For an answer that is neither text
/// nor free of headers worth reading, such as a replay's tape, whose
/// `Content-Type` is part of what it promises.
#[allow(dead_code)] // only the replay tests read a binary answer's headers
pub fn http_raw(
    port: u16,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: &str,
) -> (u16, String, Vec<u8>) {
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect http");
    let auth = token.map_or(String::new(), |t| format!("Authorization: Bearer {t}\r\n"));
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n{auth}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).expect("write http");
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).expect("read http");
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("a header block");
    let head = String::from_utf8_lossy(&raw[..split]).to_lowercase();
    let status: u16 = head
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("http status");
    assert!(
        !head.contains("transfer-encoding: chunked"),
        "this reader does not de-chunk: {head}"
    );
    (status, head, raw[split + 4..].to_vec())
}
