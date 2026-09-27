//! A bug report end to end (#307, #308, #315): two players play a game
//! through a real gateway to its end, and their reports go from the client's
//! own report builder through the gateway to a real feedback service with a
//! database schema and a port of its own. What the service keeps is then
//! read back through its read API, the way whoever reads reports reads them,
//! and the game record it holds is played again on a fresh engine.
//!
//! `e2e_reports.rs` holds the gateway's side against a stub service; this
//! holds that the three agree.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use std::io::Read as _;
use std::sync::Arc;

use baylee_client_core::bugreport::{
    self, Build, Consent, CrashFile, CrashRecord, Gathered, Holding, Screenshot, Secret, Settings,
    System, Unsendable,
};
use baylee_engine::choice::{Pending, PlayerAction, timeout_answer};
use baylee_protocol::v1::{self, Envelope};
use common::{Gateway, Socket, attach_agent, http, http_bytes, json_field, login};
use futures_util::SinkExt;
use prost::Message;
use sea_orm::ConnectionTrait as _;

const INTAKE: &str = "intake-token-for-the-e2e";
const READ: &str = "read-token-for-the-e2e-0";
const GATEWAY_NAME: &str = "Baylee Test";
const PUBLIC_URL: &str = "https://gateway.example";

/// A feedback service in a schema of its own, dropped with it.
struct Service {
    port: u16,
    schema: String,
    server: tokio::task::JoinHandle<()>,
}

impl Service {
    async fn start() -> Self {
        let base = common::database_url();
        let schema = format!("fb_e2e_{}", uuid::Uuid::now_v7().simple());
        let admin = sea_orm::Database::connect(&base).await.expect("connecting");
        admin
            .execute_unprepared(&format!("CREATE SCHEMA \"{schema}\""))
            .await
            .expect("a schema");
        let sep = if base.contains('?') { '&' } else { '?' };
        let scoped = format!("{base}{sep}options=-c%20search_path%3D{schema}");
        let db = baylee_feedback::connect(&scoped, 2)
            .await
            .expect("the service migrates");
        let config = baylee_feedback::Config::new(&format!("e2e={INTAKE}"), Some(READ), None)
            .expect("config");
        let app = baylee_feedback::app(Arc::new(baylee_feedback::AppState { db, config }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Self {
            port,
            schema,
            server,
        }
    }

    /// A report as the read API shows it, `client` included.
    fn report(&self, id: &str) -> serde_json::Value {
        let (status, body) = http(self.port, "GET", &format!("/reports/{id}"), Some(READ), "");
        assert_eq!(status, 200, "{body}");
        serde_json::from_str(&body).expect("a report")
    }

    /// The record a report holds, unpacked.
    fn record(&self, id: &str) -> Vec<u8> {
        let (status, gz) = http_bytes(
            self.port,
            "GET",
            &format!("/reports/{id}/record"),
            Some(READ),
            "application/json",
            b"",
        );
        assert_eq!(status, 200);
        let mut record = Vec::new();
        flate2::read::MultiGzDecoder::new(&gz[..])
            .read_to_end(&mut record)
            .expect("one gzip stream");
        record
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        self.server.abort();
        let schema = self.schema.clone();
        let _ = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("a runtime")
                .block_on(async move {
                    if let Ok(db) = sea_orm::Database::connect(&common::database_url()).await {
                        let _ = db
                            .execute_unprepared(&format!("DROP SCHEMA \"{schema}\" CASCADE"))
                            .await;
                    }
                });
        })
        .join();
    }
}

