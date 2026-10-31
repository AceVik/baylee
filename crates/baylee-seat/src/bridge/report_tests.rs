//! A game's reports as the bridge files them: one bug report to a
//! stand-in gateway on this machine, under the seat's session or on the
//! seat token of a chair taken on its host's ticket, or none.

use super::*;
use crate::lobby::{Chair, Lobby, Session};
use crate::mind::{Disclosure, Request, Thinking};
use crate::reports::{Filed, Report};
use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use std::sync::Mutex;

/// What the stand-in gateway was sent: the path, the bearer it was shown
/// and the body.
type Sent = Arc<Mutex<Vec<(String, Option<String>, Value)>>>;

/// Where a chair taken on its host's ticket sends its report.
const CHAIR_REPORT: &str = "/lobby/games/TEST-game/chair/report";

/// A gateway whose `POST /reports` and `POST …/chair/report` answer
/// `status`, keeping what they were sent.
async fn gateway(status: u16) -> (String, Sent) {
    let sent: Sent = Arc::default();
    let take = move |State(sent): State<Sent>, uri: Uri, headers: HeaderMap, body: String| async move {
        let bearer = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        crate::llm::lock(&sent).push((
            uri.path().to_string(),
            bearer,
            serde_json::from_str(&body).unwrap(),
        ));
        let status = StatusCode::from_u16(status).unwrap();
        let answer: Response = if status.is_success() {
            (status, axum::Json(json!({"report_id": "TEST-report"}))).into_response()
        } else {
            (
                status,
                axum::Json(json!({"error": "reports are not configured"})),
            )
                .into_response()
        };
        answer
    };
    let app = Router::new()
        .route("/reports", axum::routing::post(take))
        .route("/lobby/games/{id}/chair/report", axum::routing::post(take))
        .with_state(Arc::clone(&sent));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await });
    (format!("http://{address}"), sent)
}

/// A mind that filed `whats` at this game's seat.
struct Reporting {
    filed: Mutex<Vec<Filed>>,
}

impl Reporting {
    fn new(whats: &[&str]) -> Arc<Self> {
        let filed = whats
            .iter()
            .enumerate()
            .map(|(n, what)| Filed {
                game_id: "TEST-game".into(),
                seat: 1,
                question: 10 + n as u64,
                turn: 3,
                model: "TEST-model".into(),
                profile: Some("collector".into()),
                report: Report {
                    what: (*what).into(),
                    ..Report::default()
                },
                lookups: 0,
                unix: 1,
                seen: Vec::new(),
                log_lines: 3,
            })
            .collect();
        Arc::new(Self {
            filed: Mutex::new(filed),
        })
    }
}

impl Mind for Reporting {
    fn decide(&self, _request: Request) -> Thinking<'_> {
        Box::pin(async { Err(MindError::Declined("not asked here".into())) })
    }

    fn reports(&self, game: &str, seat: u8) -> Vec<Filed> {
        assert_eq!((game, seat), ("TEST-game", 1));
        std::mem::take(&mut *crate::llm::lock(&self.filed))
    }

    fn disclosure(&self) -> Disclosure {
        Disclosure::Llm
    }
}

/// The link of a chair at `base`, under a session or none.
fn link(base: &str, session: bool) -> SeatLink {
    let chair = Chair {
        game_id: "TEST-game".into(),
        seat: 1,
        seat_token: "TEST-seat-token".into(),
    };
    SeatLink::new(
        Lobby::new(base),
        chair,
        session.then(|| Session::from_token("TEST-session".into())),
    )
}

/// A bridge whose mind filed `whats` in the game.
fn bridge(whats: &[&str], send_reports: bool) -> Bridge {
    let mut bridge = Bridge::new(
        Reporting::new(whats),
        PlayOptions {
            send_reports,
            ..PlayOptions::default()
        },
        None,
        Vec::new(),
    );
    bridge.game_seat = Some(("TEST-game".into(), 1));
    bridge
}

/// The `llm_reports` lines of a transcript.
fn noted(transcript: &Transcript) -> Vec<Value> {
    transcript
        .lines()
        .iter()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .filter_map(|line| line.get("llm_reports").cloned())
        .collect()
}

/// A memory the table told nothing.
fn untold() -> crate::TableMemory {
    crate::TableMemory::default()
}

