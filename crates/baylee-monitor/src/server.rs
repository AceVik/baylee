//! Local assets and bounded WebSocket fanout, with same-origin admission.

use crate::{
    admission::Admission,
    types::{Facts, Point},
};
use axum::{
    Router,
    body::Body,
    extract::{
        State as Extract, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{Semaphore, watch};

#[derive(Clone)]
pub(crate) struct State {
    pub facts: Facts,
    pub history: Arc<Mutex<VecDeque<Point>>>,
    pub receiver: watch::Receiver<Arc<String>>,
    pub clients: Arc<Semaphore>,
    pub admission: Admission,
}

pub(crate) fn router(state: State) -> Router {
    Router::new()
        .route(
            "/",
            get(|| async { Html(include_str!("../web/index.html")) }),
        )
        .route(
            "/style.css",
            get(|| async {
                (
                    [("content-type", "text/css; charset=utf-8")],
                    include_str!("../web/style.css"),
                )
            }),
        )
        .route(
            "/app.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../web/app.js"),
                )
            }),
        )
        .route("/api/snapshot", get(snapshot))
        .route("/ws", get(upgrade))
        .route("/source", get(source))
        .layer(middleware::from_fn_with_state(state.clone(), protect))
        .with_state(state)
}

async fn snapshot(Extract(state): Extract<State>) -> Response {
    let json = state.receiver.borrow().as_ref().clone();
    if json.is_empty() {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    ([("content-type", "application/json")], json).into_response()
}

async fn protect(Extract(state): Extract<State>, request: Request<Body>, next: Next) -> Response {
    // Refusals carry the same headers: the 403 page is shown to people too.
    let mut response = match state.admission.check(request.headers()) {
        Ok(()) => next.run(request).await,
        Err(rejection) => state.admission.refuse(&rejection),
    };
    for (name, value) in [
        (
            "content-security-policy",
            "default-src 'self'; connect-src 'self'; style-src 'self'; script-src 'self'; img-src 'self' data:; frame-ancestors 'none'; base-uri 'none'; form-action 'none'",
        ),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "no-referrer"),
        ("cache-control", "no-store"),
    ] {
        response
            .headers_mut()
            .insert(name, HeaderValue::from_static(value));
    }
    response
}

async fn upgrade(Extract(state): Extract<State>, ws: WebSocketUpgrade) -> Response {
    // `protect` has admitted the handshake: browsers always send Origin on
    // websocket handshakes; native clients can omit it.
    let Ok(permit) = state.clients.clone().try_acquire_owned() else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    ws.max_message_size(1024)
        .max_frame_size(1024)
        .on_upgrade(move |socket| async move {
            let _permit = permit;
            stream(socket, state).await;
        })
}

async fn send(socket: &mut WebSocket, json: String) -> bool {
    matches!(
        tokio::time::timeout(
            Duration::from_secs(5),
            socket.send(Message::Text(json.into()))
        )
        .await,
        Ok(Ok(()))
    )
}

async fn stream(mut socket: WebSocket, mut state: State) {
    let initial = {
        let ring = state
            .history
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        serde_json::json!({"type": "hello", "facts": state.facts, "history": *ring}).to_string()
    };
    if !send(&mut socket, initial).await {
        return;
    }
    let latest = state.receiver.borrow_and_update().as_ref().clone();
    if !latest.is_empty() && !send(&mut socket, latest).await {
        return;
    }
    loop {
        tokio::select! {
            changed = state.receiver.changed() => {
                if changed.is_err() { break; }
                let json = state.receiver.borrow_and_update().as_ref().clone();
                if !send(&mut socket, json).await { break; }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Ping(data))) => {
                        if !matches!(tokio::time::timeout(Duration::from_secs(5), socket.send(Message::Pong(data))).await, Ok(Ok(()))) { break; }
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    _ => break, // No commands accepted, including text/binary messages.
                }
            }
        }
    }
}

