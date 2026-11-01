//! The seat's own game log in its bug report: the desktop client's shape,
//! nobody's name, nothing the table did not tell the seat, within the
//! bound, and each report placed in it.

use super::*;
use baylee_client_core::bugreport::{LogRow, RosterSeat, SeatLog};
use baylee_client_core::test_support::{ViewBuilder, statics};
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_protocol::v1::{self, Envelope};
use baylee_view::{LogEntry, LogEvent, LogObject, LogTail, SeatIdentity};
use prost::Message as _;

/// The players' names at the table, which no report may carry.
pub(crate) const NAMES: [&str; 2] = ["Adelheid#0a1b", "Bartholomew#77ff"];

/// The seat the bridge played.
const ME: PlayerId = PlayerId::new(1);

fn entry(turn: u32, event: LogEvent) -> LogEntry {
    LogEntry {
        at: 1_700_000_000_000,
        turn,
        repeat: 1,
        event,
        phase: None,
    }
}

fn frame(msg: v1::envelope::Msg) -> Vec<u8> {
    Envelope { msg: Some(msg) }.encode_to_vec()
}

/// What seat 1 of a two-player table was told, as its memory keeps it:
/// the roster under the players' real names, its view, and `entries` of
/// its log.
pub(crate) fn told(entries: Vec<LogEntry>) -> TableMemory {
    let mut game = statics(0);
    game.your_seat = ME;
    game.seats = NAMES
        .iter()
        .zip(0..)
        .map(|(name, seat)| SeatIdentity {
            player: PlayerId::new(seat),
            display_name: (*name).to_string(),
            is_ai: false,
            away: false,
            team: None,
        })
        .collect();
    let mut view = ViewBuilder::new(2).build();
    view.seat = ME;
    let mut memory = TableMemory::default();
    memory.hear(&frame(v1::envelope::Msg::GameStatic(v1::GameStaticMsg {
        game_id: game.game_id.clone(),
        view_version: game.view_version,
        static_json: serde_json::to_vec(&game).unwrap(),
    })));
    memory.hear(&frame(v1::envelope::Msg::StateDelta(v1::StateDelta {
        game_id: game.game_id.clone(),
        seq: 1,
        view_json: serde_json::to_vec(&view).unwrap(),
        log_json: serde_json::to_vec(&LogTail { from: 0, entries }).unwrap(),
    })));
    memory
}

/// Two turns: the other seat's, with a draw this seat may not see, and
/// this seat's, with a draw it may.
pub(crate) fn game() -> Vec<LogEntry> {
    vec![
        entry(
            1,
            LogEvent::TurnStarted {
                active: PlayerId::new(0),
            },
        ),
        entry(
            1,
            LogEvent::Drew {
                player: PlayerId::new(0),
                cards: vec![LogObject::Hidden],
            },
        ),
        entry(2, LogEvent::TurnStarted { active: ME }),
        entry(
            2,
            LogEvent::Drew {
                player: ME,
                cards: vec![LogObject::Known {
                    id: ObjectId::new(50, 0),
                    card: None,
                    token: None,
                    name: "Lightning Bolt".into(),
                }],
            },
        ),
    ]
}

/// Two reports of the game, each its own dedupe key: the first made after
/// `early` lines of the log, the second after `late`.
fn early_and_late(early: u32, late: u32) -> [Filed; 2] {
    let mut first = filed("Bolt dealt 2.", 12);
    first.log_lines = early;
    let mut second = filed_with(report("No trigger.", "603.2"), 14);
    second.log_lines = late;
    [first, second]
}

/// SHA-256, in hex, of every line's text followed by a line break: what
/// `record_sha256` is, recomputed from the log as sent.
fn sha_of(lines: &[Value]) -> String {
    use sha2::{Digest as _, Sha256};
    let mut hash = Sha256::new();
    for line in lines {
        hash.update(line["text"].as_str().unwrap().as_bytes());
        hash.update(b"\n");
    }
    crate::reports::checks::hex(&hash.finalize())
}

