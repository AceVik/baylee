//! The anonymised export of game records for training and balancing
//! (`baylee-gateway records export`, `docs/privacy.md` §"Game records and
//! reports"): a game played over the real sockets is exported with
//! nothing in it that ties it to the account that played it — no account
//! id, username, display name, handle, game id, token or time — and still
//! replays, seats, decks and all.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use std::io::Read as _;

use baylee_engine::choice::PlayerAction;
use baylee_protocol::v1::{self, Envelope};
use common::{Gateway, Socket, attach_agent, http, json_field, login, spawn_gateway};
use futures_util::SinkExt;
use prost::Message;

/// Plays a game against the house to its end, by conceding once the table
/// is open. Its id and the seat's token.
async fn a_finished_game(port: u16, token: &str) -> (String, String) {
    let body = r#"{"name":"swamps","cards":["60 Swamp"]}"#;
    let (status, body) = http(port, "POST", "/decks", Some(token), body);
    assert_eq!(status, 200, "create deck: {body}");
    let deck_id = json_field(&body, "deck_id").to_string();
    let create = format!("{{\"deck_id\":\"{deck_id}\",\"mode\":\"ai\"}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(token), &create);
    assert_eq!(status, 200, "create ai game: {body}");
    let game = json_field(&body, "game_id").to_string();
    let seat_token = json_field(&body, "seat_token").to_string();
    let mut ws: Socket = common::dial_seat(port, &game, &seat_token).await;
    tokio::time::timeout(common::WAIT_BUDGET, async {
        loop {
            match common::next_msg(&mut ws).await {
                Some(v1::envelope::Msg::GameStatic(_)) => {
                    let ready = Envelope {
                        msg: Some(v1::envelope::Msg::SeatReady(v1::SeatReady {})),
                    };
                    common::send(&mut ws, &ready).await;
                }
                Some(v1::envelope::Msg::Curtain(_)) => break,
                Some(_) => {}
                None => panic!("the socket closed before play was allowed"),
            }
        }
    })
    .await
    .expect("the table opened");
    let concede = Envelope {
        msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
            game_id: String::new(),
            seat_token: String::new(),
            action_json: serde_json::to_vec(&PlayerAction::Concede).unwrap(),
        })),
    };
    ws.send(tokio_tungstenite::tungstenite::Message::Binary(
        concede.encode_to_vec().into(),
    ))
    .await
    .expect("send the concession");
    (game, seat_token)
}

/// Waits until the game's record is complete in the store.
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

/// `baylee-gateway records export --out <dir>` against this gateway's
/// schema: what it printed.
fn export(gw: &Gateway, out: &std::path::Path) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_baylee-gateway"))
        .args(["records", "export", "--out"])
        .arg(out)
        .env("DATABASE_URL", gw.database_url())
        .output()
        .expect("the command runs")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_exported_record_names_no_account_and_still_replays() {
    let gw = spawn_gateway("record-export");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let session = login(port, "exporter", "RheaExporter");
    let (game, seat_token) = a_finished_game(port, &session).await;
    stored(&gw, &game).await;
    let account_id = gw.text("SELECT id::text FROM account WHERE username = 'exporter'");

    let out = std::env::temp_dir().join(format!("baylee-export-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let said = export(&gw, &out);
    assert!(said.status.success(), "{said:?}");
    assert_eq!(
        String::from_utf8_lossy(&said.stdout).trim(),
        "exported 1 records, refused 0"
    );
    let files: Vec<_> = std::fs::read_dir(&out)
        .expect("the export")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(files, ["000001.jsonl.gz"], "named by a count alone");
    let mut text = String::new();
    flate2::read::MultiGzDecoder::new(
        std::fs::read(out.join("000001.jsonl.gz"))
            .expect("read")
            .as_slice(),
    )
    .read_to_string(&mut text)
    .expect("one gzip of text");

    // Nothing that names the account, the game or the time.
    let lower = text.to_lowercase();
    for (what, needle) in [
        ("the account's id", account_id.as_str()),
        ("the game's id", game.as_str()),
        ("the seat token", seat_token.as_str()),
        ("the session", session.as_str()),
        ("the username", "exporter"),
        ("the display name", "rheaexporter"),
    ] {
        assert!(
            !lower.contains(&needle.to_lowercase()),
            "{what} is in the export"
        );
    }
    // The handle is the display name and a tag: with the name gone, so is
    // the handle.
    let lines: Vec<serde_json::Value> = text
        .lines()
        .map(|l| serde_json::from_str(l).expect("a line"))
        .collect();
    for line in &lines {
        if let Some(at) = line.get("at") {
            assert_eq!(at, 0, "a time survived: {line}");
        }
    }
    // What training reads is all there.
    assert_eq!(lines[0]["kind"], "header");
    assert_eq!(lines.last().expect("lines")["kind"], "end");
    let seats = lines[0]["preset"]["seats"].as_array().expect("seats");
    assert_eq!(seats.len(), 2);
    assert_eq!(seats[0]["deck"].as_array().map(Vec::len), Some(60));
    let replayed = baylee_gamehost::record::replay(text.as_bytes());
    assert!(replayed.is_ok(), "the export no longer replays");

    // A second export into the same place is refused, not numbered over.
    let again = export(&gw, &out);
    assert!(!again.status.success(), "{again:?}");
    let _ = std::fs::remove_dir_all(&out);
}
