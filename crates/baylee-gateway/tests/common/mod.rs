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
}

impl Drop for Gateway {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.store_path);
    }
}

/// Starts a gateway on a free port with an empty store.
///
/// `label` keeps two tests running at once from sharing a store file.
pub fn spawn_gateway(label: &str) -> Gateway {
    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("a free port");
    let port = probe.local_addr().expect("bound").port();
    drop(probe);
    let store_path = std::env::temp_dir().join(format!("baylee-gateway-{label}-{port}.json"));
    let _ = std::fs::remove_file(&store_path);
    let agent_token = format!("test-agent-secret-{port}");
    let loud = std::env::var("GATEWAY_DEBUG").is_ok();
    let child = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-gateway"))
        // The gateway reads `data/acceptance-decks.txt` for the house deck by
        // a workspace-relative path; a test binary's working directory is its
        // own crate, which is not where that file is.
        .current_dir(workspace_root())
        .env("PORT", port.to_string())
        .env("STORE_PATH", &store_path)
        .env("BAYLEE_AGENT_TOKEN", &agent_token)
        .env("RUST_LOG", if loud { "info" } else { "off" })
        .stdout(std::process::Stdio::null())
        .stderr(if loud {
            std::process::Stdio::inherit()
        } else {
            std::process::Stdio::null()
        })
        .spawn()
        .expect("spawn gateway");
    for _ in 0..50 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Gateway {
        port,
        agent_token,
        child,
        store_path,
    }
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
type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Connects an agent and waits until the gateway has registered it.
///
/// Returning only after the welcome matters: a game created before any agent
/// is registered is refused, which is correct and is not what a test about
/// something else wants to discover.
pub async fn attach_agent(gateway: &Gateway) -> tokio::task::JoinHandle<()> {
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
                tokio::spawn(run_engine(start));
            }
        }
    })
}

/// One engine, for one game, over the real engine link.
///
/// No decision clock: a test that wants a seat to run out of time can say so
/// itself, and a clock running under every other test would only add a way for
/// them to fail on a slow machine.
async fn run_engine(start: v1::StartEngine) {
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
        for out in runner.handle(Envelope { msg: Some(msg) }) {
            send(&mut ws, &out).await;
        }
        if runner.finished() {
            break;
        }
    }
}

/// Dials a websocket, retrying while the listener comes up.
async fn dial(url: &str) -> Option<Socket> {
    for _ in 0..50 {
        if let Ok((socket, _)) = tokio_tungstenite::connect_async(url).await {
            return Some(socket);
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    None
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
