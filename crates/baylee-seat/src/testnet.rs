//! Test support: an address on this machine that answers every request
//! with a redirect, and the listener the redirect points to, which counts
//! whatever reaches it. Every HTTP client the bridge builds is held to it.

use axum::Router;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

/// A redirecting address and where it points.
pub(crate) struct Redirect {
    /// The address that answers with the redirect.
    pub base: String,
    /// The port the redirect names, to look for in what the client said.
    pub target_port: String,
    /// The headers of every request that reached `base`.
    asked: Arc<Mutex<Vec<HeaderMap>>>,
    /// Requests that reached the address the redirect names.
    followed: Arc<AtomicUsize>,
}

impl Redirect {
    /// The headers of every request that reached the redirecting address.
    pub fn asked(&self) -> Vec<HeaderMap> {
        self.asked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// How many requests reached the address the redirect names.
    pub fn followed(&self) -> usize {
        self.followed.load(Ordering::SeqCst)
    }
}

/// An address that answers every request with `status`, a `Location` on
/// another listener of this machine (the same path there), and `body`, in
/// which `LOCATION` is replaced by where it points: a client that read the
/// body would repeat the address.
pub(crate) async fn redirect(status: u16, body: &serde_json::Value) -> Redirect {
    let followed = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&followed);
    let target = Router::new().fallback(move || {
        let counted = Arc::clone(&counted);
        async move {
            counted.fetch_add(1, Ordering::SeqCst);
            StatusCode::OK
        }
    });
    let target_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a port");
    let target_port = target_listener.local_addr().expect("an address").port();
    tokio::spawn(async move { axum::serve(target_listener, target).await });

    let asked = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&asked);
    let body = body.to_string();
    let redirecting = Router::new().fallback(move |uri: Uri, headers: HeaderMap| {
        let kept = Arc::clone(&kept);
        let body = body.clone();
        async move {
            kept.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(headers);
            let location = format!("http://127.0.0.1:{target_port}{}", uri.path());
            let status = StatusCode::from_u16(status).expect("a status");
            let answer: Response = (
                status,
                [
                    ("location", location.clone()),
                    ("content-type", "application/json".to_string()),
                ],
                body.replace("LOCATION", &location),
            )
                .into_response();
            answer
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a port");
    let addr = listener.local_addr().expect("an address");
    tokio::spawn(async move { axum::serve(listener, redirecting).await });
    Redirect {
        base: format!("http://{addr}"),
        target_port: target_port.to_string(),
        asked,
        followed,
    }
}
