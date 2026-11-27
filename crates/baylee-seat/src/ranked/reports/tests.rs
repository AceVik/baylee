//! The runner's direct reports against a stand-in official gateway on
//! this machine: the outbox keeps a report until Central filed it, tries
//! again after a limit or no answer, sets a refused one aside with its
//! answer, and queues nothing that holds a secret.

use super::*;
use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::IntoResponse as _;
use serde_json::json;
use std::collections::VecDeque;
use std::sync::Mutex;

/// What the stand-in was sent (path, whether it was shown a bearer, body),
/// and the statuses it answers with, in turn; `201` once they run out.
#[derive(Clone, Default)]
struct Desk {
    sent: Arc<Mutex<Vec<(String, bool, Value)>>>,
    answers: Arc<Mutex<VecDeque<u16>>>,
}

impl Desk {
    fn sent(&self) -> Vec<(String, bool, Value)> {
        crate::llm::lock(&self.sent).clone()
    }
}

/// A gateway whose `POST /client/reports` answers as `answers` says.
async fn gateway(answers: &[u16]) -> (String, Desk) {
    let desk = Desk::default();
    crate::llm::lock(&desk.answers).extend(answers);
    let take = |State(desk): State<Desk>, uri: Uri, headers: HeaderMap, body: String| async move {
        let n = {
            let mut sent = crate::llm::lock(&desk.sent);
            sent.push((
                uri.path().to_string(),
                headers.contains_key("authorization"),
                serde_json::from_str(&body).unwrap_or(Value::Null),
            ));
            sent.len()
        };
        let status = crate::llm::lock(&desk.answers).pop_front().unwrap_or(201);
        let status = StatusCode::from_u16(status).unwrap();
        if status.is_success() {
            (
                status,
                axum::Json(json!({"report_id": format!("TEST-report-{n}")})),
            )
                .into_response()
        } else {
            (status, axum::Json(json!({"error": "TEST refusal"}))).into_response()
        }
    };
    let app = Router::new()
        .route("/client/reports", axum::routing::post(take))
        .with_state(desk.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await });
    (format!("http://{address}"), desk)
}

