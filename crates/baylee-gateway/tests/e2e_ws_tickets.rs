//! Opening a socket with a ticket instead of a token (#294).
//!
//! Every socket a player opens used to carry its secret in the address, where
//! a reverse proxy's access log sees it. Now a client trades its bearer token
//! for a ticket over `POST /ws-ticket` and dials with `?ticket=`. These tests
//! hold the gateway to what a ticket promises: it opens one socket, once, for
//! a while, and only the socket it was bought for, and only while what bought
//! it still stands. And to the transition: the old `?token=` is accepted until
//! the dated end of its window and refused after it.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_engine::choice::PlayerAction;
use baylee_protocol::v1::{self, Envelope};
use common::{Socket, attach_agent, http, json_field, login, spawn_gateway, spawn_gateway_with};
use futures_util::{SinkExt, StreamExt};
use prost::Message as _;

/// The gateway's `wsticket::LEGACY_UNTIL`: 2026-11-01T00:00:00Z, the first
/// second the old `?token=` is refused. Restated because the gateway is a
/// binary a test cannot import from; `docs/protocol.md` names the date.
const LEGACY_UNTIL: u64 = 1_793_491_200;

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Tries an upgrade and says how it went: `Ok` for an open socket, or the
/// refusal's status and body.
async fn upgrade(url: &str) -> Result<Socket, (u16, String)> {
    match tokio_tungstenite::connect_async(url).await {
        Ok((socket, _)) => Ok(socket),
        Err(tokio_tungstenite::tungstenite::Error::Http(refused)) => Err((
            refused.status().as_u16(),
            refused
                .body()
                .as_ref()
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .unwrap_or_default(),
        )),
        Err(other) => panic!("the upgrade failed below HTTP: {other}"),
    }
}

/// Asserts an upgrade is refused as a stale ticket is: `401` and the one
/// sentence a client recognises.
async fn refused_as_stale(url: &str, what: &str) {
    match upgrade(url).await {
        Ok(_) => panic!("{what}: the socket opened"),
        Err((status, body)) => {
            assert_eq!(status, 401, "{what}: {body}");
            assert!(
                body.contains(baylee_protocol::TICKET_REFUSED),
                "{what}: not the stale-ticket sentence: {body}"
            );
        }
    }
}

/// The next text frame on a lobby socket.
async fn next_text(socket: &mut Socket) -> String {
    loop {
        let frame = tokio::time::timeout(common::WAIT_BUDGET, socket.next())
            .await
            .expect("the lobby socket said nothing")
            .expect("the lobby socket closed")
            .expect("the lobby socket errored");
        if let tokio_tungstenite::tungstenite::Message::Text(text) = frame {
            return text.to_string();
        }
    }
}