/// The log rides in the shape the desktop client sends it in, whole, and
/// each report says after which of its lines it was made.
#[test]
fn the_report_carries_the_seat_s_log_in_the_client_s_shape() {
    let memory = told(game());
    let log = game_log(&memory).expect("the seat was told a log");
    let body = bundle(&early_and_late(2, 4), Some(&log)).unwrap();
    let client = &body["client"];
    assert_eq!(
        keys(client),
        [
            "game_id",
            "log",
            "log_omitted",
            "model",
            "profile",
            "record_sha256",
            "refs",
            "reports",
            "seat",
            "source",
            "untrusted"
        ]
    );
    let log = &client["log"];
    assert_eq!(keys(log), ["lines", "roster", "seat"]);
    assert_eq!(log["seat"], 1);
    let roster = log["roster"].as_array().unwrap();
    assert_eq!(keys(&roster[0]), ["is_ai", "name", "seat", "team"]);
    let names: Vec<&str> = roster.iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["P1", "You"], "the names the model knew");
    let lines = log["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 4, "the whole log");
    assert_eq!(keys(&lines[0]), ["at", "text", "turn"]);
    assert_eq!(lines[3]["turn"], 2);
    assert_eq!(client["log_omitted"], 0);
    assert_eq!(client["reports"][0]["decision"]["log_line"], 2);
    assert_eq!(client["reports"][1]["decision"]["log_line"], 4);
    let text = body["text"].as_str().unwrap();
    assert!(text.contains("is in client.log (4 lines)"), "{text}");
    assert!(
        text.contains("this seat (P2 to its model) is \"You\" there and the others P1,"),
        "{text}"
    );
    assert!(
        text.contains("1. q12, turn 4, after log line 2: wrong_outcome"),
        "{text}"
    );
    assert!(
        text.contains("2. q14, turn 4, after log line 4: wrong_outcome"),
        "{text}"
    );
}

/// The seat has no record of the game, only its own log: it attaches no
/// record (the gateway attaches the game's own by `game_id`), and names
/// the game, the hash of its whole log and, for each report, the lines it
/// had been told last before it.
#[test]
fn a_report_names_the_game_and_its_log_and_attaches_no_record() {
    let log = game_log(&told(game())).expect("a log");
    let bodies = submissions(&early_and_late(2, 4), Some(&log));
    assert_eq!(bodies.len(), 2);
    let sent = bodies[0]["client"]["log"]["lines"].as_array().unwrap();
    for body in &bodies {
        assert!(
            body.get("local_record").is_none(),
            "no record of the seat's"
        );
        assert_eq!(body["game_id"], "TEST-game");
        let client = &body["client"];
        assert_eq!(client["game_id"], "TEST-game");
        assert_eq!(client["record_sha256"], json!(sha_of(sent)));
    }
    let early = bodies[0]["client"]["record_tail"].as_array().unwrap();
    assert_eq!(early.len(), 2, "the lines before the report, no later one");
    assert_eq!(keys(&early[0]), ["text", "turn"]);
    assert_eq!(early[1]["text"], sent[1]["text"]);
    let late = bodies[1]["client"]["record_tail"].as_array().unwrap();
    assert_eq!(late.len(), 4);
    assert_eq!(late[3]["text"], sent[3]["text"]);
    // Many lines: the last ones before the report, each held to its bound.
    let long = SeatLog {
        lines: (0..100)
            .map(|n| LogRow {
                turn: n,
                at: 0,
                text: format!("{n} {}", "y".repeat(1_000)),
            })
            .collect(),
        ..game_log(&told(game())).unwrap()
    };
    let [mut at, _] = early_and_late(60, 0);
    at.log_lines = 60;
    let body = &submissions(&[at], Some(&long))[0];
    let tail = body["client"]["record_tail"].as_array().unwrap();
    assert_eq!(tail.len(), RECORD_TAIL_LINES);
    assert_eq!(tail[0]["turn"], 60 - RECORD_TAIL_LINES);
    assert_eq!(tail[RECORD_TAIL_LINES - 1]["turn"], 59);
    assert_eq!(
        tail[0]["text"].as_str().unwrap().chars().count(),
        RECORD_TAIL_CHARS
    );
}

