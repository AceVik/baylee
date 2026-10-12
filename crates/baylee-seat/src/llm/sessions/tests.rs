use super::*;
use axum::{
    Json, Router,
    extract::State as Extract,
    http::{HeaderMap, Method, Uri},
    routing::any,
};
use std::sync::atomic::AtomicUsize;

#[derive(Clone, Default)]
struct Fixture {
    events: Arc<Mutex<Vec<(Method, String, Value)>>>,
    creates: Arc<AtomicUsize>,
    entered: Arc<tokio::sync::Notify>,
    resume: Arc<tokio::sync::Notify>,
    block: Arc<AtomicBool>,
    expire: Arc<AtomicBool>,
}
async fn serve(
    Extract(f): Extract<Fixture>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> (axum::http::StatusCode, Json<Value>) {
    assert_eq!(headers["authorization"], "Bearer TEST-only");
    lock(&f.events).push((
        method.clone(),
        uri.path().into(),
        serde_json::from_slice(&body).unwrap_or(Value::Null),
    ));
    let value = if uri.path() == "/v1/sessions" {
        let n = f.creates.fetch_add(1, Ordering::SeqCst);
        f.entered.notify_one();
        if f.block.load(Ordering::SeqCst) {
            f.resume.notified().await;
        }
        json!({"id":format!("session_{n}"),"idle_timeout_secs":3})
    } else if uri.path().ends_with("/touch") && f.expire.load(Ordering::SeqCst) {
        return (axum::http::StatusCode::GONE, Json(json!({})));
    } else {
        json!({"deleted":method == Method::DELETE})
    };
    (axum::http::StatusCode::OK, Json(value))
}
async fn fixture() -> (Arc<Session>, Fixture, tokio::task::JoinHandle<()>) {
    let f = Fixture::default();
    let app = Router::new().fallback(any(serve)).with_state(f.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let agent = ureq::Agent::config_builder()
        .max_redirects(0)
        .http_status_as_error(false)
        .build()
        .new_agent();
    (
        Session::new(
            agent,
            base,
            vec![("authorization", "Bearer TEST-only".into())],
            2,
        ),
        f,
        task,
    )
}
async fn id(session: &Arc<Session>) -> Result<String, String> {
    let session = session.clone();
    tokio::task::spawn_blocking(move || session.id(TIMEOUT))
        .await
        .unwrap()
}
async fn until(f: &Fixture, suffix: &str, method: Method) {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if lock(&f.events)
                .iter()
                .any(|(m, p, _)| *m == method && p.ends_with(suffix))
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn reuse_heartbeats_expiry_and_idempotent_close() {
    let (session, f, server) = fixture().await;
    assert_eq!(id(&session).await.unwrap(), "session_0");
    assert_eq!(id(&session).await.unwrap(), "session_0");
    assert_eq!(f.creates.load(Ordering::SeqCst), 1);
    until(&f, "/session_0/touch", Method::POST).await;
    assert!(
        lock(&f.events)[0].2["label"]
            .as_str()
            .unwrap()
            .ends_with(" / seat 2")
    );
    f.expire.store(true, Ordering::SeqCst);
    let touch = session.clone();
    tokio::task::spawn_blocking(move || touch.touch())
        .await
        .unwrap();
    assert_eq!(id(&session).await.unwrap(), "session_1");
    session.close();
    session.close();
    until(&f, "/session_1", Method::DELETE).await;
    assert!(id(&session).await.is_err());
    assert_eq!(
        lock(&f.events)
            .iter()
            .filter(|(m, _, _)| *m == Method::DELETE)
            .count(),
        1
    );
    server.abort();
}
#[tokio::test]
async fn a_creation_arriving_after_game_end_is_still_deleted() {
    let (session, f, server) = fixture().await;
    f.block.store(true, Ordering::SeqCst);
    let create = session.clone();
    let pending = tokio::task::spawn_blocking(move || create.id(TIMEOUT));
    f.entered.notified().await;
    session.close();
    f.resume.notify_one();
    assert!(pending.await.unwrap().is_err());
    until(&f, "/session_0", Method::DELETE).await;
    server.abort();
}
#[test]
fn identifiers_cannot_inject_a_header_or_escape_the_session_path() {
    for id in ["", "../other", "x?secret", "x\r\ny", "é"] {
        assert!(!valid_id(id));
    }
    assert!(!valid_id(&"a".repeat(129)));
    assert!(valid_id("sess_example-0123"));
}

#[tokio::test]
async fn late_expiry_does_not_forget_a_replacement_and_turns_are_exclusive() {
    let (session, f, server) = fixture().await;
    let first = id(&session).await.unwrap();
    session.forget_expired(&first);
    let second = id(&session).await.unwrap();
    session.forget_expired(&first);
    assert_eq!(id(&session).await.unwrap(), second);
    assert_eq!(f.creates.load(Ordering::SeqCst), 2);
    {
        let turn = session.turn().unwrap();
        assert!(session.turn().is_err());
        drop(turn);
        assert!(session.turn().is_ok());
    }
    session.close();
    until(&f, &format!("/{second}"), Method::DELETE).await;
    server.abort();
}
