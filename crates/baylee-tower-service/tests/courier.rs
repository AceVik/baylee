//! Real bounded HTTP and durable outbox; remote Central is explicitly synthetic here.
#![forbid(unsafe_code)]
#[path = "courier/fixture.rs"]
mod fixture;
#[path = "../../baylee-tower-store/tests/support/mod.rs"]
mod support;

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use baylee_tower_service::{Courier, DeliveryStep};
use baylee_tower_store::Role;
use central_api::{
    Endpoint, Token,
    tower_awards::{Delivery, Disposition, Receipt},
    wire,
};
use sea_orm::ConnectionTrait;
use std::{
    future::pending,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::{Notify, oneshot};

struct Remote {
    token: Token,
    mode: AtomicU8,
    accepted: Mutex<Option<Delivery>>,
    calls: AtomicUsize,
    stolen: AtomicUsize,
    started: Notify,
    resume: Notify,
}

async fn receive(
    State(remote): State<Arc<Remote>>,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> Response {
    remote.calls.fetch_add(1, Ordering::SeqCst);
    if headers
        .get("authorization")
        .map(axum::http::HeaderValue::as_bytes)
        != Some(remote.token.bearer().as_bytes())
    {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    assert_eq!(
        headers[central_api::PROTOCOL_HEADER],
        central_api::PROTOCOL_VERSION
    );
    let wire::Request::TowerAward { delivery } = central_api::decode(&body).unwrap() else {
        panic!("scoped award request");
    };
    delivery.validate().unwrap();
    let mode = remote.mode.load(Ordering::SeqCst);
    if mode == 0 {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    if mode == 4 {
        return ([("content-type", "application/json")], "x".repeat(4097)).into_response();
    }
    if mode == 5 {
        return (StatusCode::TEMPORARY_REDIRECT, [("location", "/stolen")]).into_response();
    }
    {
        let mut stored = remote.accepted.lock().unwrap();
        if let Some(previous) = *stored {
            assert!(previous == delivery, "exact event is retried unchanged");
        } else {
            *stored = Some(delivery);
        }
    }
    if mode == 6 {
        remote.started.notify_one();
        remote.resume.notified().await;
    }
    let mut receipt = Receipt::new(
        &delivery,
        if mode == 7 {
            Disposition::AccountGone
        } else {
            Disposition::Recorded
        },
    );
    if mode == 1 {
        receipt.stage += 1;
    }
    let response = if mode == 3 {
        wire::Response::TowerProgress {
            progress: central_api::tower_awards::Progress::default(),
        }
    } else {
        wire::Response::TowerAward { receipt }
    };
    (
        [(
            "content-type",
            if mode == 8 {
                "text/plain"
            } else {
                "application/json"
            },
        )],
        central_api::encode(&response).unwrap(),
    )
        .into_response()
}

async fn stolen(State(remote): State<Arc<Remote>>) -> StatusCode {
    remote.stolen.fetch_add(1, Ordering::SeqCst);
    StatusCode::OK
}

#[tokio::test]
async fn delivery_is_bounded_bound_to_the_receipt_and_recovers_after_an_ambiguous_response() {
    let sandbox = support::Sandbox::open().await;
    let remote = Arc::new(Remote {
        token: Token::generate().unwrap(),
        mode: AtomicU8::new(0),
        accepted: Mutex::new(None),
        calls: AtomicUsize::new(0),
        stolen: AtomicUsize::new(0),
        started: Notify::new(),
        resume: Notify::new(),
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let endpoint = Endpoint::development(&format!("http://{address}")).unwrap();
    let router = Router::new()
        .route("/v1/native", post(receive))
        .route("/stolen", post(stolen))
        .with_state(remote.clone());
    let (stop, closed) = oneshot::channel();
    let mut server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = closed.await;
            })
            .await
            .unwrap();
    });
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(exercise(store, db, schema, remote, endpoint)).await;
    let _ = stop.send(());
    let stopped = tokio::time::timeout(Duration::from_secs(5), &mut server).await;
    if stopped.is_err() {
        server.abort();
        let _ = server.await;
    }
    let closed = tokio::net::TcpStream::connect(address).await.is_err();
    sandbox.close().await;
    assert!(
        stopped.is_ok_and(|r| r.is_ok()) && closed,
        "owned HTTP listener and database closed"
    );
    observed.expect("durable delivery failure scenarios completed");
}

async fn exercise(
    store: baylee_tower_store::Store,
    db: sea_orm::DatabaseConnection,
    schema: String,
    remote: Arc<Remote>,
    endpoint: Endpoint,
) {
    fixture::event(&store, uuid::Uuid::now_v7()).await;
    let principal = fixture::operator(&store, Role::Courier).await;
    let host = fixture::operator(&store, Role::Host).await;
    let token = Token::from_hex(&remote.token.hex()).unwrap();
    assert!(
        Courier::new(
            store.clone(),
            host,
            &endpoint,
            "http://127.0.0.1:29871",
            token
        )
        .is_err()
    );
    let mut courier = Courier::new(
        store.clone(),
        principal.clone(),
        &endpoint,
        "http://127.0.0.1:29871",
        Token::from_hex(&remote.token.hex()).unwrap(),
    )
    .unwrap();
    remote_failures(&mut courier, &db, &schema, &remote).await;
    // A crash after a remote commit but before a local ACK also retries exactly.
    db.execute_unprepared(&format!(
        "UPDATE \"{schema}\".award_outbox SET delivered_at=NULL,lease_until=1"
    ))
    .await
    .unwrap();
    assert_eq!(
        courier.next(pending()).await.unwrap(),
        DeliveryStep::Recorded
    );
    let stored = remote.accepted.lock().unwrap().unwrap();
    let run = store
        .get(
            uuid::Uuid::from_bytes(stored.account),
            uuid::Uuid::from_bytes(stored.fact.run),
        )
        .await
        .unwrap();
    assert_eq!(
        (run.state.wins(), run.state.economy.gold),
        (1, 30),
        "delivery never alters gameplay rewards"
    );
    db.execute_unprepared(&format!(
        "UPDATE \"{schema}\".award_outbox SET delivered_at=NULL,lease_until=1"
    ))
    .await
    .unwrap();
    remote.mode.store(7, Ordering::SeqCst);
    assert_eq!(
        courier.next(pending()).await.unwrap(),
        DeliveryStep::AccountGone
    );
    db.execute_unprepared(&format!(
        "UPDATE \"{schema}\".operators SET enabled=false WHERE role='courier'"
    ))
    .await
    .unwrap();
    let summary = courier
        .serve(tokio::time::sleep(Duration::from_millis(50)))
        .await;
    assert_eq!(
        summary.unavailable, 1,
        "local revocation does not terminate the service or spin"
    );
}

async fn remote_failures(
    courier: &mut Courier,
    db: &sea_orm::DatabaseConnection,
    schema: &str,
    remote: &Remote,
) {
    let reset = format!(
        "UPDATE \"{schema}\".award_outbox SET retry_after=0,lease_until=CASE WHEN claim IS NULL THEN 0 ELSE 1 END"
    );
    for mode in [0, 1, 3, 4, 5, 8] {
        remote.mode.store(mode, Ordering::SeqCst);
        assert_eq!(
            courier.next(pending()).await.unwrap(),
            DeliveryStep::Deferred
        );
        assert_eq!(
            courier.next(pending()).await.unwrap(),
            DeliveryStep::Idle,
            "backoff prevents immediate retries"
        );
        db.execute_unprepared(&reset).await.unwrap();
    }
    assert_eq!(
        remote.stolen.load(Ordering::SeqCst),
        0,
        "no redirect receives the issuer credential"
    );
    remote.mode.store(6, Ordering::SeqCst);
    assert_eq!(
        courier.next(remote.started.notified()).await.err(),
        Some(baylee_tower_service::Error::Cancelled)
    );
    remote.mode.store(2, Ordering::SeqCst);
    remote.resume.notify_one();
    assert_eq!(
        courier.next(pending()).await.unwrap(),
        DeliveryStep::Idle,
        "an ambiguous attempt retains its lease"
    );
    db.execute_unprepared(&reset).await.unwrap();
    assert_eq!(
        courier.next(pending()).await.unwrap(),
        DeliveryStep::Recorded
    );
    assert_eq!(courier.next(pending()).await.unwrap(), DeliveryStep::Idle);
    assert_eq!(remote.calls.load(Ordering::SeqCst), 8);
}