/// Nobody's name reaches the report, the other seat's hidden draw is a
/// card as the table told it, and nothing but the log's lines came from
/// the table: no view, so no hand or library either.
#[test]
fn the_log_names_nobody_and_tells_only_what_the_seat_was_told() {
    let body = submissions(&[filed("x", 1)], game_log(&told(game())).as_ref())
        .pop()
        .unwrap();
    let json = body.to_string();
    for name in NAMES {
        let bare = name.split('#').next().unwrap();
        assert!(!json.contains(bare), "{bare} reached the report");
    }
    let lines: Vec<&str> = body["client"]["log"]["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line["text"].as_str().unwrap())
        .collect();
    assert_eq!(lines[1], "P1 drew 1 card", "{lines:?}");
    assert!(lines[3].contains("Lightning Bolt"), "{lines:?}");
    for absent in ["game", "view", "hand", "pending"] {
        assert!(body["client"].get(absent).is_none(), "{absent} was sent");
    }
}

/// Before the table told the seat anything there is no log to send, and
/// the report goes without one.
#[test]
fn a_seat_told_nothing_sends_no_log() {
    assert!(game_log(&TableMemory::default()).is_none());
    assert!(game_log(&told(Vec::new())).is_none());
}

/// The lines of a log too large for a report.
const ROWS: u32 = 12_000;

/// A log of [`ROWS`] lines, too large for a report's `client`.
fn too_large() -> SeatLog {
    let seat = |seat: u8, name: &str| RosterSeat {
        seat,
        name: name.into(),
        is_ai: false,
        team: None,
    };
    let log = SeatLog {
        seat: 1,
        roster: vec![seat(0, "P1"), seat(1, "You")],
        lines: (0..ROWS)
            .map(|n| LogRow {
                turn: n / 10,
                at: 1_700_000_000_000,
                text: format!("line {n:05} {}", "x".repeat(200)),
            })
            .collect(),
    };
    assert!(serde_json::to_vec(&log).unwrap().len() > CLIENT_BYTES);
    log
}

/// The game's two reports, made after the log's fifth line and three
/// before its last, with [`too_large`]: in one bug report, and one each.
fn with_too_large() -> (Value, Vec<Value>) {
    let log = too_large();
    let reports = early_and_late(5, ROWS - 3);
    (
        bundle(&reports, Some(&log)).unwrap(),
        submissions(&reports, Some(&log)),
    )
}

/// A log that would take `client` past its bound loses its oldest lines,
/// keeps its newest, says how many went, in `client` and in the text, and
/// each report's place is counted in the lines sent; the hash is of the
/// whole log.
#[test]
fn a_log_too_large_is_cut_from_its_start_and_says_so() {
    let rows = ROWS;
    let whole: Vec<Value> = too_large()
        .lines
        .iter()
        .map(|line| json!({"text": line.text}))
        .collect();
    let (bundled, each) = with_too_large();
    let in_bundle = &bundled["client"]["reports"];
    // Each bug report, and the early report's and the late one's in it.
    let cases = [
        (&bundled, Some(&in_bundle[0]), Some(&in_bundle[1])),
        (&each[0], Some(&each[0]["client"]["report"]), None),
        (&each[1], None, Some(&each[1]["client"]["report"])),
    ];
    for (body, early, late) in cases {
        let client = &body["client"];
        let weight = serde_json::to_vec(client).unwrap().len();
        assert!(weight <= CLIENT_BYTES, "{weight}");
        assert!(
            weight > CLIENT_BYTES - LOG_RESERVE - 1_000,
            "as much of the log as fits is kept: {weight}"
        );
        assert_eq!(client["record_sha256"], json!(sha_of(&whole)));
        let omitted = usize::try_from(client["log_omitted"].as_u64().unwrap()).unwrap();
        let lines = client["log"]["lines"].as_array().unwrap();
        assert!(omitted > 0 && !lines.is_empty());
        assert_eq!(lines.len() + omitted, rows as usize);
        assert!(
            lines.last().unwrap()["text"]
                .as_str()
                .unwrap()
                .starts_with("line 11999"),
            "the newest line is kept"
        );
        let late_line = rows as usize - 3 - omitted;
        let text = body["text"].as_str().unwrap();
        assert!(
            text.contains(&format!(
                "its last {} of {rows} lines (the first {omitted} left out",
                lines.len()
            )),
            "{text}"
        );
        if let Some(early) = early {
            assert_eq!(early["decision"]["log_line"], 0);
            assert!(
                text.contains("q12, turn 4, before the log's first line: "),
                "{text}"
            );
        }
        if let Some(late) = late {
            assert_eq!(late["decision"]["log_line"], late_line);
            assert!(
                text.contains(&format!("q14, turn 4, after log line {late_line}: ")),
                "{text}"
            );
        }
    }
}

/// Each report's tail is of the whole log, whatever was left out of the
/// log sent with it, in one bug report or one each.
#[test]
fn a_report_s_tail_is_of_the_whole_log() {
    let (bundled, each) = with_too_large();
    let in_bundle = &bundled["client"]["reports"];
    let early_tail = in_bundle[0]["record_tail"].as_array().unwrap();
    assert_eq!(early_tail.len(), 5, "the whole log's first five lines");
    assert!(
        early_tail[0]["text"]
            .as_str()
            .unwrap()
            .starts_with("line 00000")
    );
    assert_eq!(
        each[0]["client"]["record_tail"],
        in_bundle[0]["record_tail"]
    );
    assert_eq!(
        in_bundle[1]["record_tail"].as_array().unwrap().len(),
        RECORD_TAIL_LINES
    );
    assert_eq!(
        each[1]["client"]["record_tail"],
        in_bundle[1]["record_tail"]
    );
}