// Offer the exact embedded source even before this branch is published. The
// downloadable JSON maps relative file paths to UTF-8 contents; no local files
// or runtime arguments are read by the endpoint.
async fn source() -> Response {
    let files = serde_json::json!({
        "README.md": include_str!("../README.md"),
        "Cargo.toml": include_str!("../Cargo.toml"),
        "workspace-Cargo.toml": include_str!("../../../Cargo.toml"),
        "Cargo.lock": include_str!("../../../Cargo.lock"),
        "LICENSE": include_str!("../../../LICENSE"),
        "src/main.rs": include_str!("main.rs"), "src/server.rs": include_str!("server.rs"),
        "src/sampler.rs": include_str!("sampler.rs"), "src/types.rs": include_str!("types.rs"),
        "src/gpu.rs": include_str!("gpu.rs"), "src/power.rs": include_str!("power.rs"),
        "src/listen.rs": include_str!("listen.rs"), "src/admission.rs": include_str!("admission.rs"),
        "src/background.rs": include_str!("background.rs"),
        "src/procfs/mod.rs": include_str!("procfs/mod.rs"), "src/procfs/meminfo.rs": include_str!("procfs/meminfo.rs"),
        "src/procfs/pressure.rs": include_str!("procfs/pressure.rs"), "src/procfs/stat.rs": include_str!("procfs/stat.rs"),
        "src/procfs/swaps.rs": include_str!("procfs/swaps.rs"), "src/procfs/vmstat.rs": include_str!("procfs/vmstat.rs"),
        "src/procfs/netdev.rs": include_str!("procfs/netdev.rs"), "src/procfs/sockstat.rs": include_str!("procfs/sockstat.rs"),
        "src/network.rs": include_str!("network.rs"), "src/sensors.rs": include_str!("sensors.rs"),
        "src/procfs/diskstats.rs": include_str!("procfs/diskstats.rs"), "src/procfs/mountinfo.rs": include_str!("procfs/mountinfo.rs"),
        "src/storage/mod.rs": include_str!("storage/mod.rs"), "src/storage/inventory.rs": include_str!("storage/inventory.rs"),
        "src/storage/rates.rs": include_str!("storage/rates.rs"), "src/storage/udev.rs": include_str!("storage/udev.rs"),
        "src/storage/usage.rs": include_str!("storage/usage.rs"),
        "src/training/mod.rs": include_str!("training/mod.rs"), "src/training/dbus.rs": include_str!("training/dbus.rs"),
        "src/training/units.rs": include_str!("training/units.rs"), "src/training/pipeline.rs": include_str!("training/pipeline.rs"),
        "web/index.html": include_str!("../web/index.html"), "web/style.css": include_str!("../web/style.css"), "web/app.js": include_str!("../web/app.js")
    });
    (
        [
            ("content-type", "application/json"),
            (
                "content-disposition",
                "attachment; filename=baylee-pulse-source.json",
            ),
        ],
        files.to_string(),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};

    #[tokio::test]
    async fn websocket_delivers_history_updates_and_releases_connection_slots() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, receiver) =
            watch::channel(Arc::new(r#"{"type":"sample","sequence":1}"#.to_string()));
        let clients = Arc::new(Semaphore::new(1));
        let state = test_state(address, receiver, clients.clone());
        let server = tokio::spawn(async move {
            axum::serve(listener, router(state)).await.unwrap();
        });
        let result = tokio::time::timeout(Duration::from_secs(5), async {
            let url = format!("ws://{address}/ws");
            let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
            let hello = ws.next().await.unwrap().unwrap().into_text().unwrap();
            let hello: serde_json::Value = serde_json::from_str(&hello).unwrap();
            assert_eq!(hello["type"], "hello");
            assert_eq!(hello["history"].as_array().unwrap().len(), 1);
            assert!(ws.next().await.unwrap().unwrap().into_text().unwrap().contains("\"sequence\":1"));
            sender.send_replace(Arc::new(r#"{"type":"sample","sequence":2}"#.into()));
            assert!(ws.next().await.unwrap().unwrap().into_text().unwrap().contains("\"sequence\":2"));
            let error = tokio_tungstenite::connect_async(&url).await.unwrap_err();
            assert!(matches!(error, tokio_tungstenite::tungstenite::Error::Http(r) if r.status() == StatusCode::TOO_MANY_REQUESTS));
            ws.send(tokio_tungstenite::tungstenite::Message::Text("no commands".into())).await.unwrap();
            let _ = ws.next().await;
            while clients.available_permits() == 0 { tokio::task::yield_now().await; }
        }).await;
        server.abort();
        assert!(server.await.unwrap_err().is_cancelled());
        assert!(result.is_ok(), "websocket exchange timed out");
    }

    #[tokio::test]
    async fn unknown_hosts_get_a_readable_refusal_and_no_websocket() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (_sender, receiver) = watch::channel(Arc::new(String::new()));
        let state = test_state(address, receiver, Arc::new(Semaphore::new(1)));
        let server = tokio::spawn(async move {
            axum::serve(listener, router(state)).await.unwrap();
        });
        let get = |host: String| async move {
            let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
            let request = format!("GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
            stream.write_all(request.as_bytes()).await.unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).await.unwrap();
            response
        };
        let result = tokio::time::timeout(Duration::from_secs(5), async {
            let refused = get("rebound.invalid:8787".into()).await;
            assert!(refused.starts_with("HTTP/1.1 403"), "{refused}");
            assert!(refused.contains("content-type: text/plain; charset=utf-8"), "{refused}");
            assert!(refused.contains("x-content-type-options: nosniff"), "{refused}");
            assert!(refused.contains(&format!("  http://{address}\n")), "{refused}");
            assert!(get(address.to_string()).await.starts_with("HTTP/1.1 200"));

            let mut request = format!("ws://{address}/ws").into_client_request().unwrap();
            request.headers_mut().insert("host", HeaderValue::from_static("rebound.invalid:8787"));
            let error = tokio_tungstenite::connect_async(request).await.unwrap_err();
            assert!(matches!(error, tokio_tungstenite::tungstenite::Error::Http(r) if r.status() == StatusCode::FORBIDDEN));
        })
        .await;
        server.abort();
        assert!(result.is_ok(), "refusal exchange timed out");
    }

    fn test_state(
        address: std::net::SocketAddr,
        receiver: watch::Receiver<Arc<String>>,
        clients: Arc<Semaphore>,
    ) -> State {
        State {
            facts: Facts {
                hostname: "test".into(),
                os: "test".into(),
                cpu: "test".into(),
                physical_cores: Some(1),
                threads: 2,
                interval_ms: 1000,
            },
            history: Arc::new(Mutex::new(VecDeque::from([Point::default()]))),
            receiver,
            clients,
            admission: Admission::new(address, &[]),
        }
    }
}
