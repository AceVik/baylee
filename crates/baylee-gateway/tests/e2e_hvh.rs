//! End-to-end regression test for human-vs-human over the gateway: spawns
//! the real `baylee-gateway` binary, attaches an agent that runs a real
//! engine, registers two accounts over HTTP, creates and joins a lobby game,
//! then connects BOTH seat sockets and asserts that player B sees the game
//! advance when player A acts.
//!
//! Before the per-game broadcast this failed: the gateway filtered every
//! pumped envelope to the acting seat, so B's socket stayed silent until B
//! acted — which B couldn't do, not knowing the game state.
//!
//! It now also covers the whole circle. The gateway runs no rules: it asks an
//! agent for an engine, the engine dials back, and every frame in this test
//! crosses both sockets.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_engine::choice::{Pending, PlayerAction};
use baylee_protocol::v1::{self, Envelope};
use common::{attach_agent, http, json_field, login, spawn_gateway};
use futures_util::{SinkExt, StreamExt};
use prost::Message;

async fn recv_until<F>(
    ws: &mut (
             impl StreamExt<
        Item = Result<
            tokio_tungstenite::tungstenite::Message,
            tokio_tungstenite::tungstenite::Error,
        >,
    > + Unpin
         ),
    mut accept: F,
) -> Envelope
where
    F: FnMut(&Envelope) -> bool,
{
    for _ in 0..50 {
        let frame = tokio::time::timeout(common::WAIT_BUDGET, ws.next())
            .await
            .expect("no frame within the wait budget")
            .expect("stream open")
            .expect("frame ok");
        if !frame.is_binary() {
            continue;
        }
        let env = Envelope::decode(frame.into_data()).expect("decode envelope");
        if accept(&env) {
            return env;
        }
    }
    panic!("expected envelope never arrived");
}

