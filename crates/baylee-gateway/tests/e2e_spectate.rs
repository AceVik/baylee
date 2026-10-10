//! Spectators (`docs/protocol.md` §"Spectators"): a signed-in player
//! watches a running table on its public view, the seats are told how many
//! watch, the listing counts them, and a table that allows none refuses a
//! spectator's ticket with 403.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_protocol::v1::{self, Envelope};
use baylee_view::SpectatorView;
use common::{Socket, attach_agent, http, json_field, login, spawn_gateway};

fn deck(port: u16, token: &str) -> String {
    let body = r#"{"name":"d","cards":["40 Forest","20 Swamp"]}"#;
    let (status, body) = http(port, "POST", "/decks", Some(token), body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_string()
}

fn ai_game(port: u16, player: &str, extra: &str) -> (String, String) {
    let create = format!(
        "{{\"deck_id\":\"{}\",\"mode\":\"ai\"{extra}}}",
        deck(port, player)
    );
    let (status, body) = http(port, "POST", "/lobby/games", Some(player), &create);
    assert_eq!(status, 200, "a game against the house: {body}");
    (
        json_field(&body, "game_id").to_string(),
        json_field(&body, "seat_token").to_string(),
    )
}

fn watch_ticket(port: u16, session: &str, game: &str) -> (u16, String) {
    common::ask_ticket(
        port,
        session,
        &format!("{{\"socket\":\"watch\",\"game\":\"{game}\"}}"),
    )
}

async fn dial_watch(port: u16, session: &str, game: &str) -> Socket {
    for _ in 0..100 {
        let (status, body) = watch_ticket(port, session, game);
        if status == 200 {
            let url = format!(
                "ws://127.0.0.1:{port}{}",
                baylee_protocol::watch_socket_path(game, json_field(&body, "ticket"))
            );
            if let Some(ws) = common::dial(&url).await {
                return ws;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("the watch socket never opened");
}

/// Reads until the spectator holds a payload, a view and the curtain.
async fn spectator_table(ws: &mut Socket) -> SpectatorView {
    tokio::time::timeout(common::WAIT_BUDGET, async {
        let (mut statics, mut view) = (false, None);
        loop {
            match common::next_msg(ws).await {
                Some(v1::envelope::Msg::GameStatic(_)) => statics = true,
                Some(v1::envelope::Msg::StateDelta(delta)) => {
                    view = Some(
                        serde_json::from_slice::<SpectatorView>(&delta.view_json)
                            .expect("a spectator view"),
                    );
                }
                Some(v1::envelope::Msg::Curtain(_)) => {
                    assert!(statics, "the payload comes first");
                    return view.expect("a view before the curtain");
                }
                Some(v1::envelope::Msg::ChoiceRequest(_)) => panic!("a spectator was asked"),
                Some(_) => {}
                None => panic!("the watch socket closed"),
            }
        }
    })
    .await
    .expect("the spectator was shown the table")
}

async fn told_watchers(ws: &mut Socket, count: u32) {
    tokio::time::timeout(common::WAIT_BUDGET, async {
        loop {
            match common::next_msg(ws).await {
                Some(v1::envelope::Msg::Spectators(s)) if s.count == count => return,
                Some(_) => {}
                None => panic!("the seat socket closed"),
            }
        }
    })
    .await
    .expect("the seat was told how many watch");
}

fn listed_spectators(port: u16, session: &str, game: &str) -> String {
    let (status, body) = http(port, "GET", "/lobby/games", Some(session), "");
    assert_eq!(status, 200, "{body}");
    let listing: serde_json::Value = serde_json::from_str(&body).expect("json");
    let games = listing
        .get("games")
        .and_then(|g| g.as_array())
        .cloned()
        .or_else(|| listing.as_array().cloned())
        .expect("a list");
    let row = games
        .iter()
        .find(|g| g["id"] == game)
        .expect("the game is listed");
    format!("{}/{}", row["spectators_allowed"], row["spectators"])
}

#[tokio::test]
async fn a_player_watches_a_running_table_and_leaves() {
    let gw = spawn_gateway("spectate-watch");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let player = login(port, "spectate-player", "Player");
    let watcher = login(port, "spectate-watcher", "Watcher");
    let (game, seat_token) = ai_game(port, &player, "");
    let mut seat = common::dial_seat(port, &game, &seat_token).await;

    let mut ws = dial_watch(port, &watcher, &game).await;
    let view = spectator_table(&mut ws).await;
    let me = view.seats.first().expect("seat 0");
    assert_eq!(me.hand_count, 7, "a hand is a count to a spectator");
    told_watchers(&mut seat, 1).await;
    assert_eq!(listed_spectators(port, &player, &game), "true/1");

    // Whatever a spectator sends is dropped; it is still watching.
    common::send(
        &mut ws,
        &Envelope {
            msg: Some(v1::envelope::Msg::SeatReady(v1::SeatReady {})),
        },
    )
    .await;

    drop(ws);
    told_watchers(&mut seat, 0).await;
    assert_eq!(listed_spectators(port, &player, &game), "true/0");
}

#[tokio::test]
async fn a_table_that_allows_no_spectators_refuses_one() {
    let gw = spawn_gateway("spectate-closed");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let player = login(port, "spectate-host", "Host");
    let watcher = login(port, "spectate-peek", "Peek");
    let (game, _) = ai_game(port, &player, ",\"allow_spectators\":false");
    let (status, body) = watch_ticket(port, &watcher, &game);
    assert_eq!(status, 403, "{body}");
    assert_eq!(listed_spectators(port, &player, &game), "false/0");
}
