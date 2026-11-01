//! A game's reports as the bridge files them: one bug report to a
//! stand-in gateway on this machine, under the seat's session or on the
//! seat token of a chair taken on its host's ticket, or none.

use super::*;
use crate::lobby::{Chair, Lobby, Session};
use crate::mind::{Disclosure, Request, Thinking};
use crate::reports::Filed;
use crate::reports::tests::{filed_with, report};
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
/// `status`, keeping what they were sent; the `n`th report it takes is
/// `TEST-report-<n>`.
async fn gateway(status: u16) -> (String, Sent) {
    let sent: Sent = Arc::default();
    let take = move |State(sent): State<Sent>, uri: Uri, headers: HeaderMap, body: String| async move {
        let bearer = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let n = {
            let mut sent = crate::llm::lock(&sent);
            sent.push((
                uri.path().to_string(),
                bearer,
                serde_json::from_str(&body).unwrap(),
            ));
            sent.len()
        };
        let status = StatusCode::from_u16(status).unwrap();
        let answer: Response = if status.is_success() {
            (
                status,
                axum::Json(json!({"report_id": format!("TEST-report-{n}")})),
            )
                .into_response()
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

/// A mind that filed `reports` at this game's seat: each `(observed,
/// rule)`, a report that quotes Lightning Bolt's text and cites the rule.
struct Reporting {
    filed: Mutex<Vec<Filed>>,
}

impl Reporting {
    fn new(reports: &[(&str, &str)]) -> Arc<Self> {
        let filed = reports
            .iter()
            .enumerate()
            .map(|(n, (observed, rule))| {
                let mut filed = filed_with(report(observed, rule), 10 + n as u64);
                filed.log_lines = 3;
                filed
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

/// A bridge whose mind filed `reports` in the game ([`Reporting::new`]).
fn bridge(reports: &[(&str, &str)], send_reports: bool) -> Bridge {
    let mut bridge = Bridge::new(
        Reporting::new(reports),
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

/// Three reports, the second the first's dedupe key in other words.
const THREE: [(&str, &str); 3] = [
    ("Bolt dealt 2.", "120.3"),
    ("It dealt two, not three.", "120.3"),
    ("No trigger.", "603.2"),
];

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

/// Under the seat's session each report is a bug report of its own, sent
/// in the order filed, each dedupe key once, each with the seat's log.
#[tokio::test]
async fn a_game_s_reports_go_one_each_under_the_seat_s_session() {
    let (base, sent) = gateway(201).await;
    let mut bridge = bridge(&THREE, true);
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
            bug_reports: 2,
            sent: true,
            report_id: Some("TEST-report-1".into()),
            report_ids: vec!["TEST-report-1".into(), "TEST-report-2".into()],
            why: None,
            route: "reports",
        }
    );
    let sent = crate::llm::lock(&sent).clone();
    assert_eq!(sent.len(), 2, "one bug report a report");
    for (path, bearer, body) in &sent {
        assert_eq!(path, "/reports");
        assert_eq!(
            bearer.as_deref(),
            Some("Bearer TEST-session"),
            "the session, not the seat token"
        );
        assert_eq!(body["kind"], "bug");
        assert_eq!(body["game_id"], "TEST-game");
        let client = &body["client"];
        assert_eq!(client["source"], "llm-seat");
        assert!(client.get("reports").is_none());
        assert!(client["report"]["dedupe_key"].is_string());
        assert!(!body.to_string().contains("TEST-seat-token"));
        // The seat's own log rides with each, as the core's memory held
        // it, and the report says where in it it was made.
        let lines = client["log"]["lines"].as_array().unwrap();
        assert_eq!(lines.len(), memory.log().len());
        assert_eq!(client["log_omitted"], 0);
        assert_eq!(client["report"]["decision"]["log_line"], 3);
        assert_eq!(client["record_tail"].as_array().unwrap().len(), 3);
        for name in crate::reports::tests::log::NAMES {
            let bare = name.split('#').next().unwrap();
            assert!(!body.to_string().contains(bare), "{bare} was sent");
        }
    }
    assert_eq!(sent[0].2["client"]["report"]["observed"], "Bolt dealt 2.");
    assert_eq!(sent[1].2["client"]["report"]["observed"], "No trigger.");
    let noted = noted(&transcript);
    assert_eq!(noted[0]["sent"], true);
    assert_eq!(noted[0]["route"], "reports");
    assert_eq!(
        noted[0]["report_ids"],
        json!(["TEST-report-1", "TEST-report-2"])
    );
}

/// A chair taken on its host's ticket has no session, and its route takes
/// one report a chair a game: it sends the game's reports in one on its
/// seat token, each with its own fields, and the transcript says so.
#[tokio::test]
async fn a_chair_on_its_host_s_ticket_sends_them_in_one_on_its_seat_token() {
    let (base, sent) = gateway(201).await;
    let mut transcript = Transcript::memory();
    let memory = crate::reports::tests::log::told(crate::reports::tests::log::game());
    let filing = bridge(&THREE, true)
        .file_reports(&link(&base, false), &mut transcript, &memory)
        .await
        .unwrap();
    assert_eq!(
        filing,
        Filing {
            count: 2,
            bug_reports: 1,
            sent: true,
            report_id: Some("TEST-report-1".into()),
            report_ids: vec!["TEST-report-1".into()],
            why: None,
            route: "chair",
        }
    );
    let sent = crate::llm::lock(&sent).clone();
    assert_eq!(sent.len(), 1, "one a chair a game");
    let (path, bearer, body) = &sent[0];
    assert_eq!(path, CHAIR_REPORT);
    assert_eq!(bearer.as_deref(), Some("Bearer TEST-seat-token"));
    assert_eq!(body["game_id"], "TEST-game");
    let client = &body["client"];
    assert_eq!(client["source"], "llm-seat");
    let items = client["reports"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    for item in items {
        for flat in [
            "kind",
            "severity",
            "confidence",
            "dedupe_key",
            "oracle_quote_matches",
        ] {
            assert!(item.get(flat).is_some(), "{flat}");
        }
        assert_eq!(item["record_tail"].as_array().unwrap().len(), 3);
    }
    assert!(client["log"]["lines"].is_array());
    assert!(!body.to_string().contains("TEST-seat-token"));
    let noted = noted(&transcript);
    assert_eq!(
        (
            &noted[0]["sent"],
            &noted[0]["route"],
            &noted[0]["bug_reports"]
        ),
        (&json!(true), &json!("chair"), &json!(1))
    );
}

#[tokio::test]
async fn reports_stay_at_home_when_they_may_not_or_cannot_go() {
    // Told not to: nothing is sent, by either route.
    let (base, sent) = gateway(201).await;
    let mut transcript = Transcript::memory();
    for session in [true, false] {
        let filing = bridge(&THREE, false)
            .file_reports(&link(&base, session), &mut transcript, &untold())
            .await
            .unwrap();
        assert!(!filing.sent);
        assert!(filing.report_ids.is_empty());
        assert_eq!(
            filing.why.as_deref(),
            Some("not sent: the bridge was told not to")
        );
    }
    assert!(crate::llm::lock(&sent).is_empty());
    // A gateway with no feedback service refuses the first: said, and the
    // rest are not tried, since they would meet the same.
    let (base, sent) = gateway(503).await;
    let filing = bridge(&THREE, true)
        .file_reports(&link(&base, true), &mut transcript, &untold())
        .await
        .unwrap();
    assert!(!filing.sent);
    assert_eq!((filing.count, filing.bug_reports), (2, 2));
    assert!(filing.report_ids.is_empty());
    assert!(filing.why.as_deref().unwrap().contains("503"), "{filing:?}");
    assert!(!filing.why.as_deref().unwrap().contains("TEST-session"));
    assert_eq!(crate::llm::lock(&sent).len(), 1, "stopped at the first");
    // So does one that answers 429: the account's 64 an hour are spent.
    let (base, sent) = gateway(429).await;
    let filing = bridge(&THREE, true)
        .file_reports(&link(&base, true), &mut transcript, &untold())
        .await
        .unwrap();
    assert!(filing.why.as_deref().unwrap().contains("429"), "{filing:?}");
    assert_eq!(crate::llm::lock(&sent).len(), 1);
    assert_eq!(noted(&transcript).len(), 4);
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
/// not sent, and what is said of it names the token, never its value;
/// the game's other reports still go.
#[tokio::test]
async fn a_report_holding_a_token_is_not_sent() {
    let (base, sent) = gateway(201).await;
    for (observed, label, session) in [
        ("The table showed TEST-session.", "session token", true),
        ("My chair is TEST-seat-token.", "seat token", true),
        ("My chair is TEST-seat-token.", "seat token", false),
    ] {
        let mut transcript = Transcript::memory();
        let filing = bridge(&[(observed, "120.3")], true)
            .file_reports(&link(&base, session), &mut transcript, &untold())
            .await
            .unwrap();
        assert!(!filing.sent);
        let why = filing.why.unwrap();
        assert!(why.contains(label), "{why}");
        assert!(!why.contains("TEST-se"), "{why}");
    }
    assert!(crate::llm::lock(&sent).is_empty(), "nothing reached it");
    let mut transcript = Transcript::memory();
    let filing = bridge(
        &[
            ("The table showed TEST-session.", "120.3"),
            ("No trigger.", "603.2"),
        ],
        true,
    )
    .file_reports(&link(&base, true), &mut transcript, &untold())
    .await
    .unwrap();
    assert!(!filing.sent);
    assert_eq!(filing.report_ids.len(), 1, "the other one went");
    assert!(filing.why.unwrap().contains("session token"));
    let sent = crate::llm::lock(&sent).clone();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].2["client"]["report"]["observed"], "No trigger.");
}