/// Two accounts, a deck each, a room A opens and B joins, both ready and the
/// host starts it: the game's id and each seat's token.
fn start_two_seats(port: u16) -> (String, String, String) {
    // Two accounts.
    let tokens = [
        login(port, "alice", "alice_hvh"),
        login(port, "bob", "bob_hvh"),
    ];

    // One deck each (basic lands pass the registry and the count rules).
    let mut deck_ids = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        // Deliberately not the same card pool: the opening payload must have
        // a hole where the other deck's exclusive printing is.
        let cards = if i == 0 {
            "\"40 Island\",\"20 Forest\""
        } else {
            "\"40 Forest\",\"20 Swamp\""
        };
        let deck = format!("{{\"name\":\"d{i}\",\"cards\":[{cards}]}}");
        let (status, body) = http(port, "POST", "/decks", Some(token), &deck);
        assert_eq!(status, 200, "create deck {i}: {body}");
        deck_ids.push(json_field(&body, "deck_id").to_string());
    }

    // A opens a waiting game, B joins it.
    let create = format!("{{\"deck_id\":\"{}\",\"mode\":\"open\"}}", deck_ids[0]);
    let (status, body) = http(port, "POST", "/lobby/games", Some(&tokens[0]), &create);
    assert_eq!(status, 200, "create open game: {body}");
    let game_id = json_field(&body, "game_id").to_string();
    let seat_token_a = json_field(&body, "seat_token").to_string();

    let join = format!("{{\"deck_id\":\"{}\"}}", deck_ids[1]);
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/join"),
        Some(&tokens[1]),
        &join,
    );
    assert_eq!(status, 200, "join game: {body}");
    let seat_token_b = json_field(&body, "seat_token").to_string();

    // Both say they are ready, and the host starts the table. Sitting down is
    // not the same statement as being ready to play, so neither is enough on
    // its own.
    for token in &tokens {
        let (status, body) = http(
            port,
            "POST",
            &format!("/lobby/games/{game_id}/ready"),
            Some(token),
            "{}",
        );
        assert_eq!(status, 200, "ready: {body}");
    }
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/start"),
        Some(&tokens[0]),
        "",
    );
    assert_eq!(status, 200, "start: {body}");

    (game_id, seat_token_a, seat_token_b)
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // e2e scenario script
async fn human_vs_human_both_seats_receive_updates() {
    let gw = spawn_gateway("hvh");
    let port = gw.port;
    let agent = attach_agent(&gw).await;
    let (game_id, seat_token_a, seat_token_b) = start_two_seats(port);

    // Connect both seat sockets.
    // Sequentially, which is the same end state as the interleaved retry loop
    // this replaces: both seats have to open before anything below runs, and
    // neither one's opening is what unblocks the other — the engine attaching
    // is.
    let mut ws_a = common::dial_seat(port, &game_id, &seat_token_a).await;
    let mut ws_b = common::dial_seat(port, &game_id, &seat_token_b).await;

    // The very first thing a seat is sent is the roster and the print table.
    // A client has no preset to build them from, and without the print table
    // a PrintRef names no card at all — so it has to arrive before anything
    // that refers to one.
    let opening = recv_until(&mut ws_b, |_| true).await;
    let Some(v1::envelope::Msg::GameStatic(msg)) = opening.msg else {
        panic!("a seat's first frame is the opening payload");
    };
    assert_eq!(msg.view_version, baylee_view::VIEW_VERSION);
    let statics: baylee_view::GameStatic =
        serde_json::from_slice(&msg.static_json).expect("static json");
    assert_eq!(statics.your_seat, baylee_core::ids::PlayerId::new(1));
    // A roster carries handles, not bare names: a display name is not
    // unique, so two players called Alice at one table would otherwise be
    // one name twice. Nothing below the gateway knows a tag exists — the
    // view still carries a string — which is why this is asserted on its
    // shape rather than on a field.
    let alice = statics.seat_name(baylee_core::ids::PlayerId::new(0));
    let bob = statics.seat_name(baylee_core::ids::PlayerId::new(1));
    assert!(alice.starts_with("alice_hvh#"), "{alice}");
    assert!(bob.starts_with("bob_hvh#"), "{bob}");
    assert_ne!(
        alice.rsplit('#').next(),
        bob.rsplit('#').next(),
        "two seats were handed the same tag: {alice} / {bob}"
    );
    assert!(
        statics.prints.iter().any(Option::is_some),
        "the print table came along"
    );
    assert!(
        statics.prints.iter().any(Option::is_none),
        "and stopped short of the other deck: {:?}",
        statics.prints
    );
    assert!(
        statics.seats.iter().all(|s| !s.is_ai),
        "both chairs are people in a human-vs-human game"
    );

    // Seat A gets the first choice request (its mulligan).
    let first = recv_until(&mut ws_a, |env| {
        matches!(env.msg, Some(v1::envelope::Msg::ChoiceRequest(_)))
    })
    .await;
    let Some(v1::envelope::Msg::ChoiceRequest(req)) = first.msg else {
        panic!("checked above");
    };
    let pending: Pending = serde_json::from_slice(&req.pending_json).expect("pending json");
    let action = match pending {
        Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
        _ => PlayerAction::PassPriority,
    };
    // The frame below has to be one A's action *caused*, and this is what
    // says so. `Session::seq` is one counter for the whole game, and nothing
    // moves it between the engine asking A and A answering — so every frame B
    // was sent by the initial pumps carries at most this number, and a
    // greater one cannot have been written before the action.
    //
    // It replaces draining B's queue for 200 ms, which was the same question
    // asked as a stopwatch: on a loaded box a pump's frame outlives the
    // drain, `recv_until` returns that stale frame at once, and the assertion
    // passes without A's action having reached anybody. A budget that fails
    // by **passing** cannot be tuned, only removed.
    let baseline = req.seq;

    // A acts. WITHOUT B sending anything, B's socket must now deliver a
    // fresh envelope — this is the regression assertion.
    let answer = Envelope {
        msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
            game_id: String::new(),
            seat_token: String::new(),
            action_json: serde_json::to_vec(&action).unwrap(),
        })),
    };
    ws_a.send(tokio_tungstenite::tungstenite::Message::Binary(
        answer.encode_to_vec().into(),
    ))
    .await
    .expect("A sends its action");

    recv_until(&mut ws_b, |env| match &env.msg {
        Some(v1::envelope::Msg::StateDelta(d)) => d.seq > baseline,
        Some(v1::envelope::Msg::ChoiceRequest(r)) => r.seq > baseline,
        _ => false,
    })
    .await;

    agent.abort();
}

/// The gateway runs no rules of its own, so with nobody to run an engine there
/// is no game to be had — and it has to say so rather than hand out a seat
/// token for a table that will never start.
#[tokio::test]
async fn a_game_without_an_agent_is_refused() {
    let gw = spawn_gateway("no-agent");
    let token = login(gw.port, "solo", "solo_player");
    let (status, body) = http(
        gw.port,
        "POST",
        "/decks",
        Some(&token),
        "{\"name\":\"d\",\"cards\":[\"60 Forest\"]}",
    );
    assert_eq!(status, 200, "create deck: {body}");
    let deck_id = json_field(&body, "deck_id").to_string();

    let create = format!("{{\"deck_id\":\"{deck_id}\",\"mode\":\"ai\"}}");
    let (status, body) = http(gw.port, "POST", "/lobby/games", Some(&token), &create);
    assert_eq!(
        status, 503,
        "a game started with no engine to run it: {body}"
    );

    // And the table did not survive the failure as a ghost in the lobby.
    let (status, body) = http(gw.port, "GET", "/lobby/games", Some(&token), "");
    assert_eq!(status, 200);
    assert!(
        body.contains("\"games\":[]"),
        "a failed game was left in the lobby: {body}"
    );
}