/// Two accounts, a deck each, a room the first opens and the second joins,
/// started: the game's id, and each player's session and seat token.
fn two_players(port: u16) -> (String, [(String, String); 2]) {
    let sessions = [
        login(port, "alma", "AlmaPlayer"),
        login(port, "bert", "BertPlayer"),
    ];
    let mut decks = Vec::new();
    for (i, token) in sessions.iter().enumerate() {
        let deck = format!(r#"{{"name":"d{i}","cards":["60 Swamp"]}}"#);
        let (status, body) = http(port, "POST", "/decks", Some(token), &deck);
        assert_eq!(status, 200, "deck: {body}");
        decks.push(json_field(&body, "deck_id").to_string());
    }
    let create = format!(r#"{{"deck_id":"{}","mode":"open"}}"#, decks[0]);
    let (status, body) = http(port, "POST", "/lobby/games", Some(&sessions[0]), &create);
    assert_eq!(status, 200, "open: {body}");
    let game = json_field(&body, "game_id").to_string();
    let seat_a = json_field(&body, "seat_token").to_string();
    let join = format!(r#"{{"deck_id":"{}"}}"#, decks[1]);
    let path = format!("/lobby/games/{game}/join");
    let (status, body) = http(port, "POST", &path, Some(&sessions[1]), &join);
    assert_eq!(status, 200, "join: {body}");
    let seat_b = json_field(&body, "seat_token").to_string();
    for token in &sessions {
        let path = format!("/lobby/games/{game}/ready");
        assert_eq!(http(port, "POST", &path, Some(token), "{}").0, 200);
    }
    let path = format!("/lobby/games/{game}/start");
    assert_eq!(http(port, "POST", &path, Some(&sessions[0]), "").0, 200);
    let [a, b] = sessions;
    (game, [(a, seat_a), (b, seat_b)])
}

/// Reads a seat until it falls quiet or closes; the last view it was shown.
/// Not one piece of the record may reach it (#315).
async fn until_quiet(ws: &mut Socket) -> Option<baylee_view::PlayerView> {
    let mut view = None;
    while let Ok(Some(msg)) =
        tokio::time::timeout(std::time::Duration::from_secs(2), common::next_msg(ws)).await
    {
        match msg {
            v1::envelope::Msg::GameRecordChunk(_) => panic!("a seat was sent the game's record"),
            v1::envelope::Msg::StateDelta(d) => view = serde_json::from_slice(&d.view_json).ok(),
            _ => {}
        }
    }
    view
}

/// Answers what each seat is asked, as its decision clock would, until each
/// has answered `answers` questions. The first seat's last view and last
/// question.
async fn play(
    a: &mut Socket,
    b: &mut Socket,
    answers: usize,
) -> (Option<baylee_view::PlayerView>, Option<Pending>) {
    let mut view = None;
    let mut asked = None;
    let mut answered = [0_usize; 2];
    let deadline = tokio::time::Instant::now() + common::WAIT_BUDGET;
    while answered.iter().any(|&n| n < answers) {
        assert!(
            tokio::time::Instant::now() < deadline,
            "the seats stopped being asked ({answered:?} answers)"
        );
        for (seat, ws) in [&mut *a, &mut *b].into_iter().enumerate() {
            let Ok(Some(msg)) =
                tokio::time::timeout(std::time::Duration::from_millis(50), common::next_msg(ws))
                    .await
            else {
                continue;
            };
            match msg {
                v1::envelope::Msg::GameRecordChunk(_) => {
                    panic!("a seat was sent the game's record")
                }
                v1::envelope::Msg::StateDelta(d) if seat == 0 => {
                    view = serde_json::from_slice(&d.view_json).ok();
                }
                v1::envelope::Msg::ChoiceRequest(c) => {
                    let pending: Pending =
                        serde_json::from_slice(&c.pending_json).expect("a question");
                    let answer = timeout_answer(&pending)
                        .unwrap_or_else(|| panic!("no plain answer to {pending:?}"));
                    if seat == 0 {
                        asked = Some(pending);
                    }
                    common::send(
                        ws,
                        &Envelope {
                            msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
                                game_id: String::new(),
                                seat_token: String::new(),
                                action_json: serde_json::to_vec(&answer).unwrap(),
                            })),
                        },
                    )
                    .await;
                    answered[seat] += 1;
                }
                _ => {}
            }
        }
    }
    // Answers travel on two sockets, so the last of one seat's can still be
    // on its way when the other seat concedes; read, answering nothing,
    // until both fall quiet, which is after the engine has applied every
    // answer and asked its next question.
    loop {
        let mut quiet = true;
        for (seat, ws) in [&mut *a, &mut *b].into_iter().enumerate() {
            let Ok(Some(msg)) =
                tokio::time::timeout(std::time::Duration::from_millis(500), common::next_msg(ws))
                    .await
            else {
                continue;
            };
            quiet = false;
            match msg {
                v1::envelope::Msg::GameRecordChunk(_) => {
                    panic!("a seat was sent the game's record")
                }
                v1::envelope::Msg::StateDelta(d) if seat == 0 => {
                    view = serde_json::from_slice(&d.view_json).ok();
                }
                v1::envelope::Msg::ChoiceRequest(c) if seat == 0 => {
                    asked = serde_json::from_slice(&c.pending_json).ok();
                }
                _ => {}
            }
        }
        if quiet {
            break;
        }
    }
    (view, asked)
}

