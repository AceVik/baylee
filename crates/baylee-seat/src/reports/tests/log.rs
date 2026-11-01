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

/// The keys of a JSON object, sorted.
fn keys(value: &Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

/// The log rides in the shape the desktop client sends it in, whole, and
/// each report says after which of its lines it was made.
#[test]
fn the_report_carries_the_seat_s_log_in_the_client_s_shape() {
    let memory = told(game());
    let log = game_log(&memory).expect("the seat was told a log");
    let mut early = filed("Bolt dealt 2.", 12);
    early.log_lines = 2;
    let mut late = filed("No trigger.", 14);
    late.log_lines = 4;
    let body = submission(&[early, late], Some(log)).unwrap();
    let client = &body["client"];
    assert_eq!(
        keys(client),
        [
            "log",
            "log_omitted",
            "model",
            "profile",
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
    assert_eq!(client["reports"][0]["log_line"], 2);
    assert_eq!(client["reports"][1]["log_line"], 4);
    let text = body["text"].as_str().unwrap();
    assert!(text.contains("is in client.log (4 lines)"), "{text}");
    assert!(
        text.contains("this seat (P2 to its model) is \"You\" there and the others P1,"),
        "{text}"
    );
    assert!(
        text.contains("1. q12, turn 4, after log line 2: Bolt dealt 2."),
        "{text}"
    );
    assert!(
        text.contains("2. q14, turn 4, after log line 4: No trigger."),
        "{text}"
    );
}

/// Nobody's name reaches the report, the other seat's hidden draw is a
/// card as the table told it, and nothing but the log's lines came from
/// the table: no view, so no hand or library either.
#[test]
fn the_log_names_nobody_and_tells_only_what_the_seat_was_told() {
    let body = submission(&[filed("x", 1)], game_log(&told(game()))).unwrap();
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

/// A log that would take `client` past its bound loses its oldest lines,
/// keeps its newest, says how many went, in `client` and in the text, and
/// each report's place is counted in the lines sent.
#[test]
fn a_log_too_large_is_cut_from_its_start_and_says_so() {
    let rows: u32 = 12_000;
    let log = SeatLog {
        seat: 1,
        roster: vec![
            RosterSeat {
                seat: 0,
                name: "P1".into(),
                is_ai: false,
                team: None,
            },
            RosterSeat {
                seat: 1,
                name: "You".into(),
                is_ai: false,
                team: None,
            },
        ],
        lines: (0..rows)
            .map(|n| LogRow {
                turn: n / 10,
                at: 1_700_000_000_000,
                text: format!("line {n:05} {}", "x".repeat(200)),
            })
            .collect(),
    };
    assert!(serde_json::to_vec(&log).unwrap().len() > CLIENT_BYTES);
    let mut early = filed("Early.", 3);
    early.log_lines = 5;
    let mut late = filed("Late.", 9);
    late.log_lines = rows - 3;
    let body = submission(&[early, late], Some(log)).unwrap();
    let client = &body["client"];
    let weight = serde_json::to_vec(client).unwrap().len();
    assert!(weight <= CLIENT_BYTES, "{weight}");
    assert!(
        weight > CLIENT_BYTES - LOG_RESERVE - 1_000,
        "as much of the log as fits is kept: {weight}"
    );
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
    assert_eq!(client["reports"][0]["log_line"], 0);
    let late_line = rows as usize - 3 - omitted;
    assert_eq!(client["reports"][1]["log_line"], late_line);
    let text = body["text"].as_str().unwrap();
    assert!(
        text.contains(&format!(
            "its last {} of {rows} lines (the first {omitted} left out",
            lines.len()
        )),
        "{text}"
    );
    assert!(
        text.contains("1. q3, turn 4, before the log's first line: Early."),
        "{text}"
    );
    assert!(
        text.contains(&format!("2. q9, turn 4, after log line {late_line}: Late.")),
        "{text}"
    );
}