/// An agent is not a player. The control socket takes a shared secret from the
/// gateway's own configuration, and nothing a player could ever hold.
///
/// Refusing is a thing the gateway **does**, and this asks for it that way.
/// It used to ask the opposite — five seconds of silence and
/// `assert!(!welcomed)` — which is a negative behind a clock and passes for
/// two very different reasons: the socket was refused, or the socket is still
/// open and the welcome had not arrived yet. A gateway that accepted the
/// wrong secret and then said nothing satisfied it, and so did a busy box.
/// `run_agent_socket` returns on a token that does not match, which drops the
/// socket, so the end of the stream is a fact this test can wait for instead.
#[tokio::test]
async fn the_control_socket_refuses_the_wrong_secret() {
    let gw = spawn_gateway("agent-auth");
    let url = format!("ws://127.0.0.1:{}/agent/ws", gw.port);
    let (mut ws, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("the upgrade itself is not the check");
    let hello = Envelope {
        msg: Some(v1::envelope::Msg::AgentHello(v1::AgentHello {
            token: "not-the-secret".to_string(),
            name: "impostor".to_string(),
            capacity: 0,
            protocol_version: baylee_protocol::PROTOCOL_VERSION,
        })),
    };
    ws.send(tokio_tungstenite::tungstenite::Message::Binary(
        hello.encode_to_vec().into(),
    ))
    .await
    .expect("send hello");
    // Read until the socket says something that settles it. A control frame
    // is neither answer — tungstenite surfaces pings, and skipping them is
    // what keeps this about the secret rather than about keepalives.
    loop {
        let Ok(next) = tokio::time::timeout(common::WAIT_BUDGET, ws.next()).await else {
            panic!(
                "the gateway held an agent that presented the wrong secret open \
                 and silent for {:?} instead of dropping it",
                common::WAIT_BUDGET
            )
        };
        match next {
            Some(Ok(
                tokio_tungstenite::tungstenite::Message::Binary(_)
                | tokio_tungstenite::tungstenite::Message::Text(_),
            )) => {
                panic!("an agent with the wrong secret was welcomed")
            }
            Some(Ok(
                tokio_tungstenite::tungstenite::Message::Ping(_)
                | tokio_tungstenite::tungstenite::Message::Pong(_),
            )) => {}
            // A close frame, a broken stream, or the end of it: refused.
            _ => break,
        }
    }
}

/// Both seats are told the table is open as soon as both have said they
/// are ready, and not on the curtain's deadline (#256).
///
/// This is what turns a seat socket that forgot `SeatReady` into a red test.
/// Without it a table still opens, thirty seconds late, and every game is
/// slower by that for no reason anybody sees. The in-process engine here
/// runs no deadline at all, so a missing `SeatReady` never opens the table,
/// and the bound below fails.
#[tokio::test]
async fn the_curtain_goes_up_for_both_seats_once_both_are_ready() {
    let gw = spawn_gateway("hvh_curtain");
    let _agent = attach_agent(&gw).await;
    let (game_id, seat_token_a, seat_token_b) = start_two_seats(gw.port);
    let prompt = std::time::Duration::from_secs(u64::from(baylee_engine_server::CURTAIN_SECS) / 3);
    let mut seats = [
        common::dial_seat(gw.port, &game_id, &seat_token_a).await,
        common::dial_seat(gw.port, &game_id, &seat_token_b).await,
    ];
    for (seat, ws) in seats.iter_mut().enumerate() {
        let mut heard = Vec::new();
        tokio::time::timeout(prompt, async {
            loop {
                let env = recv_until(ws, |_| true).await;
                let what = match env.msg {
                    Some(v1::envelope::Msg::ChoiceRequest(_)) => "question",
                    Some(v1::envelope::Msg::Curtain(_)) => "curtain",
                    _ => "other",
                };
                heard.push(what);
                if what == "curtain" {
                    break;
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("seat {seat} saw no curtain within {prompt:?}: {heard:?}"));
        assert!(
            heard.contains(&"question"),
            "seat {seat} opened on no question: {heard:?}"
        );
    }
}