async fn stored(gw: &Gateway, game: &str) {
    for _ in 0..common::WAIT_TRIES {
        let done = gw.scalar(&format!(
            "SELECT count(*) FROM game_record WHERE game_id = '{game}' AND complete"
        ));
        if done == 1 {
            return;
        }
        tokio::time::sleep(common::WAIT_STEP).await;
    }
    panic!("the game's record was never completed");
}

/// Posts a report body; the service's id for it.
fn send(port: u16, session: &str, body: &str) -> String {
    let (status, answer) = http(port, "POST", "/reports", Some(session), body);
    assert_eq!(status, 201, "{answer}");
    json_field(&answer, "report_id").to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[allow(clippy::too_many_lines)] // one scenario: a game, four reports, read back
async fn a_report_goes_from_the_client_through_the_gateway_to_the_service() {
    let service = Service::start().await;
    let gw = common::spawn_gateway_with(
        "feedback-e2e",
        &[
            (
                "BAYLEE_FEEDBACK_URL",
                format!("http://127.0.0.1:{}/", service.port),
            ),
            ("BAYLEE_FEEDBACK_TOKEN", INTAKE.to_owned()),
            ("BAYLEE_FEEDBACK_KEY", "the-e2e-pseudonym-key".to_owned()),
            ("BAYLEE_GATEWAY_NAME", GATEWAY_NAME.to_owned()),
            ("BAYLEE_PUBLIC_URL", PUBLIC_URL.to_owned()),
        ],
    );
    let port = gw.port;
    let agent = attach_agent(&gw).await;

    // Two players, and a game played to its end: the first concedes.
    let (game, [(alma, alma_seat), (bert, bert_seat)]) = two_players(port);
    let mut ws_a = common::dial_seat(port, &game, &alma_seat).await;
    let mut ws_b = common::dial_seat(port, &game, &bert_seat).await;
    // Both play a while once the table opens (no question comes before),
    // so the record holds both seats' answers; the first then concedes.
    let (mut view_a, asked_a) = play(&mut ws_a, &mut ws_b, 8).await;
    let concede = Envelope {
        msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
            game_id: String::new(),
            seat_token: String::new(),
            action_json: serde_json::to_vec(&PlayerAction::Concede).unwrap(),
        })),
    };
    ws_a.send(tokio_tungstenite::tungstenite::Message::Binary(
        concede.encode_to_vec().into(),
    ))
    .await
    .expect("send the concession");
    view_a = until_quiet(&mut ws_a).await.or(view_a);
    until_quiet(&mut ws_b).await;
    stored(&gw, &game).await;
    let view_a = view_a.expect("the first seat was shown the table");

    // The first player's report, as the client builds it: every part ticked,
    // the table as that seat was shown it, sealed against both of its
    // secrets.
    let gathered = Gathered {
        build: Build {
            version: "0.0.0-e2e".into(),
            commit: Some("abcdef0".into()),
        },
        game_id: Some(game.clone()),
        system: Some(System {
            platform: "test/e2e".into(),
            cpus: Some(4),
            adapter: Some("Test Adapter".into()),
            backend: Some("None".into()),
            window: (1280, 720),
            scale: 2.0,
            lang: "de".into(),
        }),
        game: Some(bugreport::Game {
            table: bugreport::Table {
                seat: 0,
                seq: 1,
                when: "Turn 1".into(),
            },
            view: view_a,
            pending: asked_a,
            holding: Holding::default(),
        }),
        log: None,
        settings: Some(Settings {
            lang: "de".into(),
            saved_gateways: 2,
            ..Settings::default()
        }),
        screenshot: Some(Screenshot {
            width: 1,
            height: 1,
            png_base64: bugreport::base64_encode(b"\x89PNG\r\n\x1a\nnot really"),
        }),
    };
    let secrets = [
        Secret {
            label: "session token",
            value: &alma,
        },
        Secret {
            label: "seat token",
            value: &alma_seat,
        },
    ];
    let consent = Consent::everything();
    // The client's half of "never a session": a report carrying one is not
    // sent at all.
    assert!(matches!(
        gathered
            .submission(
                bugreport::Kind::Bug,
                &format!("my token is {alma}"),
                &consent
            )
            .sealed(&secrets),
        Err(Unsendable::Leaked(_))
    ));
    let (body, trimmed) = gathered
        .submission(
            bugreport::Kind::Bug,
            "the concession took a while",
            &consent,
        )
        .sealed(&secrets)
        .expect("the report seals");
    assert_eq!(trimmed, bugreport::Trimmed::default());
    let sent: serde_json::Value = serde_json::from_str(&body).unwrap();
    let with_game = send(port, &alma, &body);

    // A crash report, as the client builds one from its crash file: no game.
    let crash = bugreport::crash_submission(
        &CrashFile {
            gateway: Some(format!("http://127.0.0.1:{port}")),
            build: Build {
                version: "0.0.0-e2e".into(),
                commit: None,
            },
            record: CrashRecord {
                message: "index out of bounds\nsecond line".into(),
                location: Some("src/x.rs:1:2".into()),
                platform: "test/e2e".into(),
                ..CrashRecord::default()
            },
        },
        None,
    );
    let (crash_body, _) = crash.sealed(&secrets).expect("the crash seals");
    let crashed = send(port, &alma, &crash_body);

    // The second player names the same game; a third account, who did not
    // sit there, names it too; and one report says "no game" as an empty id.
    let bert_report = send(
        port,
        &bert,
        &serde_json::json!({
            "kind": "feedback", "text": "good game", "game_id": game, "client": {},
        })
        .to_string(),
    );
    let stranger = login(port, "cleo", "CleoStranger");
    let strangers = send(
        port,
        &stranger,
        &serde_json::json!({
            "kind": "other", "text": "that game", "game_id": game, "client": {},
        })
        .to_string(),
    );
    let blank = send(
        port,
        &stranger,
        r#"{"kind":"improvement","text":"","game_id":"","client":{"a":1}}"#,
    );

    // What the service keeps, as its reader sees it.
    let first = service.report(&with_game);
    assert_eq!(first["kind"], "bug");
    assert_eq!(first["text"], "the concession took a while");
    assert_eq!(first["game_id"], game.as_str());
    assert_eq!(
        first["client"], sent["client"],
        "the client object arrives as the client sealed it"
    );
    assert_eq!(first["client"]["system"]["adapter"], "Test Adapter");
    assert_eq!(first["client"]["game"]["table"]["seat"], 0);
    assert_eq!(first["gateway"], "e2e", "filed under the token's name");
    assert_eq!(first["gateway_name"], GATEWAY_NAME);
    assert_eq!(first["gateway_url"], PUBLIC_URL);
    assert_eq!(first["gateway_version"], baylee_build::short());
    assert_eq!(first["has_record"], true);
    assert_eq!(first["record_complete"], true);
    assert_eq!(
        first["record_bytes"].as_i64(),
        Some(gw.scalar(&format!(
            "SELECT bytes FROM game_record WHERE game_id = '{game}'"
        ))),
        "the record the gateway stored, whole"
    );

    let crash = service.report(&crashed);
    assert_eq!(crash["kind"], "crash");
    assert_eq!(crash["text"], "index out of bounds");
    assert_eq!(crash["client"]["crash"]["location"], "src/x.rs:1:2");
    assert_eq!(crash["game_id"], serde_json::Value::Null);
    assert_eq!(crash["has_record"], false);

    let second = service.report(&bert_report);
    assert_eq!(second["has_record"], true, "the other player sat there too");
    let strange = service.report(&strangers);
    assert_eq!(strange["game_id"], game.as_str());
    assert_eq!(
        strange["has_record"], false,
        "naming a game one did not sit at attaches nothing"
    );
    let blank = service.report(&blank);
    assert_eq!(blank["game_id"], serde_json::Value::Null);
    assert_eq!(blank["has_record"], false);

    // One pseudonym per account, a different one per account, and nothing
    // in any report that names an account.
    let reporter = |r: &serde_json::Value| r["reporter"].as_str().unwrap().to_owned();
    assert_eq!(reporter(&first), reporter(&crash));
    assert_eq!(reporter(&strange), reporter(&blank));
    assert_ne!(reporter(&first), reporter(&second));
    assert_ne!(reporter(&first), reporter(&strange));
    assert_ne!(reporter(&second), reporter(&strange));
    for r in [&first, &crash, &second, &strange, &blank] {
        let p = reporter(r);
        assert_eq!(p.len(), 64);
        assert!(p.chars().all(|c| c.is_ascii_hexdigit()));
    }
    let mut private: Vec<String> = ["alma", "bert", "cleo"]
        .iter()
        .map(|u| {
            gw.text(&format!(
                "SELECT id::text FROM account WHERE username = '{u}'"
            ))
        })
        .collect();
    // Each id also as bare hex, the one spelling of it the service's
    // reporter field would take.
    let bare: Vec<String> = private.iter().map(|id| id.replace('-', "")).collect();
    private.extend(bare);
    private.extend(
        [
            "AlmaPlayer",
            "BertPlayer",
            "CleoStranger",
            &alma,
            &bert,
            &stranger,
            &alma_seat,
            &bert_seat,
        ]
        .map(str::to_owned),
    );
    for r in [&first, &crash, &second, &strange, &blank] {
        let raw = r.to_string();
        for p in &private {
            assert!(!raw.contains(p.as_str()), "a report carried {p:?}");
        }
    }

    // The record replays on a fresh engine, hash for hash, to the end, and
    // names nobody either.
    let record = service.record(&with_game);
    assert_eq!(record, service.record(&bert_report), "one game, one record");
    let replayed = baylee_gamehost::record::replay(&record).expect("the record replays");
    assert!(replayed.ended, "to the end");
    let lines: Vec<serde_json::Value> = record
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_slice(l).expect("a record line"))
        .collect();
    let inputs: Vec<&serde_json::Value> = lines.iter().filter(|l| l["kind"] == "input").collect();
    assert_eq!(usize::try_from(replayed.inputs).unwrap(), inputs.len());
    assert_eq!(inputs.last().unwrap()["action"], "Concede");
    assert_eq!(inputs.last().unwrap()["seat"], 0);
    let by_seat: Vec<(&serde_json::Value, &serde_json::Value)> =
        inputs.iter().map(|l| (&l["seat"], &l["by"])).collect();
    // Each seat answered eight questions; an answer the engine had moved
    // past by the time it arrived (the two sockets race) is not an input,
    // so half of them is what the record is held to.
    for seat in [0, 1] {
        assert!(
            inputs
                .iter()
                .filter(|l| l["seat"] == seat && l["by"] == "seat")
                .count()
                >= 4,
            "seat {seat}'s answers are in the record: {by_seat:?}"
        );
    }
    assert_eq!(
        inputs.last().unwrap()["hash"],
        format!("{:016x}", replayed.engine.snapshot_hash()),
        "the last recorded hash is the one the replay reached"
    );
    let text = String::from_utf8(record).unwrap();
    for p in &private {
        assert!(!text.contains(p.as_str()), "the record carried {p:?}");
    }

    agent.abort();
}