/// A fresh state directory of the runner's.
fn state(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("baylee-seat-direct-{name}-{}", Uuid::now_v7()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A report as the bridge would queue one.
fn body(n: u32) -> Value {
    json!({
        "kind": "bug", "text": format!("report {n}"),
        "build": {"version": "0.2.1"}, "device": "0123456789abcdef0123456789abcdef",
        "client": {"source": "llm-seat", "game_id": "TEST-game"},
    })
}

#[test]
fn reports_go_to_an_https_origin_or_one_on_this_machine() {
    let dir = state("origin");
    assert!(DirectReports::open("https://gateway.example", &dir).is_ok());
    assert!(DirectReports::open("http://127.0.0.1:9", &dir).is_ok());
    for refused in [
        "http://gateway.example",
        "",
        "gateway.example",
        "https://a@b.example",
    ] {
        assert!(DirectReports::open(refused, &dir).is_err(), "{refused}");
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// The account's reports go under one device id, made once of random
/// bytes and kept in its state; one that is not an id is replaced.
#[test]
fn the_account_keeps_its_device_id() {
    let dir = state("device");
    let first = DirectReports::open("https://gateway.example", &dir).unwrap();
    assert!(bugreport::is_device_id(&first.device), "{}", first.device);
    let again = DirectReports::open("https://gateway.example", &dir).unwrap();
    assert_eq!(first.device, again.device);
    std::fs::write(dir.join(DEVICE), "not an id\n").unwrap();
    let replaced = DirectReports::open("https://gateway.example", &dir).unwrap();
    assert!(bugreport::is_device_id(&replaced.device));
    assert_ne!(replaced.device, first.device);
    assert_eq!(
        std::fs::read_to_string(dir.join(DEVICE)).unwrap().trim(),
        replaced.device
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// A limit keeps every report, the first tried and the ones behind it,
/// and the next pass sends them in order; each leaves the outbox once the
/// gateway answered that Central filed it. No credential goes with them.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_report_leaves_the_outbox_once_central_filed_it() {
    let (origin, desk) = gateway(&[429]).await;
    let dir = state("limit");
    let reports = DirectReports::open(&origin, &dir).unwrap();
    reports.queue(&body(1), &[]).unwrap();
    reports.queue(&body(2), &[]).unwrap();
    assert_eq!(reports.pending().len(), 2);
    let Pass::Later(why) = reports.pass().await else {
        panic!("a limit is not the end");
    };
    assert!(why.contains("429"), "{why}");
    assert_eq!(reports.pending().len(), 2, "both kept");
    assert_eq!(desk.sent().len(), 1, "the rest wait behind the first");
    assert_eq!(reports.pass().await, Pass::Clear);
    assert!(reports.pending().is_empty());
    let sent = desk.sent();
    assert_eq!(sent.len(), 3);
    for (path, bearer, _) in &sent {
        assert_eq!(path, "/client/reports");
        assert!(!bearer, "a direct report carries no credential");
    }
    assert_eq!(sent[1].2, body(1), "in the order queued");
    assert_eq!(sent[2].2, body(2));
    let _ = std::fs::remove_dir_all(dir);
}

/// A report the gateway refuses for what it is would be refused again: it
/// leaves the outbox for the refused directory, its answer beside it, and
/// the ones behind it still go.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_refused_report_is_set_aside_with_its_answer() {
    let (origin, desk) = gateway(&[400]).await;
    let dir = state("refused");
    let reports = DirectReports::open(&origin, &dir).unwrap();
    let first = reports.queue(&body(1), &[]).unwrap();
    reports.queue(&body(2), &[]).unwrap();
    assert_eq!(reports.pass().await, Pass::Clear);
    assert!(reports.pending().is_empty());
    assert_eq!(desk.sent().len(), 2);
    let kept = reports.refused().join(first.file_name().unwrap());
    let kept_body: Value = serde_json::from_slice(&std::fs::read(&kept).unwrap()).unwrap();
    assert_eq!(kept_body, body(1));
    let why = std::fs::read_to_string(kept.with_extension("why")).unwrap();
    assert!(why.contains("400") && why.contains("TEST refusal"), "{why}");
    let _ = std::fs::remove_dir_all(dir);
}

/// A gateway that does not answer, or does not take direct reports now,
/// keeps the report for later.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_answer_or_a_gateway_not_taking_them_keeps_the_report() {
    let closed = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap()
    };
    let dir = state("unreachable");
    let reports = DirectReports::open(&format!("http://{closed}"), &dir).unwrap();
    reports.queue(&body(1), &[]).unwrap();
    assert!(matches!(reports.pass().await, Pass::Later(_)));
    assert_eq!(reports.pending().len(), 1);
    let (origin, _desk) = gateway(&[503, 500]).await;
    let busy = DirectReports::open(&origin, &dir).unwrap();
    for _ in 0..2 {
        assert!(matches!(busy.pass().await, Pass::Later(_)));
        assert_eq!(busy.pending().len(), 1);
    }
    assert_eq!(busy.pass().await, Pass::Clear);
    let _ = std::fs::remove_dir_all(dir);
}

/// A report that holds one of the secrets is not queued, and the reason
/// names the secret, never its value.
#[test]
fn a_report_holding_a_secret_is_not_queued() {
    let dir = state("secret");
    let reports = DirectReports::open("https://gateway.example", &dir).unwrap();
    let ticket = "TEST-seat-ticket-0123456789";
    let mut leaky = body(1);
    leaky["client"]["note"] = json!(format!("the ticket was {ticket}"));
    let secrets = [Secret {
        label: Keyring::SEAT,
        value: ticket,
    }];
    let why = reports.queue(&leaky, &secrets).unwrap_err();
    assert!(
        why.contains(Keyring::SEAT) && !why.contains(ticket),
        "{why}"
    );
    assert!(reports.pending().is_empty());
    assert!(reports.queue(&body(2), &secrets).is_ok());
    let _ = std::fs::remove_dir_all(dir);
}

/// The sender posts a report as soon as it is queued, and ends when told
/// to leave.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_sender_posts_what_is_queued_and_leaves_when_told() {
    let (origin, desk) = gateway(&[]).await;
    let dir = state("sender");
    let reports = Arc::new(DirectReports::open(&origin, &dir).unwrap());
    let (stop, stopping) = watch::channel(Stop::Run);
    let sender = reports.deliver(stopping);
    tokio::time::sleep(Duration::from_millis(200)).await;
    reports.queue(&body(1), &[]).unwrap();
    let until = tokio::time::Instant::now() + Duration::from_secs(10);
    while !reports.pending().is_empty() {
        assert!(tokio::time::Instant::now() < until, "not sent");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(desk.sent().len(), 1);
    stop.send(Stop::Leave).unwrap();
    let until = tokio::time::Instant::now() + Duration::from_secs(5);
    while !sender.0.is_finished() {
        assert!(
            tokio::time::Instant::now() < until,
            "the sender did not leave"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let _ = std::fs::remove_dir_all(dir);
}
