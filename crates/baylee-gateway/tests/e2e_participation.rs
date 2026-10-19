//! Leaving preserves participation; an authoritative concession releases it.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_engine::choice::PlayerAction;
use baylee_protocol::v1::{self, Envelope};
use common::{Socket, attach_agent, http, json_field, login, spawn_gateway};

fn post(port: u16, token: &str, path: &str, body: &str) -> String {
    let (status, body) = http(port, "POST", path, Some(token), body);
    assert_eq!(status, 200, "{path}: {body}");
    body
}

fn deck(port: u16, token: &str) -> String {
    json_field(
        &post(
            port,
            token,
            "/decks",
            r#"{"name":"d","cards":["60 Forest"]}"#,
        ),
        "deck_id",
    )
    .into()
}

fn room(port: u16, token: &str, deck: &str, chairs: u8) -> (String, String) {
    let body = post(
        port,
        token,
        "/lobby/games",
        &format!(r#"{{"deck_id":"{deck}","seats":{chairs}}}"#),
    );
    (
        json_field(&body, "game_id").into(),
        json_field(&body, "seat_token").into(),
    )
}

async fn opened(socket: &mut Socket) {
    tokio::time::timeout(common::WAIT_BUDGET, async {
        loop {
            match common::next_msg(socket).await {
                Some(v1::envelope::Msg::GameStatic(_)) => {
                    common::send(
                        socket,
                        &Envelope {
                            msg: Some(v1::envelope::Msg::SeatReady(v1::SeatReady {})),
                        },
                    )
                    .await;
                }
                Some(v1::envelope::Msg::Curtain(_)) => return,
                Some(_) => {}
                None => panic!("the socket closed before the table opened"),
            }
        }
    })
    .await
    .expect("the shared entrance finished");
}

fn resumable(port: u16, token: &str, game: &str) -> bool {
    let (status, body) = http(port, "GET", "/lobby/seats", Some(token), "");
    assert_eq!(status, 200, "{body}");
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    body["seats"]
        .as_array()
        .unwrap()
        .iter()
        .any(|seat| seat["game_id"] == game)
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // one sequential end-to-end participation scenario
async fn leaving_with_people_blocks_new_games_until_concession_and_start_is_atomic() {
    let gateway = spawn_gateway("participation");
    let agent = attach_agent(&gateway).await;
    let port = gateway.port;
    let alice = login(port, "participating-a", "Alice");
    let bob = login(port, "participating-b", "Bob");
    let cara = login(port, "participating-c", "Cara");
    let outsider = login(port, "participating-other", "Other");
    let decks = [deck(port, &alice), deck(port, &bob), deck(port, &cara)];
    let alternatives = [
        room(port, &alice, &decks[0], 2).0,
        room(port, &alice, &decks[0], 2).0,
    ];
    for game in &alternatives {
        post(
            port,
            &alice,
            &format!("/lobby/games/{game}/seats/1"),
            r#"{"kind":"ai"}"#,
        );
    }
    let other_game = room(port, &outsider, "", 2).0;
    let (game, alice_seat) = room(port, &alice, &decks[0], 3);
    let mut tokens = vec![alice_seat];
    for (player, deck) in [(&bob, &decks[1]), (&cara, &decks[2])] {
        let body = post(
            port,
            player,
            &format!("/lobby/games/{game}/join"),
            &format!(r#"{{"deck_id":"{deck}"}}"#),
        );
        tokens.push(json_field(&body, "seat_token").into());
        post(port, player, &format!("/lobby/games/{game}/ready"), "{}");
    }
    post(port, &alice, &format!("/lobby/games/{game}/start"), "{}");
    let mut alice_socket = common::dial_seat(port, &game, &tokens[0]).await;
    let mut bob_socket = common::dial_seat(port, &game, &tokens[1]).await;
    let mut cara_socket = common::dial_seat(port, &game, &tokens[2]).await;
    tokio::join!(
        opened(&mut alice_socket),
        opened(&mut bob_socket),
        opened(&mut cara_socket)
    );

    assert_eq!(
        http(
            port,
            "POST",
            &format!("/lobby/games/{}/start", alternatives[0]),
            Some(&alice),
            "{}"
        )
        .0,
        409
    );
    assert_eq!(
        http(port, "POST", "/lobby/depart", Some(&alice), "{}").0,
        204
    );
    for (path, body) in [
        ("/lobby/games".into(), "{}".into()),
        (
            format!("/lobby/games/{other_game}/join"),
            format!(r#"{{"deck_id":"{}"}}"#, decks[0]),
        ),
    ] {
        let (status, body) = http(port, "POST", &path, Some(&alice), &body);
        assert_eq!(
            status, 409,
            "unfinished participation must block {path}: {body}"
        );
    }
    assert!(resumable(port, &alice, &game));
    let body = post(port, &alice, &format!("/lobby/games/{game}/seat"), "{}");
    let mut resumed = common::dial_seat(port, &game, json_field(&body, "seat_token")).await;
    opened(&mut resumed).await;
    common::send(
        &mut resumed,
        &Envelope {
            msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
                game_id: String::new(),
                seat_token: String::new(),
                action_json: serde_json::to_vec(&PlayerAction::Concede).unwrap(),
            })),
        },
    )
    .await;
    let mut ended = false;
    for _ in 0..common::WAIT_TRIES {
        if !resumable(port, &alice, &game) {
            ended = true;
            break;
        }
        tokio::time::sleep(common::WAIT_STEP).await;
    }
    assert!(
        ended,
        "the engine never reported Alice's completed participation"
    );
    assert!(
        resumable(port, &bob, &game) && resumable(port, &cara, &game),
        "the other two are still playing"
    );
    assert_eq!(
        http(
            port,
            "POST",
            &format!("/lobby/games/{game}/seat"),
            Some(&alice),
            "{}"
        )
        .0,
        409
    );
    post(port, &alice, "/lobby/games", "{}");
    post(
        port,
        &alice,
        &format!("/lobby/games/{other_game}/join"),
        &format!(r#"{{"deck_id":"{}"}}"#, decks[0]),
    );

    // Departure left the old waiting rooms; create two fresh candidates now
    // that the engine has accepted Alice's concession.
    let alternatives = [
        room(port, &alice, &decks[0], 2).0,
        room(port, &alice, &decks[0], 2).0,
    ];
    for game in &alternatives {
        post(
            port,
            &alice,
            &format!("/lobby/games/{game}/seats/1"),
            r#"{"kind":"ai"}"#,
        );
    }

    // Both requests may hydrate concurrently. The final transition under
    // the lobby lock must admit exactly one new running game for Alice.
    let barrier = std::sync::Barrier::new(2);
    let run = |game: &str| {
        barrier.wait();
        http(
            port,
            "POST",
            &format!("/lobby/games/{game}/start"),
            Some(&alice),
            "{}",
        )
        .0
    };
    let mut outcomes = std::thread::scope(|scope| {
        let first = scope.spawn(|| run(&alternatives[0]));
        let second = scope.spawn(|| run(&alternatives[1]));
        [first.join().unwrap(), second.join().unwrap()]
    });
    outcomes.sort_unstable();
    assert_eq!(outcomes, [200, 409]);
    agent.abort();
}
