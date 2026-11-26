//! A running game survives a restart of its gateway: stopped as a deploy
//! stops it, or killed, the gateway starts again on the same database, a new
//! engine picks the game up from its stored record, and the seat that dials
//! back is asked what it was asked (`docs/proposals/games-survive-restarts.md`
//! §"Step 3b").
//!
//! Every check is made against the game as it stood before the restart, not
//! against "a game": a gateway that quietly started a new game from the same
//! preset would also ask its seat a question.

#![allow(clippy::missing_docs_in_private_items)]

use crate::common;

use std::io::Read as _;

use baylee_engine::choice::{Pending, PlayerAction, timeout_answer};
use baylee_protocol::v1::{self, Envelope};
use common::{EngineProbe, Gateway, Socket, http, json_field, login, spawn_gateway_with};

fn deck(port: u16, token: &str) -> String {
    let body = r#"{"name":"swamps","cards":["60 Swamp"]}"#;
    let (status, body) = http(port, "POST", "/decks", Some(token), body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_string()
}

/// A game against the house, its table open: its id, the seat's token and
/// socket, and the question it is asked, which may come before the curtain
/// rises.
async fn a_game(port: u16, token: &str) -> (String, String, Socket, Vec<u8>) {
    let deck_id = deck(port, token);
    let create = format!("{{\"deck_id\":\"{deck_id}\",\"mode\":\"ai\"}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(token), &create);
    assert_eq!(status, 200, "create ai game: {body}");
    let game = json_field(&body, "game_id").to_string();
    let seat_token = json_field(&body, "seat_token").to_string();
    let mut ws = common::dial_seat(port, &game, &seat_token).await;
    let mut asked = None;
    tokio::time::timeout(common::WAIT_BUDGET, async {
        loop {
            match common::next_msg(&mut ws).await {
                Some(v1::envelope::Msg::GameStatic(_)) => {
                    common::send(
                        &mut ws,
                        &Envelope {
                            msg: Some(v1::envelope::Msg::SeatReady(v1::SeatReady {})),
                        },
                    )
                    .await;
                }
                Some(v1::envelope::Msg::Curtain(_)) => break,
                Some(v1::envelope::Msg::ChoiceRequest(c)) => asked = Some(c.pending),
                Some(_) => {}
                None => panic!("the socket closed before play was allowed"),
            }
        }
    })
    .await
    .expect("the table opened");
    let asked = last_question(&mut ws)
        .await
        .or(asked)
        .expect("the seat is asked");
    (game, seat_token, ws, asked)
}

/// The last question the socket was asked once it falls quiet, as the
/// engine encoded it.
async fn last_question(ws: &mut Socket) -> Option<Vec<u8>> {
    let mut asked = None;
    while let Ok(Some(msg)) =
        tokio::time::timeout(std::time::Duration::from_millis(700), common::next_msg(ws)).await
    {
        if let v1::envelope::Msg::ChoiceRequest(c) = msg {
            asked = Some(c.pending);
        }
    }
    asked
}

/// The first question a socket is asked, within the budget.
async fn first_question(ws: &mut Socket) -> Vec<u8> {
    tokio::time::timeout(common::WAIT_BUDGET, async {
        loop {
            match common::next_msg(ws).await {
                Some(v1::envelope::Msg::ChoiceRequest(c)) => return c.pending,
                Some(_) => {}
                None => panic!("the seat's socket closed before it was asked"),
            }
        }
    })
    .await
    .expect("the seat is asked")
}

async fn say(ws: &mut Socket, action: &PlayerAction) {
    common::send(
        ws,
        &Envelope {
            msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
                game_id: String::new(),
                seat_token: String::new(),
                action: baylee_core::wire::encode(action),
            })),
        },
    )
    .await;
}

/// Answers `asked` as a timeout would and waits until the engine took it:
/// the hash it moved to.
async fn answer(ws: &mut Socket, asked: &[u8], probe: &EngineProbe) -> u64 {
    let before = probe.hash.borrow().expect("the game is built");
    let pending: Pending = baylee_core::wire::decode(asked).expect("a question");
    let action = timeout_answer(&pending).expect("the question has a plain answer");
    say(ws, &action).await;
    moved_from(probe, before).await
}