#[tokio::test]
async fn a_game_s_reports_go_to_the_gateway_once_under_the_seat_s_session() {
    let (base, sent) = gateway(201).await;
    let mut bridge = bridge(&["Bolt dealt 2.", "BOLT dealt 2.", "No trigger."], true);
    let mut transcript = Transcript::memory();
    let memory = crate::reports::tests::log::told(crate::reports::tests::log::game());
    let filing = bridge
        .file_reports(&link(&base, true), &mut transcript, &memory)
        .await
        .expect("reports were filed");
    assert_eq!(
        filing,
        Filing {
            count: 2,
            sent: true,
            report_id: Some("TEST-report".into()),
            why: None,
            route: "reports",
        }
    );
    let sent = crate::llm::lock(&sent).clone();
    assert_eq!(sent.len(), 1, "one bug report a game");
    let (path, bearer, body) = &sent[0];
    assert_eq!(path, "/reports");
    assert_eq!(
        bearer.as_deref(),
        Some("Bearer TEST-session"),
        "the session, not the seat token"
    );
    assert_eq!(body["kind"], "bug");
    assert_eq!(body["game_id"], "TEST-game");
    assert_eq!(body["client"]["source"], "llm-seat");
    assert_eq!(body["client"]["reports"].as_array().unwrap().len(), 2);
    assert!(!body.to_string().contains("TEST-seat-token"));
    // The seat's own log rides with them, as the core's memory held it,
    // and each report says where in it it was made.
    let lines = body["client"]["log"]["lines"].as_array().unwrap();
    assert_eq!(lines.len(), memory.log().len());
    assert_eq!(body["client"]["log_omitted"], 0);
    assert_eq!(body["client"]["reports"][0]["log_line"], 3);
    for name in crate::reports::tests::log::NAMES {
        let bare = name.split('#').next().unwrap();
        assert!(!body.to_string().contains(bare), "{bare} was sent");
    }
    assert_eq!(noted(&transcript)[0]["sent"], true);
    assert_eq!(noted(&transcript)[0]["route"], "reports");
}

/// A chair taken on its host's ticket has no session: it sends the same
/// bug report on its seat token, to its chair's own route, and the
/// transcript says so.
#[tokio::test]
async fn a_chair_on_its_host_s_ticket_sends_on_its_seat_token() {
    let (base, sent) = gateway(201).await;
    let mut transcript = Transcript::memory();
    let memory = crate::reports::tests::log::told(crate::reports::tests::log::game());
    let filing = bridge(&["Bolt dealt 2."], true)
        .file_reports(&link(&base, false), &mut transcript, &memory)
        .await
        .unwrap();
    assert_eq!(
        filing,
        Filing {
            count: 1,
            sent: true,
            report_id: Some("TEST-report".into()),
            why: None,
            route: "chair",
        }
    );
    let sent = crate::llm::lock(&sent).clone();
    assert_eq!(sent.len(), 1);
    let (path, bearer, body) = &sent[0];
    assert_eq!(path, CHAIR_REPORT);
    assert_eq!(bearer.as_deref(), Some("Bearer TEST-seat-token"));
    assert_eq!(body["game_id"], "TEST-game");
    assert_eq!(body["client"]["source"], "llm-seat");
    assert!(body["client"]["log"]["lines"].is_array());
    assert!(!body.to_string().contains("TEST-seat-token"));
    let noted = noted(&transcript);
    assert_eq!(
        (&noted[0]["sent"], &noted[0]["route"]),
        (&json!(true), &json!("chair"))
    );
}

#[tokio::test]
async fn reports_stay_at_home_when_they_may_not_or_cannot_go() {
    // Told not to: nothing is sent, by either route.
    let (base, sent) = gateway(201).await;
    let mut transcript = Transcript::memory();
    for session in [true, false] {
        let filing = bridge(&["x"], false)
            .file_reports(&link(&base, session), &mut transcript, &untold())
            .await
            .unwrap();
        assert!(!filing.sent);
        assert_eq!(
            filing.why.as_deref(),
            Some("not sent: the bridge was told not to")
        );
    }
    assert!(crate::llm::lock(&sent).is_empty());
    // A gateway with no feedback service refuses: said, and nothing more.
    let (base, sent) = gateway(503).await;
    let filing = bridge(&["x"], true)
        .file_reports(&link(&base, true), &mut transcript, &untold())
        .await
        .unwrap();
    assert!(!filing.sent);
    assert!(filing.why.as_deref().unwrap().contains("503"), "{filing:?}");
    assert!(!filing.why.as_deref().unwrap().contains("TEST-session"));
    assert_eq!(crate::llm::lock(&sent).len(), 1);
    assert_eq!(noted(&transcript).len(), 3);
    // No reports: no bug report, no line.
    let mut quiet = Transcript::memory();
    assert_eq!(
        bridge(&[], true)
            .file_reports(&link(&base, true), &mut quiet, &untold())
            .await,
        None
    );
    assert!(noted(&quiet).is_empty());
    assert_eq!(crate::llm::lock(&sent).len(), 1);
}

/// A bug report that holds the seat's session or seat token anywhere is
/// not sent, and what is said of it names the token, never its value.
#[tokio::test]
async fn a_report_holding_a_token_is_not_sent() {
    let (base, sent) = gateway(201).await;
    for (what, label, session) in [
        ("The table showed TEST-session.", "session token", true),
        ("My chair is TEST-seat-token.", "seat token", true),
        ("My chair is TEST-seat-token.", "seat token", false),
    ] {
        let mut transcript = Transcript::memory();
        let filing = bridge(&[what], true)
            .file_reports(&link(&base, session), &mut transcript, &untold())
            .await
            .unwrap();
        assert!(!filing.sent);
        let why = filing.why.unwrap();
        assert!(why.contains(label), "{why}");
        assert!(!why.contains("TEST-se"), "{why}");
    }
    assert!(crate::llm::lock(&sent).is_empty(), "nothing reached it");
}
