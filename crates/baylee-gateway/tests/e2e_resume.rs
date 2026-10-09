//! A client restarted into an update comes back to its table (beta.6,
//! `docs/client.md` §"Restarting into an update"): the old client's socket
//! closes mid-game, the engine holds the chair, and the new client — which
//! has the account's session and nothing else, the seat's secret having died
//! with the old process — asks for its own chair, is handed a fresh ticket
//! for it, and opens the same game where it was. Nobody else's session gets
//! that chair, and the old secret opens nothing any more.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_engine::choice::PlayerAction;
use baylee_protocol::v1::{self, Envelope};
use baylee_view::PlayerView;
use common::{Socket, attach_agent, http, json_field, login, spawn_gateway};

fn deck(port: u16, token: &str) -> String {
    let body = r#"{"name":"d","cards":["40 Forest","20 Swamp"]}"#;
    let (status, body) = http(port, "POST", "/decks", Some(token), body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_string()
}

fn send_action(action: &PlayerAction) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
            game_id: String::new(),
            seat_token: String::new(),
            action_json: serde_json::to_vec(action).expect("an action serializes"),
        })),
    }
}

/// Reads frames until the table has said a view and a question for this
/// seat, sending `SeatReady` on the opening payload: the latest view.
async fn the_seat_is_asked(ws: &mut Socket) -> PlayerView {
    tokio::time::timeout(common::WAIT_BUDGET, async {
        let mut view: Option<PlayerView> = None;
        loop {
            match common::next_msg(ws).await {
                Some(v1::envelope::Msg::GameStatic(_)) => {
                    let ready = Envelope {
                        msg: Some(v1::envelope::Msg::SeatReady(v1::SeatReady {})),
                    };
                    common::send(ws, &ready).await;
                }
                Some(v1::envelope::Msg::StateDelta(delta)) => {
                    view = Some(serde_json::from_slice(&delta.view_json).expect("a view"));
                }
                Some(v1::envelope::Msg::ChoiceRequest(_)) => {
                    if let Some(view) = view.take() {
                        return view;
                    }
                }
                Some(_) => {}
                None => panic!("the table closed the socket"),
            }
        }
    })
    .await
    .expect("the table asked this seat something")
}

#[tokio::test]
async fn a_restarted_client_takes_its_chair_back_with_its_session() {
    let gw = spawn_gateway("resume-seat");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let player = login(port, "resume-player", "Player");
    let stranger = login(port, "resume-other", "Other");

    let create = format!(
        "{{\"deck_id\":\"{}\",\"mode\":\"ai\"}}",
        deck(port, &player)
    );
    let (status, body) = http(port, "POST", "/lobby/games", Some(&player), &create);
    assert_eq!(status, 200, "a game against the house: {body}");
    let game = json_field(&body, "game_id").to_string();
    let first = json_field(&body, "seat_token").to_string();

    // The old client plays its opening answer, then quits to restart.
    let mut ws = common::dial_seat(port, &game, &first).await;
    let opening = the_seat_is_asked(&mut ws).await;
    common::send(&mut ws, &send_action(&PlayerAction::MulliganKeep)).await;
    let before = the_seat_is_asked(&mut ws).await;
    assert!(before.seq > opening.seq, "the keep moved the game");
    drop(ws);

    // Another account's session is handed nothing.
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/seat"),
        Some(&stranger),
        "{}",
    );
    assert_eq!(status, 403, "not that account's chair: {body}");

    // The new client, with the session the restart handed it, asks for its
    // own chair while the game is being played.
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/seat"),
        Some(&player),
        "{}",
    );
    assert_eq!(status, 200, "the chair, handed back: {body}");
    assert!(body.contains("\"seat\":0"), "the same chair: {body}");
    let second = json_field(&body, "seat_token").to_string();
    assert_ne!(second, first, "a fresh secret");
    let (status, _) = common::ask_ticket(
        port,
        &first,
        &format!("{{\"socket\":\"seat\",\"game\":\"{game}\"}}"),
    );
    assert_eq!(status, 401, "the old client's secret buys nothing now");

    // And the table it opens is the one it left: the same seat, no frame
    // earlier than the last it saw, the hand it kept still in it.
    let mut ws = common::dial_seat(port, &game, &second).await;
    let after = the_seat_is_asked(&mut ws).await;
    assert_eq!(after.seat, before.seat);
    assert!(after.seq >= before.seq, "{} < {}", after.seq, before.seq);
    let kept: Vec<_> = before.hand.iter().map(|card| card.id).collect();
    let now: Vec<_> = after.hand.iter().map(|card| card.id).collect();
    assert!(
        kept.iter().all(|card| now.contains(card)),
        "the kept hand is the hand it comes back to: {kept:?} vs {now:?}"
    );
}
