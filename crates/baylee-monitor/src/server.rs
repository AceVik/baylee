//! Local assets and bounded WebSocket fanout, with same-origin admission.

use crate::types::{Facts, Point};
use axum::{
    Router,
    body::Body,
    extract::{
        State as Extract, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, HeaderValue, Request, StatusCode},
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
    pub authority: String,
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

fn allowed(headers: &HeaderMap, authority: &str) -> bool {
    let host = headers.get("host").and_then(|v| v.to_str().ok());
    if host != Some(authority) {
        return false;
    }
    if headers
        .get("sec-fetch-site")
        .is_some_and(|v| v == "cross-site")
    {
        return false;
    }
    headers
        .get("origin")
        .is_none_or(|v| v == format!("http://{authority}").as_str())
}

async fn protect(Extract(state): Extract<State>, request: Request<Body>, next: Next) -> Response {
    if !allowed(request.headers(), &state.authority) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut response = next.run(request).await;
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

async fn upgrade(
    Extract(state): Extract<State>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    // Browsers always send Origin on websocket handshakes; native clients can omit it.
    if !allowed(&headers, &state.authority) {
        return StatusCode::FORBIDDEN.into_response();
    }
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

    #[test]
    fn prevents_cross_origin_reads_and_dns_rebinding() {
        let mut h = HeaderMap::new();
        h.insert("host", HeaderValue::from_static("192.168.0.103:8787"));
        assert!(allowed(&h, "192.168.0.103:8787"));
        h.insert(
            "origin",
            HeaderValue::from_static("http://attacker.invalid"),
        );
        assert!(!allowed(&h, "192.168.0.103:8787"));
        h.insert(
            "origin",
            HeaderValue::from_static("http://192.168.0.103:8787"),
        );
        assert!(allowed(&h, "192.168.0.103:8787"));
        h.insert("host", HeaderValue::from_static("rebound.invalid:8787"));
        assert!(!allowed(&h, "192.168.0.103:8787"));
    }

    #[tokio::test]
    async fn websocket_delivers_history_updates_and_releases_connection_slots() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, receiver) =
            watch::channel(Arc::new(r#"{"type":"sample","sequence":1}"#.to_string()));
        let clients = Arc::new(Semaphore::new(1));
        let state = State {
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
            clients: clients.clone(),
            authority: address.to_string(),
        };
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
}