fn deck(port: u16, token: &str) -> String {
    let body = r#"{"name":"d","cards":["40 Forest","20 Swamp"]}"#;
    let (status, body) = http(port, "POST", "/decks", Some(token), body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_string()
}

/// A game against the house: its id and the seat token.
fn game_against_the_house(port: u16, session: &str) -> (String, String) {
    let create = format!(
        "{{\"deck_id\":\"{}\",\"mode\":\"ai\"}}",
        deck(port, session)
    );
    let (status, body) = http(port, "POST", "/lobby/games", Some(session), &create);
    assert_eq!(status, 200, "a game against the house: {body}");
    (
        json_field(&body, "game_id").to_string(),
        json_field(&body, "seat_token").to_string(),
    )
}

fn seat_body(game: &str) -> String {
    format!("{{\"socket\":\"seat\",\"game\":\"{game}\"}}")
}

/// A seat socket whose upgrade was let through: the first frame is the
/// table's opening payload.
async fn opens_on_its_table(ws: &mut Socket) {
    let first = tokio::time::timeout(common::WAIT_BUDGET, common::next_msg(ws))
        .await
        .expect("the table said nothing");
    assert!(
        matches!(first, Some(v1::envelope::Msg::GameStatic(_))),
        "a seat's first frame is the opening payload: {first:?}"
    );
}

#[tokio::test]
async fn a_lobby_ticket_opens_the_feed_once() {
    let gw = spawn_gateway("tickets-lobby-once");
    let port = gw.port;
    let session = login(port, "t-lobby", "Lobby");

    let (status, body) = common::ask_ticket(port, &session, r#"{"socket":"lobby"}"#);
    assert_eq!(status, 200, "{body}");
    assert_eq!(common::json_number(&body, "expires_in"), 45, "the default");
    let ticket = json_field(&body, "ticket").to_string();
    assert_eq!(ticket.len(), 64, "256 bits, hex: {ticket}");
    assert!(!ticket.contains(&session));

    let url = format!("ws://127.0.0.1:{port}/lobby/ws?ticket={ticket}");
    let mut socket = upgrade(&url).await.expect("the ticket opens the feed");
    assert!(next_text(&mut socket).await.contains("\"games\""));
    refused_as_stale(&url, "the same ticket a second time").await;
    drop(socket);
    refused_as_stale(&url, "the same ticket after its socket closed").await;
}

#[tokio::test]
async fn a_seat_ticket_opens_its_seat_once() {
    let gw = spawn_gateway("tickets-seat-once");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let session = login(port, "t-seat", "Seat");
    let (game, seat_token) = game_against_the_house(port, &session);

    let ticket = common::seat_ticket(port, &game, &seat_token);
    let url = common::seat_url_with(port, &game, &ticket);
    assert!(
        !url.contains(&seat_token),
        "the seat token is not in the address"
    );
    let mut ws = upgrade(&url).await.expect("the ticket opens the seat");
    opens_on_its_table(&mut ws).await;
    refused_as_stale(&url, "the same seat ticket a second time").await;
}

/// A ticket nobody used dies after `BAYLEE_WS_TICKET_SECS`, and the answer
/// says so in the words a client acts on.
#[tokio::test]
async fn an_unused_ticket_expires() {
    let gw = spawn_gateway_with("tickets-expire", &[("BAYLEE_WS_TICKET_SECS", "1".into())]);
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let session = login(port, "t-expire", "Expire");
    let (game, seat_token) = game_against_the_house(port, &session);

    let (status, body) = common::ask_ticket(port, &session, r#"{"socket":"lobby"}"#);
    assert_eq!(status, 200, "{body}");
    assert_eq!(common::json_number(&body, "expires_in"), 1);
    let lobby = format!(
        "ws://127.0.0.1:{port}/lobby/ws?ticket={}",
        json_field(&body, "ticket")
    );
    let seat = common::seat_url(port, &game, &seat_token);
    tokio::time::sleep(std::time::Duration::from_millis(1300)).await;
    refused_as_stale(&lobby, "an expired lobby ticket").await;
    refused_as_stale(&seat, "an expired seat ticket").await;
    // A fresh one works at once: expiry is the ticket's, not the session's.
    let mut ws = upgrade(&common::seat_url(port, &game, &seat_token))
        .await
        .expect("a fresh seat ticket");
    opens_on_its_table(&mut ws).await;
}

/// A ticket opens only the socket it was bought for: not the other kind,
/// not another game's seat. And shown at the wrong door it is spent.
#[tokio::test]
async fn a_ticket_opens_only_its_own_door() {
    let gw = spawn_gateway("tickets-doors");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let session = login(port, "t-doors", "Doors");
    let (game_a, token_a) = game_against_the_house(port, &session);
    let (game_b, _token_b) = game_against_the_house(port, &session);

    let lobby_ticket = common::lobby_ticket(port, &session);
    refused_as_stale(
        &common::seat_url_with(port, &game_a, &lobby_ticket),
        "a lobby ticket at a seat",
    )
    .await;
    refused_as_stale(
        &format!("ws://127.0.0.1:{port}/lobby/ws?ticket={lobby_ticket}"),
        "a lobby ticket already shown at the wrong door",
    )
    .await;

    let seat_ticket = common::seat_ticket(port, &game_a, &token_a);
    refused_as_stale(
        &format!("ws://127.0.0.1:{port}/lobby/ws?ticket={seat_ticket}"),
        "a seat ticket at the lobby",
    )
    .await;

    let seat_ticket = common::seat_ticket(port, &game_a, &token_a);
    refused_as_stale(
        &common::seat_url_with(port, &game_b, &seat_ticket),
        "game A's seat ticket at game B",
    )
    .await;

    // A seat token buys tickets for its own game only.
    let (status, body) = common::ask_ticket(port, &token_a, &seat_body(&game_b));
    assert_eq!(status, 401, "game A's token bought game B's ticket: {body}");
    // A session is not a seat token, nor the other way round.
    let (status, _) = common::ask_ticket(port, &session, &seat_body(&game_a));
    assert_eq!(status, 401, "a session bought a seat ticket");
    let (status, _) = common::ask_ticket(port, &token_a, r#"{"socket":"lobby"}"#);
    assert_eq!(status, 401, "a seat token bought a lobby ticket");
    let (status, _) = common::ask_ticket(port, &token_a, &seat_body("no-such-game"));
    assert_eq!(status, 404);
    let (status, _) = common::ask_ticket(port, &session, r#"{"socket":"cellar"}"#);
    assert!(
        (400..500).contains(&status),
        "a socket nobody has: {status}"
    );
    let (status, _) = http(
        port,
        "POST",
        baylee_protocol::WS_TICKET_PATH,
        None,
        r#"{"socket":"lobby"}"#,
    );
    assert_eq!(status, 401, "no bearer, no ticket");
}

/// A ticket is only as good as what bought it: a session signed out or
/// deleted since, or a seat whose token was handed out again, opens nothing.
#[tokio::test]
async fn a_ticket_dies_with_what_bought_it() {
    let gw = spawn_gateway("tickets-standing");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;

    // Signed out.
    let session = login(port, "t-signout", "Signout");
    let ticket = common::lobby_ticket(port, &session);
    let (status, body) = http(port, "POST", "/auth/logout", Some(&session), "{}");
    assert!(status == 204 || status == 200, "sign out: {status} {body}");
    refused_as_stale(
        &format!("ws://127.0.0.1:{port}/lobby/ws?ticket={ticket}"),
        "a lobby ticket of a signed-out session",
    )
    .await;

    // Deleted.
    let session = login(port, "t-delete", "Delete");
    let ticket = common::lobby_ticket(port, &session);
    let (status, body) = http(
        port,
        "DELETE",
        "/account",
        Some(&session),
        r#"{"password":"a-very-fine-password"}"#,
    );
    assert_eq!(status, 204, "{body}");
    refused_as_stale(
        &format!("ws://127.0.0.1:{port}/lobby/ws?ticket={ticket}"),
        "a lobby ticket of a deleted account",
    )
    .await;

    // A seat handed out again: the old token's ticket opens nothing, the new
    // token's does.
    let session = login(port, "t-rotate", "Rotate");
    let (game, old_token) = game_against_the_house(port, &session);
    let old_ticket = common::seat_ticket(port, &game, &old_token);
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/seat"),
        Some(&session),
        "{}",
    );
    assert_eq!(status, 200, "take the seat again: {body}");
    let new_token = json_field(&body, "seat_token").to_string();
    assert_ne!(new_token, old_token);
    refused_as_stale(
        &common::seat_url_with(port, &game, &old_ticket),
        "a ticket bought with a seat token since replaced",
    )
    .await;
    let (status, _) = common::ask_ticket(port, &old_token, &seat_body(&game));
    assert_eq!(status, 401, "a replaced seat token buys nothing");
    let mut ws = upgrade(&common::seat_url(port, &game, &new_token))
        .await
        .expect("the new token's ticket");
    opens_on_its_table(&mut ws).await;
}

/// Inside the window a client from before #294 still gets in the old way;
/// once it closes, it does not. The window's end is the gateway's own date,
/// so which of the two this run sees depends on the day it runs, and both
/// are right: the test never fails because a date passed.
#[tokio::test]
async fn the_old_query_token_works_while_its_window_is_open() {
    let gw = spawn_gateway("tickets-legacy-open");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let session = login(port, "t-legacy", "Legacy");
    let (game, seat_token) = game_against_the_house(port, &session);
    let open = now_secs() < LEGACY_UNTIL;

    let lobby = upgrade(&format!("ws://127.0.0.1:{port}/lobby/ws?token={session}")).await;
    let seat = upgrade(&format!(
        "ws://127.0.0.1:{port}/games/{game}/ws?token={seat_token}&protocol={}",
        baylee_protocol::PROTOCOL_VERSION
    ))
    .await;
    let (cosmetics, _) = http(
        port,
        "GET",
        &format!("/games/{game}/cosmetics?token={seat_token}"),
        None,
        "",
    );
    if open {
        let mut lobby = lobby.expect("the old lobby query inside the window");
        assert!(next_text(&mut lobby).await.contains("\"games\""));
        opens_on_its_table(&mut seat.expect("the old seat query inside the window")).await;
        assert_eq!(cosmetics, 200, "the old cosmetics query inside the window");
    } else {
        assert_eq!(lobby.err().map(|e| e.0), Some(401));
        assert_eq!(seat.err().map(|e| e.0), Some(401));
        assert_eq!(cosmetics, 401);
    }
    // A wrong old token is refused either way.
    assert!(
        upgrade(&format!("ws://127.0.0.1:{port}/lobby/ws?token=not-a-token"))
            .await
            .is_err()
    );
}

/// `BAYLEE_WS_LEGACY_TOKENS=off` is the window closed: every old query is
/// refused, and the new ways keep working.
#[tokio::test]
async fn the_old_query_token_is_refused_once_the_window_closed() {
    let gw = spawn_gateway_with(
        "tickets-legacy-off",
        &[("BAYLEE_WS_LEGACY_TOKENS", "off".into())],
    );
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let session = login(port, "t-closed", "Closed");
    let (game, seat_token) = game_against_the_house(port, &session);

    let (status, body) = upgrade(&format!("ws://127.0.0.1:{port}/lobby/ws?token={session}"))
        .await
        .expect_err("the old lobby query opened a socket");
    assert_eq!(status, 401, "{body}");
    assert!(body.contains("update the client"), "{body}");
    let (status, body) = upgrade(&format!(
        "ws://127.0.0.1:{port}/games/{game}/ws?token={seat_token}&protocol={}",
        baylee_protocol::PROTOCOL_VERSION
    ))
    .await
    .expect_err("the old seat query opened a socket");
    assert_eq!(status, 401, "{body}");
    let (status, _) = http(
        port,
        "GET",
        &format!("/games/{game}/cosmetics?token={seat_token}"),
        None,
        "",
    );
    assert_eq!(status, 401, "the old cosmetics query");

    // The new ways.
    let (status, body) = http(
        port,
        "GET",
        &format!("/games/{game}/cosmetics"),
        Some(&seat_token),
        "",
    );
    assert_eq!(
        status, 200,
        "cosmetics with the seat token in a header: {body}"
    );
    let mut lobby = upgrade(&common::lobby_url(port, &session, ""))
        .await
        .expect("the lobby with a ticket");
    assert!(next_text(&mut lobby).await.contains("\"games\""));
    let mut ws = upgrade(&common::seat_url(port, &game, &seat_token))
        .await
        .expect("the seat with a ticket");
    opens_on_its_table(&mut ws).await;
}

/// A ticket lifetime the gateway cannot honour refuses startup rather than
/// being found out in production.
#[test]
fn a_ticket_lifetime_out_of_bounds_refuses_startup() {
    for (key, value) in [
        ("BAYLEE_WS_TICKET_SECS", "0"),
        ("BAYLEE_WS_TICKET_SECS", "601"),
        ("BAYLEE_WS_TICKET_SECS", "soon"),
        ("BAYLEE_WS_LEGACY_TOKENS", "forever"),
    ] {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-gateway"))
            .env_remove("DATABASE_URL")
            .env("PORT", "0")
            .env(key, value)
            .output()
            .expect("run the gateway");
        assert!(!out.status.success(), "{key}={value} started");
        let said = String::from_utf8_lossy(&out.stderr);
        assert!(said.contains(key), "{key}={value}: {said}");
    }
}

/// Neither a ticket nor a token reaches the gateway's own log, at the
/// loudest level, on any of the routes that carry one.
#[tokio::test]
async fn no_secret_reaches_the_log() {
    let gw = spawn_gateway_with("tickets-log", &[("RUST_LOG", "debug".into())]);
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let session = login(port, "t-log", "Log");
    let (game, seat_token) = game_against_the_house(port, &session);

    let lobby_ticket = common::lobby_ticket(port, &session);
    let lobby = format!("ws://127.0.0.1:{port}/lobby/ws?ticket={lobby_ticket}");
    let _feed = upgrade(&lobby).await.expect("lobby");
    refused_as_stale(&lobby, "reused").await;
    let seat_ticket = common::seat_ticket(port, &game, &seat_token);
    let seat = common::seat_url_with(port, &game, &seat_ticket);
    let _ws = upgrade(&seat).await.expect("seat");
    refused_as_stale(&seat, "reused").await;
    let _ = upgrade(&format!("ws://127.0.0.1:{port}/lobby/ws?token={session}")).await;
    let _ = http(
        port,
        "GET",
        &format!("/games/{game}/cosmetics?token={seat_token}"),
        None,
        "",
    );
    let logs = gw.logs();
    assert!(
        logs.contains("DEBUG"),
        "the gateway logged nothing at debug, so this proves nothing about it: {logs:.400}"
    );
    for (what, secret) in [
        ("session", &session),
        ("seat token", &seat_token),
        ("lobby ticket", &lobby_ticket),
        ("seat ticket", &seat_ticket),
    ] {
        assert!(!logs.contains(secret.as_str()), "the {what} is in the log");
    }
}

/// A whole game over ticketed sockets: dealt in, an answer given, the
/// socket lost and dialled again with a fresh ticket, and played on to its
/// end, which the gateway records.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_game_is_played_to_its_end_over_ticketed_sockets_with_a_reconnect() {
    let gw = spawn_gateway("tickets-game");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let session = login(port, "t-game", "Game");
    let (game, seat_token) = game_against_the_house(port, &session);

    let mut ws = common::dial_seat(port, &game, &seat_token).await;
    let mut seq = 0;
    let mut asked = false;
    tokio::time::timeout(common::WAIT_BUDGET, async {
        loop {
            match common::next_msg(&mut ws).await {
                Some(v1::envelope::Msg::StateDelta(delta)) => seq = seq.max(delta.seq),
                Some(v1::envelope::Msg::ChoiceRequest(request)) => {
                    seq = seq.max(request.seq);
                    asked = true;
                }
                Some(_) => {}
                None => panic!("the socket closed before the first question"),
            }
            if asked {
                break;
            }
        }
    })
    .await
    .expect("the first question");
    send_action(&mut ws, &PlayerAction::MulliganKeep).await;
    // Wait for the table to move past the mulligan before the line drops.
    tokio::time::timeout(common::WAIT_BUDGET, async {
        loop {
            match common::next_msg(&mut ws).await {
                Some(v1::envelope::Msg::StateDelta(delta)) if delta.seq > seq => {
                    seq = delta.seq;
                    break;
                }
                Some(_) => {}
                None => panic!("the socket closed after the keep"),
            }
        }
    })
    .await
    .expect("the game moved after the keep");
    drop(ws);

    // Back in, on a ticket of its own: the seat token never travels in the
    // address, and the game is the same one.
    let mut ws = common::dial_seat(port, &game, &seat_token).await;
    common::send(
        &mut ws,
        &Envelope {
            msg: Some(v1::envelope::Msg::Resume(v1::ResumeGame {
                game_id: game.clone(),
                seat_token: String::new(),
                last_seq: seq,
            })),
        },
    )
    .await;
    tokio::time::timeout(common::WAIT_BUDGET, async {
        let mut statics = false;
        loop {
            match common::next_msg(&mut ws).await {
                Some(v1::envelope::Msg::GameStatic(_)) => statics = true,
                Some(v1::envelope::Msg::StateDelta(delta)) if statics => {
                    assert!(delta.seq >= seq, "a reconnect never rewinds the game");
                    break;
                }
                Some(v1::envelope::Msg::ChoiceRequest(request)) => {
                    let pending: baylee_engine::choice::Pending =
                        serde_json::from_slice(&request.pending_json).expect("pending");
                    assert!(
                        !matches!(pending, baylee_engine::choice::Pending::Mulligan { .. }),
                        "the reconnected seat was asked its mulligan again"
                    );
                }
                Some(_) => {}
                None => panic!("the reconnected socket closed"),
            }
        }
    })
    .await
    .expect("the table again after the reconnect");

    send_action(&mut ws, &PlayerAction::Concede).await;
    for _ in 0..common::WAIT_TRIES {
        if gw.scalar(&format!(
            "SELECT count(*) FROM game_record WHERE game_id = '{game}' AND complete"
        )) == 1
        {
            return;
        }
        tokio::time::sleep(common::WAIT_STEP).await;
    }
    panic!("the game never ended within {:?}", common::WAIT_BUDGET);
}

async fn send_action(ws: &mut Socket, action: &PlayerAction) {
    let answer = Envelope {
        msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
            game_id: String::new(),
            seat_token: String::new(),
            action_json: serde_json::to_vec(action).expect("an action encodes"),
        })),
    };
    ws.send(tokio_tungstenite::tungstenite::Message::Binary(
        answer.encode_to_vec().into(),
    ))
    .await
    .expect("send an answer");
}
