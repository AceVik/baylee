//! Local assets and bounded WebSocket fanout, with same-origin admission.

use crate::{
    admission::Admission,
    assets,
    history::{History, Recorded},
    types::Facts,
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
use serde::Serialize;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{Semaphore, watch};

/// A live update must get through quickly, or the viewer is too slow to follow.
const SEND_TIMEOUT: Duration = Duration::from_secs(5);
/// The first message carries the history, a few megabytes on a phone's Wi-Fi.
const HELLO_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub(crate) struct State {
    pub facts: Facts,
    pub history: Arc<Mutex<History>>,
    pub receiver: watch::Receiver<Arc<String>>,
    pub clients: Arc<Semaphore>,
    pub admission: Admission,
}

pub(crate) fn router(state: State) -> Router {
    let mut router = Router::new().route(
        "/",
        get(|| async { Html(include_str!("../web/index.html")) }),
    );
    for &(path, text) in assets::WEB {
        let (url, kind) = assets::served(path);
        router = router.route(
            url,
            get(move || async move { ([("content-type", kind)], text) }),
        );
    }
    router
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

async fn send(socket: &mut WebSocket, json: String, patience: Duration) -> bool {
    matches!(
        tokio::time::timeout(patience, socket.send(Message::Text(json.into()))).await,
        Ok(Ok(()))
    )
}

/// What a new viewer gets first. Serialized straight from the rings: a JSON
/// tree of them would cost ten times their size, per connecting viewer.
#[derive(Serialize)]
struct Hello<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    facts: &'a Facts,
    #[serde(flatten)]
    recorded: Recorded<'a>,
}

async fn stream(mut socket: WebSocket, mut state: State) {
    let initial = {
        let history = state
            .history
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        serde_json::to_string(&Hello {
            kind: "hello",
            facts: &state.facts,
            recorded: history.recorded(),
        })
    };
    let Ok(initial) = initial else { return };
    if !send(&mut socket, initial, HELLO_TIMEOUT).await {
        return;
    }
    let latest = state.receiver.borrow_and_update().as_ref().clone();
    if !latest.is_empty() && !send(&mut socket, latest, SEND_TIMEOUT).await {
        return;
    }
    loop {
        tokio::select! {
            changed = state.receiver.changed() => {
                if changed.is_err() { break; }
                let json = state.receiver.borrow_and_update().as_ref().clone();
                if !send(&mut socket, json, SEND_TIMEOUT).await { break; }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Ping(data))) => {
                        if !matches!(tokio::time::timeout(SEND_TIMEOUT, socket.send(Message::Pong(data))).await, Ok(Ok(()))) { break; }
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
    let files: BTreeMap<_, _> = assets::source_files().collect();
    let Ok(files) = serde_json::to_string(&files) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    (
        [
            ("content-type", "application/json"),
            (
                "content-disposition",
                "attachment; filename=baylee-pulse-source.json",
            ),
        ],
        files,
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
            assert_eq!(hello["minutes"].as_array().unwrap().len(), 0);
            assert_eq!(hello["facts"]["hostname"], "test");
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
        let mut history = History::new(10);
        history.push(crate::types::Point::default());
        State {
            facts: Facts {
                hostname: "test".into(),
                os: "test".into(),
                kernel: None,
                cpu: "test".into(),
                physical_cores: Some(1),
                threads: 2,
                interval_ms: 1000,
            },
            history: Arc::new(Mutex::new(history)),
            receiver,
            clients,
            admission: Admission::new(address, &[]),
        }
    }
}