async fn moved_from(probe: &EngineProbe, before: u64) -> u64 {
    tokio::time::timeout(common::WAIT_BUDGET, async {
        loop {
            if let Some(now) = *probe.hash.borrow()
                && now != before
            {
                return now;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("the engine took the answer")
}

/// Waits until the engine `probe` watches holds the game at `hash`.
async fn at(probe: &EngineProbe, hash: u64) {
    tokio::time::timeout(common::WAIT_BUDGET, async {
        while *probe.hash.borrow() != Some(hash) {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!(
            "the resumed engine never reached the game as it was ({hash:x}); it is at {:?}",
            *probe.hash.borrow()
        )
    });
}

/// The record as stored, unpacked.
fn stored_record(gw: &Gateway, game: &str) -> Vec<u8> {
    let packed = gw.text(&format!(
        "SELECT coalesce(encode(string_agg(data, ''::bytea ORDER BY seq), 'hex'), '') \
         FROM game_record_chunk WHERE game_id = '{game}'"
    ));
    let gz = (0..packed.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&packed[at..at + 2], 16).unwrap())
        .collect::<Vec<u8>>();
    let mut record = Vec::new();
    flate2::read::MultiGzDecoder::new(&gz[..])
        .read_to_end(&mut record)
        .expect("one gzip stream");
    record
}

/// The game a record replays to, and whether it is over.
fn replayed(record: &[u8]) -> (u64, bool) {
    let replayed = baylee_gamehost::record::replay_exact_with_detection::<
        baylee_engine::engine::NoLoopDetection,
    >(record)
    .expect("the record replays");
    (replayed.engine.snapshot_hash(), replayed.ended)
}

/// Waits until the stored record holds the game at `hash`: the engine sent
/// it on its own bound, as a killed gateway leaves it.
async fn stored_at(gw: &Gateway, game: &str, hash: u64) {
    for _ in 0..common::WAIT_TRIES {
        if !stored_record(gw, game).is_empty() && replayed(&stored_record(gw, game)).0 == hash {
            return;
        }
        tokio::time::sleep(common::WAIT_STEP).await;
    }
    panic!("the record never reached the game's last answer");
}

fn kept(gw: &Gateway, game: &str) -> i64 {
    gw.scalar(&format!(
        "SELECT count(*) FROM running_game WHERE game_id = '{game}'"
    ))
}

async fn kept_soon(gw: &Gateway, game: &str) {
    for _ in 0..common::WAIT_TRIES {
        if kept(gw, game) == 1 {
            return;
        }
        tokio::time::sleep(common::WAIT_STEP).await;
    }
    panic!("the running game was never kept");
}

/// The gateway, stopped as systemd stops it, from a blocking thread.
async fn stopped(gw: Gateway) -> Gateway {
    tokio::task::spawn_blocking(move || {
        let mut gw = gw;
        assert!(gw.stop(), "a stopped gateway exits cleanly");
        gw
    })
    .await
    .expect("the stop returns")
}

/// The gateway started again, the player's session known to it again (a
/// client's lobby redials first), and an agent there: the engine it is
/// ordered resumes the game.
async fn started_again(
    gw: &mut Gateway,
    token: &str,
) -> (tokio::task::JoinHandle<()>, EngineProbe) {
    gw.restart();
    let _ = common::lobby_ticket(gw.port, token);
    common::attach_agent_recording(gw).await
}

/// What the player's own list of their games says.
fn own_seats(port: u16, token: &str) -> String {
    let (status, body) = http(port, "GET", "/lobby/seats", Some(token), "");
    assert_eq!(status, 200, "{body}");
    body
}

/// A game a few answers in goes on across a stop and then a kill of its
/// gateway: each time the new engine holds the game as it was, the seat is
/// asked what it was asked, the record goes on where it stopped, and the
/// player's list says the same. Once the game is over its row is gone and
/// the whole record replays to the end.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines)] // one game across two restarts, top to bottom
async fn a_game_goes_on_across_a_stop_and_a_kill() {
    let gw = spawn_gateway_with("restart", &[]);
    let (agent, probe) = common::attach_agent_recording(&gw).await;
    let token = login(gw.port, "rina", "RinaRestart");
    let (game, seat_token, mut ws, asked) = a_game(gw.port, &token).await;
    answer(&mut ws, &asked, &probe).await;
    let asked = last_question(&mut ws)
        .await
        .expect("the seat is asked again");
    let hash = probe.hash.borrow().expect("the game is built");
    kept_soon(&gw, &game).await;
    let seats = own_seats(gw.port, &token);
    assert!(seats.contains(&game), "{seats}");

    // A stop: the engine is held and its record stored first.
    let mut gw = stopped(gw).await;
    agent.abort();
    assert_eq!(kept(&gw, &game), 1, "the stop leaves the game kept");
    let pieces = gw.scalar(&format!(
        "SELECT count(*) FROM game_record_chunk WHERE game_id = '{game}'"
    ));
    let before = stored_record(&gw, &game);
    assert_eq!(
        replayed(&before).0,
        hash,
        "the record is the game as it was left"
    );

    let (agent, probe) = started_again(&mut gw, &token).await;
    at(&probe, hash).await;
    assert_eq!(
        own_seats(gw.port, &token),
        seats,
        "the player's list is the same"
    );
    let mut ws = common::dial_seat(gw.port, &game, &seat_token).await;
    assert_eq!(
        first_question(&mut ws).await,
        asked,
        "the seat is asked what it was asked"
    );
    let asked = last_question(&mut ws).await.unwrap_or(asked);
    let hash = answer(&mut ws, &asked, &probe).await;
    stored_at(&gw, &game, hash).await;
    let after = stored_record(&gw, &game);
    assert!(
        after.starts_with(&before),
        "the record goes on from what was stored, which is unchanged"
    );
    assert!(
        gw.scalar(&format!(
            "SELECT min(seq)::bigint FROM game_record_chunk WHERE game_id = '{game}' AND seq >= {pieces}"
        )) == i64::from(u32::try_from(pieces).unwrap()),
        "the first new piece is the next after the stored ones"
    );
    let asked = last_question(&mut ws)
        .await
        .expect("the seat is asked again");
    let hash = probe.hash.borrow().expect("the game is built");
    stored_at(&gw, &game, hash).await;

    // A kill: nothing runs on the way down.
    gw.crash();
    agent.abort();
    assert_eq!(kept(&gw, &game), 1, "a killed gateway leaves the game kept");
    let (agent, probe) = started_again(&mut gw, &token).await;
    at(&probe, hash).await;
    let mut ws = common::dial_seat(gw.port, &game, &seat_token).await;
    assert_eq!(
        first_question(&mut ws).await,
        asked,
        "asked what it was asked"
    );

    say(&mut ws, &PlayerAction::Concede).await;
    for _ in 0..common::WAIT_TRIES {
        if gw.scalar(&format!(
            "SELECT count(*) FROM game_record WHERE game_id = '{game}' AND complete"
        )) == 1
        {
            break;
        }
        tokio::time::sleep(common::WAIT_STEP).await;
    }
    assert_eq!(kept(&gw, &game), 0, "a game that is over is not kept");
    let (_, ended) = replayed(&stored_record(&gw, &game));
    assert!(
        ended,
        "the whole record, across both restarts, replays to the end"
    );
    agent.abort();
}

/// A seat token reissued before a kill is the one the restarted gateway
/// takes: the reissue was kept before it was handed out, and the token it
/// replaced opens nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reissued_seat_token_outlives_a_kill_and_the_old_one_does_not() {
    let mut gw = spawn_gateway_with("restart-token", &[]);
    let (agent, probe) = common::attach_agent_recording(&gw).await;
    let token = login(gw.port, "tom", "TomToken");
    let (game, old_token, _ws, asked) = a_game(gw.port, &token).await;
    kept_soon(&gw, &game).await;
    let hash = probe.hash.borrow().expect("the game is built");
    stored_at(&gw, &game, hash).await;

    let (status, body) = http(
        gw.port,
        "POST",
        &format!("/lobby/games/{game}/seat"),
        Some(&token),
        "",
    );
    assert_eq!(status, 200, "{body}");
    let new_token = json_field(&body, "seat_token").to_string();
    assert_ne!(new_token, old_token);
    gw.crash();
    agent.abort();

    let (agent, probe) = started_again(&mut gw, &token).await;
    at(&probe, hash).await;
    let (status, _) = common::ask_ticket(
        gw.port,
        &old_token,
        &format!("{{\"socket\":\"seat\",\"game\":\"{game}\"}}"),
    );
    assert_ne!(
        status, 200,
        "the replaced token opens nothing after the restart"
    );
    let mut ws = common::dial_seat(gw.port, &game, &new_token).await;
    assert_eq!(first_question(&mut ws).await, asked);
    agent.abort();
}

/// A game that ends is let go at once, and nothing else is kept beside it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_game_that_ends_is_let_go() {
    let gw = spawn_gateway_with("restart-ended", &[]);
    let (agent, probe) = common::attach_agent_recording(&gw).await;
    let token = login(gw.port, "ella", "EllaEnded");
    let (game, _, mut ws, _) = a_game(gw.port, &token).await;
    kept_soon(&gw, &game).await;
    assert!(probe.hash.borrow().is_some());
    say(&mut ws, &PlayerAction::Concede).await;
    for _ in 0..common::WAIT_TRIES {
        if kept(&gw, &game) == 0 {
            break;
        }
        tokio::time::sleep(common::WAIT_STEP).await;
    }
    assert_eq!(kept(&gw, &game), 0, "a game that ended is let go");
    assert_eq!(
        gw.scalar("SELECT count(*) FROM running_game"),
        0,
        "nothing else is kept"
    );
    agent.abort();
}
